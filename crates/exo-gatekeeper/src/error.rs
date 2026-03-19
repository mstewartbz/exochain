//! Gatekeeper-specific errors.

use thiserror::Error;

/// Top-level error type for the gatekeeper crate.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GatekeeperError {
    #[error("kernel integrity check failed: expected {expected}, got {actual}")]
    KernelIntegrityFailure { expected: String, actual: String },

    #[error("invariant violation: {0}")]
    InvariantViolation(String),

    #[error("combinator reduction failed: {0}")]
    CombinatorError(String),

    #[error("holon error: {0}")]
    HolonError(String),

    #[error("MCP violation: {0}")]
    McpViolation(String),

    #[error("TEE attestation failed: {0}")]
    TeeError(String),

    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    #[error("timeout after {0} ms")]
    Timeout(u64),

    #[error("checkpoint error: {0}")]
    CheckpointError(String),

    #[error("core error: {0}")]
    Core(String),
}

impl From<exo_core::ExoError> for GatekeeperError {
    fn from(e: exo_core::ExoError) -> Self {
        GatekeeperError::Core(e.to_string())
    }
}
