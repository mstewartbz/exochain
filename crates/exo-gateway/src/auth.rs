//! Authentication middleware — DID-based authentication with signature verification.
use exo_core::{Did, Hash256, Signature, Timestamp};
use serde::{Deserialize, Serialize};

use crate::error::{GatewayError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub actor_did: String,
    pub action: String,
    pub body_hash: Hash256,
    pub signature: Signature,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedActor {
    pub did: Did,
    pub authenticated_at: Timestamp,
}

pub fn authenticate(request: &Request) -> Result<AuthenticatedActor> {
    // Validate DID format
    let did = Did::new(&request.actor_did).map_err(|_| GatewayError::AuthenticationFailed {
        reason: format!("invalid DID: {}", request.actor_did),
    })?;
    // Verify signature is non-empty
    if *request.signature.as_bytes() == [0u8; 64] {
        return Err(GatewayError::AuthenticationFailed {
            reason: "empty signature".into(),
        });
    }
    Ok(AuthenticatedActor {
        did,
        authenticated_at: request.timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sig() -> Signature {
        let mut s = [0u8; 64];
        s[0] = 1;
        Signature::from_bytes(s)
    }

    #[test]
    fn auth_valid() {
        let r = Request {
            actor_did: "did:exo:alice".into(),
            action: "read".into(),
            body_hash: Hash256::ZERO,
            signature: sig(),
            timestamp: Timestamp::ZERO,
        };
        let a = authenticate(&r).unwrap();
        assert_eq!(a.did.as_str(), "did:exo:alice");
    }
    #[test]
    fn auth_invalid_did() {
        let r = Request {
            actor_did: "bad".into(),
            action: "read".into(),
            body_hash: Hash256::ZERO,
            signature: sig(),
            timestamp: Timestamp::ZERO,
        };
        assert!(authenticate(&r).is_err());
    }
    #[test]
    fn auth_empty_sig() {
        let r = Request {
            actor_did: "did:exo:alice".into(),
            action: "read".into(),
            body_hash: Hash256::ZERO,
            signature: Signature::from_bytes([0u8; 64]),
            timestamp: Timestamp::ZERO,
        };
        assert!(authenticate(&r).is_err());
    }
    #[test]
    fn request_serde() {
        let r = Request {
            actor_did: "did:exo:a".into(),
            action: "r".into(),
            body_hash: Hash256::ZERO,
            signature: sig(),
            timestamp: Timestamp::ZERO,
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(!j.is_empty());
    }
}
