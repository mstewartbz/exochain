use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyPresence {
    Missing,
    Present,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationState {
    Missing,
    Present,
    Passed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptMetadataHandling {
    SafeOnly,
    IncludesRawSensitiveData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyDataClass {
    PhenotypicalMedicalSummary,
    LegacyCharterMetadata,
    LegacyCharterContents,
    GeneticPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyOperation {
    EmergencyTier0Read,
    LegacyCharterReview,
    PosthumousRepresentation,
    GeneticUnveiling,
    Erasure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErasureEvidenceKind {
    NotRequested,
    KeyDestructionReceipt,
    StorageDeletionGuarantee,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityKind {
    EmergencyRead,
    LegacyCharterActivation,
    PosthumousRepresentation,
    GeneticUnveiling,
    SelfRetirement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityActivationState {
    Inactive,
    VerifiedActive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyClaim {
    InactiveRequirementStatement,
    PosthumousGuarantee,
    GeneticGuarantee,
    ErasureGuarantee,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyDependencyRequest {
    pub dependency_presence: DependencyPresence,
    pub adapter_verified: bool,
    pub charter_hash_state: ValidationState,
    pub invariant_validation_state: ValidationState,
    pub receipt_metadata_handling: ReceiptMetadataHandling,
    pub data_class: LegacyDataClass,
    pub interaction_text_in_receipt_metadata: bool,
    pub operation: LegacyOperation,
    pub payment_state_blocks_operation: bool,
    pub quorum_state_blocks_operation: bool,
    pub emergency_access_authorized: bool,
    pub erasure_evidence_kind: ErasureEvidenceKind,
    pub capability_kind: CapabilityKind,
    pub capability_activation_state: CapabilityActivationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyCopyReviewRequest {
    pub claims: Vec<LegacyClaim>,
    pub verified_code_and_policy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyDependencyDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_legacy_dependency(request: LegacyDependencyRequest) -> LegacyDependencyDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.operation == LegacyOperation::EmergencyTier0Read {
        if !request.emergency_access_authorized {
            reasons
                .insert("Emergency Tier-0 access requires its own authorization boundary.".into());
            required_evidence.insert(
                "Emergency Tier-0 authorization evidence independent from legacy quorum or payment state."
                    .into(),
            );
        }

        return legacy_dependency_decision(reasons, required_evidence);
    }

    if request.dependency_presence != DependencyPresence::Present {
        reasons.insert(
            "Missing exo-legacy dependency evidence keeps legacy capabilities inactive.".into(),
        );
        required_evidence.insert(
            "Verified exo-legacy crate presence, workspace registration, and adapter documentation."
                .into(),
        );
    }

    if !request.adapter_verified {
        reasons.insert(
            "Legacy capabilities require a verified exo-legacy adapter response before activation."
                .into(),
        );
        required_evidence.insert(
            "Adapter test evidence covering permit-only activation for legacy capabilities.".into(),
        );
    }

    if request.charter_hash_state != ValidationState::Present {
        reasons.insert("Legacy activation is denied when the charter hash is missing.".into());
        required_evidence.insert(
            "Canonical charter-hash evidence returned through the verified adapter.".into(),
        );
    }

    if request.invariant_validation_state != ValidationState::Passed {
        reasons.insert(
            "Legacy activation is denied when exo-legacy invariant validation fails.".into(),
        );
        required_evidence.insert(
            "Passing invariant-validation evidence for the requested legacy capability.".into(),
        );
    }

    if request.receipt_metadata_handling != ReceiptMetadataHandling::SafeOnly
        || matches!(
            request.data_class,
            LegacyDataClass::LegacyCharterContents | LegacyDataClass::GeneticPayload
        )
    {
        reasons.insert(
            "Legacy receipt metadata must never store charter contents or genetic payloads.".into(),
        );
        required_evidence.insert(
            "Receipt metadata showing commitments, hashes, and policy references only.".into(),
        );
    }

    if request.interaction_text_in_receipt_metadata {
        reasons.insert("Legacy receipt metadata must never store interaction-memory text.".into());
        required_evidence.insert(
            "Receipt metadata boundary proving interaction-memory text stays off-chain.".into(),
        );
    }

    if request.operation == LegacyOperation::Erasure
        && request.erasure_evidence_kind != ErasureEvidenceKind::KeyDestructionReceipt
    {
        reasons.insert(
            "Erasure status must be represented as key-destruction receipt evidence, not a storage deletion guarantee."
                .into(),
        );
        required_evidence
            .insert("Key-destruction receipt evidence for any erasure-state projection.".into());
    }

    if request.capability_activation_state != CapabilityActivationState::VerifiedActive {
        reasons.insert(
            "Legacy capability labels remain inactive until a verified adapter response marks them active."
                .into(),
        );
        required_evidence.insert(format!(
            "Verified adapter activation evidence for {}.",
            capability_label(request.capability_kind)
        ));
    }

    if request.payment_state_blocks_operation || request.quorum_state_blocks_operation {
        reasons.insert(
            "Legacy posthumous and charter capabilities remain blocked when payment or quorum policy denies the operation."
                .into(),
        );
        required_evidence.insert(
            "Policy evidence showing the requested non-emergency legacy operation is unblocked."
                .into(),
        );
    }

    legacy_dependency_decision(reasons, required_evidence)
}

pub fn review_legacy_copy(request: LegacyCopyReviewRequest) -> LegacyDependencyDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    let includes_unverified_guarantee = request.claims.iter().any(|claim| {
        matches!(
            claim,
            LegacyClaim::PosthumousGuarantee
                | LegacyClaim::GeneticGuarantee
                | LegacyClaim::ErasureGuarantee
        )
    });

    if includes_unverified_guarantee && !request.verified_code_and_policy {
        reasons.insert(
            "Product copy must not guarantee posthumous, genetic, or erasure outcomes without verified code and policy evidence."
                .into(),
        );
        required_evidence.insert(
            "Verified code paths plus approved policy evidence for any public guarantee.".into(),
        );
    }

    legacy_dependency_decision(reasons, required_evidence)
}

fn legacy_dependency_decision(
    reasons: BTreeSet<String>,
    required_evidence: BTreeSet<String>,
) -> LegacyDependencyDecision {
    LegacyDependencyDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}

fn capability_label(capability_kind: CapabilityKind) -> &'static str {
    match capability_kind {
        CapabilityKind::EmergencyRead => "emergency-read",
        CapabilityKind::LegacyCharterActivation => "legacy-charter-activation",
        CapabilityKind::PosthumousRepresentation => "posthumous-representation",
        CapabilityKind::GeneticUnveiling => "genetic-unveiling",
        CapabilityKind::SelfRetirement => "self-retirement",
    }
}
