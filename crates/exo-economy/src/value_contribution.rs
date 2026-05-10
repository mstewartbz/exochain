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

//! Generalized value contribution nodes.
//!
//! A `ValueContributionNode` is the offerable object in the HonorGood
//! economy. It does not create payment by itself; it records provenance,
//! terms, settlement eligibility, and policy references for later adoption.

use exo_core::{Did, Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::error::EconomyError;

/// Domain tag for value contribution node content hashes.
pub const VALUE_CONTRIBUTION_NODE_HASH_DOMAIN: &str = "exo.economy.value_contribution_node.v1";

/// Opaque participant reference. Sensitive identity, estate, tax,
/// banking, or family details must remain off-ledger.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ParticipantRef {
    Did(Did),
    PublicName(String),
    ProjectTreasury {
        project: String,
        treasury_ref: String,
    },
    VaultPointer {
        vault_id: String,
        record_hash: Hash256,
    },
    HashedReference(Hash256),
}

impl ParticipantRef {
    /// Validate that the reference is non-empty and opaque.
    pub fn validate(&self, field: &'static str) -> Result<(), EconomyError> {
        match self {
            Self::Did(_) => Ok(()),
            Self::HashedReference(value) => {
                if *value == Hash256::ZERO {
                    return Err(EconomyError::InvalidInput {
                        reason: format!("{field} hashed reference must not use zero hash"),
                    });
                }
                Ok(())
            }
            Self::PublicName(value) => require_non_empty(value, field),
            Self::ProjectTreasury {
                project,
                treasury_ref,
            } => {
                require_non_empty(project, field)?;
                require_non_empty(treasury_ref, field)
            }
            Self::VaultPointer {
                vault_id,
                record_hash,
            } => {
                require_non_empty(vault_id, field)?;
                if *record_hash == Hash256::ZERO {
                    return Err(EconomyError::InvalidInput {
                        reason: format!("{field} vault pointer must not use zero record hash"),
                    });
                }
                Ok(())
            }
        }
    }
}

/// Opaque authority reference proving that an agent or holon is acting
/// inside a delegated authority envelope.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthorityEnvelopeRef {
    pub envelope_id: Hash256,
    pub authority_proof_hash: Hash256,
    pub principal_ref: ParticipantRef,
}

