use livesafe::storage_entitlement::{
    BillingMode, BillingState, CONTENT_ADDRESSED_PROVIDERS, INITIAL_STORAGE_LEVEL_CODES,
    LIVESAFE_INITIAL_STORAGE_OFFERING, ProviderKind, ReadOperation, StorageAuthorizationState,
    StorageLevelCode, VaultStorageWriteRequest, WriteOperation, evaluate_vault_storage_access,
    evaluate_vault_storage_write, validate_storage_offering,
};

#[test]
fn initial_offering_includes_paid_storage_levels() {
    assert_eq!(
        INITIAL_STORAGE_LEVEL_CODES,
        [
            StorageLevelCode::BasicIncluded,
            StorageLevelCode::PersonalPaid,
            StorageLevelCode::FamilyPaid,
            StorageLevelCode::TeamPaid
        ]
    );

    let decision = validate_storage_offering(&LIVESAFE_INITIAL_STORAGE_OFFERING);

    assert!(decision.allowed, "{decision:?}");
    let paid_initial_codes: Vec<StorageLevelCode> = LIVESAFE_INITIAL_STORAGE_OFFERING
        .iter()
        .filter(|level| level.included_in_initial_offering && level.paid_storage_level)
        .map(|level| level.code)
        .collect();

    assert_eq!(
        paid_initial_codes,
        [
            StorageLevelCode::PersonalPaid,
            StorageLevelCode::FamilyPaid,
            StorageLevelCode::TeamPaid
        ]
    );
}

#[test]
fn paid_storage_levels_require_stripe_catalog_or_custom_contract() {
    let mut offering = LIVESAFE_INITIAL_STORAGE_OFFERING.to_vec();
    offering[1].stripe_catalog_required = false;
    offering[2].quota_mib = 0;

    let decision = validate_storage_offering(&offering);

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"Paid storage levels require Stripe catalog binding.".into())
    );
    assert!(
        decision
            .reasons
            .contains(&"Storage levels must declare a positive quota.".into())
    );
}

#[test]
fn initial_offering_requires_ipfs_or_content_addressed_provider() {
    let mut offering = LIVESAFE_INITIAL_STORAGE_OFFERING.to_vec();
    for level in &mut offering {
        level
            .provider_kinds
            .retain(|provider| !CONTENT_ADDRESSED_PROVIDERS.contains(provider));
    }

    let decision = validate_storage_offering(&offering);

    assert!(!decision.allowed);
    assert!(decision.reasons.contains(
        &"Initial storage offering must include IPFS or another content-addressed provider option.".into()
    ));
}

#[test]
fn ipfs_style_storage_rejects_raw_or_human_readable_sensitive_material() {
    let decision = evaluate_vault_storage_write(VaultStorageWriteRequest {
        provider_kind: ProviderKind::IpfsContentAddressed,
        encrypted_before_provider: false,
        content_addressed: true,
        stores_raw_sensitive_data: true,
        provider_metadata_includes_sensitive_labels: true,
        exochain_anchor_fields: vec!["cid".into(), "commitment".into()],
        current_usage_mib: 100,
        requested_write_mib: 10,
        storage_quota_mib: 1024,
        billing_state: BillingState::Current,
        operation: WriteOperation::UserUpload,
    });

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"Vault storage providers may receive only encrypted blobs.".into())
    );
    assert!(decision.reasons.contains(
        &"Raw sensitive data must not be written to IPFS, content-addressed storage, object storage, logs, or fixtures.".into()
    ));
    assert!(
        decision.reasons.contains(
            &"Provider metadata must not include human-readable sensitive labels.".into()
        )
    );
}

#[test]
fn encrypted_content_addressed_writes_with_safe_exochain_anchors_are_allowed() {
    let decision = evaluate_vault_storage_write(VaultStorageWriteRequest {
        provider_kind: ProviderKind::IpfsContentAddressed,
        encrypted_before_provider: true,
        content_addressed: true,
        stores_raw_sensitive_data: false,
        provider_metadata_includes_sensitive_labels: false,
        exochain_anchor_fields: vec![
            "cid".into(),
            "commitment".into(),
            "custody-receipt".into(),
            "policy-reference".into(),
        ],
        current_usage_mib: 2048,
        requested_write_mib: 512,
        storage_quota_mib: 5120,
        billing_state: BillingState::Current,
        operation: WriteOperation::GeneticImport,
    });

    assert_eq!(decision.reasons, Vec::<String>::new());
    assert_eq!(decision.required_evidence, Vec::<String>::new());
    assert!(decision.allowed);
}

#[test]
fn unsafe_exochain_anchor_fields_are_denied() {
    let decision = evaluate_vault_storage_write(VaultStorageWriteRequest {
        provider_kind: ProviderKind::ManagedVaultStore,
        encrypted_before_provider: true,
        content_addressed: false,
        stores_raw_sensitive_data: false,
        provider_metadata_includes_sensitive_labels: false,
        exochain_anchor_fields: vec!["commitment".into(), "raw-medical-record".into()],
        current_usage_mib: 10,
        requested_write_mib: 1,
        storage_quota_mib: 100,
        billing_state: BillingState::Current,
        operation: WriteOperation::UserUpload,
    });

    assert!(!decision.allowed);
    assert!(decision.reasons.contains(
        &"EXOCHAIN storage anchors may include only safe commitments, references, and receipts.".into()
    ));
}

#[test]
fn storage_writes_enforce_quota_and_billing_state() {
    let decision = evaluate_vault_storage_write(VaultStorageWriteRequest {
        provider_kind: ProviderKind::S3CompatibleObjectStore,
        encrypted_before_provider: true,
        content_addressed: false,
        stores_raw_sensitive_data: false,
        provider_metadata_includes_sensitive_labels: false,
        exochain_anchor_fields: vec!["commitment".into()],
        current_usage_mib: 900,
        requested_write_mib: 200,
        storage_quota_mib: 1024,
        billing_state: BillingState::PastDue,
        operation: WriteOperation::UserUpload,
    });

    assert!(!decision.allowed);
    assert!(
        decision
            .reasons
            .contains(&"Storage write exceeds the account storage level.".into())
    );
    assert!(decision.reasons.contains(
        &"Paid storage writes require current billing, active trial, gift, or frontline entitlement.".into()
    ));
}

#[test]
fn tier0_emergency_reads_ignore_billing_and_quota_but_not_authorization() {
    let allowed = evaluate_vault_storage_access(
        ReadOperation::Tier0EmergencyRead,
        BillingState::PastDue,
        0,
        9000,
        4,
        StorageAuthorizationState::Authorized,
    );

    assert!(allowed.allowed, "{allowed:?}");

    let denied = evaluate_vault_storage_access(
        ReadOperation::Tier0EmergencyRead,
        BillingState::PastDue,
        0,
        9000,
        4,
        StorageAuthorizationState::Denied,
    );

    assert!(!denied.allowed);
    assert!(denied.reasons.contains(
        &"Vault reads require authorization even when billing is bypassed for Tier-0 emergency access.".into()
    ));
}

#[test]
fn enterprise_custom_storage_uses_custom_contract_not_stripe_recurring() {
    let enterprise = LIVESAFE_INITIAL_STORAGE_OFFERING
        .iter()
        .find(|level| level.code == StorageLevelCode::EnterpriseCustom)
        .expect("enterprise storage level should exist");

    assert_eq!(enterprise.billing_mode, BillingMode::CustomContract);
    assert!(enterprise.paid_storage_level);
    assert!(!enterprise.included_in_initial_offering);
}
