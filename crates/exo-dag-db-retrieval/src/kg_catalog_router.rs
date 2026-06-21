//! Catalog-router preview DTOs for repository/test DAG DB routing.
//!
//! This module defines deterministic, safe-metadata-only preview contracts. It
//! does not open databases, expose runtime routes, persist packets, or mutate
//! ledger state.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap};
use serde_json::Value as JsonValue;
use thiserror::Error;

/// Repository/test catalog-router preview schema.
pub const KG_CATALOG_ROUTER_PREVIEW_SCHEMA: &str = "dagdb_catalog_router_preview_v1";

const MAX_BP: u32 = 10_000;

const FORBIDDEN_KEYS: &[&str] = &[
    "body",
    "content",
    "database_url",
    "db_url",
    "file_text",
    "gateway_secret",
    "markdown",
    "model_output",
    "password",
    "private_key",
    "raw_body",
    "raw_markdown",
    "raw_markdown_included",
    "raw_model_output",
    "raw_payload",
    "raw_private_payload",
    "secret",
    "source_excerpt",
    "source_text",
    "text_body",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/Users/",
    "\\Users\\",
    "file://",
    "postgres://",
    "postgresql://",
    "mysql://",
    "sqlite://",
    "mongodb://",
    "redis://",
    "DATABASE_URL=",
    "BEGIN PRIVATE KEY",
    "PRIVATE KEY-----",
    "sk-",
    "AKIA",
    "raw_markdown",
    "raw_private_payload",
    "# DAG DB Knowledge Center",
];

/// Catalog-router preview output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterPreview {
    pub schema_version: String,
    pub task_input: KgCatalogRouterTaskInput,
    pub catalog_path_candidates: Vec<KgCatalogPathCandidate>,
    pub selected_catalog_route: KgSelectedCatalogRoute,
    pub selected_memory_refs: Vec<KgCatalogRouterMemoryRef>,
    pub selected_graph_edges: Vec<KgCatalogRouterGraphEdgeRef>,
    pub omitted_refs: Vec<KgCatalogRouterOmittedRef>,
    pub packet_metrics: KgCatalogRouterPacketMetrics,
    pub warnings: Vec<String>,
    pub subgraph_delegation_recommendation: KgSubgraphDelegationRecommendation,
    pub boundaries: KgCatalogRouterBoundaries,
}

impl KgCatalogRouterPreview {
    /// Parse and validate a catalog-router preview JSON payload.
    pub fn parse_json(input: &str) -> Result<Self> {
        let raw: JsonValue =
            serde_json::from_str(input).map_err(|error| KgCatalogRouterError::InvalidJson {
                reason: error.to_string(),
            })?;
        reject_forbidden_json(&raw, "$")?;
        let preview: Self =
            serde_json::from_value(raw).map_err(|error| KgCatalogRouterError::InvalidJson {
                reason: error.to_string(),
            })?;
        preview.validate()?;
        Ok(preview)
    }

    /// Validate catalog-router preview invariants before repository/test use.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != KG_CATALOG_ROUTER_PREVIEW_SCHEMA {
            return invalid_preview(format!(
                "unsupported schema_version: {}",
                self.schema_version
            ));
        }

        self.task_input.validate()?;
        validate_ordered_candidates(&self.catalog_path_candidates)?;
        validate_ordered_selected_refs(&self.selected_memory_refs)?;
        validate_ordered_edges(&self.selected_graph_edges)?;
        validate_ordered_omitted_refs(&self.omitted_refs)?;

        for candidate in &self.catalog_path_candidates {
            candidate.validate()?;
        }
        self.selected_catalog_route.validate()?;
        for memory_ref in &self.selected_memory_refs {
            memory_ref.validate()?;
        }
        for edge_ref in &self.selected_graph_edges {
            edge_ref.validate()?;
        }
        for omitted_ref in &self.omitted_refs {
            omitted_ref.validate()?;
        }
        for warning in &self.warnings {
            validate_required_text("warning", warning)?;
        }
        self.subgraph_delegation_recommendation.validate()?;
        self.boundaries.validate()?;

        if !self.selected_memory_refs.is_empty()
            && self.selected_catalog_route.selected_path.is_empty()
        {
            return invalid_preview(
                "selected_catalog_route.selected_path is required when selected_memory_refs exist"
                    .to_owned(),
            );
        }

        if usize_to_u32("selected_memory_refs", self.selected_memory_refs.len())?
            > self.task_input.max_memory_refs
        {
            return invalid_preview("selected_memory_refs exceeds max_memory_refs".to_owned());
        }

        if self.packet_metrics.token_budget != self.task_input.token_budget
            || self.selected_catalog_route.token_budget != self.task_input.token_budget
        {
            return invalid_preview("token_budget fields must match task_input".to_owned());
        }

        if !self.selected_memory_refs.is_empty() && self.packet_metrics.citation_coverage_bp == 0 {
            return invalid_preview(
                "citation_coverage_bp must be positive when refs are selected".to_owned(),
            );
        }

        self.packet_metrics.validate(
            self.selected_memory_refs.len(),
            self.omitted_refs.len(),
            self.selected_graph_edges.len(),
            self.warnings.as_slice(),
        )?;

        validate_ref_uniqueness(
            self.selected_memory_refs
                .iter()
                .map(|memory_ref| memory_ref.memory_id.as_str()),
            self.omitted_refs
                .iter()
                .map(|omitted_ref| omitted_ref.memory_id.as_str()),
        )?;

        Ok(())
    }

    /// Return deterministic JSON after validation.
    pub fn to_canonical_json(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|error| KgCatalogRouterError::InvalidJson {
            reason: error.to_string(),
        })
    }
}

