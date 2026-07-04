use crate::ai_help_topics::{HelpAiQuestionInput, HelpAiSessionOutcome, HelpTopicData};
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED_TOPIC_IDS: [&str; 21] = [
    "getting-started",
    "account-setup",
    "pace-contacts",
    "emergency-card",
    "qr-activation",
    "responder-access",
    "emergency-profile",
    "medical-jacket",
    "phenotypical-records",
    "genotypical-imports",
    "consent-revocation",
    "vault-vitallock",
    "ambient-context",
    "marketplace-templates",
    "family-plans",
    "team-plans",
    "gift-subscriptions",
    "frontline-eligibility",
    "trial-paid-capabilities",
    "trust-state",
    "privacy-safety-boundaries",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpManualAnswer {
    pub text: String,
    pub outcome: HelpAiSessionOutcome,
    pub cited_topic_ids: Vec<String>,
}

pub fn validate_knowledge_base(topics: &[HelpTopicData]) -> Result<(), Vec<String>> {
    let known_topic_ids = topics
        .iter()
        .map(|topic| topic.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    let missing = REQUIRED_TOPIC_IDS
        .iter()
        .filter(|topic_id| !known_topic_ids.contains(**topic_id))
        .map(|topic_id| (*topic_id).to_string())
        .collect::<Vec<_>>();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

pub fn build_system_prompt(topics: &[HelpTopicData]) -> Result<String, Vec<String>> {
    validate_knowledge_base(topics)?;

    let topic_lines = topics
        .iter()
        .map(|topic| {
            format!(
                "- {} [{}] {} :: {}",
                topic.title, topic.id, topic.summary, topic.body
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
        concat!(
            "Use only supplied documentation.\n",
            "Do not invent features, plan behavior, legal effect, medical advice, eligibility outcomes, payment outcomes, or EXOCHAIN enforcement.\n",
            "If the manual lacks the information, say that the current LiveSafe manual lacks the information and offer the feedback path.\n",
            "Be concise and use product terminology from the docs.\n",
            "End with classification lines:\n",
            "[OUTCOME: ANSWERED|PARTIALLY_ANSWERED|UNANSWERED|BUG_INDICATED|CONFUSION_DETECTED|PRIVACY_SAFETY_RISK]\n",
            "[CITED: comma-separated-topic-ids]\n\n",
            "Approved help topics:\n",
            "{}"
        ),
        topic_lines
    ))
}

pub fn answer_from_manual(
    input: &HelpAiQuestionInput,
    topics: &[HelpTopicData],
) -> Result<HelpManualAnswer, Vec<String>> {
    validate_knowledge_base(topics)?;

    let topic_index = topics
        .iter()
        .map(|topic| (topic.id.clone(), topic))
        .collect::<BTreeMap<_, _>>();

    let mut scored_matches = topics
        .iter()
        .map(|topic| (topic.id.clone(), semantic_overlap_bonus(input, topic)))
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();

    scored_matches.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    scored_matches.truncate(3);

    if scored_matches.is_empty() || scored_matches[0].1 < 5 {
        if let Some(context_topic_id) = input.context_topic_id.as_ref()
            && let Some(topic) = topic_index.get(context_topic_id)
        {
            return Ok(HelpManualAnswer {
                text: format!(
                    "{}\n\nIf this does not answer your exact workflow, the current LiveSafe manual may not cover that detail yet. Please use feedback so it can be reviewed safely.",
                    topic.summary
                ),
                outcome: HelpAiSessionOutcome::PartiallyAnswered,
                cited_topic_ids: vec![topic.id.clone()],
            });
        }

        return Ok(HelpManualAnswer {
            text: "The current LiveSafe manual does not cover that yet. Please use feedback so the gap can be reviewed without inventing unsupported product behavior.".into(),
            outcome: HelpAiSessionOutcome::Unanswered,
            cited_topic_ids: Vec::new(),
        });
    }

    let cited_topic_ids = scored_matches
        .iter()
        .map(|(topic_id, _)| topic_id.clone())
        .collect::<Vec<_>>();

    let answer_sections = scored_matches
        .iter()
        .filter_map(|(topic_id, _)| topic_index.get(topic_id))
        .map(|topic| topic.summary.as_str())
        .collect::<Vec<_>>();

    let outcome = if scored_matches[0].1 >= 10 {
        HelpAiSessionOutcome::Answered
    } else {
        HelpAiSessionOutcome::PartiallyAnswered
    };

    Ok(HelpManualAnswer {
        text: format!(
            "{}\n\nIf you need a workflow that is not covered here, use feedback so the manual can be updated without unsupported claims.",
            answer_sections.join("\n")
        ),
        outcome,
        cited_topic_ids,
    })
}

fn semantic_overlap_bonus(input: &HelpAiQuestionInput, topic: &HelpTopicData) -> u32 {
    let query_terms = normalized_terms(&input.question);
    let title_terms = normalized_terms(&topic.title);
    let keyword_terms = topic
        .keywords
        .iter()
        .flat_map(|keyword| normalized_terms(keyword))
        .collect::<BTreeSet<_>>();
    let id_terms = normalized_terms(&topic.id.replace('-', " "));

    let mut score = 0;
    for term in query_terms {
        if title_terms.contains(&term) {
            score += 3;
        }
        if keyword_terms.contains(&term) {
            score += 2;
        }
        if id_terms.contains(&term) {
            score += 1;
        }
    }

    score
}

fn normalized_terms(input: &str) -> BTreeSet<String> {
    input
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .map(|term| {
            if term.len() > 3 && term.ends_with('s') {
                term[..term.len() - 1].to_string()
            } else {
                term
            }
        })
        .collect()
}
