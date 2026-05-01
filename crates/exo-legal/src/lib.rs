//! EXOCHAIN constitutional trust fabric — litigation-grade evidence,
//! eDiscovery, privilege assertions, fiduciary duty tracking, records
//! management, and conflict-of-interest disclosure.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod ai_transparency;
pub mod bundle;
pub mod cert_902_11;
pub mod compliance_report;
pub mod conflict_disclosure;
pub mod dgcl144;
pub mod ediscovery;
pub mod error;
pub mod evidence;
pub mod fiduciary;
#[cfg(test)]
mod nist_compliance_tests;
pub mod nist_mapping;
pub mod privilege;
pub mod records;
