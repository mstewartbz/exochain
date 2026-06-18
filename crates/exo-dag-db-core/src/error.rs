//! Error types for ExoChain DAG DB contract helpers.

use thiserror::Error;

/// Result alias used by ExoChain DAG DB contract helpers.
pub type Result<T> = std::result::Result<T, DagDbError>;

/// Errors produced by deterministic DAG DB contract code.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DagDbError {
    /// CBOR serialization failed while building canonical hash material.
    #[error("dagdb serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::{DagDbError, Result};

    #[test]
    fn serialization_error_display_is_stable() {
        let error = DagDbError::Serialization("cbor writer failed".into());
        assert_eq!(
            error.to_string(),
            "dagdb serialization error: cbor writer failed"
        );

        let result: Result<()> = Err(error);
        assert!(matches!(result, Err(DagDbError::Serialization(_))));
    }
}
