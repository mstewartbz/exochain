use livesafe::qr_pointer::{
    EndpointStatus, QrPointerContract, QrPointerDecision, QrPurpose, evaluate_qr_pointer,
};

fn valid_pointer() -> QrPointerContract {
    QrPointerContract {
        token_ref: "qr:token:synthetic:2026-05-25".into(),
        endpoint_ref: "config:endpoint:responder-scan".into(),
        purpose: QrPurpose::EmergencyAccess,
        endpoint_status: EndpointStatus::Current,
        policy_ref: "policy:responder-access:current".into(),
        generated_at_ref: "card-version:2026-05-25".into(),
        includes_raw_sensitive_payload: false,
        includes_direct_contact_value: false,
        includes_location_trace: false,
        rotation_ref_present: true,
        is_synthetic_fixture: true,
    }
}

fn reasons(decision: &QrPointerDecision) -> &[String] {
    &decision.reasons
}

#[test]
fn qr_pointer_requires_synthetic_metadata_and_current_server_policy() {
    let mut pointer = valid_pointer();
    pointer.token_ref = String::new();
    pointer.endpoint_ref = String::new();
    pointer.policy_ref = String::new();
    pointer.generated_at_ref = String::new();
    pointer.is_synthetic_fixture = false;

    let decision = evaluate_qr_pointer(&pointer);

    assert!(!decision.allowed);
    assert!(reasons(&decision).contains(
        &"QR pointers require synthetic token, endpoint, policy, and generation references.".into()
    ));
    assert!(reasons(&decision).contains(
        &"QR pointer fixtures must remain synthetic until runtime activation exists.".into()
    ));
}

#[test]
fn qr_pointer_denies_raw_sensitive_payloads_contact_values_and_location_traces() {
    let mut pointer = valid_pointer();
    pointer.includes_raw_sensitive_payload = true;
    pointer.includes_direct_contact_value = true;
    pointer.includes_location_trace = true;

    let decision = evaluate_qr_pointer(&pointer);

    assert!(!decision.allowed);
    assert!(reasons(&decision).contains(
        &"QR payloads must contain only retrieval or activation pointers, never raw sensitive data.".into()
    ));
    assert!(reasons(&decision).contains(
        &"QR payloads must not embed direct contact values; printed and scan-time contact details stay configuration-backed.".into()
    ));
    assert!(reasons(&decision).contains(
        &"QR payloads must not embed location traces or responder-tracking data.".into()
    ));
}

#[test]
fn qr_pointer_denies_non_current_or_malformed_targets_and_requires_rotation_path() {
    for status in [
        EndpointStatus::Expired,
        EndpointStatus::Replaced,
        EndpointStatus::Revoked,
        EndpointStatus::Malformed,
        EndpointStatus::Unknown,
    ] {
        let mut pointer = valid_pointer();
        pointer.endpoint_status = status;
        pointer.rotation_ref_present = false;

        let decision = evaluate_qr_pointer(&pointer);

        assert!(!decision.allowed, "{status:?} should fail closed");
        assert!(reasons(&decision).contains(
            &"QR pointers must resolve through a current server-side access policy; expired, replaced, revoked, malformed, or unknown targets are denied.".into()
        ));
        assert!(reasons(&decision).contains(
            &"QR pointers require a disablement or rotation reference when endpoint targets change.".into()
        ));
    }
}

#[test]
fn valid_qr_pointer_allows_current_activation_metadata_only() {
    let decision = evaluate_qr_pointer(&valid_pointer());

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
    assert_eq!(decision.required_evidence, Vec::<String>::new());
}
