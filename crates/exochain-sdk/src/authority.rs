//! Authority delegation and chain verification.
//!
//! An **authority chain** is an ordered list of delegation links where the
//! `grantee` of each link is the `grantor` of the next. The chain terminates
//! at a specific actor (the "terminal"). [`AuthorityChainBuilder`] accumulates
//! links and validates the resulting topology on
//! [`AuthorityChainBuilder::build`]. Structurally:
//!
//! ```text
//! root --[perms]--> middle --[perms]--> leaf
//! ```
//!
//! ## Why use this module
//!
//! - You need to prove that a terminal actor was delegated authority through
//!   a well-formed chain rooted at a specific principal.
//! - You want the SDK to reject broken topologies (gaps, wrong terminal,
//!   empty chain) before you ever submit them to the kernel.
//! - You want the validated chain as a serializable artifact that survives
//!   network hops and storage.
//!
//! ## Quick start
//!
//! ```
//! use exochain_sdk::authority::AuthorityChainBuilder;
//! use exo_core::Did;
//!
//! let root = Did::new("did:exo:root").expect("valid");
//! let mid = Did::new("did:exo:mid").expect("valid");
//! let leaf = Did::new("did:exo:leaf").expect("valid");
//!
//! let chain = AuthorityChainBuilder::new()
//!     .add_link(root, mid.clone(), vec!["delegate".into()])
//!     .add_link(mid, leaf.clone(), vec!["read".into()])
//!     .build(&leaf)?;
//!
//! assert_eq!(chain.depth, 2);
//! assert_eq!(chain.terminal, leaf);
//! # Ok::<(), exochain_sdk::error::ExoError>(())
//! ```

use exo_core::Did;
use serde::{Deserialize, Serialize};

use crate::error::{ExoError, ExoResult};

/// Builder for a validated authority chain.
///
/// Links are appended one at a time with [`AuthorityChainBuilder::add_link`]
/// and the full chain is validated by [`AuthorityChainBuilder::build`]. The
/// builder is `Default` and `Clone`, so a partially-built chain can be forked.
///
/// # Examples
///
/// ```
/// use exochain_sdk::authority::AuthorityChainBuilder;
/// use exo_core::Did;
///
/// let root = Did::new("did:exo:root").expect("valid");
/// let leaf = Did::new("did:exo:leaf").expect("valid");
/// let chain = AuthorityChainBuilder::new()
///     .add_link(root, leaf.clone(), vec!["all".into()])
///     .build(&leaf)?;
/// assert_eq!(chain.depth, 1);
/// # Ok::<(), exochain_sdk::error::ExoError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct AuthorityChainBuilder {
    links: Vec<ChainLink>,
}

impl AuthorityChainBuilder {
    /// Construct an empty builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use exochain_sdk::authority::AuthorityChainBuilder;
    /// let builder = AuthorityChainBuilder::new();
    /// # let _ = builder;
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Append a new delegation link.
    ///
    /// Each link records that `grantor` has delegated `permissions` to
    /// `grantee`. Permission strings are opaque to the SDK; downstream
    /// consumers are free to interpret them.
    ///
    /// # Examples
    ///
    /// ```
    /// # use exochain_sdk::authority::AuthorityChainBuilder;
    /// # use exo_core::Did;
    /// let a = Did::new("did:exo:a").expect("valid");
    /// let b = Did::new("did:exo:b").expect("valid");
    /// let builder = AuthorityChainBuilder::new()
    ///     .add_link(a, b, vec!["read".into(), "write".into()]);
    /// # let _ = builder;
    /// ```
    #[must_use]
    pub fn add_link(mut self, grantor: Did, grantee: Did, permissions: Vec<String>) -> Self {
        self.links.push(ChainLink {
            grantor,
            grantee,
            permissions,
        });
        self
    }

