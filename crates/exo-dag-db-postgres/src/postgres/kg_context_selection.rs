//! Read-only Postgres loader for persistent graph context selection (`M03`).
//!
//! Converts scoped DAG DB rows into `GraphContextSelectionState` for `M01`. This
//! module does not activate routes, persist packets, or mutate ledger state.

use std::collections::BTreeMap;

use exo_core::Hash256;
use exo_dag_db_api::{
    DagDbGraphContextSelectionRequest, MemoryEdgeKind, MemoryGraphStyle, SafeMetadata,
    SafeMetadataDecision, ValidationStatus,
};
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::{
    graph::MemoryGraphEdge,
    graph_context_selection::{GraphContextMemoryCandidate, GraphContextSelectionState},
    kg_import::hash_from_hex,
    kg_retrieval::{citation_handle, hex_from_hash_column, memory_token_estimate},
    scoring::{DomainError, DomainResult},
};

const FORBIDDEN_TEXT_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "file://",
    "postgres://",
    "postgresql://",
    "database_url",
    "database_url=",
    "begin private key",
    "private key-----",
    "raw_markdown",
    "raw_body",
    "raw_private_payload",
    "source_path",
];

/// Loaded Postgres row counts and graph selection state for persistent context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPersistentGraphContext {
    pub memory_row_count: u32,
    pub catalog_row_count: u32,
    pub graph_edge_row_count: u32,
    pub validation_row_count: u32,
    pub receipt_row_count: u32,
    pub skipped_row_count: u32,
    pub memory_receipt_hashes: BTreeMap<String, String>,
    pub boundary_warnings: Vec<String>,
    pub state: GraphContextSelectionState,
}

