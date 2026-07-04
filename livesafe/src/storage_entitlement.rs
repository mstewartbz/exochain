use std::collections::BTreeSet;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProviderKind {
    IpfsContentAddressed,
    FilecoinContentAddressed,
    S3CompatibleObjectStore,
    ManagedVaultStore,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StorageLevelCode {
    BasicIncluded,
    PersonalPaid,
    FamilyPaid,
    TeamPaid,
    EnterpriseCustom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BillingMode {
    Included,
    StripeRecurring,
    StripeMetered,
    CustomContract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BillingState {
    Current,
    Trial,
    Gift,
    FrontlineFree,
    PastDue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteOperation {
    UserUpload,
    MedicalImport,
    GeneticImport,
    AmbientExport,
    RetentionExport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadOperation {
    Tier0EmergencyRead,
    UserVaultRead,
    FamilyPlanRead,
    ExportRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageAuthorizationState {
    Authorized,
    Denied,
    Expired,
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLevelDefinition {
    pub code: StorageLevelCode,
    pub label: &'static str,
    pub billing_mode: BillingMode,
    pub quota_mib: u64,
    pub paid_storage_level: bool,
    pub stripe_catalog_required: bool,
    pub included_in_initial_offering: bool,
    pub provider_kinds: Vec<ProviderKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultStorageWriteRequest {
    pub provider_kind: ProviderKind,
    pub encrypted_before_provider: bool,
    pub content_addressed: bool,
    pub stores_raw_sensitive_data: bool,
    pub provider_metadata_includes_sensitive_labels: bool,
    pub exochain_anchor_fields: Vec<String>,
    pub current_usage_mib: u64,
    pub requested_write_mib: u64,
    pub storage_quota_mib: u64,
    pub billing_state: BillingState,
    pub operation: WriteOperation,
}

pub const INITIAL_STORAGE_LEVEL_CODES: [StorageLevelCode; 4] = [
    StorageLevelCode::BasicIncluded,
    StorageLevelCode::PersonalPaid,
    StorageLevelCode::FamilyPaid,
    StorageLevelCode::TeamPaid,
];

pub const CONTENT_ADDRESSED_PROVIDERS: [ProviderKind; 2] = [
    ProviderKind::IpfsContentAddressed,
    ProviderKind::FilecoinContentAddressed,
];

const SAFE_EXOCHAIN_ANCHOR_FIELDS: [&str; 6] = [
    "cid",
    "commitment",
    "custody-receipt",
    "policy-reference",
    "retention-policy-reference",
    "encryption-key-commitment",
];

const INITIAL_PROVIDER_SET: [ProviderKind; 4] = [
    ProviderKind::ManagedVaultStore,
    ProviderKind::IpfsContentAddressed,
    ProviderKind::FilecoinContentAddressed,
    ProviderKind::S3CompatibleObjectStore,
];

pub static LIVESAFE_INITIAL_STORAGE_OFFERING: LazyLock<Vec<StorageLevelDefinition>> =
    LazyLock::new(|| {
        vec![
            StorageLevelDefinition {
                code: StorageLevelCode::BasicIncluded,
                label: "Basic Included Vault Storage",
                billing_mode: BillingMode::Included,
                quota_mib: 512,
                paid_storage_level: false,
                stripe_catalog_required: false,
                included_in_initial_offering: true,
                provider_kinds: INITIAL_PROVIDER_SET.to_vec(),
            },
            StorageLevelDefinition {
                code: StorageLevelCode::PersonalPaid,
                label: "Personal Paid Vault Storage",
                billing_mode: BillingMode::StripeRecurring,
                quota_mib: 10_240,
                paid_storage_level: true,
                stripe_catalog_required: true,
                included_in_initial_offering: true,
                provider_kinds: INITIAL_PROVIDER_SET.to_vec(),
            },
            StorageLevelDefinition {
                code: StorageLevelCode::FamilyPaid,
                label: "Family Paid Vault Storage",
                billing_mode: BillingMode::StripeRecurring,
                quota_mib: 51_200,
                paid_storage_level: true,
                stripe_catalog_required: true,
                included_in_initial_offering: true,
                provider_kinds: INITIAL_PROVIDER_SET.to_vec(),
            },
            StorageLevelDefinition {
                code: StorageLevelCode::TeamPaid,
                label: "Team Paid Vault Storage",
                billing_mode: BillingMode::StripeRecurring,
                quota_mib: 204_800,
                paid_storage_level: true,
                stripe_catalog_required: true,
                included_in_initial_offering: true,
                provider_kinds: INITIAL_PROVIDER_SET.to_vec(),
            },
            StorageLevelDefinition {
                code: StorageLevelCode::EnterpriseCustom,
                label: "Enterprise Custom Vault Storage",
                billing_mode: BillingMode::CustomContract,
                quota_mib: 1_048_576,
                paid_storage_level: true,
                stripe_catalog_required: false,
                included_in_initial_offering: false,
                provider_kinds: INITIAL_PROVIDER_SET.to_vec(),
            },
        ]
    });

pub fn validate_storage_offering(levels: &[StorageLevelDefinition]) -> StorageDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();
    let initial_level_codes: BTreeSet<StorageLevelCode> = levels
        .iter()
        .filter(|level| level.included_in_initial_offering)
        .map(|level| level.code)
        .collect();

    for required_code in INITIAL_STORAGE_LEVEL_CODES {
        if !initial_level_codes.contains(&required_code) {
            reasons.insert(format!(
                "Initial storage offering is missing {}.",
                storage_level_code(required_code)
            ));
            required_evidence
                .insert("Initial storage catalog with free and paid storage levels.".to_string());
        }
    }

    if !levels
        .iter()
        .filter(|level| level.included_in_initial_offering)
        .any(|level| {
            level
                .provider_kinds
                .iter()
                .any(|provider| CONTENT_ADDRESSED_PROVIDERS.contains(provider))
        })
    {
        reasons.insert(
            "Initial storage offering must include IPFS or another content-addressed provider option."
                .to_string(),
        );
        required_evidence
            .insert("Content-addressed encrypted blob storage configuration.".to_string());
    }

    for level in levels {
        if level.quota_mib == 0 {
            reasons.insert("Storage levels must declare a positive quota.".to_string());
            required_evidence.insert(format!(
                "Positive integer quota for {}.",
                storage_level_code(level.code)
            ));
        }

        if level.paid_storage_level
            && level.billing_mode != BillingMode::CustomContract
            && !level.stripe_catalog_required
        {
            reasons.insert("Paid storage levels require Stripe catalog binding.".to_string());
            required_evidence.insert(format!(
                "Stripe product and price ids for {}.",
                storage_level_code(level.code)
            ));
        }

        if !level.paid_storage_level && level.billing_mode != BillingMode::Included {
            reasons.insert("Included storage levels must use included billing mode.".to_string());
            required_evidence.insert(format!(
                "Billing-mode review for {}.",
                storage_level_code(level.code)
            ));
        }

        if level.provider_kinds.is_empty() {
            reasons.insert("Storage levels must declare at least one provider kind.".to_string());
            required_evidence.insert(format!(
                "Provider configuration for {}.",
                storage_level_code(level.code)
            ));
        }
    }

    storage_decision(reasons, required_evidence)
}

pub fn evaluate_vault_storage_write(request: VaultStorageWriteRequest) -> StorageDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if !request.encrypted_before_provider {
        reasons.insert("Vault storage providers may receive only encrypted blobs.".to_string());
        required_evidence.insert(
            "Client or server envelope-encryption proof before provider write.".to_string(),
        );
    }

    if request.stores_raw_sensitive_data {
        reasons.insert(
            "Raw sensitive data must not be written to IPFS, content-addressed storage, object storage, logs, or fixtures."
                .to_string(),
        );
        required_evidence.insert("Storage payload redaction and encryption review.".to_string());
    }

    if request.provider_metadata_includes_sensitive_labels {
        reasons.insert(
            "Provider metadata must not include human-readable sensitive labels.".to_string(),
        );
        required_evidence.insert("Opaque object naming and metadata policy.".to_string());
    }

    if CONTENT_ADDRESSED_PROVIDERS.contains(&request.provider_kind) && !request.content_addressed {
        reasons.insert("IPFS and Filecoin provider writes must be content-addressed.".to_string());
        required_evidence.insert("CID or equivalent content-addressed reference.".to_string());
    }

    if request
        .exochain_anchor_fields
        .iter()
        .any(|field| !SAFE_EXOCHAIN_ANCHOR_FIELDS.contains(&field.as_str()))
    {
        reasons.insert(
            "EXOCHAIN storage anchors may include only safe commitments, references, and receipts."
                .to_string(),
        );
        required_evidence.insert(
            "Anchor schema limited to CID, commitments, policy references, and receipts."
                .to_string(),
        );
    }

    if request.requested_write_mib == 0 {
        reasons.insert("Storage writes must declare a positive integer size.".to_string());
        required_evidence.insert("Measured encrypted blob size before write.".to_string());
    }

    let projected_usage = request
        .current_usage_mib
        .saturating_add(request.requested_write_mib);
    if projected_usage > request.storage_quota_mib {
        reasons.insert("Storage write exceeds the account storage level.".to_string());
        required_evidence.insert(
            "Upgrade, gift, trial, or storage-level change before accepting write.".to_string(),
        );
    }

    if request.billing_state == BillingState::PastDue {
        reasons.insert(
            "Paid storage writes require current billing, active trial, gift, or frontline entitlement."
                .to_string(),
        );
        required_evidence.insert("Current entitlement state before write.".to_string());
    }

    storage_decision(reasons, required_evidence)
}

pub fn evaluate_vault_storage_access(
    operation: ReadOperation,
    billing_state: BillingState,
    storage_quota_mib: u64,
    current_usage_mib: u64,
    requested_read_mib: u64,
    authorization_state: StorageAuthorizationState,
) -> StorageDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if authorization_state != StorageAuthorizationState::Authorized {
        reasons.insert(
            "Vault reads require authorization even when billing is bypassed for Tier-0 emergency access."
                .to_string(),
        );
        required_evidence
            .insert("Current consent, authority, or emergency-access grant.".to_string());
    }

    if operation != ReadOperation::Tier0EmergencyRead {
        if billing_state == BillingState::PastDue {
            reasons.insert(
                "Non-emergency vault reads require current billing or active entitlement."
                    .to_string(),
            );
            required_evidence
                .insert("Current billing, trial, gift, or frontline entitlement.".to_string());
        }

        if current_usage_mib > storage_quota_mib {
            reasons.insert(
                "Non-emergency vault reads require storage account remediation when over quota."
                    .to_string(),
            );
            required_evidence.insert("Storage remediation or upgraded level.".to_string());
        }
    }

    if requested_read_mib == 0 {
        reasons.insert("Vault reads must declare a positive integer read size.".to_string());
        required_evidence.insert("Measured encrypted blob read size.".to_string());
    }

    storage_decision(reasons, required_evidence)
}

fn storage_decision(
    reasons: BTreeSet<String>,
    required_evidence: BTreeSet<String>,
) -> StorageDecision {
    StorageDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn storage_level_code(code: StorageLevelCode) -> &'static str {
    match code {
        StorageLevelCode::BasicIncluded => "basic-included",
        StorageLevelCode::PersonalPaid => "personal-paid",
        StorageLevelCode::FamilyPaid => "family-paid",
        StorageLevelCode::TeamPaid => "team-paid",
        StorageLevelCode::EnterpriseCustom => "enterprise-custom",
    }
}
