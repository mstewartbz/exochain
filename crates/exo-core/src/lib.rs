//! # exo-core
//!
//! Foundational crate for the EXOCHAIN constitutional trust fabric.
//!
//! **Determinism contract**: this crate enforces absolute determinism.
//! - No floating-point arithmetic.
//! - `BTreeMap` only — `HashMap` is never exposed.
//! - Canonical CBOR serialization for all hashing.
//! - Hybrid Logical Clock for causal ordering.
//!
//! All other EXOCHAIN crates depend on `exo-core`.

pub mod bcts;
pub mod crypto;
pub mod error;
pub mod events;
pub mod hash;
pub mod hlc;
pub mod invariants;
pub mod types;

// Re-export the most commonly used items at crate root for ergonomics.
pub use error::{ExoError, Result};
pub use types::{
    CorrelationId, DeterministicMap, Did, Hash256, PqPublicKey, PqSecretKey, PublicKey,
    SIGNER_PREFIX_AI, SIGNER_PREFIX_HUMAN, SecretKey, Signature, SignerType, Timestamp, Version,
};
