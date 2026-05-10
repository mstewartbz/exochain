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

//! Canonical HonorGood fixtures and Apex Velocity Catalyst policy templates.

use exo_core::{Did, Hash256, Timestamp};

use crate::{
    error::EconomyError,
    legacy::{
        BeneficiaryRef, BeneficiaryType, LegacyReceipt, LegacyReceiptStatus, LegalEffect,
        MaterialityReview, MaterialityReviewStatus, MaterialityTier,
    },
    mission::{Mission, MissionPurpose, MissionStatus, MissionType},
    ruleset::{
        DurationPolicy, HonorGoodRuleset, ReviewFrequency, RulesetRecipientType, RulesetScope,
        RulesetShareLine, RulesetStatus, SettlementBasis,
    },
    types::{MicroExo, ZeroFeeReason},
    value_contribution::{ParticipantRef, require_nonzero_hash},
};

fn h(byte: u8) -> Hash256 {
    Hash256::from_bytes([byte; 32])
}

fn ts(ms: u64) -> Timestamp {
    Timestamp::new(ms, 0)
}

fn did(label: &str) -> Result<Did, EconomyError> {
    Did::new(&format!("did:exo:{label}")).map_err(EconomyError::from)
}

fn project_treasury(project: &str) -> ParticipantRef {
    ParticipantRef::ProjectTreasury {
        project: project.into(),
        treasury_ref: format!("public-project-treasury:{project}"),
    }
}

fn beneficiary(project: &str) -> BeneficiaryRef {
    BeneficiaryRef {
        beneficiary_type: BeneficiaryType::ProjectTreasury,
        reference: project_treasury(project),
    }
}

fn materiality_review(
    tier: MaterialityTier,
    evidence_hash: Hash256,
    rationale_hash: Hash256,
    rationale_ref: &str,
) -> Result<MaterialityReview, EconomyError> {
    Ok(MaterialityReview {
        tier,
        reviewer_did: did("honorgood-materiality-reviewer")?,
        evidence_hash,
        rationale_hash,
        rationale_ref: Some(rationale_ref.into()),
        reviewed_at: ts(10_000),
        status: MaterialityReviewStatus::EvidenceBacked,
    })
}

