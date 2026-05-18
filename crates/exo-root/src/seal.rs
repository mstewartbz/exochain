//! Share sealing and pairwise round-two payload protection.

use argon2::Argon2;
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::{Result, RootError};

/// AEAD-wrapped certifier share artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedShare {
    /// Argon2id salt.
    pub salt: Vec<u8>,
    /// XChaCha20-Poly1305 nonce.
    pub nonce: [u8; 24],
    /// Ciphertext and tag.
    pub ciphertext: Vec<u8>,
}

/// Recipient-bound encrypted payload for DKG round two exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairwiseEncryptedPayload {
    /// XChaCha20-Poly1305 nonce.
    pub nonce: [u8; 24],
    /// Ciphertext and tag.
    pub ciphertext: Vec<u8>,
}

fn chacha_from_key(key: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(key.into())
}

fn derive_sealing_key(passphrase: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(protection_error)?;
    Ok(key)
}

fn derive_pairwise_key(
    local_secret: &[u8; 32],
    peer_public: &[u8; 32],
    associated_data: &[u8],
) -> Result<[u8; 32]> {
    let secret = StaticSecret::from(*local_secret);
    let peer_public = X25519PublicKey::from(*peer_public);
    let shared = secret.diffie_hellman(&peer_public);
    let hkdf = Hkdf::<Sha256>::new(Some(associated_data), shared.as_bytes());
    let mut key = [0u8; 32];
    hkdf.expand(b"EXOCHAIN_ROOT_PAIRWISE_V1", &mut key)
        .map_err(protection_error)?;
    Ok(key)
}

fn protection_error(error: impl core::fmt::Display) -> RootError {
    RootError::ProtectionFailed {
        reason: error.to_string(),
    }
}

/// Seal one serialized share artifact with passphrase-derived AEAD.
pub fn seal_share(
    share_bytes: &[u8],
    passphrase: &[u8],
    associated_data: &[u8],
    salt: &[u8; 16],
    nonce: &[u8; 24],
) -> Result<SealedShare> {
    let mut key = derive_sealing_key(passphrase, salt)?;
    let cipher = chacha_from_key(&key);
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: share_bytes,
                aad: associated_data,
            },
        )
        .map_err(|_| protection_message("share encryption failed"))?;
    key.zeroize();
    Ok(SealedShare {
        salt: salt.to_vec(),
        nonce: *nonce,
        ciphertext,
    })
}

/// Open one sealed share artifact.
pub fn unseal_share(
    sealed: &SealedShare,
    passphrase: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>> {
    let mut key = derive_sealing_key(passphrase, sealed.salt.as_slice())?;
    let cipher = chacha_from_key(&key);
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(&sealed.nonce),
            Payload {
                msg: sealed.ciphertext.as_slice(),
                aad: associated_data,
            },
        )
        .map_err(|_| protection_message("share opening failed"))?;
    key.zeroize();
    Ok(plaintext)
}

/// Encrypt a DKG round-two payload for exactly one recipient.
pub fn encrypt_pairwise_payload(
    sender_transport_secret: &[u8; 32],
    recipient_transport_public: &[u8; 32],
    payload: &[u8],
    associated_data: &[u8],
    nonce: &[u8; 24],
) -> Result<PairwiseEncryptedPayload> {
    let mut key = derive_pairwise_key(
        sender_transport_secret,
        recipient_transport_public,
        associated_data,
    )?;
    let cipher = chacha_from_key(&key);
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: payload,
                aad: associated_data,
            },
        )
        .map_err(|_| protection_message("pairwise encryption failed"))?;
    key.zeroize();
    Ok(PairwiseEncryptedPayload {
        nonce: *nonce,
        ciphertext,
    })
}

/// Decrypt a DKG round-two payload from one sender.
pub fn decrypt_pairwise_payload(
    recipient_transport_secret: &[u8; 32],
    sender_transport_public: &[u8; 32],
    encrypted: &PairwiseEncryptedPayload,
    associated_data: &[u8],
) -> Result<Vec<u8>> {
    let mut key = derive_pairwise_key(
        recipient_transport_secret,
        sender_transport_public,
        associated_data,
    )?;
    let cipher = chacha_from_key(&key);
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(&encrypted.nonce),
            Payload {
                msg: encrypted.ciphertext.as_slice(),
                aad: associated_data,
            },
        )
        .map_err(|_| protection_message("pairwise opening failed"))?;
    key.zeroize();
    Ok(plaintext)
}

fn protection_message(reason: &str) -> RootError {
    RootError::ProtectionFailed {
        reason: reason.to_owned(),
    }
}