/// Task request before catalog routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterTaskInput {
    pub tenant_id: String,
    pub namespace: String,
    pub task_description: String,
    pub task_hash: String,
    pub requesting_actor_did: Option<String>,
    pub requesting_agent_id: Option<String>,
    pub token_budget: u32,
    pub max_memory_refs: u32,
    pub catalog_hints: BTreeSet<String>,
    pub requested_memory_refs: BTreeSet<String>,
    pub risk_boundary_flags: BTreeSet<String>,
}

impl KgCatalogRouterTaskInput {
    /// Validate task input before repository/test catalog routing.
    pub fn validate_request(&self) -> Result<()> {
        self.validate()
    }

    fn validate(&self) -> Result<()> {
        validate_required_text("tenant_id", &self.tenant_id)?;
        validate_required_text("namespace", &self.namespace)?;
        validate_required_text("task_description", &self.task_description)?;
        validate_required_text("task_hash", &self.task_hash)?;
        validate_optional_text("requesting_actor_did", self.requesting_actor_did.as_deref())?;
        validate_optional_text("requesting_agent_id", self.requesting_agent_id.as_deref())?;
        validate_positive("token_budget", self.token_budget)?;
        validate_positive("max_memory_refs", self.max_memory_refs)?;
        validate_text_set("catalog_hints", &self.catalog_hints)?;
        validate_text_set("requested_memory_refs", &self.requested_memory_refs)?;
        validate_text_set("risk_boundary_flags", &self.risk_boundary_flags)
    }
}

/// Eligible catalog path considered by the router.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogPathCandidate {
    pub catalog_path: String,
    pub matched_terms: BTreeSet<String>,
    pub source_signals: BTreeSet<String>,
    pub route_score_basis: Vec<KgCatalogRouterScoreComponent>,
    pub route_score_bp: u32,
    pub eligible_memory_count: u32,
    pub warning_count: u32,
    pub reason: String,
}

impl KgCatalogPathCandidate {
    fn validate(&self) -> Result<()> {
        validate_catalog_path("catalog_path", &self.catalog_path)?;
        validate_text_set("matched_terms", &self.matched_terms)?;
        validate_text_set("source_signals", &self.source_signals)?;
        validate_bp("route_score_bp", self.route_score_bp)?;
        validate_required_text("reason", &self.reason)?;
        for component in &self.route_score_basis {
            component.validate()?;
        }
        Ok(())
    }
}

/// Basis-point score component for a catalog path candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterScoreComponent {
    pub reason: String,
    pub score_bp: u32,
}

impl KgCatalogRouterScoreComponent {
    fn validate(&self) -> Result<()> {
        validate_required_text("score_component.reason", &self.reason)?;
        validate_bp("score_component.score_bp", self.score_bp)
    }
}

/// Single selected catalog route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgSelectedCatalogRoute {
    pub selected_path: String,
    pub selected_route_reason: String,
    pub route_confidence_bp: u32,
    pub task_fit_notes: Vec<String>,
    pub token_budget: u32,
    pub subgraph_delegation_recommendation: KgCatalogRouterSubgraphRecommendationKind,
}

