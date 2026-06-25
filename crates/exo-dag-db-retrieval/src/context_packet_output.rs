//! Pure Rust graph context packet output from `M01` selection responses.
//!
//! This module renders bounded JSON-compatible packets and Markdown without
//! graph traversal, Postgres reads, gateway routes, or production/runtime claims.

use std::collections::BTreeSet;

use exo_dag_db_api::{
    DagDbContextPacketBoundaries, DagDbContextPacketCitationRef,
    DagDbContextPacketImportTrackingStatus, DagDbContextPacketMetrics, DagDbGraphContextPacket,
    DagDbGraphContextPacketBuildRequest, DagDbSelectedContextRef, DagDbSelectedGraphEdgeRef,
    SafeMetadata,
};
use serde::{Deserialize, Serialize};

use crate::scoring::{DomainError, DomainResult, hash_event_body};

/// Schema version for graph context packets emitted by `M02`.
pub const GRAPH_CONTEXT_PACKET_SCHEMA_VERSION: &str = "dagdb_graph_context_packet_v1";
/// Schema version for additive PRD03 layer-aware packet output.
pub const LAYERED_CONTEXT_PACKET_OUTPUT_SCHEMA_VERSION: &str =
    "dagdb_layered_context_packet_output_v1";

const FORBIDDEN_MATERIAL_FRAGMENTS: &[&str] = &[
    "/Users/",
    "/home/",
    "/private/",
    "~/",
    "DATABASE_URL",
    "PRIVATE KEY",
    "api_key",
    "bearer ",
    ".env",
    "raw_markdown",
    "raw_body",
    "raw_private_payload",
    "source_excerpt",
    "postgres://",
    "postgresql://",
    "file://",
];

const FORBIDDEN_JSON_KEY_SOURCE_PATH: &str = "\"source_path\":";

const BLOCKED_BOUNDARY_STATUS: &str = "blocked";
const BLOCKED_SAVINGS_STATUS: &str = "blocked";
const CITATION_LOCATOR_BLOCKED: &str = "omitted_citation_locator_blocked";
const CITATION_STATUS_METADATA_ONLY: &str = "metadata_only_no_locator";
const BUDGET_STATUS_WITHIN_BUDGET: &str = "within_budget";

/// Layer reference exposed by PRD03 packet output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketSelectedLayer {
    /// Existing `dagdb_graph_layers.layer_id`.
    pub layer_id: String,
    /// Stable layer path visible to agents.
    pub layer_path: String,
    /// Root is depth zero; child layers are positive depth.
    pub layer_depth: u32,
    /// Existing graph style hosted by the selected layer.
    pub graph_style: exo_dag_db_api::MemoryGraphStyle,
    /// Deterministic reason this layer was selected.
    pub selection_reason: String,
    /// Optional rollup summary handle for this layer.
    pub rollup_summary_ref: Option<String>,
}

/// Layer edge reference exposed by PRD03 packet output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketSelectedLayerEdge {
    /// Existing `dagdb_graph_layer_edges.layer_edge_id`.
    pub layer_edge_id: String,
    /// Source layer identifier.
    pub from_layer_id: String,
    /// Target layer identifier.
    pub to_layer_id: String,
    /// Stable edge-kind label.
    pub edge_kind: String,
    /// Deterministic reason this layer edge was selected.
    pub selection_reason: String,
}

/// Rollup summary exposed by PRD03 packet output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketRollupSummary {
    /// Stable summary handle referenced by selected layers.
    pub rollup_summary_ref: String,
    /// Layer summarized by this rollup.
    pub layer_id: String,
    /// Safe summary metadata; raw bodies are not allowed.
    pub summary: SafeMetadata,
    /// Token estimate for the summary.
    pub token_estimate: u32,
}

/// Hygiene counts exposed by PRD04 packet output.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketHygieneReport {
    /// Active layer edges available to retrieval after hygiene filtering.
    pub active_layer_edge_count: u32,
    /// Demoted layer edges excluded from default retrieval.
    pub excluded_demoted_layer_edge_count: u32,
    /// Tombstoned layer edges excluded from default retrieval.
    pub excluded_tombstoned_layer_edge_count: u32,
    /// Child layers classified stale by the caller's hygiene pass.
    pub stale_child_layer_count: u32,
    /// Rollups requiring refresh after child-layer or edge-set changes.
    pub rollup_refresh_required_count: u32,
}

/// Caller-provided additive layer evidence for PRD03 packet output.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketAdditions {
    /// Selected layers from layer-aware traversal.
    pub selected_layers: Vec<LayeredContextPacketSelectedLayer>,
    /// Selected edges between selected layers.
    pub selected_layer_edges: Vec<LayeredContextPacketSelectedLayerEdge>,
    /// Rollup summaries for selected layers.
    pub rollup_summaries: Vec<LayeredContextPacketRollupSummary>,
    /// Hygiene counts from the PRD04 layer-governance pass.
    #[serde(default)]
    pub hygiene_report: LayeredContextPacketHygieneReport,
    /// True when the caller explicitly used flat fallback.
    pub flat_fallback_used: bool,
}

/// Budget report exposed by PRD03 packet output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketBudgetReport {
    /// Token budget supplied by the caller.
    pub token_budget: u32,
    /// Selected token estimate from graph context selection.
    pub selected_token_estimate: u32,
    /// Remaining token budget after selected refs.
    pub remaining_token_budget: u32,
    /// Number of selected refs.
    pub selected_ref_count: u32,
    /// Number of selected graph edges.
    pub selected_graph_edge_count: u32,
    /// Number of selected layers.
    pub selected_layer_count: u32,
    /// Number of selected layer edges.
    pub selected_layer_edge_count: u32,
    /// Active layer edges available after hygiene filtering.
    pub active_layer_edge_count: u32,
    /// Demoted layer edges excluded from default retrieval.
    pub excluded_demoted_layer_edge_count: u32,
    /// Tombstoned layer edges excluded from default retrieval.
    pub excluded_tombstoned_layer_edge_count: u32,
    /// Child layers classified stale by the caller's hygiene pass.
    pub stale_child_layer_count: u32,
    /// Rollups requiring refresh after child-layer or edge-set changes.
    pub rollup_refresh_required_count: u32,
    /// Number of rollup summaries.
    pub rollup_summary_count: u32,
    /// Stable budget status.
    pub budget_status: String,
}

/// Additive PRD03 packet output that preserves existing flat packet compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredContextPacketOutput {
    /// Layered packet schema version.
    pub schema_version: String,
    /// Existing base packet schema version.
    pub base_schema_version: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Request identifier.
    pub request_id: String,
    /// Task text.
    pub task: String,
    /// Task hash.
    pub task_hash: String,
    /// Hash over the additive layered packet material.
    pub packet_hash: String,
    /// Hash emitted by the base graph context packet.
    pub base_packet_hash: String,
    /// Selected layers.
    pub selected_layers: Vec<LayeredContextPacketSelectedLayer>,
    /// Selected layer edges.
    pub selected_layer_edges: Vec<LayeredContextPacketSelectedLayerEdge>,
    /// Existing selected graph edges, filtered to selected refs.
    pub selected_graph_edges: Vec<DagDbSelectedGraphEdgeRef>,
    /// Existing selected memory refs under the PRD03 field name.
    pub selected_refs: Vec<DagDbSelectedContextRef>,
    /// Rollup summaries for selected layers.
    pub rollup_summaries: Vec<LayeredContextPacketRollupSummary>,
    /// Layer hygiene counts explaining excluded layer edges and stale children.
    pub hygiene_report: LayeredContextPacketHygieneReport,
    /// Layer-aware budget report.
    pub budget_report: LayeredContextPacketBudgetReport,
    /// Existing citation refs.
    pub citation_refs: Vec<DagDbContextPacketCitationRef>,
    /// Existing blocked claim boundaries.
    pub boundaries: DagDbContextPacketBoundaries,
    /// Existing agent usage instructions.
    pub agent_usage_instructions: Vec<String>,
    /// True when output fell back to flat graph context.
    pub flat_fallback_used: bool,
    /// Rendered markdown with additive layer sections.
    pub markdown: String,
}

