//! Governed pure domain services for DAG DB.
//!
//! Facade removal is complete; this crate owns domain models, gates, route and
//! context-packet services, validation, placement, writeback, and lifecycle
//! contracts. Downstream compatibility uses explicit bridges such as
//! `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub use exo_dag_db_core::{error, hash, metadata, similarity, tenant};
pub use exo_dag_db_graph::{
    layer_creation_policy, layered_graph, layered_hygiene, layered_placement,
};

pub mod canonicalization;
pub mod context;
pub mod context_packet_persistence;
pub mod continuation_packet;
pub mod continuation_persistence;
pub mod council;
pub mod default_route;
pub mod export_finality;
pub mod graph;
pub mod intake;
pub mod lifecycle_action;
pub mod model;
pub mod placement;
pub mod route;
pub mod route_invalidation;
pub mod scoring;
pub mod state;
pub mod trust;
pub mod validation;
pub mod writeback;

pub use scoring::{DomainError, DomainResult};
