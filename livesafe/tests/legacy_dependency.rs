use livesafe::legacy_dependency::{
    CapabilityActivationState, CapabilityKind, DependencyPresence, ErasureEvidenceKind,
    LegacyClaim, LegacyCopyReviewRequest, LegacyDataClass, LegacyDependencyDecision,
    LegacyDependencyRequest, LegacyOperation, ReceiptMetadataHandling, ValidationState,
    evaluate_legacy_dependency, review_legacy_copy,
};

fn assert_denied(decision: &LegacyDependencyDecision, reason: &str) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn legacy_capabilities_stay_inactive_without_a_verified_adapter() {
    let decision = evaluate_legacy_dependency(LegacyDependencyRequest {
        dependency_presence: DependencyPresence::Missing,
        adapter_verified: false,
        charter_hash_state: ValidationState::Present,
        invariant_validation_state: ValidationState::Passed,
        receipt_metadata_handling: ReceiptMetadataHandling::SafeOnly,
        data_class: LegacyDataClass::LegacyCharterMetadata,
        interaction_text_in_receipt_metadata: false,
        operation: LegacyOperation::PosthumousRepresentation,
        payment_state_blocks_operation: true,
        quorum_state_blocks_operation: true,
        emergency_access_authorized: false,
        erasure_evidence_kind: ErasureEvidenceKind::StorageDeletionGuarantee,
        capability_kind: CapabilityKind::PosthumousRepresentation,
        capability_activation_state: CapabilityActivationState::Inactive,
    });

    assert_denied(
        &decision,
        "Missing exo-legacy dependency evidence keeps legacy capabilities inactive.",
    );
    assert_denied(
        &decision,
        "Legacy capabilities require a verified exo-legacy adapter response before activation.",
    );
}

#[test]
fn legacy_contract_rejects_missing_charter_hash_and_failed_invariants() {
    let decision = evaluate_legacy_dependency(LegacyDependencyRequest {
        dependency_presence: DependencyPresence::Present,
        adapter_verified: true,
        charter_hash_state: ValidationState::Missing,
        invariant_validation_state: ValidationState::Failed,
        receipt_metadata_handling: ReceiptMetadataHandling::SafeOnly,
        data_class: LegacyDataClass::LegacyCharterMetadata,
        interaction_text_in_receipt_metadata: false,
        operation: LegacyOperation::LegacyCharterReview,
        payment_state_blocks_operation: false,
        quorum_state_blocks_operation: false,
        emergency_access_authorized: false,
        erasure_evidence_kind: ErasureEvidenceKind::KeyDestructionReceipt,
        capability_kind: CapabilityKind::LegacyCharterActivation,
        capability_activation_state: CapabilityActivationState::VerifiedActive,
    });

    assert_denied(
        &decision,
        "Legacy activation is denied when the charter hash is missing.",
    );
    assert_denied(
        &decision,
        "Legacy activation is denied when exo-legacy invariant validation fails.",
    );
}

#[test]
fn legacy_receipt_metadata_never_carries_charter_genetic_or_interaction_text() {
    for data_class in [
        LegacyDataClass::LegacyCharterContents,
        LegacyDataClass::GeneticPayload,
    ] {
        let decision = evaluate_legacy_dependency(LegacyDependencyRequest {
            dependency_presence: DependencyPresence::Present,
            adapter_verified: true,
            charter_hash_state: ValidationState::Present,
            invariant_validation_state: ValidationState::Passed,
            receipt_metadata_handling: ReceiptMetadataHandling::IncludesRawSensitiveData,
            data_class,
            interaction_text_in_receipt_metadata: true,
            operation: LegacyOperation::GeneticUnveiling,
            payment_state_blocks_operation: false,
            quorum_state_blocks_operation: false,
            emergency_access_authorized: false,
            erasure_evidence_kind: ErasureEvidenceKind::KeyDestructionReceipt,
            capability_kind: CapabilityKind::GeneticUnveiling,
            capability_activation_state: CapabilityActivationState::VerifiedActive,
        });

        assert_denied(
            &decision,
            "Legacy receipt metadata must never store charter contents or genetic payloads.",
        );
        assert_denied(
            &decision,
            "Legacy receipt metadata must never store interaction-memory text.",
        );
    }
}

