use livesafe::responder_access_display::{
    ResponderAccessDisplayRequest, ResponderAccessDisplayScope, ResponderAccessDisplaySurface,
    ResponderAccessRuntimeState, ResponderDisplayPanel, evaluate_responder_access_display,
};

fn valid_request() -> ResponderAccessDisplayRequest {
    ResponderAccessDisplayRequest {
        responder_session_ref: "responder-session:synthetic".into(),
        policy_ref: "policy:responder-access-display:current".into(),
        disablement_ref: "disablement:responder-access-display:current".into(),
        surface: ResponderAccessDisplaySurface::ResponderWebView,
        runtime_state: ResponderAccessRuntimeState::Inactive,
        scope: ResponderAccessDisplayScope::EmergencySubset,
        visible_panels: vec![
            ResponderDisplayPanel::EmergencyIdentity,
            ResponderDisplayPanel::EmergencyMedicalSummary,
            ResponderDisplayPanel::QrActivationStatus,
        ],
        emergency_profile_contract_passed: true,
        qr_activation_contract_passed: true,
        vitallock_vault_contract_passed: false,
        accessible_label_present: true,
        machine_state_present: true,
        synthetic_fixture_only: true,
        includes_raw_sensitive_payload: false,
        includes_direct_contact_value: false,
        includes_location_trace: false,
        claims_verified_responder_access: false,
    }
}

fn assert_denied(
    decision: &livesafe::responder_access_display::ResponderAccessDisplayDecision,
    reason: &str,
) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn responder_display_requires_references_accessibility_and_machine_state() {
    let mut request = valid_request();
    request.responder_session_ref = String::new();
    request.policy_ref = String::new();
    request.disablement_ref = String::new();
    request.accessible_label_present = false;
    request.machine_state_present = false;

    let decision = evaluate_responder_access_display(request);

    assert_denied(
        &decision,
        "Responder access displays require synthetic session, policy, and disablement references.",
    );
    assert_denied(
        &decision,
        "Responder access displays require an accessible label equivalent to the visible responder status.",
    );
    assert_denied(
        &decision,
        "Responder access displays require the canonical machine-readable responder state.",
    );
}

#[test]
fn responder_display_denies_non_synthetic_and_sensitive_payloads() {
    let mut request = valid_request();
    request.synthetic_fixture_only = false;
    request.includes_raw_sensitive_payload = true;
    request.includes_direct_contact_value = true;
    request.includes_location_trace = true;

    let decision = evaluate_responder_access_display(request);

    assert_denied(
        &decision,
        "Responder access displays must remain synthetic until a verified responder runtime path exists.",
    );
    assert_denied(
        &decision,
        "Responder access displays must not embed raw sensitive payloads.",
    );
    assert_denied(
        &decision,
        "Responder access displays must not embed direct contact values.",
    );
    assert_denied(
        &decision,
        "Responder access displays must not embed location traces or responder-tracking data.",
    );
}

#[test]
fn responder_display_allows_inactive_emergency_subset_without_verified_claims() {
    let decision = evaluate_responder_access_display(valid_request());

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.machine_state, "responder_access_inactive");
    assert_eq!(decision.display_text, "RESPONDER ACCESS NOT YET VERIFIED");
    assert_eq!(
        decision.allowed_panels,
        vec![
            ResponderDisplayPanel::EmergencyIdentity,
            ResponderDisplayPanel::EmergencyMedicalSummary,
            ResponderDisplayPanel::QrActivationStatus,
        ]
    );
}

#[test]
fn responder_display_blocks_expanded_scope_and_unapproved_panels() {
    let mut request = valid_request();
    request.scope = ResponderAccessDisplayScope::ExpandedResponderRequest;
    request.visible_panels = vec![
        ResponderDisplayPanel::EmergencyIdentity,
        ResponderDisplayPanel::PaceContactSummary,
        ResponderDisplayPanel::FullVaultExport,
    ];

    let decision = evaluate_responder_access_display(request);

    assert_denied(
        &decision,
        "Expanded responder access displays remain blocked until Bob approves the live responder-access scope.",
    );
    assert_denied(
        &decision,
        "Responder access displays exclude panels outside the approved emergency subset.",
    );
}

#[test]
fn responder_display_requires_underlying_contracts_and_verified_state_for_claims() {
    let mut request = valid_request();
    request.visible_panels = vec![
        ResponderDisplayPanel::EmergencyIdentity,
        ResponderDisplayPanel::EmergencyMedicalSummary,
        ResponderDisplayPanel::QrActivationStatus,
        ResponderDisplayPanel::VitalLockEmergencyBadge,
    ];
    request.qr_activation_contract_passed = false;
    request.vitallock_vault_contract_passed = false;
    request.claims_verified_responder_access = true;

    let decision = evaluate_responder_access_display(request);

    assert_denied(
        &decision,
        "Responder access displays require a passing QR activation contract for QR-linked responder panels.",
    );
    assert_denied(
        &decision,
        "Responder access displays require a passing VitalLock vault contract for responder vault panels.",
    );
    assert_denied(
        &decision,
        "Verified responder-access claims are blocked unless the display path is in a verified permit state.",
    );
}
