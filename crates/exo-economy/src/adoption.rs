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

//! Adoption, use, and value events for value-for-value settlement loops.

use exo_core::{Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    bailment::BailmentWrapper,
    contribution_acceptance::ContributionAcceptance,
    contribution_offer::ContributionOffer,
    error::EconomyError,
    legacy::MaterialityTier,
    types::MicroExo,
    value_contribution::{
        ParticipantRef, require_non_empty, require_nonzero_hash, require_nonzero_timestamp,
    },
};

pub const ADOPTION_EVENT_HASH_DOMAIN: &str = "exo.economy.adoption_event.v1";
pub const USE_EVENT_HASH_DOMAIN: &str = "exo.economy.use_event.v1";
pub const VALUE_EVENT_HASH_DOMAIN: &str = "exo.economy.value_event.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptionEvent {
    pub adoption_id: Hash256,
    pub contribution_node_id: Hash256,
    pub offer_id: Hash256,
    pub acceptance_id: Hash256,
    pub adopter_ref: ParticipantRef,
    pub adopting_system: String,
    pub mission_id: Option<Hash256>,
    pub accepted_terms_hash: Hash256,
    pub bailment_wrapper_id: Hash256,
    pub intended_use: String,
    pub materiality_at_adoption: MaterialityTier,
    pub authority_proof_hash: Hash256,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct AdoptionEventHashPayload<'a> {
    domain: &'static str,
    contribution_node_id: &'a Hash256,
    offer_id: &'a Hash256,
    acceptance_id: &'a Hash256,
    adopter_ref: &'a ParticipantRef,
    adopting_system: &'a str,
    mission_id: Option<&'a Hash256>,
    accepted_terms_hash: &'a Hash256,
    bailment_wrapper_id: &'a Hash256,
    intended_use: &'a str,
    materiality_at_adoption: MaterialityTier,
    authority_proof_hash: &'a Hash256,
    created_at_hlc: &'a Timestamp,
}

