//! Error type for root genesis authority operations.

use thiserror::Error;

/// Result alias used by the root genesis crate.
pub type Result<T> = core::result::Result<T, RootError>;

/// Failures returned by root genesis ceremony, DKG, signing, portal, and share
/// protection operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RootError {
    /// Ceremony policy or roster validation failed.
    #[error("ceremony configuration rejected: {reason}")]
    InvalidConfig { reason: String },

    /// Canonical CBOR encoding failed before hashing or signing.
    #[error("canonical encoding failed: {detail}")]
    CanonicalEncoding { detail: String },

    /// FROST DKG or threshold signing failed.
    #[error("frost operation failed: {detail}")]
    Frost { detail: String },

    /// The supplied signer set does not satisfy the configured threshold.
    #[error("threshold not met: required {required}, supplied {supplied}")]
    ThresholdNotMet { required: u16, supplied: u16 },

    /// A root signature or certifier envelope signature did not verify.
    #[error("signature verification failed: {reason}")]
    SignatureRejected { reason: String },

    /// Root trust bundle contents are inconsistent with their signature or ID.
    #[error("root bundle rejected: {reason}")]
    BundleRejected { reason: String },

    /// Portal relay policy rejected an envelope.
    #[error("portal envelope rejected: {reason}")]
    PortalRejected { reason: String },

    /// Share sealing, opening, or pairwise payload protection failed.
    #[error("share protection failed: {reason}")]
    ProtectionFailed { reason: String },
}
