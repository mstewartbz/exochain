//! SDK error types.
//!
//! All SDK operations that can fail return [`ExoResult<T>`], which is an alias
//! for [`std::result::Result<T, ExoError>`]. Each variant narrows the error to
//! a specific subsystem to aid programmatic handling.

use thiserror::Error;

/// Errors that can be produced by the SDK.
#[derive(Debug, Error)]
pub enum ExoError {
    /// An identity-related operation failed (e.g. DID derivation or key handling).
    #[error("identity error: {0}")]
    Identity(String),
    /// A consent/bailment-related operation failed.
    #[error("consent error: {0}")]
    Consent(String),
    /// A governance-related operation failed (e.g. voting, quorum).
    #[error("governance error: {0}")]
    Governance(String),
    /// An authority-chain operation failed (e.g. invalid topology).
    #[error("authority error: {0}")]
    Authority(String),
    /// The kernel denied an action.
    #[error("kernel denied: {0}")]
    KernelDenied(String),
    /// The kernel escalated an action for review.
    #[error("kernel escalated: {0}")]
    KernelEscalated(String),
    /// A cryptographic operation failed.
    #[error("crypto error: {0}")]
    Crypto(String),
    /// A provided DID string is not valid.
    #[error("invalid DID: {0}")]
    InvalidDid(String),
    /// Serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Convenience alias for `Result<T, ExoError>`.
pub type ExoResult<T> = std::result::Result<T, ExoError>;
