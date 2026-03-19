//! Key lifecycle management for EXOCHAIN identities.

use std::collections::BTreeMap;
use exo_core::{Did, PublicKey, SecretKey, Timestamp};
use exo_core::crypto::generate_keypair;
use serde::{Deserialize, Serialize};
use crate::error::IdentityError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyStatus {
    Active,
    Rotated,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRecord {
    pub public_key: PublicKey,
    pub status: KeyStatus,
    pub created: Timestamp,
    pub rotated_to: Option<PublicKey>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Default)]
pub struct KeyStore {
    records: BTreeMap<String, Vec<KeyRecord>>,
    clock: u64,
}

impl KeyStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_key(&mut self, did: &Did) -> Result<(PublicKey, SecretKey), IdentityError> {
        let (pk, sk) = generate_keypair();
        self.clock += 1;

        let record = KeyRecord {
            public_key: pk,
            status: KeyStatus::Active,
            created: Timestamp::new(self.clock, 0),
            rotated_to: None,
            revocation_reason: None,
        };

        self.records
            .entry(did.as_str().to_owned())
            .or_default()
            .push(record);

        Ok((pk, sk))
    }

    pub fn rotate_key(
        &mut self,
        did: &Did,
        old: &PublicKey,
    ) -> Result<(PublicKey, SecretKey), IdentityError> {
        let records = self
            .records
            .get_mut(did.as_str())
            .ok_or_else(|| IdentityError::DidNotFound(did.clone()))?;

        let old_record = records
            .iter_mut()
            .find(|r| r.public_key == *old)
            .ok_or_else(|| IdentityError::KeyNotFound(did.clone()))?;

        match old_record.status {
            KeyStatus::Active => {}
            KeyStatus::Rotated => return Err(IdentityError::KeyAlreadyRotated),
            KeyStatus::Revoked | KeyStatus::Expired => return Err(IdentityError::KeyAlreadyRevoked),
        }

        let (new_pk, new_sk) = generate_keypair();
        self.clock += 1;

        old_record.status = KeyStatus::Rotated;
        old_record.rotated_to = Some(new_pk);

        let new_record = KeyRecord {
            public_key: new_pk,
            status: KeyStatus::Active,
            created: Timestamp::new(self.clock, 0),
            rotated_to: None,
            revocation_reason: None,
        };
        records.push(new_record);

        Ok((new_pk, new_sk))
    }

    pub fn revoke_key(
        &mut self,
        did: &Did,
        key: &PublicKey,
        reason: &str,
    ) -> Result<(), IdentityError> {
        let records = self
            .records
            .get_mut(did.as_str())
            .ok_or_else(|| IdentityError::DidNotFound(did.clone()))?;

        let record = records
            .iter_mut()
            .find(|r| r.public_key == *key)
            .ok_or_else(|| IdentityError::KeyNotFound(did.clone()))?;

        if record.status == KeyStatus::Revoked {
            return Err(IdentityError::KeyAlreadyRevoked);
        }

        record.status = KeyStatus::Revoked;
        record.revocation_reason = Some(reason.to_owned());
        Ok(())
    }

    #[must_use]
    pub fn get_keys(&self, did: &Did) -> Option<&[KeyRecord]> {
        self.records.get(did.as_str()).map(Vec::as_slice)
    }

    #[must_use]
    pub fn active_key(&self, did: &Did) -> Option<&PublicKey> {
        self.records.get(did.as_str()).and_then(|records| {
            records
                .iter()
                .rev()
                .find(|r| r.status == KeyStatus::Active)
                .map(|r| &r.public_key)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_did(label: &str) -> Did {
        Did::new(&format!("did:exo:{label}")).expect("valid did")
    }

    #[test]
    fn create_key_stores_record() {
        let did = make_did("test1");
        let mut store = KeyStore::new();
        let (pk, _sk) = store.create_key(&did).unwrap();

        let keys = store.get_keys(&did).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].status, KeyStatus::Active);
        assert_eq!(keys[0].public_key, pk);
    }

    #[test]
    fn active_key_returns_latest() {
        let did = make_did("test2");
        let mut store = KeyStore::new();
        let (pk1, _sk1) = store.create_key(&did).unwrap();
        assert_eq!(store.active_key(&did), Some(&pk1));
    }

    #[test]
    fn rotate_key_marks_old_and_creates_new() {
        let did = make_did("test3");
        let mut store = KeyStore::new();
        let (pk1, _sk1) = store.create_key(&did).unwrap();
        let (pk2, _sk2) = store.rotate_key(&did, &pk1).unwrap();

        let keys = store.get_keys(&did).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].status, KeyStatus::Rotated);
        assert_eq!(keys[0].rotated_to.as_ref(), Some(&pk2));
        assert_eq!(keys[1].status, KeyStatus::Active);
        assert_eq!(keys[1].public_key, pk2);
        assert_eq!(store.active_key(&did), Some(&pk2));
    }

    #[test]
    fn rotate_unknown_did_fails() {
        let did = make_did("unknown");
        let (pk, _) = generate_keypair();
        let mut store = KeyStore::new();
        assert!(matches!(store.rotate_key(&did, &pk).unwrap_err(), IdentityError::DidNotFound(_)));
    }

    #[test]
    fn rotate_unknown_key_fails() {
        let did = make_did("test4");
        let mut store = KeyStore::new();
        store.create_key(&did).unwrap();
        let (other_pk, _) = generate_keypair();
        assert!(matches!(store.rotate_key(&did, &other_pk).unwrap_err(), IdentityError::KeyNotFound(_)));
    }

    #[test]
    fn rotate_already_rotated_fails() {
        let did = make_did("test5");
        let mut store = KeyStore::new();
        let (pk1, _) = store.create_key(&did).unwrap();
        store.rotate_key(&did, &pk1).unwrap();
        assert!(matches!(store.rotate_key(&did, &pk1).unwrap_err(), IdentityError::KeyAlreadyRotated));
    }

    #[test]
    fn rotate_revoked_key_fails() {
        let did = make_did("test5b");
        let mut store = KeyStore::new();
        let (pk1, _) = store.create_key(&did).unwrap();
        store.revoke_key(&did, &pk1, "compromised").unwrap();
        assert!(matches!(store.rotate_key(&did, &pk1).unwrap_err(), IdentityError::KeyAlreadyRevoked));
    }

    #[test]
    fn revoke_key_success() {
        let did = make_did("test6");
        let mut store = KeyStore::new();
        let (pk, _) = store.create_key(&did).unwrap();
        store.revoke_key(&did, &pk, "compromised").unwrap();

        let keys = store.get_keys(&did).unwrap();
        assert_eq!(keys[0].status, KeyStatus::Revoked);
        assert_eq!(keys[0].revocation_reason.as_deref(), Some("compromised"));
        assert!(store.active_key(&did).is_none());
    }

    #[test]
    fn revoke_unknown_did_fails() {
        let did = make_did("unknown2");
        let (pk, _) = generate_keypair();
        let mut store = KeyStore::new();
        assert!(matches!(store.revoke_key(&did, &pk, "reason").unwrap_err(), IdentityError::DidNotFound(_)));
    }

    #[test]
    fn revoke_unknown_key_fails() {
        let did = make_did("test7");
        let mut store = KeyStore::new();
        store.create_key(&did).unwrap();
        let (other_pk, _) = generate_keypair();
        assert!(matches!(store.revoke_key(&did, &other_pk, "reason").unwrap_err(), IdentityError::KeyNotFound(_)));
    }

    #[test]
    fn revoke_already_revoked_fails() {
        let did = make_did("test8");
        let mut store = KeyStore::new();
        let (pk, _) = store.create_key(&did).unwrap();
        store.revoke_key(&did, &pk, "first").unwrap();
        assert!(matches!(store.revoke_key(&did, &pk, "second").unwrap_err(), IdentityError::KeyAlreadyRevoked));
    }

    #[test]
    fn get_keys_unknown_did() {
        let store = KeyStore::new();
        let did = make_did("unknown3");
        assert!(store.get_keys(&did).is_none());
    }

    #[test]
    fn active_key_unknown_did() {
        let store = KeyStore::new();
        let did = make_did("unknown4");
        assert!(store.active_key(&did).is_none());
    }

    #[test]
    fn multiple_keys_per_did() {
        let did = make_did("multi");
        let mut store = KeyStore::new();
        let (pk1, _) = store.create_key(&did).unwrap();
        let (pk2, _) = store.create_key(&did).unwrap();

        let keys = store.get_keys(&did).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].public_key, pk1);
        assert_eq!(keys[1].public_key, pk2);
    }

    #[test]
    fn clock_increments_deterministically() {
        let did = make_did("clock");
        let mut store = KeyStore::new();
        store.create_key(&did).unwrap();
        store.create_key(&did).unwrap();

        let keys = store.get_keys(&did).unwrap();
        assert_eq!(keys[0].created, Timestamp::new(1, 0));
        assert_eq!(keys[1].created, Timestamp::new(2, 0));
    }
}
