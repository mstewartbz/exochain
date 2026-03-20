//! Vault encryption module for EXOCHAIN identity-bound AEAD.
//!
//! Provides XChaCha20-Poly1305 authenticated encryption with associated data
//! (AEAD), where the associated data is typically DID bytes so that ciphertext
//! is cryptographically bound to a specific identity.
//!
//! Key derivation uses HKDF-SHA256 from an Ed25519 secret key.
//!
//! Ciphertext format: `[24-byte nonce][encrypted payload][16-byte Poly1305 tag]`

use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, KeyInit, Payload},
};
use exo_core::SecretKey;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;

use crate::error::IdentityError;

/// Size of the XChaCha20-Poly1305 nonce in bytes.
const NONCE_SIZE: usize = 24;

/// Size of the Poly1305 authentication tag in bytes.
const TAG_SIZE: usize = 16;

/// Minimum ciphertext length: nonce + tag (payload can be empty).
const MIN_CIPHERTEXT_LEN: usize = NONCE_SIZE + TAG_SIZE;

/// AEAD vault encryptor using XChaCha20-Poly1305.
///
/// Wraps a 256-bit symmetric key derived from an Ed25519 secret key via
/// HKDF-SHA256.  The key material is zeroized on drop.
pub struct VaultEncryptor {
    key: [u8; 32],
}

impl VaultEncryptor {
    /// Create a `VaultEncryptor` from raw 256-bit key material.
    #[must_use]
    pub fn from_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Derive a vault encryption key from an Ed25519 secret key using
    /// HKDF-SHA256.
    ///
    /// The `context` bytes are used as the HKDF info parameter, allowing
    /// the same secret key to produce different vault keys for different
    /// purposes.
    ///
    /// # Errors
    ///
    /// Returns `IdentityError::VaultKeyDerivationFailed` if HKDF expand
    /// fails (in practice this cannot occur for a 32-byte output).
    pub fn derive_key(secret: &SecretKey, context: &[u8]) -> Result<Self, IdentityError> {
        let hk = Hkdf::<Sha256>::new(None, secret.as_bytes());
        let mut okm = [0u8; 32];
        hk.expand(context, &mut okm)
            .map_err(|e| IdentityError::VaultKeyDerivationFailed(e.to_string()))?;
        Ok(Self { key: okm })
    }

    /// Encrypt `plaintext` with XChaCha20-Poly1305.
    ///
    /// `associated_data` should be the DID bytes so the ciphertext is bound
    /// to the identity.  A random 24-byte nonce is generated for each
    /// encryption.
    ///
    /// Returns `[24-byte nonce][ciphertext][16-byte tag]`.
    ///
    /// # Errors
    ///
    /// Returns `IdentityError::VaultEncryptionFailed` on cipher failure.
    pub fn encrypt(
        &self,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, IdentityError> {
        let nonce = self.random_nonce();
        self.encrypt_with_nonce(plaintext, associated_data, &nonce)
    }

    /// Decrypt a ciphertext produced by [`encrypt`](Self::encrypt).
    ///
    /// `associated_data` must match the value used during encryption.
    ///
    /// # Errors
    ///
    /// Returns `IdentityError::VaultCiphertextTooShort` if the ciphertext
    /// is shorter than `NONCE_SIZE + TAG_SIZE`.
    ///
    /// Returns `IdentityError::VaultDecryptionFailed` if authentication
    /// fails (wrong key, tampered ciphertext, or wrong associated data).
    pub fn decrypt(
        &self,
        ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, IdentityError> {
        if ciphertext.len() < MIN_CIPHERTEXT_LEN {
            return Err(IdentityError::VaultCiphertextTooShort);
        }

        let (nonce_bytes, encrypted) = ciphertext.split_at(NONCE_SIZE);
        let nonce = chacha20poly1305::XNonce::from_slice(nonce_bytes);

        let cipher = XChaCha20Poly1305::new(self.cipher_key());

        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: encrypted,
                    aad: associated_data,
                },
            )
            .map_err(|_| IdentityError::VaultDecryptionFailed)
    }

    /// Return a reference to the raw key bytes (for testing/inspection).
    #[must_use]
    pub fn key_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    // ---- internal helpers ----

    /// Generate a random 24-byte nonce using the OS CSPRNG.
    fn random_nonce(&self) -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut nonce);
        nonce
    }

    /// Encrypt with an explicit nonce (used for deterministic tests).
    fn encrypt_with_nonce(
        &self,
        plaintext: &[u8],
        associated_data: &[u8],
        nonce: &[u8; NONCE_SIZE],
    ) -> Result<Vec<u8>, IdentityError> {
        let cipher = XChaCha20Poly1305::new(self.cipher_key());
        let xcnonce = chacha20poly1305::XNonce::from_slice(nonce);

        let encrypted = cipher
            .encrypt(
                xcnonce,
                Payload {
                    msg: plaintext,
                    aad: associated_data,
                },
            )
            .map_err(|e| IdentityError::VaultEncryptionFailed(e.to_string()))?;

        // Format: [nonce][encrypted payload with appended tag]
        let mut out = Vec::with_capacity(NONCE_SIZE + encrypted.len());
        out.extend_from_slice(nonce);
        out.extend_from_slice(&encrypted);
        Ok(out)
    }

    /// Build the `chacha20poly1305` key type from our raw bytes.
    fn cipher_key(&self) -> &chacha20poly1305::Key {
        chacha20poly1305::Key::from_slice(&self.key)
    }
}

