//! Contract-only hybrid retrieval preview over existing KG and graph selectors.
//!
//! This module does not call providers, open databases, activate routes, persist
//! packets, or return raw source material. It reconciles already-built
//! repository/test KG retrieval previews with graph context selection output and
//! selects only memory refs that both inputs agree on.

use std::collections::{BTreeMap, BTreeSet};

use exo_dag_db_api::{
    DagDbGraphContextSelectionResponse, DagDbSelectedContextRef, DagDbSelectedGraphEdgeRef,
    MemoryEdgeKind, MemoryGraphStyle,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::kg_retrieval::{
    KG_CONTEXT_PACKET_PREVIEW_SCHEMA, KgContextPacketPreview, KgGraphEdgeRef, KgMemoryRef,
};

/// Repository/test hybrid retrieval schema.
pub const HYBRID_RETRIEVAL_CONTRACT_SCHEMA: &str = "dagdb_hybrid_retrieval_contract_v1";

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
    "raw_model_output",
    "raw_payload",
    "raw_private_payload",
    "secret",
    "source_excerpt",
    "source_path",
    "text_body",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "authorization",
    "bearer ",
    "begin private key",
    "database_url",
    "db_url",
    "file://",
    "password",
    "postgres://",
    "postgresql://",
    "private key-----",
    "raw_body",
    "raw_markdown",
    "raw_model_output",
    "raw_private_payload",
    "secret",
    "sk-proj-",
    "source_excerpt",
];

/// Request envelope for contract-only hybrid retrieval reconciliation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task_hash: String,
    pub token_budget: u32,
    pub max_memory_refs: u32,
}

/// Contract-only hybrid retrieval preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalPreview {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task_hash: String,
    pub kg_context_packet_id: String,
    pub kg_route_hint_id: String,
    pub selected_memory_refs: Vec<HybridRetrievalMemoryRef>,
    pub selected_graph_edges: Vec<HybridRetrievalGraphEdgeRef>,
    pub omitted_memory_refs: Vec<HybridRetrievalOmittedRef>,
    pub diagnostics: HybridRetrievalDiagnostics,
    pub warnings: Vec<String>,
    pub acceptance: HybridRetrievalAcceptance,
}

impl HybridRetrievalPreview {
    /// Validate the contract preview before repository/test use.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != HYBRID_RETRIEVAL_CONTRACT_SCHEMA {
            return invalid_preview(format!(
                "unsupported schema_version: {}",
                self.schema_version
            ));
        }
        validate_required_text("tenant_id", &self.tenant_id)?;
        validate_required_text("namespace", &self.namespace)?;
        validate_required_text("request_id", &self.request_id)?;
        validate_required_text("task_hash", &self.task_hash)?;
        validate_required_text("kg_context_packet_id", &self.kg_context_packet_id)?;
        validate_required_text("kg_route_hint_id", &self.kg_route_hint_id)?;
        validate_unique_selected_refs(&self.selected_memory_refs)?;
        validate_unique_omitted_refs(&self.omitted_memory_refs)?;
        for memory_ref in &self.selected_memory_refs {
            memory_ref.validate()?;
        }
        for graph_edge in &self.selected_graph_edges {
            graph_edge.validate()?;
        }
        for omitted in &self.omitted_memory_refs {
            omitted.validate()?;
        }
        for warning in &self.warnings {
            validate_required_text("warning", warning)?;
        }
        self.diagnostics.validate(
            self.selected_memory_refs.len(),
            self.selected_graph_edges.len(),
            self.omitted_memory_refs.len(),
        )?;
        self.acceptance.validate()?;
        if self.diagnostics.token_estimate > self.diagnostics.token_budget {
            return invalid_preview("token_estimate exceeds token_budget".to_owned());
        }
        if usize_to_u32("selected_memory_refs", self.selected_memory_refs.len())?
            > self.diagnostics.max_memory_refs
        {
            return invalid_preview("selected_memory_refs exceeds max_memory_refs".to_owned());
        }
        reject_forbidden_json(&serde_json::to_value(self).map_err(|error| {
            HybridRetrievalError::InvalidJson {
                reason: error.to_string(),
            }
        })?)?;
        Ok(())
    }

    /// Return deterministic JSON after validation.
    pub fn to_canonical_json(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|error| HybridRetrievalError::InvalidJson {
            reason: error.to_string(),
        })
    }
}

