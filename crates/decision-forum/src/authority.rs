//! Forum authority management.
use exo_core::{Did, Hash256, Signature};
use serde::{Deserialize, Serialize};
use crate::error::{ForumError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumRule { pub name: String, pub hash: Hash256 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumAuthority {
    pub root_did: Did,
    pub constitution_hash: Hash256,
    pub rules: Vec<ForumRule>,
    pub signature: Signature,
}

pub fn verify_forum_authority(authority: &ForumAuthority) -> Result<()> {
    // Verify signature is non-empty
    if *authority.signature.as_bytes() == [0u8; 64] {
        return Err(ForumError::AuthorityInvalid { reason: "empty signature".into() });
    }
    // Verify constitution hash is non-zero
    if authority.constitution_hash == Hash256::ZERO {
        return Err(ForumError::AuthorityInvalid { reason: "zero constitution hash".into() });
    }
    // Verify at least one rule exists
    if authority.rules.is_empty() {
        return Err(ForumError::AuthorityInvalid { reason: "no rules defined".into() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn did() -> Did { Did::new("did:exo:root").unwrap() }
    fn sig() -> Signature { let mut s = [0u8; 64]; s[0] = 1; Signature::from_bytes(s) }
    fn auth() -> ForumAuthority {
        ForumAuthority { root_did: did(), constitution_hash: Hash256::digest(b"const"),
            rules: vec![ForumRule { name: "r1".into(), hash: Hash256::digest(b"r1") }],
            signature: sig() }
    }

    #[test] fn valid_authority() { verify_forum_authority(&auth()).unwrap(); }
    #[test] fn empty_sig() { let mut a = auth(); a.signature = Signature::from_bytes([0u8; 64]); assert!(verify_forum_authority(&a).is_err()); }
    #[test] fn zero_hash() { let mut a = auth(); a.constitution_hash = Hash256::ZERO; assert!(verify_forum_authority(&a).is_err()); }
    #[test] fn no_rules() { let mut a = auth(); a.rules.clear(); assert!(verify_forum_authority(&a).is_err()); }
    #[test] fn serde() { let a = auth(); let j = serde_json::to_string(&a).unwrap(); assert!(!j.is_empty()); }
}