    /// Validate the chain topology and produce a [`ValidatedChain`].
    ///
    /// Validation rules:
    /// - The chain must contain at least one link.
    /// - For each consecutive pair of links,
    ///   `links[i].grantee == links[i+1].grantor`.
    /// - The final link's `grantee` must equal `terminal_actor`.
    ///
    /// # Errors
    ///
    /// Returns [`ExoError::Authority`] if any rule is violated: empty chain,
    /// a broken delegation between consecutive links, or a mismatch between
    /// the terminal link's grantee and `terminal_actor`.
    ///
    /// # Examples
    ///
    /// A valid three-link chain:
    ///
    /// ```
    /// use exochain_sdk::authority::AuthorityChainBuilder;
    /// use exo_core::Did;
    ///
    /// let root = Did::new("did:exo:root").expect("valid");
    /// let mid = Did::new("did:exo:mid").expect("valid");
    /// let leaf = Did::new("did:exo:leaf").expect("valid");
    /// let chain = AuthorityChainBuilder::new()
    ///     .add_link(root, mid.clone(), vec!["delegate".into()])
    ///     .add_link(mid, leaf.clone(), vec!["read".into()])
    ///     .build(&leaf)?;
    /// assert_eq!(chain.depth, 2);
    /// # Ok::<(), exochain_sdk::error::ExoError>(())
    /// ```
    ///
    /// A broken chain (middle grantee does not match next grantor):
    ///
    /// ```
    /// use exochain_sdk::authority::AuthorityChainBuilder;
    /// use exochain_sdk::error::ExoError;
    /// use exo_core::Did;
    ///
    /// let root = Did::new("did:exo:root").expect("valid");
    /// let mid = Did::new("did:exo:mid").expect("valid");
    /// let other = Did::new("did:exo:other").expect("valid");
    /// let leaf = Did::new("did:exo:leaf").expect("valid");
    /// let err = AuthorityChainBuilder::new()
    ///     .add_link(root, mid, vec!["read".into()])
    ///     .add_link(other, leaf.clone(), vec!["read".into()])
    ///     .build(&leaf)
    ///     .unwrap_err();
    /// assert!(matches!(err, ExoError::Authority(_)));
    /// ```
    pub fn build(self, terminal_actor: &Did) -> ExoResult<ValidatedChain> {
        if self.links.is_empty() {
            return Err(ExoError::Authority("authority chain is empty".into()));
        }

        // Each grantee must match the next grantor.
        for window in self.links.windows(2) {
            let a = &window[0];
            let b = &window[1];
            if a.grantee != b.grantor {
                return Err(ExoError::Authority(format!(
                    "broken delegation: {} != {}",
                    a.grantee, b.grantor
                )));
            }
        }

        // The terminal grantee must equal the claimed terminal actor.
        let last = self
            .links
            .last()
            .ok_or_else(|| ExoError::Authority("authority chain is empty".into()))?;
        if &last.grantee != terminal_actor {
            return Err(ExoError::Authority(format!(
                "terminal mismatch: chain ends at {} but terminal_actor is {}",
                last.grantee, terminal_actor
            )));
        }

        let depth = self.links.len();
        Ok(ValidatedChain {
            depth,
            links: self.links,
            terminal: terminal_actor.clone(),
        })
    }
}

/// A single delegation link in an authority chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainLink {
    /// The delegating authority.
    pub grantor: Did,
    /// The recipient of the delegation.
    pub grantee: Did,
    /// Permissions delegated at this link.
    pub permissions: Vec<String>,
}

/// A validated authority chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedChain {
    /// Number of links in the chain.
    pub depth: usize,
    /// The chain of delegation links, root-first.
    pub links: Vec<ChainLink>,
    /// The terminal actor the chain delegates authority to.
    pub terminal: Did,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn did(s: &str) -> Did {
        Did::new(s).expect("valid DID")
    }

    #[test]
    fn valid_chain_passes() {
        let root = did("did:exo:root");
        let mid = did("did:exo:mid");
        let leaf = did("did:exo:leaf");
        let chain = AuthorityChainBuilder::new()
            .add_link(root.clone(), mid.clone(), vec!["read".into()])
            .add_link(mid.clone(), leaf.clone(), vec!["read".into()])
            .build(&leaf)
            .expect("valid");
        assert_eq!(chain.depth, 2);
        assert_eq!(chain.terminal, leaf);
        assert_eq!(chain.links[0].grantor, root);
        assert_eq!(chain.links[1].grantee, leaf);
    }

    #[test]
    fn single_link_chain_passes() {
        let root = did("did:exo:root");
        let leaf = did("did:exo:leaf");
        let chain = AuthorityChainBuilder::new()
            .add_link(root, leaf.clone(), vec!["all".into()])
            .build(&leaf)
            .expect("valid");
        assert_eq!(chain.depth, 1);
    }

    #[test]
    fn empty_chain_fails() {
        let leaf = did("did:exo:leaf");
        let err = AuthorityChainBuilder::new().build(&leaf).unwrap_err();
        assert!(matches!(err, ExoError::Authority(_)));
    }

    #[test]
    fn broken_chain_fails() {
        let root = did("did:exo:root");
        let mid = did("did:exo:mid");
        let other = did("did:exo:other");
        let leaf = did("did:exo:leaf");
        let err = AuthorityChainBuilder::new()
            .add_link(root, mid, vec!["read".into()])
            .add_link(other, leaf.clone(), vec!["read".into()])
            .build(&leaf)
            .unwrap_err();
        assert!(matches!(err, ExoError::Authority(_)));
    }

    #[test]
    fn wrong_terminal_fails() {
        let root = did("did:exo:root");
        let mid = did("did:exo:mid");
        let leaf = did("did:exo:leaf");
        let claimed = did("did:exo:claimed");
        let err = AuthorityChainBuilder::new()
            .add_link(root, mid.clone(), vec!["read".into()])
            .add_link(mid, leaf, vec!["read".into()])
            .build(&claimed)
            .unwrap_err();
        assert!(matches!(err, ExoError::Authority(_)));
    }

    #[test]
    fn validated_chain_serde_roundtrip() {
        let root = did("did:exo:root");
        let leaf = did("did:exo:leaf");
        let chain = AuthorityChainBuilder::new()
            .add_link(root, leaf.clone(), vec!["read".into(), "write".into()])
            .build(&leaf)
            .expect("ok");
        let json = serde_json::to_string(&chain).expect("serialize");
        let decoded: ValidatedChain = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(chain, decoded);
    }

    #[test]
    fn default_builder_is_empty() {
        let b = AuthorityChainBuilder::default();
        assert!(b.links.is_empty());
    }
}
