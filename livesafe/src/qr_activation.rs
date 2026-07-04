use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationMode {
    OwnerSetupPreview,
    ResponderScan,
    PaceNetworkActivation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationRuntimeState {
    Inactive,
    VerifiedPermit,
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationDisclosureScope {
    NoResponderDisclosure,
    EmergencySubsetOnly,
    ExpandedResponderRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QrActivationRequest {
    pub token_ref: String,
    pub policy_ref: String,
    pub session_ref: String,
    pub mode: ActivationMode,
    pub runtime_state: ActivationRuntimeState,
    pub disclosure_scope: ActivationDisclosureScope,
    pub pointer_contract_passed: bool,
    pub includes_raw_sensitive_payload: bool,
    pub includes_direct_contact_value: bool,
    pub includes_location_trace: bool,
    pub has_disablement_ref: bool,
    pub is_synthetic_fixture: bool,
    pub claims_verified_exochain_activation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QrActivationDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_qr_activation(request: QrActivationRequest) -> QrActivationDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.token_ref.trim().is_empty()
        || request.policy_ref.trim().is_empty()
        || request.session_ref.trim().is_empty()
    {
        reasons.insert(
            "QR activation requires synthetic token, policy, and session references.".into(),
        );
        required_evidence.insert(
            "Synthetic activation token reference, current policy reference, and activation session reference."
                .into(),
        );
    }

    if !request.is_synthetic_fixture {
        reasons.insert(
            "QR activation fixtures must remain synthetic until a verified runtime activation path exists."
                .into(),
        );
        required_evidence.insert(
            "Synthetic-only QR activation fixtures until a verified runtime activation path exists."
                .into(),
        );
    }

    if !request.pointer_contract_passed {
        reasons.insert(
            "QR activation depends on a current QR pointer policy that already passed fail-closed validation."
                .into(),
        );
        required_evidence.insert(
            "Passing QR pointer contract evidence for the active activation target.".into(),
        );
    }

    if request.includes_raw_sensitive_payload {
        reasons.insert(
            "QR activation payloads must stay metadata-only and exclude raw sensitive records."
                .into(),
        );
        required_evidence.insert(
            "Activation payload review proving only metadata-safe references are exposed.".into(),
        );
    }

    if request.includes_direct_contact_value {
        reasons.insert("QR activation payloads must not embed direct contact values.".into());
        required_evidence.insert(
            "Configuration-backed contact presentation instead of QR-embedded contact values."
                .into(),
        );
    }

    if request.includes_location_trace {
        reasons.insert(
            "QR activation payloads must not embed location traces or responder-tracking data."
                .into(),
        );
        required_evidence.insert(
            "Scan-time policy review proving location traces remain outside the activation payload."
                .into(),
        );
    }

    if !request.has_disablement_ref {
        reasons.insert(
            "QR activation routes require a disablement or rotation reference before activation can be shown."
                .into(),
        );
        required_evidence.insert(
            "Disablement or rotation reference covering QR activation targets and landing behavior."
                .into(),
        );
    }

    if request.disclosure_scope == ActivationDisclosureScope::ExpandedResponderRequest {
        reasons.insert(
            "Expanded responder disclosure remains blocked for QR activation until Bob approves the live scope."
                .into(),
        );
        required_evidence.insert(
            "Bob-approved live responder disclosure scope before expanded QR activation output."
                .into(),
        );
    }

    if matches!(
        request.mode,
        ActivationMode::ResponderScan | ActivationMode::PaceNetworkActivation
    ) && request.runtime_state != ActivationRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Responder and network activation remain inactive until a verified adapter path returns permit."
                .into(),
        );
        required_evidence.insert(
            "Verified adapter path returning permit for responder or network QR activation.".into(),
        );
    }

    if request.mode == ActivationMode::ResponderScan
        && request.disclosure_scope != ActivationDisclosureScope::EmergencySubsetOnly
    {
        reasons.insert(
            "Responder QR activation is limited to the approved emergency subset until broader policy is approved."
                .into(),
        );
        required_evidence
            .insert("Emergency-subset-only responder landing contract for QR activation.".into());
    }

    if request.claims_verified_exochain_activation
        && request.runtime_state != ActivationRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Verified EXOCHAIN activation claims are blocked unless the activation path is in a verified permit state."
                .into(),
        );
        required_evidence.insert(
            "Verified permit-state evidence before any EXOCHAIN-backed activation claim is shown."
                .into(),
        );
    }

    QrActivationDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