pub fn archon_exoforge_ruleset() -> Result<HonorGoodRuleset, EconomyError> {
    HonorGoodRuleset {
        ruleset_id: Hash256::ZERO,
        applies_to: vec![RulesetScope::ReceivingSystem("ExoForge".into())],
        share_lines: vec![
            RulesetShareLine {
                recipient: project_treasury("Archon"),
                recipient_type: RulesetRecipientType::ProjectTreasury,
                basis: SettlementBasis::NetRevenue,
                share_bp: 100,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: project_treasury("Archon"),
                recipient_type: RulesetRecipientType::ProjectTreasury,
                basis: SettlementBasis::ProtocolFee,
                share_bp: 500,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
        ],
        duration_policy: DurationPolicy::EvergreenWhileMateriallyUsed,
        review_frequency: ReviewFrequency::OnMaterialChange,
        requires_human_approval: true,
        allows_overlapping_bases: false,
        legal_effect_required: LegalEffect::RatifiedAgreement,
        status: RulesetStatus::Offered,
        created_at: ts(10_100),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn paperclip_commandbase_ruleset() -> Result<HonorGoodRuleset, EconomyError> {
    HonorGoodRuleset {
        ruleset_id: Hash256::ZERO,
        applies_to: vec![RulesetScope::ReceivingSystem("CommandBase".into())],
        share_lines: vec![
            RulesetShareLine {
                recipient: project_treasury("Paperclip"),
                recipient_type: RulesetRecipientType::ProjectTreasury,
                basis: SettlementBasis::SoftwareArr,
                share_bp: 150,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: project_treasury("Paperclip"),
                recipient_type: RulesetRecipientType::ProjectTreasury,
                basis: SettlementBasis::ImplementationFee,
                share_bp: 500,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
        ],
        duration_policy: DurationPolicy::EvergreenWhileMateriallyUsed,
        review_frequency: ReviewFrequency::OnMaterialChange,
        requires_human_approval: true,
        allows_overlapping_bases: false,
        legal_effect_required: LegalEffect::RatifiedAgreement,
        status: RulesetStatus::Offered,
        created_at: ts(10_200),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn archon_exoforge_legacy_receipt() -> Result<LegacyReceipt, EconomyError> {
    let ruleset = archon_exoforge_ruleset()?;
    require_nonzero_hash(ruleset.ruleset_id, "honorgood.archon.ruleset_id")?;
    LegacyReceipt {
        legacy_receipt_id: Hash256::ZERO,
        contributor: project_treasury("Archon"),
        contribution_name: "Archon".into(),
        contribution_type: "open-source implementation automation".into(),
        source_uri: "https://github.com/coleam00/Archon".into(),
        license: "MIT".into(),
        receiving_system: "ExoForge".into(),
        materiality_tier: MaterialityTier::Genesis,
        materiality_review: materiality_review(
            MaterialityTier::Genesis,
            h(0xA1),
            h(0xA2),
            "docs/economy/examples/archon_exoforge_legacy_receipt.yml",
        )?,
        attribution_required: true,
        settlement_eligible: false,
        economic_ruleset_id: Some(ruleset.ruleset_id),
        beneficiary: beneficiary("Archon"),
        active_while_materially_used: true,
        legal_effect: LegalEffect::VoluntaryRecognitionOnly,
        status: LegacyReceiptStatus::Proposed,
        signed_contributor_acceptance_hash: None,
        human_ratifier_did: None,
        created_at: ts(10_300),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn paperclip_commandbase_legacy_receipt() -> Result<LegacyReceipt, EconomyError> {
    let ruleset = paperclip_commandbase_ruleset()?;
    require_nonzero_hash(ruleset.ruleset_id, "honorgood.paperclip.ruleset_id")?;
    LegacyReceipt {
        legacy_receipt_id: Hash256::ZERO,
        contributor: project_treasury("Paperclip"),
        contribution_name: "Paperclip".into(),
        contribution_type: "open-source operational cockpit pattern".into(),
        source_uri: "https://github.com/paperclip-ui/paperclip".into(),
        license: "MIT".into(),
        receiving_system: "CommandBase".into(),
        materiality_tier: MaterialityTier::Foundational,
        materiality_review: materiality_review(
            MaterialityTier::Foundational,
            h(0xB1),
            h(0xB2),
            "docs/economy/examples/paperclip_commandbase_legacy_receipt.yml",
        )?,
        attribution_required: true,
        settlement_eligible: false,
        economic_ruleset_id: Some(ruleset.ruleset_id),
        beneficiary: beneficiary("Paperclip"),
        active_while_materially_used: true,
        legal_effect: LegalEffect::VoluntaryRecognitionOnly,
        status: LegacyReceiptStatus::Proposed,
        signed_contributor_acceptance_hash: None,
        human_ratifier_did: None,
        created_at: ts(10_400),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn apex_velocity_catalyst_client_services_ruleset() -> Result<HonorGoodRuleset, EconomyError> {
    HonorGoodRuleset {
        ruleset_id: Hash256::ZERO,
        applies_to: vec![
            RulesetScope::ApexVelocityCatalystCohort,
            RulesetScope::MissionType(MissionType::ApexVelocityCatalystClientServices),
        ],
        share_lines: vec![
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xC1)),
                recipient_type: RulesetRecipientType::ProtocolTreasury,
                basis: SettlementBasis::NetRevenue,
                share_bp: 1_500,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xC2)),
                recipient_type: RulesetRecipientType::Contributor,
                basis: SettlementBasis::NetRevenue,
                share_bp: 1_000,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xC3)),
                recipient_type: RulesetRecipientType::Contributor,
                basis: SettlementBasis::NetRevenue,
                share_bp: 500,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xC4)),
                recipient_type: RulesetRecipientType::Contributor,
                basis: SettlementBasis::NetRevenue,
                share_bp: 6_000,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xC5)),
                recipient_type: RulesetRecipientType::Contributor,
                basis: SettlementBasis::NetRevenue,
                share_bp: 1_000,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
        ],
        duration_policy: DurationPolicy::OneTime,
        review_frequency: ReviewFrequency::Quarterly,
        requires_human_approval: false,
        allows_overlapping_bases: false,
        legal_effect_required: LegalEffect::AcceptedTerms,
        status: RulesetStatus::Active,
        created_at: ts(10_500),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn apex_velocity_catalyst_software_channel_ruleset() -> Result<HonorGoodRuleset, EconomyError> {
    HonorGoodRuleset {
        ruleset_id: Hash256::ZERO,
        applies_to: vec![
            RulesetScope::ApexVelocityCatalystCohort,
            RulesetScope::MissionType(MissionType::ApexVelocityCatalystSoftwareChannel),
        ],
        share_lines: vec![
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xD1)),
                recipient_type: RulesetRecipientType::Contributor,
                basis: SettlementBasis::ChannelFee,
                share_bp: 4_000,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xD2)),
                recipient_type: RulesetRecipientType::Contributor,
                basis: SettlementBasis::ChannelFee,
                share_bp: 4_000,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
            RulesetShareLine {
                recipient: ParticipantRef::HashedReference(h(0xD3)),
                recipient_type: RulesetRecipientType::ProtocolTreasury,
                basis: SettlementBasis::ChannelFee,
                share_bp: 2_000,
                source_receipt_id: None,
                legacy_receipt_id: None,
            },
        ],
        duration_policy: DurationPolicy::EvergreenWhileMateriallyUsed,
        review_frequency: ReviewFrequency::Quarterly,
        requires_human_approval: false,
        allows_overlapping_bases: false,
        legal_effect_required: LegalEffect::AcceptedTerms,
        status: RulesetStatus::Active,
        created_at: ts(10_600),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn apex_velocity_catalyst_client_services_mission(
    expected_value_micro_exo: Option<MicroExo>,
) -> Result<Mission, EconomyError> {
    let ruleset = apex_velocity_catalyst_client_services_ruleset()?;
    Mission {
        mission_id: Hash256::ZERO,
        name: "Apex Velocity Catalyst client services mission".into(),
        mission_type: MissionType::ApexVelocityCatalystClientServices,
        owner_did: did("apex-velocity-catalyst-owner")?,
        principal_did: did("apex-velocity-catalyst-principal")?,
        purpose: MissionPurpose {
            problem: "Client requires governed implementation of useful EXOCHAIN-aligned systems"
                .into(),
            served_party: "client principal".into(),
            promised_outcome: "implemented outcome with contribution receipts and settlement lines"
                .into(),
            expected_value: "mission-specific value recorded in micro EXO units when known".into(),
            risk_surface: "delivery, governance, provenance, and adoption risk".into(),
            proof_required: "Mission, contribution receipts, ruleset, and settlement objects"
                .into(),
            success_condition: "accepted client outcome and auditable settlement record".into(),
        },
        related_platforms: vec!["EXOCHAIN".into(), "Apex Velocity Catalyst".into()],
        expected_value_micro_exo,
        ruleset_id: ruleset.ruleset_id,
        status: MissionStatus::Active,
        created_at: ts(10_700),
        content_hash: Hash256::ZERO,
    }
    .anchor()
}

