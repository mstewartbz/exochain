use livesafe::ambient_signal::{
    AmbientRuntimeState, AmbientSignalDecision, AmbientSignalMode, AmbientSignalRequest,
    AmbientVisibility, evaluate_ambient_signal,
};

fn valid_request() -> AmbientSignalRequest {
    AmbientSignalRequest {
        signal_ref: "ambient-signal:synthetic:2026-05-26".into(),
        policy_ref: "policy:ambient-signal:current".into(),
        session_ref: "ambient-session:synthetic:2026-05-26".into(),
        mode: AmbientSignalMode::ContextPackDispatch,
        runtime_state: AmbientRuntimeState::VerifiedPermit,
        visibility: AmbientVisibility::MetadataOnly,
        marketplace_template_declared: true,
        consent_acknowledged: true,
        includes_raw_sensitive_payload: false,
        includes_direct_contact_value: false,
        includes_location_trace: false,
        has_disablement_ref: true,
        is_synthetic_fixture: true,
        claims_verified_ambient_trust: false,
    }
}

fn assert_denied(decision: &AmbientSignalDecision, reason: &str) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn ambient_signal_requires_references_and_synthetic_fixtures() {
    let mut request = valid_request();
    request.signal_ref = String::new();
    request.policy_ref = String::new();
    request.session_ref = String::new();
    request.is_synthetic_fixture = false;

    let decision = evaluate_ambient_signal(request);

    assert_denied(
        &decision,
        "Ambient signals require synthetic signal, policy, and session references.",
    );
    assert_denied(
        &decision,
        "Ambient signal fixtures must remain synthetic until a verified runtime path exists.",
    );
}

#[test]
fn ambient_context_pack_dispatch_requires_template_and_consent() {
    let mut request = valid_request();
    request.marketplace_template_declared = false;
    request.consent_acknowledged = false;

    let decision = evaluate_ambient_signal(request);

    assert_denied(
        &decision,
        "Ambient context dispatch requires a declared marketplace template.",
    );
    assert_denied(
        &decision,
        "Ambient context dispatch requires acknowledged Ambient signal consent.",
    );
}

#[test]
fn ambient_signal_denies_raw_sensitive_contact_and_location_payloads() {
    let mut request = valid_request();
    request.includes_raw_sensitive_payload = true;
    request.includes_direct_contact_value = true;
    request.includes_location_trace = true;

    let decision = evaluate_ambient_signal(request);

    assert_denied(
        &decision,
        "Ambient signal payloads must stay metadata-only and exclude raw sensitive records.",
    );
    assert_denied(
        &decision,
        "Ambient signal payloads must not embed direct contact values.",
    );
    assert_denied(
        &decision,
        "Ambient signal payloads must not embed location traces or responder-tracking data.",
    );
}

#[test]
fn recipient_visible_dispatch_requires_verified_permit_and_disablement() {
    let mut request = valid_request();
    request.runtime_state = AmbientRuntimeState::Inactive;
    request.visibility = AmbientVisibility::RecipientVisibleSummary;
    request.has_disablement_ref = false;

    let decision = evaluate_ambient_signal(request);

    assert_denied(
        &decision,
        "Recipient-visible Ambient signals remain inactive until a verified adapter path returns permit.",
    );
    assert_denied(
        &decision,
        "Ambient signal routes require a disablement reference before recipient-visible delivery can be shown.",
    );
}

#[test]
fn owner_preview_can_remain_inactive_without_verified_claims() {
    let mut request = valid_request();
    request.mode = AmbientSignalMode::OwnerPreview;
    request.runtime_state = AmbientRuntimeState::Inactive;
    request.visibility = AmbientVisibility::MetadataOnly;

    let decision = evaluate_ambient_signal(request);

    assert!(decision.allowed, "{decision:?}");
}

#[test]
fn verified_ambient_claims_require_verified_permit_state() {
    let mut request = valid_request();
    request.claims_verified_ambient_trust = true;
    request.runtime_state = AmbientRuntimeState::Unavailable;

    let decision = evaluate_ambient_signal(request);

    assert_denied(
        &decision,
        "Verified Ambient trust claims are blocked unless the signal path is in a verified permit state.",
    );
}
