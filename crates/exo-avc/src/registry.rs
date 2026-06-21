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

//! Deterministic in-memory registry for AVC credentials, revocations,
//! receipts, and the ancillary state required by validation
//! (issuer public keys, validated authority chain hashes, consent and
//! policy reference existence).
//!
//! All maps and sets are `BTreeMap`/`BTreeSet` so iteration order is
//! deterministic. Runtime adapters can provide durable storage by
//! implementing the same registry traits.

use std::collections::{BTreeMap, BTreeSet};

use exo_authority::permission::Permission;
use exo_core::{Did, Hash256, PublicKey, Timestamp, crypto};
use serde::{Deserialize, Serialize};

use crate::{
    credential::AutonomousVolitionCredential, error::AvcError, receipt::AvcTrustReceipt,
    revocation::AvcRevocation,
};

/// Read-only registry interface used by validation.
pub trait AvcRegistryRead {
    fn resolve_public_key(&self, did: &Did) -> Option<PublicKey>;
    fn resolve_issuer_permission_grant(&self, did: &Did) -> Option<Vec<Permission>>;
    fn resolve_human_approval_key(&self, did: &Did) -> Option<PublicKey>;
    fn is_revoked(&self, credential_id: &Hash256) -> bool;
    fn get_revocation(&self, credential_id: &Hash256) -> Option<AvcRevocation>;
    fn consent_ref_exists(&self, consent_id: &Hash256) -> bool;
    fn policy_ref_exists(&self, policy_id: &Hash256, policy_version: u16) -> bool;
    /// Returns true when the registry has previously verified an
    /// authority chain whose hash equals `chain_hash` and which has not
    /// expired as of `now`. The registry is expected to update this
    /// state via an out-of-band integration with `exo-authority`.
    fn authority_chain_valid(&self, chain_hash: &Hash256, now: &Timestamp) -> bool;
    fn get_credential(&self, credential_id: &Hash256) -> Option<AutonomousVolitionCredential>;
    fn list_credentials_for_subject(&self, subject_did: &Did) -> Vec<AutonomousVolitionCredential>;
}

/// Mutating registry interface used by node API handlers and tests.
pub trait AvcRegistryWrite: AvcRegistryRead {
    /// Store a credential only after its issuer key resolves and its
    /// issuer signature verifies against the canonical signing payload.
    fn put_credential(
        &mut self,
        credential: AutonomousVolitionCredential,
    ) -> Result<Hash256, AvcError>;
    fn put_revocation(&mut self, revocation: AvcRevocation) -> Result<(), AvcError>;
    fn put_receipt(&mut self, receipt: AvcTrustReceipt) -> Result<(), AvcError>;
    fn put_public_key(&mut self, did: Did, public_key: PublicKey);
    fn put_receipt_validator_public_key(&mut self, did: Did, public_key: PublicKey);
    fn put_issuer_permission_grant(&mut self, did: Did, granted_permissions: Vec<Permission>);
    fn put_human_approval_key(&mut self, did: Did, public_key: PublicKey);
    fn add_consent_ref(&mut self, consent_id: Hash256);
    fn add_policy_ref(&mut self, policy_id: Hash256, policy_version: u16);
    fn mark_authority_chain_valid(&mut self, chain_hash: Hash256);
    fn revoke_authority_chain(&mut self, chain_hash: &Hash256);
}

/// Durable AVC runtime records.
///
/// This intentionally persists issued credentials, revocations, and receipts,
/// but not issuer/actor public-key trust anchors. Runtime adapters must
/// re-establish key trust from verified configuration on startup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvcRegistryDurableState {
    pub credentials: BTreeMap<Hash256, AutonomousVolitionCredential>,
    pub revocations: BTreeMap<Hash256, AvcRevocation>,
    pub receipts: BTreeMap<Hash256, AvcTrustReceipt>,
    #[serde(default)]
    pub receipt_chain_head: Option<Hash256>,
}

/// Deterministic in-memory implementation of the registry traits.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAvcRegistry {
    credentials: BTreeMap<Hash256, AutonomousVolitionCredential>,
    by_subject: BTreeMap<Did, BTreeSet<Hash256>>,
    revocations: BTreeMap<Hash256, AvcRevocation>,
    receipts: BTreeMap<Hash256, AvcTrustReceipt>,
    receipt_chain_head: Option<Hash256>,
    public_keys: BTreeMap<Did, PublicKey>,
    receipt_validator_public_keys: BTreeMap<Did, PublicKey>,
    issuer_permission_grants: BTreeMap<Did, BTreeSet<Permission>>,
    human_approval_keys: BTreeMap<Did, PublicKey>,
    consent_refs: BTreeSet<Hash256>,
    policy_refs: BTreeSet<(Hash256, u16)>,
    authority_chains: BTreeSet<Hash256>,
}

