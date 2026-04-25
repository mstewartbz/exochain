//! Session proof-of-possession helpers for 0dentity.
//!
//! Session bootstrap and mutating authenticated requests are signed over
//! domain-tagged CBOR payloads so the same logical authorization challenge has
//! one deterministic byte representation.

use exo_core::types::{Did, Hash256, PublicKey, Signature};
use serde::Serialize;

pub(crate) const BOOTSTRAP_SIGNING_DOMAIN: &str = "exo.zerodentity.session_bootstrap.v1";
pub(crate) const REQUEST_SIGNING_DOMAIN: &str = "exo.zerodentity.session_request.v1";

#[derive(Serialize)]
struct BootstrapSigningPayload<'a> {
    domain: &'static str,
    challenge_id: &'a str,
    subject_did: &'a str,
    public_key: &'a PublicKey,
}

#[derive(Serialize)]
struct RequestSigningPayload<'a> {
    domain: &'static str,
    method: &'a str,
    path_and_query: &'a str,
    session_token: &'a str,
    nonce: &'a str,
    body_hash: &'a Hash256,
}

fn encode_cbor<T: Serialize>(payload: &T) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    ciborium::into_writer(payload, &mut encoded)
        .map_err(|e| format!("canonical CBOR encoding failed: {e:?}"))?;
    Ok(encoded)
}

pub(crate) fn bootstrap_signing_payload(
    challenge_id: &str,
    subject_did: &Did,
    public_key: &PublicKey,
) -> Result<Vec<u8>, String> {
    encode_cbor(&BootstrapSigningPayload {
        domain: BOOTSTRAP_SIGNING_DOMAIN,
        challenge_id,
        subject_did: subject_did.as_str(),
        public_key,
    })
}

pub(crate) fn request_signing_payload(
    method: &str,
    path_and_query: &str,
    session_token: &str,
    nonce: &str,
    body_hash: &Hash256,
) -> Result<Vec<u8>, String> {
    encode_cbor(&RequestSigningPayload {
        domain: REQUEST_SIGNING_DOMAIN,
        method,
        path_and_query,
        session_token,
        nonce,
        body_hash,
    })
}

pub(crate) fn public_key_from_hex(value: &str) -> Result<PublicKey, String> {
    let bytes = hex::decode(value).map_err(|_| "public_key must be hex".to_owned())?;
    if bytes.len() != 32 {
        return Err(format!("public_key must be 32 bytes, got {}", bytes.len()));
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Err("public_key must not be all zero".to_owned());
    }

    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&bytes);
    Ok(PublicKey::from_bytes(public_key))
}

pub(crate) fn signature_from_hex(value: &str) -> Result<Signature, String> {
    let bytes = hex::decode(value).map_err(|_| "signature must be hex".to_owned())?;
    if bytes.len() != 64 {
        return Err(format!("signature must be 64 bytes, got {}", bytes.len()));
    }

    let mut signature = [0u8; 64];
    signature.copy_from_slice(&bytes);
    Ok(Signature::from_bytes(signature))
}

pub(crate) fn public_key_from_session_bytes(value: &[u8]) -> Result<PublicKey, String> {
    if value.len() != 32 {
        return Err(format!(
            "session public key must be 32 bytes, got {}",
            value.len()
        ));
    }
    if value.iter().all(|byte| *byte == 0) {
        return Err("session public key must not be all zero".to_owned());
    }

    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(value);
    Ok(PublicKey::from_bytes(public_key))
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use exo_core::types::Hash256;

    use super::*;

    fn must<T, E: Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error:?}"),
        }
    }

    fn did() -> Did {
        must(Did::new("did:exo:session-auth"))
    }

    fn public_key() -> PublicKey {
        PublicKey::from_bytes([7u8; 32])
    }

    #[test]
    fn bootstrap_payload_is_deterministic() {
        let first = must(bootstrap_signing_payload(
            "challenge-1",
            &did(),
            &public_key(),
        ));
        let second = must(bootstrap_signing_payload(
            "challenge-1",
            &did(),
            &public_key(),
        ));
        assert_eq!(first, second);
    }

    #[test]
    fn request_payload_changes_with_nonce_and_body() {
        let body_a = Hash256::digest(b"a");
        let body_b = Hash256::digest(b"b");
        let first = must(request_signing_payload(
            "POST", "/path", "token", "nonce-1", &body_a,
        ));
        let different_nonce = must(request_signing_payload(
            "POST", "/path", "token", "nonce-2", &body_a,
        ));
        let different_body = must(request_signing_payload(
            "POST", "/path", "token", "nonce-1", &body_b,
        ));

        assert_ne!(first, different_nonce);
        assert_ne!(first, different_body);
    }

    #[test]
    fn parsers_reject_wrong_lengths_and_zero_key() {
        assert!(public_key_from_hex(&hex::encode([0u8; 32])).is_err());
        assert!(public_key_from_hex(&hex::encode([1u8; 31])).is_err());
        assert!(signature_from_hex(&hex::encode([1u8; 63])).is_err());
    }
}