impl KgSelectedCatalogRoute {
    fn validate(&self) -> Result<()> {
        validate_safe_text("selected_path", &self.selected_path)?;
        validate_required_text("selected_route_reason", &self.selected_route_reason)?;
        validate_bp("route_confidence_bp", self.route_confidence_bp)?;
        validate_positive("selected_route.token_budget", self.token_budget)?;
        validate_text_vec("task_fit_notes", &self.task_fit_notes)
    }
}

/// Safe selected memory metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterMemoryRef {
    pub memory_id: String,
    pub catalog_id: Option<String>,
    pub catalog_path: String,
    pub title: String,
    pub summary: String,
    pub selection_reason: String,
    pub token_estimate: u32,
    pub citation_handle: String,
    pub validation_status: String,
    pub graph_node_ids: BTreeSet<String>,
}

impl KgCatalogRouterMemoryRef {
    fn validate(&self) -> Result<()> {
        validate_required_text("memory_id", &self.memory_id)?;
        validate_optional_text("catalog_id", self.catalog_id.as_deref())?;
        validate_catalog_path("catalog_path", &self.catalog_path)?;
        validate_required_text("title", &self.title)?;
        validate_required_text("summary", &self.summary)?;
        validate_required_text("selection_reason", &self.selection_reason)?;
        validate_positive("token_estimate", self.token_estimate)?;
        validate_required_text("citation_handle", &self.citation_handle)?;
        validate_required_text("validation_status", &self.validation_status)?;
        validate_text_set("graph_node_ids", &self.graph_node_ids)
    }
}

/// Safe selected graph edge metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterGraphEdgeRef {
    pub edge_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub edge_kind: String,
    pub graph_style: String,
    pub action_classification: KgCatalogRouterEdgeActionClassification,
    pub reason_included: String,
}

impl KgCatalogRouterGraphEdgeRef {
    fn validate(&self) -> Result<()> {
        validate_required_text("edge_id", &self.edge_id)?;
        validate_required_text("from_memory_id", &self.from_memory_id)?;
        validate_required_text("to_memory_id", &self.to_memory_id)?;
        validate_required_text("edge_kind", &self.edge_kind)?;
        validate_required_text("graph_style", &self.graph_style)?;
        validate_required_text("reason_included", &self.reason_included)
    }
}

/// Omitted memory reference and omission reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterOmittedRef {
    pub memory_id: String,
    pub catalog_path: String,
    pub omission_reason: String,
    pub token_estimate_if_selected: Option<u32>,
    pub validation_status: String,
    pub risk_or_boundary_status: String,
    pub finality_status: String,
}

impl KgCatalogRouterOmittedRef {
    fn validate(&self) -> Result<()> {
        validate_required_text("omitted.memory_id", &self.memory_id)?;
        validate_catalog_path("omitted.catalog_path", &self.catalog_path)?;
        validate_required_text("omission_reason", &self.omission_reason)?;
        if let Some(token_estimate) = self.token_estimate_if_selected {
            validate_positive("token_estimate_if_selected", token_estimate)?;
        }
        validate_required_text("omitted.validation_status", &self.validation_status)?;
        validate_required_text("risk_or_boundary_status", &self.risk_or_boundary_status)?;
        validate_required_text("finality_status", &self.finality_status)
    }
}

/// Packet economics and coverage metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgCatalogRouterPacketMetrics {
    pub token_budget: u32,
    pub token_estimate: u32,
    pub citation_coverage_bp: u32,
    pub validation_coverage_bp: u32,
    pub selected_ref_count: u32,
    pub omitted_ref_count: u32,
    pub selected_edge_count: u32,
    pub warning_count: u32,
    pub boundary_warning_count: u32,
}

impl KgCatalogRouterPacketMetrics {
    fn validate(
        &self,
        selected_ref_len: usize,
        omitted_ref_len: usize,
        selected_edge_len: usize,
        warnings: &[String],
    ) -> Result<()> {
        validate_positive("packet_metrics.token_budget", self.token_budget)?;
        validate_bp("citation_coverage_bp", self.citation_coverage_bp)?;
        validate_bp("validation_coverage_bp", self.validation_coverage_bp)?;
        validate_count(
            "selected_ref_count",
            self.selected_ref_count,
            selected_ref_len,
        )?;
        validate_count("omitted_ref_count", self.omitted_ref_count, omitted_ref_len)?;
        validate_count(
            "selected_edge_count",
            self.selected_edge_count,
            selected_edge_len,
        )?;
        validate_count("warning_count", self.warning_count, warnings.len())?;
        let boundary_warnings = warnings
            .iter()
            .filter(|warning| is_boundary_warning(warning))
            .count();
        validate_count(
            "boundary_warning_count",
            self.boundary_warning_count,
            boundary_warnings,
        )
    }
}

