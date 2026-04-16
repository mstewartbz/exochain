//! EXOCHAIN SDK — ergonomic Rust API for the constitutional governance fabric.
//!
//! This crate re-exports and wraps the underlying `exo-*` workspace crates behind
//! a single, developer-friendly surface so that applications built on EXOCHAIN
//! can depend on one crate and use one coherent API.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use exochain_sdk::prelude::*;
//!
//! // Create an identity
//! let identity = Identity::generate("my-agent");
//! println!("DID: {}", identity.did());
//!
//! // Verify a signature
//! let sig = identity.sign(b"hello");
//! assert!(identity.verify(b"hello", &sig));
//! ```

pub mod authority;
pub mod consent;
pub mod crypto;
pub mod error;
pub mod governance;
pub mod identity;
pub mod kernel;

/// Prelude — import everything you need with `use exochain_sdk::prelude::*`.
pub mod prelude {
    pub use crate::authority::AuthorityChainBuilder;
    pub use crate::consent::BailmentBuilder;
    pub use crate::crypto::{hash, sign, verify};
    pub use crate::error::{ExoError, ExoResult};
    pub use crate::governance::{Decision, DecisionBuilder, Vote};
    pub use crate::identity::Identity;
    pub use crate::kernel::ConstitutionalKernel;
}
