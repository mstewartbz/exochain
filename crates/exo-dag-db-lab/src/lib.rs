//! Diagnostics, graph explorer, benchmarks, and lab tools for DAG DB.
//!
//! Facade removal is complete; this crate owns non-product-path diagnostics,
//! graph-explorer artifacts, benchmark runners, browser artifacts, binaries,
//! and benches. Downstream compatibility uses explicit bridges such as
//! `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub use exo_dag_db_core::{error, hash, metadata, similarity, tenant};
pub use exo_dag_db_domain::{
    canonicalization, context, graph, model, placement, route, scoring, state, trust, validation,
};
pub use exo_dag_db_exchange::{kg_export, kg_import, kg_writeback, kg_writeback_hygiene};
pub use exo_dag_db_graph::{
    layer_creation_policy, layered_graph, layered_hygiene, layered_placement,
};
pub use exo_dag_db_retrieval::{
    context_packet_output, graph_context_selection, hybrid_retrieval, kg_catalog_router,
    kg_retrieval, layered_drilldown, query, source_adapter, views,
};

pub mod benchmark;
pub mod benchmark_isolation;
pub mod browser;
pub mod diagnostics;
pub mod graph_explorer;
pub mod graph_explorer_postgres;
pub mod graph_refinement;
pub mod kg_markdown_manifest;
pub mod layered_backfill;
pub mod optimization;