/// Build a bounded graph context packet from an `M01` selection response.
pub fn build_graph_context_packet(
    request: &DagDbGraphContextPacketBuildRequest,
) -> DomainResult<DagDbGraphContextPacket> {
    validate_request(request)?;

    let selected_memory_refs = request.selection.selected_memory_refs.clone();
    let selected_ids = selected_memory_refs
        .iter()
        .map(|selected| selected.memory_id.clone())
        .collect::<BTreeSet<_>>();
    let selected_graph_edges =
        filter_selected_graph_edges(&request.selection.selected_graph_edges, &selected_ids);
    let citation_refs = build_citation_refs(&selected_memory_refs);
    let packet_metrics = build_metrics(
        request.token_budget,
        request.selection.selected_token_estimate,
        &selected_memory_refs,
        &selected_graph_edges,
        &citation_refs,
    );
    let boundaries = blocked_boundaries();
    let agent_usage_instructions =
        build_agent_usage_instructions(request.import_tracking_status.is_some());
    let markdown = render_markdown(RenderMarkdownContext {
        request,
        selected_memory_refs: &selected_memory_refs,
        selected_graph_edges: &selected_graph_edges,
        citation_refs: &citation_refs,
        metrics: &packet_metrics,
        boundaries: &boundaries,
        agent_usage_instructions: &agent_usage_instructions,
        import_tracking_status: request.import_tracking_status.as_ref(),
    })?;

    let packet_hash = compute_packet_hash(
        request,
        &selected_memory_refs,
        &selected_graph_edges,
        &citation_refs,
        &packet_metrics,
        &boundaries,
        &agent_usage_instructions,
    )?;

    let packet = DagDbGraphContextPacket {
        schema_version: GRAPH_CONTEXT_PACKET_SCHEMA_VERSION.into(),
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task: request.task.clone(),
        task_hash: request.task_hash.clone(),
        packet_hash: packet_hash.to_string(),
        selected_memory_refs,
        selected_graph_edges,
        citation_refs,
        packet_metrics,
        boundaries,
        agent_usage_instructions,
        markdown,
    };

    validate_packet_no_forbidden_material(&packet)?;
    Ok(packet)
}

/// Build additive PRD03 layer-aware packet output without changing retrieval internals.
pub fn build_layered_context_packet_output(
    request: &DagDbGraphContextPacketBuildRequest,
    additions: LayeredContextPacketAdditions,
) -> DomainResult<LayeredContextPacketOutput> {
    validate_layered_additions(&additions)?;

    let base_packet = build_graph_context_packet(request)?;
    let selected_layers = sorted_selected_layers(additions.selected_layers);
    let selected_layer_ids = selected_layers
        .iter()
        .map(|layer| layer.layer_id.clone())
        .collect::<BTreeSet<_>>();
    validate_selected_layer_ids(&selected_layers, &selected_layer_ids)?;
    let selected_layer_edges =
        sort_selected_layer_edges(additions.selected_layer_edges, &selected_layer_ids)?;
    let rollup_summaries = sort_rollup_summaries(additions.rollup_summaries, &selected_layer_ids)?;
    validate_layer_rollup_refs(&selected_layers, &rollup_summaries)?;
    let hygiene_report = additions.hygiene_report;
    let flat_fallback_used = additions.flat_fallback_used || selected_layers.is_empty();
    let budget_report = build_layered_budget_report(
        &base_packet,
        selected_layers.len(),
        selected_layer_edges.len(),
        rollup_summaries.len(),
        &hygiene_report,
    );
    let markdown = render_layered_markdown(
        &base_packet.markdown,
        &selected_layers,
        &selected_layer_edges,
        &rollup_summaries,
        &hygiene_report,
        &budget_report,
        flat_fallback_used,
    );

    let packet_hash = compute_layered_packet_hash(
        &base_packet,
        &selected_layers,
        &selected_layer_edges,
        &rollup_summaries,
        &hygiene_report,
        &budget_report,
        flat_fallback_used,
    )?;

    let output = LayeredContextPacketOutput {
        schema_version: LAYERED_CONTEXT_PACKET_OUTPUT_SCHEMA_VERSION.to_owned(),
        base_schema_version: base_packet.schema_version.clone(),
        tenant_id: base_packet.tenant_id.clone(),
        namespace: base_packet.namespace.clone(),
        request_id: base_packet.request_id.clone(),
        task: base_packet.task.clone(),
        task_hash: base_packet.task_hash.clone(),
        packet_hash: packet_hash.to_string(),
        base_packet_hash: base_packet.packet_hash.clone(),
        selected_layers,
        selected_layer_edges,
        selected_graph_edges: base_packet.selected_graph_edges.clone(),
        selected_refs: base_packet.selected_memory_refs.clone(),
        rollup_summaries,
        hygiene_report,
        budget_report,
        citation_refs: base_packet.citation_refs.clone(),
        boundaries: base_packet.boundaries.clone(),
        agent_usage_instructions: base_packet.agent_usage_instructions.clone(),
        flat_fallback_used,
        markdown,
    };
    validate_layered_packet_no_forbidden_material(&output)?;
    Ok(output)
}

fn validate_layered_additions(additions: &LayeredContextPacketAdditions) -> DomainResult<()> {
    for layer in &additions.selected_layers {
        validate_no_forbidden_material(&layer.layer_id)?;
        validate_no_forbidden_material(&layer.layer_path)?;
        validate_layer_path(&layer.layer_path)?;
        validate_no_forbidden_material(&layer.selection_reason)?;
        if let Some(rollup_summary_ref) = &layer.rollup_summary_ref {
            validate_no_forbidden_material(rollup_summary_ref)?;
        }
    }
    for edge in &additions.selected_layer_edges {
        validate_no_forbidden_material(&edge.layer_edge_id)?;
        validate_no_forbidden_material(&edge.from_layer_id)?;
        validate_no_forbidden_material(&edge.to_layer_id)?;
        validate_no_forbidden_material(&edge.edge_kind)?;
        validate_no_forbidden_material(&edge.selection_reason)?;
    }
    for rollup in &additions.rollup_summaries {
        validate_no_forbidden_material(&rollup.rollup_summary_ref)?;
        validate_no_forbidden_material(&rollup.layer_id)?;
        validate_safe_metadata(&rollup.summary)?;
    }
    Ok(())
}

fn validate_layer_path(path: &str) -> DomainResult<()> {
    if path.trim().is_empty() || path.starts_with('/') || path.ends_with('/') || path.contains("//")
    {
        return Err(DomainError::ValidationFailed);
    }
    for segment in path.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.starts_with('~')
            || segment.contains('\\')
        {
            return Err(DomainError::ValidationFailed);
        }
    }
    Ok(())
}

fn sorted_selected_layers(
    mut selected_layers: Vec<LayeredContextPacketSelectedLayer>,
) -> Vec<LayeredContextPacketSelectedLayer> {
    selected_layers.sort_by(|left, right| {
        left.layer_depth
            .cmp(&right.layer_depth)
            .then(left.layer_path.cmp(&right.layer_path))
            .then(left.layer_id.cmp(&right.layer_id))
    });
    selected_layers
}

fn validate_selected_layer_ids(
    selected_layers: &[LayeredContextPacketSelectedLayer],
    selected_layer_ids: &BTreeSet<String>,
) -> DomainResult<()> {
    if selected_layers.len() != selected_layer_ids.len() {
        return Err(DomainError::ValidationFailed);
    }
    Ok(())
}

fn sort_selected_layer_edges(
    selected_layer_edges: Vec<LayeredContextPacketSelectedLayerEdge>,
    selected_layer_ids: &BTreeSet<String>,
) -> DomainResult<Vec<LayeredContextPacketSelectedLayerEdge>> {
    let mut sorted = Vec::with_capacity(selected_layer_edges.len());
    for edge in selected_layer_edges {
        if !selected_layer_ids.contains(&edge.from_layer_id)
            || !selected_layer_ids.contains(&edge.to_layer_id)
        {
            return Err(DomainError::ValidationFailed);
        }
        sorted.push(edge);
    }
    sorted.sort_by(|left, right| {
        left.layer_edge_id
            .cmp(&right.layer_edge_id)
            .then(left.from_layer_id.cmp(&right.from_layer_id))
            .then(left.to_layer_id.cmp(&right.to_layer_id))
    });
    Ok(sorted)
}

fn sort_rollup_summaries(
    rollup_summaries: Vec<LayeredContextPacketRollupSummary>,
    selected_layer_ids: &BTreeSet<String>,
) -> DomainResult<Vec<LayeredContextPacketRollupSummary>> {
    let mut sorted = Vec::with_capacity(rollup_summaries.len());
    for rollup in rollup_summaries {
        if !selected_layer_ids.contains(&rollup.layer_id) {
            return Err(DomainError::ValidationFailed);
        }
        sorted.push(rollup);
    }
    sorted.sort_by(|left, right| {
        left.rollup_summary_ref
            .cmp(&right.rollup_summary_ref)
            .then(left.layer_id.cmp(&right.layer_id))
    });
    Ok(sorted)
}

fn validate_layer_rollup_refs(
    selected_layers: &[LayeredContextPacketSelectedLayer],
    rollup_summaries: &[LayeredContextPacketRollupSummary],
) -> DomainResult<()> {
    let rollup_refs = rollup_summaries
        .iter()
        .map(|rollup| (rollup.layer_id.clone(), rollup.rollup_summary_ref.clone()))
        .collect::<BTreeSet<_>>();
    for layer in selected_layers {
        if let Some(rollup_summary_ref) = &layer.rollup_summary_ref {
            let expected = (layer.layer_id.clone(), rollup_summary_ref.clone());
            if !rollup_refs.contains(&expected) {
                return Err(DomainError::ValidationFailed);
            }
        }
    }
    Ok(())
}