impl InMemoryAvcRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Export the durable portion of the registry.
    #[must_use]
    pub fn durable_state(&self) -> AvcRegistryDurableState {
        AvcRegistryDurableState {
            credentials: self.credentials.clone(),
            revocations: self.revocations.clone(),
            receipts: self.receipts.clone(),
            receipt_chain_head: self.receipt_chain_head,
        }
    }

    /// Reconstruct an in-memory registry from durable runtime records.
    ///
    /// Issuer and actor public-key trust anchors are deliberately not restored
    /// from this state; the node must re-register those from verified startup
    /// configuration before validation can allow credentials.
    pub fn from_durable_state(state: AvcRegistryDurableState) -> Result<Self, AvcError> {
        let mut registry = Self::new();

        for (stored_id, credential) in state.credentials {
            let actual_id = credential.id()?;
            if stored_id != actual_id {
                return Err(AvcError::Registry {
                    reason: format!(
                        "durable credential key {stored_id} does not match computed id {actual_id}"
                    ),
                });
            }
            if registry.credentials.contains_key(&stored_id) {
                return Err(AvcError::Registry {
                    reason: format!("duplicate durable credential {stored_id}"),
                });
            }
            registry
                .by_subject
                .entry(credential.subject_did.clone())
                .or_default()
                .insert(stored_id);
            registry.credentials.insert(stored_id, credential);
        }

        for (stored_id, revocation) in state.revocations {
            if stored_id != revocation.credential_id {
                return Err(AvcError::Registry {
                    reason: format!(
                        "durable revocation key {stored_id} does not match credential id {}",
                        revocation.credential_id
                    ),
                });
            }
            if revocation.schema_version != crate::credential::AVC_SCHEMA_VERSION {
                return Err(AvcError::UnsupportedSchema {
                    got: revocation.schema_version,
                    supported: crate::credential::AVC_SCHEMA_VERSION,
                });
            }
            if revocation.signature.is_empty() {
                return Err(AvcError::InvalidInput {
                    reason: format!(
                        "durable revocation for credential {stored_id} has an empty signature"
                    ),
                });
            }
            let Some(credential) = registry.credentials.get(&stored_id) else {
                return Err(AvcError::InvalidInput {
                    reason: format!("durable revocation references unknown credential {stored_id}"),
                });
            };
            if revocation.revoker_did != credential.issuer_did
                && revocation.revoker_did != credential.principal_did
            {
                return Err(AvcError::InvalidInput {
                    reason: format!(
                        "durable revocation revoker {} is not authorized for credential {stored_id}",
                        revocation.revoker_did
                    ),
                });
            }
            registry.revocations.insert(stored_id, revocation);
        }

        for (stored_id, receipt) in state.receipts {
            if stored_id != receipt.receipt_id {
                return Err(AvcError::Registry {
                    reason: format!(
                        "durable receipt key {stored_id} does not match receipt id {}",
                        receipt.receipt_id
                    ),
                });
            }
            registry.validate_receipt_structural(&receipt)?;
            registry.receipts.insert(stored_id, receipt);
        }
        registry.validate_durable_receipt_evidence(state.receipt_chain_head)?;
        registry.receipt_chain_head = state.receipt_chain_head;

        Ok(registry)
    }

    /// Replace only the durable runtime records, preserving trust anchors and
    /// locally configured validation context.
    ///
    /// This is intended for runtime adapters that reload durable state from a
    /// database while issuer/actor public keys, human approval keys, consent
    /// refs, policy refs, and authority-chain cache entries remain supplied by
    /// verified startup configuration or live runtime registration.
    pub fn apply_durable_state(&mut self, state: AvcRegistryDurableState) -> Result<(), AvcError> {
        let durable = Self::from_durable_state(state)?;
        let mut candidate = self.clone();
        candidate.credentials = durable.credentials;
        candidate.by_subject = durable.by_subject;
        candidate.revocations.clear();
        candidate.receipts = durable.receipts;
        candidate.receipt_chain_head = durable.receipt_chain_head;

        for revocation in durable.revocations.into_values() {
            candidate.validate_revocation(&revocation)?;
            candidate
                .revocations
                .insert(revocation.credential_id, revocation);
        }
        candidate.validate_loaded_receipts()?;

        self.credentials = candidate.credentials;
        self.by_subject = candidate.by_subject;
        self.revocations = candidate.revocations;
        self.receipts = candidate.receipts;
        self.receipt_chain_head = candidate.receipt_chain_head;
        Ok(())
    }

    /// Revalidate durable revocations after startup trust anchors have been
    /// registered. This lets runtime adapters load durable records before
    /// verified configuration is available while still failing closed before
    /// those revocations are trusted for validation.
    pub fn validate_loaded_revocations(&self) -> Result<(), AvcError> {
        for revocation in self.revocations.values() {
            self.validate_revocation(revocation)?;
        }
        Ok(())
    }

    /// Revalidate durable receipts after validator trust anchors have been
    /// registered. Durable import only performs structural checks because
    /// startup trust anchors are intentionally restored out-of-band.
    pub fn validate_loaded_receipts(&self) -> Result<(), AvcError> {
        for receipt in self.receipts.values() {
            self.validate_receipt(receipt)?;
        }
        Ok(())
    }

    /// Number of credentials currently stored.
    #[must_use]
    pub fn credential_count(&self) -> usize {
        self.credentials.len()
    }

    /// Number of revocations currently stored.
    #[must_use]
    pub fn revocation_count(&self) -> usize {
        self.revocations.len()
    }

    /// Number of receipts currently stored.
    #[must_use]
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    #[must_use]
    pub fn receipt_chain_head(&self) -> Option<Hash256> {
        self.receipt_chain_head
    }

    fn validate_unique_action_commitment(&self, receipt: &AvcTrustReceipt) -> Result<(), AvcError> {
        let Some(action_commitment_hash) = receipt.action_commitment_hash else {
            return Ok(());
        };
        if let Some(existing) = self
            .get_receipt_by_action_commitment_excluding(&action_commitment_hash, receipt.receipt_id)
        {
            return Err(AvcError::Registry {
                reason: format!(
                    "duplicate AVC receipt action commitment {action_commitment_hash} claimed by receipts {} and {}",
                    existing.receipt_id, receipt.receipt_id
                ),
            });
        }
        Ok(())
    }

    fn validate_durable_receipt_evidence(
        &self,
        stored_chain_head: Option<Hash256>,
    ) -> Result<(), AvcError> {
        let mut commitments = BTreeMap::new();
        let mut extended = BTreeMap::new();
        for (receipt_id, receipt) in &self.receipts {
            if let Some(action_commitment_hash) = receipt.action_commitment_hash {
                if let Some(previous_receipt_id) =
                    commitments.insert(action_commitment_hash, *receipt_id)
                {
                    return Err(AvcError::Registry {
                        reason: format!(
                            "duplicate durable AVC receipt action commitment {action_commitment_hash} claimed by receipts {previous_receipt_id} and {receipt_id}"
                        ),
                    });
                }
            }
            if receipt.has_extended_evidence() {
                extended.insert(*receipt_id, receipt);
            }
        }

        if extended.is_empty() {
            if let Some(head) = stored_chain_head {
                return Err(AvcError::Registry {
                    reason: format!(
                        "durable receipt chain head {head} is set but no extended receipts are stored"
                    ),
                });
            }
            return Ok(());
        }

        let mut child_by_previous: BTreeMap<Option<Hash256>, Hash256> = BTreeMap::new();
        for (receipt_id, receipt) in &extended {
            if let Some(previous) = receipt.previous_receipt_hash {
                if !extended.contains_key(&previous) {
                    return Err(AvcError::Registry {
                        reason: format!(
                            "durable receipt {} references missing previous extended receipt {previous}",
                            receipt.receipt_id
                        ),
                    });
                }
            }
            if let Some(existing_child) =
                child_by_previous.insert(receipt.previous_receipt_hash, *receipt_id)
            {
                return Err(AvcError::Registry {
                    reason: format!(
                        "durable receipt chain branches after previous head {:?}: receipts {existing_child} and {receipt_id}",
                        receipt.previous_receipt_hash
                    ),
                });
            }
        }

        let Some(mut current) = child_by_previous.get(&None).copied() else {
            return Err(AvcError::Registry {
                reason: "durable receipt chain has no genesis receipt".into(),
            });
        };
        let mut visited = BTreeSet::new();
        let terminal = loop {
            if !visited.insert(current) {
                return Err(AvcError::Registry {
                    reason: format!("durable receipt chain contains a cycle at receipt {current}"),
                });
            }
            let Some(next) = child_by_previous.get(&Some(current)).copied() else {
                break current;
            };
            current = next;
        };

        if visited.len() != extended.len() {
            return Err(AvcError::Registry {
                reason: format!(
                    "durable receipt chain is disconnected: visited {} of {} extended receipts",
                    visited.len(),
                    extended.len()
                ),
            });
        }
        if stored_chain_head != Some(terminal) {
            return Err(AvcError::Registry {
                reason: format!(
                    "durable receipt chain head {:?} does not match computed terminal receipt {terminal}",
                    stored_chain_head
                ),
            });
        }
        Ok(())
    }

    fn validate_receipt_chain_link(&self, receipt: &AvcTrustReceipt) -> Result<(), AvcError> {
        if !receipt.has_extended_evidence() {
            return Ok(());
        }
        if receipt.previous_receipt_hash != self.receipt_chain_head {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "receipt {} previous_receipt_hash {:?} does not match current AVC receipt chain head {:?}",
                    receipt.receipt_id, receipt.previous_receipt_hash, self.receipt_chain_head
                ),
            });
        }
        Ok(())
    }

    fn validate_receipt_structural(&self, receipt: &AvcTrustReceipt) -> Result<(), AvcError> {
        if !receipt.verify_id()? {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "receipt {} for credential {} has an invalid content id",
                    receipt.receipt_id, receipt.credential_id
                ),
            });
        }
        if receipt.signature.is_empty() {
            return Err(AvcError::InvalidInput {
                reason: format!("receipt {} has an empty signature", receipt.receipt_id),
            });
        }
        if !self.credentials.contains_key(&receipt.credential_id) {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "receipt {} references unknown credential {}",
                    receipt.receipt_id, receipt.credential_id
                ),
            });
        }
        Ok(())
    }

    fn validate_receipt(&self, receipt: &AvcTrustReceipt) -> Result<(), AvcError> {
        self.validate_receipt_structural(receipt)?;
        let public_key = self
            .receipt_validator_public_keys
            .get(&receipt.validator_did)
            .ok_or_else(|| AvcError::InvalidInput {
                reason: format!(
                    "receipt validator public key for {} is unresolved",
                    receipt.validator_did
                ),
            })?;
        let payload = receipt.signing_payload()?;
        if !crypto::verify(&payload, &receipt.signature, public_key) {
            return Err(AvcError::InvalidInput {
                reason: format!("receipt signature for {} is invalid", receipt.receipt_id),
            });
        }
        Ok(())
    }

    fn validate_revocation(&self, revocation: &AvcRevocation) -> Result<(), AvcError> {
        if revocation.schema_version != crate::credential::AVC_SCHEMA_VERSION {
            return Err(AvcError::UnsupportedSchema {
                got: revocation.schema_version,
                supported: crate::credential::AVC_SCHEMA_VERSION,
            });
        }
        if revocation.signature.is_empty() {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "revocation signature for credential {} must not be empty",
                    revocation.credential_id
                ),
            });
        }

        let credential = self
            .credentials
            .get(&revocation.credential_id)
            .ok_or_else(|| AvcError::InvalidInput {
                reason: format!(
                    "revocation references unknown credential {}",
                    revocation.credential_id
                ),
            })?;
        if revocation.revoker_did != credential.issuer_did
            && revocation.revoker_did != credential.principal_did
        {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "revoker {} is not authorized to revoke credential {}",
                    revocation.revoker_did, revocation.credential_id
                ),
            });
        }

        let public_key = self
            .public_keys
            .get(&revocation.revoker_did)
            .ok_or_else(|| AvcError::InvalidInput {
                reason: format!(
                    "revocation public key for {} is unresolved",
                    revocation.revoker_did
                ),
            })?;
        let payload = revocation.signing_payload()?;
        if !crypto::verify(&payload, &revocation.signature, public_key) {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "revocation signature for credential {} is invalid",
                    revocation.credential_id
                ),
            });
        }

        Ok(())
    }

    fn validate_issuer_permission_grant(
        &self,
        credential: &AutonomousVolitionCredential,
    ) -> Result<(), AvcError> {
        let Some(granted_permissions) = self.issuer_permission_grants.get(&credential.issuer_did)
        else {
            return Ok(());
        };
        for permission in &credential.authority_scope.permissions {
            if !granted_permissions.contains(permission) {
                return Err(AvcError::InvalidInput {
                    reason: format!(
                        "credential issuer {} declares permission {permission:?} outside issuer permission grant",
                        credential.issuer_did
                    ),
                });
            }
        }
        Ok(())
    }

    fn validate_credential(
        &self,
        credential: &AutonomousVolitionCredential,
    ) -> Result<(), AvcError> {
        if credential.signature.is_empty() {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "credential signature for issuer {} must not be empty",
                    credential.issuer_did
                ),
            });
        }

        let public_key = self
            .public_keys
            .get(&credential.issuer_did)
            .ok_or_else(|| AvcError::InvalidInput {
                reason: format!(
                    "credential issuer key for {} is unresolved",
                    credential.issuer_did
                ),
            })?;
        let payload = credential.signing_payload()?;
        if !crypto::verify(&payload, &credential.signature, public_key) {
            return Err(AvcError::InvalidInput {
                reason: format!(
                    "credential signature for issuer {} is invalid",
                    credential.issuer_did
                ),
            });
        }
        self.validate_issuer_permission_grant(credential)?;

        Ok(())
    }

    /// Get the receipt with the given hash, if present.
    #[must_use]
    pub fn get_receipt(&self, receipt_hash: &Hash256) -> Option<AvcTrustReceipt> {
        self.receipts.get(receipt_hash).cloned()
    }

    #[must_use]
    pub fn get_receipt_by_action_commitment(
        &self,
        action_commitment_hash: &Hash256,
    ) -> Option<AvcTrustReceipt> {
        self.get_receipt_by_action_commitment_excluding(action_commitment_hash, Hash256::ZERO)
    }

    fn get_receipt_by_action_commitment_excluding(
        &self,
        action_commitment_hash: &Hash256,
        excluded_receipt_id: Hash256,
    ) -> Option<AvcTrustReceipt> {
        self.receipts
            .values()
            .filter(|receipt| receipt.receipt_id != excluded_receipt_id)
            .find(|receipt| receipt.action_commitment_hash.as_ref() == Some(action_commitment_hash))
            .cloned()
    }

    /// List receipts whose referenced credential belongs to `subject_did`.
    ///
    /// Receipts are stored in a `BTreeMap`, so iteration order is the
    /// canonical receipt hash order. Missing or deleted credentials are not
    /// surfaced as actor-owned receipts because the subject cannot be proven.
    #[must_use]
    pub fn list_receipts_for_subject(
        &self,
        subject_did: &Did,
        limit: usize,
    ) -> Vec<AvcTrustReceipt> {
        self.receipts
            .values()
            .filter(|receipt| {
                self.credentials
                    .get(&receipt.credential_id)
                    .is_some_and(|credential| credential.subject_did == *subject_did)
            })
            .take(limit)
            .cloned()
            .collect()
    }
}

