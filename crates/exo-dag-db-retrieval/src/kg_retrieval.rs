//! KG retrieval preview contracts for persisted DAG DB rows.
//!
//! This module defines repository-level preview DTOs. It does not expose a
//! gateway API, activate routes, persist context packets, write back memories,
//! or export knowledge.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error as StdError,
};

use exo_core::Hash256;
use exo_dag_db_api::SafeMetadata;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    hash::{parse_hash256_hex, stable_hash_parts},
    layered_hygiene::LayerHygieneEdgeState,
    scoring::hash_event_body,
};

/// Preview schema returned by the first persisted-KG retrieval path.
pub const KG_CONTEXT_PACKET_PREVIEW_SCHEMA: &str = "dagdb_kg_context_packet_preview_v1";
/// Environment variable used by Postgres-gated retrieval tests and helpers.
pub const KG_RETRIEVAL_DATABASE_URL_ENV: &str = "EXO_DAGDB_TEST_DATABASE_URL";
/// Route name used only for deterministic preview hash material.
pub const KG_RETRIEVAL_PREVIEW_ROUTE_NAME: &str = "dagdb.kg_retrieval.preview.v1";

/// Source error carried by DB-backed retrieval adapters without a reverse Postgres dependency.
pub type KgRetrievalSourceError = Box<dyn StdError + Send + Sync + 'static>;

const KG_LAYERED_RETRIEVAL_ROOT_PATH: &str = "root";
const KG_LAYERED_RETRIEVAL_TRAVERSAL_EDGE_KINDS: &[&str] = &["contains_subgraph", "drills_down_to"];

/// Repository-level retrieval request over persisted KG import rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgRetrievalRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub task_hash: Option<String>,
    pub task_description: Option<String>,
    pub token_budget: u32,
    #[serde(default)]
    pub requested_memory_ids: Vec<String>,
    pub catalog_path: Option<Vec<String>>,
    pub max_memory_refs: Option<u32>,
    #[serde(default)]
    pub layer_path: Option<String>,
    #[serde(default)]
    pub max_layer_depth: Option<u32>,
    #[serde(default)]
    pub max_layers_selected: Option<u32>,
    #[serde(default)]
    pub max_nodes_per_layer: Option<u32>,
    #[serde(default)]
    pub max_layer_edges: Option<u32>,
}

/// Bounded traversal and packet-selection budgets for layered retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayeredRetrievalBudgets {
    /// Maximum layer-edge hops from the selected start layer.
    pub max_depth: u32,
    /// Maximum number of layers selected by traversal.
    pub max_layers: u32,
    /// Maximum node refs later selected per selected layer.
    pub max_nodes_per_layer: u32,
    /// Maximum memory refs later selected across the packet.
    pub max_total_refs: u32,
    /// Maximum graph or layer edges later selected across the packet.
    pub max_layer_edges: u32,
}

impl Default for KgLayeredRetrievalBudgets {
    fn default() -> Self {
        Self {
            max_depth: KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
            max_layers: KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
            max_nodes_per_layer: KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
            max_total_refs: KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS,
            max_layer_edges: KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
        }
    }
}

/// Pure Rust layered retrieval input independent of Postgres packet output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgLayeredRetrievalRequest {
    /// Tenant scope for supplied layers and layer edges.
    pub tenant_id: String,
    /// Namespace scope for supplied layers and layer edges.
    pub namespace: String,
    /// Route-selected layer when routing has already picked one.
    pub route_layer_id: Option<String>,
    /// Known root layer when routing has not selected a layer.
    pub root_layer_id: Option<String>,
    /// Optional relative layer path supplied by graph context selection.
    pub start_layer_path: Option<String>,
    /// Traversal and downstream selection budgets.
    #[serde(default)]
    pub budgets: KgLayeredRetrievalBudgets,
}

impl KgLayeredRetrievalRequest {
    /// Validate the pure traversal request before candidate selection.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty("tenant_id", &self.tenant_id)?;
        validate_non_empty("namespace", &self.namespace)?;
        if let Some(route_layer_id) = &self.route_layer_id {
            hash_from_hex("route_layer_id", route_layer_id)?;
        }
        if let Some(root_layer_id) = &self.root_layer_id {
            hash_from_hex("root_layer_id", root_layer_id)?;
        }
        if let Some(start_layer_path) = &self.start_layer_path {
            validate_layer_path(start_layer_path)?;
        }
        self.budgets.validate()
    }
}

impl KgLayeredRetrievalBudgets {
    fn validate(&self) -> Result<()> {
        for (field, value) in [
            ("max_depth", self.max_depth),
            ("max_layers", self.max_layers),
            ("max_nodes_per_layer", self.max_nodes_per_layer),
            ("max_total_refs", self.max_total_refs),
            ("max_layer_edges", self.max_layer_edges),
        ] {
            if value == 0 {
                return Err(KgRetrievalError::InvalidRequest {
                    reason: format!("{field} must be positive"),
                });
            }
        }
        Ok(())
    }
}

/// Candidate layer row supplied to pure layered retrieval traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayerCandidate {
    pub layer_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub layer_path: String,
    pub layer_depth: u32,
    pub layer_kind: String,
    pub parent_layer_id: Option<String>,
    pub rollup_summary_ref: Option<String>,
    pub selection_score: u32,
}

/// Candidate layer-edge row supplied to pure layered retrieval traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayerEdgeCandidate {
    pub layer_edge_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub from_layer_id: String,
    pub to_layer_id: String,
    pub edge_kind: String,
    pub receipt_hash: Option<String>,
    #[serde(default)]
    pub hygiene_state: LayerHygieneEdgeState,
    pub selection_score: u32,
}

/// Selected layer returned by pure layered retrieval traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgSelectedLayer {
    pub layer_id: String,
    pub layer_path: String,
    pub layer_depth: u32,
    pub layer_kind: String,
    pub rollup_summary_ref: Option<String>,
    pub selection_reason: String,
}

/// Selected layer edge returned by pure layered retrieval traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgSelectedLayerEdge {
    pub layer_edge_id: String,
    pub from_layer_id: String,
    pub to_layer_id: String,
    pub edge_kind: String,
    pub receipt_hash: Option<String>,
    pub selection_reason: String,
}

/// Budget and fallback report for pure layered retrieval traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayeredRetrievalBudgetReport {
    pub max_depth: u32,
    pub max_layers: u32,
    pub max_nodes_per_layer: u32,
    pub max_total_refs: u32,
    pub max_layer_edges: u32,
    pub traversal_depth_reached: u32,
    pub selected_layer_count: u32,
    pub selected_node_count: u32,
    pub selected_memory_ref_count: u32,
    pub selected_layer_edge_count: u32,
    pub active_layer_edge_count: u32,
    pub excluded_demoted_layer_edge_count: u32,
    pub excluded_tombstoned_layer_edge_count: u32,
    pub depth_budget_exhausted: bool,
    pub layer_budget_exhausted: bool,
    pub layer_edge_budget_exhausted: bool,
    pub flat_fallback_used: bool,
}

/// Pure layered retrieval traversal output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayeredRetrievalSelection {
    pub selected_layers: Vec<KgSelectedLayer>,
    pub selected_layer_edges: Vec<KgSelectedLayerEdge>,
    pub budget_report: KgLayeredRetrievalBudgetReport,
    pub flat_fallback_used: bool,
    pub deterministic_ordering: bool,
    pub warnings: Vec<String>,
}

