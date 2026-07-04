use livesafe::ai_help_topics::{
    HelpAiQuestionInput, HelpAiSessionOutcome, HelpResponseParse, HelpTopicData, match_topics,
    parse_help_response,
};

fn help_topics() -> Vec<HelpTopicData> {
    vec![
        HelpTopicData {
            id: "getting-started".into(),
            title: "Getting Started".into(),
            category: "ONBOARDING".into(),
            summary: "Set up your LiveSafe account and begin the emergency card flow.".into(),
            body: "Account setup guidance for onboarding and emergency card configuration.".into(),
            keywords: vec!["account".into(), "setup".into(), "onboarding".into()],
        },
        HelpTopicData {
            id: "pace-contacts".into(),
            title: "P.A.C.E. Contacts".into(),
            category: "PACE".into(),
            summary: "Invite primary alternate contingent and emergency contacts.".into(),
            body: "P.A.C.E. contacts accept obligations before notification eligibility activates."
                .into(),
            keywords: vec!["pace".into(), "contacts".into(), "invite".into()],
        },
        HelpTopicData {
            id: "emergency-card".into(),
            title: "Emergency Card Setup".into(),
            category: "ICE_CARD".into(),
            summary: "Configure wallet card preferences and QR activation pointers.".into(),
            body: "The emergency card packet includes identity, QR, and printable instructions."
                .into(),
            keywords: vec!["card".into(), "qr".into(), "wallet".into()],
        },
        HelpTopicData {
            id: "medical-jacket".into(),
            title: "Medical Jacket".into(),
            category: "MEDICAL_JACKET".into(),
            summary: "Complete the medical jacket with safe phenotypical record classes.".into(),
            body:
                "Medical jacket guidance covers consent scopes and emergency projection boundaries."
                    .into(),
            keywords: vec!["medical".into(), "jacket".into(), "consent".into()],
        },
        HelpTopicData {
            id: "marketplace-templates".into(),
            title: "Marketplace Templates".into(),
            category: "MARKETPLACE".into(),
            summary: "Browse rule packs and LiveSafe marketplace templates.".into(),
            body: "Templates declare plan gates, consent requirements, and audit behavior.".into(),
            keywords: vec!["marketplace".into(), "templates".into(), "plans".into()],
        },
        HelpTopicData {
            id: "trust-state".into(),
            title: "Trust State".into(),
            category: "TRUST".into(),
            summary: "Understand inactive and adapter-missing trust states.".into(),
            body: "LiveSafe does not claim verified EXOCHAIN enforcement before proof gates pass."
                .into(),
            keywords: vec!["trust".into(), "verification".into(), "adapter".into()],
        },
    ]
}

#[test]
fn topic_matching_is_deterministic_and_scores_title_keywords_summary_and_body() {
    let input = HelpAiQuestionInput {
        question: "How do I set up my emergency card qr?".into(),
        context_topic_id: None,
        route: Some("/onboarding/card".into()),
        surface_id: Some("ice-card-generator".into()),
        session_id: Some("session:synthetic-help".into()),
    };

    let matches = match_topics(&input, &help_topics());

    assert_eq!(matches.len(), 4);
    assert_eq!(matches[0].topic_id, "emergency-card");
    assert_eq!(matches[0].score, 39);
    assert_eq!(matches[1].topic_id, "getting-started");
    assert!(matches[0].score > matches[1].score);
}

#[test]
fn topic_matching_returns_at_most_five_topics() {
    let input = HelpAiQuestionInput {
        question: "account onboarding invite card medical marketplace trust".into(),
        context_topic_id: None,
        route: None,
        surface_id: None,
        session_id: None,
    };

    let matches = match_topics(&input, &help_topics());

    assert_eq!(matches.len(), 5);
}

#[test]
fn context_topic_is_included_even_when_keyword_score_is_zero() {
    let input = HelpAiQuestionInput {
        question: "How do subscriptions work?".into(),
        context_topic_id: Some("trust-state".into()),
        route: Some("/billing".into()),
        surface_id: Some("entitlement-plan-selector".into()),
        session_id: None,
    };

    let matches = match_topics(&input, &help_topics());

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].topic_id, "trust-state");
    assert_eq!(matches[0].score, 0);
}

#[test]
fn response_parser_extracts_outcome_and_cited_topics_and_strips_control_lines() {
    let parsed = parse_help_response(
        "Use the emergency card generator to print a wallet card.\n[CITED: emergency-card, getting-started]\n[OUTCOME: ANSWERED]",
    );

    assert_eq!(
        parsed,
        HelpResponseParse {
            text: "Use the emergency card generator to print a wallet card.".into(),
            outcome: HelpAiSessionOutcome::Answered,
            cited_topic_ids: vec!["emergency-card".into(), "getting-started".into()],
        }
    );
}

#[test]
fn parser_defaults_to_partially_answered_when_control_lines_are_missing() {
    let parsed = parse_help_response("I think this is how it works.");

    assert_eq!(parsed.outcome, HelpAiSessionOutcome::PartiallyAnswered);
    assert_eq!(parsed.cited_topic_ids, Vec::<String>::new());
    assert_eq!(parsed.text, "I think this is how it works.");
}

#[test]
fn parser_rejects_unknown_outcome_and_falls_back_to_partial_answer() {
    let parsed = parse_help_response(
        "Escalate through feedback.\n[OUTCOME: TOTALLY_CERTAIN]\n[CITED: pace-contacts]",
    );

    assert_eq!(parsed.outcome, HelpAiSessionOutcome::PartiallyAnswered);
    assert_eq!(parsed.cited_topic_ids, vec!["pace-contacts".to_string()]);
    assert_eq!(parsed.text, "Escalate through feedback.");
}
