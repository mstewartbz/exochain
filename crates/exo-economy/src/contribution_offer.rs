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

//! Contribution offers bind value nodes to terms and permitted use policy.

use exo_core::{Hash256, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    error::EconomyError,
    legacy::LegalEffect,
    value_contribution::{ParticipantRef, require_nonzero_hash, require_nonzero_timestamp},
};

pub const CONTRIBUTION_OFFER_HASH_DOMAIN: &str = "exo.economy.contribution_offer.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContributionOfferStatus {
    Draft,
    Offered,
    Accepted,
    Suspended,
    Revoked,
    Expired,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RequiredAuthorityLevel {
    Human,
    Company,
    Trust,
    Foundation,
    DelegatedAgent,
    DelegatedHolon,
    PublicProjectTreasury,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpirationOrReview {
    NoExpiration,
    ExpiresAt(Timestamp),
    ReviewAt(Timestamp),
}

impl ExpirationOrReview {
    fn validate(&self) -> Result<(), EconomyError> {
        match self {
            Self::NoExpiration => Ok(()),
            Self::ExpiresAt(value) => require_nonzero_timestamp(*value, "offer.expires_at"),
            Self::ReviewAt(value) => require_nonzero_timestamp(*value, "offer.review_at"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionOffer {
    pub offer_id: Hash256,
    pub contribution_node_id: Hash256,
    pub offeror_ref: ParticipantRef,
    pub terms_hash: Hash256,
    pub bailment_terms_hash: Hash256,
    pub permitted_use_policy: Hash256,
    pub prohibited_use_policy: Hash256,
    pub adoption_policy_id: Hash256,
    pub settlement_ruleset_id: Hash256,
    pub required_authority_level: RequiredAuthorityLevel,
    pub expiration_or_review: ExpirationOrReview,
    pub legal_effect: LegalEffect,
    pub status: ContributionOfferStatus,
    pub created_at_hlc: Timestamp,
    pub content_hash: Hash256,
}

#[derive(Serialize)]
struct ContributionOfferHashPayload<'a> {
    domain: &'static str,
    contribution_node_id: &'a Hash256,
    offeror_ref: &'a ParticipantRef,
    terms_hash: &'a Hash256,
    bailment_terms_hash: &'a Hash256,
    permitted_use_policy: &'a Hash256,
    prohibited_use_policy: &'a Hash256,
    adoption_policy_id: &'a Hash256,
    settlement_ruleset_id: &'a Hash256,
    required_authority_level: RequiredAuthorityLevel,
    expiration_or_review: &'a ExpirationOrReview,
    legal_effect: LegalEffect,
    status: ContributionOfferStatus,
    created_at_hlc: &'a Timestamp,
}

impl ContributionOffer {
    pub fn validate(&self) -> Result<(), EconomyError> {
        require_nonzero_hash(self.contribution_node_id, "offer.contribution_node_id")?;
        self.offeror_ref.validate("offer.offeror_ref")?;
        require_nonzero_hash(self.terms_hash, "offer.terms_hash")?;
        require_nonzero_hash(self.bailment_terms_hash, "offer.bailment_terms_hash")?;
        require_nonzero_hash(self.permitted_use_policy, "offer.permitted_use_policy")?;
        require_nonzero_hash(self.prohibited_use_policy, "offer.prohibited_use_policy")?;
        require_nonzero_hash(self.adoption_policy_id, "offer.adoption_policy_id")?;
        require_nonzero_hash(self.settlement_ruleset_id, "offer.settlement_ruleset_id")?;
        self.expiration_or_review.validate()?;
        if matches!(
            self.legal_effect,
            LegalEffect::ContributorAccepted | LegalEffect::RatifiedAgreement
        ) && !matches!(self.status, ContributionOfferStatus::Accepted)
        {
            return Err(EconomyError::InvalidInput {
                reason: "unaccepted offer cannot claim contributor-accepted or ratified effect"
                    .into(),
            });
        }
        require_nonzero_timestamp(self.created_at_hlc, "offer.created_at_hlc")
    }

    pub fn recompute_content_hash(&self) -> Result<Hash256, EconomyError> {
        hash_structured(&ContributionOfferHashPayload {
            domain: CONTRIBUTION_OFFER_HASH_DOMAIN,
            contribution_node_id: &self.contribution_node_id,
            offeror_ref: &self.offeror_ref,
            terms_hash: &self.terms_hash,
            bailment_terms_hash: &self.bailment_terms_hash,
            permitted_use_policy: &self.permitted_use_policy,
            prohibited_use_policy: &self.prohibited_use_policy,
            adoption_policy_id: &self.adoption_policy_id,
            settlement_ruleset_id: &self.settlement_ruleset_id,
            required_authority_level: self.required_authority_level,
            expiration_or_review: &self.expiration_or_review,
            legal_effect: self.legal_effect,
            status: self.status,
            created_at_hlc: &self.created_at_hlc,
        })
        .map_err(EconomyError::from)
    }

    pub fn anchor(mut self) -> Result<Self, EconomyError> {
        self.validate()?;
        let hash = self.recompute_content_hash()?;
        self.offer_id = hash;
        self.content_hash = hash;
        Ok(self)
    }

    pub fn verify_hashes(&self) -> Result<bool, EconomyError> {
        let hash = self.recompute_content_hash()?;
        Ok(self.offer_id == hash && self.content_hash == hash)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use exo_core::Hash256;

    use super::*;
    use crate::value_contribution::test_support::{h, participant, ts};

    pub fn sample_offer() -> ContributionOffer {
        ContributionOffer {
            offer_id: Hash256::ZERO,
            contribution_node_id: h(0x50),
            offeror_ref: participant("offeror"),
            terms_hash: h(0x51),
            bailment_terms_hash: h(0x52),
            permitted_use_policy: h(0x53),
            prohibited_use_policy: h(0x54),
            adoption_policy_id: h(0x55),
            settlement_ruleset_id: h(0x56),
            required_authority_level: RequiredAuthorityLevel::DelegatedAgent,
            expiration_or_review: ExpirationOrReview::ReviewAt(ts(20_000)),
            legal_effect: LegalEffect::OfferedTerms,
            status: ContributionOfferStatus::Offered,
            created_at_hlc: ts(1_600),
            content_hash: Hash256::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{test_support::*, *};
    use crate::value_contribution::test_support::h;

    #[test]
    fn contribution_offer_hash_stable() {
        let offer = sample_offer().anchor().unwrap();
        let again = sample_offer().anchor().unwrap();
        assert_eq!(offer.offer_id, again.offer_id);
        assert!(offer.verify_hashes().unwrap());
    }

    #[test]
    fn contribution_offer_binds_terms_hash() {
        let offer = sample_offer().anchor().unwrap();
        let mut other = sample_offer();
        other.terms_hash = h(0xAA);
        let other = other.anchor().unwrap();
        assert_ne!(offer.content_hash, other.content_hash);
    }

    #[test]
    fn unaccepted_offer_rejects_ratified_effect() {
        let mut offer = sample_offer();
        offer.legal_effect = LegalEffect::RatifiedAgreement;
        assert!(offer.validate().is_err());
    }
}
