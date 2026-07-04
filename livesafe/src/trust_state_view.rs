use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustState {
    NotVerified,
    GenesisPending,
    InternalProof,
    ExternallyVerified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustSurface {
    InternalConsole,
    PrivateReview,
    CustomerPortal,
    PublicWebsite,
    PrintedCard,
    ApiResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustVisualColor {
    Red,
    Yellow,
    Blue,
    Green,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustVisualIcon {
    LockOpen,
    LockClock,
    ShieldCheck,
    LockCheck,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustStateView {
    pub state: TrustState,
    pub badge_text: &'static str,
    pub icon: TrustVisualIcon,
    pub color: TrustVisualColor,
    pub css_class: &'static str,
    pub glow_class: &'static str,
    pub display_text: &'static str,
    pub machine_state: &'static str,
    pub external_claim_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustStateViewRequest {
    pub state: TrustState,
    pub surface: TrustSurface,
    pub includes_trust_bearing_claim: bool,
    pub internal_proof_complete: bool,
    pub frost_genesis_complete: bool,
    pub verified_runtime_adapter: bool,
    pub accessible_label_present: bool,
    pub machine_state_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustStateViewDecision {
    pub allowed: bool,
    pub view: Option<TrustStateView>,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_trust_state_view(request: TrustStateViewRequest) -> TrustStateViewDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();
    let proof_gates_complete = request.internal_proof_complete
        && request.frost_genesis_complete
        && request.verified_runtime_adapter;

    if !request.accessible_label_present {
        reasons.insert(
            "Trust-state displays require an accessible label equivalent to the visible status."
                .to_string(),
        );
        required_evidence.insert(
            "Accessible status label bound to the canonical trust-state display.".to_string(),
        );
    }

    if !request.machine_state_present {
        reasons.insert(
            "Trust-state displays require the canonical machine-readable state.".to_string(),
        );
        required_evidence.insert(
            "Canonical machine-readable trust-state field in every rendered view.".to_string(),
        );
    }

    if request.includes_trust_bearing_claim
        && is_public_surface(request.surface)
        && request.state != TrustState::ExternallyVerified
    {
        reasons.insert(
            "Public trust-bearing claims are blocked unless the state is externally verified."
                .to_string(),
        );
        required_evidence.insert(
            "Externally verified state before any public trust-bearing claim is shown.".to_string(),
        );
    }

    if request.state == TrustState::ExternallyVerified && !proof_gates_complete {
        reasons.insert(
            "Externally verified trust display requires completed internal proof, completed genesis ceremony, and a verified runtime adapter."
                .to_string(),
        );
        required_evidence.insert(
            "Internal proof record, completed genesis ceremony evidence, and verified runtime adapter test evidence."
                .to_string(),
        );
    }

    TrustStateViewDecision {
        allowed: reasons.is_empty(),
        view: Some(trust_state_view(request.state, proof_gates_complete)),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn is_public_surface(surface: TrustSurface) -> bool {
    matches!(
        surface,
        TrustSurface::CustomerPortal
            | TrustSurface::PublicWebsite
            | TrustSurface::PrintedCard
            | TrustSurface::ApiResponse
    )
}

fn trust_state_view(state: TrustState, proof_gates_complete: bool) -> TrustStateView {
    match state {
        TrustState::NotVerified => TrustStateView {
            state,
            badge_text: "AVC",
            icon: TrustVisualIcon::LockOpen,
            color: TrustVisualColor::Red,
            css_class: "trust-signal trust-signal--red trust-signal--not-verified",
            glow_class: "trust-glow trust-glow--red",
            display_text: "THIS IS NOT YET VERIFIED",
            machine_state: "not_verified",
            external_claim_allowed: false,
        },
        TrustState::GenesisPending => TrustStateView {
            state,
            badge_text: "AVC",
            icon: TrustVisualIcon::LockClock,
            color: TrustVisualColor::Yellow,
            css_class: "trust-signal trust-signal--yellow trust-signal--genesis-pending",
            glow_class: "trust-glow trust-glow--yellow",
            display_text: "GENESIS VERIFICATION PENDING",
            machine_state: "genesis_pending",
            external_claim_allowed: false,
        },
        TrustState::InternalProof => TrustStateView {
            state,
            badge_text: "AVC",
            icon: TrustVisualIcon::ShieldCheck,
            color: TrustVisualColor::Blue,
            css_class: "trust-signal trust-signal--blue trust-signal--internal-proof",
            glow_class: "trust-glow trust-glow--blue",
            display_text: "INTERNAL PROOF ONLY",
            machine_state: "internal_proof_only",
            external_claim_allowed: false,
        },
        TrustState::ExternallyVerified => TrustStateView {
            state,
            badge_text: "AVC",
            icon: TrustVisualIcon::LockCheck,
            color: TrustVisualColor::Green,
            css_class: "trust-signal trust-signal--green trust-signal--externally-verified",
            glow_class: "trust-glow trust-glow--green",
            display_text: "VERIFIED",
            machine_state: "externally_verified",
            external_claim_allowed: proof_gates_complete,
        },
    }
}
