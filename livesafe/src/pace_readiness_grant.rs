use std::collections::{BTreeMap, BTreeSet};

use crate::onboarding_pace::{PaceAcceptanceState, PaceRole};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessGrantProgramState {
    Disabled,
    InternalDraft,
    ActivePilot,
    ActiveProduction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardFrame {
    ReadinessGrant,
    SafetyCircleCompletion,
    ReferralBounty,
    GenericCoupon,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircleStrength {
    NotStarted,
    Forming,
    AlmostComplete,
    Complete,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaceGrantContactEvidence {
    pub role: PaceRole,
    pub subscriber_ref: String,
    pub contact_ref: String,
    pub acceptance_state: PaceAcceptanceState,
    pub obligation_accepted: bool,
    pub notification_channel_verified: bool,
    pub can_decline_without_penalty: bool,
    pub can_revoke_after_acceptance: bool,
    pub replacement_for: Option<String>,
    pub contains_raw_contact_data: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaceReadinessGrantRequest {
    pub subscriber_ref: String,
    pub program_state: ReadinessGrantProgramState,
    pub reward_frame: RewardFrame,
    pub requested_grant_months: u8,
    pub contacts: Vec<PaceGrantContactEvidence>,
    pub metadata_contains_raw_sensitive_data: bool,
    pub claims_guaranteed_emergency_response: bool,
    pub claims_verified_exochain_trust: bool,
    pub exochain_trust_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaceReadinessGrantDecision {
    pub allowed: bool,
    pub grant_months: u8,
    pub circle_strength: CircleStrength,
    pub accepted_ready_roles: Vec<PaceRole>,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

const REQUIRED_ROLES: [PaceRole; 4] = [
    PaceRole::Primary,
    PaceRole::Alternate,
    PaceRole::Contingent,
    PaceRole::Emergency,
];

const COMPLETION_GRANT_MONTHS: u8 = 4;

pub fn evaluate_pace_readiness_grant(
    request: &PaceReadinessGrantRequest,
) -> PaceReadinessGrantDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.subscriber_ref.trim().is_empty() {
        reasons.insert("Readiness grants require a synthetic subscriber reference.".to_string());
        required_evidence
            .insert("Synthetic subscriber reference for readiness grant evaluation.".to_string());
    }

    if !matches!(
        request.program_state,
        ReadinessGrantProgramState::ActivePilot | ReadinessGrantProgramState::ActiveProduction
    ) {
        reasons.insert(
            "Safety Circle Completion Grant remains inactive until the program is active pilot or active production."
                .to_string(),
        );
        required_evidence.insert(
            "Product-approved readiness-grant program state before grant issuance.".to_string(),
        );
    }

    if !matches!(
        request.reward_frame,
        RewardFrame::ReadinessGrant | RewardFrame::SafetyCircleCompletion
    ) {
        reasons.insert(
            "P.A.C.E. completion must be framed as readiness recognition, not a referral bounty or generic coupon."
                .to_string(),
        );
        required_evidence.insert(
            "Reward copy reviewed as Safety Circle completion or readiness grant language."
                .to_string(),
        );
    }

    if request.requested_grant_months != COMPLETION_GRANT_MONTHS {
        reasons.insert("Safety Circle completion grant must be exactly 4 months.".to_string());
        required_evidence.insert(
            "Four-month grant policy matching the four accepted P.A.C.E. roles.".to_string(),
        );
    }

    if request.metadata_contains_raw_sensitive_data {
        reasons.insert("Readiness grant metadata must not contain raw sensitive data.".to_string());
        required_evidence.insert(
            "Synthetic grant metadata with references only, no raw medical, identity, contact, QR, vault, location, eligibility, payment, or privileged content."
                .to_string(),
        );
    }

    if request.claims_guaranteed_emergency_response {
        reasons.insert(
            "Readiness grant copy must not claim guaranteed emergency response.".to_string(),
        );
        required_evidence.insert(
            "Copy review confirming LiveSafe describes readiness support, not guaranteed response."
                .to_string(),
        );
    }

    if request.claims_verified_exochain_trust && !request.exochain_trust_verified {
        reasons.insert(
            "Readiness grant copy must not claim verified EXOCHAIN/root-backed trust without current activation-gate proof."
                .to_string(),
        );
        required_evidence.insert(
            "Current verified adapter/runtime trust-state evidence before EXOCHAIN/root-backed claims."
                .to_string(),
        );
    }

    let mut roles_seen: BTreeMap<PaceRole, usize> = BTreeMap::new();
    let mut contact_refs_seen = BTreeSet::new();
    let mut accepted_ready_roles = BTreeSet::new();

    for contact in &request.contacts {
        *roles_seen.entry(contact.role).or_insert(0) += 1;

        if contact.subscriber_ref != request.subscriber_ref {
            reasons.insert(
                "P.A.C.E. grant contacts must be bound to the readiness-grant subscriber reference."
                    .to_string(),
            );
            required_evidence.insert(
                "Subscriber-scoped P.A.C.E. contact references without raw contact values."
                    .to_string(),
            );
        }

        if contact.contact_ref.trim().is_empty() {
            reasons.insert(
                "P.A.C.E. grant contacts require synthetic contact references.".to_string(),
            );
            required_evidence
                .insert("Synthetic P.A.C.E. contact reference for every role.".to_string());
        }

        if contact.contact_ref == request.subscriber_ref {
            reasons.insert(
                "Safety Circle grants must not allow subscriber self-grant as a P.A.C.E. contact."
                    .to_string(),
            );
            required_evidence
                .insert("Distinct non-subscriber contact reference for every role.".to_string());
        }

        if !contact.contact_ref.trim().is_empty()
            && !contact_refs_seen.insert(contact.contact_ref.clone())
        {
            reasons.insert(
                "Safety Circle grants require four distinct contact references.".to_string(),
            );
            required_evidence.insert(
                "Distinct P.A.C.E. contact references for Primary, Alternate, Contingent, and Emergency roles."
                    .to_string(),
            );
        }

        if contact.contains_raw_contact_data {
            reasons.insert(
                "P.A.C.E. grant contact evidence must not contain raw contact data.".to_string(),
            );
            required_evidence.insert(
                "Contact evidence represented by synthetic references, not phone numbers, emails, names, or addresses."
                    .to_string(),
            );
        }

        if contact.replacement_for.is_some() {
            reasons.insert(
                "Replaced P.A.C.E. contacts are not eligible for Safety Circle completion grants."
                    .to_string(),
            );
            required_evidence.insert(
                "Current active P.A.C.E. role evidence without replacement state.".to_string(),
            );
        }

        if contact.acceptance_state != PaceAcceptanceState::Accepted {
            reasons.insert(
                "Safety Circle completion requires all four P.A.C.E. roles to be accepted."
                    .to_string(),
            );
            required_evidence.insert(
                "Accepted state for Primary, Alternate, Contingent, and Emergency roles."
                    .to_string(),
            );
        }

        if !contact.obligation_accepted {
            reasons.insert(
                "Safety Circle completion requires explicit P.A.C.E. obligation acceptance."
                    .to_string(),
            );
            required_evidence.insert(
                "Confirmed social-contract obligation acceptance for every P.A.C.E. role."
                    .to_string(),
            );
        }

        if !contact.notification_channel_verified {
            reasons.insert(
                "Safety Circle completion requires verified notification eligibility for every accepted role."
                    .to_string(),
            );
            required_evidence.insert(
                "Verified notification channel evidence for every accepted P.A.C.E. role."
                    .to_string(),
            );
        }

        if !contact.can_decline_without_penalty || !contact.can_revoke_after_acceptance {
            reasons.insert(
                "P.A.C.E. invites must preserve invitee autonomy: accept, decline, or revoke without penalty."
                    .to_string(),
            );
            required_evidence.insert(
                "Invitation flow evidence showing decline and revocation paths.".to_string(),
            );
        }

        if contact.acceptance_state == PaceAcceptanceState::Accepted
            && contact.obligation_accepted
            && contact.notification_channel_verified
            && contact.replacement_for.is_none()
            && !contact.contains_raw_contact_data
        {
            accepted_ready_roles.insert(contact.role);
        }
    }

    for role in REQUIRED_ROLES {
        if !roles_seen.contains_key(&role) {
            reasons.insert(
                "Safety Circle completion requires Primary, Alternate, Contingent, and Emergency roles."
                    .to_string(),
            );
            required_evidence
                .insert("One current contact for each required P.A.C.E. role.".to_string());
        }

        if roles_seen.get(&role).copied().unwrap_or(0) > 1 {
            reasons.insert(
                "Safety Circle completion requires one contact per P.A.C.E. role.".to_string(),
            );
            required_evidence.insert(
                "No duplicate P.A.C.E. role assignments in the grant evidence.".to_string(),
            );
        }
    }

    if accepted_ready_roles.len() != REQUIRED_ROLES.len() {
        required_evidence.insert(
            "Four accepted, obligation-accepted, notification-eligible P.A.C.E. roles before grant issuance."
                .to_string(),
        );
    }

    let accepted_ready_roles_vec: Vec<PaceRole> = REQUIRED_ROLES
        .iter()
        .copied()
        .filter(|role| accepted_ready_roles.contains(role))
        .collect();

    let circle_strength = if contains_hard_block(&reasons) {
        CircleStrength::Blocked
    } else {
        circle_strength_from_ready_count(accepted_ready_roles_vec.len())
    };

    let allowed = reasons.is_empty() && accepted_ready_roles_vec.len() == REQUIRED_ROLES.len();

    PaceReadinessGrantDecision {
        allowed,
        grant_months: if allowed { COMPLETION_GRANT_MONTHS } else { 0 },
        circle_strength,
        accepted_ready_roles: accepted_ready_roles_vec,
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn circle_strength_from_ready_count(count: usize) -> CircleStrength {
    match count {
        0 => CircleStrength::NotStarted,
        1 | 2 => CircleStrength::Forming,
        3 => CircleStrength::AlmostComplete,
        _ => CircleStrength::Complete,
    }
}

fn contains_hard_block(reasons: &BTreeSet<String>) -> bool {
    reasons.iter().any(|reason| {
        reason.contains("self-grant")
            || reason.contains("distinct contact")
            || reason.contains("raw")
            || reason.contains("referral bounty")
            || reason.contains("guaranteed emergency response")
            || reason.contains("EXOCHAIN/root-backed")
    })
}
