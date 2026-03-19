//! EXOCHAIN zero-knowledge proof system.
//!
//! This crate provides:
//! - R1CS circuit abstraction (`circuit`)
//! - SNARK proof generation/verification (`snark`)
//! - STARK proof system (`stark`)
//! - Zero-knowledge ML verification (`zkml`)
//! - Unified proof verifier (`verifier`)

pub mod circuit;
pub mod error;
pub mod snark;
pub mod stark;
pub mod verifier;
pub mod zkml;