/// Memory ref selected only after KG preview and graph selection agree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalMemoryRef {
    pub memory_id: String,
    pub kg_catalog_id: Option<String>,
    pub graph_catalog_id: Option<String>,
    pub kg_citation_handle: String,
    pub graph_citation_ref: String,
    pub kg_token_estimate: u32,
    pub graph_token_estimate: u32,
    pub hybrid_token_estimate: u32,
    pub selection_reasons: Vec<String>,
}

impl HybridRetrievalMemoryRef {
    fn validate(&self) -> Result<()> {
        validate_required_text("memory_id", &self.memory_id)?;
        validate_optional_text("kg_catalog_id", self.kg_catalog_id.as_deref())?;
        validate_optional_text("graph_catalog_id", self.graph_catalog_id.as_deref())?;
        validate_required_text("kg_citation_handle", &self.kg_citation_handle)?;
        validate_required_text("graph_citation_ref", &self.graph_citation_ref)?;
        if self.hybrid_token_estimate < self.kg_token_estimate
            || self.hybrid_token_estimate < self.graph_token_estimate
        {
            return invalid_preview(
                "hybrid_token_estimate must cover both source estimates".to_owned(),
            );
        }
        validate_text_list("selection_reason", &self.selection_reasons)
    }
}

/// Graph edge visible in one of the agreed selected-memory subgraphs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalGraphEdgeRef {
    pub graph_edge_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub edge_kind: String,
    pub graph_style: String,
    pub source: String,
}

impl HybridRetrievalGraphEdgeRef {
    fn validate(&self) -> Result<()> {
        validate_required_text("graph_edge_id", &self.graph_edge_id)?;
        validate_required_text("from_memory_id", &self.from_memory_id)?;
        validate_required_text("to_memory_id", &self.to_memory_id)?;
        validate_required_text("edge_kind", &self.edge_kind)?;
        validate_required_text("graph_style", &self.graph_style)?;
        validate_required_text("source", &self.source)
    }
}

/// Omitted memory ref with a stable, reviewable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalOmittedRef {
    pub memory_id: String,
    pub reason: String,
    pub token_estimate_if_selected: Option<u32>,
}

impl HybridRetrievalOmittedRef {
    fn validate(&self) -> Result<()> {
        validate_required_text("memory_id", &self.memory_id)?;
        validate_required_text("reason", &self.reason)
    }
}

/// Deterministic hybrid retrieval diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalDiagnostics {
    pub kg_selected_memory_count: u32,
    pub graph_selected_memory_count: u32,
    pub hybrid_selected_memory_count: u32,
    pub kg_only_memory_count: u32,
    pub graph_only_memory_count: u32,
    pub selected_graph_edge_count: u32,
    pub omitted_memory_count: u32,
    pub token_budget: u32,
    pub max_memory_refs: u32,
    pub token_estimate: u32,
    pub deterministic_ordering: bool,
    pub preview_only: bool,
    pub provider_calls_made: bool,
    pub live_database_required: bool,
    pub raw_material_returned: bool,
}

impl HybridRetrievalDiagnostics {
    fn validate(
        &self,
        selected_memory_count: usize,
        selected_graph_edge_count: usize,
        omitted_memory_count: usize,
    ) -> Result<()> {
        if self.hybrid_selected_memory_count
            != usize_to_u32("selected_memory_count", selected_memory_count)?
            || self.selected_graph_edge_count
                != usize_to_u32("selected_graph_edge_count", selected_graph_edge_count)?
            || self.omitted_memory_count
                != usize_to_u32("omitted_memory_count", omitted_memory_count)?
        {
            return invalid_preview(
                "diagnostic counts do not match preview collections".to_owned(),
            );
        }
        if !self.deterministic_ordering
            || !self.preview_only
            || self.provider_calls_made
            || self.live_database_required
            || self.raw_material_returned
        {
            return invalid_preview("diagnostic boundary flags are not contract-only".to_owned());
        }
        Ok(())
    }
}

/// Explicit non-activation flags for the contract preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRetrievalAcceptance {
    pub preview_only: bool,
    pub provider_retrieval_implemented: bool,
    pub live_database_reads_implemented: bool,
    pub live_database_writes_implemented: bool,
    pub route_activation_implemented: bool,
    pub raw_material_returned: bool,
}

