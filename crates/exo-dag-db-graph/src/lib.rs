//! Graph and layered organization for DAG DB.
//!
//! Facade removal is complete; this crate owns memory graph invariants, layered
//! placement, hygiene, and layer policy code. Downstream compatibility uses
//! explicit bridges such as `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod layer_creation_policy;
pub mod layered_graph;
pub mod layered_hygiene;
pub mod layered_placement;