/// Load scoped Postgres rows and convert them into graph selection state.
pub async fn load_persistent_graph_context_state(
    pool: &PgPool,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<LoadedPersistentGraphContext> {
    validate_selection_request(request)?;

    let mut tx = super::begin_tenant_transaction(pool, &request.tenant_id)
        .await
        .map_err(pg)?;
    let memories = load_memories(&mut tx, request).await?;
    let catalogs = load_catalogs(&mut tx, request).await?;
    let graph_nodes = load_graph_nodes(&mut tx, request).await?;
    let raw_edges = load_graph_edges(&mut tx, request).await?;
    let validation_reports = load_validation_reports(&mut tx, request).await?;
    let receipt_ids = load_receipt_ids(&mut tx, request).await?;
    tx.commit().await.map_err(pg)?;

    let memory_row_count = usize_to_u32(memories.len(), "memory_row_count")?;
    let catalog_row_count = usize_to_u32(catalogs.len(), "catalog_row_count")?;
    let graph_edge_row_count = usize_to_u32(raw_edges.len(), "graph_edge_row_count")?;
    let validation_row_count = usize_to_u32(
        validation_reports.values().map(Vec::len).sum::<usize>(),
        "validation_row_count",
    )?;
    let receipt_row_count = usize_to_u32(receipt_ids.len(), "receipt_row_count")?;
    let memory_receipt_hashes = memories
        .iter()
        .map(|(memory_id, memory)| (memory_id.clone(), memory.latest_receipt_hash.clone()))
        .collect();

    let mut boundary_warnings = Vec::new();
    push_warning(
        &mut boundary_warnings,
        "read_only_repository_test_persistent_read",
    );
    push_warning(
        &mut boundary_warnings,
        "markdown_content_and_origin_path_not_returned",
    );

    let (memory_candidates, skipped_row_count) = build_memory_candidates(
        request,
        &memories,
        &catalogs,
        &graph_nodes,
        &mut boundary_warnings,
    )?;

    let mut graph_edges = Vec::new();
    for edge in raw_edges {
        graph_edges.push(edge.into_memory_graph_edge(&request.tenant_id, &request.namespace)?);
    }

    let state = GraphContextSelectionState {
        memory_candidates,
        graph_edges,
        receipt_ids,
    };

    Ok(LoadedPersistentGraphContext {
        memory_row_count,
        catalog_row_count,
        graph_edge_row_count,
        validation_row_count,
        receipt_row_count,
        skipped_row_count,
        memory_receipt_hashes,
        boundary_warnings,
        state,
    })
}

fn build_memory_candidates(
    request: &DagDbGraphContextSelectionRequest,
    memories: &BTreeMap<String, RetrievedMemory>,
    catalogs: &BTreeMap<String, RetrievedCatalog>,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
    boundary_warnings: &mut Vec<String>,
) -> DomainResult<(Vec<GraphContextMemoryCandidate>, u32)> {
    let mut candidates = Vec::new();
    let mut skipped = 0u32;

    for (memory_id, memory) in memories {
        if !memory_status_allowed(&memory.status) {
            skipped = skipped.saturating_add(1);
            push_warning(
                boundary_warnings,
                format!("skipped_memory_status:{memory_id}"),
            );
            continue;
        }
        if validation_status_blocked(&memory.validation_status) && !request.force_revalidate {
            skipped = skipped.saturating_add(1);
            push_warning(
                boundary_warnings,
                format!("skipped_validation_status:{memory_id}"),
            );
            continue;
        }

        let title = safe_metadata_from_value("memory.title", memory.title.clone())?;
        let summary = safe_metadata_from_value("memory.summary", memory.summary.clone())?;
        let catalog_path = first_catalog_path(memory_id, graph_nodes);
        let document_type = infer_document_type(&catalog_path, &title.text, &summary.text);
        let catalog_id = catalogs
            .get(memory_id)
            .map(|entry| entry.catalog_id.clone());
        let citation_ref = citation_handle(
            &request.tenant_id,
            &request.namespace,
            memory_id,
            catalog_id.as_deref(),
        )
        .map_err(|_| DomainError::ValidationFailed)?;
        let validation_status = parse_validation_status(&memory.validation_status)?;

        candidates.push(GraphContextMemoryCandidate {
            memory_id: memory_id.clone(),
            tenant_id: request.tenant_id.clone(),
            namespace: request.namespace.clone(),
            catalog_id,
            title: title.clone(),
            summary: summary.clone(),
            catalog_path,
            document_type,
            token_estimate: memory_token_estimate(&title, &summary),
            validation_status,
            citation_ref,
            boundary_flags: vec!["repository_test_only".into()],
        });
    }

    candidates.sort_by(|left, right| left.memory_id.cmp(&right.memory_id));
    Ok((candidates, skipped))
}

async fn load_memories(
    tx: &mut Transaction<'_, Postgres>,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<BTreeMap<String, RetrievedMemory>> {
    // PRD-D4: exclude the telemetry facet by STRUCTURE. Usage-event and
    // context-packet rows are their own facet (node_type 'usage_event' /
    // 'context_packet') and must never enter the packet-selection candidate
    // pool. This is the structural replacement for the retired read-side
    // title-prefix heuristic and the telemetry-ref quota: telemetry simply is
    // not a candidate, so the prior `usage_event_ratio_too_high` regression
    // cannot recur. The unified store is retained — telemetry stays in this
    // table and is still reachable via recall/operator read paths
    // (`kg_retrieval`), it is only kept out of packet selection.
    let rows = sqlx::query(
        "SELECT memory_id, title, summary, status, validation_status, latest_receipt_hash \
         FROM dagdb_memory_objects \
         WHERE tenant_id = $1 AND namespace = $2 \
           AND node_type NOT IN ('usage_event', 'context_packet') \
         ORDER BY memory_id",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .fetch_all(&mut **tx)
    .await
    .map_err(pg)?;

    let mut memories = BTreeMap::new();
    for row in rows {
        let memory_id = hex_field("memory_id", row.try_get("memory_id").map_err(pg)?)?;
        memories.insert(
            memory_id,
            RetrievedMemory {
                title: row.try_get("title").map_err(pg)?,
                summary: row.try_get("summary").map_err(pg)?,
                status: row.try_get("status").map_err(pg)?,
                validation_status: row.try_get("validation_status").map_err(pg)?,
                latest_receipt_hash: hex_field(
                    "memory.latest_receipt_hash",
                    row.try_get("latest_receipt_hash").map_err(pg)?,
                )?,
            },
        );
    }
    Ok(memories)
}

async fn load_catalogs(
    tx: &mut Transaction<'_, Postgres>,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<BTreeMap<String, RetrievedCatalog>> {
    let rows = sqlx::query(
        "SELECT catalog_id, memory_id \
         FROM dagdb_catalog_entries \
         WHERE tenant_id = $1 AND namespace = $2 AND memory_id IS NOT NULL \
         ORDER BY memory_id, catalog_id",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .fetch_all(&mut **tx)
    .await
    .map_err(pg)?;

    let mut catalogs = BTreeMap::new();
    for row in rows {
        let memory_id = hex_field("catalog.memory_id", row.try_get("memory_id").map_err(pg)?)?;
        let catalog_id = hex_field("catalog_id", row.try_get("catalog_id").map_err(pg)?)?;
        catalogs
            .entry(memory_id)
            .or_insert(RetrievedCatalog { catalog_id });
    }
    Ok(catalogs)
}

async fn load_graph_nodes(
    tx: &mut Transaction<'_, Postgres>,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<BTreeMap<String, Vec<RetrievedGraphNode>>> {
    let rows = sqlx::query(
        "SELECT graph_node_id, memory_id, catalog_path \
         FROM dagdb_graph_nodes node \
         WHERE node.tenant_id = $1 AND node.namespace = $2 \
         ORDER BY memory_id, catalog_path, graph_node_id",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .fetch_all(&mut **tx)
    .await
    .map_err(pg)?;

    let mut graph_nodes: BTreeMap<String, Vec<RetrievedGraphNode>> = BTreeMap::new();
    for row in rows {
        let memory_id = hex_field(
            "graph_node.memory_id",
            row.try_get("memory_id").map_err(pg)?,
        )?;
        let graph_node_id = hex_field("graph_node_id", row.try_get("graph_node_id").map_err(pg)?)?;
        let catalog_path = row
            .try_get::<Option<String>, _>("catalog_path")
            .map_err(pg)?
            .unwrap_or_default();
        ensure_safe_text("graph_node.catalog_path", &catalog_path)?;
        graph_nodes
            .entry(memory_id)
            .or_default()
            .push(RetrievedGraphNode {
                graph_node_id,
                catalog_path: safe_catalog_path(&catalog_path),
            });
    }
    Ok(graph_nodes)
}

async fn load_graph_edges(
    tx: &mut Transaction<'_, Postgres>,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<Vec<RetrievedGraphEdge>> {
    let rows = sqlx::query(
        "SELECT graph_edge_id, graph_style, from_memory_id, to_memory_id, edge_kind, receipt_hash \
         FROM dagdb_graph_edges edge \
         WHERE edge.tenant_id = $1 AND edge.namespace = $2 \
           AND NOT EXISTS ( \
             SELECT 1 FROM dagdb_graph_edge_tombstones tombstone \
             WHERE tombstone.tenant_id = edge.tenant_id \
               AND tombstone.namespace = edge.namespace \
               AND tombstone.prior_edge_id = edge.graph_edge_id \
           ) \
         ORDER BY graph_edge_id",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .fetch_all(&mut **tx)
    .await
    .map_err(pg)?;

    let mut edges = Vec::new();
    for row in rows {
        let edge_kind: String = row.try_get("edge_kind").map_err(pg)?;
        let graph_style: String = row.try_get("graph_style").map_err(pg)?;
        ensure_safe_text("graph_edge.edge_kind", &edge_kind)?;
        ensure_safe_text("graph_edge.graph_style", &graph_style)?;
        edges.push(RetrievedGraphEdge {
            edge_id: hex_field("graph_edge_id", row.try_get("graph_edge_id").map_err(pg)?)?,
            from_memory_id: hex_field(
                "edge.from_memory_id",
                row.try_get("from_memory_id").map_err(pg)?,
            )?,
            to_memory_id: hex_field(
                "edge.to_memory_id",
                row.try_get("to_memory_id").map_err(pg)?,
            )?,
            edge_kind,
            graph_style,
            receipt_hash: row
                .try_get::<Option<Vec<u8>>, _>("receipt_hash")
                .map_err(pg)?
                .map(|hash| hex_field("edge.receipt_hash", hash))
                .transpose()?,
        });
    }
    Ok(edges)
}

async fn load_validation_reports(
    tx: &mut Transaction<'_, Postgres>,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<BTreeMap<String, Vec<String>>> {
    let rows = sqlx::query(
        "SELECT validation_report_id, subject_id \
         FROM dagdb_validation_reports \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = 'memory' \
         ORDER BY subject_id, validation_report_id",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .fetch_all(&mut **tx)
    .await
    .map_err(pg)?;

    let mut reports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let memory_id = hex_field(
            "validation.subject_id",
            row.try_get("subject_id").map_err(pg)?,
        )?;
        let report_id = hex_field(
            "validation_report_id",
            row.try_get("validation_report_id").map_err(pg)?,
        )?;
        reports.entry(memory_id).or_default().push(report_id);
    }
    Ok(reports)
}

async fn load_receipt_ids(
    tx: &mut Transaction<'_, Postgres>,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<Vec<Hash256>> {
    let rows = sqlx::query(
        "SELECT receipt_hash \
         FROM dagdb_receipts \
         WHERE tenant_id = $1 AND namespace = $2 \
         ORDER BY receipt_hash",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .fetch_all(&mut **tx)
    .await
    .map_err(pg)?;

    let mut receipt_ids = Vec::new();
    for row in rows {
        let receipt_hex = hex_field("receipt_hash", row.try_get("receipt_hash").map_err(pg)?)?;
        receipt_ids.push(
            hash_from_hex("receipt_hash", &receipt_hex)
                .map_err(|_| DomainError::ValidationFailed)?,
        );
    }
    Ok(receipt_ids)
}

#[derive(Debug, Clone)]
struct RetrievedMemory {
    title: JsonValue,
    summary: JsonValue,
    status: String,
    validation_status: String,
    latest_receipt_hash: String,
}

#[derive(Debug, Clone)]
struct RetrievedCatalog {
    catalog_id: String,
}

#[derive(Debug, Clone)]
struct RetrievedGraphNode {
    #[allow(dead_code)]
    graph_node_id: String,
    catalog_path: String,
}

#[derive(Debug, Clone)]
struct RetrievedGraphEdge {
    edge_id: String,
    from_memory_id: String,
    to_memory_id: String,
    edge_kind: String,
    graph_style: String,
    receipt_hash: Option<String>,
}

impl RetrievedGraphEdge {
    fn into_memory_graph_edge(
        self,
        tenant_id: &str,
        namespace: &str,
    ) -> DomainResult<MemoryGraphEdge> {
        let edge_kind = parse_memory_edge_kind(&self.edge_kind)?;
        let graph_style = parse_memory_graph_style(&self.graph_style)?;
        let provenance_receipt_id = self
            .receipt_hash
            .map(|hash| hash_from_hex("edge.receipt_hash", &hash))
            .transpose()
            .map_err(|_| DomainError::ValidationFailed)?;
        Ok(MemoryGraphEdge {
            edge_id: hash_from_hex("graph_edge_id", &self.edge_id)
                .map_err(|_| DomainError::ValidationFailed)?,
            tenant_id: tenant_id.to_owned(),
            namespace: namespace.to_owned(),
            from_memory_id: hash_from_hex("from_memory_id", &self.from_memory_id)
                .map_err(|_| DomainError::ValidationFailed)?,
            to_memory_id: hash_from_hex("to_memory_id", &self.to_memory_id)
                .map_err(|_| DomainError::ValidationFailed)?,
            edge_kind,
            graph_style,
            provenance_receipt_id,
        })
    }
}

fn validate_selection_request(request: &DagDbGraphContextSelectionRequest) -> DomainResult<()> {
    if request.tenant_id.trim().is_empty() || request.namespace.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    if request.request_id.trim().is_empty() || request.task.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    if request.token_budget == 0 || request.max_memory_refs == 0 {
        return Err(DomainError::ValidationFailed);
    }
    validate_no_forbidden_material(&request.tenant_id)?;
    validate_no_forbidden_material(&request.namespace)?;
    validate_no_forbidden_material(&request.request_id)?;
    validate_no_forbidden_material(&request.task)?;
    validate_no_forbidden_material(&request.task_hash)?;
    for hint in &request.catalog_hints {
        validate_no_forbidden_material(hint)?;
    }
    for memory_id in &request.requested_memory_ids {
        validate_no_forbidden_material(memory_id)?;
        hash_from_hex("requested_memory_id", memory_id)
            .map_err(|_| DomainError::ValidationFailed)?;
    }
    Ok(())
}

fn safe_metadata_from_value(field: &str, value: JsonValue) -> DomainResult<SafeMetadata> {
    let metadata: SafeMetadata =
        serde_json::from_value(value).map_err(|error| DomainError::HashMaterial {
            reason: format!("{field}_json: {}", error),
        })?;
    if metadata.decision == SafeMetadataDecision::Reject {
        return Err(DomainError::ValidationFailed);
    }
    ensure_safe_text(field, &metadata.text)?;
    Ok(metadata)
}

fn parse_validation_status(value: &str) -> DomainResult<ValidationStatus> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|_| DomainError::ValidationFailed)
}

fn parse_memory_edge_kind(value: &str) -> DomainResult<MemoryEdgeKind> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|_| DomainError::ValidationFailed)
}

fn parse_memory_graph_style(value: &str) -> DomainResult<MemoryGraphStyle> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|_| DomainError::ValidationFailed)
}

