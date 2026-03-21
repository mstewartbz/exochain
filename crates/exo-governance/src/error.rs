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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_all_variants() {
        let cases: Vec<(GovernanceError, &str)> = vec![
            (
                GovernanceError::QuorumNotMet {
                    reason: "short".into(),
                },
                "short",
            ),
            (
                GovernanceError::InsufficientIndependence {
                    details: "overlap".into(),
                },
                "overlap",
            ),
            (
                GovernanceError::ClearanceDenied {
                    required: "L3".into(),
                },
                "L3",
            ),
            (GovernanceError::ChallengeError("ch".into()), "ch"),
            (GovernanceError::DeliberationError("delib".into()), "delib"),
            (GovernanceError::AuditChainBroken { index: 7 }, "7"),
            (
                GovernanceError::DuplicateVote("did:exo:x".into()),
                "did:exo:x",
            ),
            (GovernanceError::DeliberationNotOpen, "not open"),
            (GovernanceError::ActionNotFound("act".into()), "act"),
            (GovernanceError::CaseNotFound("case".into()), "case"),
            (
                GovernanceError::InvalidStateTransition {
                    from: "Open".into(),
                    to: "Draft".into(),
                },
                "Open -> Draft",
            ),
        ];
        for (err, fragment) in cases {
            assert!(err.to_string().contains(fragment), "{err}");
        }
    }
}
