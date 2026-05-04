//! SDK re-exports for the Autonomous Volition Credential layer.
//!
//! See the `exo-avc` crate documentation for the determinism contract,
//! validation rules, delegation invariants, and signing domain tags.
//!
//! ```
//! use exochain_sdk::avc::{
//!     AVC_SCHEMA_VERSION, AVC_CREDENTIAL_SIGNING_DOMAIN, AvcDecision, AvcReasonCode,
//! };
//! assert_eq!(AVC_SCHEMA_VERSION, 1);
//! assert!(AVC_CREDENTIAL_SIGNING_DOMAIN.contains(".v1"));
//! assert_ne!(AvcDecision::Allow, AvcDecision::Deny);
//! assert_ne!(AvcReasonCode::Valid, AvcReasonCode::Expired);
//! ```
//!
//! See `exo-avc`'s crate-level doctest for a full issue → validate flow.

pub use exo_avc::{
    AVC_CREDENTIAL_SIGNING_DOMAIN, AVC_RECEIPT_SIGNING_DOMAIN, AVC_REVOCATION_SIGNING_DOMAIN,
    AVC_SCHEMA_VERSION, AVC_SIGNING_DOMAINS, AuthorityChainRef, AuthorityScope,
    AutonomousVolitionCredential, AutonomyLevel, AvcActionRequest, AvcConstraints, AvcDecision,
    AvcDraft, AvcError, AvcReasonCode, AvcRegistryRead, AvcRegistryWrite, AvcRevocation,
    AvcRevocationReason, AvcSubjectKind, AvcTrustReceipt, AvcValidationRequest,
    AvcValidationResult, ConsentRef, DataClass, DelegatedIntent, InMemoryAvcRegistry,
    MAX_BASIS_POINTS, PolicyRef, TimeWindow, create_trust_receipt, delegate_avc, issue_avc,
    parent_id_of, revoke_avc, validate_avc,
};