fn infer_document_type(catalog_path: &[String], title: &str, summary: &str) -> String {
    let haystack = format!(
        "{} {} {}",
        title.to_ascii_lowercase(),
        summary.to_ascii_lowercase(),
        catalog_path.join("/").to_ascii_lowercase()
    );
    if haystack.contains("blocker") || haystack.contains("open question") {
        return "blocker".into();
    }
    if haystack.contains("plan")
        || haystack.contains("next step")
        || haystack.contains("implementation")
    {
        return "plan".into();
    }
    if haystack.contains("route") {
        return "route".into();
    }
    if let Some(last) = catalog_path.last() {
        let normalized = last.to_ascii_lowercase();
        if normalized.contains("plan") {
            return "plan".into();
        }
        if normalized.contains("blocker") || normalized.contains("open-question") {
            return "blocker".into();
        }
    }
    "summary".into()
}

fn first_catalog_path(
    memory_id: &str,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
) -> Vec<String> {
    graph_nodes
        .get(memory_id)
        .and_then(|nodes| nodes.first())
        .map(|node| split_catalog_path(&node.catalog_path))
        .unwrap_or_else(|| vec!["uncataloged".into()])
}

fn split_catalog_path(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn safe_catalog_path(value: &str) -> String {
    value
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect::<Vec<_>>()
        .join("/")
}

fn memory_status_allowed(status: &str) -> bool {
    matches!(status, "pending" | "approved" | "routable")
}

fn validation_status_blocked(validation_status: &str) -> bool {
    matches!(validation_status, "failed" | "contradictory" | "expired")
}

fn ensure_safe_text(field: &str, value: &str) -> DomainResult<()> {
    if value.is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    let normalized = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_TEXT_FRAGMENTS
        .iter()
        .find(|fragment| normalized.contains(**fragment))
    {
        return Err(DomainError::HashMaterial {
            reason: format!("{field} contains forbidden fragment {fragment}"),
        });
    }
    if is_probable_local_absolute_path(value) {
        return Err(DomainError::HashMaterial {
            reason: format!("{field} contains probable local absolute path"),
        });
    }
    Ok(())
}

fn validate_no_forbidden_material(value: &str) -> DomainResult<()> {
    ensure_safe_text("request_field", value)
}

fn is_probable_local_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("~/")
        || (value.len() > 2
            && value.as_bytes()[1] == b':'
            && (value.as_bytes()[2] == b'\\' || value.as_bytes()[2] == b'/'))
}

fn push_warning(warnings: &mut Vec<String>, warning: impl Into<String>) {
    let warning = warning.into();
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn usize_to_u32(value: usize, field: &'static str) -> DomainResult<u32> {
    u32::try_from(value).map_err(|_| DomainError::ArithmeticOverflow { operation: field })
}

fn hex_field(field: &str, bytes: Vec<u8>) -> DomainResult<String> {
    hex_from_hash_column(field, bytes).map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

fn pg(source: sqlx::Error) -> DomainError {
    DomainError::HashMaterial {
        reason: format!("persistent_context_postgres: {source}"),
    }
}
