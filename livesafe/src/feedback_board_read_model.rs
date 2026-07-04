use crate::feedback_mandated_reporter::{
    FeedbackActivity, FeedbackCategory, FeedbackItem, FeedbackPriority, FeedbackRating,
    FeedbackStatus, FeedbackTargetType, MetadataValue,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeedbackBoardQuery {
    pub statuses: Vec<FeedbackStatus>,
    pub target_type: Option<FeedbackTargetType>,
    pub category: Option<FeedbackCategory>,
    pub work_batch_tag: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedbackBoardItemView {
    pub id: String,
    pub target_type: FeedbackTargetType,
    pub target_id: String,
    pub target_label: String,
    pub status: FeedbackStatus,
    pub hold_tag: Option<String>,
    pub priority: FeedbackPriority,
    pub category: FeedbackCategory,
    pub sprint_tag: Option<String>,
    pub title: String,
    pub body: String,
    pub rating: Option<FeedbackRating>,
    pub upvotes: u32,
    pub author: String,
    pub metadata: BTreeMap<String, MetadataValue>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_dispatch_at: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedbackBoardColumn {
    pub status: FeedbackStatus,
    pub items: Vec<FeedbackBoardItemView>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedbackBoardView {
    pub columns: Vec<FeedbackBoardColumn>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedbackTargetCount {
    pub target_type: FeedbackTargetType,
    pub target_id: String,
    pub count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedbackBoardStats {
    pub total_items: usize,
    pub open_items: usize,
    pub held_items: usize,
    pub deployed_items: usize,
    pub by_status: BTreeMap<FeedbackStatus, u32>,
    pub by_category: BTreeMap<FeedbackCategory, u32>,
    pub by_target_type: BTreeMap<FeedbackTargetType, u32>,
}

pub fn build_feedback_board_view(
    items: &[FeedbackItem],
    query: FeedbackBoardQuery,
) -> FeedbackBoardView {
    let statuses = if query.statuses.is_empty() {
        workflow_status_order()
    } else {
        query.statuses.clone()
    };

    let mut columns = Vec::with_capacity(statuses.len());
    for status in statuses {
        let mut column_items = filtered_items(items, &query)
            .into_iter()
            .filter(|item| item.status == status)
            .map(item_view)
            .collect::<Vec<_>>();
        column_items.sort_by(compare_board_items);
        columns.push(FeedbackBoardColumn {
            status,
            items: column_items,
        });
    }

    FeedbackBoardView { columns }
}

pub fn list_feedback_by_target(
    items: &[FeedbackItem],
    target_type: FeedbackTargetType,
    target_id: &str,
) -> Vec<FeedbackBoardItemView> {
    let mut views = items
        .iter()
        .filter(|item| item.target_type == target_type && item.target_id == target_id)
        .map(item_view)
        .collect::<Vec<_>>();
    views.sort_by(compare_board_items);
    views
}

pub fn list_feedback_by_work_batch(
    items: &[FeedbackItem],
    work_batch_tag: &str,
) -> Vec<FeedbackBoardItemView> {
    let mut views = items
        .iter()
        .filter(|item| item.sprint_tag.as_deref() == Some(work_batch_tag))
        .map(item_view)
        .collect::<Vec<_>>();
    views.sort_by(compare_board_items);
    views
}

pub fn find_feedback_item_view(
    items: &[FeedbackItem],
    feedback_id: &str,
) -> Option<FeedbackBoardItemView> {
    items
        .iter()
        .find(|item| item.id == feedback_id)
        .map(item_view)
}

pub fn list_feedback_activity_log(
    activities: &[FeedbackActivity],
    feedback_id: &str,
) -> Vec<FeedbackActivity> {
    let mut filtered = activities
        .iter()
        .filter(|activity| activity.feedback_id == feedback_id)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    filtered
}

pub fn summarize_feedback_target_counts(items: &[FeedbackItem]) -> Vec<FeedbackTargetCount> {
    let mut counts = BTreeMap::<(FeedbackTargetType, String), u32>::new();
    for item in items {
        *counts
            .entry((item.target_type, item.target_id.clone()))
            .or_default() += 1;
    }

    counts
        .into_iter()
        .map(|((target_type, target_id), count)| FeedbackTargetCount {
            target_type,
            target_id,
            count,
        })
        .collect()
}

pub fn summarize_feedback_stats(items: &[FeedbackItem]) -> FeedbackBoardStats {
    let mut by_status = BTreeMap::new();
    let mut by_category = BTreeMap::new();
    let mut by_target_type = BTreeMap::new();
    let mut open_items = 0usize;
    let mut held_items = 0usize;
    let mut deployed_items = 0usize;

    for item in items {
        *by_status.entry(item.status).or_default() += 1;
        *by_category.entry(item.category).or_default() += 1;
        *by_target_type.entry(item.target_type).or_default() += 1;

        if item.status == FeedbackStatus::Held {
            held_items += 1;
        } else if item.status == FeedbackStatus::Deployed {
            deployed_items += 1;
        } else {
            open_items += 1;
        }
    }

    FeedbackBoardStats {
        total_items: items.len(),
        open_items,
        held_items,
        deployed_items,
        by_status,
        by_category,
        by_target_type,
    }
}

fn filtered_items<'a>(
    items: &'a [FeedbackItem],
    query: &FeedbackBoardQuery,
) -> Vec<&'a FeedbackItem> {
    items
        .iter()
        .filter(|item| {
            query
                .target_type
                .map(|target_type| item.target_type == target_type)
                .unwrap_or(true)
        })
        .filter(|item| {
            query
                .category
                .map(|category| item.category == category)
                .unwrap_or(true)
        })
        .filter(|item| {
            query
                .work_batch_tag
                .as_deref()
                .map(|tag| item.sprint_tag.as_deref() == Some(tag))
                .unwrap_or(true)
        })
        .collect()
}

fn item_view(item: &FeedbackItem) -> FeedbackBoardItemView {
    FeedbackBoardItemView {
        id: item.id.clone(),
        target_type: item.target_type,
        target_id: item.target_id.clone(),
        target_label: item.target_label.clone(),
        status: item.status,
        hold_tag: item.hold_tag.clone(),
        priority: item.priority,
        category: item.category,
        sprint_tag: item.sprint_tag.clone(),
        title: item.title.clone(),
        body: item.body.clone(),
        rating: item.rating,
        upvotes: item.upvotes,
        author: item.author.clone(),
        metadata: sanitize_metadata(&item.metadata),
        created_at: item.created_at,
        updated_at: item.updated_at,
        last_dispatch_at: item.last_dispatch_at,
    }
}

fn compare_board_items(
    left: &FeedbackBoardItemView,
    right: &FeedbackBoardItemView,
) -> std::cmp::Ordering {
    priority_rank(right.priority)
        .cmp(&priority_rank(left.priority))
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| left.id.cmp(&right.id))
}

fn priority_rank(priority: FeedbackPriority) -> u8 {
    match priority {
        FeedbackPriority::None => 0,
        FeedbackPriority::Low => 1,
        FeedbackPriority::Medium => 2,
        FeedbackPriority::High => 3,
        FeedbackPriority::Critical => 4,
    }
}

fn workflow_status_order() -> Vec<FeedbackStatus> {
    vec![
        FeedbackStatus::New,
        FeedbackStatus::Backlog,
        FeedbackStatus::Planning,
        FeedbackStatus::Development,
        FeedbackStatus::Testing,
        FeedbackStatus::Validation,
        FeedbackStatus::Held,
        FeedbackStatus::Deployed,
    ]
}

fn sanitize_metadata(
    metadata: &BTreeMap<String, MetadataValue>,
) -> BTreeMap<String, MetadataValue> {
    metadata
        .iter()
        .filter_map(|(key, value)| {
            sanitize_metadata_entry(key, value).map(|value| (key.clone(), value))
        })
        .collect()
}

fn sanitize_metadata_entry(key: &str, value: &MetadataValue) -> Option<MetadataValue> {
    if key == "_systemUpvoteVoters" || key.starts_with('_') || metadata_key_is_unsafe(key) {
        return None;
    }

    match value {
        MetadataValue::Bool(flag) => Some(MetadataValue::Bool(*flag)),
        MetadataValue::Number(number) => Some(MetadataValue::Number(*number)),
        MetadataValue::Text(text) => {
            if metadata_text_is_unsafe(text) {
                None
            } else {
                Some(MetadataValue::Text(text.clone()))
            }
        }
        MetadataValue::List(entries) => {
            let entries = entries
                .iter()
                .filter_map(|entry| sanitize_metadata_entry(key, entry))
                .collect::<Vec<_>>();
            Some(MetadataValue::List(entries))
        }
        MetadataValue::Map(entries) => {
            let entries = entries
                .iter()
                .filter_map(|(nested_key, nested_value)| {
                    sanitize_metadata_entry(nested_key, nested_value)
                        .map(|value| (nested_key.clone(), value))
                })
                .collect::<BTreeMap<_, _>>();
            Some(MetadataValue::Map(entries))
        }
    }
}

fn metadata_key_is_unsafe(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized.contains("rawqr")
        || normalized.contains("medicalrecord")
        || normalized.contains("genetic")
        || normalized.contains("payment")
        || normalized.contains("eligibilitydocument")
        || normalized.contains("unsafe")
}

fn metadata_text_is_unsafe(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.contains("otpauth://")
        || normalized.contains("raw medical")
        || normalized.contains("private key")
}
