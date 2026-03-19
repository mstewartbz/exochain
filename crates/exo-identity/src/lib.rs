//! EXOCHAIN constitutional trust fabric — privacy-preserving identity adjudication.
//!
//! This crate provides:
//!
//! - **DID management** (`did`) — Decentralized Identity documents, registration, revocation, key rotation
//! - **Risk attestation** (`risk`) — Signed risk assessments with expiry and policy enforcement
//! - **Shamir secret sharing** (`shamir`) — Sybil-defense secret splitting over GF(256)
//! - **PACE operator continuity** (`pace`) — Primary/Alternate/Contingency/Emergency escalation
//! - **Key management** (`key_management`) — Key lifecycle tracking: create, rotate, revoke

pub mod did;
pub mod error;
pub mod key_management;
pub mod pace;
pub mod risk;
pub mod shamir;
pub mod vault;
