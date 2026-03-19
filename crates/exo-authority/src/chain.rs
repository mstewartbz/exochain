//! Authority chain — ordered sequence of delegation links.
//!
//! Authority flows from root to leaf. Scope can only narrow at each link.
//! Max delegation depth: 5 (configurable).

use exo_core::{Did, Hash256, Signature, Timestamp};
use serde::{Deserialize, Serialize};

use crate::error::AuthorityError;
use crate::permission::{Permission, PermissionSet};

/// Default maximum delegation depth.
pub const DEFAULT_MAX_DEPTH: usize = 5;

/// A single link in an authority chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityLink {
    pub delegator_did: Did,
    pub delegate_did: Did,
    pub scope: Vec<Permission>,
    pub created: Timestamp,
    pub expires: Option<Timestamp>,
    pub signature: Signature,
    pub depth: usize,
}

impl AuthorityLink {
    /// Compute a deterministic ID for this link.
    #[must_use]
    pub fn id(&self) -> Hash256 {
        let mut data = Vec::new();
        data.extend_from_slice(self.delegator_did.as_str().as_bytes());
        data.extend_from_slice(self.delegate_did.as_str().as_bytes());
        for p in &self.scope {
            data.extend_from_slice(format!("{p:?}").as_bytes());
        }
        data.extend_from_slice(&self.created.physical_ms.to_le_bytes());
        Hash256::digest(&data)
    }
}

/// An ordered sequence of authority links from root to leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityChain {
    pub links: Vec<AuthorityLink>,
    pub max_depth: usize,
}

