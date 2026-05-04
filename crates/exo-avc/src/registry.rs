//! Deterministic in-memory registry for AVC credentials, revocations,
//! receipts, and the ancillary state required by validation
//! (issuer public keys, validated authority chain hashes, consent and
//! policy reference existence).
//!
//! All maps and sets are `BTreeMap`/`BTreeSet` so iteration order is
//! deterministic. Persistence is **out of scope** for this MVP and is
//! the subject of a follow-up PR.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::{Did, Hash256, PublicKey, Timestamp};

use crate::{
    credential::AutonomousVolitionCredential, error::AvcError, receipt::AvcTrustReceipt,
    revocation::AvcRevocation,
};

/// Read-only registry interface used by validation.
pub trait AvcRegistryRead {
    fn resolve_public_key(&self, did: &Did) -> Option<PublicKey>;
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
    fn put_credential(
        &mut self,
        credential: AutonomousVolitionCredential,
    ) -> Result<Hash256, AvcError>;
    fn put_revocation(&mut self, revocation: AvcRevocation) -> Result<(), AvcError>;
    fn put_receipt(&mut self, receipt: AvcTrustReceipt) -> Result<(), AvcError>;
    fn put_public_key(&mut self, did: Did, public_key: PublicKey);
    fn add_consent_ref(&mut self, consent_id: Hash256);
    fn add_policy_ref(&mut self, policy_id: Hash256, policy_version: u16);
    fn mark_authority_chain_valid(&mut self, chain_hash: Hash256);
    fn revoke_authority_chain(&mut self, chain_hash: &Hash256);
}

/// Deterministic in-memory implementation of the registry traits.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAvcRegistry {
    credentials: BTreeMap<Hash256, AutonomousVolitionCredential>,
    by_subject: BTreeMap<Did, BTreeSet<Hash256>>,
    revocations: BTreeMap<Hash256, AvcRevocation>,
    receipts: BTreeMap<Hash256, AvcTrustReceipt>,
    public_keys: BTreeMap<Did, PublicKey>,
    consent_refs: BTreeSet<Hash256>,
    policy_refs: BTreeSet<(Hash256, u16)>,
    authority_chains: BTreeSet<Hash256>,
}

impl InMemoryAvcRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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

    /// Mark a credential as revoked using a deterministic placeholder
    /// revocation record signed by `revoker_did`. The placeholder
    /// signature is empty because no key material is available in this
    /// administrative path; downstream validation will still observe
    /// `is_revoked == true` regardless of signature presence.
    ///
    /// # Errors
    /// Returns [`AvcError::InvalidInput`] when `revoker_did` is malformed.
    pub fn mark_revoked_with(
        &mut self,
        credential_id: Hash256,
        revoker_did: Did,
    ) -> Result<(), AvcError> {
        self.revocations
            .entry(credential_id)
            .or_insert(AvcRevocation {
                schema_version: crate::credential::AVC_SCHEMA_VERSION,
                credential_id,
                revoker_did,
                reason: crate::revocation::AvcRevocationReason::IssuerRevoked,
                created_at: Timestamp::ZERO,
                signature: exo_core::Signature::empty(),
            });
        Ok(())
    }

    /// Test convenience around [`Self::mark_revoked_with`] using a
    /// fixed administrative DID. Production callers should use
    /// [`AvcRegistryWrite::put_revocation`] with a properly signed
    /// `AvcRevocation` instead.
    #[cfg(test)]
    pub fn mark_revoked(&mut self, credential_id: Hash256) {
        let did = Did::new("did:exo:test-revoker").unwrap();
        let _ = self.mark_revoked_with(credential_id, did);
    }

    /// Get the receipt with the given hash, if present.
    #[must_use]
    pub fn get_receipt(&self, receipt_hash: &Hash256) -> Option<AvcTrustReceipt> {
        self.receipts.get(receipt_hash).cloned()
    }
}

impl AvcRegistryRead for InMemoryAvcRegistry {
    fn resolve_public_key(&self, did: &Did) -> Option<PublicKey> {
        self.public_keys.get(did).copied()
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
        self.receipts.insert(key, receipt);
        Ok(())
    }

