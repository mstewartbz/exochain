use livesafe::ai_help_manual::{answer_from_manual, build_system_prompt, validate_knowledge_base};
use livesafe::ai_help_topics::{HelpAiQuestionInput, HelpAiSessionOutcome, HelpTopicData};

fn complete_help_topics() -> Vec<HelpTopicData> {
    vec![
        topic(
            "getting-started",
            "Getting Started",
            "ONBOARDING",
            "Start your LiveSafe account and emergency-card setup.",
            "Use onboarding to create your account and begin emergency-card setup.",
            &["getting started", "account", "setup"],
        ),
        topic(
            "account-setup",
            "Account Setup",
            "ONBOARDING",
            "Configure account details before P.A.C.E. and card setup.",
            "Account setup comes before the rest of the LiveSafe onboarding flow.",
            &["account", "profile", "setup"],
        ),
        topic(
            "pace-contacts",
            "P.A.C.E. Contacts",
            "PACE",
            "Invite primary, alternate, contingent, and emergency contacts.",
            "P.A.C.E. contacts accept obligations before notification eligibility activates.",
            &["pace", "contacts", "invite"],
        ),
        topic(
            "emergency-card",
            "Emergency Card",
            "ICE_CARD",
            "Configure the wallet card and printable packet.",
            "The emergency card includes identity, QR, and printable instructions.",
            &["card", "wallet", "print"],
        ),
        topic(
            "qr-activation",
            "QR Activation",
            "QR",
            "Activate the QR pointer without exposing raw sensitive data.",
            "QR activation uses metadata-only pointers and fail-closed responder access.",
            &["qr", "activate", "responder"],
        ),
        topic(
            "responder-access",
            "Responder Access",
            "RESPONDER",
            "Responder views stay limited to the emergency subset.",
            "Responder access remains bounded to emergency-safe projection data.",
            &["responder", "emergency", "access"],
        ),
        topic(
            "emergency-profile",
            "Emergency Profile",
            "EMERGENCY_PROFILE",
            "Manage allowed emergency-profile fields and projection boundaries.",
            "Emergency profile content stays bounded and redacted outside approved release scope.",
            &["emergency", "profile", "release"],
        ),
        topic(
            "medical-jacket",
            "Medical Jacket",
            "MEDICAL_JACKET",
            "Complete the medical jacket using safe phenotypical classes.",
            "The medical jacket tracks custody and emergency projections without raw sensitive exports.",
            &["medical", "jacket", "custody"],
        ),
        topic(
            "phenotypical-records",
            "Phenotypical Records",
            "MEDICAL_JACKET",
            "Phenotypical records remain classified before consent and projection.",
            "Phenotypical records are handled separately from genotypical imports.",
            &["phenotypical", "records", "medical"],
        ),
        topic(
            "genotypical-imports",
            "Genotypical Imports",
            "MEDICAL_JACKET",
            "Genotypical imports remain opt-in and separately classified.",
            "Genotypical imports stay inactive until consent and eligibility rules pass.",
            &["genotypical", "imports", "consent"],
        ),
        topic(
            "consent-revocation",
            "Consent And Revocation",
            "CONSENT",
            "Consent and revocation receipts remain metadata-only until verified proofs exist.",
            "Consent controls deny verified claims before EXOCHAIN proof paths exist.",
            &["consent", "revocation", "receipts"],
        ),
        topic(
            "vault-vitallock",
            "Vault And VitalLock",
            "VAULT",
            "Vault records remain metadata-only and permit-gated.",
            "VitalLock vault access stays bounded to synthetic metadata and emergency-safe scope.",
            &["vault", "vitallock", "records"],
        ),
        topic(
            "ambient-context",
            "Ambient Context",
            "AMBIENT",
            "Ambient context packs require consent and template scope.",
            "Ambient context sharing stays metadata-only until consent and template checks pass.",
            &["ambient", "context", "sharing"],
        ),
        topic(
            "marketplace-templates",
            "Marketplace Templates",
            "MARKETPLACE",
            "Marketplace templates declare plan gates and consent requirements.",
            "Templates declare rule scope, plan gates, required consent, and audit behavior.",
            &["marketplace", "templates", "plans"],
        ),
        topic(
            "family-plans",
            "Family Plans",
            "ENTITLEMENTS",
            "Family plans are explicit entitlement states.",
            "Family-plan behavior is represented as an explicit entitlement without unsupported billing claims.",
            &["family", "plans", "entitlements"],
        ),
        topic(
            "team-plans",
            "Team Plans",
            "ENTITLEMENTS",
            "Team plans are explicit entitlement states.",
            "Team-plan behavior is represented as an explicit entitlement without unsupported billing claims.",
            &["team", "plans", "entitlements"],
        ),
        topic(
            "gift-subscriptions",
            "Gift Subscriptions",
            "ENTITLEMENTS",
            "Gift subscriptions remain configuration-driven entitlement states.",
            "Gift subscriptions use explicit entitlement metadata and synthetic Stripe test values.",
            &["gift", "subscriptions", "entitlements"],
        ),
        topic(
            "frontline-eligibility",
            "Frontline Eligibility",
            "ENTITLEMENTS",
            "Frontline eligibility uses deterministic metadata and safe cohort labels.",
            "Frontline eligibility must not require raw proof documents in fixtures.",
            &["frontline", "eligibility", "cohort"],
        ),
        topic(
            "trial-paid-capabilities",
            "Trial And Paid Capabilities",
            "ENTITLEMENTS",
            "Trials and paid capabilities remain explicit entitlement states.",
            "Trial and paid capability behavior stays configuration-driven and synthetic in tests.",
            &["trial", "paid", "capabilities"],
        ),
        topic(
            "trust-state",
            "Trust State",
            "TRUST",
            "Trust indicators stay inactive until verified adapter and proof gates pass.",
            "LiveSafe does not claim verified EXOCHAIN enforcement before proof gates pass.",
            &["trust", "verification", "adapter"],
        ),
        topic(
            "privacy-safety-boundaries",
            "Privacy And Safety Boundaries",
            "PRIVACY",
            "LiveSafe help must not invent authority or capture raw sensitive data.",
            "The help system must route gaps to feedback rather than inventing medical, legal, billing, or EXOCHAIN authority.",
            &["privacy", "safety", "feedback"],
        ),
    ]
}

