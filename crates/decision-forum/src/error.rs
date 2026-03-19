//! Decision forum errors.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ForumError {
    #[error("authority verification failed: {reason}")]
    AuthorityInvalid { reason: String },
    #[error("constitution not ratified: {reason}")]
    NotRatified { reason: String },
    #[error("quorum not met: required {required}, got {actual}")]
    QuorumNotMet { required: usize, actual: usize },
    #[error("decision not found: {0}")]
    DecisionNotFound(uuid::Uuid),
    #[error("terms not accepted by {0}")]
    TermsNotAccepted(String),
    #[error("amendment failed: {reason}")]
    AmendmentFailed { reason: String },
    #[error("enactment failed: {reason}")]
    EnactmentFailed { reason: String },
}
pub type Result<T> = std::result::Result<T, ForumError>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn all_display() {
        let es: Vec<ForumError> = vec![
            ForumError::AuthorityInvalid{reason:"x".into()}, ForumError::NotRatified{reason:"x".into()},
            ForumError::QuorumNotMet{required:3,actual:1}, ForumError::DecisionNotFound(uuid::Uuid::nil()),
            ForumError::TermsNotAccepted("x".into()), ForumError::AmendmentFailed{reason:"x".into()},
            ForumError::EnactmentFailed{reason:"x".into()},
        ];
        for e in &es { assert!(!e.to_string().is_empty()); }
    }
}
