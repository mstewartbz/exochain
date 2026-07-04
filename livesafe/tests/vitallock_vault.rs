use livesafe::vitallock_vault::{
    VaultDisclosureScope, VaultInteractionDecision, VaultInteractionMode, VaultRecordClass,
    VaultRuntimeState, VitalLockVaultRequest, evaluate_vitallock_vault_interaction,
};

fn valid_request() -> VitalLockVaultRequest {
    VitalLockVaultRequest {
        vault_ref: "vitallock:vault:synthetic".into(),
        record_ref: "vault-record:synthetic".into(),
        policy_ref: "policy:vitallock-vault:current".into(),
        session_ref: "vault-session:synthetic".into(),
        mode: VaultInteractionMode::Tier0ResponderRead,
        record_class: VaultRecordClass::MedicalJacketPointer,
        runtime_state: VaultRuntimeState::VerifiedPermit,
        disclosure_scope: VaultDisclosureScope::EmergencySubset,
        storage_contract_passed: true,
        custody_contract_passed: true,
        authorization_active: true,
        includes_raw_sensitive_payload: false,
        includes_direct_contact_value: false,
        includes_location_trace: false,
        has_disablement_ref: true,
        is_synthetic_fixture: true,
        claims_verified_vault_protection: false,
    }
}

fn assert_denied(decision: &VaultInteractionDecision, reason: &str) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn vitallock_vault_requires_references_and_synthetic_fixtures() {
    let mut request = valid_request();
    request.vault_ref = String::new();
    request.record_ref = String::new();
    request.policy_ref = String::new();
    request.session_ref = String::new();
    request.is_synthetic_fixture = false;

    let decision = evaluate_vitallock_vault_interaction(request);

    assert_denied(
        &decision,
        "VitalLock vault interactions require synthetic vault, record, policy, and session references.",
    );
    assert_denied(
        &decision,
        "VitalLock vault fixtures must remain synthetic until a verified runtime vault path exists.",
    );
}

#[test]
fn vitallock_vault_requires_storage_and_custody_contracts() {
    let mut request = valid_request();
    request.storage_contract_passed = false;
    request.custody_contract_passed = false;

    let decision = evaluate_vitallock_vault_interaction(request);

    assert_denied(
        &decision,
        "VitalLock vault interactions depend on a passing storage entitlement contract.",
    );
    assert_denied(
        &decision,
        "Medical-jacket and consent-bound vault interactions depend on a passing custody or consent contract.",
    );
}

#[test]
fn vitallock_vault_denies_raw_sensitive_contact_and_location_payloads() {
    let mut request = valid_request();
    request.includes_raw_sensitive_payload = true;
    request.includes_direct_contact_value = true;
    request.includes_location_trace = true;

    let decision = evaluate_vitallock_vault_interaction(request);

    assert_denied(
        &decision,
        "VitalLock vault interactions must stay metadata-only and exclude raw sensitive records.",
    );
    assert_denied(
        &decision,
        "VitalLock vault interactions must not embed direct contact values.",
    );
    assert_denied(
        &decision,
        "VitalLock vault interactions must not embed location traces or responder-tracking data.",
    );
}

#[test]
fn responder_and_pace_access_require_verified_permit_and_authorization() {
    for mode in [
        VaultInteractionMode::Tier0ResponderRead,
        VaultInteractionMode::PaceDelegateRead,
    ] {
        let mut request = valid_request();
        request.mode = mode;
        request.runtime_state = VaultRuntimeState::Inactive;
        request.authorization_active = false;

        let decision = evaluate_vitallock_vault_interaction(request);

        assert_denied(
            &decision,
            "Responder and P.A.C.E. vault access remain inactive until a verified adapter path returns permit.",
        );
        assert_denied(
            &decision,
            "VitalLock vault reads require active authorization even for emergency or delegated access.",
        );
    }
}

#[test]
fn vault_interactions_block_full_export_and_require_disablement() {
    let mut request = valid_request();
    request.mode = VaultInteractionMode::PaceDelegateRead;
    request.disclosure_scope = VaultDisclosureScope::FullRecordExport;
    request.has_disablement_ref = false;

    let decision = evaluate_vitallock_vault_interaction(request);

    assert_denied(
        &decision,
        "Full VitalLock vault export remains blocked until a verified export policy exists.",
    );
    assert_denied(
        &decision,
        "VitalLock vault routes require a disablement reference before delegated or responder access can be shown.",
    );
}

#[test]
fn owner_vault_reads_can_remain_inactive_without_verified_claims() {
    let mut request = valid_request();
    request.mode = VaultInteractionMode::OwnerRead;
    request.runtime_state = VaultRuntimeState::Inactive;
    request.disclosure_scope = VaultDisclosureScope::MetadataOnly;

    let decision = evaluate_vitallock_vault_interaction(request);

    assert!(decision.allowed, "{decision:?}");
}

#[test]
fn verified_vault_claims_require_verified_permit_state() {
    let mut request = valid_request();
    request.claims_verified_vault_protection = true;
    request.runtime_state = VaultRuntimeState::Unavailable;

    let decision = evaluate_vitallock_vault_interaction(request);

    assert_denied(
        &decision,
        "Verified VitalLock vault protection claims are blocked unless the interaction path is in a verified permit state.",
    );
}