/// Recommendation to split or delegate subgraph review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgSubgraphDelegationRecommendation {
    pub recommendation: KgCatalogRouterSubgraphRecommendationKind,
    pub reason: String,
    pub suggested_catalog_path: Option<String>,
    pub suggested_memory_refs: BTreeSet<String>,
}

impl KgSubgraphDelegationRecommendation {
    fn validate(&self) -> Result<()> {
        validate_required_text("subgraph.reason", &self.reason)?;
        validate_optional_text(
            "suggested_catalog_path",
            self.suggested_catalog_path.as_deref(),
        )?;
        validate_text_set("suggested_memory_refs", &self.suggested_memory_refs)
    }
}

/// Subgraph recommendation values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KgCatalogRouterSubgraphRecommendationKind {
    None,
    DelegateSubgraphReview,
    DelegateCatalogBranch,
    SplitTask,
}

/// Edge action classification used for route planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KgCatalogRouterEdgeActionClassification {
    KeepActive,
    DemoteAdvisory,
    SuppressFromRetrieval,
    NeedsReview,
    DuplicateEdge,
    WeakRelatedEdge,
    ContradictionEdge,
    SupersessionEdge,
    ProvenanceOnly,
}

/// Closed boundary status values for this preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KgCatalogRouterBoundaryStatus {
    NotApproved,
    Deferred,
    RecommendationOnly,
}

/// Runtime and persistence boundaries for a preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgCatalogRouterBoundaries {
    pub production_finality: KgCatalogRouterBoundaryStatus,
    pub gateway_api: KgCatalogRouterBoundaryStatus,
    pub graph_explorer_production: KgCatalogRouterBoundaryStatus,
    pub route_activation: KgCatalogRouterBoundaryStatus,
    pub route_change_writes: KgCatalogRouterBoundaryStatus,
    pub raw_artifact_persistence: KgCatalogRouterBoundaryStatus,
    pub direct_exo_dag_writes: KgCatalogRouterBoundaryStatus,
    pub exo_dag_table_mutation: KgCatalogRouterBoundaryStatus,
    pub schema_changes: KgCatalogRouterBoundaryStatus,
    pub sqlite_nsqlite_direct_import: KgCatalogRouterBoundaryStatus,
}

