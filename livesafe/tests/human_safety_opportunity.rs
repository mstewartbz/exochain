use livesafe::human_safety_opportunity::{
    FirstLoopAction, HumanSafetyOpportunity, OpportunityPriority, ReadinessMetricInputs,
    SafetySegment, calculate_readiness_metrics, evaluate_human_safety_opportunity,
    human_continuity_activation_score,
};

fn ready_opportunity() -> HumanSafetyOpportunity {
    HumanSafetyOpportunity {
        subscriber_ref: "subscriber:synthetic-001".into(),
        segment: SafetySegment::CaregiverDependent,
        emergency_card_created: true,
        core_profile_completed: true,
        card_saved_or_printed: true,
        accepted_pace_obligations: 4,
        safety_circle_completed: true,
        review_fresh: true,
        dependent_family_or_team_readiness: true,
        requires_full_medical_jacket_for_first_loop: false,
        requires_genetic_import_for_first_loop: false,
        requires_clinical_trial_matching_for_first_loop: false,
        requires_responder_agency_adoption_for_first_loop: false,
        contains_raw_sensitive_payload: false,
        uses_referral_bounty_framing: false,
        claims_guaranteed_emergency_response: false,
        claims_verified_exochain_trust: false,
        livesafe_adapter_verified: false,
    }
}

#[test]
fn high_priority_loop_is_card_pace_and_people_before_platform_sprawl() {
    let decision = evaluate_human_safety_opportunity(&ready_opportunity());

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.priority, OpportunityPriority::Highest);
    assert_eq!(
        decision.next_action,
        FirstLoopAction::ReadyForFamilyOrTeamPath
    );
    assert_eq!(decision.human_continuity_activation_score, 100);
    assert_eq!(decision.reasons, Vec::<String>::new());
}

#[test]
fn first_loop_progression_prioritizes_card_then_core_profile_then_pace() {
    let mut opportunity = ready_opportunity();
    opportunity.emergency_card_created = false;
    assert_eq!(
        evaluate_human_safety_opportunity(&opportunity).next_action,
        FirstLoopAction::CreateEmergencyCard
    );

    opportunity.emergency_card_created = true;
    opportunity.core_profile_completed = false;
    assert_eq!(
        evaluate_human_safety_opportunity(&opportunity).next_action,
        FirstLoopAction::CompleteCoreEmergencyProfile
    );

    opportunity.core_profile_completed = true;
    opportunity.accepted_pace_obligations = 2;
    opportunity.safety_circle_completed = false;
    assert_eq!(
        evaluate_human_safety_opportunity(&opportunity).next_action,
        FirstLoopAction::InvitePaceContacts
    );
}

#[test]
fn later_segments_are_deferred_without_becoming_runtime_blockers() {
    let mut opportunity = ready_opportunity();
    opportunity.segment = SafetySegment::GeneticDataOwner;

    let decision = evaluate_human_safety_opportunity(&opportunity);

    assert!(decision.allowed);
    assert_eq!(decision.priority, OpportunityPriority::Later);
    assert!(
        decision.reasons.contains(
            &"Year-one opportunity is later priority because it does not anchor the immediate safety loop."
                .into()
        )
    );
}

#[test]
fn opportunity_blocks_raw_data_platform_dependencies_and_unsupported_claims() {
    let mut opportunity = ready_opportunity();
    opportunity.contains_raw_sensitive_payload = true;
    opportunity.requires_full_medical_jacket_for_first_loop = true;
    opportunity.requires_genetic_import_for_first_loop = true;
    opportunity.requires_clinical_trial_matching_for_first_loop = true;
    opportunity.requires_responder_agency_adoption_for_first_loop = true;
    opportunity.uses_referral_bounty_framing = true;
    opportunity.claims_guaranteed_emergency_response = true;
    opportunity.claims_verified_exochain_trust = true;
    opportunity.livesafe_adapter_verified = false;

    let decision = evaluate_human_safety_opportunity(&opportunity);

    assert!(!decision.allowed);
    assert_eq!(decision.priority, OpportunityPriority::Blocked);
    assert!(decision.reasons.contains(
        &"Human safety opportunity inputs must not contain raw sensitive payloads.".into()
    ));
    assert!(decision.reasons.contains(
        &"The first public loop must not require full medical-jacket completion.".into()
    ));
    assert!(
        decision
            .reasons
            .contains(&"The first public loop must not require genetic-data import.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"The first public loop must not require clinical-trial matching.".into())
    );
    assert!(decision.reasons.contains(
        &"The first public loop must not require unsupported responder agency adoption.".into()
    ));
    assert!(
        decision.reasons.contains(
            &"Safety Circle completion must be framed as readiness recognition, not referral-bounty logic."
                .into()
        )
    );
    assert!(
        decision.reasons.contains(
            &"Human safety opportunity language must not claim verified EXOCHAIN/root-backed trust without LiveSafe adapter proof."
                .into()
        )
    );
}

#[test]
fn readiness_metrics_use_integer_bps_and_milli_spread() {
    let metrics = calculate_readiness_metrics(&ReadinessMetricInputs {
        card_start_count: 100,
        card_created_count: 50,
        card_saved_or_printed_count: 25,
        card_owners_with_one_or_more_invites: 40,
        pace_invites_sent_count: 40,
        pace_invites_accepted_count: 20,
        accepted_contact_profile_created_count: 10,
        dependent_profiles_created_count: 5,
        core_profile_completed_count: 30,
        review_fresh_count: 15,
        family_or_team_intent_count: 12,
        safety_circle_completed_count: 10,
        readiness_grant_issued_count: 5,
        average_invites_per_card_milli: 4_000,
    });

    assert_eq!(metrics.card_creation_rate_bps, 5_000);
    assert_eq!(metrics.card_carry_forward_rate_bps, 5_000);
    assert_eq!(metrics.pace_invite_rate_bps, 8_000);
    assert_eq!(metrics.pace_acceptance_rate_bps, 5_000);
    assert_eq!(metrics.accepted_contact_profile_creation_rate_bps, 5_000);
    assert_eq!(metrics.dependent_profile_rate_bps, 1_000);
    assert_eq!(metrics.core_profile_completion_rate_bps, 6_000);
    assert_eq!(metrics.review_fresh_rate_bps, 3_000);
    assert_eq!(metrics.family_or_team_intent_rate_bps, 2_400);
    assert_eq!(metrics.safety_circle_completion_rate_bps, 2_000);
    assert_eq!(metrics.readiness_grant_issuance_rate_bps, 5_000);
    assert_eq!(metrics.pace_spread_coefficient_milli, 1_000);
}

#[test]
fn continuity_activation_score_requires_full_circle_for_completion_weight() {
    let mut opportunity = ready_opportunity();
    opportunity.accepted_pace_obligations = 2;
    opportunity.safety_circle_completed = false;

    assert_eq!(human_continuity_activation_score(&opportunity), 85);

    opportunity.accepted_pace_obligations = 4;
    opportunity.safety_circle_completed = true;

    assert_eq!(human_continuity_activation_score(&opportunity), 100);
}