impl HybridRetrievalAcceptance {
    fn validate(&self) -> Result<()> {
        if !self.preview_only
            || self.provider_retrieval_implemented
            || self.live_database_reads_implemented
            || self.live_database_writes_implemented
            || self.route_activation_implemented
            || self.raw_material_returned
        {
            return invalid_preview("acceptance flags must remain preview-only".to_owned());
        }
        Ok(())
    }
}

/// Errors raised by contract-only hybrid retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HybridRetrievalError {
    /// Request shape is invalid or unsafe.
    #[error("hybrid_retrieval_request_invalid: {reason}")]
    InvalidRequest {
        /// Stable validation reason.
        reason: String,
    },
    /// Input preview evidence is missing, stale, unsafe, or cross-scope.
    #[error("hybrid_retrieval_evidence_invalid: {reason}")]
    InvalidEvidence {
        /// Stable evidence reason.
        reason: String,
    },
    /// Preview JSON conversion failed.
    #[error("hybrid_retrieval_json_invalid: {reason}")]
    InvalidJson {
        /// Stable conversion reason.
        reason: String,
    },
    /// Integer conversion overflowed.
    #[error("hybrid_retrieval_arithmetic_overflow: {field}")]
    ArithmeticOverflow {
        /// Field that overflowed.
        field: &'static str,
    },
}

/// Result alias for hybrid retrieval contract work.
pub type Result<T> = std::result::Result<T, HybridRetrievalError>;