impl AvcRegistryRead for InMemoryAvcRegistry {
    fn resolve_public_key(&self, did: &Did) -> Option<PublicKey> {
        self.public_keys.get(did).copied()
    }

    fn resolve_issuer_permission_grant(&self, did: &Did) -> Option<Vec<Permission>> {
        self.issuer_permission_grants
            .get(did)
            .map(|permissions| permissions.iter().copied().collect())
    }

    fn resolve_human_approval_key(&self, did: &Did) -> Option<PublicKey> {
        self.human_approval_keys.get(did).copied()
    }

    fn is_revoked(&self, credential_id: &Hash256) -> bool {
        self.revocations.contains_key(credential_id)
    }

    fn get_revocation(&self, credential_id: &Hash256) -> Option<AvcRevocation> {
        self.revocations.get(credential_id).cloned()
    }

    fn consent_ref_exists(&self, consent_id: &Hash256) -> bool {
        self.consent_refs.contains(consent_id)
    }

    fn policy_ref_exists(&self, policy_id: &Hash256, policy_version: u16) -> bool {
        self.policy_refs.contains(&(*policy_id, policy_version))
    }

    fn authority_chain_valid(&self, chain_hash: &Hash256, _now: &Timestamp) -> bool {
        self.authority_chains.contains(chain_hash)
    }

    fn get_credential(&self, credential_id: &Hash256) -> Option<AutonomousVolitionCredential> {
        self.credentials.get(credential_id).cloned()
    }

    fn list_credentials_for_subject(&self, subject_did: &Did) -> Vec<AutonomousVolitionCredential> {
        let Some(ids) = self.by_subject.get(subject_did) else {
            return Vec::new();
        };
        ids.iter()
            .filter_map(|id| self.credentials.get(id).cloned())
            .collect()
    }
}

impl AvcRegistryWrite for InMemoryAvcRegistry {
    fn put_credential(
        &mut self,
        credential: AutonomousVolitionCredential,
    ) -> Result<Hash256, AvcError> {
        self.validate_credential(&credential)?;
        let id = credential.id()?;
        self.by_subject
            .entry(credential.subject_did.clone())
            .or_default()
            .insert(id);
        self.credentials.insert(id, credential);
        Ok(id)
    }

    fn put_revocation(&mut self, revocation: AvcRevocation) -> Result<(), AvcError> {
        let id = revocation.credential_id;
        if self.revocations.contains_key(&id) {
            return Err(AvcError::Registry {
                reason: format!("duplicate revocation for credential {id}"),
            });
        }
        self.validate_revocation(&revocation)?;
        self.revocations.insert(id, revocation);
        Ok(())
    }

    fn put_receipt(&mut self, receipt: AvcTrustReceipt) -> Result<(), AvcError> {
        let key = receipt.receipt_id;
        if self.receipts.contains_key(&key) {
            return Err(AvcError::Registry {
                reason: format!("duplicate receipt {key}"),
            });
        }
        self.validate_receipt(&receipt)?;
        self.validate_unique_action_commitment(&receipt)?;
        self.validate_receipt_chain_link(&receipt)?;
        let advances_chain = receipt.has_extended_evidence();
        self.receipts.insert(key, receipt);
        if advances_chain {
            self.receipt_chain_head = Some(key);
        }
        Ok(())
    }

    fn put_public_key(&mut self, did: Did, public_key: PublicKey) {
        self.public_keys.insert(did, public_key);
    }

    fn put_receipt_validator_public_key(&mut self, did: Did, public_key: PublicKey) {
        self.receipt_validator_public_keys.insert(did, public_key);
    }

    fn put_issuer_permission_grant(&mut self, did: Did, granted_permissions: Vec<Permission>) {
        self.issuer_permission_grants
            .insert(did, granted_permissions.into_iter().collect());
    }

    fn put_human_approval_key(&mut self, did: Did, public_key: PublicKey) {
        self.human_approval_keys.insert(did, public_key);
    }

    fn add_consent_ref(&mut self, consent_id: Hash256) {
        self.consent_refs.insert(consent_id);
    }

    fn add_policy_ref(&mut self, policy_id: Hash256, policy_version: u16) {
        self.policy_refs.insert((policy_id, policy_version));
    }

    fn mark_authority_chain_valid(&mut self, chain_hash: Hash256) {
        self.authority_chains.insert(chain_hash);
    }

    fn revoke_authority_chain(&mut self, chain_hash: &Hash256) {
        self.authority_chains.remove(chain_hash);
    }
}

#[cfg(test)]
mod tests {
    use exo_authority::permission::Permission;
    use exo_core::{Signature, crypto::KeyPair};

    use super::*;
    use crate::{
        credential::{
            issue_avc,
            test_support::{baseline_draft, did, h256, ts},
        },
        revocation::{AvcRevocation, AvcRevocationReason, revoke_avc},
    };

    fn keypair(seed: u8) -> KeyPair {
        KeyPair::from_secret_bytes([seed; 32]).unwrap()
    }

