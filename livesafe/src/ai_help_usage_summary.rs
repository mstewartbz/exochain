use crate::ai_help_topics::HelpAiSessionOutcome;
use std::collections::BTreeMap;

const SEVEN_DAY_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpUsageSessionRecord {
    pub session_id: String,
    pub created_at: i64,
    pub outcome: HelpAiSessionOutcome,
    pub cited_topic_ids: Vec<String>,
    pub normalized_question: String,
    pub generated_feedback_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopicUsageCount {
    pub topic_id: String,
    pub count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpUsageSummary {
    pub window_started_at: i64,
    pub window_ended_at: i64,
    pub total_sessions: usize,
    pub generated_feedback_count: u32,
    pub outcome_counts: BTreeMap<HelpAiSessionOutcome, u32>,
    pub topic_counts: Vec<TopicUsageCount>,
    pub top_questions: Vec<(String, u32)>,
    pub unresolved_topics: Vec<String>,
}

pub fn summarize_help_usage(sessions: &[HelpUsageSessionRecord], now: i64) -> HelpUsageSummary {
    let window_started_at = now - SEVEN_DAY_WINDOW_MS + 1;
    let mut outcome_counts = BTreeMap::<HelpAiSessionOutcome, u32>::new();
    let mut topic_counts = BTreeMap::<String, u32>::new();
    let mut question_counts = BTreeMap::<String, u32>::new();
    let mut generated_feedback_count = 0u32;
    let mut total_sessions = 0usize;

    for session in sessions {
        if session.created_at < window_started_at || session.created_at > now {
            continue;
        }

        total_sessions += 1;
        generated_feedback_count += session.generated_feedback_count;
        *outcome_counts.entry(session.outcome).or_default() += 1;

        let normalized_question = normalize_question(&session.normalized_question);
        if !normalized_question.is_empty() {
            *question_counts.entry(normalized_question).or_default() += 1;
        }

        for topic_id in &session.cited_topic_ids {
            *topic_counts.entry(topic_id.clone()).or_default() += 1;
        }
    }

    let mut topic_counts = topic_counts
        .into_iter()
        .map(|(topic_id, count)| TopicUsageCount { topic_id, count })
        .collect::<Vec<_>>();
    topic_counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.topic_id.cmp(&right.topic_id))
    });

    let mut top_questions = question_counts.into_iter().collect::<Vec<_>>();
    top_questions.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let unresolved_topics = topic_counts
        .iter()
        .filter(|topic| {
            topic_counts_for_outcomes(
                sessions,
                now,
                &topic.topic_id,
                &[
                    HelpAiSessionOutcome::Unanswered,
                    HelpAiSessionOutcome::ConfusionDetected,
                    HelpAiSessionOutcome::BugIndicated,
                    HelpAiSessionOutcome::PrivacySafetyRisk,
                ],
            ) > 0
        })
        .map(|topic| topic.topic_id.clone())
        .collect();

    HelpUsageSummary {
        window_started_at,
        window_ended_at: now,
        total_sessions,
        generated_feedback_count,
        outcome_counts,
        topic_counts,
        top_questions,
        unresolved_topics,
    }
}

fn topic_counts_for_outcomes(
    sessions: &[HelpUsageSessionRecord],
    now: i64,
    topic_id: &str,
    outcomes: &[HelpAiSessionOutcome],
) -> u32 {
    let window_started_at = now - SEVEN_DAY_WINDOW_MS + 1;
    sessions
        .iter()
        .filter(|session| session.created_at >= window_started_at && session.created_at <= now)
        .filter(|session| outcomes.contains(&session.outcome))
        .filter(|session| {
            session
                .cited_topic_ids
                .iter()
                .any(|entry| entry == topic_id)
        })
        .count() as u32
}

fn normalize_question(input: &str) -> String {
    input
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}
