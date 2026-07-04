use livesafe::exochain_adapter_activation::{
    AdapterActivationDecision, AdapterActivationRequest, AdapterCredentialState,
    AdapterDependencyState, ExochainAdapterResponse, SensitivePayloadHandling,
    evaluate_adapter_activation,
};

fn assert_denied(decision: &AdapterActivationDecision, reason: &str) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn adapter_activation_allows_only_permit_with_safe_boundaries() {
    let decision = evaluate_adapter_activation(AdapterActivationRequest {
        adapter_dependency_state: AdapterDependencyState::Available,
        exochain_response: ExochainAdapterResponse::Permit,
        credential_state: AdapterCredentialState::WellFormed,
        signature_state: AdapterCredentialState::WellFormed,
        consent_record_state: AdapterCredentialState::WellFormed,
        authority_chain_state: AdapterCredentialState::WellFormed,
        provenance_record_state: AdapterCredentialState::WellFormed,
        custody_receipt_state: AdapterCredentialState::WellFormed,
        tenant_identifier_state: AdapterCredentialState::WellFormed,
        emergency_access_grant_state: AdapterCredentialState::WellFormed,
        sensitive_payload_handling: SensitivePayloadHandling::MetadataOnly,
        status_routes_redacted: true,
    });

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
    assert_eq!(decision.required_evidence, Vec::<String>::new());
}

#[test]
fn adapter_activation_fails_closed_for_denied_rejected_timeout_unavailable_and_not_called_paths() {
    for response in [
        ExochainAdapterResponse::Deny,
        ExochainAdapterResponse::Rejected,
        ExochainAdapterResponse::Timeout,
        ExochainAdapterResponse::Unavailable,
        ExochainAdapterResponse::NotCalled,
    ] {
        let decision = evaluate_adapter_activation(AdapterActivationRequest {
            adapter_dependency_state: AdapterDependencyState::Available,
            exochain_response: response,
            credential_state: AdapterCredentialState::WellFormed,
            signature_state: AdapterCredentialState::WellFormed,
            consent_record_state: AdapterCredentialState::WellFormed,
            authority_chain_state: AdapterCredentialState::WellFormed,
            provenance_record_state: AdapterCredentialState::WellFormed,
            custody_receipt_state: AdapterCredentialState::WellFormed,
            tenant_identifier_state: AdapterCredentialState::WellFormed,
            emergency_access_grant_state: AdapterCredentialState::WellFormed,
            sensitive_payload_handling: SensitivePayloadHandling::MetadataOnly,
            status_routes_redacted: true,
        });

        assert_denied(
            &decision,
            "EXOCHAIN adapter activation must fail closed unless EXOCHAIN returns permit.",
        );
    }
}

#[test]
fn adapter_activation_rejects_malformed_authority_inputs() {
    let decision = evaluate_adapter_activation(AdapterActivationRequest {
        adapter_dependency_state: AdapterDependencyState::Available,
        exochain_response: ExochainAdapterResponse::Permit,
        credential_state: AdapterCredentialState::Malformed,
        signature_state: AdapterCredentialState::Malformed,
        consent_record_state: AdapterCredentialState::Malformed,
        authority_chain_state: AdapterCredentialState::Malformed,
        provenance_record_state: AdapterCredentialState::Malformed,
        custody_receipt_state: AdapterCredentialState::Malformed,
        tenant_identifier_state: AdapterCredentialState::Malformed,
        emergency_access_grant_state: AdapterCredentialState::Malformed,
        sensitive_payload_handling: SensitivePayloadHandling::MetadataOnly,
        status_routes_redacted: true,
    });

    assert_denied(
        &decision,
        "Credentials, signatures, consent, authority, provenance, custody, tenant, and emergency access grants must be well formed before adapter activation.",
    );
}

#[test]
fn adapter_activation_blocks_raw_sensitive_payloads_and_status_route_leaks() {
    let decision = evaluate_adapter_activation(AdapterActivationRequest {
        adapter_dependency_state: AdapterDependencyState::Available,
        exochain_response: ExochainAdapterResponse::Permit,
        credential_state: AdapterCredentialState::WellFormed,
        signature_state: AdapterCredentialState::WellFormed,
        consent_record_state: AdapterCredentialState::WellFormed,
        authority_chain_state: AdapterCredentialState::WellFormed,
        provenance_record_state: AdapterCredentialState::WellFormed,
        custody_receipt_state: AdapterCredentialState::WellFormed,
        tenant_identifier_state: AdapterCredentialState::WellFormed,
        emergency_access_grant_state: AdapterCredentialState::WellFormed,
        sensitive_payload_handling: SensitivePayloadHandling::RawSensitivePayload,
        status_routes_redacted: false,
    });

    assert_denied(
        &decision,
        "Adapter activation cannot carry raw sensitive payloads on-chain or in receipt paths.",
    );
    assert_denied(
        &decision,
        "Health, status, debug, telemetry, and error routes must redact secrets and raw sensitive records.",
    );
}

#[test]
fn adapter_activation_requires_a_real_dependency_surface() {
    let decision = evaluate_adapter_activation(AdapterActivationRequest {
        adapter_dependency_state: AdapterDependencyState::NotWired,
        exochain_response: ExochainAdapterResponse::NotCalled,
        credential_state: AdapterCredentialState::WellFormed,
        signature_state: AdapterCredentialState::WellFormed,
        consent_record_state: AdapterCredentialState::WellFormed,
        authority_chain_state: AdapterCredentialState::WellFormed,
        provenance_record_state: AdapterCredentialState::WellFormed,
        custody_receipt_state: AdapterCredentialState::WellFormed,
        tenant_identifier_state: AdapterCredentialState::WellFormed,
        emergency_access_grant_state: AdapterCredentialState::WellFormed,
        sensitive_payload_handling: SensitivePayloadHandling::MetadataOnly,
        status_routes_redacted: true,
    });

    assert_denied(
        &decision,
        "Adapter activation requires a wired EXOCHAIN dependency surface.",
    );
}
