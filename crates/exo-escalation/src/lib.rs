//! EXOCHAIN constitutional trust fabric — operational nervous system.
//!
//! Detection, triage, escalation (including Sybil adjudication), kanban,
//! feedback loops, and completeness checking.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod challenge;
pub mod completeness;
pub mod detector;
pub mod error;
pub mod escalation;
pub mod feedback;
pub mod kanban;
pub mod triage;
