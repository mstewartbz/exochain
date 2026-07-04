use livesafe::genesis_development_trust::{
    FROST_GENESIS_PARTICIPANTS, FROST_GENESIS_THRESHOLD, GenesisAudience, GenesisTrustRequest,
    GenesisTrustSource, GenesisTrustUse, ProofState, evaluate_genesis_development_trust,
};

fn assert_denied(
    decision: &livesafe::genesis_development_trust::GenesisTrustDecision,
    reason: &str,
) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn exoforge_is_allowed_for_internal_genesis_work_when_provenance_is_recorded() {
    let decision = evaluate_genesis_development_trust(GenesisTrustRequest {
        source: GenesisTrustSource::ExoForge,
        use_case: GenesisTrustUse::Implementation,
        audience: GenesisAudience::InternalDevelopment,
        source_provenance_recorded: true,
        source_classification_complete: true,
        internal_proof_state: ProofState::Incomplete,
        frost_ceremony_completed: false,
        frost_threshold: FROST_GENESIS_THRESHOLD,
        frost_participants: FROST_GENESIS_PARTICIPANTS,
        verified_runtime_adapter: false,
        signals_trust_externally: false,
    });

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
}

#[test]
fn external_signaling_requires_internal_proof_exact_completed_frost_and_verified_adapter() {
    let decision = evaluate_genesis_development_trust(GenesisTrustRequest {
        source: GenesisTrustSource::VerifiedExochainRuntime,
        use_case: GenesisTrustUse::ExternalTrustSignal,
        audience: GenesisAudience::Public,
        source_provenance_recorded: true,
        source_classification_complete: true,
        internal_proof_state: ProofState::Incomplete,
        frost_ceremony_completed: false,
        frost_threshold: FROST_GENESIS_THRESHOLD,
        frost_participants: FROST_GENESIS_PARTICIPANTS,
        verified_runtime_adapter: false,
        signals_trust_externally: true,
    });

    assert_denied(
        &decision,
        "External trust signaling requires completed internal proof.",
    );
    assert_denied(
        &decision,
        "External trust signaling requires the completed 7-of-13 FROST keygen ceremony.",
    );
    assert_denied(
        &decision,
        "External trust signaling requires a verified runtime adapter.",
    );
}

#[test]
fn external_signaling_requires_the_exact_frost_profile() {
    let decision = evaluate_genesis_development_trust(GenesisTrustRequest {
        source: GenesisTrustSource::VerifiedExochainRuntime,
        use_case: GenesisTrustUse::CustomerRuntimeClaim,
        audience: GenesisAudience::Customer,
        source_provenance_recorded: true,
        source_classification_complete: true,
        internal_proof_state: ProofState::Complete,
        frost_ceremony_completed: true,
        frost_threshold: 6,
        frost_participants: FROST_GENESIS_PARTICIPANTS,
        verified_runtime_adapter: true,
        signals_trust_externally: true,
    });

    assert_denied(
        &decision,
        "External trust signaling requires the exact 7-of-13 FROST ceremony profile.",
    );
}

#[test]
fn provenance_is_required_for_all_genesis_development_trust() {
    let decision = evaluate_genesis_development_trust(GenesisTrustRequest {
        source: GenesisTrustSource::BobDirection,
        use_case: GenesisTrustUse::DevelopmentPlanning,
        audience: GenesisAudience::InternalDevelopment,
        source_provenance_recorded: false,
        source_classification_complete: true,
        internal_proof_state: ProofState::Incomplete,
        frost_ceremony_completed: false,
        frost_threshold: FROST_GENESIS_THRESHOLD,
        frost_participants: FROST_GENESIS_PARTICIPANTS,
        verified_runtime_adapter: false,
        signals_trust_externally: false,
    });

    assert_denied(
        &decision,
        "Genesis development trust requires source provenance.",
    );
}

#[test]
fn third_party_sources_need_classification_before_internal_development_use() {
    let decision = evaluate_genesis_development_trust(GenesisTrustRequest {
        source: GenesisTrustSource::ThirdParty,
        use_case: GenesisTrustUse::InternalValidation,
        audience: GenesisAudience::InternalDevelopment,
        source_provenance_recorded: true,
        source_classification_complete: false,
        internal_proof_state: ProofState::Incomplete,
        frost_ceremony_completed: false,
        frost_threshold: FROST_GENESIS_THRESHOLD,
        frost_participants: FROST_GENESIS_PARTICIPANTS,
        verified_runtime_adapter: false,
        signals_trust_externally: false,
    });

    assert_denied(
        &decision,
        "Third-party sources cannot be trusted for internal development without classification.",
    );
}
