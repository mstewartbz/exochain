use livesafe::consent_revocation_receipt::{
    ConsentCopyClaim, ConsentCopyReviewRequest, ConsentOperation, ConsentReceiptDecision,
    ConsentReceiptMetadataHandling, ConsentReceiptProvenance, ConsentRevocationReceiptRequest,
    ExochainConsentAdapterState, evaluate_consent_revocation_receipt, review_consent_copy,
};

fn assert_denied(decision: &ConsentReceiptDecision, reason: &str) {
    assert!(!decision.allowed, "{decision:?}");
    assert!(
        decision.reasons.contains(&reason.to_string()),
        "missing reason `{reason}` in {decision:?}"
    );
}

#[test]
fn consent_and_revocation_proof_stay_inactive_without_a_verified_adapter() {
    let decision = evaluate_consent_revocation_receipt(ConsentRevocationReceiptRequest {
        adapter_state: ExochainConsentAdapterState::NotWired,
        receipt_provenance: ConsentReceiptProvenance::VerifiedExochainAdapter,
        operation: ConsentOperation::Grant,
        metadata_handling: ConsentReceiptMetadataHandling::SafeReferencesOnly,
        simulates_receipt_outside_exochain: false,
    });

    assert_denied(
        &decision,
        "Consent and revocation proof remains inactive until a verified EXOCHAIN consent adapter is wired.",
    );
}

#[test]
fn livesafe_cannot_mint_or_simulate_consent_or_revocation_receipts_locally() {
    let decision = evaluate_consent_revocation_receipt(ConsentRevocationReceiptRequest {
        adapter_state: ExochainConsentAdapterState::Verified,
        receipt_provenance: ConsentReceiptProvenance::LiveSafeSynthetic,
        operation: ConsentOperation::Revoke,
        metadata_handling: ConsentReceiptMetadataHandling::SafeReferencesOnly,
        simulates_receipt_outside_exochain: true,
    });

    assert_denied(
        &decision,
        "LiveSafe cannot mint, cache, or simulate consent or revocation receipt outcomes outside EXOCHAIN.",
    );
}

#[test]
fn consent_and_revocation_receipts_allow_only_safe_metadata_shapes() {
    let decision = evaluate_consent_revocation_receipt(ConsentRevocationReceiptRequest {
        adapter_state: ExochainConsentAdapterState::Verified,
        receipt_provenance: ConsentReceiptProvenance::VerifiedExochainAdapter,
        operation: ConsentOperation::Grant,
        metadata_handling: ConsentReceiptMetadataHandling::IncludesRawSensitiveData,
        simulates_receipt_outside_exochain: false,
    });

    assert_denied(
        &decision,
        "Consent and revocation receipts may contain commitments, references, policy ids, and hashes only.",
    );
}

#[test]
fn verified_adapter_backed_receipts_with_safe_metadata_are_allowed() {
    let decision = evaluate_consent_revocation_receipt(ConsentRevocationReceiptRequest {
        adapter_state: ExochainConsentAdapterState::Verified,
        receipt_provenance: ConsentReceiptProvenance::VerifiedExochainAdapter,
        operation: ConsentOperation::Revoke,
        metadata_handling: ConsentReceiptMetadataHandling::SafeReferencesOnly,
        simulates_receipt_outside_exochain: false,
    });

    assert!(decision.allowed, "{decision:?}");
    assert_eq!(decision.reasons, Vec::<String>::new());
}

#[test]
fn consent_copy_review_blocks_unverified_proof_claims() {
    let decision = review_consent_copy(ConsentCopyReviewRequest {
        claims: vec![
            ConsentCopyClaim::VerifiedConsentProof,
            ConsentCopyClaim::VerifiedRevocationProof,
        ],
        verified_code_and_policy: false,
    });

    assert_denied(
        &decision,
        "Product copy must not claim verified consent or revocation proof without verified code and policy evidence.",
    );
}

#[test]
fn consent_copy_review_allows_inactive_requirement_language() {
    let decision = review_consent_copy(ConsentCopyReviewRequest {
        claims: vec![ConsentCopyClaim::InactiveRequirementStatement],
        verified_code_and_policy: false,
    });

    assert!(decision.allowed, "{decision:?}");
}