/// Default bounded layer traversal depth for repository/test retrieval.
pub const KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH: u32 = 3;
/// Default selected nodes per layer for layered retrieval.
pub const KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER: u32 = 12;
/// Default selected memory references for layered retrieval.
pub const KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS: u32 = 64;
/// Default selected layer-edge evidence for layered retrieval.
pub const KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES: u32 = 128;
/// Default selected layer count for layered retrieval.
pub const KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED: u32 = 16;

const KG_RETRIEVAL_HARD_MAX_LAYER_DEPTH: u32 = 16;
const KG_RETRIEVAL_HARD_MAX_NODES_PER_LAYER: u32 = 256;
const KG_RETRIEVAL_HARD_MAX_MEMORY_REFS: u32 = 512;
const KG_RETRIEVAL_HARD_MAX_LAYER_EDGES: u32 = 512;
const KG_RETRIEVAL_HARD_MAX_LAYERS_SELECTED: u32 = 128;

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "begin private key",
    "private key-----",
    "authorization",
    "bearer ",
    "database_url",
    "db_url",
    ".env",
    "mongodb://",
    "mysql://",
    "password",
    "postgres://",
    "postgresql://",
    "redis://",
    "secret",
    "sk-proj-",
    "sqlite://",
    "raw_body",
    "raw_document_body",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_private_payload",
    "raw_prompt_body",
    "source_excerpt",
];

impl KgRetrievalRequest {
    /// Validate the retrieval request before it reaches Postgres.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty("tenant_id", &self.tenant_id)?;
        validate_non_empty("namespace", &self.namespace)?;
        if self.token_budget == 0 {
            return Err(KgRetrievalError::InvalidRequest {
                reason: "token_budget must be positive".to_owned(),
            });
        }
        if let Some(task_hash) = &self.task_hash {
            hash_from_hex("task_hash", task_hash)?;
        }
        if let Some(task_description) = &self.task_description {
            validate_non_empty("task_description", task_description)?;
        }
        if let Some(catalog_path) = &self.catalog_path {
            validate_catalog_path(catalog_path)?;
        }
        if let Some(layer_path) = &self.layer_path {
            validate_layer_path(layer_path)?;
        }
        validate_optional_budget(
            "max_layer_depth",
            self.max_layer_depth,
            Some(0),
            KG_RETRIEVAL_HARD_MAX_LAYER_DEPTH,
        )?;
        validate_optional_budget(
            "max_layers_selected",
            self.max_layers_selected,
            None,
            KG_RETRIEVAL_HARD_MAX_LAYERS_SELECTED,
        )?;
        validate_optional_budget(
            "max_nodes_per_layer",
            self.max_nodes_per_layer,
            None,
            KG_RETRIEVAL_HARD_MAX_NODES_PER_LAYER,
        )?;
        validate_optional_budget(
            "max_layer_edges",
            self.max_layer_edges,
            None,
            KG_RETRIEVAL_HARD_MAX_LAYER_EDGES,
        )?;
        validate_optional_budget(
            "max_memory_refs",
            self.max_memory_refs,
            None,
            KG_RETRIEVAL_HARD_MAX_MEMORY_REFS,
        )?;
        let mut requested = BTreeSet::new();
        for memory_id in &self.requested_memory_ids {
            hash_from_hex("requested_memory_id", memory_id)?;
            if !requested.insert(memory_id) {
                return Err(KgRetrievalError::InvalidRequest {
                    reason: "duplicate requested_memory_id".to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Effective memory-reference budget after applying layered defaults.
    #[must_use]
    pub fn effective_max_memory_refs(&self) -> u32 {
        self.max_memory_refs
            .unwrap_or(KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS)
    }

    /// Effective layer traversal depth.
    #[must_use]
    pub fn effective_max_layer_depth(&self) -> u32 {
        self.max_layer_depth
            .unwrap_or(KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH)
    }

    /// Effective selected layer count budget.
    #[must_use]
    pub fn effective_max_layers_selected(&self) -> u32 {
        self.max_layers_selected
            .unwrap_or(KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED)
    }

    /// Effective node count per selected layer.
    #[must_use]
    pub fn effective_max_nodes_per_layer(&self) -> u32 {
        self.max_nodes_per_layer
            .unwrap_or(KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER)
    }

    /// Effective selected layer-edge evidence budget.
    #[must_use]
    pub fn effective_max_layer_edges(&self) -> u32 {
        self.max_layer_edges
            .unwrap_or(KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES)
    }

    /// Deterministic task hash material used for preview IDs.
    pub fn normalized_task_hash(&self) -> Result<Hash256> {
        if let Some(task_hash) = &self.task_hash {
            return hash_from_hex("task_hash", task_hash);
        }
        let task_material = self.task_description.as_deref().unwrap_or("unspecified");
        stable_hash(
            "exo.dagdb.kg_retrieval.preview.task_hash",
            &[&self.tenant_id, &self.namespace, task_material],
        )
    }
}

/// Compact preview packet assembled from persisted KG rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgContextPacketPreview {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub context_packet_id: String,
    pub route_hint_id: String,
    pub memory_refs: Vec<KgMemoryRef>,
    pub graph_edges: Vec<KgGraphEdgeRef>,
    pub selected_refs: Vec<KgMemoryRef>,
    pub selected_layers: Vec<KgSelectedLayerRef>,
    pub selected_layer_edges: Vec<KgLayerEdgeRef>,
    pub selected_graph_edges: Vec<KgGraphEdgeRef>,
    pub rollup_summaries: Vec<KgRollupSummaryRef>,
    pub budget_report: KgLayerBudgetReport,
    pub flat_fallback_used: bool,
    pub citation_handles: Vec<KgCitationHandle>,
    pub retrieval_diagnostics: KgRetrievalDiagnostics,
    pub validation_summary: KgValidationSummary,
    pub graph_path_summary: KgGraphPathSummary,
    pub citation_diagnostics: Vec<KgCitationDiagnostic>,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub omitted_memory_ids: Vec<String>,
    pub omitted_memory_refs: Vec<KgOmittedMemoryRef>,
    pub warnings: Vec<String>,
    pub dry_run_or_preview_only: bool,
}

/// Compact memory reference returned to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgMemoryRef {
    pub memory_id: String,
    pub catalog_id: Option<String>,
    pub source_path: Option<String>,
    pub catalog_path: Vec<String>,
    pub layer_id: Option<String>,
    pub layer_path: Option<String>,
    pub layer_depth: Option<u32>,
    pub layer_kind: Option<String>,
    pub layer_membership_role: Option<String>,
    pub layer_selection_reason: Option<String>,
    pub rollup_summary_ref: Option<String>,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub latest_receipt_hash: String,
    pub memory_status: String,
    pub validation_status: String,
    pub risk_class: String,
    pub council_status: String,
    pub dag_finality_status: String,
    pub graph_node_ids: Vec<String>,
    pub validation_report_ids: Vec<String>,
    pub citation_handle: String,
    pub token_estimate: u32,
    pub selection_reasons: Vec<String>,
}

/// Compact graph edge reference returned to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgGraphEdgeRef {
    pub graph_edge_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub edge_kind: String,
    pub graph_style: String,
    pub receipt_hash: Option<String>,
}

/// Compact selected layer evidence returned to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgSelectedLayerRef {
    pub layer_id: String,
    pub layer_path: String,
    pub layer_depth: u32,
    pub layer_kind: String,
    pub graph_style: String,
    pub root_memory_id: String,
    pub parent_layer_id: Option<String>,
    pub parent_graph_node_id: Option<String>,
    pub selection_reason: String,
    pub selected_memory_count: u32,
    pub rollup_summary_ref: Option<String>,
}

