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

//! Mission and contribution workflow receipts.

use exo_core::{Did, Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    error::EconomyError,
    types::MicroExo,
    value_contribution::{
        ParticipantRef, require_non_empty, require_nonzero_hash, require_nonzero_timestamp,
    },
};

pub const CONTRIBUTION_RECEIPT_HASH_DOMAIN: &str = "exo.economy.contribution_receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContributionContributorType {
    Human,
    Agent,
    Company,
    Platform,
    OpenSourceProject,
    Community,
    Foundation,
    Trust,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContributionCategory {
    Origination,
    DealArchitecture,
    Delivery,
    Governance,
    Platform,
    ReusableIp,
    Upstream,
    Documentation,
    Quality,
    AgentWorkflow,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Submitted,
    Accepted,
    Rejected,
    Disputed,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionReceipt {
    pub receipt_id: Hash256,
    pub mission_id: Option<Hash256>,
    pub contribution_node_id: Option<Hash256>,
    pub contributor: ParticipantRef,
    pub contributor_type: ContributionContributorType,
    pub action_type: String,
    pub contribution_category: ContributionCategory,
    pub evidence_hash: Hash256,
    pub evidence_uri: Option<String>,
    pub claimed_value_micro_exo: Option<MicroExo>,
    pub accepted_value_micro_exo: Option<MicroExo>,
    pub approval_status: ApprovalStatus,
    pub approver_did: Option<Did>,
    pub created_at: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct ContributionReceiptHashPayload<'a> {
    domain: &'static str,
    mission_id: Option<&'a Hash256>,
    contribution_node_id: Option<&'a Hash256>,
    contributor: &'a ParticipantRef,
    contributor_type: ContributionContributorType,
    action_type: &'a str,
    contribution_category: ContributionCategory,
    evidence_hash: &'a Hash256,
    evidence_uri: Option<&'a str>,
    claimed_value_micro_exo: Option<MicroExo>,
    accepted_value_micro_exo: Option<MicroExo>,
    approval_status: ApprovalStatus,
    approver_did: Option<&'a Did>,
    created_at: &'a Timestamp,
}

impl ContributionReceipt {
    pub fn validate(&self) -> Result<(), EconomyError> {
        if self.mission_id.is_none() && self.contribution_node_id.is_none() {
            return Err(EconomyError::InvalidInput {
                reason: "contribution receipt requires mission_id or contribution_node_id".into(),
            });
        }
        self.contributor
            .validate("contribution_receipt.contributor")?;
        require_non_empty(&self.action_type, "contribution_receipt.action_type")?;
        require_nonzero_hash(self.evidence_hash, "contribution_receipt.evidence_hash")?;
        if let Some(uri) = &self.evidence_uri {
            require_non_empty(uri, "contribution_receipt.evidence_uri")?;
        }
        if matches!(self.approval_status, ApprovalStatus::Accepted) && self.approver_did.is_none() {
            return Err(EconomyError::InvalidInput {
                reason: "accepted contribution receipt requires approver_did".into(),
            });
        }
        require_nonzero_timestamp(self.created_at, "contribution_receipt.created_at")
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&ContributionReceiptHashPayload {
            domain: CONTRIBUTION_RECEIPT_HASH_DOMAIN,
            mission_id: self.mission_id.as_ref(),
            contribution_node_id: self.contribution_node_id.as_ref(),
            contributor: &self.contributor,
            contributor_type: self.contributor_type,
            action_type: &self.action_type,
            contribution_category: self.contribution_category,
            evidence_hash: &self.evidence_hash,
            evidence_uri: self.evidence_uri.as_deref(),
            claimed_value_micro_exo: self.claimed_value_micro_exo,
            accepted_value_micro_exo: self.accepted_value_micro_exo,
            approval_status: self.approval_status,
            approver_did: self.approver_did.as_ref(),
            created_at: &self.created_at,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.receipt_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::value_contribution::test_support::{h, participant, ts};

    pub fn sample_contribution_receipt() -> ContributionReceipt {
        ContributionReceipt {
            receipt_id: Hash256::ZERO,
            mission_id: Some(h(0x30)),
            contribution_node_id: Some(h(0x31)),
            contributor: participant("contributor"),
            contributor_type: ContributionContributorType::Human,
            action_type: "prepared governance review".into(),
            contribution_category: ContributionCategory::Governance,
            evidence_hash: h(0x32),
            evidence_uri: Some("ipfs://evidence".into()),
            claimed_value_micro_exo: Some(10_000),
            accepted_value_micro_exo: None,
            approval_status: ApprovalStatus::Submitted,
            approver_did: None,
            created_at: ts(1_200),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_support::*, *};
    use crate::value_contribution::test_support::did;

    #[test]
    fn contribution_receipt_hash_stable() {
        let receipt = sample_contribution_receipt().anchor().unwrap();
        let again = sample_contribution_receipt().anchor().unwrap();
        assert_eq!(receipt.receipt_id, again.receipt_id);
    }

    #[test]
    fn accepted_receipt_requires_approver() {
        let mut receipt = sample_contribution_receipt();
        receipt.approval_status = ApprovalStatus::Accepted;
        assert!(receipt.validate().is_err());
        receipt.approver_did = Some(did("approver"));
        assert!(receipt.validate().is_ok());
    }
}
