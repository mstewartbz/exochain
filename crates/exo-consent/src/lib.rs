//! EXOCHAIN Consent Enforcement
//!
//! Bailment-conditioned consent fabric. No action without consent.
//! Default posture: DENY.

pub mod bailment;
pub mod error;
pub mod gatekeeper;
pub mod policy;

pub use bailment::{Bailment, BailmentStatus, BailmentType};
pub use error::ConsentError;
pub use gatekeeper::ConsentGate;
pub use policy::{ConsentDecision, ConsentPolicy, ConsentRequirement, PolicyEngine};