/// Compact selected layer-edge evidence returned to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayerEdgeRef {
    pub layer_edge_id: String,
    pub from_layer_id: String,
    pub to_layer_id: String,
    pub edge_kind: String,
    pub graph_style: String,
    pub receipt_hash: Option<String>,
    pub selection_reason: String,
}

/// Safe rollup summary reference for selected layer context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgRollupSummaryRef {
    pub rollup_summary_ref: String,
    pub layer_id: String,
    pub layer_path: String,
    pub memory_id: String,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub selection_reason: String,
}

/// Budget evidence for a bounded layered retrieval pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgLayerBudgetReport {
    pub max_layer_depth: u32,
    pub max_layers_selected: u32,
    pub max_nodes_per_layer: u32,
    pub max_memory_refs: u32,
    pub max_layer_edges: u32,
    pub selected_layer_count: u32,
    pub selected_layer_edge_count: u32,
    pub active_layer_edge_count: u32,
    pub excluded_demoted_layer_edge_count: u32,
    pub excluded_tombstoned_layer_edge_count: u32,
    pub selected_memory_ref_count: u32,
    pub selected_graph_edge_count: u32,
    pub depth_budget_truncated: bool,
    pub layer_budget_truncated: bool,
    pub node_budget_truncated: bool,
    pub layer_edge_budget_truncated: bool,
    pub token_budget_truncated: bool,
    pub flat_fallback_used: bool,
}

/// Stable citation handle for selected memory and graph records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgCitationHandle {
    pub handle: String,
    pub memory_id: String,
    pub catalog_id: Option<String>,
    pub latest_receipt_hash: String,
    pub graph_node_ids: Vec<String>,
    pub graph_edge_ids: Vec<String>,
    pub validation_report_ids: Vec<String>,
}

/// Deterministic diagnostics for the preview retrieval pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgRetrievalDiagnostics {
    pub selected_memory_count: u32,
    pub omitted_memory_count: u32,
    pub selected_graph_edge_count: u32,
    pub selected_layer_count: u32,
    pub selected_layer_edge_count: u32,
    pub active_layer_edge_count: u32,
    pub excluded_demoted_layer_edge_count: u32,
    pub excluded_tombstoned_layer_edge_count: u32,
    pub citation_handle_count: u32,
    pub warning_count: u32,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub max_layer_depth: u32,
    pub max_layers_selected: u32,
    pub max_nodes_per_layer: u32,
    pub max_layer_edges: u32,
    pub layer_path_filter_applied: bool,
    pub max_memory_refs_applied: bool,
    pub catalog_path_filter_applied: bool,
    pub requested_memory_filter_applied: bool,
    pub flat_fallback_used: bool,
    pub depth_budget_truncated: bool,
    pub layer_budget_truncated: bool,
    pub node_budget_truncated: bool,
    pub layer_edge_budget_truncated: bool,
    pub deterministic_ordering: bool,
    pub raw_markdown_returned: bool,
    pub preview_only: bool,
}

/// Explanation for a memory omitted from the compact preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgOmittedMemoryRef {
    pub memory_id: String,
    pub reason: String,
    pub token_estimate_if_selected: Option<u32>,
    pub catalog_path: Vec<String>,
    pub validation_status: Option<String>,
    pub risk_class: Option<String>,
}

/// Citation coverage details for a selected memory reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgCitationDiagnostic {
    pub citation_handle: String,
    pub memory_id: String,
    pub catalog_id: Option<String>,
    pub validation_report_id: Option<String>,
    pub latest_receipt_hash: Option<String>,
    pub graph_node_ids: Vec<String>,
    pub graph_edge_ids: Vec<String>,
    pub citation_status: String,
    pub reason: String,
}

/// Validation status rollup for a preview packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgValidationSummary {
    pub selected_memory_count: u32,
    pub pending_count: u32,
    pub passed_count: u32,
    pub failed_count: u32,
    pub needs_council_count: u32,
    pub warning_count: u32,
    pub validation_status_counts: BTreeMap<String, u32>,
    pub risk_class_counts: BTreeMap<String, u32>,
    pub dag_finality_status_counts: BTreeMap<String, u32>,
    pub council_status_counts: BTreeMap<String, u32>,
}

/// Graph-path coverage for the selected preview memory set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KgGraphPathSummary {
    pub graph_edge_count: u32,
    pub graph_styles_seen: Vec<String>,
    pub edge_kinds_seen: Vec<String>,
    pub isolated_memory_count: u32,
    pub connected_memory_count: u32,
    pub missing_edge_warning_count: u32,
}

/// Errors raised by the retrieval preview layer.
#[derive(Debug, Error)]
pub enum KgRetrievalError {
    /// No database URL was supplied for persisted retrieval mode.
    #[error("kg_retrieval_database_url_missing: {env_var}")]
    MissingDatabaseUrl {
        /// Required env var.
        env_var: &'static str,
    },
    /// Request shape is invalid or unsafe.
    #[error("kg_retrieval_request_invalid: {reason}")]
    InvalidRequest {
        /// Stable validation reason.
        reason: String,
    },
    /// Supplied layer candidates or layer edges are inconsistent.
    #[error("kg_retrieval_layer_graph_invalid: {reason}")]
    InvalidLayerGraph {
        /// Stable validation reason.
        reason: String,
    },
    /// Hash material could not be parsed or computed.
    #[error("kg_retrieval_hash_failed: {reason}")]
    Hash {
        /// Stable hash reason.
        reason: String,
    },
    /// A database hash column had the wrong byte length.
    #[error("kg_retrieval_invalid_hash_column: {field}")]
    InvalidHashColumn {
        /// Column or field name.
        field: String,
    },
    /// Postgres foundation failed.
    #[error("kg_retrieval_postgres_init_failed")]
    Init {
        /// Source Postgres foundation error.
        #[source]
        source: KgRetrievalSourceError,
    },
    /// SQL operation failed.
    #[error("kg_retrieval_postgres_failed")]
    Postgres {
        /// Source SQLx error.
        #[source]
        source: KgRetrievalSourceError,
    },
    /// JSON conversion failed.
    #[error("kg_retrieval_json_failed: {reason}")]
    Json {
        /// Stable conversion reason.
        reason: String,
    },
}

/// Result alias for KG retrieval preview.
pub type Result<T> = std::result::Result<T, KgRetrievalError>;

fn hash_from_hex(field: &str, value: &str) -> Result<Hash256> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(KgRetrievalError::Hash {
            reason: format!("kg_import_hash_invalid: {field}"),
        });
    }
    parse_hash256_hex(field, value).map_err(|error| KgRetrievalError::Hash {
        reason: error.to_string(),
    })
}

fn stable_hash(domain: &str, parts: &[&str]) -> Result<Hash256> {
    stable_hash_parts(domain, parts).map_err(|error| KgRetrievalError::Hash {
        reason: format!("kg_import_hash_failed: {error}"),
    })
}

