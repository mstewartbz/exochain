//! EXOCHAIN Consent Enforcement
//!
//! Bailment-conditioned consent fabric. No action without consent.
//! Default posture: DENY.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod bailment;
pub mod contract;
pub mod error;
pub mod gatekeeper;
pub mod policy;

pub use bailment::{Bailment, BailmentStatus, BailmentType};
pub use error::ConsentError;
pub use gatekeeper::ConsentGate;
pub use policy::{ConsentDecision, ConsentPolicy, ConsentRequirement, PolicyEngine};
