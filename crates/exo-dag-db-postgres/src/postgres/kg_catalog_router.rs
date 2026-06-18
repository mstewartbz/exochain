//! Feature-gated read-only catalog-router preview over persisted KG rows.
//!
//! This adapter derives repository/test route diagnostics from existing DAG DB
//! rows. It does not activate routes, persist reports, enqueue finality, expose
//! gateway behavior, or mutate ledger state.

use std::collections::{BTreeMap, BTreeSet};

use exo_dag_db_api::{SafeMetadata, SafeMetadataDecision};
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use thiserror::Error;

use crate::{
    graph_context_selection::MAX_SELECTED_GRAPH_EDGES_PER_PACKET,
    kg_catalog_router::{
        KG_CATALOG_ROUTER_PREVIEW_SCHEMA, KgCatalogPathCandidate, KgCatalogRouterBoundaries,
        KgCatalogRouterEdgeActionClassification, KgCatalogRouterError, KgCatalogRouterGraphEdgeRef,
        KgCatalogRouterMemoryRef, KgCatalogRouterOmittedRef, KgCatalogRouterPacketMetrics,
        KgCatalogRouterPreview, KgCatalogRouterScoreComponent,
        KgCatalogRouterSubgraphRecommendationKind, KgCatalogRouterTaskInput,
        KgSelectedCatalogRoute, KgSubgraphDelegationRecommendation,
    },
    kg_retrieval::{
        KG_RETRIEVAL_DATABASE_URL_ENV, KgRetrievalError, citation_handle, hex_from_hash_column,
        memory_token_estimate,
    },
};

const MAX_BP: u32 = 10_000;
const CATALOG_HINT_WEIGHT_BP: u32 = 2_000;
const TASK_TERM_WEIGHT_BP: u32 = 2_500;
const REQUESTED_MEMORY_WEIGHT_BP: u32 = 1_500;
const VALIDATION_WEIGHT_BP: u32 = 1_500;
const RECEIPT_WEIGHT_BP: u32 = 1_500;
const MEMORY_DENSITY_WEIGHT_BP: u32 = 1_000;

