use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetySegment {
    CaregiverDependent,
    HighRiskCondition,
    ElderHousehold,
    FrontlineFamily,
    SmallTeamDutyOfCare,
    GenericWellness,
    GeneticDataOwner,
    ClinicalTrialOnly,
    EnterpriseResponderIntegrationFirst,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpportunityPriority {
    Highest,
    Later,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirstLoopAction {
    CreateEmergencyCard,
    CompleteCoreEmergencyProfile,
    InvitePaceContacts,
    SaveOrPrintCard,
    ReviewFreshness,
    ReadyForFamilyOrTeamPath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HumanSafetyOpportunity {
    pub subscriber_ref: String,
    pub segment: SafetySegment,
    pub emergency_card_created: bool,
    pub core_profile_completed: bool,
    pub card_saved_or_printed: bool,
    pub accepted_pace_obligations: u8,
    pub safety_circle_completed: bool,
    pub review_fresh: bool,
    pub dependent_family_or_team_readiness: bool,
    pub requires_full_medical_jacket_for_first_loop: bool,
    pub requires_genetic_import_for_first_loop: bool,
    pub requires_clinical_trial_matching_for_first_loop: bool,
    pub requires_responder_agency_adoption_for_first_loop: bool,
    pub contains_raw_sensitive_payload: bool,
    pub uses_referral_bounty_framing: bool,
    pub claims_guaranteed_emergency_response: bool,
    pub claims_verified_exochain_trust: bool,
    pub livesafe_adapter_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HumanSafetyOpportunityDecision {
    pub allowed: bool,
    pub priority: OpportunityPriority,
    pub next_action: FirstLoopAction,
    pub human_continuity_activation_score: u16,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadinessMetricInputs {
    pub card_start_count: u64,
    pub card_created_count: u64,
    pub card_saved_or_printed_count: u64,
    pub card_owners_with_one_or_more_invites: u64,
    pub pace_invites_sent_count: u64,
    pub pace_invites_accepted_count: u64,
    pub accepted_contact_profile_created_count: u64,
    pub dependent_profiles_created_count: u64,
    pub core_profile_completed_count: u64,
    pub review_fresh_count: u64,
    pub family_or_team_intent_count: u64,
    pub safety_circle_completed_count: u64,
    pub readiness_grant_issued_count: u64,
    pub average_invites_per_card_milli: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadinessMetrics {
    pub card_creation_rate_bps: u64,
    pub card_carry_forward_rate_bps: u64,
    pub pace_invite_rate_bps: u64,
    pub pace_acceptance_rate_bps: u64,
    pub accepted_contact_profile_creation_rate_bps: u64,
    pub dependent_profile_rate_bps: u64,
    pub core_profile_completion_rate_bps: u64,
    pub review_fresh_rate_bps: u64,
    pub family_or_team_intent_rate_bps: u64,
    pub safety_circle_completion_rate_bps: u64,
    pub readiness_grant_issuance_rate_bps: u64,
    pub pace_spread_coefficient_milli: u64,
}

const FULL_PACE_CIRCLE_COUNT: u8 = 4;

pub fn evaluate_human_safety_opportunity(
    opportunity: &HumanSafetyOpportunity,
) -> HumanSafetyOpportunityDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if opportunity.subscriber_ref.trim().is_empty() {
        reasons.insert(
            "Human safety opportunity evaluation requires a synthetic subscriber reference."
                .to_string(),
        );
        required_evidence.insert(
            "Synthetic subscriber reference for human-safety opportunity evaluation.".to_string(),
        );
    }

    if !is_high_priority_segment(opportunity.segment) {
        reasons.insert(
            "Year-one opportunity is later priority because it does not anchor the immediate safety loop."
                .to_string(),
        );
        required_evidence.insert(
            "Evidence that the segment can create a card, invite P.A.C.E. humans, and protect people before platform expansion."
                .to_string(),
        );
    }

    if opportunity.contains_raw_sensitive_payload {
        reasons.insert(
            "Human safety opportunity inputs must not contain raw sensitive payloads.".to_string(),
        );
        required_evidence.insert(
            "Synthetic references only; no raw medical, genetic, identity, contact, trustee, location, QR, vault, payment, or emergency-access data."
                .to_string(),
        );
    }

    if opportunity.requires_full_medical_jacket_for_first_loop {
        reasons.insert(
            "The first public loop must not require full medical-jacket completion.".to_string(),
        );
        required_evidence.insert(
            "Emergency card and P.A.C.E. readiness path before medical-jacket depth.".to_string(),
        );
    }

    if opportunity.requires_genetic_import_for_first_loop {
        reasons.insert("The first public loop must not require genetic-data import.".to_string());
        required_evidence
            .insert("Emergency readiness path that works without genotypical data.".to_string());
    }

    if opportunity.requires_clinical_trial_matching_for_first_loop {
        reasons
            .insert("The first public loop must not require clinical-trial matching.".to_string());
        required_evidence.insert(
            "Safety-loop readiness evidence before precision trial matching activation."
                .to_string(),
        );
    }

    if opportunity.requires_responder_agency_adoption_for_first_loop {
        reasons.insert(
            "The first public loop must not require unsupported responder agency adoption."
                .to_string(),
        );
        required_evidence.insert(
            "Owner and P.A.C.E. readiness evidence independent of responder-agency acceptance claims."
                .to_string(),
        );
    }

    if opportunity.uses_referral_bounty_framing {
        reasons.insert(
            "Safety Circle completion must be framed as readiness recognition, not referral-bounty logic."
                .to_string(),
        );
        required_evidence.insert(
            "Copy using Safety Circle or Readiness Grant language without traffic, lead, or bounty framing."
                .to_string(),
        );
    }

    if opportunity.claims_guaranteed_emergency_response {
        reasons.insert(
            "Human safety opportunity language must not claim guaranteed emergency response."
                .to_string(),
        );
        required_evidence.insert(
            "Copy review confirming LiveSafe describes readiness support, not guaranteed response."
                .to_string(),
        );
    }

    if opportunity.claims_verified_exochain_trust && !opportunity.livesafe_adapter_verified {
        reasons.insert(
            "Human safety opportunity language must not claim verified EXOCHAIN/root-backed trust without LiveSafe adapter proof."
                .to_string(),
        );
        required_evidence.insert(
            "Current verified LiveSafe adapter/runtime trust-state evidence before EXOCHAIN-root-backed public claims."
                .to_string(),
        );
    }

    let next_action = next_first_loop_action(opportunity);
    let score = human_continuity_activation_score(opportunity);
    let priority = opportunity_priority(opportunity.segment, &reasons);
    let allowed = matches!(
        priority,
        OpportunityPriority::Highest | OpportunityPriority::Later
    ) && no_hard_block(&reasons);

    HumanSafetyOpportunityDecision {
        allowed,
        priority,
        next_action,
        human_continuity_activation_score: score,
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

pub fn calculate_readiness_metrics(inputs: &ReadinessMetricInputs) -> ReadinessMetrics {
    let card_creation_rate_bps = ratio_bps(inputs.card_created_count, inputs.card_start_count);
    let card_carry_forward_rate_bps = ratio_bps(
        inputs.card_saved_or_printed_count,
        inputs.card_created_count,
    );
    let pace_invite_rate_bps = ratio_bps(
        inputs.card_owners_with_one_or_more_invites,
        inputs.card_created_count,
    );
    let pace_acceptance_rate_bps = ratio_bps(
        inputs.pace_invites_accepted_count,
        inputs.pace_invites_sent_count,
    );
    let accepted_contact_profile_creation_rate_bps = ratio_bps(
        inputs.accepted_contact_profile_created_count,
        inputs.pace_invites_accepted_count,
    );

    ReadinessMetrics {
        card_creation_rate_bps,
        card_carry_forward_rate_bps,
        pace_invite_rate_bps,
        pace_acceptance_rate_bps,
        accepted_contact_profile_creation_rate_bps,
        dependent_profile_rate_bps: ratio_bps(
            inputs.dependent_profiles_created_count,
            inputs.card_created_count,
        ),
        core_profile_completion_rate_bps: ratio_bps(
            inputs.core_profile_completed_count,
            inputs.card_created_count,
        ),
        review_fresh_rate_bps: ratio_bps(inputs.review_fresh_count, inputs.card_created_count),
        family_or_team_intent_rate_bps: ratio_bps(
            inputs.family_or_team_intent_count,
            inputs.card_created_count,
        ),
        safety_circle_completion_rate_bps: ratio_bps(
            inputs.safety_circle_completed_count,
            inputs.card_created_count,
        ),
        readiness_grant_issuance_rate_bps: ratio_bps(
            inputs.readiness_grant_issued_count,
            inputs.safety_circle_completed_count,
        ),
        pace_spread_coefficient_milli: spread_coefficient_milli(
            inputs.average_invites_per_card_milli,
            pace_acceptance_rate_bps,
            accepted_contact_profile_creation_rate_bps,
        ),
    }
}

pub fn human_continuity_activation_score(opportunity: &HumanSafetyOpportunity) -> u16 {
    let mut score = 0;

    if opportunity.emergency_card_created {
        score += 20;
    }

    if opportunity.card_saved_or_printed {
        score += 10;
    }

    if opportunity.core_profile_completed {
        score += 20;
    }

    if opportunity.accepted_pace_obligations >= 2 {
        score += 15;
    }

    if opportunity.safety_circle_completed
        && opportunity.accepted_pace_obligations >= FULL_PACE_CIRCLE_COUNT
    {
        score += 15;
    }

    if opportunity.review_fresh {
        score += 10;
    }

    if opportunity.dependent_family_or_team_readiness {
        score += 10;
    }

    score
}

fn next_first_loop_action(opportunity: &HumanSafetyOpportunity) -> FirstLoopAction {
    if !opportunity.emergency_card_created {
        return FirstLoopAction::CreateEmergencyCard;
    }

    if !opportunity.core_profile_completed {
        return FirstLoopAction::CompleteCoreEmergencyProfile;
    }

    if opportunity.accepted_pace_obligations < FULL_PACE_CIRCLE_COUNT {
        return FirstLoopAction::InvitePaceContacts;
    }

    if !opportunity.card_saved_or_printed {
        return FirstLoopAction::SaveOrPrintCard;
    }

    if !opportunity.review_fresh {
        return FirstLoopAction::ReviewFreshness;
    }

    FirstLoopAction::ReadyForFamilyOrTeamPath
}

fn is_high_priority_segment(segment: SafetySegment) -> bool {
    matches!(
        segment,
        SafetySegment::CaregiverDependent
            | SafetySegment::HighRiskCondition
            | SafetySegment::ElderHousehold
            | SafetySegment::FrontlineFamily
            | SafetySegment::SmallTeamDutyOfCare
    )
}

fn opportunity_priority(segment: SafetySegment, reasons: &BTreeSet<String>) -> OpportunityPriority {
    if contains_hard_block(reasons) {
        return OpportunityPriority::Blocked;
    }

    if is_high_priority_segment(segment) {
        OpportunityPriority::Highest
    } else {
        OpportunityPriority::Later
    }
}

fn no_hard_block(reasons: &BTreeSet<String>) -> bool {
    !contains_hard_block(reasons)
}

fn contains_hard_block(reasons: &BTreeSet<String>) -> bool {
    reasons.iter().any(|reason| {
        reason.contains("raw sensitive")
            || reason.contains("full medical-jacket")
            || reason.contains("subscriber reference")
            || reason.contains("genetic-data")
            || reason.contains("clinical-trial")
            || reason.contains("unsupported responder")
            || reason.contains("referral-bounty")
            || reason.contains("guaranteed emergency response")
            || reason.contains("EXOCHAIN/root-backed")
    })
}

fn ratio_bps(numerator: u64, denominator: u64) -> u64 {
    clamp_u128_to_u64((numerator as u128) * 10_000 / denominator.max(1) as u128)
}

fn spread_coefficient_milli(
    average_invites_per_card_milli: u64,
    acceptance_rate_bps: u64,
    profile_creation_rate_bps: u64,
) -> u64 {
    clamp_u128_to_u64(
        (average_invites_per_card_milli as u128)
            * (acceptance_rate_bps as u128)
            * (profile_creation_rate_bps as u128)
            / 100_000_000,
    )
}

fn clamp_u128_to_u64(value: u128) -> u64 {
    value.min(u64::MAX as u128) as u64
}
