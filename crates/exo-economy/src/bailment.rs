//! Bailment-style transactional wrappers for contribution adoption.

use exo_core::{Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    contribution_acceptance::ContributionAcceptance,
    contribution_offer::ContributionOffer,
    error::EconomyError,
    value_contribution::{
        ParticipantRef, require_non_empty, require_nonzero_hash, require_nonzero_timestamp,
    },
};

pub const BAILMENT_TERMS_HASH_DOMAIN: &str = "exo.economy.bailment_terms.v1";
pub const BAILMENT_WRAPPER_HASH_DOMAIN: &str = "exo.economy.bailment_wrapper.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BailmentTerms {
    pub terms_id: Hash256,
    pub terms_version: String,
    pub bailor_ref: ParticipantRef,
    pub bailee_ref_policy: Hash256,
    pub contribution_node_id: Hash256,
    pub permitted_use: String,
    pub prohibited_use: String,
    pub custody_scope: String,
    pub attribution_required: bool,
    pub settlement_required: bool,
    pub beneficiary_ref: ParticipantRef,
    pub revocation_policy_id: Hash256,
    pub dispute_policy_id: Hash256,
    pub audit_policy_id: Hash256,
    pub jurisdiction_ref: String,
    pub human_approval_required_for: Vec<String>,
    pub agent_execution_allowed: bool,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct BailmentTermsHashPayload<'a> {
    domain: &'static str,
    terms_version: &'a str,
    bailor_ref: &'a ParticipantRef,
    bailee_ref_policy: &'a Hash256,
    contribution_node_id: &'a Hash256,
    permitted_use: &'a str,
    prohibited_use: &'a str,
    custody_scope: &'a str,
    attribution_required: bool,
    settlement_required: bool,
    beneficiary_ref: &'a ParticipantRef,
    revocation_policy_id: &'a Hash256,
    dispute_policy_id: &'a Hash256,
    audit_policy_id: &'a Hash256,
    jurisdiction_ref: &'a str,
    human_approval_required_for: &'a [String],
    agent_execution_allowed: bool,
    created_at_hlc: &'a Timestamp,
}