fn build_layered_budget_report(
    base_packet: &DagDbGraphContextPacket,
    selected_layer_count: usize,
    selected_layer_edge_count: usize,
    rollup_summary_count: usize,
    hygiene_report: &LayeredContextPacketHygieneReport,
) -> LayeredContextPacketBudgetReport {
    LayeredContextPacketBudgetReport {
        token_budget: base_packet.packet_metrics.token_budget,
        selected_token_estimate: base_packet.packet_metrics.selected_token_estimate,
        remaining_token_budget: base_packet // pragma-allowlist-secret
            .packet_metrics
            .token_budget
            .saturating_sub(base_packet.packet_metrics.selected_token_estimate),
        selected_ref_count: base_packet.packet_metrics.selected_memory_ref_count,
        selected_graph_edge_count: base_packet.packet_metrics.selected_graph_edge_count,
        selected_layer_count: u32_from_usize_lossy(selected_layer_count),
        selected_layer_edge_count: u32_from_usize_lossy(selected_layer_edge_count),
        active_layer_edge_count: hygiene_report.active_layer_edge_count,
        excluded_demoted_layer_edge_count: hygiene_report.excluded_demoted_layer_edge_count,
        excluded_tombstoned_layer_edge_count: hygiene_report.excluded_tombstoned_layer_edge_count,
        stale_child_layer_count: hygiene_report.stale_child_layer_count,
        rollup_refresh_required_count: hygiene_report.rollup_refresh_required_count,
        rollup_summary_count: u32_from_usize_lossy(rollup_summary_count),
        budget_status: BUDGET_STATUS_WITHIN_BUDGET.to_owned(),
    }
}