/// Build a DB-free hybrid retrieval preview from existing preview/selection outputs.
pub fn build_hybrid_retrieval_preview(
    request: &HybridRetrievalRequest,
    kg_preview: &KgContextPacketPreview,
    graph_selection: &DagDbGraphContextSelectionResponse,
) -> Result<HybridRetrievalPreview> {
    validate_inputs(request, kg_preview, graph_selection)?;

    let kg_refs = index_kg_memory_refs(&kg_preview.memory_refs)?;
    let graph_refs = index_graph_memory_refs(&graph_selection.selected_memory_refs)?;
    let kg_ids = kg_refs.keys().cloned().collect::<BTreeSet<_>>();
    let graph_ids = graph_refs.keys().cloned().collect::<BTreeSet<_>>();

    let mut selected_memory_refs = Vec::new();
    let mut omitted_memory_refs = Vec::new();
    let mut selected_ids = BTreeSet::new();
    let mut token_estimate = 0u32;
    let mut truncated_by_token_budget = false;
    let mut truncated_by_max_memory_refs = false;

    for graph_ref in &graph_selection.selected_memory_refs {
        let Some(kg_ref) = kg_refs.get(&graph_ref.memory_id) else {
            continue;
        };
        let hybrid_ref = build_hybrid_memory_ref(kg_ref, graph_ref)?;
        if selected_memory_refs.len() >= usize_from_u32(request.max_memory_refs)? {
            truncated_by_max_memory_refs = true;
            omitted_memory_refs.push(HybridRetrievalOmittedRef {
                memory_id: graph_ref.memory_id.clone(),
                reason: "max_memory_refs_exceeded".to_owned(),
                token_estimate_if_selected: Some(hybrid_ref.hybrid_token_estimate),
            });
            continue;
        }
        let next_total = token_estimate.saturating_add(hybrid_ref.hybrid_token_estimate);
        if next_total > request.token_budget {
            truncated_by_token_budget = true;
            omitted_memory_refs.push(HybridRetrievalOmittedRef {
                memory_id: graph_ref.memory_id.clone(),
                reason: "token_budget_exceeded".to_owned(),
                token_estimate_if_selected: Some(hybrid_ref.hybrid_token_estimate),
            });
            continue;
        }
        token_estimate = next_total;
        selected_ids.insert(graph_ref.memory_id.clone());
        selected_memory_refs.push(hybrid_ref);
    }

    for memory_id in kg_ids.difference(&graph_ids) {
        let kg_ref =
            kg_refs
                .get(memory_id)
                .ok_or_else(|| HybridRetrievalError::InvalidEvidence {
                    reason: "kg memory index drifted during omission calculation".to_owned(),
                })?;
        omitted_memory_refs.push(HybridRetrievalOmittedRef {
            memory_id: (*memory_id).clone(),
            reason: "graph_selection_missing".to_owned(),
            token_estimate_if_selected: Some(kg_ref.token_estimate),
        });
    }

    for memory_id in graph_ids.difference(&kg_ids) {
        let graph_ref =
            graph_refs
                .get(memory_id)
                .ok_or_else(|| HybridRetrievalError::InvalidEvidence {
                    reason: "graph memory index drifted during omission calculation".to_owned(),
                })?;
        omitted_memory_refs.push(HybridRetrievalOmittedRef {
            memory_id: (*memory_id).clone(),
            reason: "kg_preview_missing".to_owned(),
            token_estimate_if_selected: Some(graph_ref.token_estimate),
        });
    }

    sort_omitted_refs(&mut omitted_memory_refs);
    let selected_graph_edges = build_hybrid_graph_edges(kg_preview, graph_selection, &selected_ids);
    let kg_only_memory_count = usize_to_u32(
        "kg_only_memory_count",
        kg_ids.difference(&graph_ids).count(),
    )?;
    let graph_only_memory_count = usize_to_u32(
        "graph_only_memory_count",
        graph_ids.difference(&kg_ids).count(),
    )?;
    let warnings = build_warnings(
        &selected_memory_refs,
        kg_only_memory_count,
        graph_only_memory_count,
        truncated_by_token_budget,
        truncated_by_max_memory_refs,
        kg_preview,
        graph_selection,
    );

    let preview = HybridRetrievalPreview {
        schema_version: HYBRID_RETRIEVAL_CONTRACT_SCHEMA.to_owned(),
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task_hash: request.task_hash.clone(),
        kg_context_packet_id: kg_preview.context_packet_id.clone(),
        kg_route_hint_id: kg_preview.route_hint_id.clone(),
        selected_memory_refs,
        selected_graph_edges,
        omitted_memory_refs,
        diagnostics: HybridRetrievalDiagnostics {
            kg_selected_memory_count: usize_to_u32("kg_selected_memory_count", kg_ids.len())?,
            graph_selected_memory_count: usize_to_u32(
                "graph_selected_memory_count",
                graph_ids.len(),
            )?,
            hybrid_selected_memory_count: usize_to_u32(
                "hybrid_selected_memory_count",
                selected_ids.len(),
            )?,
            kg_only_memory_count,
            graph_only_memory_count,
            selected_graph_edge_count: 0,
            omitted_memory_count: 0,
            token_budget: request.token_budget,
            max_memory_refs: request.max_memory_refs,
            token_estimate,
            deterministic_ordering: true,
            preview_only: true,
            provider_calls_made: false,
            live_database_required: false,
            raw_material_returned: false,
        },
        warnings,
        acceptance: HybridRetrievalAcceptance {
            preview_only: true,
            provider_retrieval_implemented: false,
            live_database_reads_implemented: false,
            live_database_writes_implemented: false,
            route_activation_implemented: false,
            raw_material_returned: false,
        },
    };

    let mut preview = preview;
    preview.diagnostics.selected_graph_edge_count = usize_to_u32(
        "selected_graph_edge_count",
        preview.selected_graph_edges.len(),
    )?;
    preview.diagnostics.omitted_memory_count =
        usize_to_u32("omitted_memory_count", preview.omitted_memory_refs.len())?;
    preview.validate()?;
    Ok(preview)
}

fn validate_inputs(
    request: &HybridRetrievalRequest,
    kg_preview: &KgContextPacketPreview,
    graph_selection: &DagDbGraphContextSelectionResponse,
) -> Result<()> {
    request.validate()?;
    if kg_preview.schema_version != KG_CONTEXT_PACKET_PREVIEW_SCHEMA {
        return invalid_evidence("kg preview schema is unsupported");
    }
    if !kg_preview.dry_run_or_preview_only || !kg_preview.retrieval_diagnostics.preview_only {
        return invalid_evidence("kg preview must be preview-only");
    }
    if kg_preview.retrieval_diagnostics.raw_markdown_returned {
        return invalid_evidence("kg preview returned raw material");
    }
    if kg_preview.tenant_id != request.tenant_id || kg_preview.namespace != request.namespace {
        return Err(HybridRetrievalError::InvalidEvidence {
            reason: "kg preview tenant/namespace does not match request".to_owned(),
        });
    }
    if graph_selection.tenant_id != request.tenant_id
        || graph_selection.namespace != request.namespace
    {
        return Err(HybridRetrievalError::InvalidEvidence {
            reason: "graph selection tenant/namespace does not match request".to_owned(),
        });
    }
    if graph_selection.task_hash != request.task_hash {
        return invalid_evidence("graph selection task_hash does not match request");
    }
    if kg_preview.token_budget != request.token_budget
        || graph_selection.token_budget != request.token_budget
    {
        return invalid_evidence("input token budgets must match request");
    }
    for memory_ref in &kg_preview.memory_refs {
        if memory_ref.source_path.is_some() {
            return invalid_evidence("kg preview source material must not be present");
        }
    }
    Ok(())
}

