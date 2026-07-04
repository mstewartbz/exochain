use livesafe::feedback_board_read_model::{
    FeedbackBoardQuery, FeedbackBoardStats, FeedbackTargetCount, build_feedback_board_view,
    find_feedback_item_view, list_feedback_activity_log, list_feedback_by_target,
    list_feedback_by_work_batch, summarize_feedback_stats, summarize_feedback_target_counts,
};
use livesafe::feedback_mandated_reporter::{
    FeedbackActivity, FeedbackActivityAction, FeedbackCategory, FeedbackItem, FeedbackPriority,
    FeedbackRating, FeedbackStatus, FeedbackTargetType, MetadataValue,
};
use std::collections::BTreeMap;

fn metadata_with_internal_state() -> BTreeMap<String, MetadataValue> {
    BTreeMap::from([
        (
            "route".into(),
            MetadataValue::Text("/support/feedback".into()),
        ),
        (
            "surfaceId".into(),
            MetadataValue::Text("feedback-kanban-board".into()),
        ),
        (
            "displayedValues".into(),
            MetadataValue::List(vec![MetadataValue::Text("synthetic label".into())]),
        ),
        (
            "_systemUpvoteVoters".into(),
            MetadataValue::List(vec![MetadataValue::Text("user:hidden".into())]),
        ),
        ("isSynthetic".into(), MetadataValue::Bool(true)),
    ])
}

fn feedback_item(
    id: &str,
    status: FeedbackStatus,
    priority: FeedbackPriority,
    updated_at: i64,
) -> FeedbackItem {
    FeedbackItem {
        id: id.into(),
        target_type: FeedbackTargetType::IceCard,
        target_id: "ice-card:generator".into(),
        target_label: "ICE card generator".into(),
        status,
        hold_tag: None,
        priority,
        category: FeedbackCategory::UiUx,
        sprint_tag: Some("batch:card-flow".into()),
        title: format!("Feedback {id}"),
        body: "Synthetic board detail.".into(),
        rating: Some(FeedbackRating::Helpful),
        upvotes: 2,
        author: "user:synthetic".into(),
        metadata: metadata_with_internal_state(),
        created_at: updated_at - 50,
        updated_at,
        last_dispatch_at: None,
    }
}

fn feedback_activity(
    id: &str,
    feedback_id: &str,
    action: FeedbackActivityAction,
    created_at: i64,
) -> FeedbackActivity {
    FeedbackActivity {
        id: id.into(),
        feedback_id: feedback_id.into(),
        action,
        from_status: Some(FeedbackStatus::Backlog),
        to_status: Some(FeedbackStatus::Planning),
        note: Some("Synthetic activity".into()),
        author: "user:synthetic".into(),
        created_at,
    }
}

#[test]
fn board_view_groups_statuses_orders_items_and_strips_internal_metadata() {
    let items = vec![
        feedback_item(
            "feedback:planning-low",
            FeedbackStatus::Planning,
            FeedbackPriority::Low,
            1_717_200_100_000,
        ),
        feedback_item(
            "feedback:planning-high",
            FeedbackStatus::Planning,
            FeedbackPriority::High,
            1_717_200_200_000,
        ),
        feedback_item(
            "feedback:backlog-medium",
            FeedbackStatus::Backlog,
            FeedbackPriority::Medium,
            1_717_200_300_000,
        ),
    ];

    let view = build_feedback_board_view(
        &items,
        FeedbackBoardQuery {
            statuses: vec![FeedbackStatus::Planning, FeedbackStatus::Backlog],
            ..FeedbackBoardQuery::default()
        },
    );

    assert_eq!(
        view.columns
            .iter()
            .map(|column| column.status)
            .collect::<Vec<_>>(),
        vec![FeedbackStatus::Planning, FeedbackStatus::Backlog]
    );
    assert_eq!(
        view.columns[0]
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["feedback:planning-high", "feedback:planning-low"]
    );
    assert!(
        !view.columns[0].items[0]
            .metadata
            .contains_key("_systemUpvoteVoters")
    );
}