fn render_layered_markdown(
    base_markdown: &str,
    selected_layers: &[LayeredContextPacketSelectedLayer],
    selected_layer_edges: &[LayeredContextPacketSelectedLayerEdge],
    rollup_summaries: &[LayeredContextPacketRollupSummary],
    hygiene_report: &LayeredContextPacketHygieneReport,
    budget_report: &LayeredContextPacketBudgetReport,
    flat_fallback_used: bool,
) -> String {
    let mut lines = vec![base_markdown.to_owned(), String::new()];
    lines.push("## Selected Layers".into());
    if selected_layers.is_empty() {
        lines.push("- none".into());
    } else {
        for layer in selected_layers {
            lines.push(format!(
                "- {} | path={} | depth={} | {:?} | reason={}",
                layer.layer_id,
                layer.layer_path,
                layer.layer_depth,
                layer.graph_style,
                layer.selection_reason
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Selected Layer Edges".into());
    if selected_layer_edges.is_empty() {
        lines.push("- none".into());
    } else {
        for edge in selected_layer_edges {
            lines.push(format!(
                "- {} | {} -> {} | {} | reason={}",
                edge.layer_edge_id,
                edge.from_layer_id,
                edge.to_layer_id,
                edge.edge_kind,
                edge.selection_reason
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Rollup Summaries".into());
    if rollup_summaries.is_empty() {
        lines.push("- none".into());
    } else {
        for rollup in rollup_summaries {
            lines.push(format!(
                "- {} | layer={} | tokens={} | {}",
                rollup.rollup_summary_ref,
                rollup.layer_id,
                rollup.token_estimate,
                rollup.summary.text
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Layer Budget Report".into());
    lines.push(format!("- token_budget: {}", budget_report.token_budget));
    lines.push(format!(
        "- selected_token_estimate: {}",
        budget_report.selected_token_estimate
    ));
    lines.push(format!(
        "- remaining_token_budget: {}",
        budget_report.remaining_token_budget
    ));
    lines.push(format!(
        "- selected_ref_count: {}",
        budget_report.selected_ref_count
    ));
    lines.push(format!(
        "- selected_graph_edge_count: {}",
        budget_report.selected_graph_edge_count
    ));
    lines.push(format!(
        "- selected_layer_count: {}",
        budget_report.selected_layer_count
    ));
    lines.push(format!(
        "- selected_layer_edge_count: {}",
        budget_report.selected_layer_edge_count
    ));
    lines.push(format!(
        "- active_layer_edge_count: {}",
        hygiene_report.active_layer_edge_count
    ));
    lines.push(format!(
        "- excluded_demoted_layer_edge_count: {}",
        hygiene_report.excluded_demoted_layer_edge_count
    ));
    lines.push(format!(
        "- excluded_tombstoned_layer_edge_count: {}",
        hygiene_report.excluded_tombstoned_layer_edge_count
    ));
    lines.push(format!(
        "- stale_child_layer_count: {}",
        hygiene_report.stale_child_layer_count
    ));
    lines.push(format!(
        "- rollup_refresh_required_count: {}",
        hygiene_report.rollup_refresh_required_count
    ));
    lines.push(format!(
        "- rollup_summary_count: {}",
        budget_report.rollup_summary_count
    ));
    lines.push(format!("- budget_status: {}", budget_report.budget_status));
    lines.push(format!("- flat_fallback_used: {flat_fallback_used}"));
    lines.join("\n")
}

#[derive(Serialize)]
struct LayeredContextPacketHashMaterial<'a> {
    schema_version: &'a str,
    base_packet_hash: &'a str,
    selected_layers: &'a [LayeredContextPacketSelectedLayer],
    selected_layer_edges: &'a [LayeredContextPacketSelectedLayerEdge],
    selected_graph_edges: &'a [DagDbSelectedGraphEdgeRef],
    selected_refs: &'a [DagDbSelectedContextRef],
    rollup_summaries: &'a [LayeredContextPacketRollupSummary],
    hygiene_report: &'a LayeredContextPacketHygieneReport,
    budget_report: &'a LayeredContextPacketBudgetReport,
    flat_fallback_used: bool,
}

fn compute_layered_packet_hash(
    base_packet: &DagDbGraphContextPacket,
    selected_layers: &[LayeredContextPacketSelectedLayer],
    selected_layer_edges: &[LayeredContextPacketSelectedLayerEdge],
    rollup_summaries: &[LayeredContextPacketRollupSummary],
    hygiene_report: &LayeredContextPacketHygieneReport,
    budget_report: &LayeredContextPacketBudgetReport,
    flat_fallback_used: bool,
) -> DomainResult<exo_core::Hash256> {
    hash_event_body(&LayeredContextPacketHashMaterial {
        schema_version: LAYERED_CONTEXT_PACKET_OUTPUT_SCHEMA_VERSION,
        base_packet_hash: &base_packet.packet_hash,
        selected_layers,
        selected_layer_edges,
        selected_graph_edges: &base_packet.selected_graph_edges,
        selected_refs: &base_packet.selected_memory_refs,
        rollup_summaries,
        hygiene_report,
        budget_report,
        flat_fallback_used,
    })
}

fn validate_layered_packet_no_forbidden_material(
    output: &LayeredContextPacketOutput,
) -> DomainResult<()> {
    let serialized = serde_json::to_string(output).map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })?;
    for fragment in FORBIDDEN_MATERIAL_FRAGMENTS {
        if serialized.contains(fragment) {
            return Err(DomainError::ValidationFailed);
        }
    }
    if serialized.contains(FORBIDDEN_JSON_KEY_SOURCE_PATH) {
        return Err(DomainError::ValidationFailed);
    }
    validate_no_forbidden_material(&output.markdown)?;
    Ok(())
}

fn validate_request(request: &DagDbGraphContextPacketBuildRequest) -> DomainResult<()> {
    if request.task.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    if request.token_budget == 0 {
        return Err(DomainError::ValidationFailed);
    }
    if request.selection.selected_token_estimate > request.token_budget {
        return Err(DomainError::ValidationFailed);
    }
    if request.selection.token_budget != request.token_budget {
        return Err(DomainError::ValidationFailed);
    }

    if request.tenant_id != request.selection.tenant_id
        || request.namespace != request.selection.namespace
        || request.request_id != request.selection.request_id
        || request.task_hash != request.selection.task_hash
    {
        return Err(DomainError::ValidationFailed);
    }

    validate_no_forbidden_material(&request.task)?;
    validate_no_forbidden_material(&request.audit_id)?;
    validate_no_forbidden_material(&request.tenant_id)?;
    validate_no_forbidden_material(&request.namespace)?;
    validate_no_forbidden_material(&request.request_id)?;
    validate_no_forbidden_material(&request.task_hash)?;

    let mut token_sum = 0u32;
    for selected in &request.selection.selected_memory_refs {
        validate_selected_ref(selected)?;
        token_sum = token_sum.checked_add(selected.token_estimate).ok_or(
            DomainError::ArithmeticOverflow {
                operation: "context_packet_selected_token_sum",
            },
        )?;
        if selected.token_estimate > request.token_budget {
            return Err(DomainError::ValidationFailed);
        }
    }
    if token_sum != request.selection.selected_token_estimate {
        return Err(DomainError::ValidationFailed);
    }

    let max_memory_refs = request.max_memory_refs.unwrap_or(request.token_budget);
    let max_ref_count =
        usize::try_from(max_memory_refs.min(request.token_budget)).map_err(|_| {
            DomainError::ArithmeticOverflow {
                operation: "context_packet_max_memory_refs_usize",
            }
        })?;
    if request.selection.selected_memory_refs.len() > max_ref_count {
        return Err(DomainError::ValidationFailed);
    }

    for edge in &request.selection.selected_graph_edges {
        validate_no_forbidden_material(&edge.graph_edge_id)?;
        validate_no_forbidden_material(&edge.from_memory_id)?;
        validate_no_forbidden_material(&edge.to_memory_id)?;
        validate_no_forbidden_material(&edge.selection_reason)?;
    }

    if let Some(import_status) = &request.import_tracking_status {
        validate_import_tracking_status(import_status)?;
    }

    Ok(())
}

fn validate_selected_ref(selected: &DagDbSelectedContextRef) -> DomainResult<()> {
    validate_no_forbidden_material(&selected.memory_id)?;
    validate_safe_metadata(&selected.title)?;
    validate_safe_metadata(&selected.summary)?;
    for segment in &selected.catalog_path {
        validate_no_forbidden_material(segment)?;
    }
    validate_no_forbidden_material(&selected.document_type)?;
    validate_no_forbidden_material(&selected.selection_reason)?;
    validate_no_forbidden_material(&selected.citation_ref)?;
    if let Some(catalog_id) = &selected.catalog_id {
        validate_no_forbidden_material(catalog_id)?;
    }
    for flag in &selected.boundary_flags {
        validate_no_forbidden_material(flag)?;
    }
    Ok(())
}

fn validate_safe_metadata(metadata: &SafeMetadata) -> DomainResult<()> {
    validate_no_forbidden_material(&metadata.text)?;
    validate_no_forbidden_material(&metadata.original_hash)?;
    Ok(())
}

fn validate_import_tracking_status(
    import_status: &DagDbContextPacketImportTrackingStatus,
) -> DomainResult<()> {
    validate_no_forbidden_material(&import_status.manifest_json)?;
    validate_no_forbidden_material(&import_status.manifest_status)?;
    if import_status.source_path_status != CITATION_LOCATOR_BLOCKED {
        return Err(DomainError::ValidationFailed);
    }
    Ok(())
}

fn validate_no_forbidden_material(text: &str) -> DomainResult<()> {
    let lowered_text = text.to_ascii_lowercase();
    for fragment in FORBIDDEN_MATERIAL_FRAGMENTS {
        if lowered_text.contains(&fragment.to_ascii_lowercase()) {
            return Err(DomainError::ValidationFailed);
        }
    }
    Ok(())
}

fn validate_packet_no_forbidden_material(packet: &DagDbGraphContextPacket) -> DomainResult<()> {
    let serialized = serde_json::to_string(packet).map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })?;
    for fragment in FORBIDDEN_MATERIAL_FRAGMENTS {
        if serialized.contains(fragment) {
            return Err(DomainError::ValidationFailed);
        }
    }
    if serialized.contains(FORBIDDEN_JSON_KEY_SOURCE_PATH) {
        return Err(DomainError::ValidationFailed);
    }
    validate_no_forbidden_material(&packet.markdown)?;
    Ok(())
}

fn filter_selected_graph_edges(
    edges: &[DagDbSelectedGraphEdgeRef],
    selected_ids: &BTreeSet<String>,
) -> Vec<DagDbSelectedGraphEdgeRef> {
    let mut filtered = edges
        .iter()
        .filter(|edge| {
            selected_ids.contains(&edge.from_memory_id) && selected_ids.contains(&edge.to_memory_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.graph_edge_id
            .cmp(&right.graph_edge_id)
            .then(left.from_memory_id.cmp(&right.from_memory_id))
            .then(left.to_memory_id.cmp(&right.to_memory_id))
    });
    filtered
}

fn build_citation_refs(
    selected_memory_refs: &[DagDbSelectedContextRef],
) -> Vec<DagDbContextPacketCitationRef> {
    selected_memory_refs
        .iter()
        .map(|selected| DagDbContextPacketCitationRef {
            citation_ref: selected.citation_ref.clone(),
            memory_id: selected.memory_id.clone(),
            citation_status: CITATION_STATUS_METADATA_ONLY.into(),
        })
        .collect()
}

fn build_metrics(
    token_budget: u32,
    selected_token_estimate: u32,
    selected_memory_refs: &[DagDbSelectedContextRef],
    selected_graph_edges: &[DagDbSelectedGraphEdgeRef],
    citation_refs: &[DagDbContextPacketCitationRef],
) -> DagDbContextPacketMetrics {
    DagDbContextPacketMetrics {
        token_budget,
        selected_token_estimate,
        selected_memory_ref_count: u32_from_usize_lossy(selected_memory_refs.len()),
        selected_graph_edge_count: u32_from_usize_lossy(selected_graph_edges.len()),
        citation_ref_count: u32_from_usize_lossy(citation_refs.len()),
        end_to_end_savings_status: BLOCKED_SAVINGS_STATUS.into(),
        cost_savings_status: BLOCKED_SAVINGS_STATUS.into(),
    }
}

fn blocked_boundaries() -> DagDbContextPacketBoundaries {
    DagDbContextPacketBoundaries {
        repository_test_level_only: true,
        production_runtime: BLOCKED_BOUNDARY_STATUS.into(),
        default_context_replacement: BLOCKED_BOUNDARY_STATUS.into(),
        citation_locator_status: CITATION_LOCATOR_BLOCKED.into(),
        billing_savings: BLOCKED_BOUNDARY_STATUS.into(),
    }
}

fn build_agent_usage_instructions(import_tracking_present: bool) -> Vec<String> {
    let mut instructions = vec![
        "Use this packet only at repository/test level; production runtime remains blocked.".into(),
        "Do not treat this packet as default-context replacement; default-use remains blocked."
            .into(),
        "Do not claim end-to-end or billing cost savings from this packet.".into(),
        "Citation locators remain blocked; use citation_ref metadata handles only.".into(),
    ];
    if import_tracking_present {
        instructions.push(
            "Apply import-tracking manifest constraints; source paths remain omitted (omitted_citation_locator_blocked)."
                .into(),
        );
    }
    instructions
}

struct RenderMarkdownContext<'a> {
    request: &'a DagDbGraphContextPacketBuildRequest,
    selected_memory_refs: &'a [DagDbSelectedContextRef],
    selected_graph_edges: &'a [DagDbSelectedGraphEdgeRef],
    citation_refs: &'a [DagDbContextPacketCitationRef],
    metrics: &'a DagDbContextPacketMetrics,
    boundaries: &'a DagDbContextPacketBoundaries,
    agent_usage_instructions: &'a [String],
    import_tracking_status: Option<&'a DagDbContextPacketImportTrackingStatus>,
}

fn render_markdown(context: RenderMarkdownContext<'_>) -> DomainResult<String> {
    let RenderMarkdownContext {
        request,
        selected_memory_refs,
        selected_graph_edges,
        citation_refs,
        metrics,
        boundaries,
        agent_usage_instructions,
        import_tracking_status,
    } = context;
    let mut lines = vec![
        "# DAG DB Graph Context Packet".into(),
        String::new(),
        "## Task".into(),
        request.task.clone(),
        String::new(),
        "## Selected Memory Refs".into(),
    ];
    if selected_memory_refs.is_empty() {
        lines.push("- none".into());
    } else {
        for selected in selected_memory_refs {
            lines.push(format!(
                "- {} | {} | tokens={} | reason={}",
                selected.memory_id,
                selected.title.text,
                selected.token_estimate,
                selected.selection_reason
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Selected Graph Edges".into());
    if selected_graph_edges.is_empty() {
        lines.push("- none".into());
    } else {
        for edge in selected_graph_edges {
            lines.push(format!(
                "- {} | {} -> {} | {:?} | {:?}",
                edge.graph_edge_id,
                edge.from_memory_id,
                edge.to_memory_id,
                edge.edge_kind,
                edge.graph_style
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Citation Refs".into());
    if citation_refs.is_empty() {
        lines.push("- none".into());
    } else {
        for citation in citation_refs {
            lines.push(format!(
                "- {} | {} | {}",
                citation.citation_ref, citation.memory_id, citation.citation_status
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Metrics".into());
    lines.push(format!("- token_budget: {}", metrics.token_budget));
    lines.push(format!(
        "- selected_token_estimate: {}",
        metrics.selected_token_estimate
    ));
    lines.push(format!(
        "- selected_memory_ref_count: {}",
        metrics.selected_memory_ref_count
    ));
    lines.push(format!(
        "- selected_graph_edge_count: {}",
        metrics.selected_graph_edge_count
    ));
    lines.push(format!(
        "- citation_ref_count: {}",
        metrics.citation_ref_count
    ));
    lines.push(format!(
        "- end_to_end_savings_status: {}",
        metrics.end_to_end_savings_status
    ));
    lines.push(format!(
        "- cost_savings_status: {}",
        metrics.cost_savings_status
    ));
    lines.push(String::new());
    lines.push("## Boundaries".into());
    lines.push(format!(
        "- repository_test_level_only: {}",
        boundaries.repository_test_level_only
    ));
    lines.push(format!(
        "- production_runtime: {}",
        boundaries.production_runtime
    ));
    lines.push(format!(
        "- default_context_replacement: {}",
        boundaries.default_context_replacement
    ));
    lines.push(format!(
        "- citation_locator_status: {}",
        boundaries.citation_locator_status
    ));
    lines.push(format!("- billing_savings: {}", boundaries.billing_savings));
    if let Some(import_status) = import_tracking_status {
        lines.push(format!(
            "- import_manifest_status: {}",
            import_status.manifest_status
        ));
        lines.push(format!(
            "- import_source_path_status: {}",
            import_status.source_path_status
        ));
    }
    lines.push(String::new());
    lines.push("## Agent Usage Instructions".into());
    for instruction in agent_usage_instructions {
        lines.push(format!("- {instruction}"));
    }
    Ok(lines.join("\n"))
}

#[derive(Serialize)]
struct GraphContextPacketHashMaterial<'a> {
    schema_version: &'a str,
    tenant_id: &'a str,
    namespace: &'a str,
    request_id: &'a str,
    task: &'a str,
    task_hash: &'a str,
    audit_id: &'a str,
    selected_memory_refs: &'a [DagDbSelectedContextRef],
    selected_graph_edges: &'a [DagDbSelectedGraphEdgeRef],
    citation_refs: &'a [DagDbContextPacketCitationRef],
    packet_metrics: &'a DagDbContextPacketMetrics,
    boundaries: &'a DagDbContextPacketBoundaries,
    agent_usage_instructions: &'a [String],
    import_tracking_status: &'a Option<DagDbContextPacketImportTrackingStatus>,
}

fn compute_packet_hash(
    request: &DagDbGraphContextPacketBuildRequest,
    selected_memory_refs: &[DagDbSelectedContextRef],
    selected_graph_edges: &[DagDbSelectedGraphEdgeRef],
    citation_refs: &[DagDbContextPacketCitationRef],
    packet_metrics: &DagDbContextPacketMetrics,
    boundaries: &DagDbContextPacketBoundaries,
    agent_usage_instructions: &[String],
) -> DomainResult<exo_core::Hash256> {
    hash_event_body(&GraphContextPacketHashMaterial {
        schema_version: GRAPH_CONTEXT_PACKET_SCHEMA_VERSION,
        tenant_id: &request.tenant_id,
        namespace: &request.namespace,
        request_id: &request.request_id,
        task: &request.task,
        task_hash: &request.task_hash,
        audit_id: &request.audit_id,
        selected_memory_refs,
        selected_graph_edges,
        citation_refs,
        packet_metrics,
        boundaries,
        agent_usage_instructions,
        import_tracking_status: &request.import_tracking_status,
    })
}

fn u32_from_usize_lossy(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use exo_core::Hash256;
    use exo_dag_db_api::{
        DagDbContextPacketImportTrackingStatus, DagDbGraphContextPacketBuildRequest,
        DagDbGraphContextSelectionRequest, DagDbGraphContextSelectionResponse, MemoryEdgeKind,
        MemoryGraphStyle, SafeMetadata, SafeMetadataDecision, ValidationStatus,
    };

    use super::*;
    use crate::{GraphContextMemoryCandidate, GraphContextSelectionState, select_graph_context};

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn safe(text: &str) -> SafeMetadata {
        SafeMetadata {
            decision: SafeMetadataDecision::Allow,
            text: text.into(),
            redaction_codes: Vec::new(),
            original_hash: h(0xee).to_string(),
            truncated: false,
            byte_len: u32::try_from(text.len()).expect("fixture fits"),
        }
    }

    fn candidate(
        memory_id: &str,
        title: &str,
        summary: &str,
        tokens: u32,
    ) -> GraphContextMemoryCandidate {
        GraphContextMemoryCandidate {
            memory_id: memory_id.into(),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            catalog_id: Some(format!("catalog-{memory_id}")),
            title: safe(title),
            summary: safe(summary),
            catalog_path: vec!["04_Plans".into()],
            document_type: "plan".into(),
            token_estimate: tokens,
            validation_status: ValidationStatus::Passed,
            citation_ref: format!("citation:{memory_id}"),
            boundary_flags: vec!["repository_test_only".into()],
        }
    }

    fn selection_response() -> DagDbGraphContextSelectionResponse {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate("mem-a", "Plan A", "Summary A", 120),
                candidate("mem-b", "Plan B", "Summary B", 140),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let request = DagDbGraphContextSelectionRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task: "Select next implementation step".into(),
            task_hash: h(0x11).to_string(),
            token_budget: 1_000,
            max_memory_refs: 4,
            catalog_hints: Vec::new(),
            requested_memory_ids: Vec::new(),
            force_revalidate: false,
        };
        select_graph_context(&request, &state).expect("selection")
    }

    fn build_request(
        selection: DagDbGraphContextSelectionResponse,
        import_tracking_status: Option<DagDbContextPacketImportTrackingStatus>,
    ) -> DagDbGraphContextPacketBuildRequest {
        DagDbGraphContextPacketBuildRequest {
            tenant_id: selection.tenant_id.clone(),
            namespace: selection.namespace.clone(),
            request_id: selection.request_id.clone(),
            task: "Build bounded context packet for M02".into(),
            task_hash: selection.task_hash.clone(),
            audit_id: "audit-m02".into(),
            token_budget: selection.token_budget,
            max_memory_refs: None,
            selection,
            import_tracking_status,
        }
    }

    fn assert_validation_failed(request: &DagDbGraphContextPacketBuildRequest) {
        assert_eq!(
            build_graph_context_packet(request),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_builds_deterministic_packet() {
        let selection = selection_response();
        let request = build_request(selection, None);
        let first = build_graph_context_packet(&request).expect("first packet");
        let second = build_graph_context_packet(&request).expect("second packet");
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).expect("json"),
            serde_json::to_string(&second).expect("json")
        );
        assert_eq!(first.schema_version, GRAPH_CONTEXT_PACKET_SCHEMA_VERSION);
        assert!(!first.packet_hash.is_empty());
    }

    #[test]
    fn context_packet_output_rejects_forbidden_task_material() {
        let selection = selection_response();
        let mut request = build_request(selection, None);
        request.task = "Leaked /Users/max/project path".into();
        assert_eq!(
            build_graph_context_packet(&request),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_import_tracking_instruction_only_when_present() {
        let selection = selection_response();
        let without = build_graph_context_packet(&build_request(selection.clone(), None))
            .expect("without import");
        assert!(
            !without
                .agent_usage_instructions
                .iter()
                .any(|line| line.contains("import-tracking"))
        );

        let import_status = DagDbContextPacketImportTrackingStatus {
            manifest_json: "{\"tracked\":true}".into(),
            manifest_status: "clean_manifest".into(),
            tracked_clean_evidence_enforced: true,
            source_path_status: CITATION_LOCATOR_BLOCKED.into(),
        };
        let with = build_graph_context_packet(&build_request(selection, Some(import_status)))
            .expect("with import");
        assert!(
            with.agent_usage_instructions
                .iter()
                .any(|line| line.contains("import-tracking"))
        );
    }

    #[test]
    fn context_packet_output_rejects_selection_scope_mismatch() {
        let selection = selection_response();
        let mut request = build_request(selection, None);
        request.tenant_id = "tenant-b".into();
        assert_eq!(
            build_graph_context_packet(&request),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_empty_task_and_zero_budget() {
        let selection = selection_response();
        let mut empty_task = build_request(selection.clone(), None);
        empty_task.task.clear();
        assert_eq!(
            build_graph_context_packet(&empty_task),
            Err(DomainError::ValidationFailed)
        );

        let mut zero_budget = build_request(selection, None);
        zero_budget.token_budget = 0;
        assert_eq!(
            build_graph_context_packet(&zero_budget),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_token_sum_mismatch() {
        let mut selection = selection_response();
        selection.selected_token_estimate = selection.selected_token_estimate.saturating_add(1);
        assert_eq!(
            build_graph_context_packet(&build_request(selection, None)),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_bad_import_tracking_locator_status() {
        let selection = selection_response();
        let import_status = DagDbContextPacketImportTrackingStatus {
            manifest_json: "{}".into(),
            manifest_status: "clean_manifest".into(),
            tracked_clean_evidence_enforced: true,
            source_path_status: "forbidden_locator".into(),
        };
        assert_eq!(
            build_graph_context_packet(&build_request(selection, Some(import_status))),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_renders_nonempty_graph_edges_section() {
        let mut selection = selection_response();
        assert!(
            selection.selected_memory_refs.len() >= 2,
            "fixture must select at least two memory refs"
        );
        let from = selection.selected_memory_refs[0].memory_id.clone();
        let to = selection.selected_memory_refs[1].memory_id.clone();
        selection
            .selected_graph_edges
            .push(exo_dag_db_api::DagDbSelectedGraphEdgeRef {
                graph_edge_id: "edge-1".into(),
                from_memory_id: from,
                to_memory_id: to,
                edge_kind: MemoryEdgeKind::RelatedTo,
                graph_style: MemoryGraphStyle::DependencyDag,
                selection_reason: "selected_edge_between_selected_memories".into(),
            });
        let packet = build_graph_context_packet(&build_request(selection, None)).expect("packet");
        assert!(!packet.selected_graph_edges.is_empty());
        assert!(packet.markdown.contains("## Selected Graph Edges"));
        assert!(packet.markdown.contains("edge-1"));
    }

    #[test]
    fn context_packet_output_renders_import_tracking_status_in_markdown() {
        let selection = selection_response();
        let import_status = DagDbContextPacketImportTrackingStatus {
            manifest_json: "{\"tracked\":true}".into(),
            manifest_status: "clean_manifest".into(),
            tracked_clean_evidence_enforced: true,
            source_path_status: CITATION_LOCATOR_BLOCKED.into(),
        };
        let packet = build_graph_context_packet(&build_request(selection, Some(import_status)))
            .expect("packet");
        assert!(packet.markdown.contains("import_manifest_status"));
        assert!(packet.markdown.contains("import_source_path_status"));
    }

    #[test]
    fn context_packet_output_renders_empty_memory_and_citation_sections() {
        let mut selection = selection_response();
        selection.selected_memory_refs.clear();
        selection.selected_graph_edges.clear();
        selection.selected_token_estimate = 0;

        let packet = build_graph_context_packet(&build_request(selection, None)).expect("packet");

        assert!(packet.selected_memory_refs.is_empty());
        assert!(packet.selected_graph_edges.is_empty());
        assert!(packet.citation_refs.is_empty());
        assert_eq!(packet.packet_metrics.selected_memory_ref_count, 0);
        assert_eq!(packet.packet_metrics.citation_ref_count, 0);
        assert_eq!(packet.markdown.matches("- none").count(), 3);
        assert!(packet.markdown.contains("## Selected Memory Refs\n- none"));
        assert!(packet.markdown.contains("## Citation Refs\n- none"));
    }

    #[test]
    fn context_packet_output_sorts_selected_edges_and_renders_edge_metadata() {
        let mut selection = selection_response();
        let first = selection.selected_memory_refs[0].memory_id.clone();
        let second = selection.selected_memory_refs[1].memory_id.clone();
        selection.selected_graph_edges = vec![
            exo_dag_db_api::DagDbSelectedGraphEdgeRef {
                graph_edge_id: "edge-b".into(),
                from_memory_id: second.clone(),
                to_memory_id: first.clone(),
                edge_kind: MemoryEdgeKind::RelatedTo,
                graph_style: MemoryGraphStyle::DependencyDag,
                selection_reason: "selected_edge".into(),
            },
            exo_dag_db_api::DagDbSelectedGraphEdgeRef {
                graph_edge_id: "edge-a".into(),
                from_memory_id: second,
                to_memory_id: first,
                edge_kind: MemoryEdgeKind::DependsOn,
                graph_style: MemoryGraphStyle::SemanticCatalogGraph,
                selection_reason: "selected_edge".into(),
            },
        ];

        let packet = build_graph_context_packet(&build_request(selection, None)).expect("packet");

        assert_eq!(packet.selected_graph_edges[0].graph_edge_id, "edge-a");
        assert_eq!(packet.selected_graph_edges[1].graph_edge_id, "edge-b");
        assert!(packet.markdown.contains("edge-a"));
        assert!(packet.markdown.contains("DependsOn"));
        assert!(packet.markdown.contains("SemanticCatalogGraph"));
        assert!(
            packet.markdown.find("edge-a").expect("edge-a rendered")
                < packet.markdown.find("edge-b").expect("edge-b rendered")
        );
    }

    #[test]
    fn context_packet_output_rejects_per_ref_token_over_budget() {
        let mut selection = selection_response();
        selection.selected_memory_refs[0].token_estimate = selection.token_budget.saturating_add(1);
        assert_eq!(
            build_graph_context_packet(&build_request(selection, None)),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_too_many_selected_refs_for_budget() {
        let mut selection = selection_response();
        selection.token_budget = 2;
        selection.selected_token_estimate = 0;
        selection.selected_memory_refs = vec![
            {
                let mut first = selection.selected_memory_refs[0].clone();
                first.token_estimate = 0;
                first
            },
            {
                let mut second = selection.selected_memory_refs[1].clone();
                second.token_estimate = 0;
                second
            },
            {
                let mut third = selection.selected_memory_refs[0].clone();
                third.memory_id = "mem-c".into();
                third.citation_ref = "citation:mem-c".into();
                third.token_estimate = 0;
                third
            },
        ];
        assert_eq!(
            build_graph_context_packet(&build_request(selection, None)),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_accepts_selected_ref_without_catalog_id() {
        let mut selection = selection_response();
        selection.selected_memory_refs[0].catalog_id = None;
        let packet = build_graph_context_packet(&build_request(selection, None)).expect("packet");
        assert!(packet.selected_memory_refs[0].catalog_id.is_none());
    }

    #[test]
    fn context_packet_output_rejects_forbidden_catalog_id_on_selected_ref() {
        let mut selection = selection_response();
        selection.selected_memory_refs[0].catalog_id = Some("Leaked DATABASE_URL".into());
        assert_eq!(
            build_graph_context_packet(&build_request(selection, None)),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_forbidden_material_in_serialized_packet() {
        let selection = selection_response();
        let mut request = build_request(selection, None);
        request.task = "safe task".into();
        let mut packet = build_graph_context_packet(&request).expect("packet");
        packet.markdown = "contains raw_markdown fragment".into();
        assert_eq!(
            validate_packet_no_forbidden_material(&packet),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_forbidden_material_in_deep_tier_summary() {
        // PRD-D3 (D3-S3): the drilldown pass renders the DEEP tier as a selected
        // ref's `summary` SafeMetadata. A poisoned deep tier must be rejected by
        // exactly the same validator path as a poisoned short tier — proving the
        // deep tier is screened, never relaxed. We poison the selected ref's
        // summary (the field the deep tier occupies) with each forbidden fragment
        // class and assert the packet fails closed, identical to a short-tier leak.
        for poisoned in [
            "deep detail leaks /Users/max/secret.md path",
            "deep detail sets DATABASE_URL=postgres://u:p@h/db",
            "deep detail embeds an api_key value",
            "deep detail dumps raw_markdown body",
            "deep detail names file://source.md",
        ] {
            let mut selection = selection_response();
            // The first selected ref's summary is what a drilldown deep tier fills.
            selection.selected_memory_refs[0].summary = safe(poisoned);
            let request = build_request(selection, None);
            assert_eq!(
                build_graph_context_packet(&request),
                Err(DomainError::ValidationFailed),
                "poisoned deep-tier summary must be rejected: {poisoned:?}"
            );
        }

        // The same fragment in the short tier (also the ref `summary`) is rejected
        // identically — the deep tier strengthens, never relaxes, the discipline.
        let mut short_tier = selection_response();
        short_tier.selected_memory_refs[0].summary = safe("short tier leaks postgres:// url");
        assert_eq!(
            build_graph_context_packet(&build_request(short_tier, None)),
            Err(DomainError::ValidationFailed),
        );
    }

    #[test]
    fn context_packet_output_rejects_forbidden_audit_id() {
        let selection = selection_response();
        let mut request = build_request(selection, None);
        request.audit_id = "Leaked postgres:// credential".into();
        assert_eq!(
            build_graph_context_packet(&request),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_forbidden_scope_identifiers() {
        let selection = selection_response();

        let mut forbidden_tenant = build_request(selection.clone(), None);
        forbidden_tenant.tenant_id = "tenant DATABASE_URL".into();
        forbidden_tenant.selection.tenant_id = forbidden_tenant.tenant_id.clone();
        assert_validation_failed(&forbidden_tenant);

        let mut forbidden_namespace = build_request(selection.clone(), None);
        forbidden_namespace.namespace = "namespace raw_body".into();
        forbidden_namespace.selection.namespace = forbidden_namespace.namespace.clone();
        assert_validation_failed(&forbidden_namespace);

        let mut forbidden_request_id = build_request(selection.clone(), None);
        forbidden_request_id.request_id = "request source_excerpt".into();
        forbidden_request_id.selection.request_id = forbidden_request_id.request_id.clone();
        assert_validation_failed(&forbidden_request_id);

        let mut forbidden_task_hash = build_request(selection, None);
        forbidden_task_hash.task_hash = "file://task-hash".into();
        forbidden_task_hash.selection.task_hash = forbidden_task_hash.task_hash.clone();
        assert_validation_failed(&forbidden_task_hash);
    }

    #[test]
    fn context_packet_output_rejects_forbidden_memory_and_edge_identifiers() {
        let selection = selection_response();

        let mut forbidden_memory_id = selection.clone();
        forbidden_memory_id.selected_memory_refs[0].memory_id = "raw_body memory".into();
        assert_validation_failed(&build_request(forbidden_memory_id, None));

        let from = selection.selected_memory_refs[0].memory_id.clone();
        let to = selection.selected_memory_refs[1].memory_id.clone();
        let safe_edge = exo_dag_db_api::DagDbSelectedGraphEdgeRef {
            graph_edge_id: "edge-safe".into(),
            from_memory_id: from,
            to_memory_id: to,
            edge_kind: MemoryEdgeKind::RelatedTo,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "selected_edge".into(),
        };

        let mut forbidden_edge_id = selection.clone();
        let mut edge = safe_edge.clone();
        edge.graph_edge_id = "edge DATABASE_URL".into();
        forbidden_edge_id.selected_graph_edges = vec![edge];
        assert_validation_failed(&build_request(forbidden_edge_id, None));

        let mut forbidden_edge_from = selection.clone();
        let mut edge = safe_edge.clone();
        edge.from_memory_id = "file://source".into();
        forbidden_edge_from.selected_graph_edges = vec![edge];
        assert_validation_failed(&build_request(forbidden_edge_from, None));

        let mut forbidden_edge_to = selection;
        let mut edge = safe_edge;
        edge.to_memory_id = "postgres://target".into();
        forbidden_edge_to.selected_graph_edges = vec![edge];
        assert_validation_failed(&build_request(forbidden_edge_to, None));
    }

    #[test]
    fn context_packet_output_caps_lossy_metric_counts_at_u32_max() {
        let one_past_u32_max = usize::try_from(u32::MAX)
            .expect("u32::MAX fits usize")
            .saturating_add(1);

        assert_eq!(u32_from_usize_lossy(7), 7);
        assert_eq!(u32_from_usize_lossy(one_past_u32_max), u32::MAX);
    }

    #[test]
    fn context_packet_output_rejects_forbidden_boundary_flag_on_selected_ref() {
        let mut selection = selection_response();
        selection.selected_memory_refs[0].boundary_flags = vec!["Leaked .env fragment".into()];
        assert_eq!(
            build_graph_context_packet(&build_request(selection, None)),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_mismatched_selection_token_budget() {
        let selection = selection_response();
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: selection.tenant_id.clone(),
            namespace: selection.namespace.clone(),
            request_id: selection.request_id.clone(),
            task: "Build bounded context packet for M02".into(),
            task_hash: selection.task_hash.clone(),
            audit_id: "audit-m02".into(),
            token_budget: selection.token_budget,
            max_memory_refs: None,
            selection: {
                let mut mismatched = selection;
                mismatched.token_budget = mismatched.token_budget.saturating_add(1);
                mismatched
            },
            import_tracking_status: None,
        };
        assert_eq!(
            build_graph_context_packet(&request),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn context_packet_output_rejects_each_selection_scope_mismatch() {
        let selection = selection_response();

        let mut wrong_namespace = build_request(selection.clone(), None);
        wrong_namespace.namespace = "secondary".into();
        assert_validation_failed(&wrong_namespace);

        let mut wrong_request_id = build_request(selection.clone(), None);
        wrong_request_id.request_id = "req-2".into();
        assert_validation_failed(&wrong_request_id);

        let mut wrong_task_hash = build_request(selection, None);
        wrong_task_hash.task_hash = h(0x77).to_string();
        assert_validation_failed(&wrong_task_hash);
    }

    #[test]
    fn context_packet_output_rejects_selected_token_estimate_over_budget() {
        let mut selection = selection_response();
        selection.selected_token_estimate = selection.token_budget.saturating_add(1);
        assert_validation_failed(&build_request(selection, None));
    }

    #[test]
    fn context_packet_output_rejects_selected_token_sum_overflow() {
        let mut selection = selection_response();
        let mut first = selection.selected_memory_refs[0].clone();
        first.memory_id = "mem-overflow-a".into();
        first.citation_ref = "citation:mem-overflow-a".into();
        first.token_estimate = u32::MAX; // pragma-allowlist-secret
        let mut second = selection.selected_memory_refs[1].clone();
        second.memory_id = "mem-overflow-b".into();
        second.citation_ref = "citation:mem-overflow-b".into();
        second.token_estimate = 1;
        selection.selected_memory_refs = vec![first, second];
        selection.selected_graph_edges.clear();
        selection.selected_token_estimate = u32::MAX; // pragma-allowlist-secret
        selection.token_budget = u32::MAX; // pragma-allowlist-secret

        assert_eq!(
            build_graph_context_packet(&build_request(selection, None)),
            Err(DomainError::ArithmeticOverflow {
                operation: "context_packet_selected_token_sum"
            })
        );
    }

    #[test]
    fn context_packet_output_rejects_forbidden_selected_ref_fields() {
        let selection = selection_response();

        let mut forbidden_title = selection.clone();
        forbidden_title.selected_memory_refs[0].title.text = "DATABASE_URL title".into();
        assert_validation_failed(&build_request(forbidden_title, None));

        let mut forbidden_summary_hash = selection.clone();
        forbidden_summary_hash.selected_memory_refs[0]
            .summary
            .original_hash = "postgres://hash".into();
        assert_validation_failed(&build_request(forbidden_summary_hash, None));

        let mut forbidden_catalog_path = selection.clone();
        forbidden_catalog_path.selected_memory_refs[0].catalog_path = vec!["raw_body".into()];
        assert_validation_failed(&build_request(forbidden_catalog_path, None));

        let mut forbidden_document_type = selection.clone();
        forbidden_document_type.selected_memory_refs[0].document_type = "file://plan".into();
        assert_validation_failed(&build_request(forbidden_document_type, None));

        let mut forbidden_selection_reason = selection.clone();
        forbidden_selection_reason.selected_memory_refs[0].selection_reason =
            "contains source_excerpt".into();
        assert_validation_failed(&build_request(forbidden_selection_reason, None));

        let mut forbidden_citation_ref = selection;
        forbidden_citation_ref.selected_memory_refs[0].citation_ref = "PRIVATE KEY citation".into();
        assert_validation_failed(&build_request(forbidden_citation_ref, None));
    }

    #[test]
    fn context_packet_output_rejects_forbidden_edge_and_import_fields() {
        let mut selection = selection_response();
        let from = selection.selected_memory_refs[0].memory_id.clone();
        let to = selection.selected_memory_refs[1].memory_id.clone();
        selection
            .selected_graph_edges
            .push(exo_dag_db_api::DagDbSelectedGraphEdgeRef {
                graph_edge_id: "edge-safe".into(),
                from_memory_id: from,
                to_memory_id: to,
                edge_kind: MemoryEdgeKind::RelatedTo,
                graph_style: MemoryGraphStyle::DependencyDag,
                selection_reason: "raw_private_payload".into(),
            });
        assert_validation_failed(&build_request(selection.clone(), None));

        let mut forbidden_manifest = DagDbContextPacketImportTrackingStatus {
            manifest_json: "{\"source_path\":\"/Users/max/project\"}".into(),
            manifest_status: "clean_manifest".into(),
            tracked_clean_evidence_enforced: true,
            source_path_status: CITATION_LOCATOR_BLOCKED.into(),
        };
        let clean_selection = selection_response();
        let valid_import_status = DagDbContextPacketImportTrackingStatus {
            manifest_json: "{}".into(),
            manifest_status: "clean_manifest".into(),
            tracked_clean_evidence_enforced: true,
            source_path_status: CITATION_LOCATOR_BLOCKED.into(),
        };
        build_graph_context_packet(&build_request(
            clean_selection.clone(),
            Some(valid_import_status),
        ))
        .expect("clean import status");
        assert_validation_failed(&build_request(
            clean_selection.clone(),
            Some(forbidden_manifest.clone()),
        ));

        forbidden_manifest.manifest_json = "{}".into();
        forbidden_manifest.manifest_status = "raw_markdown present".into();
        assert_validation_failed(&build_request(
            clean_selection.clone(),
            Some(forbidden_manifest.clone()),
        ));

        forbidden_manifest.manifest_status = "clean_manifest".into();
        forbidden_manifest.source_path_status = "locator_ready".into();
        assert_validation_failed(&build_request(clean_selection, Some(forbidden_manifest)));
    }

    #[test]
    fn layered_context_packet_exposes_prd03_fields_and_budget_report() {
        let selection = selection_response_with_edge();
        let request = build_request(selection, None);

        let packet = build_layered_context_packet_output(&request, layered_additions())
            .expect("layered packet");

        assert_eq!(
            packet.schema_version,
            LAYERED_CONTEXT_PACKET_OUTPUT_SCHEMA_VERSION
        );
        assert_eq!(
            packet.base_schema_version,
            GRAPH_CONTEXT_PACKET_SCHEMA_VERSION
        );
        assert_eq!(packet.selected_layers.len(), 2);
        assert_eq!(packet.selected_layers[0].layer_path, "root");
        assert_eq!(packet.selected_layers[1].layer_path, "root/repository");
        assert_eq!(packet.selected_layer_edges.len(), 1);
        assert_eq!(packet.selected_layer_edges[0].layer_edge_id, "layer-edge-a");
        assert_eq!(packet.selected_graph_edges.len(), 1);
        assert_eq!(packet.selected_refs.len(), 2);
        assert_eq!(packet.rollup_summaries.len(), 1);
        assert_eq!(packet.rollup_summaries[0].rollup_summary_ref, "rollup-root");
        assert_eq!(packet.hygiene_report.active_layer_edge_count, 3);
        assert_eq!(packet.hygiene_report.excluded_demoted_layer_edge_count, 1);
        assert_eq!(
            packet.hygiene_report.excluded_tombstoned_layer_edge_count,
            1
        );
        assert_eq!(packet.hygiene_report.stale_child_layer_count, 1);
        assert_eq!(packet.hygiene_report.rollup_refresh_required_count, 1);
        assert!(!packet.flat_fallback_used);
        assert_eq!(packet.budget_report.selected_layer_count, 2);
        assert_eq!(packet.budget_report.selected_layer_edge_count, 1);
        assert_eq!(packet.budget_report.active_layer_edge_count, 3);
        assert_eq!(packet.budget_report.excluded_demoted_layer_edge_count, 1);
        assert_eq!(packet.budget_report.excluded_tombstoned_layer_edge_count, 1);
        assert_eq!(packet.budget_report.stale_child_layer_count, 1);
        assert_eq!(packet.budget_report.rollup_refresh_required_count, 1);
        assert_eq!(packet.budget_report.rollup_summary_count, 1);
        assert_eq!(
            packet.budget_report.selected_ref_count,
            u32_from_usize_lossy(packet.selected_refs.len())
        );
        assert_eq!(packet.budget_report.budget_status, "within_budget");
        assert!(packet.markdown.contains("## Selected Layers"));
        assert!(packet.markdown.contains("## Layer Budget Report"));

        let serialized = serde_json::to_value(&packet).expect("json value");
        assert!(serialized.get("selected_layers").is_some());
        assert!(serialized.get("selected_layer_edges").is_some());
        assert!(serialized.get("selected_graph_edges").is_some());
        assert!(serialized.get("selected_refs").is_some());
        assert!(serialized.get("rollup_summaries").is_some());
        assert!(serialized.get("hygiene_report").is_some());
        assert!(serialized.get("budget_report").is_some());
        assert!(serialized.get("flat_fallback_used").is_some());
        assert!(
            packet
                .markdown
                .contains("excluded_demoted_layer_edge_count")
        );
    }

    #[test]
    fn layered_context_packet_labels_flat_fallback_when_layers_absent() {
        let selection = selection_response();
        let request = build_request(selection, None);

        let packet =
            build_layered_context_packet_output(&request, LayeredContextPacketAdditions::default())
                .expect("flat fallback packet");

        assert!(packet.selected_layers.is_empty());
        assert!(packet.selected_layer_edges.is_empty());
        assert!(packet.rollup_summaries.is_empty());
        assert!(packet.flat_fallback_used);
        assert_eq!(packet.budget_report.selected_layer_count, 0);
        assert_eq!(packet.budget_report.selected_layer_edge_count, 0);
        assert_eq!(packet.budget_report.active_layer_edge_count, 0);
        assert_eq!(packet.budget_report.excluded_demoted_layer_edge_count, 0);
        assert_eq!(packet.budget_report.excluded_tombstoned_layer_edge_count, 0);
        assert_eq!(packet.budget_report.rollup_summary_count, 0);
        assert!(packet.markdown.contains("- flat_fallback_used: true"));
    }

    #[test]
    fn layered_context_packet_output_is_deterministic() {
        let selection = selection_response_with_edge();
        let request = build_request(selection, None);

        let first = build_layered_context_packet_output(&request, layered_additions())
            .expect("first layered packet");
        let second = build_layered_context_packet_output(&request, layered_additions())
            .expect("second layered packet");

        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).expect("first json"),
            serde_json::to_string(&second).expect("second json")
        );
        assert!(!first.packet_hash.is_empty());
        assert_ne!(first.packet_hash, first.base_packet_hash);
    }

    #[test]
    fn layered_context_packet_rejects_forbidden_layer_material() {
        let selection = selection_response();
        let request = build_request(selection, None);

        for forbidden_path in [
            "/Users/max/project",
            "/home/max/project",
            "~/project",
            "root/../project",
            "root//project",
            "root\\project",
        ] {
            let mut forbidden_layer = layered_additions();
            forbidden_layer.selected_layers[0].layer_path = forbidden_path.into();
            assert_eq!(
                build_layered_context_packet_output(&request, forbidden_layer),
                Err(DomainError::ValidationFailed),
                "expected forbidden layer path to fail: {forbidden_path}"
            );
        }

        let mut forbidden_reason = layered_additions();
        forbidden_reason.selected_layers[0].selection_reason = "database_url matched".into();
        assert_eq!(
            build_layered_context_packet_output(&request, forbidden_reason),
            Err(DomainError::ValidationFailed)
        );

        let mut forbidden_rollup = layered_additions();
        forbidden_rollup.rollup_summaries[0].summary.text = "raw_body leaked".into();
        assert_eq!(
            build_layered_context_packet_output(&request, forbidden_rollup),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn layered_context_packet_rejects_dangling_layer_evidence() {
        let selection = selection_response();
        let request = build_request(selection, None);

        let mut dangling_edge = layered_additions();
        dangling_edge.selected_layer_edges[0].from_layer_id = "layer-missing".into();
        assert_eq!(
            build_layered_context_packet_output(&request, dangling_edge),
            Err(DomainError::ValidationFailed)
        );

        let mut dangling_rollup = layered_additions();
        dangling_rollup.rollup_summaries[0].layer_id = "layer-missing".into();
        assert_eq!(
            build_layered_context_packet_output(&request, dangling_rollup),
            Err(DomainError::ValidationFailed)
        );

        let mut missing_declared_rollup = layered_additions();
        missing_declared_rollup.selected_layers[1].rollup_summary_ref =
            Some("rollup-missing".into());
        assert_eq!(
            build_layered_context_packet_output(&request, missing_declared_rollup),
            Err(DomainError::ValidationFailed)
        );
    }

    fn selection_response_with_edge() -> DagDbGraphContextSelectionResponse {
        let mut selection = selection_response();
        let from = selection.selected_memory_refs[0].memory_id.clone();
        let to = selection.selected_memory_refs[1].memory_id.clone();
        selection
            .selected_graph_edges
            .push(exo_dag_db_api::DagDbSelectedGraphEdgeRef {
                graph_edge_id: "edge-1".into(),
                from_memory_id: from,
                to_memory_id: to,
                edge_kind: MemoryEdgeKind::RelatedTo,
                graph_style: MemoryGraphStyle::DependencyDag,
                selection_reason: "selected_edge_between_selected_memories".into(),
            });
        selection
    }

    fn layered_additions() -> LayeredContextPacketAdditions {
        LayeredContextPacketAdditions {
            selected_layers: vec![
                LayeredContextPacketSelectedLayer {
                    layer_id: "layer-repository".into(),
                    layer_path: "root/repository".into(),
                    layer_depth: 1,
                    graph_style: MemoryGraphStyle::DependencyDag,
                    selection_reason: "repository layer matched task".into(),
                    rollup_summary_ref: None,
                },
                LayeredContextPacketSelectedLayer {
                    layer_id: "layer-root".into(),
                    layer_path: "root".into(),
                    layer_depth: 0,
                    graph_style: MemoryGraphStyle::SemanticCatalogGraph,
                    selection_reason: "root layer anchors traversal".into(),
                    rollup_summary_ref: Some("rollup-root".into()),
                },
            ],
            selected_layer_edges: vec![LayeredContextPacketSelectedLayerEdge {
                layer_edge_id: "layer-edge-a".into(),
                from_layer_id: "layer-root".into(),
                to_layer_id: "layer-repository".into(),
                edge_kind: "contains_subgraph".into(),
                selection_reason: "drilldown matched repository evidence".into(),
            }],
            rollup_summaries: vec![LayeredContextPacketRollupSummary {
                rollup_summary_ref: "rollup-root".into(),
                layer_id: "layer-root".into(),
                summary: safe("Root layer rollup summary"),
                token_estimate: 40,
            }],
            hygiene_report: LayeredContextPacketHygieneReport {
                active_layer_edge_count: 3,
                excluded_demoted_layer_edge_count: 1,
                excluded_tombstoned_layer_edge_count: 1,
                stale_child_layer_count: 1,
                rollup_refresh_required_count: 1,
            },
            flat_fallback_used: false,
        }
    }
}
