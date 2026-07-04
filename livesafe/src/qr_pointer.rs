use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QrPurpose {
    EmergencyAccess,
    ActivationOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointStatus {
    Current,
    Expired,
    Replaced,
    Revoked,
    Malformed,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QrPointerContract {
    pub token_ref: String,
    pub endpoint_ref: String,
    pub purpose: QrPurpose,
    pub endpoint_status: EndpointStatus,
    pub policy_ref: String,
    pub generated_at_ref: String,
    pub includes_raw_sensitive_payload: bool,
    pub includes_direct_contact_value: bool,
    pub includes_location_trace: bool,
    pub rotation_ref_present: bool,
    pub is_synthetic_fixture: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QrPointerDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_qr_pointer(pointer: &QrPointerContract) -> QrPointerDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if pointer.token_ref.trim().is_empty()
        || pointer.endpoint_ref.trim().is_empty()
        || pointer.policy_ref.trim().is_empty()
        || pointer.generated_at_ref.trim().is_empty()
    {
        reasons.insert(
            "QR pointers require synthetic token, endpoint, policy, and generation references."
                .to_string(),
        );
        required_evidence.insert(
            "Synthetic QR token metadata, configuration-backed endpoint reference, server-policy reference, and generation reference."
                .to_string(),
        );
    }

    if !pointer.is_synthetic_fixture {
        reasons.insert(
            "QR pointer fixtures must remain synthetic until runtime activation exists."
                .to_string(),
        );
        required_evidence.insert(
            "Synthetic-only QR pointer fixtures until a verified runtime activation path exists."
                .to_string(),
        );
    }

    if pointer.includes_raw_sensitive_payload {
        reasons.insert(
            "QR payloads must contain only retrieval or activation pointers, never raw sensitive data."
                .to_string(),
        );
        required_evidence.insert(
            "Payload review proving the QR only carries an activation or retrieval pointer."
                .to_string(),
        );
    }

    if pointer.includes_direct_contact_value {
        reasons.insert(
            "QR payloads must not embed direct contact values; printed and scan-time contact details stay configuration-backed."
                .to_string(),
        );
        required_evidence.insert(
            "Configuration-backed contact presentation instead of QR-embedded contact values."
                .to_string(),
        );
    }

    if pointer.includes_location_trace {
        reasons.insert(
            "QR payloads must not embed location traces or responder-tracking data.".to_string(),
        );
        required_evidence.insert(
            "Scan-time policy review proving location traces remain outside the QR payload."
                .to_string(),
        );
    }

    if pointer.endpoint_status != EndpointStatus::Current {
        reasons.insert(
            "QR pointers must resolve through a current server-side access policy; expired, replaced, revoked, malformed, or unknown targets are denied."
                .to_string(),
        );
        required_evidence.insert(
            "Current server-side access-policy validation for the active QR endpoint target."
                .to_string(),
        );
    }

    if !pointer.rotation_ref_present
        && (pointer.endpoint_status != EndpointStatus::Current
            || matches!(pointer.purpose, QrPurpose::ActivationOnly))
    {
        reasons.insert(
            "QR pointers require a disablement or rotation reference when endpoint targets change."
                .to_string(),
        );
        required_evidence.insert(
            "Disablement or rotation reference for QR targets when policy or endpoint state changes."
                .to_string(),
        );
    }

    QrPointerDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
