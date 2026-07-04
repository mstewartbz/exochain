use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PaceRole {
    Primary,
    Alternate,
    Contingent,
    Emergency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaceAcceptanceState {
    Invited,
    Accepted,
    Declined,
    Revoked,
    Replaced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntitlementSelectionState {
    NotSelected,
    BasicFree,
    FamilyPaid,
    TeamPaid,
    Trial,
    Gift,
    FrontlineBasicFamily,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OnboardingNextAction {
    CreateAccount,
    ConfigureEmergencyCard,
    InvitePaceContacts,
    StartMedicalJacket,
    CompleteMedicalJacket,
    SelectEntitlement,
    ReadyForActivation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaceContact {
    pub role: PaceRole,
    pub subscriber_ref: String,
    pub contact_ref: String,
    pub acceptance_state: PaceAcceptanceState,
    pub obligation_accepted: bool,
    pub replacement_for: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardingState {
    pub account_created: bool,
    pub emergency_card_configured: bool,
    pub pace_contacts: Vec<PaceContact>,
    pub medical_jacket_started: bool,
    pub medical_jacket_complete: bool,
    pub entitlement: EntitlementSelectionState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardingDecision {
    pub allowed: bool,
    pub complete: bool,
    pub next_action: OnboardingNextAction,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaceDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

const REQUIRED_PACE_ROLES: [PaceRole; 4] = [
    PaceRole::Primary,
    PaceRole::Alternate,
    PaceRole::Contingent,
    PaceRole::Emergency,
];

pub fn notification_eligible(contact: &PaceContact) -> bool {
    contact.acceptance_state == PaceAcceptanceState::Accepted
        && contact.obligation_accepted
        && contact.replacement_for.is_none()
}

pub fn evaluate_pace_contacts(subscriber_ref: &str, contacts: &[PaceContact]) -> PaceDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();
    let mut roles_seen = BTreeSet::new();
    let mut role_duplicate = false;

    if subscriber_ref.trim().is_empty() {
        reasons.insert("P.A.C.E. evaluation requires a subscriber reference.".to_string());
        required_evidence
            .insert("Synthetic subscriber reference for onboarding state.".to_string());
    }

    for contact in contacts {
        if contact.subscriber_ref != subscriber_ref {
            reasons.insert(
                "P.A.C.E. contacts must be bound to the onboarding subscriber reference."
                    .to_string(),
            );
            required_evidence.insert(
                "Subscriber-scoped P.A.C.E. contact references without raw contact details."
                    .to_string(),
            );
        }

        if contact.contact_ref == subscriber_ref {
            reasons
                .insert("P.A.C.E. contacts must not self-grant subscriber authority.".to_string());
            required_evidence
                .insert("Distinct P.A.C.E. contact reference for each role.".to_string());
        }

        if !roles_seen.insert(contact.role) {
            role_duplicate = true;
        }

        if contact.acceptance_state == PaceAcceptanceState::Accepted && !contact.obligation_accepted
        {
            reasons.insert(
                "Accepted P.A.C.E. contacts must accept the social-contract obligation."
                    .to_string(),
            );
            required_evidence
                .insert("Signed or confirmed P.A.C.E. obligation acceptance state.".to_string());
        }
    }

    if role_duplicate {
        reasons.insert("P.A.C.E. contact roles must be distinct.".to_string());
        required_evidence.insert("One active contact for each P.A.C.E. role.".to_string());
    }

    if REQUIRED_PACE_ROLES
        .iter()
        .any(|required_role| !roles_seen.contains(required_role))
    {
        reasons.insert(
            "P.A.C.E. contacts must include one Primary, Alternate, Contingent, and Emergency role."
                .to_string(),
        );
        required_evidence.insert(
            "Primary, Alternate, Contingent, and Emergency contact references.".to_string(),
        );
    }

    PaceDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

pub fn evaluate_onboarding_progress(
    subscriber_ref: &str,
    state: &OnboardingState,
) -> OnboardingDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if state.entitlement == EntitlementSelectionState::NotSelected {
        required_evidence.insert(
            "Entitlement selection for free basic, family, team, trial, gift, or frontline path."
                .to_string(),
        );
    }

    if !state.account_created {
        return progress_decision(
            false,
            OnboardingNextAction::CreateAccount,
            reasons,
            required_evidence,
        );
    }

    if !state.emergency_card_configured {
        return progress_decision(
            false,
            OnboardingNextAction::ConfigureEmergencyCard,
            reasons,
            required_evidence,
        );
    }

    let pace_decision = evaluate_pace_contacts(subscriber_ref, &state.pace_contacts);
    if !pace_decision.allowed || !pace_notifications_ready(&state.pace_contacts) {
        reasons.extend(pace_decision.reasons);
        required_evidence.extend(pace_decision.required_evidence);
        if !pace_notifications_ready(&state.pace_contacts) {
            reasons.insert(
                "P.A.C.E. contacts must be accepted and notification eligible before onboarding can advance."
                    .to_string(),
            );
            required_evidence.insert(
                "Accepted P.A.C.E. invitations with active notification eligibility.".to_string(),
            );
        }
        return progress_decision(
            false,
            OnboardingNextAction::InvitePaceContacts,
            reasons,
            required_evidence,
        );
    }

    if !state.medical_jacket_started {
        return progress_decision(
            false,
            OnboardingNextAction::StartMedicalJacket,
            reasons,
            required_evidence,
        );
    }

    if !state.medical_jacket_complete {
        return progress_decision(
            false,
            OnboardingNextAction::CompleteMedicalJacket,
            reasons,
            required_evidence,
        );
    }

    if state.entitlement == EntitlementSelectionState::NotSelected {
        return progress_decision(
            false,
            OnboardingNextAction::SelectEntitlement,
            reasons,
            required_evidence,
        );
    }

    progress_decision(
        true,
        OnboardingNextAction::ReadyForActivation,
        reasons,
        required_evidence,
    )
}

fn pace_notifications_ready(contacts: &[PaceContact]) -> bool {
    REQUIRED_PACE_ROLES.iter().all(|role| {
        contacts
            .iter()
            .any(|contact| contact.role == *role && notification_eligible(contact))
    })
}

fn progress_decision(
    complete: bool,
    next_action: OnboardingNextAction,
    reasons: BTreeSet<String>,
    required_evidence: BTreeSet<String>,
) -> OnboardingDecision {
    OnboardingDecision {
        allowed: reasons.is_empty(),
        complete,
        next_action,
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