impl HybridRetrievalRequest {
    /// Validate request fields before preview reconciliation.
    pub fn validate(&self) -> Result<()> {
        validate_required_text("tenant_id", &self.tenant_id)?;
        validate_required_text("namespace", &self.namespace)?;
        validate_required_text("request_id", &self.request_id)?;
        validate_required_text("task_hash", &self.task_hash)?;
        if self.token_budget == 0 {
            return invalid_request("token_budget must be positive");
        }
        if self.max_memory_refs == 0 {
            return invalid_request("max_memory_refs must be positive");
        }
        Ok(())
    }
}

fn build_hybrid_memory_ref(
    kg_ref: &KgMemoryRef,
    graph_ref: &DagDbSelectedContextRef,
) -> Result<HybridRetrievalMemoryRef> {
    let mut reasons = BTreeSet::new();
    reasons.insert("kg_retrieval_preview_selected".to_owned());
    reasons.insert("graph_context_selection_selected".to_owned());
    for reason in &kg_ref.selection_reasons {
        validate_required_text("kg_selection_reason", reason)?;
        reasons.insert(format!("kg:{reason}"));
    }
    validate_required_text("graph_selection_reason", &graph_ref.selection_reason)?;
    reasons.insert(format!("graph:{}", graph_ref.selection_reason));

    Ok(HybridRetrievalMemoryRef {
        memory_id: graph_ref.memory_id.clone(),
        kg_catalog_id: kg_ref.catalog_id.clone(),
        graph_catalog_id: graph_ref.catalog_id.clone(),
        kg_citation_handle: kg_ref.citation_handle.clone(),
        graph_citation_ref: graph_ref.citation_ref.clone(),
        kg_token_estimate: kg_ref.token_estimate,
        graph_token_estimate: graph_ref.token_estimate,
        hybrid_token_estimate: kg_ref.token_estimate.max(graph_ref.token_estimate),
        selection_reasons: reasons.into_iter().collect(),
    })
}

fn build_hybrid_graph_edges(
    kg_preview: &KgContextPacketPreview,
    graph_selection: &DagDbGraphContextSelectionResponse,
    selected_ids: &BTreeSet<String>,
) -> Vec<HybridRetrievalGraphEdgeRef> {
    let mut edges = BTreeMap::<String, HybridRetrievalGraphEdgeRef>::new();
    for edge in &kg_preview.graph_edges {
        if selected_ids.contains(&edge.from_memory_id) && selected_ids.contains(&edge.to_memory_id)
        {
            let key = format!("kg:{}", edge.graph_edge_id);
            edges.insert(key, kg_graph_edge(edge));
        }
    }
    for edge in &graph_selection.selected_graph_edges {
        if selected_ids.contains(&edge.from_memory_id) && selected_ids.contains(&edge.to_memory_id)
        {
            let key = format!("graph:{}", edge.graph_edge_id);
            edges.insert(key, graph_selection_edge(edge));
        }
    }
    edges.into_values().collect()
}

fn kg_graph_edge(edge: &KgGraphEdgeRef) -> HybridRetrievalGraphEdgeRef {
    HybridRetrievalGraphEdgeRef {
        graph_edge_id: edge.graph_edge_id.clone(),
        from_memory_id: edge.from_memory_id.clone(),
        to_memory_id: edge.to_memory_id.clone(),
        edge_kind: edge.edge_kind.clone(),
        graph_style: edge.graph_style.clone(),
        source: "kg_retrieval_preview".to_owned(),
    }
}

fn graph_selection_edge(edge: &DagDbSelectedGraphEdgeRef) -> HybridRetrievalGraphEdgeRef {
    HybridRetrievalGraphEdgeRef {
        graph_edge_id: edge.graph_edge_id.clone(),
        from_memory_id: edge.from_memory_id.clone(),
        to_memory_id: edge.to_memory_id.clone(),
        edge_kind: edge_kind_label(edge.edge_kind).to_owned(),
        graph_style: graph_style_label(edge.graph_style).to_owned(),
        source: "graph_context_selection".to_owned(),
    }
}