#[test]
fn emergency_tier0_access_is_separate_from_posthumous_quorum_and_payment_state() {
    let allowed = evaluate_legacy_dependency(LegacyDependencyRequest {
        dependency_presence: DependencyPresence::Missing,
        adapter_verified: false,
        charter_hash_state: ValidationState::Missing,
        invariant_validation_state: ValidationState::Missing,
        receipt_metadata_handling: ReceiptMetadataHandling::SafeOnly,
        data_class: LegacyDataClass::PhenotypicalMedicalSummary,
        interaction_text_in_receipt_metadata: false,
        operation: LegacyOperation::EmergencyTier0Read,
        payment_state_blocks_operation: true,
        quorum_state_blocks_operation: true,
        emergency_access_authorized: true,
        erasure_evidence_kind: ErasureEvidenceKind::NotRequested,
        capability_kind: CapabilityKind::EmergencyRead,
        capability_activation_state: CapabilityActivationState::Inactive,
    });

    assert!(allowed.allowed, "{allowed:?}");

    let denied = evaluate_legacy_dependency(LegacyDependencyRequest {
        emergency_access_authorized: false,
        ..LegacyDependencyRequest {
            dependency_presence: DependencyPresence::Missing,
            adapter_verified: false,
            charter_hash_state: ValidationState::Missing,
            invariant_validation_state: ValidationState::Missing,
            receipt_metadata_handling: ReceiptMetadataHandling::SafeOnly,
            data_class: LegacyDataClass::PhenotypicalMedicalSummary,
            interaction_text_in_receipt_metadata: false,
            operation: LegacyOperation::EmergencyTier0Read,
            payment_state_blocks_operation: true,
            quorum_state_blocks_operation: true,
            emergency_access_authorized: true,
            erasure_evidence_kind: ErasureEvidenceKind::NotRequested,
            capability_kind: CapabilityKind::EmergencyRead,
            capability_activation_state: CapabilityActivationState::Inactive,
        }
    });

    assert_denied(
        &denied,
        "Emergency Tier-0 access requires its own authorization boundary.",
    );
}

#[test]
fn erasure_requires_key_destruction_receipt_not_storage_deletion_claims() {
    let decision = evaluate_legacy_dependency(LegacyDependencyRequest {
        dependency_presence: DependencyPresence::Present,
        adapter_verified: true,
        charter_hash_state: ValidationState::Present,
        invariant_validation_state: ValidationState::Passed,
        receipt_metadata_handling: ReceiptMetadataHandling::SafeOnly,
        data_class: LegacyDataClass::LegacyCharterMetadata,
        interaction_text_in_receipt_metadata: false,
        operation: LegacyOperation::Erasure,
        payment_state_blocks_operation: false,
        quorum_state_blocks_operation: false,
        emergency_access_authorized: false,
        erasure_evidence_kind: ErasureEvidenceKind::StorageDeletionGuarantee,
        capability_kind: CapabilityKind::SelfRetirement,
        capability_activation_state: CapabilityActivationState::VerifiedActive,
    });

    assert_denied(
        &decision,
        "Erasure status must be represented as key-destruction receipt evidence, not a storage deletion guarantee.",
    );
}

#[test]
fn verified_legacy_capabilities_can_activate_with_safe_metadata() {
    let decision = evaluate_legacy_dependency(LegacyDependencyRequest {
        dependency_presence: DependencyPresence::Present,
        adapter_verified: true,
        charter_hash_state: ValidationState::Present,
        invariant_validation_state: ValidationState::Passed,
        receipt_metadata_handling: ReceiptMetadataHandling::SafeOnly,
        data_class: LegacyDataClass::LegacyCharterMetadata,
        interaction_text_in_receipt_metadata: false,
        operation: LegacyOperation::PosthumousRepresentation,
        payment_state_blocks_operation: false,
        quorum_state_blocks_operation: false,
        emergency_access_authorized: false,
        erasure_evidence_kind: ErasureEvidenceKind::KeyDestructionReceipt,
        capability_kind: CapabilityKind::PosthumousRepresentation,
        capability_activation_state: CapabilityActivationState::VerifiedActive,
    });

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
}

#[test]
fn legacy_copy_review_blocks_unverified_posthumous_genetic_and_erasure_claims() {
    let decision = review_legacy_copy(LegacyCopyReviewRequest {
        claims: vec![
            LegacyClaim::PosthumousGuarantee,
            LegacyClaim::GeneticGuarantee,
            LegacyClaim::ErasureGuarantee,
        ],
        verified_code_and_policy: false,
    });

    assert_denied(
        &decision,
        "Product copy must not guarantee posthumous, genetic, or erasure outcomes without verified code and policy evidence.",
    );
}

#[test]
fn legacy_copy_review_allows_inactive_requirement_language() {
    let decision = review_legacy_copy(LegacyCopyReviewRequest {
        claims: vec![LegacyClaim::InactiveRequirementStatement],
        verified_code_and_policy: false,
    });

    assert!(decision.allowed, "{decision:?}");
}
