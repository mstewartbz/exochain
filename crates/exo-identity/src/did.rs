//! Decentralized Identity (DID) document management.

use std::collections::BTreeMap;

use exo_core::{Did, PublicKey, Signature, Timestamp};
use exo_core::crypto;
use serde::{Deserialize, Serialize};

use crate::error::IdentityError;

/// Authentication method associated with a DID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticationMethod {
    pub id: String,
    pub method_type: String,
    pub public_key: PublicKey,
}

/// A service endpoint advertised by a DID subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: String,
    pub service_type: String,
    pub endpoint: String,
}

/// Proof that a DID holder authorized a revocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationProof {
    pub did: Did,
    pub signature: Signature,
}

/// A DID document describing a decentralized identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: Did,
    pub public_keys: Vec<PublicKey>,
    pub authentication: Vec<AuthenticationMethod>,
    pub service_endpoints: Vec<ServiceEndpoint>,
    pub created: Timestamp,
    pub updated: Timestamp,
    pub revoked: bool,
}

/// In-memory DID registry using a `BTreeMap` for deterministic ordering.
#[derive(Debug, Default)]
pub struct DidRegistry {
    documents: BTreeMap<String, DidDocument>,
}

impl DidRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn resolve(&self, did: &Did) -> Option<&DidDocument> {
        self.documents.get(did.as_str()).filter(|doc| !doc.revoked)
    }

    pub fn register(&mut self, doc: DidDocument) -> Result<(), IdentityError> {
        if self.documents.contains_key(doc.id.as_str()) {
            return Err(IdentityError::DuplicateDid(doc.id));
        }
        self.documents.insert(doc.id.as_str().to_owned(), doc);
        Ok(())
    }

    pub fn revoke(&mut self, did: &Did, proof: &RevocationProof) -> Result<(), IdentityError> {
        let doc = self
            .documents
            .get_mut(did.as_str())
            .ok_or_else(|| IdentityError::DidNotFound(did.clone()))?;

        let msg = did.as_str().as_bytes();
        let valid = doc
            .public_keys
            .iter()
            .any(|pk| crypto::verify(msg, &proof.signature, pk));

        if !valid {
            return Err(IdentityError::InvalidRevocationProof(did.clone()));
        }

        doc.revoked = true;
        Ok(())
    }

    pub fn rotate_key(
        &mut self,
        did: &Did,
        new_key: &PublicKey,
        proof: &Signature,
    ) -> Result<(), IdentityError> {
        let doc = self
            .documents
            .get_mut(did.as_str())
            .ok_or_else(|| IdentityError::DidNotFound(did.clone()))?;

        if doc.revoked {
            return Err(IdentityError::DidRevoked(did.clone()));
        }

        let msg = new_key.as_bytes();
        let valid = doc.public_keys.iter().any(|pk| crypto::verify(msg, proof, pk));

        if !valid {
            return Err(IdentityError::InvalidSignature);
        }

        doc.public_keys.push(*new_key);
        doc.updated = Timestamp::new(doc.updated.physical_ms + 1, 0);
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exo_core::crypto::{generate_keypair, sign};

    fn make_did(label: &str) -> Did {
        Did::new(&format!("did:exo:{label}")).expect("valid did")
    }

    fn make_doc(did: Did, pk: PublicKey) -> DidDocument {
        DidDocument {
            id: did,
            public_keys: vec![pk],
            authentication: vec![],
            service_endpoints: vec![],
            created: Timestamp::new(1000, 0),
            updated: Timestamp::new(1000, 0),
            revoked: false,
        }
    }

    #[test]
    fn register_and_resolve() {
        let (pk, _sk) = generate_keypair();
        let did = make_did("alice");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());

        let resolved = reg.resolve(&did).unwrap();
        assert_eq!(resolved.id, did);
    }

    #[test]
    fn duplicate_registration_fails() {
        let (pk, _sk) = generate_keypair();
        let did = make_did("bob");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc.clone()).unwrap();
        let err = reg.register(doc).unwrap_err();
        assert!(matches!(err, IdentityError::DuplicateDid(_)));
    }

    #[test]
    fn resolve_unknown_did_returns_none() {
        let reg = DidRegistry::new();
        assert!(reg.is_empty());
        let did = make_did("nonexistent");
        assert!(reg.resolve(&did).is_none());
    }

    #[test]
    fn revoke_did() {
        let (pk, sk) = generate_keypair();
        let did = make_did("charlie");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();

        let proof = RevocationProof {
            did: did.clone(),
            signature: sign(did.as_str().as_bytes(), &sk),
        };
        reg.revoke(&did, &proof).unwrap();
        assert!(reg.resolve(&did).is_none());
    }

    #[test]
    fn revoke_unknown_did_fails() {
        let (_pk, sk) = generate_keypair();
        let did = make_did("unknown");
        let proof = RevocationProof {
            did: did.clone(),
            signature: sign(did.as_str().as_bytes(), &sk),
        };

        let mut reg = DidRegistry::new();
        let err = reg.revoke(&did, &proof).unwrap_err();
        assert!(matches!(err, IdentityError::DidNotFound(_)));
    }

    #[test]
    fn revoke_with_invalid_proof_fails() {
        let (pk, _sk) = generate_keypair();
        let did = make_did("dave");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();

        let (_pk2, sk2) = generate_keypair();
        let proof = RevocationProof {
            did: did.clone(),
            signature: sign(did.as_str().as_bytes(), &sk2),
        };
        let err = reg.revoke(&did, &proof).unwrap_err();
        assert!(matches!(err, IdentityError::InvalidRevocationProof(_)));
    }

    #[test]
    fn rotate_key_success() {
        let (pk, sk) = generate_keypair();
        let did = make_did("eve");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();

        let (new_pk, _new_sk) = generate_keypair();
        let proof = sign(new_pk.as_bytes(), &sk);
        reg.rotate_key(&did, &new_pk, &proof).unwrap();

        let resolved = reg.resolve(&did).unwrap();
        assert_eq!(resolved.public_keys.len(), 2);
        assert!(resolved.updated.physical_ms > 1000);
    }

    #[test]
    fn rotate_key_unknown_did_fails() {
        let (_pk, sk) = generate_keypair();
        let did = make_did("unknown2");
        let (new_pk, _) = generate_keypair();
        let proof = sign(new_pk.as_bytes(), &sk);

        let mut reg = DidRegistry::new();
        let err = reg.rotate_key(&did, &new_pk, &proof).unwrap_err();
        assert!(matches!(err, IdentityError::DidNotFound(_)));
    }

    #[test]
    fn rotate_key_revoked_did_fails() {
        let (pk, sk) = generate_keypair();
        let did = make_did("frank");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();

        let revocation = RevocationProof {
            did: did.clone(),
            signature: sign(did.as_str().as_bytes(), &sk),
        };
        reg.revoke(&did, &revocation).unwrap();

        let (new_pk, _) = generate_keypair();
        let proof = sign(new_pk.as_bytes(), &sk);
        let err = reg.rotate_key(&did, &new_pk, &proof).unwrap_err();
        assert!(matches!(err, IdentityError::DidRevoked(_)));
    }

    #[test]
    fn rotate_key_invalid_signature_fails() {
        let (pk, _sk) = generate_keypair();
        let did = make_did("grace");
        let doc = make_doc(did.clone(), pk);

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();

        let (new_pk, _) = generate_keypair();
        let (_other_pk, other_sk) = generate_keypair();
        let bad_proof = sign(new_pk.as_bytes(), &other_sk);
        let err = reg.rotate_key(&did, &new_pk, &bad_proof).unwrap_err();
        assert!(matches!(err, IdentityError::InvalidSignature));
    }

    #[test]
    fn authentication_method_and_service_endpoint() {
        let (pk, _sk) = generate_keypair();
        let did = make_did("heidi");
        let doc = DidDocument {
            id: did.clone(),
            public_keys: vec![pk],
            authentication: vec![AuthenticationMethod {
                id: "auth-1".into(),
                method_type: "Ed25519VerificationKey2020".into(),
                public_key: pk,
            }],
            service_endpoints: vec![ServiceEndpoint {
                id: "svc-1".into(),
                service_type: "ExochainMessaging".into(),
                endpoint: "https://example.com/msg".into(),
            }],
            created: Timestamp::new(1000, 0),
            updated: Timestamp::new(1000, 0),
            revoked: false,
        };

        let mut reg = DidRegistry::new();
        reg.register(doc).unwrap();
        let resolved = reg.resolve(&did).unwrap();
        assert_eq!(resolved.authentication.len(), 1);
        assert_eq!(resolved.authentication[0].id, "auth-1");
        assert_eq!(resolved.service_endpoints.len(), 1);
        assert_eq!(resolved.service_endpoints[0].endpoint, "https://example.com/msg");
    }
}