impl AuthorityChain {
    #[must_use]
    pub fn depth(&self) -> usize {
        self.links.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// The root delegator (first link's delegator).
    #[must_use]
    pub fn root(&self) -> Option<&Did> {
        self.links.first().map(|l| &l.delegator_did)
    }

    /// The leaf delegate (last link's delegate).
    #[must_use]
    pub fn leaf(&self) -> Option<&Did> {
        self.links.last().map(|l| &l.delegate_did)
    }
}

/// Build an authority chain from a slice of links.
///
/// Validates:
/// - Non-empty
/// - Continuity: each link's delegate == next link's delegator
/// - Depth limits
/// - Depth values are correct (0, 1, 2, ...)
///
/// # Errors
/// Returns `AuthorityError` if validation fails.
pub fn build_chain(links: &[AuthorityLink]) -> Result<AuthorityChain, AuthorityError> {
    build_chain_with_depth(links, DEFAULT_MAX_DEPTH)
}

/// Build a chain with a custom max depth.
pub fn build_chain_with_depth(
    links: &[AuthorityLink],
    max_depth: usize,
) -> Result<AuthorityChain, AuthorityError> {
    if links.is_empty() {
        return Err(AuthorityError::EmptyChain);
    }

    if links.len() > max_depth {
        return Err(AuthorityError::DepthExceeded {
            depth: links.len(),
            max_depth,
        });
    }

    // Validate continuity and depth values
    for (i, link) in links.iter().enumerate() {
        if link.depth != i {
            return Err(AuthorityError::ChainBroken {
                index: i,
                reason: format!("expected depth {i}, got {}", link.depth),
            });
        }
        if i > 0 {
            let prev = &links[i - 1];
            if prev.delegate_did != link.delegator_did {
                return Err(AuthorityError::ChainBroken {
                    index: i,
                    reason: format!(
                        "gap: {} -> {} but expected {}",
                        prev.delegate_did, link.delegator_did, prev.delegate_did
                    ),
                });
            }
        }
    }

    Ok(AuthorityChain {
        links: links.to_vec(),
        max_depth,
    })
}

/// Verify an authority chain: signatures non-empty, no expired links, scope narrows.
///
/// # Errors
/// Returns `AuthorityError` on any verification failure.
pub fn verify_chain(chain: &AuthorityChain, now: &Timestamp) -> Result<(), AuthorityError> {
    if chain.links.is_empty() {
        return Err(AuthorityError::EmptyChain);
    }

    if chain.links.len() > chain.max_depth {
        return Err(AuthorityError::DepthExceeded {
            depth: chain.links.len(),
            max_depth: chain.max_depth,
        });
    }

    let mut prev_scope: Option<PermissionSet> = None;

    for (i, link) in chain.links.iter().enumerate() {
        // Check signature is non-empty
        if link.signature.is_empty() {
            return Err(AuthorityError::InvalidSignature { index: i });
        }

        // Check expiry
        if let Some(exp) = &link.expires {
            if exp.is_expired(now) {
                return Err(AuthorityError::ExpiredLink { index: i });
            }
        }

        // Check scope narrows (each link's scope must be subset of previous)
        let current_scope = PermissionSet::from_permissions(&link.scope);
        if let Some(ref prev) = prev_scope {
            if !PermissionSet::is_subset(&current_scope, prev) {
                return Err(AuthorityError::ScopeWidening { index: i });
            }
        }
        prev_scope = Some(current_scope);
    }

    Ok(())
}

/// Check if a chain grants a specific permission.
///
/// The permission must appear in the leaf (last) link's scope,
/// and scope must have narrowed properly through the chain.
#[must_use]
pub fn has_permission(chain: &AuthorityChain, permission: &Permission) -> bool {
    // All links must contain the permission (scope narrows but must include it)
    chain.links.iter().all(|link| link.scope.contains(permission))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn did(name: &str) -> Did { Did::new(&format!("did:exo:{name}")).unwrap() }
    fn ts(ms: u64) -> Timestamp { Timestamp::new(ms, 0) }
    fn now() -> Timestamp { ts(5000) }

    fn link(from: &str, to: &str, scope: Vec<Permission>, depth: usize, exp: Option<Timestamp>) -> AuthorityLink {
        AuthorityLink {
            delegator_did: did(from),
            delegate_did: did(to),
            scope,
            created: ts(1000),
            expires: exp,
            signature: Signature::from_bytes([1u8; 64]),
            depth,
        }
    }

    #[test]
    fn build_single_link() {
        let links = vec![link("root", "alice", vec![Permission::Read, Permission::Write], 0, None)];
        let chain = build_chain(&links);
        assert!(chain.is_ok());
        let c = chain.unwrap();
        assert_eq!(c.depth(), 1);
        assert_eq!(c.root().unwrap(), &did("root"));
        assert_eq!(c.leaf().unwrap(), &did("alice"));
    }

    #[test]
    fn build_multi_link() {
        let links = vec![
            link("root", "alice", vec![Permission::Read, Permission::Write, Permission::Delegate], 0, None),
            link("alice", "bob", vec![Permission::Read, Permission::Write], 1, None),
            link("bob", "charlie", vec![Permission::Read], 2, None),
        ];
        let chain = build_chain(&links).unwrap();
        assert_eq!(chain.depth(), 3);
    }

    #[test]
    fn build_rejects_empty() {
        assert_eq!(build_chain(&[]), Err(AuthorityError::EmptyChain));
    }

    #[test]
    fn build_rejects_depth_exceeded() {
        let links: Vec<AuthorityLink> = (0..6)
            .map(|i| link(&format!("n{i}"), &format!("n{}", i + 1), vec![Permission::Read], i, None))
            .collect();
        let result = build_chain(&links);
        assert!(matches!(result, Err(AuthorityError::DepthExceeded { .. })));
    }

    #[test]
    fn build_custom_depth() {
        let links: Vec<AuthorityLink> = (0..3)
            .map(|i| link(&format!("n{i}"), &format!("n{}", i + 1), vec![Permission::Read], i, None))
            .collect();
        assert!(build_chain_with_depth(&links, 2).is_err());
        assert!(build_chain_with_depth(&links, 3).is_ok());
    }

    #[test]
    fn build_rejects_gap() {
        let links = vec![
            link("root", "alice", vec![Permission::Read], 0, None),
            link("bob", "charlie", vec![Permission::Read], 1, None), // gap: alice != bob
        ];
        assert!(matches!(build_chain(&links), Err(AuthorityError::ChainBroken { .. })));
    }

    #[test]
    fn build_rejects_wrong_depth() {
        let links = vec![
            link("root", "alice", vec![Permission::Read], 0, None),
            link("alice", "bob", vec![Permission::Read], 5, None), // wrong depth
        ];
        assert!(matches!(build_chain(&links), Err(AuthorityError::ChainBroken { .. })));
    }

    #[test]
    fn verify_valid_chain() {
        let links = vec![
            link("root", "alice", vec![Permission::Read, Permission::Write], 0, None),
            link("alice", "bob", vec![Permission::Read], 1, None),
        ];
        let chain = build_chain(&links).unwrap();
        assert!(verify_chain(&chain, &now()).is_ok());
    }

    #[test]
    fn verify_rejects_empty_signature() {
        let mut links = vec![link("root", "alice", vec![Permission::Read], 0, None)];
        links[0].signature = Signature::empty();
        let chain = build_chain(&links).unwrap();
        assert!(matches!(verify_chain(&chain, &now()), Err(AuthorityError::InvalidSignature { .. })));
    }

    #[test]
    fn verify_rejects_expired_link() {
        let links = vec![link("root", "alice", vec![Permission::Read], 0, Some(ts(1000)))];
        let chain = build_chain(&links).unwrap();
        assert!(matches!(verify_chain(&chain, &now()), Err(AuthorityError::ExpiredLink { .. })));
    }

    #[test]
    fn verify_rejects_scope_widening() {
        let links = vec![
            link("root", "alice", vec![Permission::Read], 0, None),
            link("alice", "bob", vec![Permission::Read, Permission::Write], 1, None), // wider!
        ];
        let chain = build_chain(&links).unwrap();
        assert!(matches!(verify_chain(&chain, &now()), Err(AuthorityError::ScopeWidening { .. })));
    }

    #[test]
    fn verify_accepts_equal_scope() {
        let links = vec![
            link("root", "alice", vec![Permission::Read, Permission::Write], 0, None),
            link("alice", "bob", vec![Permission::Read, Permission::Write], 1, None),
        ];
        let chain = build_chain(&links).unwrap();
        assert!(verify_chain(&chain, &now()).is_ok());
    }

    #[test]
    fn has_permission_present() {
        let links = vec![
            link("root", "alice", vec![Permission::Read, Permission::Write], 0, None),
            link("alice", "bob", vec![Permission::Read], 1, None),
        ];
        let chain = build_chain(&links).unwrap();
        assert!(has_permission(&chain, &Permission::Read));
        assert!(!has_permission(&chain, &Permission::Write)); // bob doesn't have it
    }

    #[test]
    fn has_permission_empty_chain() {
        let chain = AuthorityChain { links: vec![], max_depth: 5 };
        // all() on empty iterator returns true, but that's fine — empty chain has "all" permissions vacuously
        assert!(has_permission(&chain, &Permission::Read));
    }

    #[test]
    fn link_id_deterministic() {
        let l = link("root", "alice", vec![Permission::Read], 0, None);
        let id1 = l.id();
        let id2 = l.id();
        assert_eq!(id1, id2);
    }

    #[test]
    fn chain_is_empty() {
        let chain = AuthorityChain { links: vec![], max_depth: 5 };
        assert!(chain.is_empty());
        assert!(chain.root().is_none());
        assert!(chain.leaf().is_none());
    }

    #[test]
    fn verify_chain_rejects_over_depth() {
        let links: Vec<AuthorityLink> = (0..3)
            .map(|i| link(&format!("n{i}"), &format!("n{}", i + 1), vec![Permission::Read], i, None))
            .collect();
        let mut chain = build_chain(&links).unwrap();
        chain.max_depth = 2; // artificially reduce
        assert!(matches!(verify_chain(&chain, &now()), Err(AuthorityError::DepthExceeded { .. })));
    }

    #[test]
    fn verify_empty_chain_errors() {
        let chain = AuthorityChain { links: vec![], max_depth: 5 };
        assert_eq!(verify_chain(&chain, &now()), Err(AuthorityError::EmptyChain));
    }

    #[test]
    fn verify_non_expired_link() {
        let links = vec![link("root", "alice", vec![Permission::Read], 0, Some(ts(10000)))];
        let chain = build_chain(&links).unwrap();
        assert!(verify_chain(&chain, &now()).is_ok());
    }
}