impl AuthorityEnvelopeRef {
    /// Validate required authority hash references.
    pub fn validate(&self) -> Result<(), EconomyError> {
        if self.envelope_id == Hash256::ZERO {
            return Err(EconomyError::InvalidInput {
                reason: "authority_envelope.envelope_id must not be zero".into(),
            });
        }
        if self.authority_proof_hash == Hash256::ZERO {
            return Err(EconomyError::InvalidInput {
                reason: "authority_envelope.authority_proof_hash must not be zero".into(),
            });
        }
        self.principal_ref
            .validate("authority_envelope.principal_ref")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContributorType {
    Human,
    Holon,
    Agent,
    Company,
    Project,
    Foundation,
    Trust,
    Community,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContributionType {
    Code,
    Prompt,
    Workflow,
    Dataset,
    GovernanceModel,
    Design,
    Documentation,
    Agent,
    Service,
    Template,
    Policy,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValueContributionStatus {
    Draft,
    Offered,
    Active,
    Suspended,
    Revoked,
    Deprecated,
    Superseded,
}

/// A useful contribution made offerable in the EXOCHAIN economy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueContributionNode {
    pub contribution_node_id: Hash256,
    pub contributor_ref: ParticipantRef,
    pub contributor_type: ContributorType,
    pub contribution_name: String,
    pub contribution_type: ContributionType,
    pub source_uri: Option<String>,
    pub evidence_hash: Hash256,
    pub provenance_hash: Hash256,
    pub license_or_compact_ref: String,
    pub honor_good_terms_hash: Hash256,
    pub bailment_terms_hash: Hash256,
    pub settlement_ruleset_id: Hash256,
    pub beneficiary_ref: ParticipantRef,
    pub materiality_policy_id: Hash256,
    pub adoption_policy_id: Hash256,
    pub revocation_policy_id: Hash256,
    pub dispute_policy_id: Hash256,
    pub status: ValueContributionStatus,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct ValueContributionNodeHashPayload<'a> {
    domain: &'static str,
    contributor_ref: &'a ParticipantRef,
    contributor_type: ContributorType,
    contribution_name: &'a str,
    contribution_type: ContributionType,
    source_uri: Option<&'a str>,
    evidence_hash: &'a Hash256,
    provenance_hash: &'a Hash256,
    license_or_compact_ref: &'a str,
    honor_good_terms_hash: &'a Hash256,
    bailment_terms_hash: &'a Hash256,
    settlement_ruleset_id: &'a Hash256,
    beneficiary_ref: &'a ParticipantRef,
    materiality_policy_id: &'a Hash256,
    adoption_policy_id: &'a Hash256,
    revocation_policy_id: &'a Hash256,
    dispute_policy_id: &'a Hash256,
    status: ValueContributionStatus,
    created_at_hlc: &'a Timestamp,
}

impl ValueContributionNode {
    /// Validate structural fields.
    pub fn validate(&self) -> Result<(), EconomyError> {
        self.contributor_ref
            .validate("value_contribution.contributor_ref")?;
        self.beneficiary_ref
            .validate("value_contribution.beneficiary_ref")?;
        require_non_empty(
            &self.contribution_name,
            "value_contribution.contribution_name",
        )?;
        require_non_empty(
            &self.license_or_compact_ref,
            "value_contribution.license_or_compact_ref",
        )?;
        if let Some(source_uri) = &self.source_uri {
            require_non_empty(source_uri, "value_contribution.source_uri")?;
        }
        require_nonzero_hash(self.evidence_hash, "value_contribution.evidence_hash")?;
        require_nonzero_hash(self.provenance_hash, "value_contribution.provenance_hash")?;
        require_nonzero_hash(
            self.honor_good_terms_hash,
            "value_contribution.honor_good_terms_hash",
        )?;
        require_nonzero_hash(
            self.bailment_terms_hash,
            "value_contribution.bailment_terms_hash",
        )?;
        require_nonzero_hash(
            self.settlement_ruleset_id,
            "value_contribution.settlement_ruleset_id",
        )?;
        require_nonzero_hash(
            self.materiality_policy_id,
            "value_contribution.materiality_policy_id",
        )?;
        require_nonzero_hash(
            self.adoption_policy_id,
            "value_contribution.adoption_policy_id",
        )?;
        require_nonzero_hash(
            self.revocation_policy_id,
            "value_contribution.revocation_policy_id",
        )?;
        require_nonzero_hash(
            self.dispute_policy_id,
            "value_contribution.dispute_policy_id",
        )?;
        require_nonzero_timestamp(self.created_at_hlc, "value_contribution.created_at_hlc")?;
        Ok(())
    }

    /// Compute the canonical content hash.
    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&ValueContributionNodeHashPayload {
            domain: VALUE_CONTRIBUTION_NODE_HASH_DOMAIN,
            contributor_ref: &self.contributor_ref,
            contributor_type: self.contributor_type,
            contribution_name: &self.contribution_name,
            contribution_type: self.contribution_type,
            source_uri: self.source_uri.as_deref(),
            evidence_hash: &self.evidence_hash,
            provenance_hash: &self.provenance_hash,
            license_or_compact_ref: &self.license_or_compact_ref,
            honor_good_terms_hash: &self.honor_good_terms_hash,
            bailment_terms_hash: &self.bailment_terms_hash,
            settlement_ruleset_id: &self.settlement_ruleset_id,
            beneficiary_ref: &self.beneficiary_ref,
            materiality_policy_id: &self.materiality_policy_id,
            adoption_policy_id: &self.adoption_policy_id,
            revocation_policy_id: &self.revocation_policy_id,
            dispute_policy_id: &self.dispute_policy_id,
            status: self.status,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    /// Validate, compute, and set both canonical ID and content hash.
    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.contribution_node_id = hash;
        self.content_hash = hash;
        Ok(self)
    }

    /// Verify that ID and content hash match the canonical payload.
    pub fn verify_hashes(&self) -> Result<bool, EconomyError> {
        let hash = self.recompute_content_hash()?;
        Ok(self.contribution_node_id == hash && self.content_hash == hash)
    }
}

pub(crate) fn require_non_empty(value: &str, field: &'static str) -> Result<(), EconomyError> {
    if value.trim().is_empty() {
        Err(EconomyError::EmptyField { field })
    } else {
        Ok(())
    }
}

pub(crate) fn require_nonzero_hash(
    value: Hash256,
    field: &'static str,
) -> Result<(), EconomyError> {
    if value == Hash256::ZERO {
        Err(EconomyError::InvalidInput {
            reason: format!("{field} must not be Hash256::ZERO"),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn require_nonzero_timestamp(
    value: Timestamp,
    field: &'static str,
) -> Result<(), EconomyError> {
    if value == Timestamp::ZERO {
        Err(EconomyError::InvalidInput {
            reason: format!("{field} must not be Timestamp::ZERO"),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::{Did, Hash256, Timestamp};

    use super::*;

    pub fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    pub fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    pub fn did(label: &str) -> Did {
        Did::new(&format!("did:exo:{label}")).unwrap()
    }

    pub fn participant(label: &str) -> ParticipantRef {
        ParticipantRef::Did(did(label))
    }

    pub fn authority(label: &str) -> AuthorityEnvelopeRef {
        AuthorityEnvelopeRef {
            envelope_id: h(0x90),
            authority_proof_hash: h(0x91),
            principal_ref: participant(label),
        }
    }

    pub fn sample_node() -> ValueContributionNode {
        ValueContributionNode {
            contribution_node_id: Hash256::ZERO,
            contributor_ref: participant("contributor"),
            contributor_type: ContributorType::Human,
            contribution_name: "Useful contribution".into(),
            contribution_type: ContributionType::Code,
            source_uri: Some("https://example.test/source".into()),
            evidence_hash: h(0x01),
            provenance_hash: h(0x02),
            license_or_compact_ref: "MIT".into(),
            honor_good_terms_hash: h(0x03),
            bailment_terms_hash: h(0x04),
            settlement_ruleset_id: h(0x05),
            beneficiary_ref: ParticipantRef::HashedReference(h(0x06)),
            materiality_policy_id: h(0x07),
            adoption_policy_id: h(0x08),
            revocation_policy_id: h(0x09),
            dispute_policy_id: h(0x0A),
            status: ValueContributionStatus::Offered,
            created_at_hlc: ts(1_000),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_support::*, *};

    #[test]
    fn value_contribution_node_hash_stable() {
        let node = sample_node().anchor().unwrap();
        let again = sample_node().anchor().unwrap();
        assert_eq!(node.contribution_node_id, again.contribution_node_id);
        assert_eq!(node.content_hash, again.content_hash);
        assert!(node.verify_hashes().unwrap());
    }

    #[test]
    fn value_contribution_node_hash_changes_with_terms() {
        let node = sample_node().anchor().unwrap();
        let mut other = sample_node();
        other.honor_good_terms_hash = h(0x33);
        let other = other.anchor().unwrap();
        assert_ne!(node.content_hash, other.content_hash);
    }

    #[test]
    fn opaque_vault_pointer_requires_hash() {
        let ref_value = ParticipantRef::VaultPointer {
            vault_id: "vault".into(),
            record_hash: Hash256::ZERO,
        };
        assert!(ref_value.validate("beneficiary").is_err());
    }

    #[test]
    fn authority_envelope_rejects_zero_hashes() {
        let mut auth = authority("principal");
        auth.authority_proof_hash = Hash256::ZERO;
        assert!(auth.validate().is_err());
    }
}
