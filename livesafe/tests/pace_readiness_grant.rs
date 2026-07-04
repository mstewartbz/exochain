use livesafe::onboarding_pace::{PaceAcceptanceState, PaceRole};
use livesafe::pace_readiness_grant::{
    CircleStrength, PaceGrantContactEvidence, PaceReadinessGrantRequest,
    ReadinessGrantProgramState, RewardFrame, evaluate_pace_readiness_grant,
};

const SUBSCRIBER: &str = "subscriber:synthetic-001";

fn ready_contact(role: PaceRole, contact_ref: &str) -> PaceGrantContactEvidence {
    PaceGrantContactEvidence {
        role,
        subscriber_ref: SUBSCRIBER.into(),
        contact_ref: contact_ref.into(),
        acceptance_state: PaceAcceptanceState::Accepted,
        obligation_accepted: true,
        notification_channel_verified: true,
        can_decline_without_penalty: true,
        can_revoke_after_acceptance: true,
        replacement_for: None,
        contains_raw_contact_data: false,
    }
}

fn complete_request() -> PaceReadinessGrantRequest {
    PaceReadinessGrantRequest {
        subscriber_ref: SUBSCRIBER.into(),
        program_state: ReadinessGrantProgramState::ActivePilot,
        reward_frame: RewardFrame::SafetyCircleCompletion,
        requested_grant_months: 4,
        contacts: vec![
            ready_contact(PaceRole::Primary, "pace:primary"),
            ready_contact(PaceRole::Alternate, "pace:alternate"),
            ready_contact(PaceRole::Contingent, "pace:contingent"),
            ready_contact(PaceRole::Emergency, "pace:emergency"),
        ],
        metadata_contains_raw_sensitive_data: false,
        claims_guaranteed_emergency_response: false,
        claims_verified_exochain_trust: false,
        exochain_trust_verified: false,
    }
}

#[test]
fn complete_safety_circle_receives_four_month_readiness_grant() {
    let decision = evaluate_pace_readiness_grant(&complete_request());

    assert!(decision.allowed);
    assert_eq!(decision.grant_months, 4);
    assert_eq!(decision.circle_strength, CircleStrength::Complete);
    assert_eq!(decision.accepted_ready_roles.len(), 4);
    assert_eq!(decision.reasons, Vec::<String>::new());
}

#[test]
fn grant_requires_completion_not_sent_invites_or_partial_acceptance() {
    let mut request = complete_request();
    request.contacts.pop();

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert_eq!(decision.grant_months, 0);
    assert_eq!(decision.circle_strength, CircleStrength::AlmostComplete);
    assert!(
        decision
            .reasons
            .contains(&"Safety Circle completion requires Primary, Alternate, Contingent, and Emergency roles.".into())
    );
}

#[test]
fn grant_denies_referral_bounty_framing() {
    let mut request = complete_request();
    request.reward_frame = RewardFrame::ReferralBounty;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert_eq!(decision.circle_strength, CircleStrength::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"P.A.C.E. completion must be framed as readiness recognition, not a referral bounty or generic coupon.".into())
    );
}

#[test]
fn grant_denies_registration_without_obligation_acceptance() {
    let mut request = complete_request();
    request.contacts[0].obligation_accepted = false;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert_eq!(decision.grant_months, 0);
    assert!(decision.reasons.contains(
        &"Safety Circle completion requires explicit P.A.C.E. obligation acceptance.".into()
    ));
}

#[test]
fn grant_denies_unverified_notification_channels() {
    let mut request = complete_request();
    request.contacts[2].notification_channel_verified = false;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert_eq!(decision.grant_months, 0);
    assert!(
        decision
            .reasons
            .contains(&"Safety Circle completion requires verified notification eligibility for every accepted role.".into())
    );
}

#[test]
fn grant_denies_self_grant_duplicate_contacts_and_duplicate_roles() {
    let mut request = complete_request();
    request.contacts[0].contact_ref = SUBSCRIBER.into();
    request.contacts[1].contact_ref = "pace:duplicate".into();
    request.contacts[2].contact_ref = "pace:duplicate".into();
    request.contacts[3].role = PaceRole::Contingent;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert_eq!(decision.circle_strength, CircleStrength::Blocked);
    assert!(decision.reasons.contains(
        &"Safety Circle grants must not allow subscriber self-grant as a P.A.C.E. contact.".into()
    ));
    assert!(
        decision
            .reasons
            .contains(&"Safety Circle grants require four distinct contact references.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"Safety Circle completion requires one contact per P.A.C.E. role.".into())
    );
}

#[test]
fn grant_preserves_invitee_decline_and_revoke_autonomy() {
    let mut request = complete_request();
    request.contacts[1].can_decline_without_penalty = false;
    request.contacts[1].can_revoke_after_acceptance = false;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"P.A.C.E. invites must preserve invitee autonomy: accept, decline, or revoke without penalty.".into())
    );
}

#[test]
fn grant_denies_raw_sensitive_metadata_and_unsupported_claims() {
    let mut request = complete_request();
    request.metadata_contains_raw_sensitive_data = true;
    request.contacts[0].contains_raw_contact_data = true;
    request.claims_guaranteed_emergency_response = true;
    request.claims_verified_exochain_trust = true;
    request.exochain_trust_verified = false;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert_eq!(decision.circle_strength, CircleStrength::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"Readiness grant metadata must not contain raw sensitive data.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"P.A.C.E. grant contact evidence must not contain raw contact data.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"Readiness grant copy must not claim guaranteed emergency response.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"Readiness grant copy must not claim verified EXOCHAIN/root-backed trust without current activation-gate proof.".into())
    );
}

#[test]
fn grant_requires_active_program_and_exact_four_month_policy() {
    let mut request = complete_request();
    request.program_state = ReadinessGrantProgramState::InternalDraft;
    request.requested_grant_months = 3;

    let decision = evaluate_pace_readiness_grant(&request);

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"Safety Circle Completion Grant remains inactive until the program is active pilot or active production.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"Safety Circle completion grant must be exactly 4 months.".into())
    );
}
