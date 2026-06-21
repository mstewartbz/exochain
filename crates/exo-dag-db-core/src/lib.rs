//! Deterministic primitives for DAG DB.
//!
//! Facade removal is complete; this crate owns hash material, safe metadata,
//! tenant identity, similarity, and shared validation primitives. Downstream
//! compatibility uses explicit bridges such as `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod error;
pub mod hash;
pub mod metadata;
pub mod similarity;
pub mod tenant;

pub use error::{DagDbError, Result};