/// Select layer traversal candidates without reading or writing Postgres rows.
pub fn select_layered_retrieval_candidates(
    request: &KgLayeredRetrievalRequest,
    layers: &[KgLayerCandidate],
    layer_edges: &[KgLayerEdgeCandidate],
) -> Result<KgLayeredRetrievalSelection> {
    request.validate()?;
    if layers.is_empty() {
        return layered_flat_fallback(request, "flat_fallback_no_layer_evidence".to_owned());
    }

    let layer_index = validate_layer_candidates(request, layers)?;
    let edge_index = validate_layer_edge_candidates(request, &layer_index, layer_edges)?;
    let mut warnings = Vec::new();
    let Some((start_layer_id, start_reason)) =
        select_start_layer(request, &layer_index, &mut warnings)
    else {
        warnings.push("flat_fallback_no_route_or_root_layer".to_owned());
        return layered_flat_fallback_with_warnings(request, warnings);
    };

    let mut selected_layers = Vec::new();
    let mut selected_layer_edges = Vec::new();
    let mut queue = VecDeque::new();
    let mut scheduled_layer_ids = BTreeSet::new();
    let mut depth_budget_exhausted = false;
    let mut layer_budget_exhausted = false;
    let mut layer_edge_budget_exhausted = false;
    let mut traversal_depth_reached = 0;

    scheduled_layer_ids.insert(start_layer_id.clone());
    queue.push_back(LayerTraversalQueueItem {
        layer_id: start_layer_id,
        traversal_depth: 0,
        selection_reason: start_reason,
    });
    let max_layers = usize::try_from(request.budgets.max_layers).map_err(|_| {
        KgRetrievalError::InvalidRequest {
            reason: "max_layers out of range".to_owned(),
        }
    })?;
    let max_layer_edges = usize::try_from(request.budgets.max_layer_edges).map_err(|_| {
        KgRetrievalError::InvalidRequest {
            reason: "max_layer_edges out of range".to_owned(),
        }
    })?;

    while let Some(item) = queue.pop_front() {
        let layer =
            layer_index
                .get(&item.layer_id)
                .ok_or_else(|| KgRetrievalError::InvalidLayerGraph {
                    reason: format!("scheduled layer {} is missing", item.layer_id),
                })?;
        traversal_depth_reached = traversal_depth_reached.max(item.traversal_depth);
        if selected_layers.len() >= max_layers {
            layer_budget_exhausted = true;
            continue;
        }
        selected_layers.push(KgSelectedLayer {
            layer_id: layer.layer_id.clone(),
            layer_path: layer.layer_path.clone(),
            layer_depth: layer.layer_depth,
            layer_kind: layer.layer_kind.clone(),
            rollup_summary_ref: layer.rollup_summary_ref.clone(),
            selection_reason: item.selection_reason,
        });

        let Some(edges) = edge_index.outgoing.get(&layer.layer_id) else {
            continue;
        };
        for edge in edges {
            if scheduled_layer_ids.contains(&edge.to_layer_id) {
                continue;
            }
            let next_depth = item.traversal_depth.saturating_add(1);
            if next_depth > request.budgets.max_depth {
                depth_budget_exhausted = true;
                continue;
            }
            if selected_layers
                .len()
                .saturating_add(queue.len())
                .saturating_add(1)
                > max_layers
            {
                layer_budget_exhausted = true;
                continue;
            }
            if selected_layer_edges.len() >= max_layer_edges {
                layer_edge_budget_exhausted = true;
                continue;
            }

            scheduled_layer_ids.insert(edge.to_layer_id.clone());
            traversal_depth_reached = traversal_depth_reached.max(next_depth);
            selected_layer_edges.push(KgSelectedLayerEdge {
                layer_edge_id: edge.layer_edge_id.clone(),
                from_layer_id: edge.from_layer_id.clone(),
                to_layer_id: edge.to_layer_id.clone(),
                edge_kind: edge.edge_kind.clone(),
                receipt_hash: edge.receipt_hash.clone(),
                selection_reason: format!("drilldown_via_{}", edge.edge_kind),
            });
            queue.push_back(LayerTraversalQueueItem {
                layer_id: edge.to_layer_id.clone(),
                traversal_depth: next_depth,
                selection_reason: format!("selected_by_layer_edge_{}", edge.edge_kind),
            });
        }
    }

    if depth_budget_exhausted {
        warnings.push("layer_depth_budget_exhausted".to_owned());
    }
    if layer_budget_exhausted {
        warnings.push("layer_count_budget_exhausted".to_owned());
    }
    if layer_edge_budget_exhausted {
        warnings.push("layer_edge_budget_exhausted".to_owned());
    }

    Ok(KgLayeredRetrievalSelection {
        budget_report: layered_budget_report(
            request,
            LayeredBudgetReportInput {
                traversal_depth_reached,
                selected_layer_count: selected_layers.len(),
                selected_layer_edge_count: selected_layer_edges.len(),
                active_layer_edge_count: edge_index.active_layer_edge_count,
                excluded_demoted_layer_edge_count: edge_index.excluded_demoted_layer_edge_count,
                excluded_tombstoned_layer_edge_count: edge_index
                    .excluded_tombstoned_layer_edge_count,
                depth_budget_exhausted,
                layer_budget_exhausted,
                layer_edge_budget_exhausted,
                flat_fallback_used: false,
            },
        )?,
        selected_layers,
        selected_layer_edges,
        flat_fallback_used: false,
        deterministic_ordering: true,
        warnings,
    })
}

/// Build a stable context-packet preview ID.
pub fn context_packet_preview_id(
    request: &KgRetrievalRequest,
    route_hint_id: &str,
    selected_memory_ids: &[String],
) -> Result<Hash256> {
    request.validate()?;
    let task_hash = request.normalized_task_hash()?.to_string();
    hash_event_body(&PreviewIdMaterial {
        tenant_id: &request.tenant_id,
        namespace: &request.namespace,
        task_hash: &task_hash,
        route_hint_id,
        selected_memory_ids,
        token_budget: request.token_budget,
    })
    .map_err(|error| KgRetrievalError::Hash {
        reason: error.to_string(),
    })
}

/// Build a stable route hint ID without activating a route.
pub fn route_hint_id(
    request: &KgRetrievalRequest,
    candidate_memory_ids: &[String],
) -> Result<Hash256> {
    request.validate()?;
    let task_hash = request.normalized_task_hash()?.to_string();
    stable_hash(
        "exo.dagdb.kg_retrieval.preview.route_hint_id",
        &[
            &request.tenant_id,
            &request.namespace,
            &task_hash,
            &request.token_budget.to_string(),
            &candidate_memory_ids.join(","),
        ],
    )
}

/// Return a deterministic token estimate for safe title/summary metadata.
#[must_use]
pub fn memory_token_estimate(title: &SafeMetadata, summary: &SafeMetadata) -> u32 {
    let byte_estimate = title
        .byte_len
        .saturating_add(summary.byte_len)
        .saturating_add(32);
    byte_estimate.saturating_add(3) / 4
}

/// Build a stable citation handle for a memory reference.
pub fn citation_handle(
    tenant_id: &str,
    namespace: &str,
    memory_id: &str,
    catalog_id: Option<&str>,
) -> Result<String> {
    let catalog = catalog_id.unwrap_or("none");
    let hash = stable_hash(
        "exo.dagdb.kg_retrieval.preview.citation_handle",
        &[tenant_id, namespace, memory_id, catalog],
    )?;
    Ok(format!("dagdb://kg/{tenant_id}/{namespace}/{hash}"))
}

#[derive(Serialize)]
struct PreviewIdMaterial<'a> {
    tenant_id: &'a str,
    namespace: &'a str,
    task_hash: &'a str,
    route_hint_id: &'a str,
    selected_memory_ids: &'a [String],
    token_budget: u32,
}

struct LayerTraversalQueueItem {
    layer_id: String,
    traversal_depth: u32,
    selection_reason: String,
}

struct LayerEdgeCandidateIndex<'a> {
    outgoing: BTreeMap<String, Vec<&'a KgLayerEdgeCandidate>>,
    active_layer_edge_count: usize,
    excluded_demoted_layer_edge_count: usize,
    excluded_tombstoned_layer_edge_count: usize,
}