    fn put_public_key(&mut self, did: Did, public_key: PublicKey) {
        self.public_keys.insert(did, public_key);
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
    use super::*;
    use crate::credential::{
        issue_avc,
        test_support::{baseline_draft, did, h256, ts},
    };
    use crate::revocation::{AvcRevocation, AvcRevocationReason};
    use exo_core::Signature;

    fn fixed_signature() -> Signature {
        Signature::from_bytes([7u8; 64])
    }

    fn fresh_registry() -> InMemoryAvcRegistry {
        InMemoryAvcRegistry::new()
    }

    fn sample_credential() -> AutonomousVolitionCredential {
        issue_avc(baseline_draft(), |_| fixed_signature()).unwrap()
    }

    fn sample_revocation(id: Hash256) -> AvcRevocation {
        AvcRevocation {
            schema_version: crate::credential::AVC_SCHEMA_VERSION,
            credential_id: id,
            revoker_did: did("revoker"),
            reason: AvcRevocationReason::IssuerRevoked,
            created_at: ts(1),
            signature: fixed_signature(),
        }
    }

    fn sample_receipt() -> AvcTrustReceipt {
        AvcTrustReceipt {
            schema_version: crate::credential::AVC_SCHEMA_VERSION,
            receipt_id: h256(0xEE),
            credential_id: h256(0xAA),
            action_id: None,
            validator_did: did("validator"),
            decision: crate::validation::AvcDecision::Allow,
            reason_codes: vec![crate::validation::AvcReasonCode::Valid],
            created_at: ts(1),
            validation_hash: h256(0xBB),
            signature: fixed_signature(),
        }
    }

    #[test]
    fn put_get_credential_round_trips() {
        let mut reg = fresh_registry();
        let cred = sample_credential();
        let id = reg.put_credential(cred.clone()).unwrap();
        assert_eq!(reg.get_credential(&id).unwrap(), cred);
        assert_eq!(reg.credential_count(), 1);
    }

    #[test]
    fn list_credentials_for_subject_returns_subject_only() {
        let mut reg = fresh_registry();
        let cred1 = sample_credential();
        reg.put_credential(cred1.clone()).unwrap();
        // Add an unrelated subject
        let mut draft2 = baseline_draft();
        draft2.subject_did = did("agent-other");
        let cred2 = issue_avc(draft2, |_| fixed_signature()).unwrap();
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
        let cred = sample_credential();
        let id = cred.id().unwrap();
        let revocation = sample_revocation(id);
        reg.put_revocation(revocation.clone()).unwrap();
        let err = reg.put_revocation(revocation).unwrap_err();
        assert!(matches!(err, AvcError::Registry { .. }));
    }

    #[test]
    fn revoked_state_visible_via_is_revoked_and_get() {
        let mut reg = fresh_registry();
        let cred = sample_credential();
        let id = cred.id().unwrap();
        assert!(!reg.is_revoked(&id));
        let revocation = sample_revocation(id);
        reg.put_revocation(revocation.clone()).unwrap();
        assert!(reg.is_revoked(&id));
        assert_eq!(reg.get_revocation(&id).unwrap(), revocation);
        assert_eq!(reg.revocation_count(), 1);
    }

    #[test]
    fn put_receipt_rejects_duplicates() {
        let mut reg = fresh_registry();
        let receipt = sample_receipt();
        reg.put_receipt(receipt.clone()).unwrap();
        let err = reg.put_receipt(receipt.clone()).unwrap_err();
        assert!(matches!(err, AvcError::Registry { .. }));
        assert_eq!(reg.receipt_count(), 1);
        assert_eq!(reg.get_receipt(&receipt.receipt_id).unwrap(), receipt);
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
    fn mark_revoked_inserts_placeholder_record() {
        let mut reg = fresh_registry();
        let id = h256(0x77);
        assert!(!reg.is_revoked(&id));
        reg.mark_revoked(id);
        assert!(reg.is_revoked(&id));
        let revocation = reg.get_revocation(&id).unwrap();
        assert_eq!(revocation.credential_id, id);
        assert!(matches!(
            revocation.reason,
            AvcRevocationReason::IssuerRevoked
        ));
    }

    #[test]
    fn mark_revoked_with_uses_supplied_did_and_is_idempotent() {
        let mut reg = fresh_registry();
        let id = h256(0x88);
        let did = did("revoker-x");
        reg.mark_revoked_with(id, did.clone()).unwrap();
        let first = reg.get_revocation(&id).unwrap();
        // Calling again must not overwrite the existing record.
        reg.mark_revoked_with(id, did.clone()).unwrap();
        let second = reg.get_revocation(&id).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.revoker_did, did);
    }

    #[test]
    fn unknown_credential_lookup_returns_none() {
        let reg = fresh_registry();
        assert!(reg.get_credential(&h256(0xFF)).is_none());
        assert!(reg.get_revocation(&h256(0xFF)).is_none());
        assert!(reg.get_receipt(&h256(0xFF)).is_none());
    }
}
