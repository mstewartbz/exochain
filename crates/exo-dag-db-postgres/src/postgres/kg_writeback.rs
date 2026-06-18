//! Feature-gated persisted KG writeback repository adapter.
//!
//! This adapter consumes validated dry-run writeback proposal reports as review
//! artifacts, verifies evidence against persisted DAG DB rows, and writes only
//! current-schema-supported rows. It does not expose gateway behavior, export
//! persistence, graph explorer behavior, production route activation, migrations,
//! or `exo-dag` table writes.

use std::collections::BTreeSet;

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    CanonicalizationDecisionKind, MemoryEdgeKind, ReceiptEventType, RiskClass, SafeMetadata,
    SafeMetadataDecision, SimilarityType, SubjectKind, ValidationStatus,
};
use serde::Serialize;
use serde_json::{Value as JsonValue, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;

use crate::{
    hash::{ReceiptHashMaterial, RequestHashMaterial},
    kg_import::{KgImportError, hash_from_hex, stable_hash},
    kg_writeback::{
        KG_WRITEBACK_DATABASE_URL_ENV, KG_WRITEBACK_PERSISTED_ROUTE_NAME,
        KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA, KgWritebackAdvisoryDeferredDiagnostics,
        KgWritebackDryRunReport, KgWritebackError, KgWritebackPersistedDiagnostics,
        KgWritebackPersistedEvidenceDiagnostics, KgWritebackPersistedIdempotencyDiagnostics,
        KgWritebackPersistedLayerDiagnostics, KgWritebackPersistedPlacementDiagnostics,
        KgWritebackPersistedRowCounts, KgWritebackPersistedSummary,
        KgWritebackPersistedValidationDiagnostics, KgWritebackSkippedSection,
    },
    scoring::hash_event_body,
};

const CREATED_AT: Timestamp = Timestamp::new(1, 0);
const EXPIRES_AT: Timestamp = Timestamp::new(86_400_001, 0);

/// Errors raised by the feature-gated persisted KG writeback adapter.
#[derive(Debug, Error)]
pub enum KgWritebackPersistenceError {
    /// No database URL was supplied for persisted mode.
    #[error("kg_writeback_database_url_missing: {env_var}")]
    MissingDatabaseUrl {
        /// Required env var.
        env_var: &'static str,
    },
    /// Dry-run writeback report validation failed.
    #[error(transparent)]
    Report(#[from] KgWritebackError),
    /// Shared hash validation failed.
    #[error(transparent)]
    ImportHash(#[from] KgImportError),
    /// Postgres foundation failed.
    #[error("kg_writeback_postgres_init_failed")]
    Init {
        /// Source Postgres foundation error.
        #[source]
        source: super::DagDbPostgresError,
    },
    /// SQL operation failed.
    #[error("kg_writeback_postgres_failed")]
    Postgres {
        /// Source SQLx error.
        #[source]
        source: sqlx::Error,
    },
    /// JSON conversion failed.
    #[error("kg_writeback_json_failed: {reason}")]
    Json {
        /// Stable conversion reason.
        reason: String,
    },
    /// Existing row, idempotency key, or evidence conflicts with the report.
    #[error("kg_writeback_conflict: {reason}")]
    Conflict {
        /// Stable conflict reason.
        reason: String,
    },
    /// The current schema cannot support the requested persisted write.
    #[error("kg_writeback_unsupported_persisted_section: {section}")]
    UnsupportedSection {
        /// Dry-run section or write target.
        section: String,
    },
    /// Timestamp value cannot be stored.
    #[error("kg_writeback_timestamp_out_of_range")]
    TimestampOutOfRange,
    /// Count cannot fit response field.
    #[error("kg_writeback_count_out_of_range")]
    CountOutOfRange,
    /// Hashing failed.
    #[error("kg_writeback_hash_failed: {reason}")]
    Hash {
        /// Stable hash reason.
        reason: String,
    },
}

/// Result alias for persisted KG writeback.
pub type Result<T> = std::result::Result<T, KgWritebackPersistenceError>;

/// Persist a dry-run KG writeback report using `EXO_DAGDB_TEST_DATABASE_URL`.
pub async fn persist_kg_writeback_report_from_env(
    report_json: &str,
) -> Result<KgWritebackPersistedSummary> {
    let database_url = std::env::var(KG_WRITEBACK_DATABASE_URL_ENV).map_err(|_| {
        KgWritebackPersistenceError::MissingDatabaseUrl {
            env_var: KG_WRITEBACK_DATABASE_URL_ENV,
        }
    })?;
    persist_kg_writeback_report_from_database_url(Some(database_url.as_str()), report_json).await
}

/// Persist a dry-run KG writeback report using an explicit database URL.
pub async fn persist_kg_writeback_report_from_database_url(
    database_url: Option<&str>,
    report_json: &str,
) -> Result<KgWritebackPersistedSummary> {
    let Some(database_url) = database_url else {
        return Err(KgWritebackPersistenceError::MissingDatabaseUrl {
            env_var: KG_WRITEBACK_DATABASE_URL_ENV,
        });
    };
    let pool = super::init_pool(database_url)
        .await
        .map_err(|source| KgWritebackPersistenceError::Init { source })?;
    let result = persist_kg_writeback_report(&pool, report_json).await;
    pool.close().await;
    result
}

/// Persist supported sections from a validated dry-run KG writeback report.
pub async fn persist_kg_writeback_report(
    pool: &PgPool,
    report_json: &str,
) -> Result<KgWritebackPersistedSummary> {
    let report = KgWritebackDryRunReport::parse_json(report_json)?;
    let idempotency_key = report.idempotency_key()?;
    let request_hash = RequestHashMaterial {
        route_name: KG_WRITEBACK_PERSISTED_ROUTE_NAME.to_owned(),
        tenant_id: report.tenant_id.clone(),
        namespace: report.namespace.clone(),
        canonical_redacted_request_body: report_json.as_bytes().to_vec(),
    }
    .hash()
    .map_err(|error| KgWritebackPersistenceError::Hash {
        reason: error.to_string(),
    })?;

    let mut tx = pool.begin().await.map_err(pg)?;
    let result = persist_kg_writeback_report_in_transaction(
        &mut tx,
        &report,
        &idempotency_key,
        request_hash,
    )
    .await;
    match result {
        Ok(summary) => {
            tx.commit().await.map_err(pg)?;
            Ok(summary)
        }
        Err(error) => {
            if let Err(rollback_error) = tx.rollback().await {
                tracing::warn!(
                    operation = "persist_kg_writeback_report",
                    tenant_id = %report.tenant_id,
                    namespace = %report.namespace,
                    candidate_id = %report.candidate_id,
                    error = %rollback_error,
                    "failed to rollback transaction after KG writeback persistence error"
                );
            }
            Err(error)
        }
    }
}

async fn persist_kg_writeback_report_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    idempotency_key: &str,
    request_hash: Hash256,
) -> Result<KgWritebackPersistedSummary> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(pg)?;

    if let Some(summary) =
        fetch_idempotency_replay(tx, report, idempotency_key, request_hash).await?
    {
        return Ok(summary);
    }

    verify_evidence_memories(tx, report).await?;

    let mut counts = PersistedCounts::default();
    let memory_receipt = insert_receipt(
        tx,
        report,
        SubjectKind::Memory,
        &report.candidate_id,
        ReceiptEventType::WritebackCreated,
        "writeback_created",
    )
    .await?;
    counts.inserted_receipt_count = counts
        .inserted_receipt_count
        .saturating_add(memory_receipt.inserted_receipt_count);
    counts.inserted_subject_receipt_head_count = counts
        .inserted_subject_receipt_head_count
        .saturating_add(memory_receipt.inserted_subject_receipt_head_count);
    counts.inserted_memory_count = counts
        .inserted_memory_count
        .saturating_add(insert_memory(tx, report, memory_receipt.receipt_hash).await?);
    counts.inserted_catalog_count = counts
        .inserted_catalog_count
        .saturating_add(insert_catalog(tx, report, memory_receipt.receipt_hash).await?);
    counts.inserted_graph_node_count = counts
        .inserted_graph_node_count
        .saturating_add(insert_graph_node(tx, report).await?);
    let layer_counts = insert_layered_writeback(tx, report, memory_receipt.receipt_hash).await?;
    counts.inserted_layer_count = counts
        .inserted_layer_count
        .saturating_add(layer_counts.inserted_layer_count);
    counts.inserted_layer_membership_count = counts
        .inserted_layer_membership_count
        .saturating_add(layer_counts.inserted_layer_membership_count);
    counts.inserted_layer_edge_count = counts
        .inserted_layer_edge_count
        .saturating_add(layer_counts.inserted_layer_edge_count);
    counts.layer_receipt_hash = layer_counts.layer_receipt_hash;

    for parent_memory_id in &report.evidence_binding.selected_memory_ids {
        counts.inserted_graph_edge_count = counts.inserted_graph_edge_count.saturating_add(
            insert_parent_context_graph_edge(
                tx,
                report,
                parent_memory_id,
                memory_receipt.receipt_hash,
            )
            .await?,
        );
        counts.inserted_memory_edge_count = counts
            .inserted_memory_edge_count
            .saturating_add(insert_parent_context_memory_edge(tx, report, parent_memory_id).await?);
    }

    for edge in &report.placement_proposal.edges_to_create {
        counts.inserted_graph_edge_count = counts.inserted_graph_edge_count.saturating_add(
            insert_graph_edge(tx, report, edge, memory_receipt.receipt_hash).await?,
        );
    }

    for similarity in &report.placement_proposal.similarity_results {
        counts.inserted_similarity_result_count = counts
            .inserted_similarity_result_count
            .saturating_add(insert_similarity_result(tx, report, similarity).await?);
    }

    counts.inserted_placement_decision_count =
        counts.inserted_placement_decision_count.saturating_add(
            insert_canonicalization_decision(tx, report, memory_receipt.receipt_hash).await?,
        );
    counts.inserted_placement_trace_count = counts
        .inserted_placement_trace_count
        .saturating_add(insert_placement_trace(tx, report).await?);

    let validation_receipt = insert_receipt(
        tx,
        report,
        SubjectKind::ValidationReport,
        &report.validation_proposal.validation_report_id,
        ReceiptEventType::ValidationCreated,
        "validation_created",
    )
    .await?;
    counts.inserted_receipt_count = counts
        .inserted_receipt_count
        .saturating_add(validation_receipt.inserted_receipt_count);
    counts.inserted_subject_receipt_head_count = counts
        .inserted_subject_receipt_head_count
        .saturating_add(validation_receipt.inserted_subject_receipt_head_count);
    counts.inserted_validation_report_count =
        counts.inserted_validation_report_count.saturating_add(
            insert_validation_report(tx, report, validation_receipt.receipt_hash).await?,
        );
    counts.inserted_idempotency_response_count = 1;

    let summary = persisted_summary(report, idempotency_key, request_hash, counts, false)?;
    insert_idempotency_response(tx, &summary, request_hash).await?;
    Ok(summary)
}

#[derive(Default)]
struct PersistedCounts {
    inserted_memory_count: u32,
    inserted_catalog_count: u32,
    inserted_graph_node_count: u32,
    inserted_graph_edge_count: u32,
    inserted_layer_count: u32,
    inserted_layer_membership_count: u32,
    inserted_layer_edge_count: u32,
    inserted_memory_edge_count: u32,
    inserted_similarity_result_count: u32,
    inserted_validation_report_count: u32,
    inserted_placement_decision_count: u32,
    inserted_placement_trace_count: u32,
    inserted_receipt_count: u32,
    inserted_subject_receipt_head_count: u32,
    inserted_idempotency_response_count: u32,
    layer_receipt_hash: Option<Hash256>,
}

#[derive(Default)]
struct PersistedLayerCounts {
    inserted_layer_count: u32,
    inserted_layer_membership_count: u32,
    inserted_layer_edge_count: u32,
    layer_receipt_hash: Option<Hash256>,
}

fn persisted_summary(
    report: &KgWritebackDryRunReport,
    idempotency_key: &str,
    request_hash: Hash256,
    counts: PersistedCounts,
    replayed: bool,
) -> Result<KgWritebackPersistedSummary> {
    let skipped_advisory_section_count = advisory_count(report)?;
    Ok(KgWritebackPersistedSummary {
        schema_version: KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA.to_owned(),
        tenant_id: report.tenant_id.clone(),
        namespace: report.namespace.clone(),
        proposal_id: report.proposal_id.clone(),
        candidate_id: report.candidate_id.clone(),
        idempotency_key: idempotency_key.to_owned(),
        replayed,
        inserted_memory_count: counts.inserted_memory_count,
        inserted_catalog_count: counts.inserted_catalog_count,
        inserted_graph_node_count: counts.inserted_graph_node_count,
        inserted_graph_edge_count: counts.inserted_graph_edge_count,
        inserted_layer_count: counts.inserted_layer_count,
        inserted_layer_membership_count: counts.inserted_layer_membership_count,
        inserted_layer_edge_count: counts.inserted_layer_edge_count,
        inserted_memory_edge_count: counts.inserted_memory_edge_count,
        inserted_similarity_result_count: counts.inserted_similarity_result_count,
        inserted_validation_report_count: counts.inserted_validation_report_count,
        inserted_placement_decision_count: counts.inserted_placement_decision_count,
        inserted_placement_trace_count: counts.inserted_placement_trace_count,
        inserted_receipt_count: counts.inserted_receipt_count,
        inserted_subject_receipt_head_count: counts.inserted_subject_receipt_head_count,
        inserted_idempotency_response_count: counts.inserted_idempotency_response_count,
        skipped_advisory_section_count,
        persisted_route_invalidation_count: 0,
        persisted_export_record_count: 0,
        preview_evidence_only: true,
        diagnostics: persisted_diagnostics(
            report,
            idempotency_key,
            request_hash,
            counts,
            skipped_advisory_section_count,
            replayed,
        )?,
    })
}

fn persisted_diagnostics(
    report: &KgWritebackDryRunReport,
    idempotency_key: &str,
    request_hash: Hash256,
    counts: PersistedCounts,
    skipped_advisory_section_count: u32,
    replayed: bool,
) -> Result<KgWritebackPersistedDiagnostics> {
    Ok(KgWritebackPersistedDiagnostics {
        persisted_row_counts: KgWritebackPersistedRowCounts {
            memory_rows: counts.inserted_memory_count,
            catalog_rows: counts.inserted_catalog_count,
            graph_node_rows: counts.inserted_graph_node_count,
            graph_edge_rows: counts.inserted_graph_edge_count,
            layer_rows: counts.inserted_layer_count,
            layer_membership_rows: counts.inserted_layer_membership_count,
            layer_edge_rows: counts.inserted_layer_edge_count,
            memory_edge_rows: counts.inserted_memory_edge_count,
            similarity_result_rows: counts.inserted_similarity_result_count,
            canonicalization_decision_rows: counts.inserted_placement_decision_count,
            placement_trace_rows: counts.inserted_placement_trace_count,
            validation_report_rows: counts.inserted_validation_report_count,
            receipt_rows: counts.inserted_receipt_count,
            subject_receipt_head_rows: counts.inserted_subject_receipt_head_count,
            idempotency_response_rows: counts.inserted_idempotency_response_count,
            route_invalidation_rows: 0,
            export_record_rows: 0,
        },
        advisory_deferred: advisory_deferred_diagnostics(report, skipped_advisory_section_count)?,
        evidence: evidence_diagnostics(report),
        placement_governance: placement_diagnostics(report)?,
        layered_writeback: layered_writeback_diagnostics(report, counts.layer_receipt_hash),
        validation_risk_council: validation_diagnostics(report)?,
        idempotency_replay: KgWritebackPersistedIdempotencyDiagnostics {
            idempotency_key: idempotency_key.to_owned(),
            replayed,
            request_hash: request_hash.to_string(),
            duplicate_writeback_detected: matches!(
                report
                    .placement_proposal
                    .canonicalization_decision
                    .decision_kind,
                CanonicalizationDecisionKind::ExactDuplicate
            ),
            replay_reason: if replayed {
                "idempotency_key_match"
            } else {
                "new_persisted_response"
            }
            .to_owned(),
        },
        warning_summaries: persisted_warning_summaries(report),
    })
}

fn advisory_deferred_diagnostics(
    report: &KgWritebackDryRunReport,
    skipped_advisory_section_count: u32,
) -> Result<KgWritebackAdvisoryDeferredDiagnostics> {
    let mut skipped_sections = Vec::new();
    for invalidation in &report.proposed_route_invalidations {
        skipped_sections.push(KgWritebackSkippedSection {
            section: "route_invalidation".to_owned(),
            status: "advisory".to_owned(),
            reason: invalidation.reason.clone(),
        });
    }
    if report.validation_proposal.needs_review {
        skipped_sections.push(KgWritebackSkippedSection {
            section: "governance_review_queue".to_owned(),
            status: "deferred".to_owned(),
            reason: "no dedicated governance review queue exists in the current schema".to_owned(),
        });
    }
    skipped_sections.push(KgWritebackSkippedSection {
        section: "memory_candidate_queue".to_owned(),
        status: "deferred".to_owned(),
        reason: "raw MemoryCandidate queue storage is out of scope for this adapter".to_owned(),
    });
    skipped_sections.push(KgWritebackSkippedSection {
        section: "export_record".to_owned(),
        status: "deferred".to_owned(),
        reason: "export persistence is out of scope for this phase".to_owned(),
    });
    skipped_sections.sort_by(|left, right| {
        left.section
            .cmp(&right.section)
            .then_with(|| left.status.cmp(&right.status))
            .then_with(|| left.reason.cmp(&right.reason))
    });
    Ok(KgWritebackAdvisoryDeferredDiagnostics {
        route_invalidation_proposals: usize_to_u32(report.proposed_route_invalidations.len())?,
        governance_review_items: if report.validation_proposal.needs_review {
            1
        } else {
            0
        },
        export_records: 0,
        memory_candidate_queue_records: 0,
        skipped_section_count: skipped_advisory_section_count,
        skipped_sections,
    })
}

fn evidence_diagnostics(
    report: &KgWritebackDryRunReport,
) -> KgWritebackPersistedEvidenceDiagnostics {
    let mut evidence_warnings = BTreeSet::new();
    evidence_warnings.insert("preview_only_context_evidence".to_owned());
    evidence_warnings.insert("route_hint_not_production_route".to_owned());
    if report
        .warnings
        .iter()
        .any(|warning| warning == "origin_path_not_persisted")
    {
        evidence_warnings.insert("origin_path_not_persisted".to_owned());
    }
    KgWritebackPersistedEvidenceDiagnostics {
        parent_context_packet_id: report.parent_context_packet_id.clone(),
        route_hint_id: report.route_hint_id.clone(),
        selected_memory_ids: report.evidence_binding.selected_memory_ids.clone(),
        citation_handles: report.evidence_binding.citation_handles.clone(),
        validation_report_ids: report.evidence_binding.validation_report_ids.clone(),
        receipt_hashes: report.evidence_binding.evidence_receipts.clone(),
        task_hash: report.task_hash.clone(),
        output_hash: report.output_hash.clone(),
        tenant_namespace_match: true,
        evidence_status: "preview_only".to_owned(),
        evidence_warnings: evidence_warnings.into_iter().collect(),
    }
}

fn placement_diagnostics(
    report: &KgWritebackDryRunReport,
) -> Result<KgWritebackPersistedPlacementDiagnostics> {
    let decision = &report.placement_proposal.canonicalization_decision;
    let graph_views_to_refresh = report
        .placement_proposal
        .graph_views_to_refresh
        .iter()
        .map(enum_sql)
        .collect::<Result<Vec<_>>>()?;
    Ok(KgWritebackPersistedPlacementDiagnostics {
        placement_decision_id: decision.decision_id.clone(),
        placement_decision_kind: decision_kind_sql(decision.decision_kind).to_owned(),
        placement_status: "persisted_review_rows".to_owned(),
        canonical_memory_id: decision.canonical_memory_id.clone(),
        matched_memory_ids: decision.matched_memory_ids.clone(),
        required_edges_to_create_count: usize_to_u32(decision.required_edges_to_create.len())?,
        graph_views_to_refresh,
        validator_report: report.placement_proposal.validator_report.clone(),
        needs_review: report.validation_proposal.needs_review,
        review_reasons: report.validation_proposal.review_reasons.clone(),
        route_invalidation_status: if report.proposed_route_invalidations.is_empty() {
            "not_applicable"
        } else {
            "advisory"
        }
        .to_owned(),
    })
}

fn layered_writeback_diagnostics(
    report: &KgWritebackDryRunReport,
    receipt_hash: Option<Hash256>,
) -> KgWritebackPersistedLayerDiagnostics {
    let Some(layered) = &report.layered_writeback else {
        return KgWritebackPersistedLayerDiagnostics {
            layered_writeback_status: "flat_only_no_layer_evidence".to_owned(),
            ..KgWritebackPersistedLayerDiagnostics::default()
        };
    };
    KgWritebackPersistedLayerDiagnostics {
        layered_writeback_status: "persisted_layer_rows".to_owned(),
        target_layer_id: Some(layered.target_layer_id.clone()),
        target_layer_path: Some(layered.target_layer_path.clone()),
        target_layer_depth: Some(layered.target_layer_depth),
        target_layer_reason: Some(layered.target_layer_reason.clone()),
        parent_layer_id: layered.parent_layer_id.clone(),
        parent_graph_node_id: layered.parent_graph_node_id.clone(),
        created_child_layer_id: layered.created_child_layer_id.clone(),
        layer_membership_id: Some(layered.layer_membership_id.clone()),
        layer_edge_id: layered.layer_edge_id.clone(),
        receipt_hash: receipt_hash.map(|hash| hash.to_string()),
    }
}

fn validation_diagnostics(
    report: &KgWritebackDryRunReport,
) -> Result<KgWritebackPersistedValidationDiagnostics> {
    let council_status = council_status_sql(report).to_owned();
    Ok(KgWritebackPersistedValidationDiagnostics {
        validation_report_id: report.validation_proposal.validation_report_id.clone(),
        validation_status: validation_status_sql(report)?.to_owned(),
        risk_class: risk_class_sql(report.validation_proposal.risk_class)?,
        risk_bp: risk_bp(report.validation_proposal.risk_class),
        council_required: council_status == "required",
        council_status,
        decision: validation_decision_sql(report).to_owned(),
        notes_status: if report.validation_proposal.needs_review {
            "review_reasons_recorded"
        } else {
            "pending_validation_note_recorded"
        }
        .to_owned(),
    })
}

fn persisted_warning_summaries(report: &KgWritebackDryRunReport) -> Vec<String> {
    let mut warnings = BTreeSet::new();
    warnings.extend(report.warnings.iter().cloned());
    warnings.insert("preview_only_not_production_route".to_owned());
    warnings.insert("route_invalidation_deferred".to_owned());
    warnings.insert("export_persistence_deferred".to_owned());
    warnings.insert("gateway_exposure_deferred".to_owned());
    warnings.insert("graph_explorer_deferred".to_owned());
    warnings.insert("memory_candidate_queue_missing".to_owned());
    warnings.insert("governance_review_queue_missing".to_owned());
    warnings.insert("payload_content_not_exported".to_owned());
    warnings.insert("no_exo_dag_write".to_owned());
    warnings.into_iter().collect()
}

async fn fetch_idempotency_replay(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    idempotency_key: &str,
    request_hash: Hash256,
) -> Result<Option<KgWritebackPersistedSummary>> {
    let row = sqlx::query(
        "SELECT request_hash, response_body FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4 \
         FOR UPDATE",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(KG_WRITEBACK_PERSISTED_ROUTE_NAME)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;

    let Some(row) = row else {
        return Ok(None);
    };
    let existing_hash = hash_from_vec(row.try_get("request_hash").map_err(pg)?)?;
    if existing_hash != request_hash {
        return Err(KgWritebackPersistenceError::Conflict {
            reason: "idempotency_key_conflict".to_owned(),
        });
    }
    let body: JsonValue = row.try_get("response_body").map_err(pg)?;
    let mut summary: KgWritebackPersistedSummary =
        serde_json::from_value(body).map_err(|error| KgWritebackPersistenceError::Json {
            reason: error.to_string(),
        })?;
    summary.replayed = true;
    summary.diagnostics.idempotency_replay.replayed = true;
    summary.diagnostics.idempotency_replay.request_hash = request_hash.to_string();
    summary.diagnostics.idempotency_replay.replay_reason = "idempotency_key_match".to_owned();
    Ok(Some(summary))
}

async fn insert_idempotency_response(
    tx: &mut Transaction<'_, Postgres>,
    summary: &KgWritebackPersistedSummary,
    request_hash: Hash256,
) -> Result<()> {
    let response_body = json_value(summary)?;
    let response_hash =
        hash_event_body(summary).map_err(|error| KgWritebackPersistenceError::Hash {
            reason: error.to_string(),
        })?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let expires_at = timestamp_parts(EXPIRES_AT)?;
    sqlx::query(
        "INSERT INTO dagdb_idempotency_keys \
         (tenant_id, namespace, route_name, idempotency_key, request_hash, response_hash, response_body, \
          status_code, cached_failure, created_at_physical_ms, created_at_logical, \
          expires_at_physical_ms, expires_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 201, false, $8, $9, $10, $11)",
    )
    .bind(&summary.tenant_id)
    .bind(&summary.namespace)
    .bind(KG_WRITEBACK_PERSISTED_ROUTE_NAME)
    .bind(&summary.idempotency_key)
    .bind(hash_bytes(request_hash))
    .bind(hash_bytes(response_hash))
    .bind(response_body)
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .bind(expires_at.physical_ms)
    .bind(expires_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    Ok(())
}

async fn verify_evidence_memories(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
) -> Result<()> {
    for memory_id in &report.evidence_binding.selected_memory_ids {
        let row = sqlx::query(
            "SELECT tenant_id, namespace FROM dagdb_memory_objects WHERE memory_id = $1",
        )
        .bind(hash_bytes(hash_from_hex("selected_memory_id", memory_id)?))
        .fetch_optional(&mut **tx)
        .await
        .map_err(pg)?;
        let Some(row) = row else {
            return Err(KgWritebackPersistenceError::Conflict {
                reason: format!("selected memory evidence not found: {memory_id}"),
            });
        };
        let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
        let namespace: String = row.try_get("namespace").map_err(pg)?;
        if tenant_id != report.tenant_id || namespace != report.namespace {
            return Err(KgWritebackPersistenceError::Conflict {
                reason: format!("selected memory evidence crosses tenant/namespace: {memory_id}"),
            });
        }
    }
    for memory_id in &report
        .placement_proposal
        .canonicalization_decision
        .matched_memory_ids
    {
        ensure_existing_memory(tx, report, memory_id).await?;
    }
    for edge in &report.placement_proposal.edges_to_create {
        if edge.from_memory_id != report.candidate_id {
            ensure_existing_memory(tx, report, &edge.from_memory_id).await?;
        }
        if edge.to_memory_id != report.candidate_id {
            ensure_existing_memory(tx, report, &edge.to_memory_id).await?;
        }
    }
    Ok(())
}

async fn ensure_existing_memory(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    memory_id: &str,
) -> Result<()> {
    let row =
        sqlx::query("SELECT tenant_id, namespace FROM dagdb_memory_objects WHERE memory_id = $1")
            .bind(hash_bytes(hash_from_hex("memory_id", memory_id)?))
            .fetch_optional(&mut **tx)
            .await
            .map_err(pg)?;
    let Some(row) = row else {
        return Err(KgWritebackPersistenceError::Conflict {
            reason: format!("placement references unknown memory {memory_id}"),
        });
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    if tenant_id == report.tenant_id && namespace == report.namespace {
        Ok(())
    } else {
        Err(KgWritebackPersistenceError::Conflict {
            reason: format!("placement references cross-scope memory {memory_id}"),
        })
    }
}

async fn insert_receipt(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    subject_kind: SubjectKind,
    subject_id: &str,
    event_type: ReceiptEventType,
    reason: &str,
) -> Result<PersistedReceiptWrite> {
    let subject_id = hash_from_hex("receipt.subject_id", subject_id)?;
    let receipt_body = json!({
        "proposal_id": report.proposal_id,
        "candidate_id": report.candidate_id,
        "reason": reason,
        "source": "kg_writeback_persisted_adapter",
        "preview_evidence_only": true
    });
    let event_body_hash = stable_hash(
        "exo.dagdb.kg_writeback.persisted.receipt_body_hash",
        &[
            &report.proposal_id,
            &subject_id.to_string(),
            event_type_sql(event_type),
            reason,
        ],
    )?;
    let receipt_hash = ReceiptHashMaterial {
        tenant_id: report.tenant_id.clone(),
        namespace: report.namespace.clone(),
        subject_kind,
        subject_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type,
        actor_did: report.requesting_agent_did.clone(),
        event_hlc: CREATED_AT,
        event_body_hash,
    }
    .hash()
    .map_err(|error| KgWritebackPersistenceError::Hash {
        reason: error.to_string(),
    })?;
    let event_hlc = timestamp_parts(CREATED_AT)?;
    let receipt_result = sqlx::query(
        "INSERT INTO dagdb_receipts \
         (receipt_hash, tenant_id, namespace, subject_kind, subject_id, prev_receipt_hash, seq, \
          event_type, actor_did, event_hlc_physical_ms, event_hlc_logical, event_hash, receipt_body, \
          created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, $9, $10, $11, $12, $9, $10) \
         ON CONFLICT (receipt_hash) DO NOTHING",
    )
    .bind(hash_bytes(receipt_hash))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(subject_kind_sql(subject_kind))
    .bind(hash_bytes(subject_id))
    .bind(hash_bytes(Hash256::ZERO))
    .bind(event_type_sql(event_type))
    .bind(&report.requesting_agent_did)
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .bind(hash_bytes(event_body_hash))
    .bind(receipt_body)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;

    let head_result = sqlx::query(
        "INSERT INTO dagdb_subject_receipt_heads \
         (tenant_id, namespace, subject_kind, subject_id, latest_receipt_hash, latest_seq, \
          updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, $5, 1, $6, $7) \
         ON CONFLICT (tenant_id, namespace, subject_kind, subject_id) DO NOTHING",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(subject_kind_sql(subject_kind))
    .bind(hash_bytes(subject_id))
    .bind(hash_bytes(receipt_hash))
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    Ok(PersistedReceiptWrite {
        receipt_hash,
        inserted_receipt_count: rows_to_u32(receipt_result.rows_affected())?,
        inserted_subject_receipt_head_count: rows_to_u32(head_result.rows_affected())?,
    })
}

#[derive(Debug, Clone, Copy)]
struct PersistedReceiptWrite {
    receipt_hash: Hash256,
    inserted_receipt_count: u32,
    inserted_subject_receipt_head_count: u32,
}

async fn insert_memory(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    latest_receipt_hash: Hash256,
) -> Result<u32> {
    ensure_memory_match(tx, report).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_memory_objects \
         (memory_id, tenant_id, namespace, node_type, source_type, consent_purpose, payload_hash, source_hash, \
          owner_did, controller_did, submitted_by_did, title, summary, keywords, risk_class, risk_bp, \
          status, validation_status, council_status, dag_finality_status, latest_receipt_hash, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, 'answer', 'generated', 'writeback', $4, $5, $6, $6, $6, $7, $8, '[]'::jsonb, \
          $9, $10, 'pending', $11, $12, 'pending', $13, $14, $15, $14, $15) \
         ON CONFLICT (memory_id) DO NOTHING",
    )
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("output_hash", &report.output_hash)?))
    .bind(hash_bytes(hash_from_hex(
        "parent_context_packet_id",
        &report.parent_context_packet_id,
    )?))
    .bind(&report.requesting_agent_did)
    .bind(json_value(&safe_metadata("Agent writeback", &report.proposal_id)?)?)
    .bind(json_value(&safe_metadata(
        &report.proposed_memory_candidate.summary,
        &report.output_hash,
    )?)?)
    .bind(risk_class_sql(report.validation_proposal.risk_class)?)
    .bind(i32::from(risk_bp(report.validation_proposal.risk_class)))
    .bind(validation_status_sql(report)?)
    .bind(council_status_sql(report))
    .bind(hash_bytes(latest_receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_catalog(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    latest_receipt_hash: Hash256,
) -> Result<u32> {
    let catalog_id = catalog_id(report)?;
    ensure_catalog_match(tx, report, catalog_id).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_catalog_entries \
         (catalog_id, tenant_id, namespace, memory_id, catalog_level, title, summary, keywords, payload_hash, source_hash, \
          status, validation_status, council_status, dag_finality_status, latest_receipt_hash, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, 3, $5, $6, '[]'::jsonb, $7, $8, 'pending', $9, $10, 'pending', $11, $12, $13, $12, $13) \
         ON CONFLICT (catalog_id) DO NOTHING",
    )
    .bind(hash_bytes(catalog_id))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(json_value(&safe_metadata("Agent writeback", &report.proposal_id)?)?)
    .bind(json_value(&safe_metadata(
        &report.proposed_memory_candidate.summary,
        &report.output_hash,
    )?)?)
    .bind(hash_bytes(hash_from_hex("output_hash", &report.output_hash)?))
    .bind(hash_bytes(hash_from_hex(
        "parent_context_packet_id",
        &report.parent_context_packet_id,
    )?))
    .bind(validation_status_sql(report)?)
    .bind(council_status_sql(report))
    .bind(hash_bytes(latest_receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_graph_node(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
) -> Result<u32> {
    let graph_node_id = graph_node_id(report)?;
    ensure_graph_node_match(tx, report, graph_node_id).await?;
    let canonical_memory_id = report
        .placement_proposal
        .canonicalization_decision
        .canonical_memory_id
        .as_ref()
        .map(|value| hash_from_hex("canonical_memory_id", value).map(hash_bytes))
        .transpose()?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_nodes \
         (graph_node_id, tenant_id, namespace, memory_id, graph_style, node_kind, canonical_memory_id, catalog_path, metadata, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, 'canonical_memory_graph', $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (graph_node_id) DO NOTHING",
    )
    .bind(hash_bytes(graph_node_id))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(node_kind_sql(
        report
            .placement_proposal
            .canonicalization_decision
            .decision_kind,
    ))
    .bind(canonical_memory_id)
    .bind("KnowledgeGraphs/dag-db/writebacks")
    .bind(json!({
        "source": "kg_writeback_dry_run",
        "preview_evidence_only": true,
        "proposal_id": report.proposal_id
    }))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_layered_writeback(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    receipt_hash: Hash256,
) -> Result<PersistedLayerCounts> {
    let Some(layered) = &report.layered_writeback else {
        return Ok(PersistedLayerCounts::default());
    };

    if layered.target_layer_depth > 0 {
        ensure_parent_layer_binding(tx, report, layered).await?;
    }

    let mut counts = PersistedLayerCounts {
        layer_receipt_hash: Some(receipt_hash),
        ..PersistedLayerCounts::default()
    };
    if layered.created_child_layer_id.is_some() {
        counts.inserted_layer_count = counts
            .inserted_layer_count
            .saturating_add(insert_writeback_layer(tx, report, layered).await?);
    } else {
        ensure_target_layer_exists(tx, report, layered).await?;
    }
    counts.inserted_layer_membership_count = counts
        .inserted_layer_membership_count
        .saturating_add(insert_writeback_layer_membership(tx, report, layered).await?);
    if layered.layer_edge_id.is_some() {
        counts.inserted_layer_edge_count = counts
            .inserted_layer_edge_count
            .saturating_add(insert_writeback_layer_edge(tx, report, layered, receipt_hash).await?);
    }
    Ok(counts)
}

async fn ensure_parent_layer_binding(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
) -> Result<()> {
    let parent_layer_id = layered.parent_layer_id.as_deref().ok_or_else(|| {
        KgWritebackPersistenceError::Conflict {
            reason: "layered writeback missing parent_layer_id".to_owned(),
        }
    })?;
    let parent_graph_node_id = layered.parent_graph_node_id.as_deref().ok_or_else(|| {
        KgWritebackPersistenceError::Conflict {
            reason: "layered writeback missing parent_graph_node_id".to_owned(),
        }
    })?;
    let row = sqlx::query(
        "SELECT 1 \
         FROM dagdb_graph_layers layer \
         JOIN dagdb_graph_layer_memberships membership \
           ON membership.tenant_id = layer.tenant_id \
          AND membership.namespace = layer.namespace \
          AND membership.layer_id = layer.layer_id \
         JOIN dagdb_graph_nodes node \
           ON node.tenant_id = layer.tenant_id \
          AND node.namespace = layer.namespace \
          AND node.graph_node_id = membership.graph_node_id \
         WHERE layer.tenant_id = $1 \
           AND layer.namespace = $2 \
           AND layer.layer_id = $3 \
           AND membership.graph_node_id = $4 \
           AND node.graph_style = $5 \
         LIMIT 1",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex(
        "parent_layer_id",
        parent_layer_id,
    )?))
    .bind(hash_bytes(hash_from_hex(
        "parent_graph_node_id",
        parent_graph_node_id,
    )?))
    .bind(&layered.target_graph_style)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    if row.is_some() {
        Ok(())
    } else {
        Err(KgWritebackPersistenceError::Conflict {
            reason: "layered writeback parent layer/node binding not found".to_owned(),
        })
    }
}

async fn ensure_target_layer_exists(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT 1 FROM dagdb_graph_layers \
         WHERE tenant_id = $1 AND namespace = $2 AND layer_id = $3 \
           AND graph_style = $4 AND layer_path = $5 AND layer_depth = $6 \
         LIMIT 1",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex(
        "target_layer_id",
        &layered.target_layer_id,
    )?))
    .bind(&layered.target_graph_style)
    .bind(&layered.target_layer_path)
    .bind(
        i32::try_from(layered.target_layer_depth)
            .map_err(|_| KgWritebackPersistenceError::CountOutOfRange)?,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    if row.is_some() {
        Ok(())
    } else {
        Err(KgWritebackPersistenceError::Conflict {
            reason: "layered writeback target layer not found".to_owned(),
        })
    }
}

async fn insert_writeback_layer(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
) -> Result<u32> {
    ensure_writeback_layer_match(tx, report, layered).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let parent_layer_id = layered
        .parent_layer_id
        .as_ref()
        .map(|value| hash_from_hex("parent_layer_id", value).map(hash_bytes))
        .transpose()?;
    let parent_graph_node_id = layered
        .parent_graph_node_id
        .as_ref()
        .map(|value| hash_from_hex("parent_graph_node_id", value).map(hash_bytes))
        .transpose()?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_layers \
         (layer_id, tenant_id, namespace, root_memory_id, parent_layer_id, parent_graph_node_id, \
          layer_depth, layer_kind, graph_style, layer_path, metadata, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $12, $13) \
         ON CONFLICT (tenant_id, namespace, layer_path) DO NOTHING",
    )
    .bind(hash_bytes(hash_from_hex(
        "target_layer_id",
        &layered.target_layer_id,
    )?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex(
        "candidate_id",
        &report.candidate_id,
    )?))
    .bind(parent_layer_id)
    .bind(parent_graph_node_id)
    .bind(
        i32::try_from(layered.target_layer_depth)
            .map_err(|_| KgWritebackPersistenceError::CountOutOfRange)?,
    )
    .bind(&layered.target_layer_kind)
    .bind(&layered.target_graph_style)
    .bind(&layered.target_layer_path)
    .bind(json!({
        "source": "kg_writeback_persisted_adapter",
        "proposal_id": report.proposal_id,
        "target_layer_reason": layered.target_layer_reason
    }))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_writeback_layer_membership(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
) -> Result<u32> {
    let graph_node_id = graph_node_id(report)?;
    ensure_writeback_layer_membership_match(tx, report, layered, graph_node_id).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_layer_memberships \
         (layer_membership_id, tenant_id, namespace, layer_id, graph_node_id, graph_style, \
          membership_role, local_node_rank, metadata, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $10, $11) \
         ON CONFLICT (tenant_id, namespace, layer_id, graph_node_id) DO NOTHING",
    )
    .bind(hash_bytes(hash_from_hex(
        "layer_membership_id",
        &layered.layer_membership_id,
    )?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex(
        "target_layer_id",
        &layered.target_layer_id,
    )?))
    .bind(hash_bytes(graph_node_id))
    .bind(&layered.target_graph_style)
    .bind(&layered.membership_role)
    .bind(
        i32::try_from(layered.local_node_rank)
            .map_err(|_| KgWritebackPersistenceError::CountOutOfRange)?,
    )
    .bind(json!({
        "source": "kg_writeback_persisted_adapter",
        "proposal_id": report.proposal_id
    }))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_writeback_layer_edge(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
    receipt_hash: Hash256,
) -> Result<u32> {
    ensure_writeback_layer_edge_match(tx, report, layered, receipt_hash).await?;
    let parent_layer_id = layered.parent_layer_id.as_deref().ok_or_else(|| {
        KgWritebackPersistenceError::Conflict {
            reason: "layer edge missing parent_layer_id".to_owned(),
        }
    })?;
    let layer_edge_id =
        layered
            .layer_edge_id
            .as_deref()
            .ok_or_else(|| KgWritebackPersistenceError::Conflict {
                reason: "layer edge missing layer_edge_id".to_owned(),
            })?;
    let layer_edge_kind = layered.layer_edge_kind.as_deref().ok_or_else(|| {
        KgWritebackPersistenceError::Conflict {
            reason: "layer edge missing layer_edge_kind".to_owned(),
        }
    })?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_layer_edges \
         (layer_edge_id, tenant_id, namespace, graph_style, from_layer_id, to_layer_id, edge_kind, \
          receipt_hash, metadata, created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $10, $11) \
         ON CONFLICT (tenant_id, namespace, graph_style, from_layer_id, to_layer_id, edge_kind) DO NOTHING",
    )
    .bind(hash_bytes(hash_from_hex("layer_edge_id", layer_edge_id)?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(&layered.target_graph_style)
    .bind(hash_bytes(hash_from_hex("parent_layer_id", parent_layer_id)?))
    .bind(hash_bytes(hash_from_hex(
        "target_layer_id",
        &layered.target_layer_id,
    )?))
    .bind(layer_edge_kind)
    .bind(hash_bytes(receipt_hash))
    .bind(json!({
        "source": "kg_writeback_persisted_adapter",
        "proposal_id": report.proposal_id,
        "hygiene_state": "active"
    }))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_graph_edge(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    edge: &exo_dag_db_api::GraphEdgeRef,
    receipt_hash: Hash256,
) -> Result<u32> {
    let graph_style = graph_style_for_edge(edge.edge_kind);
    let edge_kind = edge_kind_sql(edge.edge_kind);
    let edge_id = graph_edge_id(report, &edge.from_memory_id, &edge.to_memory_id, edge_kind)?;
    ensure_graph_edge_match(
        tx,
        edge_id,
        report,
        graph_style,
        &edge.from_memory_id,
        &edge.to_memory_id,
        edge_kind,
    )
    .await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_edges \
         (graph_edge_id, tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind, receipt_hash, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind) DO NOTHING",
    )
    .bind(hash_bytes(edge_id))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(graph_style)
    .bind(hash_bytes(hash_from_hex("edge.from_memory_id", &edge.from_memory_id)?))
    .bind(hash_bytes(hash_from_hex("edge.to_memory_id", &edge.to_memory_id)?))
    .bind(edge_kind)
    .bind(hash_bytes(receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_parent_context_graph_edge(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    parent_memory_id: &str,
    receipt_hash: Hash256,
) -> Result<u32> {
    let edge_kind = "derived_from";
    let graph_style = "canonical_memory_graph";
    let edge_id = graph_edge_id(report, &report.candidate_id, parent_memory_id, edge_kind)?;
    ensure_graph_edge_match(
        tx,
        edge_id,
        report,
        graph_style,
        &report.candidate_id,
        parent_memory_id,
        edge_kind,
    )
    .await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_edges \
         (graph_edge_id, tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind, receipt_hash, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind) DO NOTHING",
    )
    .bind(hash_bytes(edge_id))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(graph_style)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(hash_bytes(hash_from_hex("parent_memory_id", parent_memory_id)?))
    .bind(edge_kind)
    .bind(hash_bytes(receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_parent_context_memory_edge(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    parent_memory_id: &str,
) -> Result<u32> {
    let edge_type = "derived_from";
    ensure_memory_edge_match(tx, report, parent_memory_id, edge_type).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_memory_edges \
         (tenant_id, namespace, from_memory_id, to_memory_id, edge_type, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (tenant_id, namespace, from_memory_id, to_memory_id, edge_type) DO NOTHING",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(hash_bytes(hash_from_hex("parent_memory_id", parent_memory_id)?))
    .bind(edge_type)
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_similarity_result(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    similarity: &exo_dag_db_api::SimilarityResult,
) -> Result<u32> {
    ensure_existing_memory(tx, report, &similarity.candidate_memory_id).await?;
    let similarity_type = similarity_type_sql(similarity.similarity_type);
    let similarity_id = stable_hash(
        "exo.dagdb.kg_writeback.persisted.similarity_result_id",
        &[
            &report.tenant_id,
            &report.namespace,
            &report.candidate_id,
            &similarity.candidate_memory_id,
            similarity_type,
            &similarity.similarity_bp.to_string(),
        ],
    )?;
    ensure_similarity_match(tx, report, similarity, similarity_id, similarity_type).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_similarity_results \
         (similarity_result_id, tenant_id, namespace, candidate_memory_id, matched_memory_id, similarity_type, similarity_bp, matched_fields, reason, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (tenant_id, namespace, candidate_memory_id, matched_memory_id, similarity_type) DO NOTHING",
    )
    .bind(hash_bytes(similarity_id))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(hash_bytes(hash_from_hex(
        "matched_memory_id",
        &similarity.candidate_memory_id,
    )?))
    .bind(similarity_type)
    .bind(i32::from(similarity.similarity_bp))
    .bind(json_value(&similarity.matched_fields)?)
    .bind(&similarity.reason)
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_canonicalization_decision(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    receipt_hash: Hash256,
) -> Result<u32> {
    ensure_canonicalization_match(tx, report).await?;
    let decision = &report.placement_proposal.canonicalization_decision;
    let canonical_memory_id = decision
        .canonical_memory_id
        .as_ref()
        .map(|value| hash_from_hex("canonical_memory_id", value).map(hash_bytes))
        .transpose()?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_canonicalization_decisions \
         (decision_id, tenant_id, namespace, input_memory_id, canonical_memory_id, matched_memory_ids, decision_kind, \
          decision_reason, confidence_bp, risk_class, validator_status, required_edges_to_create, receipt_hash, receipt_intent, \
          created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) \
         ON CONFLICT (decision_id) DO NOTHING",
    )
    .bind(hash_bytes(hash_from_hex("decision_id", &decision.decision_id)?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(canonical_memory_id)
    .bind(json_value(&decision.matched_memory_ids)?)
    .bind(decision_kind_sql(decision.decision_kind))
    .bind(&decision.decision_reason)
    .bind(i32::from(decision.confidence_bp))
    .bind(risk_class_sql(decision.risk_class)?)
    .bind(validation_status_enum_sql(decision.validator_status)?)
    .bind(json_value(&decision.required_edges_to_create)?)
    .bind(hash_bytes(receipt_hash))
    .bind(&decision.receipt_intent)
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_placement_trace(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
) -> Result<u32> {
    let trace_id = placement_trace_id(report)?;
    ensure_placement_trace_match(tx, report, trace_id).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_graph_placement_traces \
         (placement_trace_id, tenant_id, namespace, input_memory_id, trace_steps, completed, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, true, $6, $7) \
         ON CONFLICT (placement_trace_id) DO NOTHING",
    )
    .bind(hash_bytes(trace_id))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(json_value(&report.placement_trace)?)
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_validation_report(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    latest_receipt_hash: Hash256,
) -> Result<u32> {
    ensure_validation_report_match(tx, report).await?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let policy_hash = stable_hash(
        "exo.dagdb.kg_writeback.persisted.validation_policy_hash",
        &[
            &report.tenant_id,
            &report.namespace,
            &report.validation_proposal.validation_report_id,
            "writeback_preview_policy",
        ],
    )?;
    let result = sqlx::query(
        "INSERT INTO dagdb_validation_reports \
         (validation_report_id, tenant_id, namespace, subject_kind, subject_id, validator_did, input_hash, policy_hash, \
          validation_status, risk_class, risk_bp, decision, notes, contradictory_report_ids, latest_receipt_hash, \
          created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, 'memory', $4, $5, $6, $7, $8, $9, $10, $11, $12, '[]'::jsonb, $13, $14, $15) \
         ON CONFLICT (validation_report_id) DO NOTHING",
    )
    .bind(hash_bytes(hash_from_hex(
        "validation_report_id",
        &report.validation_proposal.validation_report_id,
    )?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .bind(&report.requesting_agent_did)
    .bind(hash_bytes(hash_from_hex("output_hash", &report.output_hash)?))
    .bind(hash_bytes(policy_hash))
    .bind(validation_status_sql(report)?)
    .bind(risk_class_sql(report.validation_proposal.risk_class)?)
    .bind(i32::from(risk_bp(report.validation_proposal.risk_class)))
    .bind(validation_decision_sql(report))
    .bind(json_value(&safe_metadata(
        validation_notes(report).as_str(),
        &report.validation_proposal.validation_report_id,
    )?)?)
    .bind(hash_bytes(latest_receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn ensure_memory_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, payload_hash, source_hash, node_type FROM dagdb_memory_objects WHERE memory_id = $1",
    )
    .bind(hash_bytes(hash_from_hex("candidate_id", &report.candidate_id)?))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let payload_hash = hash_from_vec(row.try_get("payload_hash").map_err(pg)?)?;
    let source_hash = hash_from_vec(row.try_get("source_hash").map_err(pg)?)?;
    let node_type: String = row.try_get("node_type").map_err(pg)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && payload_hash == hash_from_hex("output_hash", &report.output_hash)?
        && source_hash
            == hash_from_hex("parent_context_packet_id", &report.parent_context_packet_id)?
        && node_type == "answer"
    {
        Ok(())
    } else {
        row_mismatch("dagdb_memory_objects", &report.candidate_id)
    }
}

async fn ensure_catalog_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    catalog_id: Hash256,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, memory_id, payload_hash, source_hash \
         FROM dagdb_catalog_entries WHERE catalog_id = $1",
    )
    .bind(hash_bytes(catalog_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let memory_id = hash_from_vec(row.try_get("memory_id").map_err(pg)?)?;
    let payload_hash = hash_from_vec(row.try_get("payload_hash").map_err(pg)?)?;
    let source_hash = hash_from_vec(row.try_get("source_hash").map_err(pg)?)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && memory_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && payload_hash == hash_from_hex("output_hash", &report.output_hash)?
        && source_hash
            == hash_from_hex("parent_context_packet_id", &report.parent_context_packet_id)?
    {
        Ok(())
    } else {
        row_mismatch("dagdb_catalog_entries", &catalog_id.to_string())
    }
}

async fn ensure_graph_node_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    graph_node_id: Hash256,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, memory_id, graph_style, node_kind \
         FROM dagdb_graph_nodes WHERE graph_node_id = $1",
    )
    .bind(hash_bytes(graph_node_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let memory_id = hash_from_vec(row.try_get("memory_id").map_err(pg)?)?;
    let graph_style: String = row.try_get("graph_style").map_err(pg)?;
    let node_kind: String = row.try_get("node_kind").map_err(pg)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && memory_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && graph_style == "canonical_memory_graph"
        && node_kind
            == node_kind_sql(
                report
                    .placement_proposal
                    .canonicalization_decision
                    .decision_kind,
            )
    {
        Ok(())
    } else {
        row_mismatch("dagdb_graph_nodes", &graph_node_id.to_string())
    }
}

async fn ensure_writeback_layer_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT layer_id, tenant_id, namespace, root_memory_id, parent_layer_id, parent_graph_node_id, \
                layer_depth, layer_kind, graph_style, layer_path \
         FROM dagdb_graph_layers \
         WHERE layer_id = $1 OR (tenant_id = $2 AND namespace = $3 AND layer_path = $4) \
         ORDER BY layer_id \
         LIMIT 1",
    )
    .bind(hash_bytes(hash_from_hex(
        "target_layer_id",
        &layered.target_layer_id,
    )?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(&layered.target_layer_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let existing_layer_id = hash_from_vec(row.try_get("layer_id").map_err(pg)?)?;
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let root_memory_id = hash_from_vec(row.try_get("root_memory_id").map_err(pg)?)?;
    let parent_layer_id = optional_hash_from_vec(row.try_get("parent_layer_id").map_err(pg)?)?;
    let parent_graph_node_id =
        optional_hash_from_vec(row.try_get("parent_graph_node_id").map_err(pg)?)?;
    let layer_depth: i32 = row.try_get("layer_depth").map_err(pg)?;
    let layer_kind: String = row.try_get("layer_kind").map_err(pg)?;
    let graph_style: String = row.try_get("graph_style").map_err(pg)?;
    let layer_path: String = row.try_get("layer_path").map_err(pg)?;
    let expected_parent_layer_id = layered
        .parent_layer_id
        .as_ref()
        .map(|value| hash_from_hex("parent_layer_id", value))
        .transpose()?;
    let expected_parent_graph_node_id = layered
        .parent_graph_node_id
        .as_ref()
        .map(|value| hash_from_hex("parent_graph_node_id", value))
        .transpose()?;
    if existing_layer_id == hash_from_hex("target_layer_id", &layered.target_layer_id)?
        && tenant_id == report.tenant_id
        && namespace == report.namespace
        && root_memory_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && parent_layer_id == expected_parent_layer_id
        && parent_graph_node_id == expected_parent_graph_node_id
        && layer_depth
            == i32::try_from(layered.target_layer_depth)
                .map_err(|_| KgWritebackPersistenceError::CountOutOfRange)?
        && layer_kind == layered.target_layer_kind
        && graph_style == layered.target_graph_style
        && layer_path == layered.target_layer_path
    {
        Ok(())
    } else {
        row_mismatch("dagdb_graph_layers", &layered.target_layer_id)
    }
}

async fn ensure_writeback_layer_membership_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
    graph_node_id: Hash256,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT layer_membership_id, tenant_id, namespace, layer_id, graph_node_id, graph_style, \
                membership_role, local_node_rank \
         FROM dagdb_graph_layer_memberships \
         WHERE layer_membership_id = $1 \
            OR (tenant_id = $2 AND namespace = $3 AND layer_id = $4 AND graph_node_id = $5) \
         ORDER BY layer_membership_id \
         LIMIT 1",
    )
    .bind(hash_bytes(hash_from_hex(
        "layer_membership_id",
        &layered.layer_membership_id,
    )?))
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex(
        "target_layer_id",
        &layered.target_layer_id,
    )?))
    .bind(hash_bytes(graph_node_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let existing_id = hash_from_vec(row.try_get("layer_membership_id").map_err(pg)?)?;
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let layer_id = hash_from_vec(row.try_get("layer_id").map_err(pg)?)?;
    let existing_graph_node_id = hash_from_vec(row.try_get("graph_node_id").map_err(pg)?)?;
    let graph_style: String = row.try_get("graph_style").map_err(pg)?;
    let membership_role: String = row.try_get("membership_role").map_err(pg)?;
    let local_node_rank: i32 = row.try_get("local_node_rank").map_err(pg)?;
    if existing_id == hash_from_hex("layer_membership_id", &layered.layer_membership_id)?
        && tenant_id == report.tenant_id
        && namespace == report.namespace
        && layer_id == hash_from_hex("target_layer_id", &layered.target_layer_id)?
        && existing_graph_node_id == graph_node_id
        && graph_style == layered.target_graph_style
        && membership_role == layered.membership_role
        && local_node_rank
            == i32::try_from(layered.local_node_rank)
                .map_err(|_| KgWritebackPersistenceError::CountOutOfRange)?
    {
        Ok(())
    } else {
        row_mismatch(
            "dagdb_graph_layer_memberships",
            &layered.layer_membership_id,
        )
    }
}

async fn ensure_writeback_layer_edge_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    layered: &crate::kg_writeback::KgWritebackLayeredWriteback,
    receipt_hash: Hash256,
) -> Result<()> {
    let Some(parent_layer_id) = layered.parent_layer_id.as_deref() else {
        return Ok(());
    };
    let Some(layer_edge_id) = layered.layer_edge_id.as_deref() else {
        return Ok(());
    };
    let Some(layer_edge_kind) = layered.layer_edge_kind.as_deref() else {
        return Ok(());
    };
    let expected_from = hash_from_hex("parent_layer_id", parent_layer_id)?;
    let expected_to = hash_from_hex("target_layer_id", &layered.target_layer_id)?;
    let expected_edge_id = hash_from_hex("layer_edge_id", layer_edge_id)?;
    let id_row = sqlx::query(
        "SELECT layer_edge_id, tenant_id, namespace, graph_style, from_layer_id, to_layer_id, edge_kind, receipt_hash \
         FROM dagdb_graph_layer_edges WHERE layer_edge_id = $1 LIMIT 1",
    )
    .bind(hash_bytes(expected_edge_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    if let Some(row) = id_row {
        return ensure_layer_edge_row_matches(
            row,
            Some(expected_edge_id),
            report,
            &layered.target_graph_style,
            expected_from,
            expected_to,
            layer_edge_kind,
            receipt_hash,
        );
    }

    let natural_row = sqlx::query(
        "SELECT layer_edge_id, tenant_id, namespace, graph_style, from_layer_id, to_layer_id, edge_kind, receipt_hash \
         FROM dagdb_graph_layer_edges \
         WHERE tenant_id = $1 AND namespace = $2 AND graph_style = $3 \
           AND from_layer_id = $4 AND to_layer_id = $5 AND edge_kind = $6 \
         LIMIT 1",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(&layered.target_graph_style)
    .bind(hash_bytes(expected_from))
    .bind(hash_bytes(expected_to))
    .bind(layer_edge_kind)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    if let Some(row) = natural_row {
        return ensure_layer_edge_row_matches(
            row,
            None,
            report,
            &layered.target_graph_style,
            expected_from,
            expected_to,
            layer_edge_kind,
            receipt_hash,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn ensure_layer_edge_row_matches(
    row: sqlx::postgres::PgRow,
    expected_edge_id: Option<Hash256>,
    report: &KgWritebackDryRunReport,
    graph_style: &str,
    expected_from: Hash256,
    expected_to: Hash256,
    edge_kind: &str,
    receipt_hash: Hash256,
) -> Result<()> {
    let existing_edge_id = hash_from_vec(row.try_get("layer_edge_id").map_err(pg)?)?;
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let existing_graph_style: String = row.try_get("graph_style").map_err(pg)?;
    let existing_from = hash_from_vec(row.try_get("from_layer_id").map_err(pg)?)?;
    let existing_to = hash_from_vec(row.try_get("to_layer_id").map_err(pg)?)?;
    let existing_kind: String = row.try_get("edge_kind").map_err(pg)?;
    let existing_receipt_hash = optional_hash_from_vec(row.try_get("receipt_hash").map_err(pg)?)?;
    let edge_id_matches = match expected_edge_id {
        Some(expected) => existing_edge_id == expected,
        None => true,
    };
    if edge_id_matches
        && tenant_id == report.tenant_id
        && namespace == report.namespace
        && existing_graph_style == graph_style
        && existing_from == expected_from
        && existing_to == expected_to
        && existing_kind == edge_kind
        && existing_receipt_hash == Some(receipt_hash)
    {
        Ok(())
    } else {
        row_mismatch("dagdb_graph_layer_edges", &existing_edge_id.to_string())
    }
}

#[allow(clippy::too_many_arguments)]
async fn ensure_graph_edge_match(
    tx: &mut Transaction<'_, Postgres>,
    graph_edge_id: Hash256,
    report: &KgWritebackDryRunReport,
    graph_style: &str,
    from_memory_id: &str,
    to_memory_id: &str,
    edge_kind: &str,
) -> Result<()> {
    let expected_from = hash_from_hex("from_memory_id", from_memory_id)?;
    let expected_to = hash_from_hex("to_memory_id", to_memory_id)?;
    let computed_id_row = sqlx::query(
        "SELECT graph_edge_id, tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind \
         FROM dagdb_graph_edges \
         WHERE graph_edge_id = $1 \
         LIMIT 1",
    )
    .bind(hash_bytes(graph_edge_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    if let Some(row) = computed_id_row {
        return ensure_graph_edge_row_matches(
            row,
            graph_edge_id,
            report,
            graph_style,
            expected_from,
            expected_to,
            edge_kind,
        );
    }

    let natural_key_row = sqlx::query(
        "SELECT graph_edge_id, tenant_id, namespace, graph_style, from_memory_id, to_memory_id, edge_kind \
         FROM dagdb_graph_edges \
         WHERE tenant_id = $1 AND namespace = $2 AND graph_style = $3 \
           AND from_memory_id = $4 AND to_memory_id = $5 AND edge_kind = $6 \
         LIMIT 1",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(graph_style)
    .bind(hash_bytes(expected_from))
    .bind(hash_bytes(expected_to))
    .bind(edge_kind)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    if natural_key_row.is_some() {
        return Ok(());
    }
    Ok(())
}

fn ensure_graph_edge_row_matches(
    row: sqlx::postgres::PgRow,
    graph_edge_id: Hash256,
    report: &KgWritebackDryRunReport,
    graph_style: &str,
    expected_from: Hash256,
    expected_to: Hash256,
    edge_kind: &str,
) -> Result<()> {
    let existing_edge_id = hash_from_vec(row.try_get("graph_edge_id").map_err(pg)?)?;
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let existing_graph_style: String = row.try_get("graph_style").map_err(pg)?;
    let existing_from = hash_from_vec(row.try_get("from_memory_id").map_err(pg)?)?;
    let existing_to = hash_from_vec(row.try_get("to_memory_id").map_err(pg)?)?;
    let existing_kind: String = row.try_get("edge_kind").map_err(pg)?;
    if existing_edge_id == graph_edge_id
        && tenant_id == report.tenant_id
        && namespace == report.namespace
        && existing_graph_style == graph_style
        && existing_from == expected_from
        && existing_to == expected_to
        && existing_kind == edge_kind
    {
        Ok(())
    } else {
        row_mismatch("dagdb_graph_edges", &graph_edge_id.to_string())
    }
}

async fn ensure_memory_edge_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    parent_memory_id: &str,
    edge_type: &str,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, from_memory_id, to_memory_id, edge_type \
         FROM dagdb_memory_edges \
         WHERE tenant_id = $1 AND namespace = $2 AND from_memory_id = $3 \
            AND to_memory_id = $4 AND edge_type = $5 \
         LIMIT 1",
    )
    .bind(&report.tenant_id)
    .bind(&report.namespace)
    .bind(hash_bytes(hash_from_hex(
        "candidate_id",
        &report.candidate_id,
    )?))
    .bind(hash_bytes(hash_from_hex(
        "parent_memory_id",
        parent_memory_id,
    )?))
    .bind(edge_type)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let existing_from = hash_from_vec(row.try_get("from_memory_id").map_err(pg)?)?;
    let existing_to = hash_from_vec(row.try_get("to_memory_id").map_err(pg)?)?;
    let existing_type: String = row.try_get("edge_type").map_err(pg)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && existing_from == hash_from_hex("candidate_id", &report.candidate_id)?
        && existing_to == hash_from_hex("parent_memory_id", parent_memory_id)?
        && existing_type == edge_type
    {
        Ok(())
    } else {
        row_mismatch("dagdb_memory_edges", &report.candidate_id)
    }
}

async fn ensure_similarity_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    similarity: &exo_dag_db_api::SimilarityResult,
    similarity_id: Hash256,
    similarity_type: &str,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, candidate_memory_id, matched_memory_id, similarity_type \
         FROM dagdb_graph_similarity_results WHERE similarity_result_id = $1",
    )
    .bind(hash_bytes(similarity_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let candidate_memory_id = hash_from_vec(row.try_get("candidate_memory_id").map_err(pg)?)?;
    let matched_memory_id = hash_from_vec(row.try_get("matched_memory_id").map_err(pg)?)?;
    let existing_type: String = row.try_get("similarity_type").map_err(pg)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && candidate_memory_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && matched_memory_id == hash_from_hex("matched_memory_id", &similarity.candidate_memory_id)?
        && existing_type == similarity_type
    {
        Ok(())
    } else {
        row_mismatch("dagdb_graph_similarity_results", &similarity_id.to_string())
    }
}

async fn ensure_canonicalization_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
) -> Result<()> {
    let decision = &report.placement_proposal.canonicalization_decision;
    let row = sqlx::query(
        "SELECT tenant_id, namespace, input_memory_id, decision_kind \
         FROM dagdb_graph_canonicalization_decisions WHERE decision_id = $1",
    )
    .bind(hash_bytes(hash_from_hex(
        "decision_id",
        &decision.decision_id,
    )?))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let input_memory_id = hash_from_vec(row.try_get("input_memory_id").map_err(pg)?)?;
    let decision_kind: String = row.try_get("decision_kind").map_err(pg)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && input_memory_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && decision_kind == decision_kind_sql(decision.decision_kind)
    {
        Ok(())
    } else {
        row_mismatch(
            "dagdb_graph_canonicalization_decisions",
            &decision.decision_id,
        )
    }
}

async fn ensure_placement_trace_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
    trace_id: Hash256,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, input_memory_id, completed \
         FROM dagdb_graph_placement_traces WHERE placement_trace_id = $1",
    )
    .bind(hash_bytes(trace_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let input_memory_id = hash_from_vec(row.try_get("input_memory_id").map_err(pg)?)?;
    let completed: bool = row.try_get("completed").map_err(pg)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && input_memory_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && completed
    {
        Ok(())
    } else {
        row_mismatch("dagdb_graph_placement_traces", &trace_id.to_string())
    }
}

async fn ensure_validation_report_match(
    tx: &mut Transaction<'_, Postgres>,
    report: &KgWritebackDryRunReport,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT tenant_id, namespace, subject_kind, subject_id, input_hash \
         FROM dagdb_validation_reports WHERE validation_report_id = $1",
    )
    .bind(hash_bytes(hash_from_hex(
        "validation_report_id",
        &report.validation_proposal.validation_report_id,
    )?))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;
    let Some(row) = row else {
        return Ok(());
    };
    let tenant_id: String = row.try_get("tenant_id").map_err(pg)?;
    let namespace: String = row.try_get("namespace").map_err(pg)?;
    let subject_kind: String = row.try_get("subject_kind").map_err(pg)?;
    let subject_id = hash_from_vec(row.try_get("subject_id").map_err(pg)?)?;
    let input_hash = hash_from_vec(row.try_get("input_hash").map_err(pg)?)?;
    if tenant_id == report.tenant_id
        && namespace == report.namespace
        && subject_kind == "memory"
        && subject_id == hash_from_hex("candidate_id", &report.candidate_id)?
        && input_hash == hash_from_hex("output_hash", &report.output_hash)?
    {
        Ok(())
    } else {
        row_mismatch(
            "dagdb_validation_reports",
            &report.validation_proposal.validation_report_id,
        )
    }
}

fn row_mismatch<T>(table: &str, id: &str) -> Result<T> {
    Err(KgWritebackPersistenceError::Conflict {
        reason: format!("existing {table} row mismatch for {id}"),
    })
}

fn advisory_count(report: &KgWritebackDryRunReport) -> Result<u32> {
    let count = report.proposed_route_invalidations.len() + report.warnings.len();
    u32::try_from(count).map_err(|_| KgWritebackPersistenceError::CountOutOfRange)
}

fn catalog_id(report: &KgWritebackDryRunReport) -> Result<Hash256> {
    Ok(stable_hash(
        "exo.dagdb.kg_writeback.persisted.catalog_id",
        &[&report.tenant_id, &report.namespace, &report.candidate_id],
    )?)
}

fn graph_node_id(report: &KgWritebackDryRunReport) -> Result<Hash256> {
    Ok(stable_hash(
        "exo.dagdb.kg_writeback.persisted.graph_node_id",
        &[
            &report.tenant_id,
            &report.namespace,
            &report.candidate_id,
            "canonical_memory_graph",
        ],
    )?)
}

fn graph_edge_id(
    report: &KgWritebackDryRunReport,
    from_memory_id: &str,
    to_memory_id: &str,
    edge_kind: &str,
) -> Result<Hash256> {
    Ok(stable_hash(
        "exo.dagdb.kg_writeback.persisted.graph_edge_id",
        &[
            &report.tenant_id,
            &report.namespace,
            from_memory_id,
            to_memory_id,
            edge_kind,
        ],
    )?)
}

fn placement_trace_id(report: &KgWritebackDryRunReport) -> Result<Hash256> {
    Ok(stable_hash(
        "exo.dagdb.kg_writeback.persisted.placement_trace_id",
        &[
            &report.proposal_id,
            &report.candidate_id,
            &report
                .placement_proposal
                .canonicalization_decision
                .decision_id,
        ],
    )?)
}

fn safe_metadata(text: &str, original_hash: &str) -> Result<SafeMetadata> {
    Ok(SafeMetadata {
        decision: SafeMetadataDecision::Allow,
        text: text.to_owned(),
        redaction_codes: Vec::new(),
        original_hash: original_hash.to_owned(),
        truncated: false,
        byte_len: u32::try_from(text.len()).map_err(|_| {
            KgWritebackPersistenceError::UnsupportedSection {
                section: "safe_metadata_text_too_large".to_owned(),
            }
        })?,
    })
}

fn risk_bp(risk_class: RiskClass) -> u16 {
    match risk_class {
        RiskClass::R0 => 0,
        RiskClass::R1 => 1_000,
        RiskClass::R2 => 2_500,
        RiskClass::R3 => 5_000,
        RiskClass::R4 => 7_500,
        RiskClass::R5 => 10_000,
    }
}

fn risk_class_sql(risk_class: RiskClass) -> Result<String> {
    enum_sql(&risk_class)
}

fn validation_status_enum_sql(status: ValidationStatus) -> Result<String> {
    enum_sql(&status)
}

fn validation_status_sql(report: &KgWritebackDryRunReport) -> Result<&'static str> {
    if report.validation_proposal.needs_review {
        Ok("needs_council")
    } else if report.validation_proposal.validation_status == "passed" {
        Ok("passed")
    } else {
        Err(KgWritebackPersistenceError::UnsupportedSection {
            section: format!(
                "validation_status:{}",
                report.validation_proposal.validation_status
            ),
        })
    }
}

fn validation_decision_sql(report: &KgWritebackDryRunReport) -> &'static str {
    if report.validation_proposal.needs_review {
        "needs_council"
    } else {
        "allow"
    }
}

fn council_status_sql(report: &KgWritebackDryRunReport) -> &'static str {
    if report.validation_proposal.needs_review
        || matches!(
            report.validation_proposal.risk_class,
            RiskClass::R3 | RiskClass::R4 | RiskClass::R5
        )
    {
        "required"
    } else {
        "not_required"
    }
}

fn validation_notes(report: &KgWritebackDryRunReport) -> String {
    if report.validation_proposal.needs_review {
        format!(
            "writeback needs review: {}",
            report.validation_proposal.review_reasons.join(",")
        )
    } else {
        "writeback pending validation".to_owned()
    }
}

fn node_kind_sql(kind: CanonicalizationDecisionKind) -> &'static str {
    match kind {
        CanonicalizationDecisionKind::NewCanonical => "canonical",
        CanonicalizationDecisionKind::ExactDuplicate
        | CanonicalizationDecisionKind::NearDuplicate => "duplicate_reference",
        CanonicalizationDecisionKind::Related => "related",
        CanonicalizationDecisionKind::Replacement => "replacement",
        CanonicalizationDecisionKind::Contradiction => "contradiction",
        CanonicalizationDecisionKind::Supersession => "supersession",
        CanonicalizationDecisionKind::AlternateSummary => "alternate_summary",
        CanonicalizationDecisionKind::RejectedNeedsReview => "decision",
    }
}

fn graph_style_for_edge(edge_kind: MemoryEdgeKind) -> &'static str {
    match edge_kind {
        MemoryEdgeKind::Contradicts | MemoryEdgeKind::Supersedes | MemoryEdgeKind::Replaces => {
            "contradiction_supersession_graph"
        }
        _ => "canonical_memory_graph",
    }
}

fn edge_kind_sql(edge_kind: MemoryEdgeKind) -> &'static str {
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

fn decision_kind_sql(kind: CanonicalizationDecisionKind) -> &'static str {
    match kind {
        CanonicalizationDecisionKind::NewCanonical => "new_canonical",
        CanonicalizationDecisionKind::ExactDuplicate => "exact_duplicate",
        CanonicalizationDecisionKind::NearDuplicate => "near_duplicate",
        CanonicalizationDecisionKind::Related => "related",
        CanonicalizationDecisionKind::Replacement => "replacement",
        CanonicalizationDecisionKind::Contradiction => "contradiction",
        CanonicalizationDecisionKind::Supersession => "supersession",
        CanonicalizationDecisionKind::AlternateSummary => "alternate_summary",
        CanonicalizationDecisionKind::RejectedNeedsReview => "rejected_needs_review",
    }
}

fn similarity_type_sql(similarity_type: SimilarityType) -> &'static str {
    match similarity_type {
        SimilarityType::ExactHash => "exact_hash",
        SimilarityType::NearDuplicate => "near_duplicate",
        SimilarityType::ConceptOverlap => "concept_overlap",
        SimilarityType::WeakRelated => "weak_related",
    }
}

fn subject_kind_sql(kind: SubjectKind) -> &'static str {
    match kind {
        SubjectKind::Memory => "memory",
        SubjectKind::Catalog => "catalog",
        SubjectKind::Route => "route",
        SubjectKind::ContextPacket => "context_packet",
        SubjectKind::ValidationReport => "validation_report",
        SubjectKind::AgentSafetyScore => "agent_safety_score",
        SubjectKind::InboundAgentCredential => "inbound_agent_credential",
        SubjectKind::CouncilDecision => "council_decision",
    }
}

fn event_type_sql(event_type: ReceiptEventType) -> &'static str {
    match event_type {
        ReceiptEventType::IntakeCreated => "intake_created",
        ReceiptEventType::DuplicateRejected => "duplicate_rejected",
        ReceiptEventType::ValidationCreated => "validation_created",
        ReceiptEventType::ValidationPassed => "validation_passed",
        ReceiptEventType::ValidationFailed => "validation_failed",
        ReceiptEventType::MemoryApproved => "memory_approved",
        ReceiptEventType::MemoryRoutable => "memory_routable",
        ReceiptEventType::MemoryRevoked => "memory_revoked",
        ReceiptEventType::MemorySuperseded => "memory_superseded",
        ReceiptEventType::RouteCreated => "route_created",
        ReceiptEventType::RouteActivated => "route_activated",
        ReceiptEventType::RouteStale => "route_stale",
        ReceiptEventType::RouteInvalidated => "route_invalidated",
        ReceiptEventType::ContextPacketCreated => "context_packet_created",
        ReceiptEventType::WritebackCreated => "writeback_created",
        ReceiptEventType::TrustCheckCreated => "trust_check_created",
        ReceiptEventType::CouncilDecisionRecorded => "council_decision_recorded",
        ReceiptEventType::DagFinalityCommitted => "dag_finality_committed",
        ReceiptEventType::DagFinalityFailed => "dag_finality_failed",
        ReceiptEventType::DagFinalityCompensated => "dag_finality_compensated",
    }
}

fn enum_sql<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value).map_err(|error| KgWritebackPersistenceError::Json {
        reason: error.to_string(),
    })? {
        JsonValue::String(value) => Ok(value),
        other => Err(KgWritebackPersistenceError::Json {
            reason: format!("enum did not serialize as string: {other}"),
        }),
    }
}

fn json_value<T: Serialize>(value: &T) -> Result<JsonValue> {
    serde_json::to_value(value).map_err(|error| KgWritebackPersistenceError::Json {
        reason: error.to_string(),
    })
}

#[derive(Debug, Clone, Copy)]
struct SqlTimestamp {
    physical_ms: i64,
    logical: i32,
}

fn timestamp_parts(timestamp: Timestamp) -> Result<SqlTimestamp> {
    Ok(SqlTimestamp {
        physical_ms: i64::try_from(timestamp.physical_ms)
            .map_err(|_| KgWritebackPersistenceError::TimestampOutOfRange)?,
        logical: i32::try_from(timestamp.logical)
            .map_err(|_| KgWritebackPersistenceError::TimestampOutOfRange)?,
    })
}

fn rows_to_u32(rows: u64) -> Result<u32> {
    u32::try_from(rows).map_err(|_| KgWritebackPersistenceError::CountOutOfRange)
}

fn usize_to_u32(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| KgWritebackPersistenceError::CountOutOfRange)
}

fn hash_bytes(hash: Hash256) -> Vec<u8> {
    hash.as_bytes().to_vec()
}

fn hash_from_vec(bytes: Vec<u8>) -> Result<Hash256> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| KgWritebackPersistenceError::Conflict {
            reason: "hash column had invalid length".to_owned(),
        })?;
    Ok(Hash256::from_bytes(bytes))
}

fn optional_hash_from_vec(bytes: Option<Vec<u8>>) -> Result<Option<Hash256>> {
    bytes.map(hash_from_vec).transpose()
}

fn pg(source: sqlx::Error) -> KgWritebackPersistenceError {
    KgWritebackPersistenceError::Postgres { source }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writeback_sql_mapping_helpers_cover_all_variants() {
        assert_eq!(risk_bp(RiskClass::R0), 0);
        assert_eq!(risk_bp(RiskClass::R1), 1_000);
        assert_eq!(risk_bp(RiskClass::R2), 2_500);
        assert_eq!(risk_bp(RiskClass::R3), 5_000);
        assert_eq!(risk_bp(RiskClass::R4), 7_500);
        assert_eq!(risk_bp(RiskClass::R5), 10_000);
        assert_eq!(risk_class_sql(RiskClass::R4).expect("risk class"), "R4");
        assert_eq!(
            validation_status_enum_sql(ValidationStatus::NeedsCouncil).expect("validation status"),
            "needs_council"
        );

        for (kind, expected) in [
            (CanonicalizationDecisionKind::NewCanonical, "canonical"),
            (
                CanonicalizationDecisionKind::ExactDuplicate,
                "duplicate_reference",
            ),
            (
                CanonicalizationDecisionKind::NearDuplicate,
                "duplicate_reference",
            ),
            (CanonicalizationDecisionKind::Related, "related"),
            (CanonicalizationDecisionKind::Replacement, "replacement"),
            (CanonicalizationDecisionKind::Contradiction, "contradiction"),
            (CanonicalizationDecisionKind::Supersession, "supersession"),
            (
                CanonicalizationDecisionKind::AlternateSummary,
                "alternate_summary",
            ),
            (
                CanonicalizationDecisionKind::RejectedNeedsReview,
                "decision",
            ),
        ] {
            assert_eq!(node_kind_sql(kind), expected);
            assert!(!decision_kind_sql(kind).is_empty());
        }

        for (kind, expected_edge, expected_style) in [
            (
                MemoryEdgeKind::DerivedFrom,
                "derived_from",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::Summarizes,
                "summarizes",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::Supports,
                "supports",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::Contradicts,
                "contradicts",
                "contradiction_supersession_graph",
            ),
            (
                MemoryEdgeKind::Supersedes,
                "supersedes",
                "contradiction_supersession_graph",
            ),
            (
                MemoryEdgeKind::Replaces,
                "replaces",
                "contradiction_supersession_graph",
            ),
            (
                MemoryEdgeKind::DuplicateOf,
                "duplicate_of",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::NearDuplicateOf,
                "near_duplicate_of",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::RelatedTo,
                "related_to",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::AlternativeSummaryOf,
                "alternative_summary_of",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::DependsOn,
                "depends_on",
                "canonical_memory_graph",
            ),
            (MemoryEdgeKind::PartOf, "part_of", "canonical_memory_graph"),
            (
                MemoryEdgeKind::OwnedBy,
                "owned_by",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::AccessGrantedBy,
                "access_granted_by",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::VerifiedBy,
                "verified_by",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::UsedByRoute,
                "used_by_route",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::IncludedInContextPacket,
                "included_in_context_packet",
                "canonical_memory_graph",
            ),
            (
                MemoryEdgeKind::RevokedBy,
                "revoked_by",
                "canonical_memory_graph",
            ),
        ] {
            assert_eq!(edge_kind_sql(kind), expected_edge);
            assert_eq!(graph_style_for_edge(kind), expected_style);
        }

        for (kind, expected) in [
            (SimilarityType::ExactHash, "exact_hash"),
            (SimilarityType::NearDuplicate, "near_duplicate"),
            (SimilarityType::ConceptOverlap, "concept_overlap"),
            (SimilarityType::WeakRelated, "weak_related"),
        ] {
            assert_eq!(similarity_type_sql(kind), expected);
        }

        for (kind, expected) in [
            (SubjectKind::Memory, "memory"),
            (SubjectKind::Catalog, "catalog"),
            (SubjectKind::Route, "route"),
            (SubjectKind::ContextPacket, "context_packet"),
            (SubjectKind::ValidationReport, "validation_report"),
            (SubjectKind::AgentSafetyScore, "agent_safety_score"),
            (
                SubjectKind::InboundAgentCredential,
                "inbound_agent_credential",
            ),
            (SubjectKind::CouncilDecision, "council_decision"),
        ] {
            assert_eq!(subject_kind_sql(kind), expected);
        }

        for (event_type, expected) in [
            (ReceiptEventType::IntakeCreated, "intake_created"),
            (ReceiptEventType::DuplicateRejected, "duplicate_rejected"),
            (ReceiptEventType::ValidationCreated, "validation_created"),
            (ReceiptEventType::ValidationPassed, "validation_passed"),
            (ReceiptEventType::ValidationFailed, "validation_failed"),
            (ReceiptEventType::MemoryApproved, "memory_approved"),
            (ReceiptEventType::MemoryRoutable, "memory_routable"),
            (ReceiptEventType::MemoryRevoked, "memory_revoked"),
            (ReceiptEventType::MemorySuperseded, "memory_superseded"),
            (ReceiptEventType::RouteCreated, "route_created"),
            (ReceiptEventType::RouteActivated, "route_activated"),
            (ReceiptEventType::RouteStale, "route_stale"),
            (ReceiptEventType::RouteInvalidated, "route_invalidated"),
            (
                ReceiptEventType::ContextPacketCreated,
                "context_packet_created",
            ),
            (ReceiptEventType::WritebackCreated, "writeback_created"),
            (ReceiptEventType::TrustCheckCreated, "trust_check_created"),
            (
                ReceiptEventType::CouncilDecisionRecorded,
                "council_decision_recorded",
            ),
            (
                ReceiptEventType::DagFinalityCommitted,
                "dag_finality_committed",
            ),
            (ReceiptEventType::DagFinalityFailed, "dag_finality_failed"),
            (
                ReceiptEventType::DagFinalityCompensated,
                "dag_finality_compensated",
            ),
        ] {
            assert_eq!(event_type_sql(event_type), expected);
        }
    }

    #[test]
    fn writeback_conversion_helpers_fail_closed() {
        let timestamp = timestamp_parts(Timestamp::new(42, 7)).expect("timestamp");
        assert_eq!(timestamp.physical_ms, 42);
        assert_eq!(timestamp.logical, 7);
        assert_eq!(
            rows_to_u32(u64::from(u32::MAX)).expect("max rows"),
            u32::MAX
        );
        assert!(rows_to_u32(u64::from(u32::MAX) + 1).is_err());
        assert_eq!(
            usize_to_u32(usize::try_from(u32::MAX).expect("max usize")).expect("max usize"),
            u32::MAX
        );
        let hash = Hash256::from_bytes([0x5a; 32]);
        assert_eq!(hash_from_vec(hash_bytes(hash)).expect("hash"), hash);
        assert_eq!(
            optional_hash_from_vec(Some(hash_bytes(hash))).expect("optional hash"),
            Some(hash)
        );
        assert_eq!(optional_hash_from_vec(None).expect("none hash"), None);
        assert!(hash_from_vec(vec![0x01; 31]).is_err());
    }
}