impl AdoptionEvent {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(self.contribution_node_id, "adoption.contribution_node_id")?;
        require_nonzero_hash(self.offer_id, "adoption.offer_id")?;
        require_nonzero_hash(self.acceptance_id, "adoption.acceptance_id")?;
        self.adopter_ref.validate("adoption.adopter_ref")?;
        require_non_empty(&self.adopting_system, "adoption.adopting_system")?;
        if let Some(id) = self.mission_id {
            require_nonzero_hash(id, "adoption.mission_id")?;
        }
        require_nonzero_hash(self.accepted_terms_hash, "adoption.accepted_terms_hash")?;
        require_nonzero_hash(self.bailment_wrapper_id, "adoption.bailment_wrapper_id")?;
        require_non_empty(&self.intended_use, "adoption.intended_use")?;
        require_nonzero_hash(self.authority_proof_hash, "adoption.authority_proof_hash")?;
        require_nonzero_timestamp(self.created_at_hlc, "adoption.created_at_hlc")
    }

    pub fn validate_against(
        &self,
        offer: &ContributionOffer,
        acceptance: &ContributionAcceptance,
        wrapper: &BailmentWrapper,
    ) -> Result<(), EconomyError> {
        self.validate()?;
        if self.contribution_node_id != offer.contribution_node_id
            || self.contribution_node_id != acceptance.contribution_node_id
            || self.contribution_node_id != wrapper.contribution_node_id
        {
            return Err(EconomyError::HashMismatch {
                field: "adoption.contribution_node_id",
            });
        }
        if self.offer_id != offer.offer_id || self.offer_id != wrapper.offer_id {
            return Err(EconomyError::HashMismatch {
                field: "adoption.offer_id",
            });
        }
        if self.acceptance_id != acceptance.acceptance_id
            || self.acceptance_id != wrapper.acceptance_id
        {
            return Err(EconomyError::HashMismatch {
                field: "adoption.acceptance_id",
            });
        }
        if self.accepted_terms_hash != offer.terms_hash
            || self.accepted_terms_hash != acceptance.accepted_terms_hash
        {
            return Err(EconomyError::HashMismatch {
                field: "adoption.accepted_terms_hash",
            });
        }
        if self.bailment_wrapper_id != wrapper.wrapper_id {
            return Err(EconomyError::HashMismatch {
                field: "adoption.bailment_wrapper_id",
            });
        }
        if self.authority_proof_hash != acceptance.authority_proof_hash {
            return Err(EconomyError::HashMismatch {
                field: "adoption.authority_proof_hash",
            });
        }
        Ok(())
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&AdoptionEventHashPayload {
            domain: ADOPTION_EVENT_HASH_DOMAIN,
            contribution_node_id: &self.contribution_node_id,
            offer_id: &self.offer_id,
            acceptance_id: &self.acceptance_id,
            adopter_ref: &self.adopter_ref,
            adopting_system: &self.adopting_system,
            mission_id: self.mission_id.as_ref(),
            accepted_terms_hash: &self.accepted_terms_hash,
            bailment_wrapper_id: &self.bailment_wrapper_id,
            intended_use: &self.intended_use,
            materiality_at_adoption: self.materiality_at_adoption,
            authority_proof_hash: &self.authority_proof_hash,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.adoption_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UseType {
    RuntimeExecution,
    BuildDependency,
    DesignReference,
    GovernanceApplication,
    DocumentationReuse,
    AgentWorkflow,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseEvent {
    pub use_event_id: Hash256,
    pub adoption_id: Hash256,
    pub contribution_node_id: Hash256,
    pub using_system: String,
    pub mission_id: Option<Hash256>,
    pub use_type: UseType,
    pub materiality_observed: MaterialityTier,
    pub evidence_hash: Hash256,
    pub bailment_wrapper_id: Hash256,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct UseEventHashPayload<'a> {
    domain: &'static str,
    adoption_id: &'a Hash256,
    contribution_node_id: &'a Hash256,
    using_system: &'a str,
    mission_id: Option<&'a Hash256>,
    use_type: UseType,
    materiality_observed: MaterialityTier,
    evidence_hash: &'a Hash256,
    bailment_wrapper_id: &'a Hash256,
    created_at_hlc: &'a Timestamp,
}

impl UseEvent {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(self.adoption_id, "use_event.adoption_id")?;
        require_nonzero_hash(self.contribution_node_id, "use_event.contribution_node_id")?;
        require_non_empty(&self.using_system, "use_event.using_system")?;
        if let Some(id) = self.mission_id {
            require_nonzero_hash(id, "use_event.mission_id")?;
        }
        require_nonzero_hash(self.evidence_hash, "use_event.evidence_hash")?;
        require_nonzero_hash(self.bailment_wrapper_id, "use_event.bailment_wrapper_id")?;
        require_nonzero_timestamp(self.created_at_hlc, "use_event.created_at_hlc")
    }

    pub fn validate_against_adoption(&self, adoption: &AdoptionEvent) -> Result<(), EconomyError> {
        self.validate()?;
        if self.adoption_id != adoption.adoption_id {
            return Err(EconomyError::HashMismatch {
                field: "use_event.adoption_id",
            });
        }
        if self.contribution_node_id != adoption.contribution_node_id {
            return Err(EconomyError::HashMismatch {
                field: "use_event.contribution_node_id",
            });
        }
        if self.bailment_wrapper_id != adoption.bailment_wrapper_id {
            return Err(EconomyError::HashMismatch {
                field: "use_event.bailment_wrapper_id",
            });
        }
        Ok(())
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&UseEventHashPayload {
            domain: USE_EVENT_HASH_DOMAIN,
            adoption_id: &self.adoption_id,
            contribution_node_id: &self.contribution_node_id,
            using_system: &self.using_system,
            mission_id: self.mission_id.as_ref(),
            use_type: self.use_type,
            materiality_observed: self.materiality_observed,
            evidence_hash: &self.evidence_hash,
            bailment_wrapper_id: &self.bailment_wrapper_id,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.use_event_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValueBasis {
    Revenue,
    ProtocolFee,
    ChannelFee,
    MissionSurplus,
    CostSavings,
    UsageMetric,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueEvent {
    pub value_event_id: Hash256,
    pub use_event_id: Hash256,
    pub contribution_node_id: Hash256,
    pub mission_id: Option<Hash256>,
    pub value_basis: ValueBasis,
    pub measured_value_micro_exo: MicroExo,
    pub measurement_evidence_hash: Hash256,
    pub measurement_policy_id: Hash256,
    pub settlement_triggered: bool,
    pub zero_fee_reason_required: bool,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct ValueEventHashPayload<'a> {
    domain: &'static str,
    use_event_id: &'a Hash256,
    contribution_node_id: &'a Hash256,
    mission_id: Option<&'a Hash256>,
    value_basis: &'a ValueBasis,
    measured_value_micro_exo: MicroExo,
    measurement_evidence_hash: &'a Hash256,
    measurement_policy_id: &'a Hash256,
    settlement_triggered: bool,
    zero_fee_reason_required: bool,
    created_at_hlc: &'a Timestamp,
}

impl ValueEvent {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(self.use_event_id, "value_event.use_event_id")?;
        require_nonzero_hash(
            self.contribution_node_id,
            "value_event.contribution_node_id",
        )?;
        if let Some(id) = self.mission_id {
            require_nonzero_hash(id, "value_event.mission_id")?;
        }
        if matches!(self.value_basis, ValueBasis::Other(_)) {
            return Err(EconomyError::UnsupportedSettlementBasis {
                basis: "value_event.other".into(),
            });
        }
        require_nonzero_hash(
            self.measurement_evidence_hash,
            "value_event.measurement_evidence_hash",
        )?;
        require_nonzero_hash(
            self.measurement_policy_id,
            "value_event.measurement_policy_id",
        )?;
        if self.measured_value_micro_exo == 0 && !self.zero_fee_reason_required {
            return Err(EconomyError::InvalidInput {
                reason: "zero measured value requires explicit zero-fee settlement reason".into(),
            });
        }
        require_nonzero_timestamp(self.created_at_hlc, "value_event.created_at_hlc")
    }

    pub fn validate_against_use_event(&self, use_event: &UseEvent) -> Result<(), EconomyError> {
        self.validate()?;
        if self.use_event_id != use_event.use_event_id {
            return Err(EconomyError::HashMismatch {
                field: "value_event.use_event_id",
            });
        }
        if self.contribution_node_id != use_event.contribution_node_id {
            return Err(EconomyError::HashMismatch {
                field: "value_event.contribution_node_id",
            });
        }
        Ok(())
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&ValueEventHashPayload {
            domain: VALUE_EVENT_HASH_DOMAIN,
            use_event_id: &self.use_event_id,
            contribution_node_id: &self.contribution_node_id,
            mission_id: self.mission_id.as_ref(),
            value_basis: &self.value_basis,
            measured_value_micro_exo: self.measured_value_micro_exo,
            measurement_evidence_hash: &self.measurement_evidence_hash,
            measurement_policy_id: &self.measurement_policy_id,
            settlement_triggered: self.settlement_triggered,
            zero_fee_reason_required: self.zero_fee_reason_required,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.value_event_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::{
        bailment::test_support::sample_wrapper,
        value_contribution::test_support::{h, participant, ts},
    };

    pub fn sample_adoption() -> AdoptionEvent {
        let wrapper = sample_wrapper().anchor().unwrap();
        AdoptionEvent {
            adoption_id: Hash256::ZERO,
            contribution_node_id: wrapper.contribution_node_id,
            offer_id: wrapper.offer_id,
            acceptance_id: wrapper.acceptance_id,
            adopter_ref: participant("adopter"),
            adopting_system: "CommandBase".into(),
            mission_id: Some(h(0x80)),
            accepted_terms_hash: wrapper.accepted_terms_hash,
            bailment_wrapper_id: wrapper.wrapper_id,
            intended_use: "operational cockpit workflow".into(),
            materiality_at_adoption: MaterialityTier::Foundational,
            authority_proof_hash: h(0x91),
            created_at_hlc: ts(2_000),
            content_hash: Hash256::ZERO,
        }
    }

    pub fn sample_use_event() -> UseEvent {
        let adoption = sample_adoption().anchor().unwrap();
        UseEvent {
            use_event_id: Hash256::ZERO,
            adoption_id: adoption.adoption_id,
            contribution_node_id: adoption.contribution_node_id,
            using_system: "CommandBase".into(),
            mission_id: adoption.mission_id,
            use_type: UseType::AgentWorkflow,
            materiality_observed: MaterialityTier::Foundational,
            evidence_hash: h(0x81),
            bailment_wrapper_id: adoption.bailment_wrapper_id,
            created_at_hlc: ts(2_100),
            content_hash: Hash256::ZERO,
        }
    }

    pub fn sample_value_event() -> ValueEvent {
        let use_event = sample_use_event().anchor().unwrap();
        ValueEvent {
            value_event_id: Hash256::ZERO,
            use_event_id: use_event.use_event_id,
            contribution_node_id: use_event.contribution_node_id,
            mission_id: use_event.mission_id,
            value_basis: ValueBasis::Revenue,
            measured_value_micro_exo: 0,
            measurement_evidence_hash: h(0x82),
            measurement_policy_id: h(0x83),
            settlement_triggered: true,
            zero_fee_reason_required: true,
            created_at_hlc: ts(2_200),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ValueBasis, test_support::*};
    use crate::{
        bailment::test_support::{coherent_offer_acceptance_terms, sample_wrapper},
        value_contribution::test_support::h,
    };

    #[test]
    fn adoption_requires_offer_acceptance_and_wrapper() {
        let (offer, acceptance, terms) = coherent_offer_acceptance_terms();
        let wrapper = sample_wrapper().anchor().unwrap();
        assert!(
            wrapper
                .validate_against(&offer, &acceptance, &terms)
                .is_ok()
        );
        let adoption = sample_adoption().anchor().unwrap();
        assert!(
            adoption
                .validate_against(&offer, &acceptance, &wrapper)
                .is_ok()
        );
    }

    #[test]
    fn use_event_requires_valid_adoption() {
        let adoption = sample_adoption().anchor().unwrap();
        let use_event = sample_use_event().anchor().unwrap();
        assert!(use_event.validate_against_adoption(&adoption).is_ok());
    }

    #[test]
    fn value_event_requires_valid_use_event_and_measurement_policy() {
        let use_event = sample_use_event().anchor().unwrap();
        let value_event = sample_value_event().anchor().unwrap();
        assert!(value_event.validate_against_use_event(&use_event).is_ok());
    }

    #[test]
    fn value_event_rejects_missing_zero_reason_requirement() {
        let mut event = sample_value_event();
        event.zero_fee_reason_required = false;
        assert!(event.validate().is_err());
    }

    #[test]
    fn value_event_rejects_mismatched_use_event() {
        let use_event = sample_use_event().anchor().unwrap();
        let mut value_event = sample_value_event();
        value_event.use_event_id = h(0xEF);
        let value_event = value_event.anchor().unwrap();
        assert!(value_event.validate_against_use_event(&use_event).is_err());
    }

    #[test]
    fn adoption_rejects_mismatched_offer_acceptance_wrapper_and_authority() {
        let (offer, acceptance, _) = coherent_offer_acceptance_terms();
        let wrapper = sample_wrapper().anchor().unwrap();
        let adoption = sample_adoption().anchor().unwrap();

        let mut wrong_offer = adoption.clone();
        wrong_offer.offer_id = h(0xA0);
        assert!(
            wrong_offer
                .validate_against(&offer, &acceptance, &wrapper)
                .is_err()
        );

        let mut wrong_acceptance = adoption.clone();
        wrong_acceptance.acceptance_id = h(0xA1);
        assert!(
            wrong_acceptance
                .validate_against(&offer, &acceptance, &wrapper)
                .is_err()
        );

        let mut wrong_terms = adoption.clone();
        wrong_terms.accepted_terms_hash = h(0xA2);
        assert!(
            wrong_terms
                .validate_against(&offer, &acceptance, &wrapper)
                .is_err()
        );

        let mut wrong_wrapper = adoption.clone();
        wrong_wrapper.bailment_wrapper_id = h(0xA3);
        assert!(
            wrong_wrapper
                .validate_against(&offer, &acceptance, &wrapper)
                .is_err()
        );

        let mut wrong_authority = adoption;
        wrong_authority.authority_proof_hash = h(0xA4);
        assert!(
            wrong_authority
                .validate_against(&offer, &acceptance, &wrapper)
                .is_err()
        );
    }

    #[test]
    fn use_event_rejects_mismatched_adoption_fields() {
        let adoption = sample_adoption().anchor().unwrap();
        let use_event = sample_use_event().anchor().unwrap();

        let mut wrong_adoption = use_event.clone();
        wrong_adoption.adoption_id = h(0xB0);
        assert!(wrong_adoption.validate_against_adoption(&adoption).is_err());

        let mut wrong_node = use_event.clone();
        wrong_node.contribution_node_id = h(0xB1);
        assert!(wrong_node.validate_against_adoption(&adoption).is_err());

        let mut wrong_wrapper = use_event;
        wrong_wrapper.bailment_wrapper_id = h(0xB2);
        assert!(wrong_wrapper.validate_against_adoption(&adoption).is_err());
    }

    #[test]
    fn value_event_rejects_unsupported_basis_and_zero_mission_id() {
        let mut value_event = sample_value_event();
        value_event.value_basis = ValueBasis::Other("custom".into());
        assert!(matches!(
            value_event.validate(),
            Err(crate::error::EconomyError::UnsupportedSettlementBasis { .. })
        ));

        value_event.value_basis = ValueBasis::UsageMetric;
        value_event.mission_id = Some(exo_core::Hash256::ZERO);
        assert!(value_event.validate().is_err());
    }
}