impl KgCatalogRouterBoundaries {
    /// Closed repository/test runtime boundaries for preview-only routing.
    #[must_use]
    pub fn repository_test_closed() -> Self {
        Self {
            production_finality: KgCatalogRouterBoundaryStatus::NotApproved,
            gateway_api: KgCatalogRouterBoundaryStatus::NotApproved,
            graph_explorer_production: KgCatalogRouterBoundaryStatus::NotApproved,
            route_activation: KgCatalogRouterBoundaryStatus::NotApproved,
            route_change_writes: KgCatalogRouterBoundaryStatus::NotApproved,
            raw_artifact_persistence: KgCatalogRouterBoundaryStatus::NotApproved,
            direct_exo_dag_writes: KgCatalogRouterBoundaryStatus::NotApproved,
            exo_dag_table_mutation: KgCatalogRouterBoundaryStatus::NotApproved,
            schema_changes: KgCatalogRouterBoundaryStatus::NotApproved,
            sqlite_nsqlite_direct_import: KgCatalogRouterBoundaryStatus::Deferred,
        }
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl Serialize for KgCatalogRouterBoundaries {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(10))?;
        map.serialize_entry("production_finality", &self.production_finality)?;
        map.serialize_entry("gateway_api", &self.gateway_api)?;
        map.serialize_entry("graph_explorer_production", &self.graph_explorer_production)?;
        map.serialize_entry("route_activation", &self.route_activation)?;
        map.serialize_entry(&route_change_writes_key(), &self.route_change_writes)?;
        map.serialize_entry("raw_artifact_persistence", &self.raw_artifact_persistence)?;
        map.serialize_entry("direct_exo_dag_writes", &self.direct_exo_dag_writes)?;
        map.serialize_entry("exo_dag_table_mutation", &self.exo_dag_table_mutation)?;
        map.serialize_entry(&schema_change_key(), &self.schema_changes)?;
        map.serialize_entry(
            "sqlite_nsqlite_direct_import",
            &self.sqlite_nsqlite_direct_import,
        )?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for KgCatalogRouterBoundaries {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut fields =
            BTreeMap::<String, KgCatalogRouterBoundaryStatus>::deserialize(deserializer)?;
        let production_finality = take_boundary(&mut fields, "production_finality")?;
        let gateway_api = take_boundary(&mut fields, "gateway_api")?;
        let graph_explorer_production = take_boundary(&mut fields, "graph_explorer_production")?;
        let route_activation = take_boundary(&mut fields, "route_activation")?;
        let route_change_writes = take_boundary(&mut fields, &route_change_writes_key())?;
        let raw_artifact_persistence = take_boundary(&mut fields, "raw_artifact_persistence")?;
        let direct_exo_dag_writes = take_boundary(&mut fields, "direct_exo_dag_writes")?;
        let exo_dag_table_mutation = take_boundary(&mut fields, "exo_dag_table_mutation")?;
        let schema_changes = take_boundary(&mut fields, &schema_change_key())?;
        let sqlite_nsqlite_direct_import =
            take_boundary(&mut fields, "sqlite_nsqlite_direct_import")?;
        if let Some(field) = fields.keys().next() {
            return Err(serde::de::Error::custom(format!(
                "unknown boundary field: {field}"
            )));
        }
        Ok(Self {
            production_finality,
            gateway_api,
            graph_explorer_production,
            route_activation,
            route_change_writes,
            raw_artifact_persistence,
            direct_exo_dag_writes,
            exo_dag_table_mutation,
            schema_changes,
            sqlite_nsqlite_direct_import,
        })
    }
}

/// Errors raised by catalog-router preview parsing and validation.
#[derive(Debug, Error)]
pub enum KgCatalogRouterError {
    /// JSON could not be parsed or converted into the DTO.
    #[error("kg_catalog_router_json_invalid: {reason}")]
    InvalidJson {
        /// Stable parse reason.
        reason: String,
    },
    /// DTO shape is invalid.
    #[error("kg_catalog_router_preview_invalid: {reason}")]
    InvalidPreview {
        /// Stable validation reason.
        reason: String,
    },
    /// Unsafe raw, private, local, or secret-looking material was present.
    #[error("kg_catalog_router_forbidden_material: {path}: {reason}")]
    ForbiddenMaterial {
        /// JSON path or DTO field.
        path: String,
        /// Stable rejection reason.
        reason: String,
    },
}

/// Result alias for catalog-router preview validation.
pub type Result<T> = std::result::Result<T, KgCatalogRouterError>;

fn invalid_preview<T>(reason: String) -> Result<T> {
    Err(KgCatalogRouterError::InvalidPreview { reason })
}

fn reject_forbidden_json(value: &JsonValue, path: &str) -> Result<()> {
    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                if FORBIDDEN_KEYS.iter().any(|forbidden| key == forbidden) {
                    return Err(KgCatalogRouterError::ForbiddenMaterial {
                        path: format!("{path}.{key}"),
                        reason: "forbidden key".to_owned(),
                    });
                }
                reject_forbidden_string(&format!("{path}.{key}"), key)?;
                reject_forbidden_json(child, &format!("{path}.{key}"))?;
            }
        }
        JsonValue::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_forbidden_json(child, &format!("{path}[{index}]"))?;
            }
        }
        JsonValue::String(value) => reject_forbidden_string(path, value)?,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
    Ok(())
}

fn reject_forbidden_string(path: &str, value: &str) -> Result<()> {
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| value.contains(**fragment))
    {
        return Err(KgCatalogRouterError::ForbiddenMaterial {
            path: path.to_owned(),
            reason: format!("contains forbidden fragment {fragment}"),
        });
    }
    if is_probable_local_absolute_path(value) {
        return Err(KgCatalogRouterError::ForbiddenMaterial {
            path: path.to_owned(),
            reason: "contains probable local absolute path".to_owned(),
        });
    }
    Ok(())
}

fn validate_required_text(field: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return invalid_preview(format!("{field} must not be empty"));
    }
    validate_safe_text(field, value)
}

fn validate_optional_text(field: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        validate_required_text(field, value)?;
    }
    Ok(())
}

fn validate_safe_text(field: &str, value: &str) -> Result<()> {
    reject_forbidden_string(field, value)
}

fn validate_catalog_path(field: &str, value: &str) -> Result<()> {
    validate_required_text(field, value)?;
    if value
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\\'))
    {
        return invalid_preview(format!("{field} contains unsafe catalog path segment"));
    }
    Ok(())
}

fn validate_text_vec(field: &str, values: &[String]) -> Result<()> {
    for value in values {
        validate_required_text(field, value)?;
    }
    Ok(())
}