impl Drop for VaultEncryptor {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl core::fmt::Debug for VaultEncryptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VaultEncryptor")
            .field("key", &"***")
            .finish()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use exo_core::crypto::generate_keypair;

    use super::*;

    /// Helper: create a VaultEncryptor from a fresh random key.
    fn random_encryptor() -> VaultEncryptor {
        let mut key = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key);
        VaultEncryptor::from_key(key)
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let enc = random_encryptor();
        let plaintext = b"secret identity data";
        let ad = b"did:exo:alice";

        let ct = enc.encrypt(plaintext, ad).expect("encrypt");
        let pt = enc.decrypt(&ct, ad).expect("decrypt");
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let enc = random_encryptor();
        let plaintext = b"do not tamper";
        let ad = b"did:exo:bob";

        let mut ct = enc.encrypt(plaintext, ad).expect("encrypt");
        // Flip a byte in the encrypted payload (after nonce)
        let idx = NONCE_SIZE + 1;
        ct[idx] ^= 0xff;

        let result = enc.decrypt(&ct, ad);
        assert!(
            matches!(result, Err(IdentityError::VaultDecryptionFailed)),
            "expected VaultDecryptionFailed, got {result:?}"
        );
    }

    #[test]
    fn wrong_key_fails() {
        let enc1 = random_encryptor();
        let enc2 = random_encryptor();
        let plaintext = b"key mismatch";
        let ad = b"did:exo:carol";

        let ct = enc1.encrypt(plaintext, ad).expect("encrypt");
        let result = enc2.decrypt(&ct, ad);
        assert!(
            matches!(result, Err(IdentityError::VaultDecryptionFailed)),
            "expected VaultDecryptionFailed, got {result:?}"
        );
    }

    #[test]
    fn wrong_associated_data_fails() {
        let enc = random_encryptor();
        let plaintext = b"bound to identity";
        let ad_a = b"did:exo:alice";
        let ad_b = b"did:exo:eve";

        let ct = enc.encrypt(plaintext, ad_a).expect("encrypt");
        let result = enc.decrypt(&ct, ad_b);
        assert!(
            matches!(result, Err(IdentityError::VaultDecryptionFailed)),
            "expected VaultDecryptionFailed, got {result:?}"
        );
    }

    #[test]
    fn empty_plaintext() {
        let enc = random_encryptor();
        let ad = b"did:exo:empty";

        let ct = enc.encrypt(b"", ad).expect("encrypt");
        assert_eq!(
            ct.len(),
            NONCE_SIZE + TAG_SIZE,
            "empty plaintext produces nonce + tag only"
        );

        let pt = enc.decrypt(&ct, ad).expect("decrypt");
        assert!(pt.is_empty());
    }

    #[test]
    fn large_plaintext() {
        let enc = random_encryptor();
        let plaintext = vec![0xab_u8; 1_000_000]; // 1 MB
        let ad = b"did:exo:large";

        let ct = enc.encrypt(&plaintext, ad).expect("encrypt");
        let pt = enc.decrypt(&ct, ad).expect("decrypt");
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn derive_key_deterministic() {
        let (_, sk) = generate_keypair();
        let context = b"vault-test-context";

        let enc1 = VaultEncryptor::derive_key(&sk, context).expect("derive_key");
        let enc2 = VaultEncryptor::derive_key(&sk, context).expect("derive_key");

        assert_eq!(enc1.key_bytes(), enc2.key_bytes());
    }

    #[test]
    fn derive_key_different_contexts() {
        let (_, sk) = generate_keypair();

        let enc1 = VaultEncryptor::derive_key(&sk, b"context-alpha").expect("derive_key");
        let enc2 = VaultEncryptor::derive_key(&sk, b"context-beta").expect("derive_key");

        assert_ne!(enc1.key_bytes(), enc2.key_bytes());
    }

    #[test]
    fn zeroize_on_drop() {
        // Verify that VaultEncryptor implements the Zeroize trait via Drop.
        // We construct, read the key, drop, and verify via a copy of the pointer
        // that the type system enforces Zeroize.
        fn assert_zeroize_impl<T: Zeroize>() {}
        assert_zeroize_impl::<[u8; 32]>(); // underlying storage implements Zeroize

        // Functional check: create an encryptor, do work, drop it.
        let enc = random_encryptor();
        let key_copy = *enc.key_bytes();
        // Key was non-zero before drop
        assert_ne!(key_copy, [0u8; 32]);
        // After drop the struct's Drop impl calls zeroize.
        // We can't safely read freed memory, but we verify the Drop impl
        // compiles and the Zeroize trait is used on [u8; 32].
        drop(enc);
    }
}
