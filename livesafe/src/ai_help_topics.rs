use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpAiQuestionInput {
    pub question: String,
    pub context_topic_id: Option<String>,
    pub route: Option<String>,
    pub surface_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HelpAiSessionOutcome {
    Answered,
    PartiallyAnswered,
    Unanswered,
    BugIndicated,
    ConfusionDetected,
    PrivacySafetyRisk,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpTopicData {
    pub id: String,
    pub title: String,
    pub category: String,
    pub summary: String,
    pub body: String,
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopicMatch {
    pub topic_id: String,
    pub score: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpResponseParse {
    pub text: String,
    pub outcome: HelpAiSessionOutcome,
    pub cited_topic_ids: Vec<String>,
}

pub fn match_topics(input: &HelpAiQuestionInput, topics: &[HelpTopicData]) -> Vec<TopicMatch> {
    let query_terms = tokenize(&input.question);
    let mut matches = Vec::new();

    for topic in topics {
        let title_terms = tokenize(&topic.title);
        let keyword_terms = topic
            .keywords
            .iter()
            .flat_map(|keyword| tokenize(keyword))
            .collect::<BTreeSet<_>>();
        let summary_terms = tokenize(&topic.summary);
        let body_terms = tokenize(&topic.body);

        let mut score = 0;
        for term in &query_terms {
            if title_terms.contains(term) {
                score += 10;
            }
            if keyword_terms.contains(term) {
                score += 5;
            }
            if summary_terms.contains(term) {
                score += 3;
            }
            if body_terms.contains(term) {
                score += 1;
            }
        }

        let context_match = input.context_topic_id.as_deref() == Some(topic.id.as_str());
        if score > 0 || context_match {
            matches.push(TopicMatch {
                topic_id: topic.id.clone(),
                score,
            });
        }
    }

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.topic_id.cmp(&right.topic_id))
    });
    matches.truncate(5);
    matches
}

pub fn parse_help_response(response: &str) -> HelpResponseParse {
    let mut content_lines = Vec::new();
    let mut outcome = None;
    let mut cited_topic_ids = Vec::new();

    for line in response.lines() {
        if let Some(parsed_outcome) = parse_outcome_line(line) {
            outcome = Some(parsed_outcome);
            continue;
        }

        if let Some(parsed_citations) = parse_cited_line(line) {
            cited_topic_ids = parsed_citations;
            continue;
        }

        content_lines.push(line.trim_end());
    }

    let text = content_lines.join("\n").trim().to_string();

    HelpResponseParse {
        text,
        outcome: outcome.unwrap_or(HelpAiSessionOutcome::PartiallyAnswered),
        cited_topic_ids,
    }
}

fn parse_outcome_line(line: &str) -> Option<HelpAiSessionOutcome> {
    let value = bracket_value(line, "OUTCOME")?;
    Some(match value {
        "ANSWERED" => HelpAiSessionOutcome::Answered,
        "PARTIALLY_ANSWERED" => HelpAiSessionOutcome::PartiallyAnswered,
        "UNANSWERED" => HelpAiSessionOutcome::Unanswered,
        "BUG_INDICATED" => HelpAiSessionOutcome::BugIndicated,
        "CONFUSION_DETECTED" => HelpAiSessionOutcome::ConfusionDetected,
        "PRIVACY_SAFETY_RISK" => HelpAiSessionOutcome::PrivacySafetyRisk,
        _ => HelpAiSessionOutcome::PartiallyAnswered,
    })
}

fn parse_cited_line(line: &str) -> Option<Vec<String>> {
    let value = bracket_value(line, "CITED")?;
    let citations = value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    Some(citations)
}

fn bracket_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("[{key}:");
    let trimmed = line.trim();
    if !trimmed.starts_with(&prefix) || !trimmed.ends_with(']') {
        return None;
    }

    Some(trimmed[prefix.len()..trimmed.len() - 1].trim())
}

fn tokenize(input: &str) -> BTreeSet<String> {
    input
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}
