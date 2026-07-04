use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultInteractionMode {
    OwnerRead,
    OwnerWrite,
    Tier0ResponderRead,
    PaceDelegateRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultRecordClass {
    MedicalJacketPointer,
    ConsentReceiptPointer,
    PaceContactPointer,
    EmergencyInstructionPointer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultRuntimeState {
    Inactive,
    VerifiedPermit,
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultDisclosureScope {
    MetadataOnly,
    EmergencySubset,
    FullRecordExport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VitalLockVaultRequest {
    pub vault_ref: String,
    pub record_ref: String,
    pub policy_ref: String,
    pub session_ref: String,
    pub mode: VaultInteractionMode,
    pub record_class: VaultRecordClass,
    pub runtime_state: VaultRuntimeState,
    pub disclosure_scope: VaultDisclosureScope,
    pub storage_contract_passed: bool,
    pub custody_contract_passed: bool,
    pub authorization_active: bool,
    pub includes_raw_sensitive_payload: bool,
    pub includes_direct_contact_value: bool,
    pub includes_location_trace: bool,
    pub has_disablement_ref: bool,
    pub is_synthetic_fixture: bool,
    pub claims_verified_vault_protection: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultInteractionDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_vitallock_vault_interaction(
    request: VitalLockVaultRequest,
) -> VaultInteractionDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.vault_ref.trim().is_empty()
        || request.record_ref.trim().is_empty()
        || request.policy_ref.trim().is_empty()
        || request.session_ref.trim().is_empty()
    {
        reasons.insert(
            "VitalLock vault interactions require synthetic vault, record, policy, and session references."
                .into(),
        );
        required_evidence.insert(
            "Synthetic vault reference, record reference, policy reference, and interaction session reference."
                .into(),
        );
    }

    if !request.is_synthetic_fixture {
        reasons.insert(
            "VitalLock vault fixtures must remain synthetic until a verified runtime vault path exists."
                .into(),
        );
        required_evidence.insert(
            "Synthetic-only VitalLock vault fixtures until a verified runtime vault path exists."
                .into(),
        );
    }

    if !request.storage_contract_passed {
        reasons.insert(
            "VitalLock vault interactions depend on a passing storage entitlement contract.".into(),
        );
        required_evidence.insert(
            "Passing storage entitlement contract evidence for the referenced vault record.".into(),
        );
    }

    if matches!(
        request.record_class,
        VaultRecordClass::MedicalJacketPointer | VaultRecordClass::ConsentReceiptPointer
    ) && !request.custody_contract_passed
    {
        reasons.insert(
            "Medical-jacket and consent-bound vault interactions depend on a passing custody or consent contract."
                .into(),
        );
        required_evidence.insert(
            "Passing medical-jacket custody or consent-receipt contract evidence for the referenced vault record."
                .into(),
        );
    }

    if request.includes_raw_sensitive_payload {
        reasons.insert(
            "VitalLock vault interactions must stay metadata-only and exclude raw sensitive records."
                .into(),
        );
        required_evidence.insert(
            "Interaction payload review proving that only metadata-safe references are exposed."
                .into(),
        );
    }

    if request.includes_direct_contact_value {
        reasons.insert("VitalLock vault interactions must not embed direct contact values.".into());
        required_evidence.insert(
            "Configuration-backed contact presentation instead of vault-embedded direct contact values."
                .into(),
        );
    }

    if request.includes_location_trace {
        reasons.insert(
            "VitalLock vault interactions must not embed location traces or responder-tracking data."
                .into(),
        );
        required_evidence.insert(
            "Policy review proving location traces remain outside the vault interaction payload."
                .into(),
        );
    }

    if request.disclosure_scope == VaultDisclosureScope::FullRecordExport {
        reasons.insert(
            "Full VitalLock vault export remains blocked until a verified export policy exists."
                .into(),
        );
        required_evidence.insert(
            "Verified export policy and data-class review before any full vault export path can activate."
                .into(),
        );
    }

    if matches!(
        request.mode,
        VaultInteractionMode::Tier0ResponderRead | VaultInteractionMode::PaceDelegateRead
    ) && !request.has_disablement_ref
    {
        reasons.insert(
            "VitalLock vault routes require a disablement reference before delegated or responder access can be shown."
                .into(),
        );
        required_evidence.insert(
            "Disablement reference covering delegated and responder-facing VitalLock vault behavior."
                .into(),
        );
    }

    if matches!(
        request.mode,
        VaultInteractionMode::Tier0ResponderRead | VaultInteractionMode::PaceDelegateRead
    ) && request.runtime_state != VaultRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Responder and P.A.C.E. vault access remain inactive until a verified adapter path returns permit."
                .into(),
        );
        required_evidence.insert(
            "Verified adapter path returning permit for responder or delegated VitalLock vault access."
                .into(),
        );
    }

    if matches!(
        request.mode,
        VaultInteractionMode::Tier0ResponderRead | VaultInteractionMode::PaceDelegateRead
    ) && !request.authorization_active
    {
        reasons.insert(
            "VitalLock vault reads require active authorization even for emergency or delegated access."
                .into(),
        );
        required_evidence.insert(
            "Current delegated or Tier-0 authorization evidence for the requested vault interaction."
                .into(),
        );
    }

    if request.mode == VaultInteractionMode::Tier0ResponderRead
        && request.disclosure_scope != VaultDisclosureScope::EmergencySubset
    {
        reasons.insert(
            "Tier-0 responder vault access is limited to the approved emergency subset.".into(),
        );
        required_evidence.insert(
            "Emergency-subset-only responder projection contract for VitalLock vault interactions."
                .into(),
        );
    }

    if request.mode == VaultInteractionMode::PaceDelegateRead
        && request.disclosure_scope != VaultDisclosureScope::MetadataOnly
    {
        reasons.insert(
            "P.A.C.E. delegated vault access is limited to metadata-only interaction until broader policy is verified."
                .into(),
        );
        required_evidence.insert(
            "Verified delegated-access policy before any non-metadata VitalLock vault interaction can activate."
                .into(),
        );
    }

    if request.claims_verified_vault_protection
        && request.runtime_state != VaultRuntimeState::VerifiedPermit
    {
        reasons.insert(
            "Verified VitalLock vault protection claims are blocked unless the interaction path is in a verified permit state."
                .into(),
        );
        required_evidence.insert(
            "Verified permit-state evidence before any VitalLock protection claim is shown.".into(),
        );
    }

    VaultInteractionDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