const FORBIDDEN_TEXT_FRAGMENTS: &[&str] = &[
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

/// Errors raised by read-only catalog-router Postgres previews.
#[derive(Debug, Error)]
pub enum KgCatalogRouterPostgresError {
    /// No database URL was supplied for persisted preview mode.
    #[error("kg_catalog_router_database_url_missing: {env_var}")]
    MissingDatabaseUrl {
        /// Required env var.
        env_var: &'static str,
    },
    /// Catalog-router DTO validation failed.
    #[error(transparent)]
    Preview(#[from] KgCatalogRouterError),
    /// Retrieval helper failed while decoding persisted hash or citation data.
    #[error(transparent)]
    Retrieval(#[from] KgRetrievalError),
    /// Postgres foundation failed.
    #[error("kg_catalog_router_postgres_init_failed")]
    Init {
        /// Source Postgres foundation error.
        #[source]
        source: super::DagDbPostgresError,
    },
    /// SQL operation failed.
    #[error("kg_catalog_router_postgres_failed")]
    Postgres {
        /// Source SQLx error.
        #[source]
        source: sqlx::Error,
    },
    /// JSON conversion failed.
    #[error("kg_catalog_router_json_failed: {reason}")]
    Json {
        /// Stable conversion reason.
        reason: String,
    },
    /// Persisted safe metadata cannot be returned in route diagnostics.
    #[error("kg_catalog_router_unsafe_metadata: {field}: {reason}")]
    UnsafeMetadata {
        /// Field being decoded.
        field: String,
        /// Stable rejection reason.
        reason: String,
    },
    /// Count cannot fit in the DTO.
    #[error("kg_catalog_router_count_out_of_range: {field}")]
    CountOutOfRange {
        /// Count field.
        field: String,
    },
}

/// Result alias for read-only catalog-router Postgres previews.
pub type Result<T> = std::result::Result<T, KgCatalogRouterPostgresError>;

/// Build a catalog-router preview using `EXO_DAGDB_TEST_DATABASE_URL`.
pub async fn build_kg_catalog_router_preview_from_env(
    task_input: &KgCatalogRouterTaskInput,
) -> Result<KgCatalogRouterPreview> {
    let database_url = std::env::var(KG_RETRIEVAL_DATABASE_URL_ENV).map_err(|_| {
        KgCatalogRouterPostgresError::MissingDatabaseUrl {
            env_var: KG_RETRIEVAL_DATABASE_URL_ENV,
        }
    })?;
    build_kg_catalog_router_preview_from_database_url(Some(database_url.as_str()), task_input).await
}

/// Build a catalog-router preview using an explicit database URL.
pub async fn build_kg_catalog_router_preview_from_database_url(
    database_url: Option<&str>,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<KgCatalogRouterPreview> {
    let Some(database_url) = database_url else {
        return Err(KgCatalogRouterPostgresError::MissingDatabaseUrl {
            env_var: KG_RETRIEVAL_DATABASE_URL_ENV,
        });
    };
    let pool = super::init_pool(database_url)
        .await
        .map_err(|source| KgCatalogRouterPostgresError::Init { source })?;
    let result = build_kg_catalog_router_preview(&pool, task_input).await;
    pool.close().await;
    result
}

/// Build a validated read-only catalog-router preview from an existing pool.
pub async fn build_kg_catalog_router_preview(
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<KgCatalogRouterPreview> {
    task_input.validate_request()?;
    let memories = load_memories(pool, task_input).await?;
    let catalogs = load_catalogs(pool, task_input).await?;
    let graph_nodes = load_graph_nodes(pool, task_input).await?;
    let graph_edges = load_graph_edges(pool, task_input).await?;
    let validation_reports = load_validation_reports(pool, task_input).await?;
    let receipt_hashes = load_receipt_hashes(pool, task_input).await?;

    let preview = build_preview(
        task_input,
        memories,
        catalogs,
        graph_nodes,
        graph_edges,
        validation_reports,
        receipt_hashes,
    )?;
    preview.validate()?;
    Ok(preview)
}

fn build_preview(
    task_input: &KgCatalogRouterTaskInput,
    memories: BTreeMap<String, RetrievedMemory>,
    catalogs: BTreeMap<String, RetrievedCatalog>,
    graph_nodes: BTreeMap<String, Vec<RetrievedGraphNode>>,
    graph_edges: Vec<RetrievedGraphEdge>,
    validation_reports: BTreeMap<String, Vec<String>>,
    receipt_hashes: BTreeSet<String>,
) -> Result<KgCatalogRouterPreview> {
    let mut warnings = base_boundary_warnings();
    push_warning(&mut warnings, "read_only_repository_test_preview");
    push_warning(&mut warnings, "markdown_content_not_returned");
    push_warning(&mut warnings, "origin_path_not_returned");

    let candidate_memory_ids = candidate_memory_ids_by_path(&memories, &graph_nodes);
    let total_eligible_count = candidate_memory_ids
        .values()
        .map(BTreeSet::len)
        .sum::<usize>();
    let task_terms = task_terms(&task_input.task_description);
    let candidate_context = CandidateBuildContext {
        task_input,
        memories: &memories,
        validation_reports: &validation_reports,
        receipt_hashes: &receipt_hashes,
        task_terms: &task_terms,
        total_eligible_count,
    };
    let mut catalog_path_candidates = Vec::new();
    for (catalog_path, memory_ids) in &candidate_memory_ids {
        let candidate = build_candidate(&candidate_context, catalog_path, memory_ids)?;
        catalog_path_candidates.push(candidate);
    }
    sort_candidates(&mut catalog_path_candidates);

    if catalog_path_candidates.is_empty() {
        push_warning(&mut warnings, "no_catalog_route_candidates");
    }

    let selected_path = catalog_path_candidates
        .first()
        .map(|candidate| candidate.catalog_path.clone())
        .unwrap_or_default();
    let mut selected_memory_refs = Vec::new();
    let mut omitted_reasons = BTreeMap::<String, String>::new();
    let mut selected_token_estimate = 0u32;
    let mut truncated_by_token_budget = false;
    let mut truncated_by_max_memory_refs = false;

    let selected_path_memory_ids = if selected_path.is_empty() {
        BTreeSet::new()
    } else {
        candidate_memory_ids
            .get(&selected_path)
            .cloned()
            .unwrap_or_default()
    };
    let mut route_memory_ids = selected_path_memory_ids.iter().cloned().collect::<Vec<_>>();
    route_memory_ids.sort_by(|left, right| {
        let left_memory = memories.get(left);
        let right_memory = memories.get(right);
        memory_sort_key(left, left_memory, &graph_nodes).cmp(&memory_sort_key(
            right,
            right_memory,
            &graph_nodes,
        ))
    });

    let requested_filter = !task_input.requested_memory_refs.is_empty();
    for requested_memory_id in &task_input.requested_memory_refs {
        if !memories.contains_key(requested_memory_id) {
            omitted_reasons.insert(
                requested_memory_id.clone(),
                "requested_memory_not_found".to_owned(),
            );
            push_warning(
                &mut warnings,
                format!("requested_memory_not_found:{requested_memory_id}"),
            );
        }
    }

    for memory_id in &route_memory_ids {
        let Some(memory) = memories.get(memory_id) else {
            continue;
        };
        if requested_filter && !task_input.requested_memory_refs.contains(memory_id) {
            omitted_reasons.insert(
                memory_id.clone(),
                "requested_memory_filter_mismatch".to_owned(),
            );
            continue;
        }
        if !memory_status_allowed(&memory.status) {
            omitted_reasons.insert(memory_id.clone(), "memory_status_not_routable".to_owned());
            continue;
        }
        if validation_status_blocked(&memory.validation_status) {
            omitted_reasons.insert(
                memory_id.clone(),
                "validation_status_not_allowed".to_owned(),
            );
            continue;
        }
        let token_estimate = memory_token_estimate(&memory.title, &memory.summary);
        if usize_to_u32("selected_memory_refs", selected_memory_refs.len())?
            >= task_input.max_memory_refs
        {
            omitted_reasons.insert(memory_id.clone(), "max_memory_refs_exceeded".to_owned());
            truncated_by_max_memory_refs = true;
            continue;
        }
        if selected_token_estimate.saturating_add(token_estimate) > task_input.token_budget {
            omitted_reasons.insert(memory_id.clone(), "token_budget_exceeded".to_owned());
            truncated_by_token_budget = true;
            continue;
        }

        let catalog = catalogs.get(memory_id);
        let citation = citation_handle(
            &task_input.tenant_id,
            &task_input.namespace,
            memory_id,
            catalog.map(|entry| entry.catalog_id.as_str()),
        )?;
        selected_token_estimate = selected_token_estimate.saturating_add(token_estimate);
        selected_memory_refs.push(KgCatalogRouterMemoryRef {
            memory_id: memory_id.clone(),
            catalog_id: catalog.map(|entry| entry.catalog_id.clone()),
            catalog_path: selected_path.clone(),
            title: memory.title.text.clone(),
            summary: memory.summary.text.clone(),
            selection_reason: "selected_from_db_backed_catalog_route".to_owned(),
            token_estimate,
            citation_handle: citation,
            validation_status: memory.validation_status.clone(),
            graph_node_ids: graph_node_ids(memory_id, &graph_nodes),
        });
    }

    for (memory_id, memory) in &memories {
        if selected_memory_refs
            .iter()
            .any(|selected| selected.memory_id == *memory_id)
            || omitted_reasons.contains_key(memory_id)
        {
            continue;
        }
        let reason = if selected_path.is_empty() {
            "no_selected_catalog_route"
        } else if !memory_status_allowed(&memory.status) {
            "memory_status_not_routable"
        } else if validation_status_blocked(&memory.validation_status) {
            "validation_status_not_allowed"
        } else if !memory_has_catalog_path(memory_id, &selected_path, &graph_nodes) {
            "outside_selected_catalog_route"
        } else {
            "not_selected_by_catalog_route"
        };
        omitted_reasons.insert(memory_id.clone(), reason.to_owned());
    }

    if selected_memory_refs.is_empty() {
        push_warning(&mut warnings, "no_selected_memory_refs");
    }
    if truncated_by_token_budget {
        push_warning(&mut warnings, "context_truncated_by_token_budget");
    }
    if truncated_by_max_memory_refs {
        push_warning(&mut warnings, "context_truncated_by_max_memory_refs");
    }

    sort_selected_refs(&mut selected_memory_refs);
    let selected_ids = selected_memory_refs
        .iter()
        .map(|memory| memory.memory_id.clone())
        .collect::<BTreeSet<_>>();
    let mut selected_graph_edges = selected_graph_edges(&graph_edges, &selected_ids);
    sort_edges(&mut selected_graph_edges);
    if selected_graph_edges.len() > MAX_SELECTED_GRAPH_EDGES_PER_PACKET {
        selected_graph_edges.truncate(MAX_SELECTED_GRAPH_EDGES_PER_PACKET);
        push_warning(&mut warnings, "selected_graph_edges_truncated_by_budget");
    }
    if selected_graph_edges.is_empty() && selected_memory_refs.len() > 1 {
        push_warning(&mut warnings, "selected_graph_edges_empty");
    }

    let mut omitted_refs =
        build_omitted_refs(&omitted_reasons, &memories, &graph_nodes, &selected_path);
    sort_omitted_refs(&mut omitted_refs);

    let subgraph_recommendation = subgraph_recommendation(
        &selected_path,
        &selected_path_memory_ids,
        &selected_graph_edges,
        &memories,
        &graph_nodes,
        task_input.token_budget,
    );
    if subgraph_recommendation.recommendation != KgCatalogRouterSubgraphRecommendationKind::None {
        push_warning(&mut warnings, "subgraph_delegation_recommendation_only");
    }

    let selected_catalog_route = KgSelectedCatalogRoute {
        selected_path: selected_path.clone(),
        selected_route_reason: if selected_path.is_empty() {
            "No eligible catalog route could be selected from scoped persisted rows.".to_owned()
        } else {
            "Highest deterministic repository/test route score.".to_owned()
        },
        route_confidence_bp: catalog_path_candidates
            .first()
            .map_or(0, |candidate| candidate.route_score_bp),
        task_fit_notes: selected_route_notes(&selected_path, &selected_memory_refs),
        token_budget: task_input.token_budget,
        subgraph_delegation_recommendation: subgraph_recommendation.recommendation,
    };

    let packet_metrics = KgCatalogRouterPacketMetrics {
        token_budget: task_input.token_budget,
        token_estimate: selected_token_estimate,
        citation_coverage_bp: coverage_bp(
            selected_memory_refs
                .iter()
                .filter(|memory| {
                    memories
                        .get(&memory.memory_id)
                        .is_some_and(|stored| receipt_hashes.contains(&stored.latest_receipt_hash))
                })
                .count(),
            selected_memory_refs.len(),
        )?,
        validation_coverage_bp: coverage_bp(
            selected_memory_refs
                .iter()
                .filter(|memory| {
                    validation_reports
                        .get(&memory.memory_id)
                        .is_some_and(|reports| !reports.is_empty())
                })
                .count(),
            selected_memory_refs.len(),
        )?,
        selected_ref_count: usize_to_u32("selected_ref_count", selected_memory_refs.len())?,
        omitted_ref_count: usize_to_u32("omitted_ref_count", omitted_refs.len())?,
        selected_edge_count: usize_to_u32("selected_edge_count", selected_graph_edges.len())?,
        warning_count: usize_to_u32("warning_count", warnings.len())?,
        boundary_warning_count: usize_to_u32(
            "boundary_warning_count",
            warnings
                .iter()
                .filter(|warning| is_boundary_warning(warning))
                .count(),
        )?,
    };

    Ok(KgCatalogRouterPreview {
        schema_version: KG_CATALOG_ROUTER_PREVIEW_SCHEMA.to_owned(),
        task_input: task_input.clone(),
        catalog_path_candidates,
        selected_catalog_route,
        selected_memory_refs,
        selected_graph_edges,
        omitted_refs,
        packet_metrics,
        warnings,
        subgraph_delegation_recommendation: subgraph_recommendation,
        boundaries: KgCatalogRouterBoundaries::repository_test_closed(),
    })
}

async fn load_memories(
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<BTreeMap<String, RetrievedMemory>> {
    let rows = sqlx::query(
        "SELECT memory_id, title, summary, risk_class, status, validation_status, \
                dag_finality_status, latest_receipt_hash \
         FROM dagdb_memory_objects \
         WHERE tenant_id = $1 AND namespace = $2 \
         ORDER BY memory_id",
    )
    .bind(&task_input.tenant_id)
    .bind(&task_input.namespace)
    .fetch_all(pool)
    .await
    .map_err(pg)?;

    let mut memories = BTreeMap::new();
    for row in rows {
        let memory_id = hex_from_hash_column("memory_id", row.try_get("memory_id").map_err(pg)?)?;
        let title = safe_metadata_from_value("memory.title", row.try_get("title").map_err(pg)?)?;
        let summary =
            safe_metadata_from_value("memory.summary", row.try_get("summary").map_err(pg)?)?;
        let latest_receipt_hash = hex_from_hash_column(
            "latest_receipt_hash",
            row.try_get("latest_receipt_hash").map_err(pg)?,
        )?;
        memories.insert(
            memory_id,
            RetrievedMemory {
                title,
                summary,
                risk_class: row.try_get("risk_class").map_err(pg)?,
                status: row.try_get("status").map_err(pg)?,
                validation_status: row.try_get("validation_status").map_err(pg)?,
                dag_finality_status: row.try_get("dag_finality_status").map_err(pg)?,
                latest_receipt_hash,
            },
        );
    }
    Ok(memories)
}

async fn load_catalogs(
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<BTreeMap<String, RetrievedCatalog>> {
    let rows = sqlx::query(
        "SELECT catalog_id, memory_id \
         FROM dagdb_catalog_entries \
         WHERE tenant_id = $1 AND namespace = $2 AND memory_id IS NOT NULL \
         ORDER BY memory_id, catalog_id",
    )
    .bind(&task_input.tenant_id)
    .bind(&task_input.namespace)
    .fetch_all(pool)
    .await
    .map_err(pg)?;

    let mut catalogs = BTreeMap::new();
    for row in rows {
        let memory_id =
            hex_from_hash_column("catalog.memory_id", row.try_get("memory_id").map_err(pg)?)?;
        let catalog_id =
            hex_from_hash_column("catalog_id", row.try_get("catalog_id").map_err(pg)?)?;
        catalogs
            .entry(memory_id)
            .or_insert(RetrievedCatalog { catalog_id });
    }
    Ok(catalogs)
}

async fn load_graph_nodes(
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<BTreeMap<String, Vec<RetrievedGraphNode>>> {
    let rows = sqlx::query(
        "SELECT graph_node_id, memory_id, catalog_path \
         FROM dagdb_graph_nodes node \
         WHERE node.tenant_id = $1 AND node.namespace = $2 \
         ORDER BY memory_id, catalog_path, graph_node_id",
    )
    .bind(&task_input.tenant_id)
    .bind(&task_input.namespace)
    .fetch_all(pool)
    .await
    .map_err(pg)?;

    let mut graph_nodes: BTreeMap<String, Vec<RetrievedGraphNode>> = BTreeMap::new();
    for row in rows {
        let memory_id = hex_from_hash_column(
            "graph_node.memory_id",
            row.try_get("memory_id").map_err(pg)?,
        )?;
        let graph_node_id =
            hex_from_hash_column("graph_node_id", row.try_get("graph_node_id").map_err(pg)?)?;
        let catalog_path = row
            .try_get::<Option<String>, _>("catalog_path")
            .map_err(pg)?
            .unwrap_or_else(|| "uncataloged".to_owned());
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
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<Vec<RetrievedGraphEdge>> {
    let rows = sqlx::query(
        "SELECT graph_edge_id, graph_style, from_memory_id, to_memory_id, edge_kind \
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
    .bind(&task_input.tenant_id)
    .bind(&task_input.namespace)
    .fetch_all(pool)
    .await
    .map_err(pg)?;

    let mut edges = Vec::new();
    for row in rows {
        let edge_kind: String = row.try_get("edge_kind").map_err(pg)?;
        let graph_style: String = row.try_get("graph_style").map_err(pg)?;
        ensure_safe_text("graph_edge.edge_kind", &edge_kind)?;
        ensure_safe_text("graph_edge.graph_style", &graph_style)?;
        edges.push(RetrievedGraphEdge {
            edge_id: hex_from_hash_column(
                "graph_edge_id",
                row.try_get("graph_edge_id").map_err(pg)?,
            )?,
            from_memory_id: hex_from_hash_column(
                "edge.from_memory_id",
                row.try_get("from_memory_id").map_err(pg)?,
            )?,
            to_memory_id: hex_from_hash_column(
                "edge.to_memory_id",
                row.try_get("to_memory_id").map_err(pg)?,
            )?,
            edge_kind,
            graph_style,
        });
    }
    Ok(edges)
}

async fn load_validation_reports(
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<BTreeMap<String, Vec<String>>> {
    let rows = sqlx::query(
        "SELECT validation_report_id, subject_id \
         FROM dagdb_validation_reports \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = 'memory' \
         ORDER BY subject_id, validation_report_id",
    )
    .bind(&task_input.tenant_id)
    .bind(&task_input.namespace)
    .fetch_all(pool)
    .await
    .map_err(pg)?;

    let mut reports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let memory_id = hex_from_hash_column(
            "validation.subject_id",
            row.try_get("subject_id").map_err(pg)?,
        )?;
        let report_id = hex_from_hash_column(
            "validation_report_id",
            row.try_get("validation_report_id").map_err(pg)?,
        )?;
        reports.entry(memory_id).or_default().push(report_id);
    }
    Ok(reports)
}

async fn load_receipt_hashes(
    pool: &PgPool,
    task_input: &KgCatalogRouterTaskInput,
) -> Result<BTreeSet<String>> {
    let rows = sqlx::query(
        "SELECT receipt_hash \
         FROM dagdb_receipts \
         WHERE tenant_id = $1 AND namespace = $2 \
         ORDER BY receipt_hash",
    )
    .bind(&task_input.tenant_id)
    .bind(&task_input.namespace)
    .fetch_all(pool)
    .await
    .map_err(pg)?;

    let mut receipt_hashes = BTreeSet::new();
    for row in rows {
        receipt_hashes.insert(hex_from_hash_column(
            "receipt_hash",
            row.try_get("receipt_hash").map_err(pg)?,
        )?);
    }
    Ok(receipt_hashes)
}

struct CandidateBuildContext<'a> {
    task_input: &'a KgCatalogRouterTaskInput,
    memories: &'a BTreeMap<String, RetrievedMemory>,
    validation_reports: &'a BTreeMap<String, Vec<String>>,
    receipt_hashes: &'a BTreeSet<String>,
    task_terms: &'a BTreeSet<String>,
    total_eligible_count: usize,
}

fn build_candidate(
    context: &CandidateBuildContext<'_>,
    catalog_path: &str,
    memory_ids: &BTreeSet<String>,
) -> Result<KgCatalogPathCandidate> {
    let mut matched_terms = BTreeSet::new();
    let mut candidate_text = catalog_path.to_ascii_lowercase();
    let mut total_tokens = 0u32;
    let mut validation_count = 0usize;
    let mut receipt_count = 0usize;
    for memory_id in memory_ids {
        if let Some(memory) = context.memories.get(memory_id) {
            candidate_text.push(' ');
            candidate_text.push_str(&memory.title.text.to_ascii_lowercase());
            candidate_text.push(' ');
            candidate_text.push_str(&memory.summary.text.to_ascii_lowercase());
            total_tokens =
                total_tokens.saturating_add(memory_token_estimate(&memory.title, &memory.summary));
            if context
                .validation_reports
                .get(memory_id)
                .is_some_and(|reports| !reports.is_empty())
            {
                validation_count = validation_count.saturating_add(1);
            }
            if context.receipt_hashes.contains(&memory.latest_receipt_hash) {
                receipt_count = receipt_count.saturating_add(1);
            }
        }
    }
    for term in context.task_terms {
        if candidate_text.contains(term) {
            matched_terms.insert(term.clone());
        }
    }

    let hint_match = context.task_input.catalog_hints.iter().any(|hint| {
        let hint = hint.to_ascii_lowercase();
        let path = catalog_path.to_ascii_lowercase();
        path.contains(&hint) || hint.contains(&path)
    });
    let requested_match_count = context
        .task_input
        .requested_memory_refs
        .iter()
        .filter(|memory_id| memory_ids.contains(*memory_id))
        .count();
    let task_term_match_bp = coverage_bp(matched_terms.len(), context.task_terms.len())?;
    let requested_match_bp = coverage_bp(
        requested_match_count,
        context.task_input.requested_memory_refs.len(),
    )?;
    let validation_bp = coverage_bp(validation_count, memory_ids.len())?;
    let receipt_bp = coverage_bp(receipt_count, memory_ids.len())?;
    let density_bp = coverage_bp(memory_ids.len(), context.total_eligible_count)?;

    let route_score_basis = vec![
        KgCatalogRouterScoreComponent {
            reason: "catalog_hint_match".to_owned(),
            score_bp: if hint_match {
                CATALOG_HINT_WEIGHT_BP
            } else {
                0
            },
        },
        KgCatalogRouterScoreComponent {
            reason: "task_term_match".to_owned(),
            score_bp: weighted_bp(task_term_match_bp, TASK_TERM_WEIGHT_BP),
        },
        KgCatalogRouterScoreComponent {
            reason: "requested_memory_match".to_owned(),
            score_bp: weighted_bp(requested_match_bp, REQUESTED_MEMORY_WEIGHT_BP),
        },
        KgCatalogRouterScoreComponent {
            reason: "validation_coverage".to_owned(),
            score_bp: weighted_bp(validation_bp, VALIDATION_WEIGHT_BP),
        },
        KgCatalogRouterScoreComponent {
            reason: "receipt_citation_coverage".to_owned(),
            score_bp: weighted_bp(receipt_bp, RECEIPT_WEIGHT_BP),
        },
        KgCatalogRouterScoreComponent {
            reason: "eligible_memory_density".to_owned(),
            score_bp: weighted_bp(density_bp, MEMORY_DENSITY_WEIGHT_BP),
        },
    ];
    let route_score_bp = route_score_basis
        .iter()
        .fold(0u32, |sum, component| {
            sum.saturating_add(component.score_bp)
        })
        .min(MAX_BP);
    let mut warning_count = 0u32;
    if total_tokens > context.task_input.token_budget {
        warning_count = warning_count.saturating_add(1);
    }
    if validation_count == 0 {
        warning_count = warning_count.saturating_add(1);
    }
    if receipt_count == 0 {
        warning_count = warning_count.saturating_add(1);
    }

    Ok(KgCatalogPathCandidate {
        catalog_path: catalog_path.to_owned(),
        matched_terms,
        source_signals: candidate_source_signals(memory_ids.len(), total_tokens, hint_match),
        route_score_basis,
        route_score_bp,
        eligible_memory_count: usize_to_u32("eligible_memory_count", memory_ids.len())?,
        warning_count,
        reason: "Deterministic repository/test score from persisted catalog rows.".to_owned(),
    })
}

fn candidate_memory_ids_by_path(
    memories: &BTreeMap<String, RetrievedMemory>,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut candidates = BTreeMap::<String, BTreeSet<String>>::new();
    for (memory_id, memory) in memories {
        if !memory_status_allowed(&memory.status)
            || validation_status_blocked(&memory.validation_status)
        {
            continue;
        }
        let paths = catalog_paths_for_memory(memory_id, graph_nodes);
        for path in paths {
            candidates
                .entry(path)
                .or_default()
                .insert(memory_id.clone());
        }
    }
    candidates
}

fn selected_graph_edges(
    graph_edges: &[RetrievedGraphEdge],
    selected_ids: &BTreeSet<String>,
) -> Vec<KgCatalogRouterGraphEdgeRef> {
    graph_edges
        .iter()
        .filter(|edge| {
            selected_ids.contains(&edge.from_memory_id) && selected_ids.contains(&edge.to_memory_id)
        })
        .map(|edge| KgCatalogRouterGraphEdgeRef {
            edge_id: edge.edge_id.clone(),
            from_memory_id: edge.from_memory_id.clone(),
            to_memory_id: edge.to_memory_id.clone(),
            edge_kind: edge.edge_kind.clone(),
            graph_style: edge.graph_style.clone(),
            action_classification: action_classification(&edge.edge_kind),
            reason_included: "selected_endpoints_share_catalog_route".to_owned(),
        })
        .collect()
}

fn build_omitted_refs(
    omitted_reasons: &BTreeMap<String, String>,
    memories: &BTreeMap<String, RetrievedMemory>,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
    selected_path: &str,
) -> Vec<KgCatalogRouterOmittedRef> {
    omitted_reasons
        .iter()
        .map(|(memory_id, reason)| {
            if let Some(memory) = memories.get(memory_id) {
                KgCatalogRouterOmittedRef {
                    memory_id: memory_id.clone(),
                    catalog_path: first_catalog_path(memory_id, graph_nodes, selected_path),
                    omission_reason: reason.clone(),
                    token_estimate_if_selected: Some(memory_token_estimate(
                        &memory.title,
                        &memory.summary,
                    )),
                    validation_status: memory.validation_status.clone(),
                    risk_or_boundary_status: memory.risk_class.clone(),
                    finality_status: memory.dag_finality_status.clone(),
                }
            } else {
                KgCatalogRouterOmittedRef {
                    memory_id: memory_id.clone(),
                    catalog_path: "unknown".to_owned(),
                    omission_reason: reason.clone(),
                    token_estimate_if_selected: None,
                    validation_status: "not_found".to_owned(),
                    risk_or_boundary_status: "unknown".to_owned(),
                    finality_status: "repository_test_only".to_owned(),
                }
            }
        })
        .collect()
}

fn subgraph_recommendation(
    selected_path: &str,
    selected_path_memory_ids: &BTreeSet<String>,
    selected_graph_edges: &[KgCatalogRouterGraphEdgeRef],
    memories: &BTreeMap<String, RetrievedMemory>,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
    token_budget: u32,
) -> KgSubgraphDelegationRecommendation {
    let selected_total_tokens = selected_path_memory_ids
        .iter()
        .filter_map(|memory_id| memories.get(memory_id))
        .fold(0u32, |sum, memory| {
            sum.saturating_add(memory_token_estimate(&memory.title, &memory.summary))
        });
    let mut degree_by_memory = BTreeMap::<String, u32>::new();
    for edge in selected_graph_edges {
        degree_by_memory
            .entry(edge.from_memory_id.clone())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
        degree_by_memory
            .entry(edge.to_memory_id.clone())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
    }
    let recommendation = if !selected_path.is_empty() && selected_total_tokens > token_budget {
        (
            KgCatalogRouterSubgraphRecommendationKind::DelegateCatalogBranch,
            "catalog_cluster_exceeds_token_budget",
        )
    } else if degree_by_memory.values().any(|count| *count > 12) {
        (
            KgCatalogRouterSubgraphRecommendationKind::DelegateSubgraphReview,
            "high_degree_node",
        )
    } else if selected_path_memory_ids.len() > 12 {
        (
            KgCatalogRouterSubgraphRecommendationKind::DelegateCatalogBranch,
            "high_degree_catalog_cluster",
        )
    } else {
        (
            KgCatalogRouterSubgraphRecommendationKind::None,
            "subgraph_delegation_not_recommended",
        )
    };

    KgSubgraphDelegationRecommendation {
        recommendation: recommendation.0,
        reason: recommendation.1.to_owned(),
        suggested_catalog_path: if selected_path.is_empty() {
            None
        } else {
            Some(selected_path.to_owned())
        },
        suggested_memory_refs: selected_path_memory_ids
            .iter()
            .filter(|memory_id| memories.contains_key(*memory_id))
            .filter(|memory_id| !catalog_paths_for_memory(memory_id, graph_nodes).is_empty())
            .cloned()
            .collect(),
    }
}

fn selected_route_notes(
    selected_path: &str,
    selected_memory_refs: &[KgCatalogRouterMemoryRef],
) -> Vec<String> {
    if selected_path.is_empty() {
        return vec!["No scoped catalog route had eligible persisted memory.".to_owned()];
    }
    vec![
        "Read-only persisted-row diagnostics; no route activation.".to_owned(),
        format!("selected_catalog_path:{selected_path}"),
        format!("selected_memory_count:{}", selected_memory_refs.len()),
    ]
}

fn safe_metadata_from_value(field: &str, value: JsonValue) -> Result<SafeMetadata> {
    let metadata: SafeMetadata =
        serde_json::from_value(value).map_err(|error| KgCatalogRouterPostgresError::Json {
            reason: error.to_string(),
        })?;
    if metadata.decision == SafeMetadataDecision::Reject {
        return Err(KgCatalogRouterPostgresError::UnsafeMetadata {
            field: field.to_owned(),
            reason: "safe metadata decision is reject".to_owned(),
        });
    }
    ensure_safe_text(field, &metadata.text)?;
    Ok(metadata)
}

fn ensure_safe_text(field: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(KgCatalogRouterPostgresError::UnsafeMetadata {
            field: field.to_owned(),
            reason: "text is empty".to_owned(),
        });
    }
    if let Some(fragment) = FORBIDDEN_TEXT_FRAGMENTS
        .iter()
        .find(|fragment| value.contains(**fragment))
    {
        return Err(KgCatalogRouterPostgresError::UnsafeMetadata {
            field: field.to_owned(),
            reason: format!("contains forbidden fragment {fragment}"),
        });
    }
    if is_probable_local_absolute_path(value) {
        return Err(KgCatalogRouterPostgresError::UnsafeMetadata {
            field: field.to_owned(),
            reason: "contains probable local absolute path".to_owned(),
        });
    }
    Ok(())
}

fn is_probable_local_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("~/")
        || (value.len() > 2
            && value.as_bytes()[1] == b':'
            && (value.as_bytes()[2] == b'\\' || value.as_bytes()[2] == b'/'))
}

fn action_classification(edge_kind: &str) -> KgCatalogRouterEdgeActionClassification {
    match edge_kind {
        "supports"
        | "depends_on"
        | "part_of"
        | "related_to"
        | "used_by_route"
        | "included_in_context_packet" => KgCatalogRouterEdgeActionClassification::KeepActive,
        "contradicts" => KgCatalogRouterEdgeActionClassification::ContradictionEdge,
        "supersedes" | "replaces" | "revoked_by" => {
            KgCatalogRouterEdgeActionClassification::SupersessionEdge
        }
        "duplicate_of" | "near_duplicate_of" => {
            KgCatalogRouterEdgeActionClassification::DuplicateEdge
        }
        "alternative_summary_of" => KgCatalogRouterEdgeActionClassification::DemoteAdvisory,
        "derived_from" | "summarizes" | "owned_by" | "access_granted_by" | "verified_by" => {
            KgCatalogRouterEdgeActionClassification::ProvenanceOnly
        }
        _ => KgCatalogRouterEdgeActionClassification::NeedsReview,
    }
}

fn memory_status_allowed(status: &str) -> bool {
    matches!(status, "pending" | "approved" | "routable")
}

fn validation_status_blocked(validation_status: &str) -> bool {
    matches!(validation_status, "failed" | "contradictory" | "expired")
}

fn catalog_paths_for_memory(
    memory_id: &str,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
) -> BTreeSet<String> {
    let mut paths = graph_nodes
        .get(memory_id)
        .into_iter()
        .flat_map(|nodes| nodes.iter())
        .map(|node| node.catalog_path.clone())
        .filter(|path| !path.is_empty())
        .collect::<BTreeSet<_>>();
    if paths.is_empty() {
        paths.insert("uncataloged".to_owned());
    }
    paths
}

fn first_catalog_path(
    memory_id: &str,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
    selected_path: &str,
) -> String {
    catalog_paths_for_memory(memory_id, graph_nodes)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            if selected_path.is_empty() {
                "unknown".to_owned()
            } else {
                selected_path.to_owned()
            }
        })
}

fn memory_has_catalog_path(
    memory_id: &str,
    catalog_path: &str,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
) -> bool {
    catalog_paths_for_memory(memory_id, graph_nodes).contains(catalog_path)
}

fn graph_node_ids(
    memory_id: &str,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
) -> BTreeSet<String> {
    graph_nodes
        .get(memory_id)
        .into_iter()
        .flat_map(|nodes| nodes.iter())
        .map(|node| node.graph_node_id.clone())
        .collect()
}

fn memory_sort_key(
    memory_id: &str,
    memory: Option<&RetrievedMemory>,
    graph_nodes: &BTreeMap<String, Vec<RetrievedGraphNode>>,
) -> (String, u32, String) {
    let token_estimate = memory.map_or(0, |memory| {
        memory_token_estimate(&memory.title, &memory.summary)
    });
    (
        first_catalog_path(memory_id, graph_nodes, "unknown"),
        token_estimate,
        memory_id.to_owned(),
    )
}

fn task_terms(task_description: &str) -> BTreeSet<String> {
    task_description
        .split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() >= 3)
        .collect()
}

fn candidate_source_signals(
    memory_count: usize,
    total_tokens: u32,
    hint_match: bool,
) -> BTreeSet<String> {
    let mut signals = BTreeSet::from([
        "persisted_memory_rows".to_owned(),
        "persisted_graph_nodes".to_owned(),
    ]);
    if memory_count > 0 {
        signals.insert("eligible_memory_rows".to_owned());
    }
    if total_tokens > 0 {
        signals.insert("token_estimates_available".to_owned());
    }
    if hint_match {
        signals.insert("catalog_hint_match".to_owned());
    }
    signals
}

fn safe_catalog_path(value: &str) -> String {
    value
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect::<Vec<_>>()
        .join("/")
}

fn base_boundary_warnings() -> Vec<String> {
    vec![
        "production_finality_not_approved".to_owned(),
        "gateway_api_not_approved".to_owned(),
        "route_activation_not_approved".to_owned(),
        "route_invalidation_writes_not_approved".to_owned(),
        "graph_explorer_production_not_approved".to_owned(),
        "raw_artifact_persistence_not_approved".to_owned(),
        "direct_exo_dag_writes_not_approved".to_owned(),
        "exo_dag_table_mutation_not_approved".to_owned(),
        "migrations_not_approved".to_owned(),
        "sqlite_nsqlite_direct_import_deferred".to_owned(),
    ]
}

fn push_warning(warnings: &mut Vec<String>, warning: impl Into<String>) {
    let warning = warning.into();
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn is_boundary_warning(value: &str) -> bool {
    value.contains("not_approved") || value.contains("unapproved") || value.contains("deferred")
}

fn coverage_bp(numerator: usize, denominator: usize) -> Result<u32> {
    if denominator == 0 {
        return Ok(0);
    }
    let numerator =
        u64::try_from(numerator).map_err(|_| KgCatalogRouterPostgresError::CountOutOfRange {
            field: "coverage_bp_numerator".to_owned(),
        })?;
    let denominator =
        u64::try_from(denominator).map_err(|_| KgCatalogRouterPostgresError::CountOutOfRange {
            field: "coverage_bp_denominator".to_owned(),
        })?;
    let scaled = numerator.saturating_mul(u64::from(MAX_BP)) / denominator;
    u32::try_from(scaled).map_err(|_| KgCatalogRouterPostgresError::CountOutOfRange {
        field: "coverage_bp".to_owned(),
    })
}

fn weighted_bp(score_bp: u32, weight_bp: u32) -> u32 {
    score_bp.saturating_mul(weight_bp) / MAX_BP
}

fn usize_to_u32(field: &str, value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| KgCatalogRouterPostgresError::CountOutOfRange {
        field: field.to_owned(),
    })
}

fn sort_candidates(candidates: &mut [KgCatalogPathCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .route_score_bp
            .cmp(&left.route_score_bp)
            .then(left.warning_count.cmp(&right.warning_count))
            .then(left.catalog_path.cmp(&right.catalog_path))
    });
}

fn sort_selected_refs(memory_refs: &mut [KgCatalogRouterMemoryRef]) {
    memory_refs.sort_by(|left, right| {
        left.catalog_path
            .cmp(&right.catalog_path)
            .then(left.token_estimate.cmp(&right.token_estimate))
            .then(left.memory_id.cmp(&right.memory_id))
    });
}

fn sort_edges(edges: &mut [KgCatalogRouterGraphEdgeRef]) {
    edges.sort_by(|left, right| {
        left.action_classification
            .cmp(&right.action_classification)
            .then(left.edge_kind.cmp(&right.edge_kind))
            .then(left.from_memory_id.cmp(&right.from_memory_id))
            .then(left.to_memory_id.cmp(&right.to_memory_id))
            .then(left.edge_id.cmp(&right.edge_id))
    });
}

fn sort_omitted_refs(omitted_refs: &mut [KgCatalogRouterOmittedRef]) {
    omitted_refs.sort_by(|left, right| {
        left.omission_reason
            .cmp(&right.omission_reason)
            .then(left.catalog_path.cmp(&right.catalog_path))
            .then(left.memory_id.cmp(&right.memory_id))
    });
}

fn pg(source: sqlx::Error) -> KgCatalogRouterPostgresError {
    KgCatalogRouterPostgresError::Postgres { source }
}

#[derive(Debug, Clone)]
struct RetrievedMemory {
    title: SafeMetadata,
    summary: SafeMetadata,
    risk_class: String,
    status: String,
    validation_status: String,
    dag_finality_status: String,
    latest_receipt_hash: String,
}

#[derive(Debug, Clone)]
struct RetrievedCatalog {
    catalog_id: String,
}

#[derive(Debug, Clone)]
struct RetrievedGraphNode {
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
}
