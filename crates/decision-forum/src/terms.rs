//! Terms & Conditions enforcement.
use exo_core::{Did, Signature, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use exo_core::Hash256;
use crate::error::{ForumError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Terms {
    pub version: u64,
    pub hash: Hash256,
    pub acceptance_required: bool,
    pub accepted_by: BTreeMap<Did, Timestamp>,
}

impl Terms {
    #[must_use] pub fn new(content: &[u8], acceptance_required: bool) -> Self {
        Terms { version: 1, hash: Hash256::digest(content), acceptance_required, accepted_by: BTreeMap::new() }
    }
}

#[must_use]
pub fn require_acceptance(terms: &Terms, actor: &Did) -> bool {
    terms.acceptance_required && !terms.accepted_by.contains_key(actor)
}

pub fn accept(terms: &mut Terms, actor: &Did, signature: &Signature) -> Result<()> {
    if *signature.as_bytes() == [0u8; 64] {
        return Err(ForumError::TermsNotAccepted(format!("{actor}: empty signature")));
    }
    terms.accepted_by.insert(actor.clone(), Timestamp::ZERO);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn did(n: &str) -> Did { Did::new(&format!("did:exo:{n}")).unwrap() }
    fn sig() -> Signature { let mut s = [0u8; 64]; s[0] = 1; Signature::from_bytes(s) }

    #[test] fn new_terms() { let t = Terms::new(b"content", true); assert!(t.acceptance_required); assert!(t.accepted_by.is_empty()); }
    #[test] fn require_before_accept() { let t = Terms::new(b"c", true); assert!(require_acceptance(&t, &did("a"))); }
    #[test] fn not_required() { let t = Terms::new(b"c", false); assert!(!require_acceptance(&t, &did("a"))); }
    #[test] fn accept_ok() { let mut t = Terms::new(b"c", true); accept(&mut t, &did("a"), &sig()).unwrap(); assert!(!require_acceptance(&t, &did("a"))); }
    #[test] fn accept_empty_sig() { let mut t = Terms::new(b"c", true); assert!(accept(&mut t, &did("a"), &Signature::from_bytes([0u8; 64])).is_err()); }
    #[test] fn accept_multiple() { let mut t = Terms::new(b"c", true); accept(&mut t, &did("a"), &sig()).unwrap(); accept(&mut t, &did("b"), &sig()).unwrap(); assert_eq!(t.accepted_by.len(), 2); }
    #[test] fn hash_deterministic() { assert_eq!(Terms::new(b"c", true).hash, Terms::new(b"c", true).hash); }
    #[test] fn hash_differs() { assert_ne!(Terms::new(b"a", true).hash, Terms::new(b"b", true).hash); }
    #[test] fn terms_serde() { let t = Terms::new(b"c", true); let j = serde_json::to_string(&t).unwrap(); let r: Terms = serde_json::from_str(&j).unwrap(); assert_eq!(r.version, 1); }
}
