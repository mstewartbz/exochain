//! X25519 Diffie-Hellman key exchange for E2E encrypted messaging.
//!
//! Generates ephemeral X25519 keypairs and derives shared secrets via ECDH.
//! The shared secret is then expanded via HKDF-SHA256 into a 256-bit
//! symmetric key suitable for XChaCha20-Poly1305.

use hkdf::Hkdf;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::error::MessagingError;

const X25519_HKDF_SALT_DOMAIN: &[u8] = b"exo.messaging.x25519.hkdf.salt.v1";

/// An X25519 public key (32 bytes).
#[derive(Clone, PartialEq, Eq)]
pub struct X25519PublicKey([u8; 32]);

impl X25519PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, MessagingError> {
        validate_x25519_public_key(&bytes)?;
        Ok(Self(bytes))
    }

    fn from_trusted_bytes(bytes: [u8; 32]) -> Self {
        debug_assert!(validate_x25519_public_key(&bytes).is_ok());
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create from hex string.
    pub fn from_hex(hex: &str) -> Result<Self, MessagingError> {
        let bytes = hex::decode(hex)
            .map_err(|e| MessagingError::KeyExchangeFailed(format!("invalid hex: {e}")))?;
        if bytes.len() != 32 {
            return Err(MessagingError::KeyExchangeFailed(format!(
                "expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Self::from_bytes(arr)
    }

    /// Encode as hex string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Serialize for X25519PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for X25519PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <[u8; 32]>::deserialize(deserializer)?;
        Self::from_bytes(bytes).map_err(de::Error::custom)
    }
}

impl core::fmt::Debug for X25519PublicKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "X25519PublicKey({})", self.to_hex())
    }
}

/// An X25519 secret key (32 bytes). Zeroized on drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct X25519SecretKey([u8; 32]);

impl X25519SecretKey {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create from hex string.
    pub fn from_hex(hex: &str) -> Result<Self, MessagingError> {
        let bytes = zeroize::Zeroizing::new(
            hex::decode(hex)
                .map_err(|e| MessagingError::KeyExchangeFailed(format!("invalid hex: {e}")))?,
        );
        if bytes.len() != 32 {
            return Err(MessagingError::KeyExchangeFailed(format!(
                "expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Derive the public key corresponding to this secret key.
    #[must_use]
    pub fn public_key(&self) -> X25519PublicKey {
        let secret = StaticSecret::from(self.0);
        let public = PublicKey::from(&secret);
        X25519PublicKey::from_trusted_bytes(public.to_bytes())
    }

    fn static_secret(&self) -> StaticSecret {
        StaticSecret::from(self.0)
    }
}

impl core::fmt::Debug for X25519SecretKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("X25519SecretKey")
            .field("key", &"***")
            .finish()
    }
}

/// An X25519 keypair for Diffie-Hellman key exchange.
#[derive(Debug)]
pub struct X25519KeyPair {
    pub public: X25519PublicKey,
    pub secret: X25519SecretKey,
}

impl X25519KeyPair {
    /// Generate a fresh random X25519 keypair.
    #[must_use]
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
        let public = PublicKey::from(&secret);
        Self {
            public: X25519PublicKey::from_trusted_bytes(public.to_bytes()),
            secret: X25519SecretKey(secret.to_bytes()),
        }
    }

    /// Reconstruct from raw secret bytes.
    #[must_use]
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        let secret = StaticSecret::from(bytes);
        let public = PublicKey::from(&secret);
        Self {
            public: X25519PublicKey::from_trusted_bytes(public.to_bytes()),
            secret: X25519SecretKey(secret.to_bytes()),
        }
    }
}

/// Perform X25519 ECDH and derive a 256-bit symmetric key via HKDF-SHA256.
///
/// The `context` parameter binds the derived key to a specific purpose
/// (e.g., `b"vitallock-message-v1"`).
pub fn derive_shared_key(
    our_secret: &X25519SecretKey,
    their_public: &X25519PublicKey,
    context: &[u8],
) -> Result<[u8; 32], MessagingError> {
    let secret = our_secret.static_secret();
    let public = PublicKey::from(*their_public.as_bytes());
    let shared_secret = secret.diffie_hellman(&public);
    if shared_secret.as_bytes().iter().all(|byte| *byte == 0) {
        return Err(MessagingError::KeyExchangeFailed(
            "invalid X25519 public key: low-order shared secret".to_owned(),
        ));
    }
    let our_public = X25519PublicKey::from_trusted_bytes(PublicKey::from(&secret).to_bytes());
    let salt = hkdf_salt(&our_public, their_public);

    // HKDF-SHA256: extract from shared secret with a deterministic transcript
    // salt, then expand with caller-supplied context.
    let hk = Hkdf::<Sha256>::new(Some(&salt), shared_secret.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(context, &mut okm)
        .map_err(|e| MessagingError::KeyExchangeFailed(e.to_string()))?;
    Ok(okm)
}

fn hkdf_salt(our_public: &X25519PublicKey, their_public: &X25519PublicKey) -> [u8; 32] {
    let (first, second) = if our_public.as_bytes() <= their_public.as_bytes() {
        (our_public.as_bytes(), their_public.as_bytes())
    } else {
        (their_public.as_bytes(), our_public.as_bytes())
    };

    let mut hasher = Sha256::new();
    hasher.update(X25519_HKDF_SALT_DOMAIN);
    hasher.update(first);
    hasher.update(second);
    hasher.finalize().into()
}

fn validate_x25519_public_key(bytes: &[u8; 32]) -> Result<(), MessagingError> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Err(MessagingError::KeyExchangeFailed(
            "invalid X25519 public key: all-zero value".to_owned(),
        ));
    }

