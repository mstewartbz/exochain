use livesafe::trust_state_view::{
    TrustState, TrustStateViewRequest, TrustSurface, TrustVisualColor, evaluate_trust_state_view,
};

#[test]
fn trust_state_views_define_the_full_safe_palette() {
    let not_verified = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::NotVerified,
        surface: TrustSurface::PublicWebsite,
        includes_trust_bearing_claim: false,
        internal_proof_complete: false,
        frost_genesis_complete: false,
        verified_runtime_adapter: false,
        accessible_label_present: true,
        machine_state_present: true,
    });

    assert!(not_verified.allowed, "{not_verified:?}");
    let view = not_verified.view.expect("view metadata should exist");
    assert_eq!(view.badge_text, "AVC");
    assert_eq!(view.color, TrustVisualColor::Red);
    assert_eq!(view.display_text, "THIS IS NOT YET VERIFIED");
    assert_eq!(view.machine_state, "not_verified");
    assert!(!view.external_claim_allowed);

    let genesis_pending = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::GenesisPending,
        surface: TrustSurface::PrintedCard,
        includes_trust_bearing_claim: false,
        internal_proof_complete: false,
        frost_genesis_complete: false,
        verified_runtime_adapter: false,
        accessible_label_present: true,
        machine_state_present: true,
    });
    assert_eq!(
        genesis_pending
            .view
            .expect("genesis pending view should exist")
            .color,
        TrustVisualColor::Yellow
    );

    let internal_proof = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::InternalProof,
        surface: TrustSurface::PrivateReview,
        includes_trust_bearing_claim: false,
        internal_proof_complete: true,
        frost_genesis_complete: false,
        verified_runtime_adapter: false,
        accessible_label_present: true,
        machine_state_present: true,
    });
    assert_eq!(
        internal_proof
            .view
            .expect("internal proof view should exist")
            .color,
        TrustVisualColor::Blue
    );

    let externally_verified = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::ExternallyVerified,
        surface: TrustSurface::CustomerPortal,
        includes_trust_bearing_claim: true,
        internal_proof_complete: true,
        frost_genesis_complete: true,
        verified_runtime_adapter: true,
        accessible_label_present: true,
        machine_state_present: true,
    });
    let externally_verified_view = externally_verified
        .view
        .expect("externally verified view should exist");
    assert_eq!(externally_verified_view.color, TrustVisualColor::Green);
    assert_eq!(externally_verified_view.display_text, "VERIFIED");
    assert!(externally_verified_view.external_claim_allowed);
}

#[test]
fn public_trust_claims_fail_closed_until_the_state_and_proof_gates_allow_them() {
    let denied_not_verified = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::NotVerified,
        surface: TrustSurface::PublicWebsite,
        includes_trust_bearing_claim: true,
        internal_proof_complete: false,
        frost_genesis_complete: false,
        verified_runtime_adapter: false,
        accessible_label_present: true,
        machine_state_present: true,
    });

    assert!(!denied_not_verified.allowed);
    assert!(denied_not_verified.reasons.contains(
        &"Public trust-bearing claims are blocked unless the state is externally verified.".into()
    ));

    let denied_missing_gates = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::ExternallyVerified,
        surface: TrustSurface::ApiResponse,
        includes_trust_bearing_claim: true,
        internal_proof_complete: true,
        frost_genesis_complete: false,
        verified_runtime_adapter: false,
        accessible_label_present: true,
        machine_state_present: true,
    });

    assert!(!denied_missing_gates.allowed);
    assert!(denied_missing_gates.reasons.contains(
        &"Externally verified trust display requires completed internal proof, completed genesis ceremony, and a verified runtime adapter.".into()
    ));

    let allowed = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::ExternallyVerified,
        surface: TrustSurface::PublicWebsite,
        includes_trust_bearing_claim: true,
        internal_proof_complete: true,
        frost_genesis_complete: true,
        verified_runtime_adapter: true,
        accessible_label_present: true,
        machine_state_present: true,
    });

    assert!(allowed.allowed, "{allowed:?}");
    assert_eq!(allowed.reasons, Vec::<String>::new());
}

#[test]
fn trust_state_views_require_accessible_and_machine_readable_status_fields() {
    let decision = evaluate_trust_state_view(TrustStateViewRequest {
        state: TrustState::GenesisPending,
        surface: TrustSurface::PrivateReview,
        includes_trust_bearing_claim: false,
        internal_proof_complete: false,
        frost_genesis_complete: false,
        verified_runtime_adapter: false,
        accessible_label_present: false,
        machine_state_present: false,
    });

    assert!(!decision.allowed);
    assert!(
        decision.reasons.contains(
            &"Trust-state displays require an accessible label equivalent to the visible status."
                .into()
        )
    );
    assert!(
        decision
            .reasons
            .contains(&"Trust-state displays require the canonical machine-readable state.".into())
    );
}