    fn signed_revocation(
        id: Hash256,
        revoker_did: Did,
        revoker_keypair: &KeyPair,
    ) -> AvcRevocation {
        revoke_avc(
            id,
            revoker_did,
            AvcRevocationReason::IssuerRevoked,
            ts(2),
            |bytes| revoker_keypair.sign(bytes),
        )
        .unwrap()
    }

    fn fresh_registry() -> InMemoryAvcRegistry {
        InMemoryAvcRegistry::new()
    }

    fn sample_credential() -> AutonomousVolitionCredential {
        let issuer = keypair(0x11);
        issue_avc(baseline_draft(), |bytes| issuer.sign(bytes)).unwrap()
    }

    fn put_issuer_key(reg: &mut InMemoryAvcRegistry) -> KeyPair {
        let issuer = keypair(0x11);
        reg.put_public_key(did("issuer"), issuer.public);
        issuer
    }

    fn put_validator_key(reg: &mut InMemoryAvcRegistry) -> KeyPair {
        let validator = keypair(0x33);
        reg.put_receipt_validator_public_key(did("validator"), validator.public);
        validator
    }

    fn register_sample_credential_and_issuer_key(
        reg: &mut InMemoryAvcRegistry,
    ) -> (Hash256, KeyPair) {
        let cred = sample_credential();
        let id = cred.id().unwrap();
        let issuer_keypair = put_issuer_key(reg);
        reg.put_credential(cred).unwrap();
        (id, issuer_keypair)
    }

    fn sample_issuer_revocation(id: Hash256, issuer_keypair: &KeyPair) -> AvcRevocation {
        signed_revocation(id, did("issuer"), issuer_keypair)
    }

    fn receipt_for_credential(
        credential_id: Hash256,
        validator_keypair: &KeyPair,
    ) -> AvcTrustReceipt {
        let mut receipt = AvcTrustReceipt {
            schema_version: crate::credential::AVC_SCHEMA_VERSION,
            receipt_id: Hash256::ZERO,
            credential_id,
            action_id: None,
            action_commitment_hash: None,
            action_descriptor: None,
            action_descriptor_hash: None,
            previous_receipt_hash: None,
            timestamp_provenance: None,
            external_timestamp_proof: None,
            validator_did: did("validator"),
            decision: crate::validation::AvcDecision::Allow,
            reason_codes: vec![crate::validation::AvcReasonCode::Valid],
            created_at: ts(3),
            validation_hash: h256(0xBB),
            signature: Signature::empty(),
        };
        let payload = receipt.signing_payload().unwrap();
        receipt.receipt_id = Hash256::digest(&payload);
        receipt.signature = validator_keypair.sign(&payload);
        receipt
    }

    fn chained_receipt_for_credential(
        credential_id: Hash256,
        validator_keypair: &KeyPair,
        previous_receipt_hash: Option<Hash256>,
        action_byte: u8,
    ) -> AvcTrustReceipt {
        let mut receipt = AvcTrustReceipt {
            schema_version: crate::credential::AVC_SCHEMA_VERSION,
            receipt_id: Hash256::ZERO,
            credential_id,
            action_id: Some(h256(action_byte)),
            action_commitment_hash: Some(h256(action_byte.wrapping_add(1))),
            action_descriptor: None,
            action_descriptor_hash: None,
            previous_receipt_hash,
            timestamp_provenance: Some(
                crate::receipt::AvcReceiptTimestampProvenance::LocalHybridLogicalClock,
            ),
            external_timestamp_proof: None,
            validator_did: did("validator"),
            decision: crate::validation::AvcDecision::Allow,
            reason_codes: vec![crate::validation::AvcReasonCode::Valid],
            created_at: ts(u64::from(action_byte) + 3),
            validation_hash: h256(0xBB),
            signature: Signature::empty(),
        };
        let payload = receipt.signing_payload().unwrap();
        receipt.receipt_id = Hash256::digest(&payload);
        receipt.signature = validator_keypair.sign(&payload);
        receipt
    }

    fn resign_receipt(receipt: &mut AvcTrustReceipt, validator_keypair: &KeyPair) {
        receipt.receipt_id = Hash256::ZERO;
        receipt.signature = Signature::empty();
        let payload = receipt.signing_payload().unwrap();
        receipt.receipt_id = Hash256::digest(&payload);
        receipt.signature = validator_keypair.sign(&payload);
    }

    #[test]
    fn put_get_credential_round_trips() {
        let mut reg = fresh_registry();
        put_issuer_key(&mut reg);
        let cred = sample_credential();
        let id = reg.put_credential(cred.clone()).unwrap();
        assert_eq!(reg.get_credential(&id).unwrap(), cred);
        assert_eq!(reg.credential_count(), 1);
    }

    #[test]
    fn resolve_issuer_permission_grant_returns_none_for_unregistered_issuer() {
        let reg = fresh_registry();

        assert_eq!(reg.resolve_issuer_permission_grant(&did("issuer")), None);
    }

    #[test]
    fn put_issuer_permission_grant_deduplicates_and_sorts_permissions() {
        let mut reg = fresh_registry();

        reg.put_issuer_permission_grant(
            did("issuer"),
            vec![Permission::Write, Permission::Read, Permission::Write],
        );

        assert_eq!(
            reg.resolve_issuer_permission_grant(&did("issuer")),
            Some(vec![Permission::Read, Permission::Write])
        );
    }

    #[test]
    fn put_credential_accepts_without_registered_issuer_grant() {
        let mut reg = fresh_registry();
        put_issuer_key(&mut reg);
        let cred = sample_credential();
        let id = cred.id().unwrap();

        let stored_id = reg
            .put_credential(cred)
            .expect("credential must store when no issuer grant is registered");

        assert_eq!(stored_id, id);
        assert_eq!(reg.resolve_issuer_permission_grant(&did("issuer")), None);
        assert_eq!(reg.credential_count(), 1);
    }

    #[test]
    fn put_credential_accepts_scope_within_registered_issuer_grant() {
        let mut reg = fresh_registry();
        let issuer = put_issuer_key(&mut reg);
        reg.put_issuer_permission_grant(did("issuer"), vec![Permission::Read]);
        let mut draft = baseline_draft();
        draft.authority_scope.permissions = vec![Permission::Read];
        let cred = issue_avc(draft, |bytes| issuer.sign(bytes)).unwrap();
        let id = cred.id().unwrap();

        let stored_id = reg
            .put_credential(cred)
            .expect("credential inside root issuer grant must store");

        assert_eq!(stored_id, id);
        assert_eq!(
            reg.resolve_issuer_permission_grant(&did("issuer")),
            Some(vec![Permission::Read])
        );
        assert_eq!(reg.credential_count(), 1);
    }

    #[test]
    fn put_credential_rejects_scope_wider_than_registered_issuer_grant() {
        let mut reg = fresh_registry();
        let issuer = put_issuer_key(&mut reg);
        reg.put_issuer_permission_grant(
            did("issuer"),
            vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::Delegate,
            ],
        );
        let mut draft = baseline_draft();
        draft.authority_scope.permissions = vec![Permission::Govern];
        let cred = issue_avc(draft, |bytes| issuer.sign(bytes)).unwrap();
        let id = cred.id().unwrap();

        let error = reg
            .put_credential(cred)
            .expect_err("credential widening beyond root issuer grant must fail closed");

