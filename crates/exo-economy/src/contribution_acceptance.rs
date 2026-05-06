//! Accepted contribution terms and delegated authority references.

use exo_core::{Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    contribution_offer::ContributionOffer,
    error::EconomyError,
    value_contribution::{
        AuthorityEnvelopeRef, ParticipantRef, require_non_empty, require_nonzero_hash,
        require_nonzero_timestamp,
    },
};

pub const CONTRIBUTION_ACCEPTANCE_HASH_DOMAIN: &str = "exo.economy.contribution_acceptance.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdopterType {
    Human,
    Holon,
    Agent,
    Company,
    Platform,
    Foundation,
    Trust,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionAcceptance {
    pub acceptance_id: Hash256,
    pub offer_id: Hash256,
    pub contribution_node_id: Hash256,
    pub adopter_ref: ParticipantRef,
    pub adopter_type: AdopterType,
    pub accepted_terms_hash: Hash256,
    pub accepted_bailment_terms_hash: Hash256,
    pub authority_proof_hash: Hash256,
    pub authority_envelope: AuthorityEnvelopeRef,
    pub intended_use: String,
    pub custody_scope: String,
    pub signature_ref: Hash256,
    pub accepted_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct ContributionAcceptanceHashPayload<'a> {
    domain: &'static str,
    offer_id: &'a Hash256,
    contribution_node_id: &'a Hash256,
    adopter_ref: &'a ParticipantRef,
    adopter_type: AdopterType,
    accepted_terms_hash: &'a Hash256,
    accepted_bailment_terms_hash: &'a Hash256,
    authority_proof_hash: &'a Hash256,
    authority_envelope: &'a AuthorityEnvelopeRef,
    intended_use: &'a str,
    custody_scope: &'a str,
    signature_ref: &'a Hash256,
    accepted_at_hlc: &'a Timestamp,
}

impl ContributionAcceptance {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(self.offer_id, "acceptance.offer_id")?;
        require_nonzero_hash(self.contribution_node_id, "acceptance.contribution_node_id")?;
        self.adopter_ref.validate("acceptance.adopter_ref")?;
        require_nonzero_hash(self.accepted_terms_hash, "acceptance.accepted_terms_hash")?;
        require_nonzero_hash(
            self.accepted_bailment_terms_hash,
            "acceptance.accepted_bailment_terms_hash",
        )?;
        require_nonzero_hash(self.authority_proof_hash, "acceptance.authority_proof_hash")?;
        self.authority_envelope.validate()?;
        if self.authority_envelope.authority_proof_hash != self.authority_proof_hash {
            return Err(EconomyError::HashMismatch {
                field: "acceptance.authority_proof_hash",
            });
        }
        require_non_empty(&self.intended_use, "acceptance.intended_use")?;
        require_non_empty(&self.custody_scope, "acceptance.custody_scope")?;
        require_nonzero_hash(self.signature_ref, "acceptance.signature_ref")?;
        require_nonzero_timestamp(self.accepted_at_hlc, "acceptance.accepted_at_hlc")
    }

    pub fn validate_against_offer(&self, offer: &ContributionOffer) -> Result<(), EconomyError> {
        self.validate()?;
        offer.validate()?;
        if self.offer_id != offer.offer_id {
            return Err(EconomyError::HashMismatch {
                field: "acceptance.offer_id",
            });
        }
        if self.contribution_node_id != offer.contribution_node_id {
            return Err(EconomyError::HashMismatch {
                field: "acceptance.contribution_node_id",
            });
        }
        if self.accepted_terms_hash != offer.terms_hash {
            return Err(EconomyError::HashMismatch {
                field: "acceptance.accepted_terms_hash",
            });
        }
        if self.accepted_bailment_terms_hash != offer.bailment_terms_hash {
            return Err(EconomyError::HashMismatch {
                field: "acceptance.accepted_bailment_terms_hash",
            });
        }
        Ok(())
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&ContributionAcceptanceHashPayload {
            domain: CONTRIBUTION_ACCEPTANCE_HASH_DOMAIN,
            offer_id: &self.offer_id,
            contribution_node_id: &self.contribution_node_id,
            adopter_ref: &self.adopter_ref,
            adopter_type: self.adopter_type,
            accepted_terms_hash: &self.accepted_terms_hash,
            accepted_bailment_terms_hash: &self.accepted_bailment_terms_hash,
            authority_proof_hash: &self.authority_proof_hash,
            authority_envelope: &self.authority_envelope,
            intended_use: &self.intended_use,
            custody_scope: &self.custody_scope,
            signature_ref: &self.signature_ref,
            accepted_at_hlc: &self.accepted_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.acceptance_id = hash;
        self.content_hash = hash;
        Ok(self)
    }

    pub fn verify_hashes(&self) -> Result<bool, EconomyError> {
        let hash = self.recompute_content_hash()?;
        Ok(self.acceptance_id == hash && self.content_hash == hash)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::{
        contribution_offer::test_support::sample_offer,
        value_contribution::test_support::{authority, h, participant, ts},
    };

    pub fn sample_acceptance() -> ContributionAcceptance {
        let offer = sample_offer().anchor().unwrap();
        ContributionAcceptance {
            acceptance_id: Hash256::ZERO,
            offer_id: offer.offer_id,
            contribution_node_id: offer.contribution_node_id,
            adopter_ref: participant("adopter"),
            adopter_type: AdopterType::Agent,
            accepted_terms_hash: offer.terms_hash,
            accepted_bailment_terms_hash: offer.bailment_terms_hash,
            authority_proof_hash: h(0x91),
            authority_envelope: authority("adopter-principal"),
            intended_use: "adopt into governed workflow".into(),
            custody_scope: "use and audit only".into(),
            signature_ref: h(0x61),
            accepted_at_hlc: ts(1_700),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_support::*, *};
    use crate::{
        contribution_offer::test_support::sample_offer, value_contribution::test_support::h,
    };

    #[test]
    fn contribution_acceptance_hash_stable() {
        let acceptance = sample_acceptance().anchor().unwrap();
        let again = sample_acceptance().anchor().unwrap();
        assert_eq!(acceptance.acceptance_id, again.acceptance_id);
        assert!(acceptance.verify_hashes().unwrap());
    }

    #[test]
    fn terms_hash_mismatch_rejected() {
        let offer = sample_offer().anchor().unwrap();
        let mut acceptance = sample_acceptance();
        acceptance.accepted_terms_hash = h(0xEE);
        let acceptance = acceptance.anchor().unwrap();
        assert!(matches!(
            acceptance.validate_against_offer(&offer),
            Err(EconomyError::HashMismatch {
                field: "acceptance.accepted_terms_hash"
            })
        ));
    }

    #[test]
    fn agent_acceptance_requires_authority_envelope_match() {
        let mut acceptance = sample_acceptance();
        acceptance.authority_proof_hash = h(0xA1);
        assert!(acceptance.validate().is_err());
    }
}
