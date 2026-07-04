use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterDependencyState {
    NotWired,
    Available,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExochainAdapterResponse {
    Permit,
    Deny,
    Rejected,
    Timeout,
    Unavailable,
    NotCalled,
    Stale,
    Revoked,
    Contradicted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterCredentialState {
    WellFormed,
    Malformed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SensitivePayloadHandling {
    MetadataOnly,
    RawSensitivePayload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterActivationRequest {
    pub adapter_dependency_state: AdapterDependencyState,
    pub exochain_response: ExochainAdapterResponse,
    pub credential_state: AdapterCredentialState,
    pub signature_state: AdapterCredentialState,
    pub consent_record_state: AdapterCredentialState,
    pub authority_chain_state: AdapterCredentialState,
    pub provenance_record_state: AdapterCredentialState,
    pub custody_receipt_state: AdapterCredentialState,
    pub tenant_identifier_state: AdapterCredentialState,
    pub emergency_access_grant_state: AdapterCredentialState,
    pub sensitive_payload_handling: SensitivePayloadHandling,
    pub status_routes_redacted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterActivationDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_adapter_activation(request: AdapterActivationRequest) -> AdapterActivationDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.adapter_dependency_state != AdapterDependencyState::Available {
        reasons.insert("Adapter activation requires a wired EXOCHAIN dependency surface.".into());
        required_evidence
            .insert("Verified LiveSafe adapter path invoking the EXOCHAIN dependency.".into());
    }

    if request.exochain_response != ExochainAdapterResponse::Permit {
        reasons.insert(
            "EXOCHAIN adapter activation must fail closed unless EXOCHAIN returns permit.".into(),
        );
        required_evidence.insert(
            "Denied, rejected, timeout, unavailable, not-called, stale, revoked, and contradicted adapter regression tests.".into(),
        );
    }

    if has_malformed_authority_input(&request) {
        reasons.insert("Credentials, signatures, consent, authority, provenance, custody, tenant, and emergency access grants must be well formed before adapter activation.".into());
        required_evidence.insert(
            "Adapter input validation for credentials, signatures, consent, authority, provenance, custody, tenant, and emergency access grants."
                .into(),
        );
    }

    if request.sensitive_payload_handling == SensitivePayloadHandling::RawSensitivePayload {
        reasons.insert(
            "Adapter activation cannot carry raw sensitive payloads on-chain or in receipt paths."
                .into(),
        );
        required_evidence.insert(
            "Receipt boundary proving commitments, references, policy ids, and hashes only.".into(),
        );
    }

    if !request.status_routes_redacted {
        reasons.insert(
            "Health, status, debug, telemetry, and error routes must redact secrets and raw sensitive records."
                .into(),
        );
        required_evidence.insert(
            "Redaction coverage for health, status, debug, telemetry, and error routes.".into(),
        );
    }

    AdapterActivationDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn has_malformed_authority_input(request: &AdapterActivationRequest) -> bool {
    [
        request.credential_state,
        request.signature_state,
        request.consent_record_state,
        request.authority_chain_state,
        request.provenance_record_state,
        request.custody_receipt_state,
        request.tenant_identifier_state,
        request.emergency_access_grant_state,
    ]
    .into_iter()
    .any(|state| state == AdapterCredentialState::Malformed)
}
