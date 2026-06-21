//! Import, export, and writeback contracts for DAG DB.
//!
//! Facade removal is complete; this crate owns KG import, portable export,
//! writeback proposal, hygiene, and drift-repair contracts. Downstream
//! compatibility uses explicit bridges such as `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub use exo_dag_db_core::{error, hash, metadata, similarity, tenant};
pub use exo_dag_db_domain::{graph, model, placement, scoring};
pub use exo_dag_db_graph::{layer_creation_policy, layered_hygiene, layered_placement};
pub use exo_dag_db_retrieval::kg_retrieval;

pub mod import_drift_repair;
pub mod kg_export;
pub mod kg_import;
pub mod kg_writeback;
pub mod kg_writeback_hygiene;
