use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmbientSignalMode {
    OwnerPreview,
    ContextPackDispatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmbientRuntimeState {
    Inactive,
    VerifiedPermit,
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmbientVisibility {
    MetadataOnly,
    RecipientVisibleSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmbientSignalRequest {
    pub signal_ref: String,
    pub policy_ref: String,
    pub session_ref: String,
    pub mode: AmbientSignalMode,
    pub runtime_state: AmbientRuntimeState,
    pub visibility: AmbientVisibility,
    pub marketplace_template_declared: bool,
    pub consent_acknowledged: bool,
    pub includes_raw_sensitive_payload: bool,
    pub includes_direct_contact_value: bool,
    pub includes_location_trace: bool,
    pub has_disablement_ref: bool,
    pub is_synthetic_fixture: bool,
    pub claims_verified_ambient_trust: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmbientSignalDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_ambient_signal(request: AmbientSignalRequest) -> AmbientSignalDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.signal_ref.trim().is_empty()
        || request.policy_ref.trim().is_empty()
        || request.session_ref.trim().is_empty()
    {
        reasons.insert(
            "Ambient signals require synthetic signal, policy, and session references.".into(),
        );
        required_evidence
            .insert("Synthetic signal reference, policy reference, and session reference.".into());
    }

    if !request.is_synthetic_fixture {
        reasons.insert(
            "Ambient signal fixtures must remain synthetic until a verified runtime path exists."
                .into(),
        );
        required_evidence.insert(
            "Synthetic-only Ambient signal fixtures until a verified runtime path exists.".into(),
        );
    }

    if request.includes_raw_sensitive_payload {
        reasons.insert(
            "Ambient signal payloads must stay metadata-only and exclude raw sensitive records."
                .into(),
        );
        required_evidence.insert(
            "Payload review proving Ambient signal delivery contains only metadata-safe references."
                .into(),
        );
    }

    if request.includes_direct_contact_value {
        reasons.insert("Ambient signal payloads must not embed direct contact values.".into());
        required_evidence.insert(
            "Configuration-backed contact presentation instead of direct contact values in Ambient signals."
                .into(),
        );
    }

    if request.includes_location_trace {
        reasons.insert(
            "Ambient signal payloads must not embed location traces or responder-tracking data."
                .into(),
        );
        required_evidence.insert(
            "Policy review proving location traces remain outside Ambient signal payloads.".into(),
        );
    }

    if request.mode == AmbientSignalMode::ContextPackDispatch
        && !request.marketplace_template_declared
    {
        reasons.insert("Ambient context dispatch requires a declared marketplace template.".into());
        required_evidence.insert(
            "Ambient marketplace template evidence declaring rule scope, plan gate, consent, audit, and disablement."
                .into(),
        );
    }

    if request.mode == AmbientSignalMode::ContextPackDispatch && !request.consent_acknowledged {
        reasons.insert(
            "Ambient context dispatch requires acknowledged Ambient signal consent.".into(),
        );
        required_evidence.insert(
            "Ambient signal acknowledgement evidence before context dispatch can activate.".into(),
        );
    }

    if request.visibility == AmbientVisibility::RecipientVisibleSummary
        && request.runtime_state != AmbientRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Recipient-visible Ambient signals remain inactive until a verified adapter path returns permit."
                .into(),
        );
        required_evidence.insert(
            "Verified adapter path returning permit before recipient-visible Ambient delivery can activate."
                .into(),
        );
    }

    if request.visibility == AmbientVisibility::RecipientVisibleSummary
        && !request.has_disablement_ref
    {
        reasons.insert(
            "Ambient signal routes require a disablement reference before recipient-visible delivery can be shown."
                .into(),
        );
        required_evidence.insert(
            "Disablement reference covering recipient-visible Ambient signal behavior.".into(),
        );
    }

    if request.claims_verified_ambient_trust
        && request.runtime_state != AmbientRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Verified Ambient trust claims are blocked unless the signal path is in a verified permit state."
                .into(),
        );
        required_evidence.insert(
            "Verified permit-state evidence before any Ambient trust claim is shown.".into(),
        );
    }

    AmbientSignalDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
