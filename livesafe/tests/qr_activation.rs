use livesafe::qr_activation::{
    ActivationDisclosureScope, ActivationMode, ActivationRuntimeState, QrActivationDecision,
    QrActivationRequest, evaluate_qr_activation,
};

fn valid_request() -> QrActivationRequest {
    QrActivationRequest {
        token_ref: "qr:activation:synthetic:2026-05-26".into(),
        policy_ref: "policy:qr-activation:current".into(),
        session_ref: "activation-session:synthetic:2026-05-26".into(),
        mode: ActivationMode::ResponderScan,
        runtime_state: ActivationRuntimeState::VerifiedPermit,
        disclosure_scope: ActivationDisclosureScope::EmergencySubsetOnly,
        pointer_contract_passed: true,
        includes_raw_sensitive_payload: false,
        includes_direct_contact_value: false,
        includes_location_trace: false,
        has_disablement_ref: true,
        is_synthetic_fixture: true,
        claims_verified_exochain_activation: false,
    }
}

fn assert_denied(decision: &QrActivationDecision, reason: &str) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn qr_activation_requires_references_and_synthetic_fixtures() {
    let mut request = valid_request();
    request.token_ref = String::new();
    request.policy_ref = String::new();
    request.session_ref = String::new();
    request.is_synthetic_fixture = false;

    let decision = evaluate_qr_activation(request);

    assert_denied(
        &decision,
        "QR activation requires synthetic token, policy, and session references.",
    );
    assert_denied(
        &decision,
        "QR activation fixtures must remain synthetic until a verified runtime activation path exists.",
    );
}

#[test]
fn responder_and_network_activation_require_current_pointer_and_verified_permit() {
    for mode in [
        ActivationMode::ResponderScan,
        ActivationMode::PaceNetworkActivation,
    ] {
        let mut request = valid_request();
        request.mode = mode;
        request.pointer_contract_passed = false;
        request.runtime_state = ActivationRuntimeState::Inactive;

        let decision = evaluate_qr_activation(request);

        assert_denied(
            &decision,
            "QR activation depends on a current QR pointer policy that already passed fail-closed validation.",
        );
        assert_denied(
            &decision,
            "Responder and network activation remain inactive until a verified adapter path returns permit.",
        );
    }
}

#[test]
fn qr_activation_denies_raw_sensitive_contact_and_location_payloads() {
    let mut request = valid_request();
    request.includes_raw_sensitive_payload = true;
    request.includes_direct_contact_value = true;
    request.includes_location_trace = true;

    let decision = evaluate_qr_activation(request);

    assert_denied(
        &decision,
        "QR activation payloads must stay metadata-only and exclude raw sensitive records.",
    );
    assert_denied(
        &decision,
        "QR activation payloads must not embed direct contact values.",
    );
    assert_denied(
        &decision,
        "QR activation payloads must not embed location traces or responder-tracking data.",
    );
}

#[test]
fn qr_activation_blocks_expanded_responder_scope_and_requires_disablement() {
    let mut request = valid_request();
    request.disclosure_scope = ActivationDisclosureScope::ExpandedResponderRequest;
    request.has_disablement_ref = false;

    let decision = evaluate_qr_activation(request);

    assert_denied(
        &decision,
        "Expanded responder disclosure remains blocked for QR activation until Bob approves the live scope.",
    );
    assert_denied(
        &decision,
        "QR activation routes require a disablement or rotation reference before activation can be shown.",
    );
}

#[test]
fn owner_setup_preview_can_remain_inactive_without_verified_claims() {
    let mut request = valid_request();
    request.mode = ActivationMode::OwnerSetupPreview;
    request.runtime_state = ActivationRuntimeState::Inactive;
    request.disclosure_scope = ActivationDisclosureScope::NoResponderDisclosure;

    let decision = evaluate_qr_activation(request);

    assert!(decision.allowed, "{decision:?}");
}

#[test]
fn verified_activation_claims_require_verified_permit_state() {
    let mut request = valid_request();
    request.claims_verified_exochain_activation = true;
    request.runtime_state = ActivationRuntimeState::Unavailable;

    let decision = evaluate_qr_activation(request);

    assert_denied(
        &decision,
        "Verified EXOCHAIN activation claims are blocked unless the activation path is in a verified permit state.",
    );
}
