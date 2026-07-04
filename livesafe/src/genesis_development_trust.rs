use std::collections::BTreeSet;

pub const FROST_GENESIS_THRESHOLD: u8 = 7;
pub const FROST_GENESIS_PARTICIPANTS: u8 = 13;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenesisTrustSource {
    BobDirection,
    ExoForge,
    VerifiedExochainRuntime,
    ThirdParty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenesisTrustUse {
    DevelopmentPlanning,
    Implementation,
    InternalValidation,
    ExternalTrustSignal,
    CustomerRuntimeClaim,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenesisAudience {
    InternalDevelopment,
    PrivateReview,
    Customer,
    Public,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofState {
    Incomplete,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenesisTrustRequest {
    pub source: GenesisTrustSource,
    pub use_case: GenesisTrustUse,
    pub audience: GenesisAudience,
    pub source_provenance_recorded: bool,
    pub source_classification_complete: bool,
    pub internal_proof_state: ProofState,
    pub frost_ceremony_completed: bool,
    pub frost_threshold: u8,
    pub frost_participants: u8,
    pub verified_runtime_adapter: bool,
    pub signals_trust_externally: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenesisTrustDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_genesis_development_trust(request: GenesisTrustRequest) -> GenesisTrustDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if !request.source_provenance_recorded {
        reasons.insert("Genesis development trust requires source provenance.".to_string());
        required_evidence.insert(
            "Source record naming Bob direction, ExoForge output, or verified runtime evidence."
                .to_string(),
        );
    }

    if request.source == GenesisTrustSource::ThirdParty
        && is_internal_use(&request)
        && !request.source_classification_complete
    {
        reasons.insert(
            "Third-party sources cannot be trusted for internal development without classification."
                .to_string(),
        );
        required_evidence
            .insert("Repository intake, license review, and IP classification.".to_string());
    }

    if is_externally_visible(&request) {
        if request.internal_proof_state != ProofState::Complete {
            reasons
                .insert("External trust signaling requires completed internal proof.".to_string());
            required_evidence.insert("Internal proof gate report.".to_string());
        }

        if !request.frost_ceremony_completed {
            reasons.insert(
                "External trust signaling requires the completed 7-of-13 FROST keygen ceremony."
                    .to_string(),
            );
            required_evidence.insert(
                "FROST keygen ceremony transcript and participant attestations.".to_string(),
            );
        }

        if !has_exact_frost_profile(&request) {
            reasons.insert(
                "External trust signaling requires the exact 7-of-13 FROST ceremony profile."
                    .to_string(),
            );
            required_evidence.insert(
                "Ceremony profile proving threshold 7 and participant count 13.".to_string(),
            );
        }

        if !request.verified_runtime_adapter {
            reasons.insert(
                "External trust signaling requires a verified runtime adapter.".to_string(),
            );
            required_evidence
                .insert("Runtime adapter tests proving fail-closed EXOCHAIN behavior.".to_string());
        }
    }

    GenesisTrustDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn is_internal_use(request: &GenesisTrustRequest) -> bool {
    matches!(request.audience, GenesisAudience::InternalDevelopment)
        && !request.signals_trust_externally
        && matches!(
            request.use_case,
            GenesisTrustUse::DevelopmentPlanning
                | GenesisTrustUse::Implementation
                | GenesisTrustUse::InternalValidation
        )
}

fn is_externally_visible(request: &GenesisTrustRequest) -> bool {
    request.signals_trust_externally
        || matches!(
            request.use_case,
            GenesisTrustUse::ExternalTrustSignal | GenesisTrustUse::CustomerRuntimeClaim
        )
        || matches!(
            request.audience,
            GenesisAudience::Customer | GenesisAudience::Public
        )
}

fn has_exact_frost_profile(request: &GenesisTrustRequest) -> bool {
    request.frost_threshold == FROST_GENESIS_THRESHOLD
        && request.frost_participants == FROST_GENESIS_PARTICIPANTS
}
