use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExochainConsentAdapterState {
    NotWired,
    Verified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentReceiptProvenance {
    VerifiedExochainAdapter,
    LiveSafeSynthetic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentOperation {
    Grant,
    Revoke,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentReceiptMetadataHandling {
    SafeReferencesOnly,
    IncludesRawSensitiveData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentCopyClaim {
    InactiveRequirementStatement,
    VerifiedConsentProof,
    VerifiedRevocationProof,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsentRevocationReceiptRequest {
    pub adapter_state: ExochainConsentAdapterState,
    pub receipt_provenance: ConsentReceiptProvenance,
    pub operation: ConsentOperation,
    pub metadata_handling: ConsentReceiptMetadataHandling,
    pub simulates_receipt_outside_exochain: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsentCopyReviewRequest {
    pub claims: Vec<ConsentCopyClaim>,
    pub verified_code_and_policy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsentReceiptDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
    pub required_evidence: Vec<String>,
}

pub fn evaluate_consent_revocation_receipt(
    request: ConsentRevocationReceiptRequest,
) -> ConsentReceiptDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    if request.adapter_state != ExochainConsentAdapterState::Verified {
        reasons.insert(
            "Consent and revocation proof remains inactive until a verified EXOCHAIN consent adapter is wired.".into(),
        );
        required_evidence.insert(
            "Verified LiveSafe consent-adapter path with fail-closed tests for consent and revocation proof."
                .into(),
        );
    }

    if request.receipt_provenance != ConsentReceiptProvenance::VerifiedExochainAdapter
        || request.simulates_receipt_outside_exochain
    {
        reasons.insert(
            "LiveSafe cannot mint, cache, or simulate consent or revocation receipt outcomes outside EXOCHAIN."
                .into(),
        );
        required_evidence.insert(
            "Verified EXOCHAIN-issued consent or revocation receipt reference for the requested operation."
                .into(),
        );
    }

    if request.metadata_handling != ConsentReceiptMetadataHandling::SafeReferencesOnly {
        reasons.insert(
            "Consent and revocation receipts may contain commitments, references, policy ids, and hashes only."
                .into(),
        );
        required_evidence.insert(
            "Receipt boundary proving consent and revocation metadata excludes raw sensitive payloads."
                .into(),
        );
    }

    if request.operation == ConsentOperation::Revoke
        && request.receipt_provenance != ConsentReceiptProvenance::VerifiedExochainAdapter
    {
        required_evidence.insert(
            "Verified EXOCHAIN revocation receipt reference before any revocation-proof projection."
                .into(),
        );
    }

    consent_receipt_decision(reasons, required_evidence)
}

pub fn review_consent_copy(request: ConsentCopyReviewRequest) -> ConsentReceiptDecision {
    let mut reasons = BTreeSet::new();
    let mut required_evidence = BTreeSet::new();

    let includes_verified_proof_claim = request.claims.iter().any(|claim| {
        matches!(
            claim,
            ConsentCopyClaim::VerifiedConsentProof | ConsentCopyClaim::VerifiedRevocationProof
        )
    });

    if includes_verified_proof_claim && !request.verified_code_and_policy {
        reasons.insert(
            "Product copy must not claim verified consent or revocation proof without verified code and policy evidence."
                .into(),
        );
        required_evidence.insert(
            "Verified adapter path plus approved policy evidence for any consent or revocation proof claim."
                .into(),
        );
    }

    consent_receipt_decision(reasons, required_evidence)
}

fn consent_receipt_decision(
    reasons: BTreeSet<String>,
    required_evidence: BTreeSet<String>,
) -> ConsentReceiptDecision {
    ConsentReceiptDecision {
        allowed: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
        required_evidence: required_evidence.into_iter().collect(),
    }
}