#[test]
fn read_model_supports_target_work_batch_item_and_activity_queries() {
    let mut target_match = feedback_item(
        "feedback:target-match",
        FeedbackStatus::New,
        FeedbackPriority::Medium,
        1_717_200_400_000,
    );
    target_match.target_type = FeedbackTargetType::TrustState;
    target_match.target_id = "trust-state:banner".into();
    target_match.sprint_tag = Some("batch:trust-copy".into());

    let mut unrelated = feedback_item(
        "feedback:other",
        FeedbackStatus::Backlog,
        FeedbackPriority::Low,
        1_717_200_500_000,
    );
    unrelated.target_type = FeedbackTargetType::General;
    unrelated.target_id = "general:queue".into();
    unrelated.sprint_tag = Some("batch:other".into());

    let items = vec![target_match.clone(), unrelated];
    let activities = vec![
        feedback_activity(
            "activity:2",
            "feedback:target-match",
            FeedbackActivityAction::Commented,
            1_717_200_700_000,
        ),
        feedback_activity(
            "activity:1",
            "feedback:target-match",
            FeedbackActivityAction::Created,
            1_717_200_600_000,
        ),
        feedback_activity(
            "activity:3",
            "feedback:other",
            FeedbackActivityAction::Created,
            1_717_200_800_000,
        ),
    ];

    let target_items =
        list_feedback_by_target(&items, FeedbackTargetType::TrustState, "trust-state:banner");
    assert_eq!(target_items.len(), 1);
    assert_eq!(target_items[0].id, "feedback:target-match");

    let work_batch_items = list_feedback_by_work_batch(&items, "batch:trust-copy");
    assert_eq!(work_batch_items.len(), 1);
    assert_eq!(work_batch_items[0].id, "feedback:target-match");

    let item = find_feedback_item_view(&items, "feedback:target-match")
        .expect("expected matching feedback item");
    assert_eq!(item.target_id, "trust-state:banner");
    assert!(!item.metadata.contains_key("_systemUpvoteVoters"));

    let activity_log = list_feedback_activity_log(&activities, "feedback:target-match");
    assert_eq!(
        activity_log
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["activity:1", "activity:2"]
    );
}

#[test]
fn read_model_summarizes_target_counts_and_board_stats() {
    let mut trust_state = feedback_item(
        "feedback:trust",
        FeedbackStatus::New,
        FeedbackPriority::High,
        1_717_200_900_000,
    );
    trust_state.target_type = FeedbackTargetType::TrustState;
    trust_state.target_id = "trust-state:banner".into();
    trust_state.category = FeedbackCategory::DocumentationGap;

    let mut card_bug = feedback_item(
        "feedback:card",
        FeedbackStatus::Development,
        FeedbackPriority::Critical,
        1_717_201_000_000,
    );
    card_bug.category = FeedbackCategory::Bug;

    let mut held_item = feedback_item(
        "feedback:held",
        FeedbackStatus::Held,
        FeedbackPriority::Low,
        1_717_201_100_000,
    );
    held_item.category = FeedbackCategory::PrivacySafety;

    let items = vec![trust_state, card_bug, held_item];

    let target_counts = summarize_feedback_target_counts(&items);
    assert_eq!(
        target_counts,
        vec![
            FeedbackTargetCount {
                target_type: FeedbackTargetType::IceCard,
                target_id: "ice-card:generator".into(),
                count: 2,
            },
            FeedbackTargetCount {
                target_type: FeedbackTargetType::TrustState,
                target_id: "trust-state:banner".into(),
                count: 1,
            },
        ]
    );

    let stats = summarize_feedback_stats(&items);
    assert_eq!(
        stats,
        FeedbackBoardStats {
            total_items: 3,
            open_items: 2,
            held_items: 1,
            deployed_items: 0,
            by_status: BTreeMap::from([
                (FeedbackStatus::New, 1),
                (FeedbackStatus::Development, 1),
                (FeedbackStatus::Held, 1),
            ]),
            by_category: BTreeMap::from([
                (FeedbackCategory::Bug, 1),
                (FeedbackCategory::DocumentationGap, 1),
                (FeedbackCategory::PrivacySafety, 1),
            ]),
            by_target_type: BTreeMap::from([
                (FeedbackTargetType::IceCard, 2),
                (FeedbackTargetType::TrustState, 1),
            ]),
        }
    );
}