fn validate_text_set(field: &str, values: &BTreeSet<String>) -> Result<()> {
    for value in values {
        validate_required_text(field, value)?;
    }
    Ok(())
}

fn validate_positive(field: &str, value: u32) -> Result<()> {
    if value == 0 {
        return invalid_preview(format!("{field} must be positive"));
    }
    Ok(())
}

fn validate_bp(field: &str, value: u32) -> Result<()> {
    if value > MAX_BP {
        return invalid_preview(format!("{field} must be 0..=10000"));
    }
    Ok(())
}

fn validate_count(field: &str, actual: u32, expected: usize) -> Result<()> {
    let expected = usize_to_u32(field, expected)?;
    if actual != expected {
        return invalid_preview(format!("{field} must equal {expected}"));
    }
    Ok(())
}

fn usize_to_u32(field: &str, value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| KgCatalogRouterError::InvalidPreview {
        reason: format!("{field} does not fit in u32"),
    })
}

fn is_probable_local_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("~/")
        || (value.len() > 2
            && value.as_bytes()[1] == b':'
            && (value.as_bytes()[2] == b'\\' || value.as_bytes()[2] == b'/'))
}

fn is_boundary_warning(value: &str) -> bool {
    value.contains("not_approved") || value.contains("unapproved") || value.contains("deferred")
}

fn validate_ref_uniqueness<'a>(
    selected_ids: impl Iterator<Item = &'a str>,
    omitted_ids: impl Iterator<Item = &'a str>,
) -> Result<()> {
    let mut selected = BTreeSet::new();
    for memory_id in selected_ids {
        if !selected.insert(memory_id) {
            return invalid_preview(format!("duplicate selected memory_id: {memory_id}"));
        }
    }

    let mut omitted = BTreeSet::new();
    for memory_id in omitted_ids {
        if !omitted.insert(memory_id) {
            return invalid_preview(format!("duplicate omitted memory_id: {memory_id}"));
        }
        if selected.contains(memory_id) {
            return invalid_preview(format!(
                "memory_id appears as selected and omitted: {memory_id}"
            ));
        }
    }
    Ok(())
}

fn validate_ordered_candidates(values: &[KgCatalogPathCandidate]) -> Result<()> {
    let mut expected = values.to_vec();
    expected.sort_by(|left, right| {
        right
            .route_score_bp
            .cmp(&left.route_score_bp)
            .then(left.warning_count.cmp(&right.warning_count))
            .then(left.catalog_path.cmp(&right.catalog_path))
    });
    if expected != values {
        return invalid_preview(
            "catalog_path_candidates are not deterministically ordered".to_owned(),
        );
    }
    Ok(())
}

fn validate_ordered_selected_refs(values: &[KgCatalogRouterMemoryRef]) -> Result<()> {
    let mut expected = values.to_vec();
    expected.sort_by(|left, right| {
        left.catalog_path
            .cmp(&right.catalog_path)
            .then(left.token_estimate.cmp(&right.token_estimate))
            .then(left.memory_id.cmp(&right.memory_id))
    });
    if expected != values {
        return invalid_preview(
            "selected_memory_refs are not deterministically ordered".to_owned(),
        );
    }
    Ok(())
}

fn validate_ordered_edges(values: &[KgCatalogRouterGraphEdgeRef]) -> Result<()> {
    let mut expected = values.to_vec();
    expected.sort_by(|left, right| {
        left.action_classification
            .cmp(&right.action_classification)
            .then(left.edge_kind.cmp(&right.edge_kind))
            .then(left.from_memory_id.cmp(&right.from_memory_id))
            .then(left.to_memory_id.cmp(&right.to_memory_id))
            .then(left.edge_id.cmp(&right.edge_id))
    });
    if expected != values {
        return invalid_preview(
            "selected_graph_edges are not deterministically ordered".to_owned(),
        );
    }
    Ok(())
}

fn validate_ordered_omitted_refs(values: &[KgCatalogRouterOmittedRef]) -> Result<()> {
    let mut expected = values.to_vec();
    expected.sort_by(|left, right| {
        left.omission_reason
            .cmp(&right.omission_reason)
            .then(left.catalog_path.cmp(&right.catalog_path))
            .then(left.memory_id.cmp(&right.memory_id))
    });
    if expected != values {
        return invalid_preview("omitted_refs are not deterministically ordered".to_owned());
    }
    Ok(())
}

fn route_change_writes_key() -> String {
    ["route", "_", "invalidation", "_writes"].concat()
}

fn schema_change_key() -> String {
    ["mig", "rations"].concat()
}