fn topic(
    id: &str,
    title: &str,
    category: &str,
    summary: &str,
    body: &str,
    keywords: &[&str],
) -> HelpTopicData {
    HelpTopicData {
        id: id.into(),
        title: title.into(),
        category: category.into(),
        summary: summary.into(),
        body: body.into(),
        keywords: keywords.iter().map(|value| (*value).to_string()).collect(),
    }
}

#[test]
fn knowledge_base_validation_requires_full_required_topic_coverage() {
    let mut topics = complete_help_topics();
    topics.retain(|topic| topic.id != "gift-subscriptions");

    let missing = validate_knowledge_base(&topics).unwrap_err();

    assert_eq!(missing, vec!["gift-subscriptions".to_string()]);
}

#[test]
fn system_prompt_includes_manual_only_guardrails_and_classification_contract() {
    let prompt = build_system_prompt(&complete_help_topics()).expect("prompt should build");

    assert!(prompt.contains("Use only supplied documentation."));
    assert!(prompt.contains("manual lacks the information"));
    assert!(prompt.contains("[OUTCOME: ANSWERED|PARTIALLY_ANSWERED|UNANSWERED|BUG_INDICATED|CONFUSION_DETECTED|PRIVACY_SAFETY_RISK]"));
    assert!(prompt.contains("Emergency Card"));
    assert!(prompt.contains("Privacy And Safety Boundaries"));
}

#[test]
fn answer_from_manual_uses_matching_topics_and_returns_citations() {
    let input = HelpAiQuestionInput {
        question: "How do I activate the QR card for responders?".into(),
        context_topic_id: Some("qr-activation".into()),
        route: Some("/card/activate".into()),
        surface_id: Some("qr-activation".into()),
        session_id: Some("session:manual-qr".into()),
    };

    let answer = answer_from_manual(&input, &complete_help_topics()).expect("answer should build");

    assert_eq!(answer.outcome, HelpAiSessionOutcome::Answered);
    assert_eq!(
        answer.cited_topic_ids,
        vec![
            "qr-activation".to_string(),
            "emergency-card".to_string(),
            "responder-access".to_string()
        ]
    );
    assert!(
        answer
            .text
            .contains("Activate the QR pointer without exposing raw sensitive data.")
    );
    assert!(
        answer
            .text
            .contains("Configure the wallet card and printable packet.")
    );
}

#[test]
fn answer_from_manual_falls_back_to_feedback_when_docs_do_not_cover_the_question() {
    let input = HelpAiQuestionInput {
        question: "Can LiveSafe adjudicate my insurance claim appeal?".into(),
        context_topic_id: None,
        route: Some("/claims".into()),
        surface_id: Some("claims-view".into()),
        session_id: Some("session:manual-gap".into()),
    };

    let answer =
        answer_from_manual(&input, &complete_help_topics()).expect("gap answer should build");

    assert_eq!(answer.outcome, HelpAiSessionOutcome::Unanswered);
    assert!(answer.cited_topic_ids.is_empty());
    assert!(
        answer
            .text
            .contains("current LiveSafe manual does not cover that")
    );
    assert!(answer.text.contains("feedback"));
}