fn validate_layer_candidates<'a>(
    request: &KgLayeredRetrievalRequest,
    layers: &'a [KgLayerCandidate],
) -> Result<BTreeMap<String, &'a KgLayerCandidate>> {
    let mut by_id = BTreeMap::new();
    let mut by_path = BTreeSet::new();
    for layer in layers {
        validate_scope_match(
            "layer",
            &layer.tenant_id,
            &request.tenant_id,
            &layer.namespace,
            &request.namespace,
        )?;
        hash_from_hex("layer_id", &layer.layer_id)?;
        if let Some(parent_layer_id) = &layer.parent_layer_id {
            hash_from_hex("parent_layer_id", parent_layer_id)?;
        }
        if let Some(rollup_summary_ref) = &layer.rollup_summary_ref {
            validate_non_empty("rollup_summary_ref", rollup_summary_ref)?;
        }
        validate_non_empty("layer_kind", &layer.layer_kind)?;
        validate_layer_path(&layer.layer_path)?;
        if by_id.insert(layer.layer_id.clone(), layer).is_some() {
            return Err(KgRetrievalError::InvalidLayerGraph {
                reason: "duplicate layer_id".to_owned(),
            });
        }
        if !by_path.insert(layer.layer_path.clone()) {
            return Err(KgRetrievalError::InvalidLayerGraph {
                reason: "duplicate layer_path".to_owned(),
            });
        }
    }
    Ok(by_id)
}

fn validate_layer_edge_candidates<'a>(
    request: &KgLayeredRetrievalRequest,
    layer_index: &BTreeMap<String, &'a KgLayerCandidate>,
    layer_edges: &'a [KgLayerEdgeCandidate],
) -> Result<LayerEdgeCandidateIndex<'a>> {
    let mut by_id = BTreeSet::new();
    let mut outgoing: BTreeMap<String, Vec<&KgLayerEdgeCandidate>> = BTreeMap::new();
    let mut active_layer_edge_count = 0usize;
    let mut excluded_demoted_layer_edge_count = 0usize;
    let mut excluded_tombstoned_layer_edge_count = 0usize;
    for edge in layer_edges {
        validate_scope_match(
            "layer_edge",
            &edge.tenant_id,
            &request.tenant_id,
            &edge.namespace,
            &request.namespace,
        )?;
        hash_from_hex("layer_edge_id", &edge.layer_edge_id)?;
        hash_from_hex("from_layer_id", &edge.from_layer_id)?;
        hash_from_hex("to_layer_id", &edge.to_layer_id)?;
        if let Some(receipt_hash) = &edge.receipt_hash {
            hash_from_hex("receipt_hash", receipt_hash)?;
        }
        validate_non_empty("layer_edge_kind", &edge.edge_kind)?;
        if edge.from_layer_id == edge.to_layer_id {
            return Err(KgRetrievalError::InvalidLayerGraph {
                reason: "layer edge cannot point to itself".to_owned(),
            });
        }
        if !by_id.insert(edge.layer_edge_id.clone()) {
            return Err(KgRetrievalError::InvalidLayerGraph {
                reason: "duplicate layer_edge_id".to_owned(),
            });
        }
        if !layer_index.contains_key(&edge.from_layer_id) {
            return Err(KgRetrievalError::InvalidLayerGraph {
                reason: format!(
                    "layer edge references unknown source {}",
                    edge.from_layer_id
                ),
            });
        }
        if !layer_index.contains_key(&edge.to_layer_id) {
            return Err(KgRetrievalError::InvalidLayerGraph {
                reason: format!("layer edge references unknown target {}", edge.to_layer_id),
            });
        }
        match edge.hygiene_state {
            LayerHygieneEdgeState::Active => {
                active_layer_edge_count = active_layer_edge_count.saturating_add(1);
            }
            LayerHygieneEdgeState::Demoted => {
                excluded_demoted_layer_edge_count =
                    excluded_demoted_layer_edge_count.saturating_add(1);
                continue;
            }
            LayerHygieneEdgeState::Tombstoned => {
                excluded_tombstoned_layer_edge_count =
                    excluded_tombstoned_layer_edge_count.saturating_add(1);
                continue;
            }
        }
        if layer_edge_kind_is_traversable(&edge.edge_kind) {
            outgoing
                .entry(edge.from_layer_id.clone())
                .or_default()
                .push(edge);
        }
    }
    for edges in outgoing.values_mut() {
        edges.sort_by(|left, right| {
            let Some(left_layer) = layer_index.get(&left.to_layer_id) else {
                return std::cmp::Ordering::Equal;
            };
            let Some(right_layer) = layer_index.get(&right.to_layer_id) else {
                return std::cmp::Ordering::Equal;
            };
            right
                .selection_score
                .cmp(&left.selection_score)
                .then_with(|| right_layer.selection_score.cmp(&left_layer.selection_score))
                .then_with(|| left_layer.layer_path.cmp(&right_layer.layer_path))
                .then_with(|| left.edge_kind.cmp(&right.edge_kind))
                .then_with(|| left.layer_edge_id.cmp(&right.layer_edge_id))
        });
    }
    Ok(LayerEdgeCandidateIndex {
        outgoing,
        active_layer_edge_count,
        excluded_demoted_layer_edge_count,
        excluded_tombstoned_layer_edge_count,
    })
}

fn select_start_layer(
    request: &KgLayeredRetrievalRequest,
    layer_index: &BTreeMap<String, &KgLayerCandidate>,
    warnings: &mut Vec<String>,
) -> Option<(String, String)> {
    if let Some(route_layer_id) = &request.route_layer_id {
        if layer_index.contains_key(route_layer_id) {
            return Some((route_layer_id.clone(), "route_layer_start".to_owned()));
        }
        warnings.push("route_layer_not_found_root_fallback".to_owned());
    }
    if let Some(root_layer_id) = &request.root_layer_id {
        if layer_index.contains_key(root_layer_id) {
            return Some((root_layer_id.clone(), "root_layer_start".to_owned()));
        }
        warnings.push("root_layer_id_not_found".to_owned());
    }
    if let Some(start_layer_path) = &request.start_layer_path {
        if let Some(layer) = layer_index
            .values()
            .find(|layer| layer.layer_path == *start_layer_path)
        {
            return Some((layer.layer_id.clone(), "start_layer_path_start".to_owned()));
        }
        warnings.push("start_layer_path_not_found".to_owned());
    }
    layer_index
        .values()
        .find(|layer| layer.layer_path == KG_LAYERED_RETRIEVAL_ROOT_PATH)
        .map(|layer| (layer.layer_id.clone(), "root_layer_path_start".to_owned()))
}

fn layered_flat_fallback(
    request: &KgLayeredRetrievalRequest,
    warning: String,
) -> Result<KgLayeredRetrievalSelection> {
    layered_flat_fallback_with_warnings(request, vec![warning])
}

fn layered_flat_fallback_with_warnings(
    request: &KgLayeredRetrievalRequest,
    warnings: Vec<String>,
) -> Result<KgLayeredRetrievalSelection> {
    Ok(KgLayeredRetrievalSelection {
        selected_layers: Vec::new(),
        selected_layer_edges: Vec::new(),
        budget_report: layered_budget_report(
            request,
            LayeredBudgetReportInput {
                traversal_depth_reached: 0,
                selected_layer_count: 0,
                selected_layer_edge_count: 0,
                active_layer_edge_count: 0,
                excluded_demoted_layer_edge_count: 0,
                excluded_tombstoned_layer_edge_count: 0,
                depth_budget_exhausted: false,
                layer_budget_exhausted: false,
                layer_edge_budget_exhausted: false,
                flat_fallback_used: true,
            },
        )?,
        flat_fallback_used: true,
        deterministic_ordering: true,
        warnings,
    })
}