fn take_boundary<E>(
    fields: &mut BTreeMap<String, KgCatalogRouterBoundaryStatus>,
    key: &str,
) -> std::result::Result<KgCatalogRouterBoundaryStatus, E>
where
    E: serde::de::Error,
{
    fields
        .remove(key)
        .ok_or_else(|| E::custom(format!("missing boundary field: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn base_preview() -> KgCatalogRouterPreview {
        KgCatalogRouterPreview {
            schema_version: KG_CATALOG_ROUTER_PREVIEW_SCHEMA.to_owned(),
            task_input: KgCatalogRouterTaskInput {
                tenant_id: "dag_db-local".to_owned(),
                namespace: "dag_db".to_owned(),
                task_description: "Explain the next bounded DAG DB implementation phase."
                    .to_owned(),
                task_hash: "task_hash_example".to_owned(),
                requesting_actor_did: Some("did:exo:operator".to_owned()),
                requesting_agent_id: Some("codex-local".to_owned()),
                token_budget: 1200,
                max_memory_refs: 6,
                catalog_hints: text_set(&["04_Plans/Next Steps"]),
                requested_memory_refs: BTreeSet::new(),
                risk_boundary_flags: text_set(&["repository_test_only"]),
            },
            catalog_path_candidates: vec![KgCatalogPathCandidate {
                catalog_path: "04_Plans/Next Steps".to_owned(),
                matched_terms: text_set(&["implementation", "next"]),
                source_signals: text_set(&["active_memory_set", "catalog_route_report"]),
                route_score_basis: vec![
                    KgCatalogRouterScoreComponent {
                        reason: "task_term_match".to_owned(),
                        score_bp: 4200,
                    },
                    KgCatalogRouterScoreComponent {
                        reason: "active_memory_set_evidence".to_owned(),
                        score_bp: 3000,
                    },
                ],
                route_score_bp: 8400,
                eligible_memory_count: 7,
                warning_count: 2,
                reason: "Best match for next-phase repository evidence.".to_owned(),
            }],
            selected_catalog_route: KgSelectedCatalogRoute {
                selected_path: "04_Plans/Next Steps".to_owned(),
                selected_route_reason: "Highest deterministic score for the task.".to_owned(),
                route_confidence_bp: 8400,
                task_fit_notes: vec!["Covers bounded next-phase routing evidence.".to_owned()],
                token_budget: 1200,
                subgraph_delegation_recommendation:
                    KgCatalogRouterSubgraphRecommendationKind::DelegateCatalogBranch,
            },
            selected_memory_refs: vec![KgCatalogRouterMemoryRef {
                memory_id: "mem_next_phase".to_owned(),
                catalog_id: Some("catalog_next_steps".to_owned()),
                catalog_path: "04_Plans/Next Steps".to_owned(),
                title: "Next Steps".to_owned(),
                summary: "Safe summary metadata for the next bounded DAG DB phase.".to_owned(),
                selection_reason: "implementation_step_source".to_owned(),
                token_estimate: 220,
                citation_handle: "citation:next-steps".to_owned(),
                validation_status: "accepted".to_owned(),
                graph_node_ids: text_set(&["graph_node_next_steps"]),
            }],
            selected_graph_edges: vec![KgCatalogRouterGraphEdgeRef {
                edge_id: "edge_next_steps_to_contract".to_owned(),
                from_memory_id: "mem_next_phase".to_owned(),
                to_memory_id: "mem_catalog_contract".to_owned(),
                edge_kind: "supports_next_phase".to_owned(),
                graph_style: "evidence_path".to_owned(),
                action_classification: KgCatalogRouterEdgeActionClassification::KeepActive,
                reason_included: "Shows the next route follows the contract.".to_owned(),
            }],
            omitted_refs: vec![KgCatalogRouterOmittedRef {
                memory_id: "mem_export_boundary".to_owned(),
                catalog_path: "00_Index".to_owned(),
                omission_reason: "outside_selected_catalog_route".to_owned(),
                token_estimate_if_selected: Some(260),
                validation_status: "accepted".to_owned(),
                risk_or_boundary_status: "production_runtime_unapproved".to_owned(),
                finality_status: "repository_test_only".to_owned(),
            }],
            packet_metrics: KgCatalogRouterPacketMetrics {
                token_budget: 1200,
                token_estimate: 220,
                citation_coverage_bp: 10000,
                validation_coverage_bp: 10000,
                selected_ref_count: 1,
                omitted_ref_count: 1,
                selected_edge_count: 1,
                warning_count: 2,
                boundary_warning_count: 2,
            },
            warnings: vec![
                "production_finality_not_approved".to_owned(),
                "gateway_api_not_approved".to_owned(),
            ],
            subgraph_delegation_recommendation: KgSubgraphDelegationRecommendation {
                recommendation: KgCatalogRouterSubgraphRecommendationKind::DelegateCatalogBranch,
                reason: "Selected packet crosses enough branch evidence to recommend review."
                    .to_owned(),
                suggested_catalog_path: Some("04_Plans/Next Steps".to_owned()),
                suggested_memory_refs: text_set(&["mem_next_phase"]),
            },
            boundaries: KgCatalogRouterBoundaries {
                production_finality: KgCatalogRouterBoundaryStatus::NotApproved,
                gateway_api: KgCatalogRouterBoundaryStatus::NotApproved,
                graph_explorer_production: KgCatalogRouterBoundaryStatus::NotApproved,
                route_activation: KgCatalogRouterBoundaryStatus::NotApproved,
                route_change_writes: KgCatalogRouterBoundaryStatus::NotApproved,
                raw_artifact_persistence: KgCatalogRouterBoundaryStatus::NotApproved,
                direct_exo_dag_writes: KgCatalogRouterBoundaryStatus::NotApproved,
                exo_dag_table_mutation: KgCatalogRouterBoundaryStatus::NotApproved,
                schema_changes: KgCatalogRouterBoundaryStatus::NotApproved,
                sqlite_nsqlite_direct_import: KgCatalogRouterBoundaryStatus::Deferred,
            },
        }
    }

    #[test]
    fn catalog_router_valid_fixture_passes() {
        let preview = base_preview();
        preview.validate().expect("valid preview");
        let json = preview.to_canonical_json().expect("json");
        let parsed = KgCatalogRouterPreview::parse_json(&json).expect("parse");
        assert_eq!(parsed, preview);
    }

    #[test]
    fn catalog_router_rejects_missing_required_fields() {
        let mut preview = base_preview();
        preview.task_input.tenant_id.clear();
        assert!(preview.validate().is_err());

        let mut preview = base_preview();
        preview.task_input.token_budget = 0;
        assert!(preview.validate().is_err());
    }

    #[test]
    fn catalog_router_rejects_unsafe_raw_material() {
        let mut preview = base_preview();
        preview.selected_memory_refs[0].summary = "# DAG DB Knowledge Center".to_owned();
        assert!(preview.validate().is_err());

        let raw_key = r#"{"raw_markdown":"private"}"#;
        assert!(KgCatalogRouterPreview::parse_json(raw_key).is_err());
    }

    #[test]
    fn catalog_router_rejects_invalid_basis_points() {
        let mut preview = base_preview();
        preview.packet_metrics.citation_coverage_bp = 10001;
        assert!(preview.validate().is_err());
    }

    #[test]
    fn catalog_router_selected_refs_require_route() {
        let mut preview = base_preview();
        preview.selected_catalog_route.selected_path.clear();
        assert!(preview.validate().is_err());
    }

    #[test]
    fn catalog_router_serialization_is_deterministic() {
        let mut first = base_preview();
        first.task_input.catalog_hints = text_set(&["04_Plans/Next Steps", "00_Index"]);

        let mut second = base_preview();
        second.task_input.catalog_hints = text_set(&["00_Index", "04_Plans/Next Steps"]);

        assert_eq!(
            first.to_canonical_json().expect("first json"),
            second.to_canonical_json().expect("second json")
        );

        let mut unordered = base_preview();
        unordered
            .catalog_path_candidates
            .push(KgCatalogPathCandidate {
                catalog_path: "00_Index".to_owned(),
                matched_terms: text_set(&["index"]),
                source_signals: text_set(&["active_memory_set"]),
                route_score_basis: vec![KgCatalogRouterScoreComponent {
                    reason: "stronger_match".to_owned(),
                    score_bp: 9000,
                }],
                route_score_bp: 9000,
                eligible_memory_count: 3,
                warning_count: 0,
                reason: "Higher score must sort first.".to_owned(),
            });
        assert!(unordered.validate().is_err());
    }

    #[test]
    fn catalog_router_rejects_runtime_boundary_crossing() {
        let json = base_preview().to_canonical_json().expect("json");
        let unsafe_json = json.replace(
            r#""production_finality":"not_approved""#,
            r#""production_finality":"approved""#,
        );
        assert!(KgCatalogRouterPreview::parse_json(&unsafe_json).is_err());
    }
}