impl BailmentTerms {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_non_empty(&self.terms_version, "bailment_terms.terms_version")?;
        self.bailor_ref.validate("bailment_terms.bailor_ref")?;
        self.beneficiary_ref
            .validate("bailment_terms.beneficiary_ref")?;
        require_nonzero_hash(self.bailee_ref_policy, "bailment_terms.bailee_ref_policy")?;
        require_nonzero_hash(
            self.contribution_node_id,
            "bailment_terms.contribution_node_id",
        )?;
        require_non_empty(&self.permitted_use, "bailment_terms.permitted_use")?;
        require_non_empty(&self.prohibited_use, "bailment_terms.prohibited_use")?;
        require_non_empty(&self.custody_scope, "bailment_terms.custody_scope")?;
        require_nonzero_hash(
            self.revocation_policy_id,
            "bailment_terms.revocation_policy_id",
        )?;
        require_nonzero_hash(self.dispute_policy_id, "bailment_terms.dispute_policy_id")?;
        require_nonzero_hash(self.audit_policy_id, "bailment_terms.audit_policy_id")?;
        require_non_empty(&self.jurisdiction_ref, "bailment_terms.jurisdiction_ref")?;
        for reason in &self.human_approval_required_for {
            require_non_empty(reason, "bailment_terms.human_approval_required_for")?;
        }
        require_nonzero_timestamp(self.created_at_hlc, "bailment_terms.created_at_hlc")
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&BailmentTermsHashPayload {
            domain: BAILMENT_TERMS_HASH_DOMAIN,
            terms_version: &self.terms_version,
            bailor_ref: &self.bailor_ref,
            bailee_ref_policy: &self.bailee_ref_policy,
            contribution_node_id: &self.contribution_node_id,
            permitted_use: &self.permitted_use,
            prohibited_use: &self.prohibited_use,
            custody_scope: &self.custody_scope,
            attribution_required: self.attribution_required,
            settlement_required: self.settlement_required,
            beneficiary_ref: &self.beneficiary_ref,
            revocation_policy_id: &self.revocation_policy_id,
            dispute_policy_id: &self.dispute_policy_id,
            audit_policy_id: &self.audit_policy_id,
            jurisdiction_ref: &self.jurisdiction_ref,
            human_approval_required_for: &self.human_approval_required_for,
            agent_execution_allowed: self.agent_execution_allowed,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.human_approval_required_for.sort();
        self.human_approval_required_for.dedup();
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.terms_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BailmentWrapperStatus {
    Draft,
    Active,
    Suspended,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BailmentWrapper {
    pub wrapper_id: Hash256,
    pub contribution_node_id: Hash256,
    pub offer_id: Hash256,
    pub acceptance_id: Hash256,
    pub accepted_terms_hash: Hash256,
    pub accepted_bailment_terms_hash: Hash256,
    pub bailor_ref: ParticipantRef,
    pub bailee_ref: ParticipantRef,
    pub custody_scope: String,
    pub settlement_ruleset_id: Hash256,
    pub signatures_or_authority_refs: Vec<Hash256>,
    pub status: BailmentWrapperStatus,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct BailmentWrapperHashPayload<'a> {
    domain: &'static str,
    contribution_node_id: &'a Hash256,
    offer_id: &'a Hash256,
    acceptance_id: &'a Hash256,
    accepted_terms_hash: &'a Hash256,
    accepted_bailment_terms_hash: &'a Hash256,
    bailor_ref: &'a ParticipantRef,
    bailee_ref: &'a ParticipantRef,
    custody_scope: &'a str,
    settlement_ruleset_id: &'a Hash256,
    signatures_or_authority_refs: &'a [Hash256],
    status: BailmentWrapperStatus,
    created_at_hlc: &'a Timestamp,
}

impl BailmentWrapper {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(
            self.contribution_node_id,
            "bailment_wrapper.contribution_node_id",
        )?;
        require_nonzero_hash(self.offer_id, "bailment_wrapper.offer_id")?;
        require_nonzero_hash(self.acceptance_id, "bailment_wrapper.acceptance_id")?;
        require_nonzero_hash(
            self.accepted_terms_hash,
            "bailment_wrapper.accepted_terms_hash",
        )?;
        require_nonzero_hash(
            self.accepted_bailment_terms_hash,
            "bailment_wrapper.accepted_bailment_terms_hash",
        )?;
        self.bailor_ref.validate("bailment_wrapper.bailor_ref")?;
        self.bailee_ref.validate("bailment_wrapper.bailee_ref")?;
        require_non_empty(&self.custody_scope, "bailment_wrapper.custody_scope")?;
        require_nonzero_hash(
            self.settlement_ruleset_id,
            "bailment_wrapper.settlement_ruleset_id",
        )?;
        if self.signatures_or_authority_refs.is_empty() {
            return Err(EconomyError::InvalidInput {
                reason: "bailment wrapper requires signature or authority reference".into(),
            });
        }
        for value in &self.signatures_or_authority_refs {
            require_nonzero_hash(*value, "bailment_wrapper.signatures_or_authority_refs")?;
        }
        require_nonzero_timestamp(self.created_at_hlc, "bailment_wrapper.created_at_hlc")
    }

    pub fn validate_against(
        &self,
        offer: &ContributionOffer,
        acceptance: &ContributionAcceptance,
        terms: &BailmentTerms,
    ) -> Result<(), EconomyError> {
        self.validate()?;
        acceptance.validate_against_offer(offer)?;
        if self.contribution_node_id != offer.contribution_node_id
            || self.contribution_node_id != terms.contribution_node_id
        {
            return Err(EconomyError::HashMismatch {
                field: "bailment_wrapper.contribution_node_id",
            });
        }
        if self.offer_id != offer.offer_id {
            return Err(EconomyError::HashMismatch {
                field: "bailment_wrapper.offer_id",
            });
        }
        if self.acceptance_id != acceptance.acceptance_id {
            return Err(EconomyError::HashMismatch {
                field: "bailment_wrapper.acceptance_id",
            });
        }
        if self.accepted_terms_hash != offer.terms_hash
            || self.accepted_terms_hash != acceptance.accepted_terms_hash
        {
            return Err(EconomyError::HashMismatch {
                field: "bailment_wrapper.accepted_terms_hash",
            });
        }
        if self.accepted_bailment_terms_hash != terms.terms_id
            || self.accepted_bailment_terms_hash != offer.bailment_terms_hash
            || self.accepted_bailment_terms_hash != acceptance.accepted_bailment_terms_hash
        {
            return Err(EconomyError::HashMismatch {
                field: "bailment_wrapper.accepted_bailment_terms_hash",
            });
        }
        if self.settlement_ruleset_id != offer.settlement_ruleset_id {
            return Err(EconomyError::HashMismatch {
                field: "bailment_wrapper.settlement_ruleset_id",
            });
        }
        if !matches!(self.status, BailmentWrapperStatus::Active) {
            return Err(EconomyError::AutomatedSettlementRejected {
                reason: "bailment wrapper must be active".into(),
            });
        }
        Ok(())
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&BailmentWrapperHashPayload {
            domain: BAILMENT_WRAPPER_HASH_DOMAIN,
            contribution_node_id: &self.contribution_node_id,
            offer_id: &self.offer_id,
            acceptance_id: &self.acceptance_id,
            accepted_terms_hash: &self.accepted_terms_hash,
            accepted_bailment_terms_hash: &self.accepted_bailment_terms_hash,
            bailor_ref: &self.bailor_ref,
            bailee_ref: &self.bailee_ref,
            custody_scope: &self.custody_scope,
            settlement_ruleset_id: &self.settlement_ruleset_id,
            signatures_or_authority_refs: &self.signatures_or_authority_refs,
            status: self.status,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.signatures_or_authority_refs.sort();
        self.signatures_or_authority_refs.dedup();
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.wrapper_id = hash;
        self.content_hash = hash;
        Ok(self)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::{
        contribution_acceptance::test_support::sample_acceptance,
        contribution_offer::test_support::sample_offer,
        value_contribution::test_support::{h, participant, ts},
    };

    pub fn sample_terms() -> BailmentTerms {
        BailmentTerms {
            terms_id: Hash256::ZERO,
            terms_version: "honorgood-bailment-v1".into(),
            bailor_ref: participant("offeror"),
            bailee_ref_policy: h(0x70),
            contribution_node_id: h(0x50),
            permitted_use: "governed adoption and use".into(),
            prohibited_use: "off-policy custody or resale".into(),
            custody_scope: "limited use and audit custody".into(),
            attribution_required: true,
            settlement_required: true,
            beneficiary_ref: participant("beneficiary"),
            revocation_policy_id: h(0x71),
            dispute_policy_id: h(0x72),
            audit_policy_id: h(0x73),
            jurisdiction_ref: "off-ledger:jurisdiction-policy".into(),
            human_approval_required_for: vec![
                "new legal template".into(),
                "dispute".into(),
                "revocation".into(),
            ],
            agent_execution_allowed: true,
            created_at_hlc: ts(1_800),
            content_hash: Hash256::ZERO,
        }
    }

    pub fn coherent_offer_acceptance_terms() -> (
        crate::contribution_offer::ContributionOffer,
        ContributionAcceptance,
        BailmentTerms,
    ) {
        let terms = sample_terms().anchor().unwrap();
        let mut offer = sample_offer();
        offer.bailment_terms_hash = terms.terms_id;
        let offer = offer.anchor().unwrap();
        let mut acceptance = sample_acceptance();
        acceptance.offer_id = offer.offer_id;
        acceptance.contribution_node_id = offer.contribution_node_id;
        acceptance.accepted_terms_hash = offer.terms_hash;
        acceptance.accepted_bailment_terms_hash = offer.bailment_terms_hash;
        let acceptance = acceptance.anchor().unwrap();
        (offer, acceptance, terms)
    }

    pub fn sample_wrapper() -> BailmentWrapper {
        let (offer, acceptance, terms) = coherent_offer_acceptance_terms();
        BailmentWrapper {
            wrapper_id: Hash256::ZERO,
            contribution_node_id: offer.contribution_node_id,
            offer_id: offer.offer_id,
            acceptance_id: acceptance.acceptance_id,
            accepted_terms_hash: offer.terms_hash,
            accepted_bailment_terms_hash: terms.terms_id,
            bailor_ref: participant("offeror"),
            bailee_ref: participant("adopter"),
            custody_scope: "limited use and audit custody".into(),
            settlement_ruleset_id: offer.settlement_ruleset_id,
            signatures_or_authority_refs: vec![acceptance.signature_ref],
            status: BailmentWrapperStatus::Active,
            created_at_hlc: ts(1_900),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use crate::value_contribution::test_support::h;

    #[test]
    fn bailment_terms_hash_stable() {
        let terms = sample_terms().anchor().unwrap();
        let again = sample_terms().anchor().unwrap();
        assert_eq!(terms.terms_id, again.terms_id);
    }

    #[test]
    fn wrapper_requires_accepted_terms_and_authority() {
        let (offer, acceptance, terms) = coherent_offer_acceptance_terms();
        let wrapper = sample_wrapper().anchor().unwrap();
        assert!(
            wrapper
                .validate_against(&offer, &acceptance, &terms)
                .is_ok()
        );
    }

    #[test]
    fn wrapper_rejects_mismatched_bailment_terms() {
        let (offer, acceptance, terms) = coherent_offer_acceptance_terms();
        let mut wrapper = sample_wrapper();
        wrapper.accepted_bailment_terms_hash = h(0xDD);
        let wrapper = wrapper.anchor().unwrap();
        assert!(
            wrapper
                .validate_against(&offer, &acceptance, &terms)
                .is_err()
        );
    }
}
