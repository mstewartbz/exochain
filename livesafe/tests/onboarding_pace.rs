use livesafe::onboarding_pace::{
    EntitlementSelectionState, OnboardingNextAction, OnboardingState, PaceAcceptanceState,
    PaceContact, PaceRole, evaluate_onboarding_progress, evaluate_pace_contacts,
    notification_eligible,
};

fn accepted_contact(role: PaceRole, contact_ref: &str) -> PaceContact {
    PaceContact {
        role,
        subscriber_ref: "subscriber:synthetic-001".into(),
        contact_ref: contact_ref.into(),
        acceptance_state: PaceAcceptanceState::Accepted,
        obligation_accepted: true,
        replacement_for: None,
    }
}

fn accepted_pace_set() -> Vec<PaceContact> {
    vec![
        accepted_contact(PaceRole::Primary, "pace:primary"),
        accepted_contact(PaceRole::Alternate, "pace:alternate"),
        accepted_contact(PaceRole::Contingent, "pace:contingent"),
        accepted_contact(PaceRole::Emergency, "pace:emergency"),
    ]
}

#[test]
fn pace_contact_contract_requires_distinct_roles_and_blocks_self_grant() {
    let contacts = vec![
        accepted_contact(PaceRole::Primary, "subscriber:synthetic-001"),
        accepted_contact(PaceRole::Primary, "pace:duplicate-primary"),
        accepted_contact(PaceRole::Alternate, "pace:alternate"),
        accepted_contact(PaceRole::Contingent, "pace:contingent"),
    ];

    let decision = evaluate_pace_contacts("subscriber:synthetic-001", &contacts);

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"P.A.C.E. contacts must not self-grant subscriber authority.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"P.A.C.E. contacts must include one Primary, Alternate, Contingent, and Emergency role.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"P.A.C.E. contact roles must be distinct.".into())
    );
}

#[test]
fn accepted_pace_set_moves_onboarding_to_medical_jacket_completion() {
    let state = OnboardingState {
        account_created: true,
        emergency_card_configured: true,
        pace_contacts: accepted_pace_set(),
        medical_jacket_started: true,
        medical_jacket_complete: false,
        entitlement: EntitlementSelectionState::BasicFree,
    };

    let progress = evaluate_onboarding_progress("subscriber:synthetic-001", &state);

    assert!(!progress.complete);
    assert_eq!(
        progress.next_action,
        OnboardingNextAction::CompleteMedicalJacket
    );
    assert_eq!(progress.reasons, Vec::<String>::new());
}

#[test]
fn pending_declined_revoked_or_replaced_contacts_are_not_notification_eligible() {
    let mut invited = accepted_contact(PaceRole::Primary, "pace:invited");
    invited.acceptance_state = PaceAcceptanceState::Invited;
    invited.obligation_accepted = false;

    let mut declined = accepted_contact(PaceRole::Alternate, "pace:declined");
    declined.acceptance_state = PaceAcceptanceState::Declined;

    let mut revoked = accepted_contact(PaceRole::Contingent, "pace:revoked");
    revoked.acceptance_state = PaceAcceptanceState::Revoked;

    let mut replaced = accepted_contact(PaceRole::Emergency, "pace:replaced");
    replaced.acceptance_state = PaceAcceptanceState::Replaced;
    replaced.replacement_for = Some("pace:prior-emergency".into());

    assert!(!notification_eligible(&invited));
    assert!(!notification_eligible(&declined));
    assert!(!notification_eligible(&revoked));
    assert!(!notification_eligible(&replaced));
    assert!(notification_eligible(&accepted_contact(
        PaceRole::Emergency,
        "pace:accepted"
    )));
}

#[test]
fn complete_onboarding_requires_account_card_pace_medical_jacket_and_entitlement() {
    let incomplete = OnboardingState {
        account_created: true,
        emergency_card_configured: true,
        pace_contacts: accepted_pace_set(),
        medical_jacket_started: false,
        medical_jacket_complete: false,
        entitlement: EntitlementSelectionState::NotSelected,
    };

    let blocked = evaluate_onboarding_progress("subscriber:synthetic-001", &incomplete);

    assert!(!blocked.complete);
    assert_eq!(
        blocked.next_action,
        OnboardingNextAction::StartMedicalJacket
    );
    assert!(
        blocked.required_evidence.contains(
            &"Entitlement selection for free basic, family, team, trial, gift, or frontline path."
                .into()
        )
    );

    let complete = OnboardingState {
        account_created: true,
        emergency_card_configured: true,
        pace_contacts: accepted_pace_set(),
        medical_jacket_started: true,
        medical_jacket_complete: true,
        entitlement: EntitlementSelectionState::FamilyPaid,
    };

    let ready = evaluate_onboarding_progress("subscriber:synthetic-001", &complete);

    assert!(ready.complete);
    assert_eq!(ready.next_action, OnboardingNextAction::ReadyForActivation);
    assert_eq!(ready.reasons, Vec::<String>::new());
}