        assert!(
            error.to_string().contains("issuer permission grant"),
            "error must identify issuer grant boundary: {error}"
        );
        assert_eq!(reg.credential_count(), 0);
        assert!(reg.get_credential(&id).is_none());
    }

    #[test]
    fn put_credential_rejects_empty_signature_without_storing() {
        let mut reg = fresh_registry();
        put_issuer_key(&mut reg);
        let mut cred = sample_credential();
        let id = cred.id().unwrap();
        cred.signature = Signature::empty();

        let err = reg.put_credential(cred).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => assert!(reason.contains("signature")),
            other => panic!("expected invalid input for unsigned credential, got {other:?}"),
        }
        assert_eq!(reg.credential_count(), 0);
        assert!(reg.get_credential(&id).is_none());
    }

    #[test]
    fn put_credential_rejects_unresolved_issuer_key_without_storing() {
        let mut reg = fresh_registry();
        let cred = sample_credential();
        let id = cred.id().unwrap();

        let err = reg.put_credential(cred).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("issuer"));
                assert!(reason.contains("unresolved"));
            }
            other => panic!("expected invalid input for unresolved issuer, got {other:?}"),
        }
        assert_eq!(reg.credential_count(), 0);
        assert!(reg.get_credential(&id).is_none());
    }

    #[test]
    fn put_credential_rejects_wrong_signature_key_without_storing() {
        let mut reg = fresh_registry();
        put_issuer_key(&mut reg);
        let attacker = keypair(0x22);
        let mut cred = sample_credential();
        let payload = cred.signing_payload().unwrap();
        cred.signature = attacker.sign(&payload);
        let id = cred.id().unwrap();

        let err = reg.put_credential(cred).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("signature"));
                assert!(reason.contains("invalid"));
            }
            other => panic!("expected invalid input for wrong signer, got {other:?}"),
        }
        assert_eq!(reg.credential_count(), 0);
        assert!(reg.get_credential(&id).is_none());
    }

    #[test]
    fn list_credentials_for_subject_returns_subject_only() {
        let mut reg = fresh_registry();
        put_issuer_key(&mut reg);
        let cred1 = sample_credential();
        reg.put_credential(cred1.clone()).unwrap();
        // Add an unrelated subject
        let mut draft2 = baseline_draft();
        draft2.subject_did = did("agent-other");
        let issuer = keypair(0x11);
        let cred2 = issue_avc(draft2, |bytes| issuer.sign(bytes)).unwrap();
        reg.put_credential(cred2).unwrap();

        let listed = reg.list_credentials_for_subject(&cred1.subject_did);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], cred1);

        let none = reg.list_credentials_for_subject(&did("nobody"));
        assert!(none.is_empty());
    }

    #[test]
    fn put_revocation_rejects_duplicates() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let revocation = sample_issuer_revocation(id, &issuer_keypair);
        reg.put_revocation(revocation.clone()).unwrap();
        let err = reg.put_revocation(revocation).unwrap_err();
        assert!(matches!(err, AvcError::Registry { .. }));
    }

    #[test]
    fn put_revocation_rejects_empty_signature_without_marking_revoked() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);

        let mut revocation = sample_issuer_revocation(id, &issuer_keypair);
        revocation.signature = Signature::empty();

        let err = reg.put_revocation(revocation).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("revocation signature"));
            }
            other => panic!("expected invalid input for unsigned revocation, got {other:?}"),
        }
        assert!(
            !reg.is_revoked(&id),
            "unsigned revocation must not create a tombstone"
        );
    }

    #[test]
    fn put_revocation_rejects_unsupported_schema_without_marking_revoked() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);

        let mut revocation = sample_issuer_revocation(id, &issuer_keypair);
        revocation.schema_version = crate::credential::AVC_SCHEMA_VERSION + 1;

        let err = reg.put_revocation(revocation).unwrap_err();
        match err {
            AvcError::UnsupportedSchema { got, supported } => {
                assert_eq!(got, crate::credential::AVC_SCHEMA_VERSION + 1);
                assert_eq!(supported, crate::credential::AVC_SCHEMA_VERSION);
            }
            other => panic!("expected unsupported schema for revocation, got {other:?}"),
        }
        assert!(
            !reg.is_revoked(&id),
            "unsupported revocation schema must not create a tombstone"
        );
    }

    #[test]
    fn put_revocation_accepts_principal_revoker() {
        let mut reg = fresh_registry();
        let mut draft = baseline_draft();
        draft.principal_did = did("principal");
        let issuer_keypair = keypair(0x11);
        let principal_keypair = keypair(0x22);
        let credential = issue_avc(draft, |bytes| issuer_keypair.sign(bytes)).unwrap();
        let id = credential.id().unwrap();
        reg.put_public_key(did("issuer"), issuer_keypair.public);
        reg.put_public_key(did("principal"), principal_keypair.public);
        reg.put_credential(credential).unwrap();
        let revocation = signed_revocation(id, did("principal"), &principal_keypair);

        reg.put_revocation(revocation.clone()).unwrap();

        assert!(reg.is_revoked(&id));
        assert_eq!(reg.get_revocation(&id).unwrap(), revocation);
    }

    #[test]
    fn put_revocation_rejects_revoker_that_is_not_issuer_or_principal() {
        let mut reg = fresh_registry();
        let cred = sample_credential();
        let id = cred.id().unwrap();
        let attacker = did("attacker");
        let attacker_keypair = keypair(0x22);
        put_issuer_key(&mut reg);
        reg.put_credential(cred).unwrap();
        reg.put_public_key(attacker.clone(), attacker_keypair.public);

        let revocation = signed_revocation(id, attacker, &attacker_keypair);

        let err = reg.put_revocation(revocation).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("not authorized"));
            }
            other => panic!("expected invalid input for unauthorized revoker, got {other:?}"),
        }
        assert!(
            !reg.is_revoked(&id),
            "unauthorized revoker must not create a tombstone"
        );
    }

    #[test]
    fn put_revocation_rejects_wrong_signature_key_without_marking_revoked() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let attacker_keypair = keypair(0x22);
        let revocation = signed_revocation(id, did("issuer"), &attacker_keypair);

        let err = reg.put_revocation(revocation).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("signature"));
                assert!(reason.contains("invalid"));
            }
            other => panic!("expected invalid input for wrong signing key, got {other:?}"),
        }
        assert!(
            !reg.is_revoked(&id),
            "wrong signing key must not create a tombstone"
        );
    }

    #[test]
    fn put_revocation_rejects_unresolved_revoker_key_without_marking_revoked() {
        let mut reg = fresh_registry();
        let mut draft = baseline_draft();
        draft.principal_did = did("principal");
        let issuer_keypair = keypair(0x11);
        let cred = issue_avc(draft, |bytes| issuer_keypair.sign(bytes)).unwrap();
        let id = cred.id().unwrap();
        reg.put_public_key(did("issuer"), issuer_keypair.public);
        reg.put_credential(cred).unwrap();
        let principal_keypair = keypair(0x22);
        let revocation = signed_revocation(id, did("principal"), &principal_keypair);

        let err = reg.put_revocation(revocation).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("public key"));
                assert!(reason.contains("unresolved"));
            }
            other => panic!("expected invalid input for unresolved revoker key, got {other:?}"),
        }
        assert!(
            !reg.is_revoked(&id),
            "unresolved revoker key must not create a tombstone"
        );
    }

    #[test]
    fn put_revocation_rejects_unknown_credential_without_marking_revoked() {
        let mut reg = fresh_registry();
        let id = h256(0x42);
        let issuer_keypair = keypair(0x11);
        reg.put_public_key(did("issuer"), issuer_keypair.public);
        let revocation = sample_issuer_revocation(id, &issuer_keypair);

        let err = reg.put_revocation(revocation).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("unknown credential"));
            }
            other => panic!("expected invalid input for unknown credential, got {other:?}"),
        }
        assert!(
            !reg.is_revoked(&id),
            "unknown credential must not create a tombstone"
        );
    }

    #[test]
    fn revoked_state_visible_via_is_revoked_and_get() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        assert!(!reg.is_revoked(&id));
        let revocation = sample_issuer_revocation(id, &issuer_keypair);
        reg.put_revocation(revocation.clone()).unwrap();
        assert!(reg.is_revoked(&id));
        assert_eq!(reg.get_revocation(&id).unwrap(), revocation);
        assert_eq!(reg.revocation_count(), 1);
    }

    #[test]
    fn put_receipt_rejects_duplicates() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let receipt = receipt_for_credential(id, &validator_keypair);
        reg.put_receipt(receipt.clone()).unwrap();
        let err = reg.put_receipt(receipt.clone()).unwrap_err();
        assert!(matches!(err, AvcError::Registry { .. }));
        assert_eq!(reg.receipt_count(), 1);
        assert_eq!(reg.get_receipt(&receipt.receipt_id).unwrap(), receipt);
    }

    #[test]
    fn put_receipt_accepts_validator_signed_receipt_with_registered_key() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let receipt = receipt_for_credential(id, &validator_keypair);

        reg.put_receipt(receipt.clone()).unwrap();

        assert_eq!(reg.receipt_count(), 1);
        assert_eq!(reg.get_receipt(&receipt.receipt_id).unwrap(), receipt);
    }

    #[test]
    fn put_receipt_accepts_scoped_validator_key_without_generic_issuer_trust() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        reg.put_receipt_validator_public_key(did("validator"), validator_keypair.public);
        let receipt = receipt_for_credential(id, &validator_keypair);

        reg.put_receipt(receipt.clone()).unwrap();

        assert_eq!(reg.receipt_count(), 1);
        assert_eq!(reg.get_receipt(&receipt.receipt_id).unwrap(), receipt);
        assert_eq!(reg.resolve_public_key(&did("validator")), None);
    }

    #[test]
    fn generic_public_key_does_not_validate_receipt_signature() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        reg.put_public_key(did("validator"), validator_keypair.public);
        let receipt = receipt_for_credential(id, &validator_keypair);

        let err = reg.put_receipt(receipt.clone()).unwrap_err();

        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("receipt validator public key") && reason.contains("unresolved"))
        );
        assert_eq!(
            reg.resolve_public_key(&did("validator")),
            Some(validator_keypair.public)
        );
        assert_eq!(reg.receipt_count(), 0);
        assert!(reg.get_receipt(&receipt.receipt_id).is_none());
    }

    #[test]
    fn receipt_validator_key_does_not_grant_credential_issuer_trust() {
        let mut reg = fresh_registry();
        let issuer_keypair = keypair(0x11);
        reg.put_receipt_validator_public_key(did("issuer"), issuer_keypair.public);
        let credential = sample_credential();

        let err = reg.put_credential(credential).unwrap_err();

        assert_eq!(reg.resolve_public_key(&did("issuer")), None);
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("credential issuer key") && reason.contains("unresolved"))
        );
        assert_eq!(reg.credential_count(), 0);
    }

    #[test]
    fn receipt_chain_advances_for_extended_receipts() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);

        reg.put_receipt(first.clone()).unwrap();
        assert_eq!(reg.receipt_chain_head(), Some(first.receipt_id));

        reg.put_receipt(second.clone()).unwrap();
        assert_eq!(reg.receipt_chain_head(), Some(second.receipt_id));
        assert_eq!(reg.receipt_count(), 2);
        assert_eq!(reg.get_receipt(&second.receipt_id).unwrap(), second);
    }

    #[test]
    fn receipt_chain_rejects_wrong_previous_hash_without_advancing() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        reg.put_receipt(first.clone()).unwrap();
        let wrong_link =
            chained_receipt_for_credential(id, &validator_keypair, Some(h256(0xFE)), 0x22);

        let err = reg.put_receipt(wrong_link.clone()).unwrap_err();

        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("previous_receipt_hash"))
        );
        assert_eq!(reg.receipt_chain_head(), Some(first.receipt_id));
        assert_eq!(reg.receipt_count(), 1);
        assert!(reg.get_receipt(&wrong_link.receipt_id).is_none());
    }

    #[test]
    fn put_receipt_rejects_duplicate_action_commitment_without_advancing() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let mut duplicate_commitment =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        duplicate_commitment.action_commitment_hash = first.action_commitment_hash;
        resign_receipt(&mut duplicate_commitment, &validator_keypair);

        reg.put_receipt(first.clone()).unwrap();
        let err = reg.put_receipt(duplicate_commitment.clone()).unwrap_err();

        assert!(
            matches!(err, AvcError::Registry { reason } if reason.contains("duplicate AVC receipt action commitment"))
        );
        assert_eq!(reg.receipt_chain_head(), Some(first.receipt_id));
        assert_eq!(reg.receipt_count(), 1);
        assert!(reg.get_receipt(&duplicate_commitment.receipt_id).is_none());
    }

    #[test]
    fn get_receipt_by_action_commitment_finds_extended_receipts() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let legacy = receipt_for_credential(id, &validator_keypair);
        let extended = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let action_commitment_hash = extended.action_commitment_hash.unwrap();
        reg.put_receipt(legacy).unwrap();
        reg.put_receipt(extended.clone()).unwrap();

        assert_eq!(
            reg.get_receipt_by_action_commitment(&action_commitment_hash),
            Some(extended)
        );
        assert!(reg.get_receipt_by_action_commitment(&h256(0xFE)).is_none());
    }

    #[test]
    fn legacy_receipts_do_not_advance_receipt_chain_head() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let receipt = receipt_for_credential(id, &validator_keypair);

        reg.put_receipt(receipt.clone()).unwrap();

        assert_eq!(reg.receipt_count(), 1);
        assert_eq!(reg.receipt_chain_head(), None);
        assert_eq!(reg.get_receipt(&receipt.receipt_id).unwrap(), receipt);
    }

    #[test]
    fn put_receipt_rejects_forged_signature_without_storing() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let attacker_keypair = keypair(0x44);
        let receipt = receipt_for_credential(id, &attacker_keypair);

        let err = reg.put_receipt(receipt.clone()).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("signature"));
                assert!(reason.contains("invalid"));
            }
            other => panic!("expected invalid input for forged receipt, got {other:?}"),
        }
        assert_eq!(reg.receipt_count(), 0);
        assert!(reg.get_receipt(&receipt.receipt_id).is_none());
        assert_ne!(validator_keypair.public, attacker_keypair.public);
    }

    #[test]
    fn put_receipt_rejects_unresolved_validator_key_without_storing() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let receipt = receipt_for_credential(id, &validator_keypair);

        let err = reg.put_receipt(receipt.clone()).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("validator public key"));
                assert!(reason.contains("unresolved"));
            }
            other => panic!("expected invalid input for unresolved validator, got {other:?}"),
        }
        assert_eq!(reg.receipt_count(), 0);
        assert!(reg.get_receipt(&receipt.receipt_id).is_none());
    }

    #[test]
    fn put_receipt_rejects_unknown_credential_without_storing() {
        let mut reg = fresh_registry();
        let validator_keypair = put_validator_key(&mut reg);
        let receipt = receipt_for_credential(h256(0xAA), &validator_keypair);
        let err = reg.put_receipt(receipt).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => {
                assert!(reason.contains("unknown credential"));
            }
            other => panic!("expected invalid receipt credential reference, got {other:?}"),
        }
        assert_eq!(reg.receipt_count(), 0);
    }

    #[test]
    fn list_receipts_for_subject_filters_by_credential_subject_and_limit() {
        let mut reg = fresh_registry();
        let issuer_keypair = put_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);

        let mut first_draft = baseline_draft();
        first_draft.subject_did = did("agent-one");
        first_draft.delegated_intent.intent_id = h256(0x31);
        let first = issue_avc(first_draft, |bytes| issuer_keypair.sign(bytes)).unwrap();
        let first_id = reg.put_credential(first).unwrap();

        let mut second_draft = baseline_draft();
        second_draft.subject_did = did("agent-one");
        second_draft.delegated_intent.intent_id = h256(0x32);
        let second = issue_avc(second_draft, |bytes| issuer_keypair.sign(bytes)).unwrap();
        let second_id = reg.put_credential(second).unwrap();

        let mut other_draft = baseline_draft();
        other_draft.subject_did = did("agent-two");
        other_draft.delegated_intent.intent_id = h256(0x33);
        let other = issue_avc(other_draft, |bytes| issuer_keypair.sign(bytes)).unwrap();
        let other_id = reg.put_credential(other).unwrap();

        let first_receipt = receipt_for_credential(first_id, &validator_keypair);
        let second_receipt = receipt_for_credential(second_id, &validator_keypair);
        let other_receipt = receipt_for_credential(other_id, &validator_keypair);
        reg.put_receipt(first_receipt.clone()).unwrap();
        reg.put_receipt(other_receipt).unwrap();
        reg.put_receipt(second_receipt.clone()).unwrap();

        let mut expected = [first_receipt.clone(), second_receipt.clone()];
        expected.sort_by_key(|receipt| receipt.receipt_id);

        let listed = reg.list_receipts_for_subject(&did("agent-one"), 10);
        assert_eq!(listed, expected);

        let limited = reg.list_receipts_for_subject(&did("agent-one"), 1);
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0], expected[0]);

        assert!(reg.list_receipts_for_subject(&did("nobody"), 10).is_empty());
    }

    #[test]
    fn durable_state_round_trips_runtime_records_without_key_trust_anchors() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let revocation = sample_issuer_revocation(id, &issuer_keypair);
        reg.put_revocation(revocation.clone()).unwrap();
        let validator_keypair = put_validator_key(&mut reg);
        let receipt = receipt_for_credential(id, &validator_keypair);
        reg.put_receipt(receipt.clone()).unwrap();

        let restored = InMemoryAvcRegistry::from_durable_state(reg.durable_state()).unwrap();

        assert_eq!(restored.credential_count(), 1);
        assert_eq!(restored.revocation_count(), 1);
        assert_eq!(restored.receipt_count(), 1);
        assert_eq!(restored.get_revocation(&id).unwrap(), revocation);
        assert_eq!(restored.get_receipt(&receipt.receipt_id).unwrap(), receipt);
        assert_eq!(restored.receipt_chain_head(), None);
        assert!(
            restored.resolve_public_key(&did("issuer")).is_none(),
            "key trust anchors must be reloaded from verified startup config"
        );
        assert!(
            restored.resolve_public_key(&did("validator")).is_none(),
            "validator trust anchors must be reloaded from verified startup config"
        );
    }

    #[test]
    fn durable_state_receipt_chain_head_round_trips() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = put_validator_key(&mut reg);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        reg.put_receipt(first).unwrap();
        reg.put_receipt(second.clone()).unwrap();

        let state = reg.durable_state();
        assert_eq!(state.receipt_chain_head, Some(second.receipt_id));
        let restored = InMemoryAvcRegistry::from_durable_state(state).unwrap();

        assert_eq!(restored.receipt_count(), 2);
        assert_eq!(restored.receipt_chain_head(), Some(second.receipt_id));
        assert_eq!(restored.get_receipt(&second.receipt_id).unwrap(), second);
    }

    #[test]
    fn durable_state_accepts_legacy_only_receipts_with_no_chain_head() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let receipt = receipt_for_credential(id, &validator_keypair);
        let mut state = reg.durable_state();
        state.receipts.insert(receipt.receipt_id, receipt.clone());
        state.receipt_chain_head = None;

        let restored = InMemoryAvcRegistry::from_durable_state(state).unwrap();

        assert_eq!(restored.receipt_count(), 1);
        assert_eq!(restored.receipt_chain_head(), None);
        assert_eq!(restored.get_receipt(&receipt.receipt_id).unwrap(), receipt);
    }

    #[test]
    fn durable_state_rejects_legacy_receipt_chain_head_without_extended_receipts() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let receipt = receipt_for_credential(id, &validator_keypair);
        let mut state = reg.durable_state();
        state.receipts.insert(receipt.receipt_id, receipt.clone());
        state.receipt_chain_head = Some(receipt.receipt_id);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();

        match err {
            AvcError::Registry { reason } => {
                assert!(reason.contains("no extended receipts are stored"));
            }
            other => panic!("expected durable receipt chain head error, got {other:?}"),
        }
    }

    #[test]
    fn durable_state_rejects_receipt_chain_missing_intermediate_prior() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let receipt =
            chained_receipt_for_credential(id, &validator_keypair, Some(h256(0xEE)), 0x21);
        let mut state = reg.durable_state();
        state.receipts.insert(receipt.receipt_id, receipt.clone());
        state.receipt_chain_head = Some(receipt.receipt_id);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();

        match err {
            AvcError::Registry { reason } => {
                assert!(reason.contains("missing previous extended receipt"));
            }
            other => panic!("expected missing previous receipt error, got {other:?}"),
        }
    }

    #[test]
    fn durable_state_rejects_receipt_chain_branch() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        let third =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x23);
        let mut state = reg.durable_state();
        state.receipts.insert(first.receipt_id, first);
        state.receipts.insert(second.receipt_id, second.clone());
        state.receipts.insert(third.receipt_id, third);
        state.receipt_chain_head = Some(second.receipt_id);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();

        assert!(
            matches!(err, AvcError::Registry { reason } if reason.contains("branches after previous head"))
        );
    }

    #[test]
    fn durable_receipt_evidence_rejects_extended_chain_with_no_genesis() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let mut first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        first.previous_receipt_hash = Some(second.receipt_id);
        reg.receipts.insert(first.receipt_id, first);
        reg.receipts.insert(second.receipt_id, second.clone());

        let err = reg
            .validate_durable_receipt_evidence(Some(second.receipt_id))
            .unwrap_err();

        match err {
            AvcError::Registry { reason } => {
                assert!(reason.contains("no genesis receipt"));
            }
            other => panic!("expected durable receipt chain genesis error, got {other:?}"),
        }
    }

    #[test]
    fn durable_receipt_evidence_rejects_disconnected_chain() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        let mut third = chained_receipt_for_credential(id, &validator_keypair, None, 0x23);
        let fourth =
            chained_receipt_for_credential(id, &validator_keypair, Some(third.receipt_id), 0x24);
        third.previous_receipt_hash = Some(fourth.receipt_id);
        reg.receipts.insert(first.receipt_id, first);
        reg.receipts.insert(second.receipt_id, second.clone());
        reg.receipts.insert(third.receipt_id, third);
        reg.receipts.insert(fourth.receipt_id, fourth);

        let err = reg
            .validate_durable_receipt_evidence(Some(second.receipt_id))
            .unwrap_err();

        match err {
            AvcError::Registry { reason } => {
                assert!(reason.contains("chain is disconnected"));
            }
            other => panic!("expected disconnected durable receipt chain error, got {other:?}"),
        }
    }

    #[test]
    fn durable_state_rejects_receipt_chain_head_that_is_not_terminal() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        let mut state = reg.durable_state();
        state.receipts.insert(first.receipt_id, first.clone());
        state.receipts.insert(second.receipt_id, second);
        state.receipt_chain_head = Some(first.receipt_id);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();

        assert!(
            matches!(err, AvcError::Registry { reason } if reason.contains("computed terminal receipt"))
        );
    }

    #[test]
    fn durable_state_rejects_duplicate_action_commitments() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let first = chained_receipt_for_credential(id, &validator_keypair, None, 0x21);
        let mut second =
            chained_receipt_for_credential(id, &validator_keypair, Some(first.receipt_id), 0x22);
        second.action_commitment_hash = first.action_commitment_hash;
        resign_receipt(&mut second, &validator_keypair);
        let mut state = reg.durable_state();
        state.receipts.insert(first.receipt_id, first);
        state.receipts.insert(second.receipt_id, second.clone());
        state.receipt_chain_head = Some(second.receipt_id);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();

        assert!(
            matches!(err, AvcError::Registry { reason } if reason.contains("duplicate durable AVC receipt action commitment"))
        );
    }

    #[test]
    fn durable_state_rejects_mismatched_credential_key() {
        let mut state = AvcRegistryDurableState::default();
        state.credentials.insert(h256(0x99), sample_credential());

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();
        match err {
            AvcError::Registry { reason } => {
                assert!(reason.contains("does not match computed id"));
            }
            other => panic!("expected durable credential key mismatch, got {other:?}"),
        }
    }

    #[test]
    fn durable_state_rejects_invalid_revocation_records() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let valid_revocation = sample_issuer_revocation(id, &issuer_keypair);

        let mut mismatched_key = reg.durable_state();
        mismatched_key
            .revocations
            .insert(h256(0x44), valid_revocation.clone());
        let err = InMemoryAvcRegistry::from_durable_state(mismatched_key).unwrap_err();
        assert!(
            matches!(err, AvcError::Registry { reason } if reason.contains("durable revocation key"))
        );

        let mut unsigned = valid_revocation.clone();
        unsigned.signature = Signature::empty();
        let mut unsigned_state = reg.durable_state();
        unsigned_state.revocations.insert(id, unsigned);
        let err = InMemoryAvcRegistry::from_durable_state(unsigned_state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("empty signature"))
        );

        let unknown_id = h256(0x66);
        let mut unknown_state = AvcRegistryDurableState::default();
        unknown_state.revocations.insert(
            unknown_id,
            signed_revocation(unknown_id, did("issuer"), &issuer_keypair),
        );
        let err = InMemoryAvcRegistry::from_durable_state(unknown_state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("unknown credential"))
        );

        let attacker_keypair = keypair(0x22);
        let mut unauthorized_state = reg.durable_state();
        unauthorized_state.revocations.insert(
            id,
            signed_revocation(id, did("attacker"), &attacker_keypair),
        );
        let err = InMemoryAvcRegistry::from_durable_state(unauthorized_state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("not authorized"))
        );
    }

    #[test]
    fn durable_state_accepts_principal_revocation() {
        let mut reg = fresh_registry();
        let mut draft = baseline_draft();
        draft.principal_did = did("principal");
        let issuer_keypair = keypair(0x11);
        let principal_keypair = keypair(0x22);
        let credential = issue_avc(draft, |bytes| issuer_keypair.sign(bytes)).unwrap();
        let id = credential.id().unwrap();
        reg.put_public_key(did("issuer"), issuer_keypair.public);
        reg.put_credential(credential).unwrap();
        let revocation = signed_revocation(id, did("principal"), &principal_keypair);
        let mut state = reg.durable_state();
        state.revocations.insert(id, revocation.clone());

        let restored = InMemoryAvcRegistry::from_durable_state(state).unwrap();

        assert_eq!(restored.revocation_count(), 1);
        assert_eq!(restored.get_revocation(&id).unwrap(), revocation);
    }

    #[test]
    fn durable_state_rejects_revocation_with_unsupported_schema() {
        let mut reg = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let mut unsupported_schema = sample_issuer_revocation(id, &issuer_keypair);
        unsupported_schema.schema_version = crate::credential::AVC_SCHEMA_VERSION + 1;

        let mut state = reg.durable_state();
        state.revocations.insert(id, unsupported_schema);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();
        assert!(
            matches!(
                err,
                AvcError::UnsupportedSchema { got, supported }
                    if got == crate::credential::AVC_SCHEMA_VERSION + 1
                        && supported == crate::credential::AVC_SCHEMA_VERSION
            ),
            "durable revocation schema must be validated before tombstone import"
        );
    }

    #[test]
    fn durable_state_rejects_invalid_receipt_records() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let receipt = receipt_for_credential(id, &validator_keypair);

        let mut mismatched_key = reg.durable_state();
        mismatched_key.receipts.insert(h256(0x77), receipt.clone());
        let err = InMemoryAvcRegistry::from_durable_state(mismatched_key).unwrap_err();
        assert!(
            matches!(err, AvcError::Registry { reason } if reason.contains("durable receipt key"))
        );

        let mut unsigned = receipt.clone();
        unsigned.signature = Signature::empty();
        let mut unsigned_state = reg.durable_state();
        unsigned_state
            .receipts
            .insert(unsigned.receipt_id, unsigned);
        let err = InMemoryAvcRegistry::from_durable_state(unsigned_state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("empty signature"))
        );

        let unknown_receipt = receipt_for_credential(h256(0xAA), &validator_keypair);
        let mut unknown_state = AvcRegistryDurableState::default();
        unknown_state
            .receipts
            .insert(unknown_receipt.receipt_id, unknown_receipt);
        let err = InMemoryAvcRegistry::from_durable_state(unknown_state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("unknown credential"))
        );
    }

    #[test]
    fn apply_durable_state_preserves_trust_anchors_and_validation_context() {
        let mut durable_source = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut durable_source);
        let revocation = sample_issuer_revocation(id, &issuer_keypair);
        durable_source.put_revocation(revocation.clone()).unwrap();
        let validator_keypair = put_validator_key(&mut durable_source);
        let receipt = receipt_for_credential(id, &validator_keypair);
        durable_source.put_receipt(receipt.clone()).unwrap();

        let subject_keypair = keypair(0x22);
        let human_keypair = keypair(0x33);
        let consent_ref = h256(0xCA);
        let policy_ref = h256(0xCB);
        let authority_chain = h256(0xCC);
        let mut live = fresh_registry();
        live.put_public_key(did("issuer"), issuer_keypair.public);
        live.put_receipt_validator_public_key(did("validator"), validator_keypair.public);
        live.put_public_key(did("subject"), subject_keypair.public);
        live.put_human_approval_key(did("human"), human_keypair.public);
        live.add_consent_ref(consent_ref);
        live.add_policy_ref(policy_ref, 7);
        live.mark_authority_chain_valid(authority_chain);

        live.apply_durable_state(durable_source.durable_state())
            .unwrap();

        assert_eq!(live.credential_count(), 1);
        assert_eq!(live.revocation_count(), 1);
        assert_eq!(live.receipt_count(), 1);
        assert_eq!(live.get_revocation(&id).unwrap(), revocation);
        assert_eq!(live.get_receipt(&receipt.receipt_id).unwrap(), receipt);
        assert_eq!(
            live.resolve_public_key(&did("issuer")).unwrap(),
            issuer_keypair.public
        );
        assert_eq!(
            live.resolve_public_key(&did("subject")).unwrap(),
            subject_keypair.public
        );
        assert_eq!(live.resolve_public_key(&did("validator")), None);
        assert_eq!(
            live.resolve_human_approval_key(&did("human")).unwrap(),
            human_keypair.public
        );
        assert!(live.consent_ref_exists(&consent_ref));
        assert!(live.policy_ref_exists(&policy_ref, 7));
        assert!(live.authority_chain_valid(&authority_chain, &ts(9)));
    }

    #[test]
    fn apply_durable_state_rejects_forged_revocation_signature_with_live_trust_anchor() {
        let mut durable_source = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut durable_source);
        let attacker_keypair = keypair(0x44);
        let forged_revocation = signed_revocation(id, did("issuer"), &attacker_keypair);
        let mut state = durable_source.durable_state();
        state.revocations.insert(id, forged_revocation);

        let mut live = fresh_registry();
        live.put_public_key(did("issuer"), issuer_keypair.public);

        let err = live.apply_durable_state(state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("signature") && reason.contains("invalid")),
            "durable revocation signatures must be verified with live startup trust anchors"
        );
        assert_eq!(live.credential_count(), 0);
        assert!(
            !live.is_revoked(&id),
            "forged durable revocation must not create a tombstone"
        );
    }

    #[test]
    fn apply_durable_state_rejects_forged_receipt_signature_with_live_trust_anchor() {
        let mut durable_source = fresh_registry();
        let (id, issuer_keypair) = register_sample_credential_and_issuer_key(&mut durable_source);
        let validator_keypair = keypair(0x33);
        let attacker_keypair = keypair(0x44);
        let forged_receipt = receipt_for_credential(id, &attacker_keypair);
        let mut state = durable_source.durable_state();
        state
            .receipts
            .insert(forged_receipt.receipt_id, forged_receipt);

        let mut live = fresh_registry();
        live.put_public_key(did("issuer"), issuer_keypair.public);
        live.put_receipt_validator_public_key(did("validator"), validator_keypair.public);

        let err = live.apply_durable_state(state).unwrap_err();
        assert!(
            matches!(err, AvcError::InvalidInput { reason } if reason.contains("signature") && reason.contains("invalid")),
            "durable receipt signatures must be verified with live startup trust anchors"
        );
        assert_eq!(live.credential_count(), 0);
        assert_eq!(live.receipt_count(), 0);
    }

    #[test]
    fn durable_state_rejects_receipt_with_invalid_content_id() {
        let mut reg = fresh_registry();
        let (id, _issuer_keypair) = register_sample_credential_and_issuer_key(&mut reg);
        let validator_keypair = keypair(0x33);
        let mut receipt = receipt_for_credential(id, &validator_keypair);
        let stored_id = receipt.receipt_id;
        receipt.validation_hash = h256(0xCC);

        let mut state = reg.durable_state();
        state.receipts.insert(stored_id, receipt);

        let err = InMemoryAvcRegistry::from_durable_state(state).unwrap_err();
        match err {
            AvcError::InvalidInput { reason } => assert!(reason.contains("invalid content id")),
            other => panic!("expected invalid durable receipt id, got {other:?}"),
        }
    }

    #[test]
    fn public_keys_round_trip() {
        let mut reg = fresh_registry();
        let key = exo_core::PublicKey::from_bytes([3u8; 32]);
        reg.put_public_key(did("issuer"), key);
        assert_eq!(reg.resolve_public_key(&did("issuer")).unwrap(), key);
        assert!(reg.resolve_public_key(&did("nobody")).is_none());
    }

    #[test]
    fn consent_and_policy_ref_existence() {
        let mut reg = fresh_registry();
        reg.add_consent_ref(h256(0xC0));
        reg.add_policy_ref(h256(0xB1), 2);

        assert!(reg.consent_ref_exists(&h256(0xC0)));
        assert!(!reg.consent_ref_exists(&h256(0xC1)));

        assert!(reg.policy_ref_exists(&h256(0xB1), 2));
        assert!(!reg.policy_ref_exists(&h256(0xB1), 1));
        assert!(!reg.policy_ref_exists(&h256(0xB2), 2));
    }

    #[test]
    fn authority_chain_validity_can_be_marked_and_revoked() {
        let mut reg = fresh_registry();
        let chain = h256(0xDE);
        assert!(!reg.authority_chain_valid(&chain, &ts(1)));
        reg.mark_authority_chain_valid(chain);
        assert!(reg.authority_chain_valid(&chain, &ts(1)));
        reg.revoke_authority_chain(&chain);
        assert!(!reg.authority_chain_valid(&chain, &ts(1)));
    }

    #[test]
    fn unknown_credential_lookup_returns_none() {
        let reg = fresh_registry();
        assert!(reg.get_credential(&h256(0xFF)).is_none());
        assert!(reg.get_revocation(&h256(0xFF)).is_none());
        assert!(reg.get_receipt(&h256(0xFF)).is_none());
    }
}
