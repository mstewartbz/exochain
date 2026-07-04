use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponderAccessDisplaySurface {
    ResponderWebView,
    QrScanLanding,
    ApiPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponderAccessRuntimeState {
    Inactive,
    VerifiedPermit,
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponderAccessDisplayScope {
    EmergencySubset,
    ExpandedResponderRequest,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResponderDisplayPanel {
    EmergencyIdentity,
    EmergencyMedicalSummary,
    QrActivationStatus,
    VitalLockEmergencyBadge,
    PaceContactSummary,
    FullVaultExport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponderAccessDisplayRequest {
    pub responder_session_ref: String,
    pub policy_ref: String,
    pub disablement_ref: String,
    pub surface: ResponderAccessDisplaySurface,
    pub runtime_state: ResponderAccessRuntimeState,
    pub scope: ResponderAccessDisplayScope,
    pub visible_panels: Vec<ResponderDisplayPanel>,
    pub emergency_profile_contract_passed: bool,
    pub qr_activation_contract_passed: bool,
    pub vitallock_vault_contract_passed: bool,
    pub accessible_label_present: bool,
    pub machine_state_present: bool,
    pub synthetic_fixture_only: bool,
    pub includes_raw_sensitive_payload: bool,
    pub includes_direct_contact_value: bool,
    pub includes_location_trace: bool,
    pub claims_verified_responder_access: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponderAccessDisplayDecision {
    pub allowed: bool,
    pub allowed_panels: Vec<ResponderDisplayPanel>,
    pub display_text: &'static str,
    pub machine_state: &'static str,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_responder_access_display(
    request: ResponderAccessDisplayRequest,
) -> ResponderAccessDisplayDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();
    let mut allowed_panels = BTreeSet::new();

    if request.responder_session_ref.trim().is_empty()
        || request.policy_ref.trim().is_empty()
        || request.disablement_ref.trim().is_empty()
    {
        reasons.insert(
            "Responder access displays require synthetic session, policy, and disablement references."
                .to_string(),
        );
        required_evidence.insert(
            "Synthetic responder session reference, current policy reference, and disablement reference for every responder display."
                .to_string(),
        );
    }

    if !request.accessible_label_present {
        reasons.insert(
            "Responder access displays require an accessible label equivalent to the visible responder status."
                .to_string(),
        );
        required_evidence.insert(
            "Accessible responder-status label bound to the rendered responder display."
                .to_string(),
        );
    }

    if !request.machine_state_present {
        reasons.insert(
            "Responder access displays require the canonical machine-readable responder state."
                .to_string(),
        );
        required_evidence.insert(
            "Canonical machine-readable responder-access state in every responder display."
                .to_string(),
        );
    }

    if !request.synthetic_fixture_only {
        reasons.insert(
            "Responder access displays must remain synthetic until a verified responder runtime path exists."
                .to_string(),
        );
        required_evidence.insert(
            "Synthetic-only responder display fixtures until a verified responder runtime path exists."
                .to_string(),
        );
    }

    if request.includes_raw_sensitive_payload {
        reasons
            .insert("Responder access displays must not embed raw sensitive payloads.".to_string());
        required_evidence.insert(
            "Responder display review proving only metadata-safe emergency summaries are shown."
                .to_string(),
        );
    }

    if request.includes_direct_contact_value {
        reasons
            .insert("Responder access displays must not embed direct contact values.".to_string());
        required_evidence.insert(
            "Configuration-backed contact handoff instead of direct contact values inside responder displays."
                .to_string(),
        );
    }

    if request.includes_location_trace {
        reasons.insert(
            "Responder access displays must not embed location traces or responder-tracking data."
                .to_string(),
        );
        required_evidence.insert(
            "Responder display payload review proving location traces remain outside the responder surface."
                .to_string(),
        );
    }

    if request.scope == ResponderAccessDisplayScope::ExpandedResponderRequest {
        reasons.insert(
            "Expanded responder access displays remain blocked until Bob approves the live responder-access scope."
                .to_string(),
        );
        required_evidence.insert(
            "Bob-approved live responder-access scope before any expanded responder display can activate."
                .to_string(),
        );
    }

    if has_emergency_profile_panel(&request.visible_panels)
        && !request.emergency_profile_contract_passed
    {
        reasons.insert(
            "Responder access displays require a passing emergency-profile contract for emergency-profile panels."
                .to_string(),
        );
        required_evidence.insert(
            "Passing emergency-profile contract evidence for the responder-visible emergency subset."
                .to_string(),
        );
    }

    if request
        .visible_panels
        .contains(&ResponderDisplayPanel::QrActivationStatus)
        && !request.qr_activation_contract_passed
    {
        reasons.insert(
            "Responder access displays require a passing QR activation contract for QR-linked responder panels."
                .to_string(),
        );
        required_evidence.insert(
            "Passing QR activation contract evidence for responder-visible QR status panels."
                .to_string(),
        );
    }

    if request
        .visible_panels
        .contains(&ResponderDisplayPanel::VitalLockEmergencyBadge)
        && !request.vitallock_vault_contract_passed
    {
        reasons.insert(
            "Responder access displays require a passing VitalLock vault contract for responder vault panels."
                .to_string(),
        );
        required_evidence.insert(
            "Passing VitalLock vault contract evidence for any responder-facing vault status panel."
                .to_string(),
        );
    }

    for panel in &request.visible_panels {
        if emergency_subset_panels().contains(panel) {
            allowed_panels.insert(*panel);
        } else {
            reasons.insert(
                "Responder access displays exclude panels outside the approved emergency subset."
                    .to_string(),
            );
            required_evidence.insert(
                "Approved responder emergency-subset panel inventory with explicit exclusions for P.A.C.E. contacts and full vault export."
                    .to_string(),
            );
        }
    }

    if request.claims_verified_responder_access
        && request.runtime_state != ResponderAccessRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Verified responder-access claims are blocked unless the display path is in a verified permit state."
                .to_string(),
        );
        required_evidence.insert(
            "Verified permit-state evidence before any responder-access verification claim is shown."
                .to_string(),
        );
    }

    let (display_text, machine_state) = responder_status_tokens(request.runtime_state);

    if matches!(
        request.surface,
        ResponderAccessDisplaySurface::ApiPayload | ResponderAccessDisplaySurface::QrScanLanding
    ) {
        required_evidence.insert(
            format!(
                "Responder display surface {:?} preserves the canonical inactive-or-verified status tokens.",
                request.surface
            ),
        );
    }

    ResponderAccessDisplayDecision {
        allowed: reasons.is_empty(),
        allowed_panels: allowed_panels.into_iter().collect(),
        display_text,
        machine_state,
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn emergency_subset_panels() -> BTreeSet<ResponderDisplayPanel> {
    BTreeSet::from([
        ResponderDisplayPanel::EmergencyIdentity,
        ResponderDisplayPanel::EmergencyMedicalSummary,
        ResponderDisplayPanel::QrActivationStatus,
        ResponderDisplayPanel::VitalLockEmergencyBadge,
    ])
}

fn has_emergency_profile_panel(panels: &[ResponderDisplayPanel]) -> bool {
    panels.contains(&ResponderDisplayPanel::EmergencyIdentity)
        || panels.contains(&ResponderDisplayPanel::EmergencyMedicalSummary)
}

fn responder_status_tokens(
    runtime_state: ResponderAccessRuntimeState,
) -> (&'static str, &'static str) {
    match runtime_state {
        ResponderAccessRuntimeState::Inactive => (
            "RESPONDER ACCESS NOT YET VERIFIED",
            "responder_access_inactive",
        ),
        ResponderAccessRuntimeState::VerifiedPermit => {
            ("RESPONDER ACCESS VERIFIED", "responder_access_verified")
        }
        ResponderAccessRuntimeState::Denied => {
            ("RESPONDER ACCESS DENIED", "responder_access_denied")
        }
        ResponderAccessRuntimeState::Unavailable => (
            "RESPONDER ACCESS UNAVAILABLE",
            "responder_access_unavailable",
        ),
    }
}
