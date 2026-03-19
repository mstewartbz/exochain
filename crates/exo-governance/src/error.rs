//! Governance error types.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GovernanceError {
    #[error("quorum not met: {reason}")]
    QuorumNotMet { reason: String },

    #[error("insufficient independence: {details}")]
    InsufficientIndependence { details: String },

    #[error("clearance denied: actor lacks {required} level")]
    ClearanceDenied { required: String },

    #[error("challenge error: {0}")]
    ChallengeError(String),

    #[error("deliberation error: {0}")]
    DeliberationError(String),

    #[error("audit chain broken at index {index}")]
    AuditChainBroken { index: usize },

    #[error("duplicate vote from {0}")]
    DuplicateVote(String),

    #[error("deliberation not open")]
    DeliberationNotOpen,

    #[error("action not found: {0}")]
    ActionNotFound(String),

    #[error("case not found: {0}")]
    CaseNotFound(String),

    #[error("invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
}
