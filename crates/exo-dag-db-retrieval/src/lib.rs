//! Retrieval, routing, and context packet selection for DAG DB.
//!
//! Facade removal is complete; this crate owns graph context selection, hybrid
//! retrieval, catalog routing, packet output, and read-side query/view helpers.
//! Downstream compatibility uses explicit bridges such as `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub use exo_dag_db_core::{error, hash, metadata, similarity, tenant};
pub use exo_dag_db_domain::{canonicalization, graph, scoring};
pub use exo_dag_db_graph::{
    layer_creation_policy, layered_graph, layered_hygiene, layered_placement,
};

pub mod citation_locator;
pub mod context_packet_output;
pub mod graph_context_selection;
pub mod hybrid_retrieval;
pub mod kg_catalog_router;
pub mod kg_retrieval;
pub mod layered_drilldown;
pub mod query;
pub mod source_adapter;
pub mod views;

pub use graph_context_selection::{
    GraphContextMemoryCandidate, GraphContextSelectionState, select_graph_context,
};
