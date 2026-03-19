//! EXOCHAIN constitutional trust fabric — legislative legitimacy.
//!
//! This crate provides governance primitives: quorum computation with
//! independence-aware counting, clearance enforcement, crosscheck verification,
//! challenge mechanisms, deliberation processes, conflict detection, and
//! hash-chained audit trails.

pub mod audit;
pub mod challenge;
pub mod clearance;
pub mod conflict;
pub mod crosscheck;
pub mod deliberation;
pub mod error;
pub mod quorum;
pub mod succession;