struct LayeredBudgetReportInput {
    traversal_depth_reached: u32,
    selected_layer_count: usize,
    selected_layer_edge_count: usize,
    active_layer_edge_count: usize,
    excluded_demoted_layer_edge_count: usize,
    excluded_tombstoned_layer_edge_count: usize,
    depth_budget_exhausted: bool,
    layer_budget_exhausted: bool,
    layer_edge_budget_exhausted: bool,
    flat_fallback_used: bool,
}

fn layered_budget_report(
    request: &KgLayeredRetrievalRequest,
    input: LayeredBudgetReportInput,
) -> Result<KgLayeredRetrievalBudgetReport> {
    Ok(KgLayeredRetrievalBudgetReport {
        max_depth: request.budgets.max_depth,
        max_layers: request.budgets.max_layers,
        max_nodes_per_layer: request.budgets.max_nodes_per_layer,
        max_total_refs: request.budgets.max_total_refs,
        max_layer_edges: request.budgets.max_layer_edges,
        traversal_depth_reached: input.traversal_depth_reached,
        selected_layer_count: u32::try_from(input.selected_layer_count).map_err(|_| {
            KgRetrievalError::InvalidLayerGraph {
                reason: "selected layer count overflow".to_owned(),
            }
        })?,
        selected_node_count: 0,
        selected_memory_ref_count: 0,
        selected_layer_edge_count: u32::try_from(input.selected_layer_edge_count).map_err(
            |_| KgRetrievalError::InvalidLayerGraph {
                reason: "selected layer edge count overflow".to_owned(),
            },
        )?,
        active_layer_edge_count: u32::try_from(input.active_layer_edge_count).map_err(|_| {
            KgRetrievalError::InvalidLayerGraph {
                reason: "active layer edge count overflow".to_owned(),
            }
        })?,
        excluded_demoted_layer_edge_count: u32::try_from(input.excluded_demoted_layer_edge_count)
            .map_err(|_| KgRetrievalError::InvalidLayerGraph {
            reason: "excluded demoted layer edge count overflow".to_owned(),
        })?,
        excluded_tombstoned_layer_edge_count: u32::try_from(
            input.excluded_tombstoned_layer_edge_count,
        )
        .map_err(|_| KgRetrievalError::InvalidLayerGraph {
            reason: "excluded tombstoned layer edge count overflow".to_owned(),
        })?,
        depth_budget_exhausted: input.depth_budget_exhausted,
        layer_budget_exhausted: input.layer_budget_exhausted,
        layer_edge_budget_exhausted: input.layer_edge_budget_exhausted,
        flat_fallback_used: input.flat_fallback_used,
    })
}

fn layer_edge_kind_is_traversable(edge_kind: &str) -> bool {
    KG_LAYERED_RETRIEVAL_TRAVERSAL_EDGE_KINDS.contains(&edge_kind)
}

fn validate_scope_match(
    row_label: &str,
    tenant_id: &str,
    expected_tenant_id: &str,
    namespace: &str,
    expected_namespace: &str,
) -> Result<()> {
    validate_non_empty("tenant_id", tenant_id)?;
    validate_non_empty("namespace", namespace)?;
    if tenant_id != expected_tenant_id || namespace != expected_namespace {
        return Err(KgRetrievalError::InvalidLayerGraph {
            reason: format!("{row_label} tenant or namespace mismatch"),
        });
    }
    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(KgRetrievalError::InvalidRequest {
            reason: format!("{field} must not be empty"),
        });
    }
    reject_forbidden_string(field, value)
}