fn build_warnings(
    selected_memory_refs: &[HybridRetrievalMemoryRef],
    kg_only_memory_count: u32,
    graph_only_memory_count: u32,
    truncated_by_token_budget: bool,
    truncated_by_max_memory_refs: bool,
    kg_preview: &KgContextPacketPreview,
    graph_selection: &DagDbGraphContextSelectionResponse,
) -> Vec<String> {
    let mut warnings = vec![
        "hybrid_retrieval_contract_preview_only".to_owned(),
        "provider_calls_not_implemented".to_owned(),
        "live_database_not_required".to_owned(),
        "route_activation_not_approved".to_owned(),
        "raw_source_material_not_returned".to_owned(),
    ];
    if selected_memory_refs.is_empty() {
        push_warning(&mut warnings, "no_selected_hybrid_memory_refs");
        if !kg_preview.memory_refs.is_empty() && !graph_selection.selected_memory_refs.is_empty() {
            push_warning(&mut warnings, "no_hybrid_overlap");
        }
    }
    if kg_only_memory_count > 0 {
        push_warning(&mut warnings, "kg_refs_not_graph_selected");
    }
    if graph_only_memory_count > 0 {
        push_warning(&mut warnings, "graph_refs_missing_from_kg_preview");
    }
    if truncated_by_token_budget {
        push_warning(&mut warnings, "context_truncated_by_token_budget");
    }
    if truncated_by_max_memory_refs {
        push_warning(&mut warnings, "context_truncated_by_max_memory_refs");
    }
    if !kg_preview.warnings.is_empty() {
        push_warning(&mut warnings, "kg_preview_warnings_present");
    }
    if !graph_selection.boundary_warnings.is_empty() {
        push_warning(&mut warnings, "graph_selection_warnings_present");
    }
    warnings
}

fn index_kg_memory_refs(memory_refs: &[KgMemoryRef]) -> Result<BTreeMap<String, &KgMemoryRef>> {
    let mut indexed = BTreeMap::new();
    for memory_ref in memory_refs {
        validate_required_text("kg_memory_id", &memory_ref.memory_id)?;
        if indexed
            .insert(memory_ref.memory_id.clone(), memory_ref)
            .is_some()
        {
            return invalid_evidence("duplicate kg memory ref");
        }
    }
    Ok(indexed)
}

fn index_graph_memory_refs(
    memory_refs: &[DagDbSelectedContextRef],
) -> Result<BTreeMap<String, &DagDbSelectedContextRef>> {
    let mut indexed = BTreeMap::new();
    for memory_ref in memory_refs {
        validate_required_text("graph_memory_id", &memory_ref.memory_id)?;
        if indexed
            .insert(memory_ref.memory_id.clone(), memory_ref)
            .is_some()
        {
            return invalid_evidence("duplicate graph memory ref");
        }
    }
    Ok(indexed)
}

fn validate_unique_selected_refs(memory_refs: &[HybridRetrievalMemoryRef]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for memory_ref in memory_refs {
        if !seen.insert(memory_ref.memory_id.clone()) {
            return invalid_preview("duplicate selected memory ref".to_owned());
        }
    }
    Ok(())
}

fn validate_unique_omitted_refs(memory_refs: &[HybridRetrievalOmittedRef]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for memory_ref in memory_refs {
        if !seen.insert(memory_ref.memory_id.clone()) {
            return invalid_preview("duplicate omitted memory ref".to_owned());
        }
    }
    Ok(())
}

fn sort_omitted_refs(memory_refs: &mut [HybridRetrievalOmittedRef]) {
    memory_refs.sort_by(|left, right| {
        left.reason
            .cmp(&right.reason)
            .then(left.memory_id.cmp(&right.memory_id))
    });
}

fn validate_text_list(field: &str, values: &[String]) -> Result<()> {
    if values.is_empty() {
        return invalid_preview(format!("{field} list must not be empty"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_required_text(field, value)?;
        if !seen.insert(value) {
            return invalid_preview(format!("duplicate {field}"));
        }
    }
    Ok(())
}

fn validate_optional_text(field: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => validate_required_text(field, value),
        None => Ok(()),
    }
}

fn validate_required_text(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return invalid_request(&format!("{field} must not be empty"));
    }
    reject_forbidden_string(field, value)
}

