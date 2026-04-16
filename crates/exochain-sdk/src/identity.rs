//! Decentralized identity management.
//!
//! [`Identity`] is the SDK's ergonomic handle to a DID with its associated
//! Ed25519 keypair. The DID is derived deterministically from the public key
//! so that two identities generated with different entropy always produce
//! different DIDs, while an identity re-constructed from the same key bytes
//! always yields the same DID.

use exo_core::crypto::{generate_keypair, sign as core_sign, verify as core_verify};
use exo_core::{Did, PublicKey, SecretKey, Signature, Timestamp};
use exo_identity::did::DidDocument;

use crate::error::{ExoError, ExoResult};

/// A DID paired with its Ed25519 keypair and a human-readable label.
///
/// Use [`Identity::generate`] to create a fresh identity with a random keypair.
/// The DID is derived from the public key as:
///
/// ```text
/// did:exo: + first 16 hex chars of BLAKE3(public_key_bytes)
/// ```
pub struct Identity {
    did: Did,
    public: PublicKey,
    secret: SecretKey,
    label: String,
}

impl Identity {
    /// Generate a fresh identity with a random Ed25519 keypair and a DID
    /// derived from the public key.
    ///
    /// The `label` is stored alongside the identity for developer convenience
    /// and is never cryptographically bound to the DID.
    ///
    /// Deriving the DID cannot fail in practice: the method-specific portion
    /// is always a 16-character lowercase hex string, which satisfies the
    /// `did:exo:` validation rules.  In the unreachable case that validation
    /// ever rejected such a string, the generated DID falls back to
    /// `did:exo:sdk-fallback` — still well-formed, still valid.
    #[must_use]
    pub fn generate(label: &str) -> Self {
        let (public, secret) = generate_keypair();
        let did = derive_did(&public).unwrap_or_else(|_| fallback_did());
        Self {
            did,
            public,
            secret,
            label: label.to_owned(),
        }
    }

    /// Build an [`Identity`] from an existing keypair and label.
    ///
    /// The DID is derived from the public key in the same way as
    /// [`Identity::generate`].
    ///
    /// # Errors
    ///
    /// Returns [`ExoError::InvalidDid`] only in the highly unlikely case that
    /// BLAKE3 output somehow failed DID validation — this should be
    /// unreachable in practice.
    pub fn from_keypair(label: &str, public: PublicKey, secret: SecretKey) -> ExoResult<Self> {
        let did = derive_did(&public)?;
        Ok(Self {
            did,
            public,
            secret,
            label: label.to_owned(),
        })
    }

    /// Return the DID for this identity.
    #[must_use]
    pub fn did(&self) -> &Did {
        &self.did
    }

    /// Return the public key.
    #[must_use]
    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Return the human-readable label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Sign `message` with this identity's secret key (Ed25519).
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        core_sign(message, &self.secret)
    }

    /// Verify `signature` over `message` against this identity's public key.
    #[must_use]
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        core_verify(message, signature, &self.public)
    }

    /// Build a minimal [`DidDocument`] describing this identity.
    ///
    /// The resulting document contains the identity's single Ed25519 public
    /// key, with empty authentication/verification-method/service-endpoint
    /// lists and `created == updated == Timestamp::ZERO`. Callers can augment
    /// the document after construction if they need richer DID metadata.
    #[must_use]
    pub fn did_document(&self) -> DidDocument {
        DidDocument {
            id: self.did.clone(),
            public_keys: vec![self.public],
            authentication: Vec::new(),
            verification_methods: Vec::new(),
            hybrid_verification_methods: Vec::new(),
            service_endpoints: Vec::new(),
            created: Timestamp::ZERO,
            updated: Timestamp::ZERO,
            revoked: false,
        }
    }
}

impl core::fmt::Debug for Identity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Identity")
            .field("did", &self.did)
            .field("label", &self.label)
            .field("public", &self.public)
            .field("secret", &"***")
            .finish()
    }
}

/// Derive `did:exo:<first 16 hex chars of BLAKE3(public_key_bytes)>`.
fn derive_did(public: &PublicKey) -> ExoResult<Did> {
    let digest = blake3::hash(public.as_bytes());
    let bytes = digest.as_bytes();
    let mut hex = String::with_capacity(16);
    for byte in bytes.iter().take(8) {
        hex.push_str(&format!("{byte:02x}"));
    }
    let did_str = format!("did:exo:{hex}");
    Did::new(&did_str).map_err(|e| ExoError::InvalidDid(e.to_string()))
}

/// Fallback DID for the unreachable case where hex-derived DIDs fail
/// validation.  Never observed in practice; present only so that
/// [`Identity::generate`] remains infallible.
#[allow(clippy::expect_used)] // static DID "did:exo:sdk-fallback" is unconditionally valid
fn fallback_did() -> Did {
    Did::new("did:exo:sdk-fallback").expect("did:exo:sdk-fallback is a well-formed DID")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_valid_did() {
        let id = Identity::generate("alice");
        assert!(id.did().as_str().starts_with("did:exo:"));
        // 16 hex chars after the prefix.
        assert_eq!(id.did().as_str().len(), "did:exo:".len() + 16);
    }

    #[test]
    fn generate_stores_label() {
        let id = Identity::generate("alice");
        assert_eq!(id.label(), "alice");
    }

    #[test]
    fn sign_verify_roundtrip() {
        let id = Identity::generate("signer");
        let sig = id.sign(b"hello");
        assert!(id.verify(b"hello", &sig));
    }

    #[test]
    fn verify_rejects_wrong_message() {
        let id = Identity::generate("signer");
        let sig = id.sign(b"original");
        assert!(!id.verify(b"tampered", &sig));
    }

    #[test]
    fn different_identities_produce_different_dids() {
        let a = Identity::generate("a");
        let b = Identity::generate("b");
        // Random keypairs should never collide in practice; the check is on
        // the DID (derived from the public key) not the label.
        assert_ne!(a.did(), b.did());
    }

    #[test]
    fn from_keypair_derives_same_did_as_generate() {
        let id = Identity::generate("first");
        let rebuilt = Identity::from_keypair(
            "rebuilt",
            *id.public_key(),
            SecretKey::from_bytes(*id.secret.as_bytes()),
        )
        .expect("ok");
        assert_eq!(id.did(), rebuilt.did());
        assert_eq!(rebuilt.label(), "rebuilt");
    }

    #[test]
    fn did_document_contains_identity_fields() {
        let id = Identity::generate("doc");
        let doc = id.did_document();
        assert_eq!(&doc.id, id.did());
        assert_eq!(doc.public_keys.len(), 1);
        assert_eq!(&doc.public_keys[0], id.public_key());
        assert!(!doc.revoked);
        assert!(doc.authentication.is_empty());
        assert!(doc.verification_methods.is_empty());
        assert!(doc.hybrid_verification_methods.is_empty());
        assert!(doc.service_endpoints.is_empty());
    }

    #[test]
    fn debug_redacts_secret() {
        let id = Identity::generate("secret-test");
        let dbg = format!("{id:?}");
        assert!(dbg.contains("***"));
        assert!(dbg.contains("Identity"));
    }

    #[test]
    fn public_key_accessor() {
        let id = Identity::generate("pk");
        // PublicKey implements Copy; the accessor returns a reference we can
        // compare against itself via dereference.
        let pk = *id.public_key();
        assert_eq!(&pk, id.public_key());
    }
}