    let validation_secret = StaticSecret::from([0x5a; 32]);
    let validation_public = PublicKey::from(*bytes);
    let validation_shared = validation_secret.diffie_hellman(&validation_public);
    if validation_shared.as_bytes().iter().all(|byte| *byte == 0) {
        return Err(MessagingError::KeyExchangeFailed(
            "invalid X25519 public key: low-order point".to_owned(),
        ));
    }

    Ok(())
}

/// Generate an ephemeral X25519 keypair for one-time use in message encryption.
///
/// This uses `EphemeralSecret` which is consumed after a single DH operation,
/// but we return the raw bytes so the ephemeral public key can be included
/// in the message envelope.
#[must_use]
pub fn generate_ephemeral() -> X25519KeyPair {
    X25519KeyPair::generate()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_generation() {
        let kp = X25519KeyPair::generate();
        assert_ne!(
            *kp.public.as_bytes(),
            [0u8; 32],
            "public key should not be all zeros"
        );
    }

    #[test]
    fn ecdh_shared_secret_agreement() {
        let alice = X25519KeyPair::generate();
        let bob = X25519KeyPair::generate();
        let context = b"test-context";

        let alice_key =
            derive_shared_key(&alice.secret, &bob.public, context).expect("alice derive");
        let bob_key = derive_shared_key(&bob.secret, &alice.public, context).expect("bob derive");

        assert_eq!(alice_key, bob_key, "shared keys must match");
    }

    #[test]
    fn different_contexts_produce_different_keys() {
        let alice = X25519KeyPair::generate();
        let bob = X25519KeyPair::generate();

        let key1 = derive_shared_key(&alice.secret, &bob.public, b"context-a").expect("derive");
        let key2 = derive_shared_key(&alice.secret, &bob.public, b"context-b").expect("derive");

        assert_ne!(key1, key2, "different contexts must produce different keys");
    }

    #[test]
    fn from_secret_bytes_deterministic() {
        let secret = X25519SecretKey::from_bytes([7u8; 32]);
        let kp1 = X25519KeyPair::from_secret_bytes([7u8; 32]);
        let kp2 = X25519KeyPair {
            public: secret.public_key(),
            secret,
        };

        assert_eq!(
            kp1.public.as_bytes(),
            kp2.public.as_bytes(),
            "same secret → same public"
        );
    }

    #[test]
    fn hex_round_trip() {
        let kp = X25519KeyPair::generate();
        let hex = kp.public.to_hex();
        let recovered = X25519PublicKey::from_hex(&hex).expect("from_hex");
        assert_eq!(kp.public, recovered);
    }

    #[test]
    fn x25519_public_key_rejects_all_zero_hex() {
        let zero_hex = "00".repeat(32);

        let result = X25519PublicKey::from_hex(&zero_hex);

        assert!(
            matches!(result, Err(MessagingError::KeyExchangeFailed(reason)) if reason.contains("invalid X25519 public key")),
            "all-zero X25519 public keys must be rejected"
        );
    }

    #[test]
    fn x25519_public_key_deserialization_rejects_all_zero_bytes() {
        let zero_bytes = format!("[{}]", vec!["0"; 32].join(","));

        let result: Result<X25519PublicKey, _> = serde_json::from_str(&zero_bytes);

        assert!(
            result.is_err(),
            "serde deserialization must validate X25519 public keys"
        );
    }

    #[test]
    fn x25519_public_key_source_does_not_expose_inner_bytes() {
        let source = include_str!("kex.rs");

        assert!(
            !source.contains(&["pub struct X25519PublicKey", "(pub"].concat()),
            "X25519 public key bytes must not be exposed through a public tuple field"
        );
    }

    #[test]
    fn x25519_secret_key_source_does_not_expose_inner_bytes_or_plain_hex() {
        let source = include_str!("kex.rs");
        let secret_impl = source
            .split("impl X25519SecretKey {")
            .nth(1)
            .and_then(|rest| {
                rest.split("impl core::fmt::Debug for X25519SecretKey")
                    .next()
            })
            .expect("secret-key impl block must be present");
        assert!(
            !source.contains(&["pub struct X25519SecretKey", "(pub"].concat()),
            "X25519 secret key bytes must not be exposed through a public tuple field"
        );
        assert!(
            !secret_impl.contains(&["pub fn to_", "hex(&self) -> String"].concat()),
            "X25519 secret keys must not expose a plain String hex encoder"
        );
        assert!(
            !source.contains(&["our_secret", ".0"].concat()),
            "internal key exchange must use the bounded secret-key accessor"
        );
    }

    #[test]
    fn derive_shared_key_uses_protocol_bound_hkdf_salt() {
        let source = include_str!("kex.rs");
        assert!(
            !source.contains(&["Hkdf::<Sha256>::new", "(None"].concat()),
            "X25519 shared-secret HKDF extraction must use a protocol-bound salt"
        );
    }
}