fn reject_forbidden_json(value: &JsonValue) -> Result<()> {
    match value {
        JsonValue::Object(values) => {
            for (key, nested) in values {
                let normalized_key = key.to_ascii_lowercase();
                if FORBIDDEN_KEYS.contains(&normalized_key.as_str()) {
                    return invalid_preview(format!("forbidden key {key}"));
                }
                reject_forbidden_json(nested)?;
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                reject_forbidden_json(value)?;
            }
        }
        JsonValue::String(text) => reject_forbidden_string("json_value", text)?,
        JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::Null => {}
    }
    Ok(())
}

fn reject_forbidden_string(field: &str, value: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| lowered.contains(**fragment))
    {
        return Err(HybridRetrievalError::InvalidRequest {
            reason: format!("{field} contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

fn push_warning(warnings: &mut Vec<String>, warning: impl Into<String>) {
    let warning = warning.into();
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn invalid_request<T>(reason: &str) -> Result<T> {
    Err(HybridRetrievalError::InvalidRequest {
        reason: reason.to_owned(),
    })
}

fn invalid_evidence<T>(reason: &str) -> Result<T> {
    Err(HybridRetrievalError::InvalidEvidence {
        reason: reason.to_owned(),
    })
}

fn invalid_preview<T>(reason: String) -> Result<T> {
    Err(HybridRetrievalError::InvalidEvidence { reason })
}

fn usize_to_u32(field: &'static str, value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| HybridRetrievalError::ArithmeticOverflow { field })
}

fn usize_from_u32(value: u32) -> Result<usize> {
    usize::try_from(value).map_err(|_| HybridRetrievalError::ArithmeticOverflow {
        field: "max_memory_refs",
    })
}

fn graph_style_label(graph_style: MemoryGraphStyle) -> &'static str {
    match graph_style {
        MemoryGraphStyle::ProvenanceReceiptDag => "provenance_receipt_dag",
        MemoryGraphStyle::CanonicalMemoryGraph => "canonical_memory_graph",
        MemoryGraphStyle::SemanticCatalogGraph => "semantic_catalog_graph",
        MemoryGraphStyle::SimilarityOverlayGraph => "similarity_overlay_graph",
        MemoryGraphStyle::DependencyDag => "dependency_dag",
        MemoryGraphStyle::RoutingViewGraph => "routing_view_graph",
        MemoryGraphStyle::ContradictionSupersessionGraph => "contradiction_supersession_graph",
        MemoryGraphStyle::ContextPacketGraph => "context_packet_graph",
    }
}

fn edge_kind_label(edge_kind: MemoryEdgeKind) -> &'static str {
    match edge_kind {
        MemoryEdgeKind::DerivedFrom => "derived_from",
        MemoryEdgeKind::Summarizes => "summarizes",
        MemoryEdgeKind::Supports => "supports",
        MemoryEdgeKind::Contradicts => "contradicts",
        MemoryEdgeKind::Supersedes => "supersedes",
        MemoryEdgeKind::Replaces => "replaces",
        MemoryEdgeKind::DuplicateOf => "duplicate_of",
        MemoryEdgeKind::NearDuplicateOf => "near_duplicate_of",
        MemoryEdgeKind::RelatedTo => "related_to",
        MemoryEdgeKind::AlternativeSummaryOf => "alternative_summary_of",
        MemoryEdgeKind::DependsOn => "depends_on",
        MemoryEdgeKind::PartOf => "part_of",
        MemoryEdgeKind::OwnedBy => "owned_by",
        MemoryEdgeKind::AccessGrantedBy => "access_granted_by",
        MemoryEdgeKind::VerifiedBy => "verified_by",
        MemoryEdgeKind::UsedByRoute => "used_by_route",
        MemoryEdgeKind::IncludedInContextPacket => "included_in_context_packet",
        MemoryEdgeKind::RevokedBy => "revoked_by",
    }
}

#[cfg(test)]
mod private_helper_coverage {
    use serde_json::json;

    use super::*;

    #[test]
    fn reject_forbidden_json_rejects_forbidden_object_keys() {
        let value = json!({"body": "safe-looking value"});
        assert!(reject_forbidden_json(&value).is_err());
    }

    #[test]
    fn push_warning_skips_duplicate_entries() {
        let mut warnings = vec!["existing".to_owned()];
        push_warning(&mut warnings, "existing");
        assert_eq!(warnings.len(), 1);
    }
}