pub fn zero_launch_mission_settlement_reason() -> ZeroFeeReason {
    ZeroFeeReason::PolicyConfiguredZero
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archon_fixture_remains_non_ratified() {
        let receipt = archon_exoforge_legacy_receipt().unwrap();
        assert_eq!(receipt.status, LegacyReceiptStatus::Proposed);
        assert_eq!(receipt.legal_effect, LegalEffect::VoluntaryRecognitionOnly);
        assert!(!receipt.settlement_eligible);
        assert!(receipt.signed_contributor_acceptance_hash.is_none());
        assert!(receipt.human_ratifier_did.is_none());
    }

    #[test]
    fn paperclip_fixture_remains_non_ratified() {
        let receipt = paperclip_commandbase_legacy_receipt().unwrap();
        assert_eq!(receipt.status, LegacyReceiptStatus::Proposed);
        assert_eq!(receipt.legal_effect, LegalEffect::VoluntaryRecognitionOnly);
        assert!(!receipt.settlement_eligible);
        assert!(receipt.signed_contributor_acceptance_hash.is_none());
        assert!(receipt.human_ratifier_did.is_none());
    }

    #[test]
    fn apex_velocity_catalyst_client_services_sums_to_ten_thousand_bp() {
        let ruleset = apex_velocity_catalyst_client_services_ruleset().unwrap();
        let total: u32 = ruleset.share_lines.iter().map(|line| line.share_bp).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn apex_velocity_catalyst_mission_hash_stable() {
        let mission = apex_velocity_catalyst_client_services_mission(Some(1_000_000)).unwrap();
        let again = apex_velocity_catalyst_client_services_mission(Some(1_000_000)).unwrap();
        assert_eq!(mission.mission_id, again.mission_id);
        assert_eq!(mission.ruleset_id, again.ruleset_id);
    }
}