fn validate_catalog_path(path: &[String]) -> Result<()> {
    if path.is_empty() {
        return Err(KgRetrievalError::InvalidRequest {
            reason: "catalog_path must not be empty when supplied".to_owned(),
        });
    }
    for part in path {
        validate_non_empty("catalog_path segment", part)?;
        if part == "." || part == ".." || part.contains('/') || part.contains('\\') {
            return Err(KgRetrievalError::InvalidRequest {
                reason: "catalog_path contains unsafe segment".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_layer_path(path: &str) -> Result<()> {
    validate_non_empty("layer_path", path)?;
    if path.starts_with('/') || path.ends_with('/') || path.contains("//") {
        return Err(KgRetrievalError::InvalidRequest {
            reason: "layer_path must be relative and normalized".to_owned(),
        });
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains('\\') {
            return Err(KgRetrievalError::InvalidRequest {
                reason: "layer_path contains unsafe segment".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_optional_budget(
    field: &str,
    value: Option<u32>,
    allowed_zero: Option<u32>,
    hard_max: u32,
) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value == 0 && allowed_zero != Some(0) {
        return Err(KgRetrievalError::InvalidRequest {
            reason: format!("{field} must be positive"),
        });
    }
    if value > hard_max {
        return Err(KgRetrievalError::InvalidRequest {
            reason: format!("{field} exceeds hard limit"),
        });
    }
    Ok(())
}

fn reject_forbidden_string(field: &str, value: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| lowered.contains(**fragment))
    {
        return Err(KgRetrievalError::InvalidRequest {
            reason: format!("{field} contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

/// Convert a 32-byte database hash column into lowercase hex.
pub fn hex_from_hash_column(field: &str, bytes: Vec<u8>) -> Result<String> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| KgRetrievalError::InvalidHashColumn {
            field: field.to_owned(),
        })?;
    Ok(Hash256::from_bytes(bytes).to_string())
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::{SafeMetadata, SafeMetadataDecision};

    use super::*;

    fn safe(text: &str) -> SafeMetadata {
        SafeMetadata {
            decision: SafeMetadataDecision::Allow,
            text: text.to_owned(),
            redaction_codes: Vec::new(),
            original_hash: "ef".repeat(32),
            truncated: false,
            byte_len: u32::try_from(text.len()).expect("fixture length"),
        }
    }

    fn hash(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn layered_request(root_layer_id: Option<String>) -> KgLayeredRetrievalRequest {
        KgLayeredRetrievalRequest {
            tenant_id: "tenant".to_owned(),
            namespace: "dag-db".to_owned(),
            route_layer_id: None,
            root_layer_id,
            start_layer_path: None,
            budgets: KgLayeredRetrievalBudgets {
                max_depth: 2,
                max_layers: 8,
                max_nodes_per_layer: 12,
                max_total_refs: 64,
                max_layer_edges: 8,
            },
        }
    }

    fn layer(
        id_byte: u8,
        layer_path: &str,
        layer_depth: u32,
        layer_kind: &str,
        selection_score: u32,
    ) -> KgLayerCandidate {
        KgLayerCandidate {
            layer_id: hash(id_byte),
            tenant_id: "tenant".to_owned(),
            namespace: "dag-db".to_owned(),
            layer_path: layer_path.to_owned(),
            layer_depth,
            layer_kind: layer_kind.to_owned(),
            parent_layer_id: None,
            rollup_summary_ref: Some(format!("rollup:{layer_path}")),
            selection_score,
        }
    }

    fn layer_edge(
        id_byte: u8,
        from_byte: u8,
        to_byte: u8,
        edge_kind: &str,
        selection_score: u32,
    ) -> KgLayerEdgeCandidate {
        KgLayerEdgeCandidate {
            layer_edge_id: hash(id_byte),
            tenant_id: "tenant".to_owned(),
            namespace: "dag-db".to_owned(),
            from_layer_id: hash(from_byte),
            to_layer_id: hash(to_byte),
            edge_kind: edge_kind.to_owned(),
            receipt_hash: Some(hash(id_byte.saturating_add(40))),
            hygiene_state: LayerHygieneEdgeState::Active,
            selection_score,
        }
    }

    #[test]
    fn layered_retrieval_traversal_selects_root_and_child_deterministically() {
        let request = KgLayeredRetrievalRequest {
            budgets: KgLayeredRetrievalBudgets {
                max_depth: 1,
                max_layers: 8,
                max_nodes_per_layer: 12,
                max_total_refs: 64,
                max_layer_edges: 8,
            },
            ..layered_request(Some(hash(1)))
        };
        let layers = vec![
            layer(3, "root/knowledge-graph/source", 2, "source_subgraph", 25),
            layer(1, "root", 0, "root", 10),
            layer(2, "root/knowledge-graph", 1, "knowledge_graph", 50),
        ];
        let layer_edges = vec![
            layer_edge(12, 2, 3, "drills_down_to", 90),
            layer_edge(11, 1, 2, "contains_subgraph", 100),
        ];

        let selection =
            select_layered_retrieval_candidates(&request, &layers, &layer_edges).expect("select");

        assert_eq!(
            selection
                .selected_layers
                .iter()
                .map(|layer| layer.layer_path.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "root/knowledge-graph"]
        );
        assert_eq!(selection.selected_layer_edges.len(), 1);
        assert_eq!(
            selection.selected_layer_edges[0].edge_kind,
            "contains_subgraph"
        );
        assert!(selection.budget_report.depth_budget_exhausted);
        assert_eq!(selection.budget_report.traversal_depth_reached, 1);
        assert_eq!(selection.budget_report.max_nodes_per_layer, 12);
        assert_eq!(selection.budget_report.max_total_refs, 64);
        assert!(!selection.flat_fallback_used);

        let reordered_layers = vec![
            layer(2, "root/knowledge-graph", 1, "knowledge_graph", 50),
            layer(3, "root/knowledge-graph/source", 2, "source_subgraph", 25),
            layer(1, "root", 0, "root", 10),
        ];
        let reordered_edges = vec![
            layer_edge(11, 1, 2, "contains_subgraph", 100),
            layer_edge(12, 2, 3, "drills_down_to", 90),
        ];
        let reordered =
            select_layered_retrieval_candidates(&request, &reordered_layers, &reordered_edges)
                .expect("reordered select");
        assert_eq!(selection, reordered);
    }

    #[test]
    fn layered_retrieval_route_layer_starts_inside_graph() {
        let mut request = layered_request(Some(hash(1)));
        request.route_layer_id = Some(hash(2));
        let layers = vec![
            layer(1, "root", 0, "root", 10),
            layer(2, "root/knowledge-graph", 1, "route", 90),
            layer(3, "root/knowledge-graph/source", 2, "source_subgraph", 50),
        ];
        let layer_edges = vec![
            layer_edge(11, 1, 2, "contains_subgraph", 100),
            layer_edge(12, 2, 3, "drills_down_to", 90),
        ];

        let selection =
            select_layered_retrieval_candidates(&request, &layers, &layer_edges).expect("select");

        assert_eq!(
            selection
                .selected_layers
                .iter()
                .map(|layer| layer.layer_path.as_str())
                .collect::<Vec<_>>(),
            vec!["root/knowledge-graph", "root/knowledge-graph/source"]
        );
        assert_eq!(
            selection.selected_layers[0].selection_reason,
            "route_layer_start"
        );
        assert!(!selection.flat_fallback_used);
    }

    #[test]
    fn layered_retrieval_layer_and_edge_budgets_truncate_selection() {
        let request = KgLayeredRetrievalRequest {
            budgets: KgLayeredRetrievalBudgets {
                max_depth: 3,
                max_layers: 3,
                max_nodes_per_layer: 4,
                max_total_refs: 5,
                max_layer_edges: 1,
            },
            ..layered_request(Some(hash(1)))
        };
        let layers = vec![
            layer(1, "root", 0, "root", 10),
            layer(2, "root/a", 1, "source_subgraph", 70),
            layer(3, "root/b", 1, "source_subgraph", 60),
        ];
        let layer_edges = vec![
            layer_edge(11, 1, 3, "contains_subgraph", 60),
            layer_edge(10, 1, 2, "contains_subgraph", 70),
        ];

        let selection =
            select_layered_retrieval_candidates(&request, &layers, &layer_edges).expect("select");

        assert_eq!(
            selection
                .selected_layers
                .iter()
                .map(|layer| layer.layer_path.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "root/a"]
        );
        assert_eq!(selection.selected_layer_edges.len(), 1);
        assert_eq!(selection.budget_report.max_nodes_per_layer, 4);
        assert_eq!(selection.budget_report.max_total_refs, 5);
        assert!(selection.budget_report.layer_edge_budget_exhausted);
        assert!(
            selection
                .warnings
                .contains(&"layer_edge_budget_exhausted".to_owned())
        );
    }

    #[test]
    fn layered_retrieval_hygiene_excludes_demoted_and_tombstoned_layer_edges() {
        let request = KgLayeredRetrievalRequest {
            budgets: KgLayeredRetrievalBudgets {
                max_depth: 2,
                max_layers: 8,
                max_nodes_per_layer: 12,
                max_total_refs: 64,
                max_layer_edges: 8,
            },
            ..layered_request(Some(hash(1)))
        };
        let layers = vec![
            layer(1, "root", 0, "root", 10),
            layer(2, "root/active", 1, "source_subgraph", 90),
            layer(3, "root/demoted", 1, "source_subgraph", 80),
            layer(4, "root/tombstoned", 1, "source_subgraph", 70),
        ];
        let mut active = layer_edge(10, 1, 2, "contains_subgraph", 60);
        active.hygiene_state = LayerHygieneEdgeState::Active;
        let mut demoted = layer_edge(11, 1, 3, "contains_subgraph", 100);
        demoted.hygiene_state = LayerHygieneEdgeState::Demoted;
        let mut tombstoned = layer_edge(12, 1, 4, "contains_subgraph", 95);
        tombstoned.hygiene_state = LayerHygieneEdgeState::Tombstoned;

        let selection =
            select_layered_retrieval_candidates(&request, &layers, &[active, demoted, tombstoned])
                .expect("select");

        assert_eq!(
            selection
                .selected_layers
                .iter()
                .map(|layer| layer.layer_path.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "root/active"]
        );
        assert_eq!(selection.selected_layer_edges.len(), 1);
        assert_eq!(selection.budget_report.active_layer_edge_count, 1);
        assert_eq!(selection.budget_report.excluded_demoted_layer_edge_count, 1);
        assert_eq!(
            selection.budget_report.excluded_tombstoned_layer_edge_count,
            1
        );
    }

    #[test]
    fn layered_retrieval_flat_fallback_is_visible_without_layer_evidence() {
        let request = layered_request(None);

        let selection =
            select_layered_retrieval_candidates(&request, &[], &[]).expect("flat fallback");

        assert!(selection.flat_fallback_used);
        assert!(selection.budget_report.flat_fallback_used);
        assert!(selection.selected_layers.is_empty());
        assert!(
            selection
                .warnings
                .contains(&"flat_fallback_no_layer_evidence".to_owned())
        );
    }

    #[test]
    fn layered_retrieval_rejects_tenant_mismatch_and_dangling_edges() {
        let request = layered_request(Some(hash(1)));
        let mut mismatched_layer = layer(1, "root", 0, "root", 10);
        mismatched_layer.tenant_id = "other-tenant".to_owned();
        assert!(matches!(
            select_layered_retrieval_candidates(&request, &[mismatched_layer], &[]),
            Err(KgRetrievalError::InvalidLayerGraph { .. })
        ));

        let layers = vec![layer(1, "root", 0, "root", 10)];
        let dangling = layer_edge(11, 1, 2, "contains_subgraph", 100);
        assert!(matches!(
            select_layered_retrieval_candidates(&request, &layers, &[dangling]),
            Err(KgRetrievalError::InvalidLayerGraph { .. })
        ));
    }

    #[test]
    fn request_validation_rejects_unsafe_shape() {
        let mut request = KgRetrievalRequest {
            tenant_id: "tenant".into(),
            namespace: "dag-db".into(),
            task_hash: Some("aa".repeat(32)),
            task_description: None,
            token_budget: 0,
            requested_memory_ids: Vec::new(),
            catalog_path: None,
            max_memory_refs: None,
            layer_path: None,
            max_layer_depth: None,
            max_layers_selected: None,
            max_nodes_per_layer: None,
            max_layer_edges: None,
        };
        assert!(request.validate().is_err());

        request.token_budget = 100;
        request.catalog_path = Some(vec!["..".into()]);
        assert!(request.validate().is_err());

        request.catalog_path = Some(vec!["KnowledgeGraphs".into(), "dag-db".into()]);
        request.tenant_id = " \n\t".into();
        assert!(request.validate().is_err());

        request.tenant_id = "tenant".into();
        request.task_description = Some("see /Users/example/.env".into());
        assert!(request.validate().is_err());

        request.task_description = Some("Bearer secret".into());
        assert!(request.validate().is_err());

        request.task_description = None;
        request.requested_memory_ids = vec!["10".repeat(32), "10".repeat(32)];
        assert!(request.validate().is_err());

        request.requested_memory_ids = vec!["not-a-hash".into()];
        assert!(request.validate().is_err());

        request.requested_memory_ids = Vec::new();
        request.catalog_path = Some(Vec::new());
        assert!(request.validate().is_err());

        for segment in [" ", "bad/segment", "bad\\segment"] {
            request.catalog_path = Some(vec![segment.into()]);
            assert!(request.validate().is_err());
        }
        request.catalog_path = None;
        for layer_path in ["", "/root", "root/", "root//child", "root/../child"] {
            request.layer_path = Some(layer_path.into());
            assert!(request.validate().is_err(), "layer_path {layer_path}");
        }
        request.layer_path = Some("root/repository".into());
        request.max_layer_depth = Some(KG_RETRIEVAL_HARD_MAX_LAYER_DEPTH + 1);
        assert!(request.validate().is_err());
        request.max_layer_depth = Some(0);
        request.max_layers_selected = Some(0);
        assert!(request.validate().is_err());
    }

    #[test]
    fn request_deserialization_rejects_mixed_case_raw_keys() {
        let request = serde_json::json!({
            "tenant_id": "tenant",
            "namespace": "dag-db",
            "task_hash": "aa".repeat(32),
            "task_description": null,
            "token_budget": 100,
            "requested_memory_ids": [],
            "catalog_path": ["KnowledgeGraphs", "dag-db"],
            "max_memory_refs": null,
            "Raw_Private_Payload": "forbidden"
        });

        assert!(serde_json::from_value::<KgRetrievalRequest>(request).is_err());
    }

    #[test]
    fn preview_hash_material_is_deterministic_and_compact() {
        let request = KgRetrievalRequest {
            tenant_id: "tenant".into(),
            namespace: "dag-db".into(),
            task_hash: Some("aa".repeat(32)),
            task_description: None,
            token_budget: 100,
            requested_memory_ids: vec!["10".repeat(32)],
            catalog_path: Some(vec!["KnowledgeGraphs".into(), "dag-db".into()]),
            max_memory_refs: Some(1),
            layer_path: Some("root/knowledge-graph".into()),
            max_layer_depth: Some(3),
            max_layers_selected: Some(4),
            max_nodes_per_layer: Some(12),
            max_layer_edges: Some(128),
        };
        request.validate().expect("valid request");
        assert_eq!(
            request.normalized_task_hash().expect("hash").to_string(),
            "aa".repeat(32)
        );
        let route = route_hint_id(&request, &request.requested_memory_ids).expect("route hint");
        let packet =
            context_packet_preview_id(&request, &route.to_string(), &request.requested_memory_ids)
                .expect("packet id");
        assert_eq!(
            packet.to_string(),
            context_packet_preview_id(&request, &route.to_string(), &request.requested_memory_ids)
                .expect("packet id")
                .to_string()
        );
        assert!(memory_token_estimate(&safe("title"), &safe("summary")) > 0);
    }

    #[test]
    fn retrieval_helpers_cover_optional_and_fail_closed_paths() {
        let request = KgRetrievalRequest {
            tenant_id: "tenant".into(),
            namespace: "dag-db".into(),
            task_hash: None,
            task_description: Some("compact task".into()),
            token_budget: 100,
            requested_memory_ids: Vec::new(),
            catalog_path: None,
            max_memory_refs: None,
            layer_path: None,
            max_layer_depth: None,
            max_layers_selected: None,
            max_nodes_per_layer: None,
            max_layer_edges: None,
        };
        let described_hash = request.normalized_task_hash().expect("task hash");
        let mut unspecified = request.clone();
        unspecified.task_description = None;
        assert_ne!(
            described_hash,
            unspecified
                .normalized_task_hash()
                .expect("default task hash")
        );

        let memory_id = "10".repeat(32);
        let with_catalog =
            citation_handle("tenant", "dag-db", &memory_id, Some("catalog")).expect("catalog cite");
        let without_catalog =
            citation_handle("tenant", "dag-db", &memory_id, None).expect("uncataloged cite");
        assert_ne!(with_catalog, without_catalog);

        assert!(matches!(
            hex_from_hash_column("memory_id", vec![1, 2, 3]),
            Err(KgRetrievalError::InvalidHashColumn { .. })
        ));

        let json_error = KgRetrievalError::Json {
            reason: "fixture".to_owned(),
        };
        assert!(json_error.to_string().contains("fixture"));
        let hash_error = hash_from_hex("task_hash", "not-hex").expect_err("hash is invalid");
        assert!(hash_error.to_string().contains("kg_import_hash_invalid"));
    }

    #[test]
    fn retrieval_rejects_each_forbidden_fragment_case_insensitively() {
        for fragment in FORBIDDEN_VALUE_FRAGMENTS {
            let request = KgRetrievalRequest {
                tenant_id: "tenant".into(),
                namespace: "dag-db".into(),
                task_hash: None,
                task_description: Some(format!("unsafe {}", fragment.to_ascii_uppercase())),
                token_budget: 100,
                requested_memory_ids: Vec::new(),
                catalog_path: None,
                max_memory_refs: None,
                layer_path: None,
                max_layer_depth: None,
                max_layers_selected: None,
                max_nodes_per_layer: None,
                max_layer_edges: None,
            };
            assert!(request.validate().is_err(), "fragment {fragment}");
        }
    }

    #[test]
    fn retrieval_rejects_review_named_forbidden_values() {
        for forbidden_value in [
            "~/dagdb",
            "/home/example/dagdb.md",
            r"C:\Users\example\dagdb.md",
            "DB_URL=redacted",
            "Bearer abc123",
            "source_excerpt leaked",
        ] {
            let request = KgRetrievalRequest {
                tenant_id: "tenant".into(),
                namespace: "dag-db".into(),
                task_hash: None,
                task_description: Some(forbidden_value.to_owned()),
                token_budget: 100,
                requested_memory_ids: Vec::new(),
                catalog_path: None,
                max_memory_refs: None,
                layer_path: None,
                max_layer_depth: None,
                max_layers_selected: None,
                max_nodes_per_layer: None,
                max_layer_edges: None,
            };
            assert!(
                request.validate().is_err(),
                "expected forbidden value to fail: {forbidden_value}"
            );
        }
    }
}
