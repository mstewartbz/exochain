// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Legacy receipts for evergreen provenance and conditional participation.

use exo_core::{Did, Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    error::EconomyError,
    value_contribution::{
        ParticipantRef, require_non_empty, require_nonzero_hash, require_nonzero_timestamp,
    },
};

pub const LEGACY_RECEIPT_HASH_DOMAIN: &str = "exo.economy.legacy_receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MaterialityTier {
    Genesis,
    Foundational,
    Material,
    Supportive,
    Incidental,
}

impl MaterialityTier {
    pub const ALL: [MaterialityTier; 5] = [
        MaterialityTier::Genesis,
        MaterialityTier::Foundational,
        MaterialityTier::Material,
        MaterialityTier::Supportive,
        MaterialityTier::Incidental,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MaterialityReviewStatus {
    Draft,
    EvidenceBacked,
    Disputed,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialityReview {
    pub tier: MaterialityTier,
    pub reviewer_did: Did,
    pub evidence_hash: Hash256,
    pub rationale_hash: Hash256,
    pub rationale_ref: Option<String>,
    pub reviewed_at: Timestamp,
    pub status: MaterialityReviewStatus,
}

impl MaterialityReview {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(self.evidence_hash, "legacy.materiality.evidence_hash")?;
        require_nonzero_hash(self.rationale_hash, "legacy.materiality.rationale_hash")?;
        if let Some(value) = &self.rationale_ref {
            require_non_empty(value, "legacy.materiality.rationale_ref")?;
        }
        require_nonzero_timestamp(self.reviewed_at, "legacy.materiality.reviewed_at")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BeneficiaryType {
    Individual,
    Company,
    Trust,
    Foundation,
    Nonprofit,
    ProjectTreasury,
    MaintainerCollective,
    ChurchOrSpiritualCommunity,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeneficiaryRef {
    pub beneficiary_type: BeneficiaryType,
    pub reference: ParticipantRef,
}

impl BeneficiaryRef {
    pub fn validate(&self) -> Result<(), EconomyError> {
        self.reference.validate("legacy.beneficiary.reference")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LegacyReceiptStatus {
    Proposed,
    Recognized,
    Offered,
    ContributorAccepted,
    Ratified,
    Rejected,
    Deprecated,
    Superseded,
}

impl LegacyReceiptStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Recognized => "Recognized",
            Self::Offered => "Offered",
            Self::ContributorAccepted => "ContributorAccepted",
            Self::Ratified => "Ratified",
            Self::Rejected => "Rejected",
            Self::Deprecated => "Deprecated",
            Self::Superseded => "Superseded",
        }
    }

    fn terminal(self) -> bool {
        matches!(self, Self::Rejected | Self::Deprecated | Self::Superseded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LegalEffect {
    VoluntaryRecognitionOnly,
    OfferedTerms,
    AcceptedTerms,
    ContributorAccepted,
    RatifiedAgreement,
    Revoked,
    Superseded,
}

impl LegalEffect {
    pub const ALL: [LegalEffect; 7] = [
        LegalEffect::VoluntaryRecognitionOnly,
        LegalEffect::OfferedTerms,
        LegalEffect::AcceptedTerms,
        LegalEffect::ContributorAccepted,
        LegalEffect::RatifiedAgreement,
        LegalEffect::Revoked,
        LegalEffect::Superseded,
    ];

    pub fn permits_settlement(self) -> bool {
        matches!(
            self,
            Self::AcceptedTerms | Self::ContributorAccepted | Self::RatifiedAgreement
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyReceipt {
    pub legacy_receipt_id: Hash256,
    pub contributor: ParticipantRef,
    pub contribution_name: String,
    pub contribution_type: String,
    pub source_uri: String,
    pub license: String,
    pub receiving_system: String,
    pub materiality_tier: MaterialityTier,
    pub materiality_review: MaterialityReview,
    pub attribution_required: bool,
    pub settlement_eligible: bool,
    pub economic_ruleset_id: Option<Hash256>,
    pub beneficiary: BeneficiaryRef,
    pub active_while_materially_used: bool,
    pub legal_effect: LegalEffect,
    pub status: LegacyReceiptStatus,
    pub signed_contributor_acceptance_hash: Option<Hash256>,
    pub human_ratifier_did: Option<Did>,
    pub created_at: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct LegacyReceiptHashPayload<'a> {
    domain: &'static str,
    contributor: &'a ParticipantRef,
    contribution_name: &'a str,
    contribution_type: &'a str,
    source_uri: &'a str,
    license: &'a str,
    receiving_system: &'a str,
    materiality_tier: MaterialityTier,
    materiality_review: &'a MaterialityReview,
    attribution_required: bool,
    settlement_eligible: bool,
    economic_ruleset_id: Option<&'a Hash256>,
    beneficiary: &'a BeneficiaryRef,
    active_while_materially_used: bool,
    legal_effect: LegalEffect,
    status: LegacyReceiptStatus,
    signed_contributor_acceptance_hash: Option<&'a Hash256>,
    human_ratifier_did: Option<&'a Did>,
    created_at: &'a Timestamp,
}

impl LegacyReceipt {
    pub fn validate(&self) -> Result<(), EconomyError> {
        self.contributor.validate("legacy.contributor")?;
        require_non_empty(&self.contribution_name, "legacy.contribution_name")?;
        require_non_empty(&self.contribution_type, "legacy.contribution_type")?;
        require_non_empty(&self.source_uri, "legacy.source_uri")?;
        require_non_empty(&self.license, "legacy.license")?;
        require_non_empty(&self.receiving_system, "legacy.receiving_system")?;
        self.materiality_review.validate()?;
        if self.materiality_review.tier != self.materiality_tier {
            return Err(EconomyError::InvalidInput {
                reason: "legacy materiality tier must match review tier".into(),
            });
        }
        if self.settlement_eligible && self.economic_ruleset_id.is_none() {
            return Err(EconomyError::InvalidInput {
                reason: "settlement-eligible legacy receipt requires economic_ruleset_id".into(),
            });
        }
        if let Some(id) = self.economic_ruleset_id {
            require_nonzero_hash(id, "legacy.economic_ruleset_id")?;
        }
        self.beneficiary.validate()?;
        if let Some(hash) = self.signed_contributor_acceptance_hash {
            require_nonzero_hash(hash, "legacy.signed_contributor_acceptance_hash")?;
        }
        self.validate_legal_effect()?;
        require_nonzero_timestamp(self.created_at, "legacy.created_at")
    }

    fn validate_legal_effect(&self) -> Result<(), EconomyError> {
        match self.status {
            LegacyReceiptStatus::Ratified => {
                if self.legal_effect != LegalEffect::RatifiedAgreement
                    || self.signed_contributor_acceptance_hash.is_none()
                    || self.human_ratifier_did.is_none()
                {
                    return Err(EconomyError::InvalidInput {
                        reason: "ratified legacy receipt requires signed contributor acceptance, human ratifier, and ratified legal effect".into(),
                    });
                }
            }
            LegacyReceiptStatus::ContributorAccepted => {
                if self.legal_effect != LegalEffect::ContributorAccepted
                    || self.signed_contributor_acceptance_hash.is_none()
                {
                    return Err(EconomyError::InvalidInput {
                        reason: "contributor-accepted legacy receipt requires signed contributor acceptance and matching legal effect".into(),
                    });
                }
            }
            LegacyReceiptStatus::Offered => {
                if !matches!(
                    self.legal_effect,
                    LegalEffect::OfferedTerms | LegalEffect::VoluntaryRecognitionOnly
                ) {
                    return Err(EconomyError::InvalidInput {
                        reason: "offered legacy receipt cannot claim accepted or ratified effect"
                            .into(),
                    });
                }
            }
            LegacyReceiptStatus::Proposed | LegacyReceiptStatus::Recognized => {
                if matches!(
                    self.legal_effect,
                    LegalEffect::AcceptedTerms
                        | LegalEffect::ContributorAccepted
                        | LegalEffect::RatifiedAgreement
                ) {
                    return Err(EconomyError::InvalidInput {
                        reason:
                            "unaccepted legacy receipt cannot claim accepted or ratified effect"
                                .into(),
                    });
                }
            }
            LegacyReceiptStatus::Rejected => {
                if self.legal_effect.permits_settlement() {
                    return Err(EconomyError::InvalidInput {
                        reason: "rejected legacy receipt cannot permit settlement".into(),
                    });
                }
            }
            LegacyReceiptStatus::Deprecated | LegacyReceiptStatus::Superseded => {}
        }
        Ok(())
    }

    pub fn can_transition_to(
        &self,
        to: LegacyReceiptStatus,
        next_effect: LegalEffect,
        signed_contributor_acceptance_hash: Option<Hash256>,
        human_ratifier_did: Option<&Did>,
    ) -> Result<(), EconomyError> {
        if self.status.terminal() {
            return Err(EconomyError::UnsupportedStatusTransition {
                from: self.status.label(),
                to: to.label(),
                reason: "terminal legacy receipt status cannot transition".into(),
            });
        }
        let allowed = matches!(
            (self.status, to),
            (
                LegacyReceiptStatus::Proposed,
                LegacyReceiptStatus::Recognized
            ) | (LegacyReceiptStatus::Proposed, LegacyReceiptStatus::Offered)
                | (LegacyReceiptStatus::Proposed, LegacyReceiptStatus::Rejected)
                | (
                    LegacyReceiptStatus::Proposed,
                    LegacyReceiptStatus::Superseded
                )
                | (
                    LegacyReceiptStatus::Recognized,
                    LegacyReceiptStatus::Offered
                )
                | (
                    LegacyReceiptStatus::Recognized,
                    LegacyReceiptStatus::Rejected
                )
                | (
                    LegacyReceiptStatus::Recognized,
                    LegacyReceiptStatus::Deprecated
                )
                | (
                    LegacyReceiptStatus::Recognized,
                    LegacyReceiptStatus::Superseded
                )
                | (
                    LegacyReceiptStatus::Offered,
                    LegacyReceiptStatus::ContributorAccepted
                )
                | (LegacyReceiptStatus::Offered, LegacyReceiptStatus::Rejected)
                | (
                    LegacyReceiptStatus::Offered,
                    LegacyReceiptStatus::Deprecated
                )
                | (
                    LegacyReceiptStatus::Offered,
                    LegacyReceiptStatus::Superseded
                )
                | (
                    LegacyReceiptStatus::ContributorAccepted,
                    LegacyReceiptStatus::Ratified
                )
                | (
                    LegacyReceiptStatus::ContributorAccepted,
                    LegacyReceiptStatus::Rejected
                )
                | (
                    LegacyReceiptStatus::ContributorAccepted,
                    LegacyReceiptStatus::Superseded
                )
                | (
                    LegacyReceiptStatus::Ratified,
                    LegacyReceiptStatus::Deprecated
                )
                | (
                    LegacyReceiptStatus::Ratified,
                    LegacyReceiptStatus::Superseded
                )
        );
        if !allowed {
            return Err(EconomyError::UnsupportedStatusTransition {
                from: self.status.label(),
                to: to.label(),
                reason: "transition is not in the legacy receipt state machine".into(),
            });
        }
        if to == LegacyReceiptStatus::ContributorAccepted {
            let Some(hash) = signed_contributor_acceptance_hash else {
                return Err(EconomyError::UnsupportedStatusTransition {
                    from: self.status.label(),
                    to: to.label(),
                    reason: "contributor acceptance requires signed acceptance hash".into(),
                });
            };
            require_nonzero_hash(hash, "legacy.transition.signed_contributor_acceptance_hash")?;
            if next_effect != LegalEffect::ContributorAccepted {
                return Err(EconomyError::UnsupportedStatusTransition {
                    from: self.status.label(),
                    to: to.label(),
                    reason: "contributor acceptance requires ContributorAccepted legal effect"
                        .into(),
                });
            }
        }
        if to == LegacyReceiptStatus::Ratified {
            let Some(hash) = signed_contributor_acceptance_hash else {
                return Err(EconomyError::UnsupportedStatusTransition {
                    from: self.status.label(),
                    to: to.label(),
                    reason: "ratification requires signed contributor acceptance hash".into(),
                });
            };
            require_nonzero_hash(hash, "legacy.transition.signed_contributor_acceptance_hash")?;
            if human_ratifier_did.is_none() || next_effect != LegalEffect::RatifiedAgreement {
                return Err(EconomyError::UnsupportedStatusTransition {
                    from: self.status.label(),
                    to: to.label(),
                    reason:
                        "ratification requires human ratifier and RatifiedAgreement legal effect"
                            .into(),
                });
            }
        }
        Ok(())
    }

    pub fn transition_to(
        mut self,
        to: LegacyReceiptStatus,
        next_effect: LegalEffect,
        signed_contributor_acceptance_hash: Option<Hash256>,
        human_ratifier_did: Option<Did>,
    ) -> Result<Self, EconomyError> {
        self.can_transition_to(
            to,
            next_effect,
            signed_contributor_acceptance_hash,
            human_ratifier_did.as_ref(),
        )?;
        self.status = to;
        self.legal_effect = next_effect;
        if signed_contributor_acceptance_hash.is_some() {
            self.signed_contributor_acceptance_hash = signed_contributor_acceptance_hash;
        }
        if human_ratifier_did.is_some() {
            self.human_ratifier_did = human_ratifier_did;
        }
        self.anchor()
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&LegacyReceiptHashPayload {
            domain: LEGACY_RECEIPT_HASH_DOMAIN,
            contributor: &self.contributor,
            contribution_name: &self.contribution_name,
            contribution_type: &self.contribution_type,
            source_uri: &self.source_uri,
            license: &self.license,
            receiving_system: &self.receiving_system,
            materiality_tier: self.materiality_tier,
            materiality_review: &self.materiality_review,
            attribution_required: self.attribution_required,
            settlement_eligible: self.settlement_eligible,
            economic_ruleset_id: self.economic_ruleset_id.as_ref(),
            beneficiary: &self.beneficiary,
            active_while_materially_used: self.active_while_materially_used,
            legal_effect: self.legal_effect,
            status: self.status,
            signed_contributor_acceptance_hash: self.signed_contributor_acceptance_hash.as_ref(),
            human_ratifier_did: self.human_ratifier_did.as_ref(),
            created_at: &self.created_at,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.legacy_receipt_id = hash;
        self.content_hash = hash;
        Ok(self)
    }

    pub fn verify_hashes(&self) -> Result<bool, EconomyError> {
        let hash = self.recompute_content_hash()?;
        Ok(self.legacy_receipt_id == hash && self.content_hash == hash)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::value_contribution::test_support::{did, h, participant, ts};

    pub fn beneficiary(label: &str) -> BeneficiaryRef {
        BeneficiaryRef {
            beneficiary_type: BeneficiaryType::ProjectTreasury,
            reference: ParticipantRef::ProjectTreasury {
                project: label.into(),
                treasury_ref: format!("treasury:{label}"),
            },
        }
    }

    pub fn materiality(tier: MaterialityTier) -> MaterialityReview {
        MaterialityReview {
            tier,
            reviewer_did: did("materiality-reviewer"),
            evidence_hash: h(0x40),
            rationale_hash: h(0x41),
            rationale_ref: Some("docs/economy/examples/materiality-review.yml".into()),
            reviewed_at: ts(1_400),
            status: MaterialityReviewStatus::EvidenceBacked,
        }
    }

    pub fn sample_legacy_receipt() -> LegacyReceipt {
        LegacyReceipt {
            legacy_receipt_id: Hash256::ZERO,
            contributor: participant("upstream-project"),
            contribution_name: "Upstream project".into(),
            contribution_type: "open-source software".into(),
            source_uri: "https://example.test/upstream".into(),
            license: "MIT".into(),
            receiving_system: "EXOCHAIN".into(),
            materiality_tier: MaterialityTier::Foundational,
            materiality_review: materiality(MaterialityTier::Foundational),
            attribution_required: true,
            settlement_eligible: true,
            economic_ruleset_id: Some(h(0x42)),
            beneficiary: beneficiary("upstream-project"),
            active_while_materially_used: true,
            legal_effect: LegalEffect::OfferedTerms,
            status: LegacyReceiptStatus::Offered,
            signed_contributor_acceptance_hash: None,
            human_ratifier_did: None,
            created_at: ts(1_500),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_support::*, *};
    use crate::value_contribution::test_support::{did, h};

    #[test]
    fn legacy_receipt_hash_stable() {
        let receipt = sample_legacy_receipt().anchor().unwrap();
        let again = sample_legacy_receipt().anchor().unwrap();
        assert_eq!(receipt.legacy_receipt_id, again.legacy_receipt_id);
        assert!(receipt.verify_hashes().unwrap());
    }

    #[test]
    fn materiality_tier_serialization_order_stable() {
        assert_eq!(MaterialityTier::ALL[0], MaterialityTier::Genesis);
        assert_eq!(MaterialityTier::ALL[1], MaterialityTier::Foundational);
        assert_eq!(MaterialityTier::ALL[2], MaterialityTier::Material);
        assert_eq!(MaterialityTier::ALL[3], MaterialityTier::Supportive);
        assert_eq!(MaterialityTier::ALL[4], MaterialityTier::Incidental);
    }

    #[test]
    fn legal_effect_serialization_order_stable() {
        assert_eq!(LegalEffect::ALL[0], LegalEffect::VoluntaryRecognitionOnly);
        assert_eq!(LegalEffect::ALL[3], LegalEffect::ContributorAccepted);
        assert_eq!(LegalEffect::ALL[4], LegalEffect::RatifiedAgreement);
    }

    #[test]
    fn direct_proposed_to_ratified_fails_closed() {
        let mut receipt = sample_legacy_receipt();
        receipt.status = LegacyReceiptStatus::Proposed;
        receipt.legal_effect = LegalEffect::VoluntaryRecognitionOnly;
        let receipt = receipt.anchor().unwrap();
        assert!(matches!(
            receipt.can_transition_to(
                LegacyReceiptStatus::Ratified,
                LegalEffect::RatifiedAgreement,
                Some(h(0xAA)),
                Some(&did("ratifier")),
            ),
            Err(EconomyError::UnsupportedStatusTransition { .. })
        ));
    }

    #[test]
    fn ratification_requires_acceptance_and_human() {
        let receipt = sample_legacy_receipt().anchor().unwrap();
        let accepted = receipt
            .transition_to(
                LegacyReceiptStatus::ContributorAccepted,
                LegalEffect::ContributorAccepted,
                Some(h(0xAA)),
                None,
            )
            .unwrap();
        let ratified = accepted
            .clone()
            .transition_to(
                LegacyReceiptStatus::Ratified,
                LegalEffect::RatifiedAgreement,
                Some(h(0xAA)),
                Some(did("ratifier")),
            )
            .unwrap();
        assert_eq!(ratified.status, LegacyReceiptStatus::Ratified);
        assert!(
            accepted
                .transition_to(
                    LegacyReceiptStatus::Ratified,
                    LegalEffect::RatifiedAgreement,
                    Some(h(0xAA)),
                    None,
                )
                .is_err()
        );
    }

    #[test]
    fn opaque_beneficiary_rejects_empty_project_reference() {
        let mut receipt = sample_legacy_receipt();
        receipt.beneficiary.reference = ParticipantRef::ProjectTreasury {
            project: String::new(),
            treasury_ref: "treasury".into(),
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn unaccepted_legacy_receipts_cannot_claim_accepted_effects() {
        let mut proposed = sample_legacy_receipt();
        proposed.status = LegacyReceiptStatus::Recognized;
        proposed.legal_effect = LegalEffect::AcceptedTerms;
        assert!(proposed.validate().is_err());

        proposed.legal_effect = LegalEffect::RatifiedAgreement;
        assert!(proposed.validate().is_err());
    }

    #[test]
    fn rejected_legacy_receipts_cannot_permit_settlement() {
        let mut receipt = sample_legacy_receipt();
        receipt.status = LegacyReceiptStatus::Rejected;
        receipt.legal_effect = LegalEffect::ContributorAccepted;
        receipt.signed_contributor_acceptance_hash = Some(h(0xAB));
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn legacy_materiality_tier_must_match_review() {
        let mut receipt = sample_legacy_receipt();
        receipt.materiality_tier = MaterialityTier::Genesis;
        receipt.materiality_review = materiality(MaterialityTier::Foundational);
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn settlement_eligible_legacy_receipt_requires_ruleset_id() {
        let mut receipt = sample_legacy_receipt();
        receipt.economic_ruleset_id = None;
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn terminal_legacy_status_cannot_transition() {
        let mut receipt = sample_legacy_receipt();
        receipt.status = LegacyReceiptStatus::Rejected;
        receipt.legal_effect = LegalEffect::VoluntaryRecognitionOnly;
        let receipt = receipt.anchor().unwrap();
        assert!(matches!(
            receipt.can_transition_to(
                LegacyReceiptStatus::Recognized,
                LegalEffect::VoluntaryRecognitionOnly,
                None,
                None,
            ),
            Err(EconomyError::UnsupportedStatusTransition { .. })
        ));
    }

    #[test]
    fn contributor_acceptance_transition_requires_matching_legal_effect() {
        let receipt = sample_legacy_receipt().anchor().unwrap();
        assert!(matches!(
            receipt.can_transition_to(
                LegacyReceiptStatus::ContributorAccepted,
                LegalEffect::AcceptedTerms,
                Some(h(0xAC)),
                None,
            ),
            Err(EconomyError::UnsupportedStatusTransition { .. })
        ));
    }

    #[test]
    fn ratified_legacy_receipt_validation_requires_acceptance_and_ratifier() {
        let mut receipt = sample_legacy_receipt();
        receipt.status = LegacyReceiptStatus::Ratified;
        receipt.legal_effect = LegalEffect::RatifiedAgreement;
        assert!(receipt.validate().is_err());
    }
}
