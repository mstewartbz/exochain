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

//! HonorGood rulesets and deterministic share templates.

use std::collections::BTreeMap;

use exo_core::{Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    error::EconomyError,
    legacy::LegalEffect,
    mission::MissionType,
    types::{BasisPoints, MAX_BASIS_POINTS},
    value_contribution::{
        ParticipantRef, require_non_empty, require_nonzero_hash, require_nonzero_timestamp,
    },
};

pub const HONOR_GOOD_RULESET_HASH_DOMAIN: &str = "exo.economy.honorgood_ruleset.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SettlementBasis {
    NetRevenue,
    ProtocolFee,
    ChannelFee,
    MissionSurplus,
    SoftwareArr,
    ImplementationFee,
    RecognitionOnly,
    CostSavings,
    UsageMetric,
    Other(String),
}

impl SettlementBasis {
    pub fn label(&self) -> String {
        match self {
            Self::NetRevenue => "NetRevenue".into(),
            Self::ProtocolFee => "ProtocolFee".into(),
            Self::ChannelFee => "ChannelFee".into(),
            Self::MissionSurplus => "MissionSurplus".into(),
            Self::SoftwareArr => "SoftwareArr".into(),
            Self::ImplementationFee => "ImplementationFee".into(),
            Self::RecognitionOnly => "RecognitionOnly".into(),
            Self::CostSavings => "CostSavings".into(),
            Self::UsageMetric => "UsageMetric".into(),
            Self::Other(value) => format!("Other({value})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RulesetScope {
    MissionType(MissionType),
    ContributionNode(Hash256),
    LegacyReceipt(Hash256),
    ReceivingSystem(String),
    ApexVelocityCatalystCohort,
    Other(String),
}

impl RulesetScope {
    fn validate(&self) -> Result<(), EconomyError> {
        match self {
            Self::MissionType(_) | Self::ApexVelocityCatalystCohort => Ok(()),
            Self::ContributionNode(id) | Self::LegacyReceipt(id) => {
                require_nonzero_hash(*id, "ruleset.scope")
            }
            Self::ReceivingSystem(value) | Self::Other(value) => {
                require_non_empty(value, "ruleset.scope")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReviewFrequency {
    None,
    Monthly,
    Quarterly,
    Annual,
    OnMaterialChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationPolicy {
    EvergreenWhileMateriallyUsed,
    FixedTerm {
        starts_at: Timestamp,
        ends_at: Timestamp,
    },
    OneTime,
    RecognitionOnly,
}

impl DurationPolicy {
    fn validate(&self) -> Result<(), EconomyError> {
        if let Self::FixedTerm { starts_at, ends_at } = self {
            require_nonzero_timestamp(*starts_at, "ruleset.duration.starts_at")?;
            require_nonzero_timestamp(*ends_at, "ruleset.duration.ends_at")?;
            if ends_at <= starts_at {
                return Err(EconomyError::InvalidInput {
                    reason: "fixed term ruleset duration must end after it starts".into(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RulesetStatus {
    Draft,
    Offered,
    Active,
    Suspended,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RulesetRecipientType {
    Individual,
    Company,
    Trust,
    Foundation,
    Nonprofit,
    ProjectTreasury,
    MaintainerCollective,
    ProtocolTreasury,
    Contributor,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesetShareLine {
    pub recipient: ParticipantRef,
    pub recipient_type: RulesetRecipientType,
    pub basis: SettlementBasis,
    pub share_bp: BasisPoints,
    pub source_receipt_id: Option<Hash256>,
    pub legacy_receipt_id: Option<Hash256>,
}

impl RulesetShareLine {
    pub fn validate(&self) -> Result<(), EconomyError> {
        self.recipient.validate("ruleset.share_line.recipient")?;
        if self.share_bp > MAX_BASIS_POINTS {
            return Err(EconomyError::BasisPointOutOfRange {
                field: "ruleset.share_line.share_bp",
                value: self.share_bp,
                max: MAX_BASIS_POINTS,
            });
        }
        if matches!(self.basis, SettlementBasis::Other(_)) {
            return Err(EconomyError::UnsupportedSettlementBasis {
                basis: self.basis.label(),
            });
        }
        if let Some(id) = self.source_receipt_id {
            require_nonzero_hash(id, "ruleset.share_line.source_receipt_id")?;
        }
        if let Some(id) = self.legacy_receipt_id {
            require_nonzero_hash(id, "ruleset.share_line.legacy_receipt_id")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HonorGoodRuleset {
    pub ruleset_id: Hash256,
    pub applies_to: Vec<RulesetScope>,
    pub share_lines: Vec<RulesetShareLine>,
    pub duration_policy: DurationPolicy,
    pub review_frequency: ReviewFrequency,
    pub requires_human_approval: bool,
    pub allows_overlapping_bases: bool,
    pub legal_effect_required: LegalEffect,
    pub status: RulesetStatus,
    pub created_at: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct HonorGoodRulesetHashPayload<'a> {
    domain: &'static str,
    applies_to: &'a [RulesetScope],
    share_lines: &'a [RulesetShareLine],
    duration_policy: &'a DurationPolicy,
    review_frequency: ReviewFrequency,
    requires_human_approval: bool,
    allows_overlapping_bases: bool,
    legal_effect_required: LegalEffect,
    status: RulesetStatus,
    created_at: &'a Timestamp,
}

impl HonorGoodRuleset {
    pub fn validate(&self) -> Result<(), EconomyError> {
        if self.applies_to.is_empty() {
            return Err(EconomyError::InvalidInput {
                reason: "honorgood ruleset requires at least one scope".into(),
            });
        }
        for scope in &self.applies_to {
            scope.validate()?;
        }
        if self.share_lines.is_empty() {
            return Err(EconomyError::InvalidInput {
                reason: "honorgood ruleset requires at least one share line".into(),
            });
        }
        self.duration_policy.validate()?;
        require_nonzero_timestamp(self.created_at, "ruleset.created_at")?;
        validate_basis_allocations(&self.share_lines)
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&HonorGoodRulesetHashPayload {
            domain: HONOR_GOOD_RULESET_HASH_DOMAIN,
            applies_to: &self.applies_to,
            share_lines: &self.share_lines,
            duration_policy: &self.duration_policy,
            review_frequency: self.review_frequency,
            requires_human_approval: self.requires_human_approval,
            allows_overlapping_bases: self.allows_overlapping_bases,
            legal_effect_required: self.legal_effect_required,
            status: self.status,
            created_at: &self.created_at,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.applies_to.sort();
        self.share_lines.sort_by(|left, right| {
            (
                &left.basis,
                left.recipient_type,
                left.share_bp,
                &left.recipient,
                left.source_receipt_id,
                left.legacy_receipt_id,
            )
                .cmp(&(
                    &right.basis,
                    right.recipient_type,
                    right.share_bp,
                    &right.recipient,
                    right.source_receipt_id,
                    right.legacy_receipt_id,
                ))
        });
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.ruleset_id = hash;
        self.content_hash = hash;
        Ok(self)
    }

    pub fn verify_hashes(&self) -> Result<bool, EconomyError> {
        let hash = self.recompute_content_hash()?;
        Ok(self.ruleset_id == hash && self.content_hash == hash)
    }
}

pub fn validate_basis_allocations(lines: &[RulesetShareLine]) -> Result<(), EconomyError> {
    let mut totals: BTreeMap<SettlementBasis, BasisPoints> = BTreeMap::new();
    for line in lines {
        line.validate()?;
        let existing = totals.get(&line.basis).copied().unwrap_or(0);
        let next = existing
            .checked_add(line.share_bp)
            .ok_or(EconomyError::ArithmeticOverflow {
                operation: "ruleset.share_bp.sum",
            })?;
        if next > MAX_BASIS_POINTS {
            return Err(EconomyError::RevenueShareOverAllocated { sum: next });
        }
        totals.insert(line.basis.clone(), next);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::value_contribution::test_support::{participant, ts};

    pub fn sample_ruleset() -> HonorGoodRuleset {
        HonorGoodRuleset {
            ruleset_id: Hash256::ZERO,
            applies_to: vec![RulesetScope::ApexVelocityCatalystCohort],
            share_lines: vec![
                RulesetShareLine {
                    recipient: participant("originator"),
                    recipient_type: RulesetRecipientType::Contributor,
                    basis: SettlementBasis::NetRevenue,
                    share_bp: 1_000,
                    source_receipt_id: None,
                    legacy_receipt_id: None,
                },
                RulesetShareLine {
                    recipient: participant("protocol"),
                    recipient_type: RulesetRecipientType::ProtocolTreasury,
                    basis: SettlementBasis::ProtocolFee,
                    share_bp: 1_500,
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
            created_at: ts(1_300),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_support::*, *};
    use crate::value_contribution::test_support::{h, participant, ts};

    #[test]
    fn ruleset_hash_stable() {
        let ruleset = sample_ruleset().anchor().unwrap();
        let again = sample_ruleset().anchor().unwrap();
        assert_eq!(ruleset.ruleset_id, again.ruleset_id);
        assert!(ruleset.verify_hashes().unwrap());
    }

    #[test]
    fn per_basis_overallocation_rejected() {
        let mut ruleset = sample_ruleset();
        ruleset.share_lines.push(RulesetShareLine {
            recipient: crate::value_contribution::test_support::participant("too-much"),
            recipient_type: RulesetRecipientType::Contributor,
            basis: SettlementBasis::NetRevenue,
            share_bp: 9_001,
            source_receipt_id: None,
            legacy_receipt_id: None,
        });
        assert!(matches!(
            ruleset.validate(),
            Err(EconomyError::RevenueShareOverAllocated { .. })
        ));
    }

    #[test]
    fn unsupported_basis_rejected() {
        let mut ruleset = sample_ruleset();
        ruleset.share_lines[0].basis = SettlementBasis::Other("custom".into());
        assert!(matches!(
            ruleset.validate(),
            Err(EconomyError::UnsupportedSettlementBasis { .. })
        ));
    }

    #[test]
    fn fixed_term_duration_must_have_ordered_nonzero_timestamps() {
        let mut ruleset = sample_ruleset();
        ruleset.duration_policy = DurationPolicy::FixedTerm {
            starts_at: ts(10),
            ends_at: ts(20),
        };
        assert!(ruleset.validate().is_ok());

        ruleset.duration_policy = DurationPolicy::FixedTerm {
            starts_at: ts(20),
            ends_at: ts(10),
        };
        assert!(ruleset.validate().is_err());

        ruleset.duration_policy = DurationPolicy::FixedTerm {
            starts_at: Timestamp::new(0, 0),
            ends_at: ts(10),
        };
        assert!(ruleset.validate().is_err());
    }

    #[test]
    fn ruleset_scopes_fail_closed_for_empty_or_zero_identifiers() {
        let mut ruleset = sample_ruleset();
        ruleset.applies_to = vec![RulesetScope::ReceivingSystem(String::new())];
        assert!(ruleset.validate().is_err());

        ruleset.applies_to = vec![RulesetScope::ContributionNode(Hash256::ZERO)];
        assert!(ruleset.validate().is_err());

        ruleset.applies_to = vec![RulesetScope::Other("custom-scope".into())];
        assert!(ruleset.validate().is_ok());
    }

    #[test]
    fn share_line_rejects_invalid_basis_points_and_zero_link_ids() {
        let mut line = sample_ruleset().share_lines.remove(0);
        line.share_bp = MAX_BASIS_POINTS + 1;
        assert!(matches!(
            line.validate(),
            Err(EconomyError::BasisPointOutOfRange { .. })
        ));

        line.share_bp = 100;
        line.source_receipt_id = Some(Hash256::ZERO);
        assert!(line.validate().is_err());

        line.source_receipt_id = Some(h(0xA0));
        line.legacy_receipt_id = Some(Hash256::ZERO);
        assert!(line.validate().is_err());

        line.legacy_receipt_id = Some(h(0xA1));
        line.recipient = participant("linked-recipient");
        assert!(line.validate().is_ok());
    }

    #[test]
    fn ruleset_requires_scope_and_share_lines() {
        let mut ruleset = sample_ruleset();
        ruleset.applies_to.clear();
        assert!(ruleset.validate().is_err());

        let mut ruleset = sample_ruleset();
        ruleset.share_lines.clear();
        assert!(ruleset.validate().is_err());
    }
}
