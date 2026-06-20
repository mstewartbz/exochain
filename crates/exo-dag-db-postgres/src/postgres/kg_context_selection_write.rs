//! Transactional Postgres write helpers for controlled graph context selection
//! usage events and context packet receipts (`M12`).

use std::collections::BTreeSet;

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    DagDbGraphContextPacket, DagDbGraphContextSelectionResponse, DagDbSelectedContextRef,
    DagDbSelectedGraphEdgeRef, MemoryEdgeKind, MemoryGraphStyle, ReceiptEventType, SafeMetadata,
    SafeMetadataDecision, SubjectKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::{
    graph_context_selection::MAX_SELECTED_GRAPH_EDGES_PER_PACKET,
    hash::{ReceiptHashMaterial, RequestHashMaterial},
    kg_import::{hash_from_hex, stable_hash},
    metadata::{MetadataField, sanitize_runtime_metadata},
    receipt::{
        OperationalReceiptInsert, insert_operational_receipt_in_transaction,
        operational_receipt_subject_id,
    },
    scoring::{DomainError, DomainResult, hash_event_body},
};

const CREATED_AT: Timestamp = Timestamp::new(1, 0);
const EXPIRES_AT: Timestamp = Timestamp::new(86_400_001, 0);
const WRITER_DID: &str = "did:exo:dagdb-context-selection-writer";

/// Route name for persisted usage-event writes.
pub const CONTEXT_SELECTION_USAGE_EVENT_ROUTE_NAME: &str =
    "dagdb.graph_context_selection.usage_event.v1";
/// Route name for persisted context-packet receipt writes.
pub const CONTEXT_SELECTION_PACKET_RECEIPT_ROUTE_NAME: &str =
    "dagdb.graph_context_selection.context_packet_receipt.v1";

/// Summary returned by controlled live DB mutation helpers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbWriteSummary {
    pub tenant_id: String,
    pub namespace: String,
    pub receipt_hash: String,
    pub inserted_rows: u32,
    pub replayed: bool,
}

/// Optional searchable metadata for the usage-event memory row created by a writeback.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEventMemoryMetadata {
    pub summary_text: Option<String>,
    /// Typed-knowledge class describing WHAT this memory is (`decision`,
    /// `finding`, `fix`, `constraint`, `handoff`). Persisted as a deterministic
    /// `knowledge:<class>` keyword so recall can search and surface the class.
    /// It travels at the same trust level as `summary_text` and never
    /// influences placement/organization.
    #[serde(default)]
    pub knowledge_class: Option<String>,
}

/// Deterministic searchable keyword tag prefix for a typed-knowledge class.
///
/// Recall surfaces the class by parsing this prefix from the persisted
/// keyword list, so the prefix is a stable part of the storage contract.
pub const KNOWLEDGE_CLASS_KEYWORD_PREFIX: &str = "knowledge:";

/// Build the deterministic keyword text that persists a knowledge class.
fn knowledge_class_keyword(class: &str) -> String {
    format!("{KNOWLEDGE_CLASS_KEYWORD_PREFIX}{class}")
}

/// Persist a graph context selection response as a scoped usage-event memory row.
pub async fn persist_usage_event_to_db(
    pool: &PgPool,
    event: &DagDbGraphContextSelectionResponse,
) -> DomainResult<DbWriteSummary> {
    persist_usage_event_to_db_with_metadata(pool, event, None).await
}

/// Persist a graph context selection response with optional searchable writeback metadata.
pub async fn persist_usage_event_to_db_with_metadata(
    pool: &PgPool,
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
) -> DomainResult<DbWriteSummary> {
    validate_selection_response(event)?;
    let idempotency_key = usage_event_idempotency_key(event)?;
    let request_hash = usage_event_request_hash(event, metadata)?;
    let receipt_hash = compute_usage_event_receipt_hash(event)?;

    let mut tx = pool.begin().await.map_err(pg)?;
    let result = persist_usage_event_in_transaction(
        &mut tx,
        event,
        metadata,
        &idempotency_key,
        request_hash,
        receipt_hash,
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
                    operation = "persist_usage_event_to_db",
                    tenant_id = %event.tenant_id,
                    namespace = %event.namespace,
                    route_name = %USAGE_EVENT_ROUTE,
                    request_id = %event.request_id,
                    idempotency_key = %idempotency_key,
                    error = %rollback_error,
                    "failed to rollback transaction after context selection usage-event persistence error"
                );
            }
            Err(error)
        }
    }
}

/// Persist a rendered context packet receipt as a scoped context-packet memory row.
pub async fn persist_context_packet_receipt_to_db(
    pool: &PgPool,
    packet: &DagDbGraphContextPacket,
) -> DomainResult<DbWriteSummary> {
    validate_context_packet(packet)?;
    let idempotency_key = context_packet_idempotency_key(packet)?;
    let request_hash = context_packet_request_hash(packet)?;
    let receipt_hash = compute_context_packet_receipt_hash(packet)?;

    let mut tx = pool.begin().await.map_err(pg)?;
    let result = persist_context_packet_in_transaction(
        &mut tx,
        packet,
        &idempotency_key,
        request_hash,
        receipt_hash,
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
                    operation = "persist_context_packet_receipt_to_db",
                    tenant_id = %packet.tenant_id,
                    namespace = %packet.namespace,
                    route_name = %PACKET_RECEIPT_ROUTE,
                    request_id = %packet.request_id,
                    idempotency_key = %idempotency_key,
                    error = %rollback_error,
                    "failed to rollback transaction after context packet receipt persistence error"
                );
            }
            Err(error)
        }
    }
}

/// Persist a usage event inside a caller-owned transaction.
///
/// The caller owns isolation level, commit, and rollback. This helper still
/// binds tenant context before touching tenant-scoped tables.
pub async fn persist_usage_event_to_db_with_metadata_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
) -> DomainResult<DbWriteSummary> {
    validate_selection_response(event)?;
    let idempotency_key = usage_event_idempotency_key(event)?;
    let request_hash = usage_event_request_hash(event, metadata)?;
    let receipt_hash = compute_usage_event_receipt_hash(event)?;

    super::bind_tenant_context(tx, &event.tenant_id)
        .await
        .map_err(pg)?;
    persist_usage_event_in_bound_transaction(
        tx,
        event,
        metadata,
        &idempotency_key,
        request_hash,
        receipt_hash,
    )
    .await
}

async fn persist_usage_event_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
    idempotency_key: &str,
    request_hash: Hash256,
    receipt_hash: Hash256,
) -> DomainResult<DbWriteSummary> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(pg)?;
    super::bind_tenant_context(tx, &event.tenant_id)
        .await
        .map_err(pg)?;
    persist_usage_event_in_bound_transaction(
        tx,
        event,
        metadata,
        idempotency_key,
        request_hash,
        receipt_hash,
    )
    .await
}

async fn persist_usage_event_in_bound_transaction(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
    idempotency_key: &str,
    request_hash: Hash256,
    receipt_hash: Hash256,
) -> DomainResult<DbWriteSummary> {
    if let Some(summary) =
        fetch_idempotency_replay(tx, event, idempotency_key, request_hash, USAGE_EVENT_ROUTE)
            .await?
    {
        return Ok(summary);
    }

    verify_selected_memories(tx, event).await?;
    verify_selected_graph_edges(
        tx,
        &event.tenant_id,
        &event.namespace,
        &selected_memory_id_set(&event.selected_memory_refs),
        &event.selected_graph_edges,
    )
    .await?;

    let memory_id = usage_event_memory_id(event)?;
    let inserted_rows =
        write_usage_event_rows(tx, event, metadata, memory_id, receipt_hash).await?;
    let summary = DbWriteSummary {
        tenant_id: event.tenant_id.clone(),
        namespace: event.namespace.clone(),
        receipt_hash: receipt_hash.to_string(),
        inserted_rows,
        replayed: false,
    };
    insert_idempotency_response(
        tx,
        &summary,
        idempotency_key,
        request_hash,
        USAGE_EVENT_ROUTE,
    )
    .await?;
    Ok(summary)
}

async fn persist_context_packet_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    packet: &DagDbGraphContextPacket,
    idempotency_key: &str,
    request_hash: Hash256,
    receipt_hash: Hash256,
) -> DomainResult<DbWriteSummary> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(pg)?;
    super::bind_tenant_context(tx, &packet.tenant_id)
        .await
        .map_err(pg)?;

    if let Some(summary) = fetch_packet_idempotency_replay(
        tx,
        packet,
        idempotency_key,
        request_hash,
        PACKET_RECEIPT_ROUTE,
    )
    .await?
    {
        return Ok(summary);
    }

    verify_packet_selected_memories(tx, packet).await?;
    verify_selected_graph_edges(
        tx,
        &packet.tenant_id,
        &packet.namespace,
        &selected_memory_id_set(&packet.selected_memory_refs),
        &packet.selected_graph_edges,
    )
    .await?;

    let subject_id = context_packet_subject_id(packet)?;
    let inserted_rows = write_context_packet_rows(tx, packet, subject_id, receipt_hash).await?;
    let summary = DbWriteSummary {
        tenant_id: packet.tenant_id.clone(),
        namespace: packet.namespace.clone(),
        receipt_hash: receipt_hash.to_string(),
        inserted_rows,
        replayed: false,
    };
    insert_idempotency_response(
        tx,
        &summary,
        idempotency_key,
        request_hash,
        PACKET_RECEIPT_ROUTE,
    )
    .await?;
    Ok(summary)
}

const USAGE_EVENT_ROUTE: &str = CONTEXT_SELECTION_USAGE_EVENT_ROUTE_NAME;
const PACKET_RECEIPT_ROUTE: &str = CONTEXT_SELECTION_PACKET_RECEIPT_ROUTE_NAME;

async fn fetch_idempotency_replay(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    idempotency_key: &str,
    request_hash: Hash256,
    route_name: &str,
) -> DomainResult<Option<DbWriteSummary>> {
    fetch_replay_by_scope(
        tx,
        &event.tenant_id,
        &event.namespace,
        route_name,
        idempotency_key,
        request_hash,
    )
    .await
}

async fn fetch_packet_idempotency_replay(
    tx: &mut Transaction<'_, Postgres>,
    packet: &DagDbGraphContextPacket,
    idempotency_key: &str,
    request_hash: Hash256,
    route_name: &str,
) -> DomainResult<Option<DbWriteSummary>> {
    fetch_replay_by_scope(
        tx,
        &packet.tenant_id,
        &packet.namespace,
        route_name,
        idempotency_key,
        request_hash,
    )
    .await
}

async fn fetch_replay_by_scope(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
) -> DomainResult<Option<DbWriteSummary>> {
    let row = sqlx::query(
        "SELECT request_hash, response_body FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4 \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;

    let Some(row) = row else {
        return Ok(None);
    };
    let existing_hash = hash_from_vec(row.try_get("request_hash").map_err(pg)?)?;
    if existing_hash != request_hash {
        return Err(DomainError::ValidationFailed);
    }
    let body: JsonValue = row.try_get("response_body").map_err(pg)?;
    let mut summary: DbWriteSummary =
        serde_json::from_value(body).map_err(|error| DomainError::HashMaterial {
            reason: format!("idempotency_response_json: {error}"),
        })?;
    summary.replayed = true;
    Ok(Some(summary))
}

async fn insert_idempotency_response(
    tx: &mut Transaction<'_, Postgres>,
    summary: &DbWriteSummary,
    idempotency_key: &str,
    request_hash: Hash256,
    route_name: &str,
) -> DomainResult<()> {
    let response_body = json_value(summary)?;
    let response_hash = hash_event_body(summary)?;
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
    .bind(route_name)
    .bind(idempotency_key)
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

async fn verify_selected_memories(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
) -> DomainResult<()> {
    for selected in &event.selected_memory_refs {
        ensure_memory_scope(tx, &event.tenant_id, &event.namespace, &selected.memory_id).await?;
    }
    Ok(())
}

async fn verify_packet_selected_memories(
    tx: &mut Transaction<'_, Postgres>,
    packet: &DagDbGraphContextPacket,
) -> DomainResult<()> {
    for selected in &packet.selected_memory_refs {
        ensure_memory_scope(
            tx,
            &packet.tenant_id,
            &packet.namespace,
            &selected.memory_id,
        )
        .await?;
    }
    Ok(())
}

fn selected_memory_id_set(selected: &[DagDbSelectedContextRef]) -> BTreeSet<String> {
    selected
        .iter()
        .map(|memory| memory.memory_id.clone())
        .collect()
}

async fn verify_selected_graph_edges(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    selected_memory_ids: &BTreeSet<String>,
    edges: &[DagDbSelectedGraphEdgeRef],
) -> DomainResult<()> {
    if edges.len() > MAX_SELECTED_GRAPH_EDGES_PER_PACKET {
        return Err(DomainError::ValidationFailed);
    }
    let mut seen_edge_ids = BTreeSet::new();
    for edge in edges {
        if !seen_edge_ids.insert(edge.graph_edge_id.clone()) {
            return Err(DomainError::ValidationFailed);
        }
        if !selected_memory_ids.contains(&edge.from_memory_id)
            || !selected_memory_ids.contains(&edge.to_memory_id)
        {
            return Err(DomainError::ValidationFailed);
        }
        ensure_graph_edge_scope(tx, tenant_id, namespace, edge).await?;
    }
    Ok(())
}

async fn ensure_graph_edge_scope(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    edge: &DagDbSelectedGraphEdgeRef,
) -> DomainResult<()> {
    let row = sqlx::query(
        "SELECT 1 \
         FROM dagdb_graph_edges edge \
         WHERE edge.graph_edge_id = $1 \
           AND edge.tenant_id = $2 \
           AND edge.namespace = $3 \
           AND edge.from_memory_id = $4 \
           AND edge.to_memory_id = $5 \
           AND edge.edge_kind = $6 \
           AND edge.graph_style = $7 \
           AND NOT EXISTS ( \
             SELECT 1 FROM dagdb_graph_edge_tombstones tombstone \
             WHERE tombstone.tenant_id = edge.tenant_id \
               AND tombstone.namespace = edge.namespace \
               AND tombstone.prior_edge_id = edge.graph_edge_id \
           )",
    )
    .bind(hash_bytes(
        hash_from_hex("graph_edge_id", &edge.graph_edge_id)
            .map_err(|_| DomainError::ValidationFailed)?,
    ))
    .bind(tenant_id)
    .bind(namespace)
    .bind(hash_bytes(
        hash_from_hex("edge.from_memory_id", &edge.from_memory_id)
            .map_err(|_| DomainError::ValidationFailed)?,
    ))
    .bind(hash_bytes(
        hash_from_hex("edge.to_memory_id", &edge.to_memory_id)
            .map_err(|_| DomainError::ValidationFailed)?,
    ))
    .bind(edge_kind_sql(edge.edge_kind))
    .bind(graph_style_sql(edge.graph_style))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;

    if row.is_some() {
        Ok(())
    } else {
        Err(DomainError::ValidationFailed)
    }
}

async fn ensure_memory_scope(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    memory_id: &str,
) -> DomainResult<()> {
    let row =
        sqlx::query("SELECT tenant_id, namespace FROM dagdb_memory_objects WHERE memory_id = $1")
            .bind(hash_bytes(
                hash_from_hex("memory_id", memory_id).map_err(|_| DomainError::ValidationFailed)?,
            ))
            .fetch_optional(&mut **tx)
            .await
            .map_err(pg)?;

    let Some(row) = row else {
        return Err(DomainError::ValidationFailed);
    };
    let stored_tenant: String = row.try_get("tenant_id").map_err(pg)?;
    let stored_namespace: String = row.try_get("namespace").map_err(pg)?;
    if stored_tenant == tenant_id && stored_namespace == namespace {
        Ok(())
    } else {
        Err(DomainError::TenantScopeMismatch {
            expected_tenant_id: stored_tenant,
            expected_namespace: stored_namespace,
            actual_tenant_id: tenant_id.to_owned(),
            actual_namespace: namespace.to_owned(),
        })
    }
}

async fn write_usage_event_rows(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
    memory_id: Hash256,
    receipt_hash: Hash256,
) -> DomainResult<u32> {
    let event_body_hash = hash_event_body(event)?;
    insert_usage_event_approval_receipts(tx, event, memory_id).await?;
    let record_accepted_rows =
        insert_usage_event_record_accepted_receipt(tx, event, memory_id).await?;
    let receipt_body = json!({
        "request_id": event.request_id,
        "task_hash": event.task_hash,
        "selection_status": event.selection_status,
        "selected_memory_ref_count": event.selected_memory_refs.len(),
        "source": "graph_context_selection_usage_event",
    });
    let receipt_rows = insert_receipt(
        tx,
        ReceiptInsert {
            tenant_id: &event.tenant_id,
            namespace: &event.namespace,
            subject_kind: SubjectKind::Memory,
            subject_id: memory_id,
            event_type: ReceiptEventType::IntakeCreated,
            event_body_hash,
            receipt_hash,
            receipt_body,
        },
    )
    .await?;
    let memory_rows = insert_usage_memory(tx, event, metadata, memory_id, receipt_hash).await?;
    Ok(record_accepted_rows
        .saturating_add(receipt_rows)
        .saturating_add(memory_rows))
}

async fn insert_usage_event_approval_receipts(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    memory_id: Hash256,
) -> DomainResult<u32> {
    let mut inserted = 0_u32;
    for event_type in [
        ReceiptEventType::DagdbApprovalRequestSubmitted,
        ReceiptEventType::DagdbApprovalGranted,
    ] {
        let receipt_body = json!({
            "route_name": USAGE_EVENT_ROUTE,
            "request_id": event.request_id,
            "task_hash": event.task_hash,
            "source": "graph_context_selection_usage_event",
        });
        let event_body_hash = hash_event_body(&receipt_body)?;
        let rows = insert_operational_receipt_in_transaction(
            tx,
            OperationalReceiptInsert {
                tenant_id: &event.tenant_id,
                namespace: &event.namespace,
                subject_kind: SubjectKind::Memory,
                subject_id: operational_receipt_subject_id(
                    USAGE_EVENT_ROUTE,
                    &memory_id.to_string(),
                    event_type,
                ),
                event_type,
                actor_did: WRITER_DID,
                event_hlc: CREATED_AT,
                event_body_hash,
                receipt_body,
            },
        )
        .await
        .map_err(|error| DomainError::HashMaterial {
            reason: error.to_string(),
        })?;
        inserted = inserted.saturating_add(rows_to_u32(rows)?);
    }
    Ok(inserted)
}

async fn insert_usage_event_record_accepted_receipt(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    memory_id: Hash256,
) -> DomainResult<u32> {
    let event_type = ReceiptEventType::DagdbRecordAccepted;
    let receipt_body = json!({
        "route_name": USAGE_EVENT_ROUTE,
        "request_id": event.request_id,
        "task_hash": event.task_hash,
        "selection_status": event.selection_status,
        "selected_memory_ref_count": event.selected_memory_refs.len(),
        "source": "graph_context_selection_usage_event",
    });
    let event_body_hash = hash_event_body(&receipt_body)?;
    let rows = insert_operational_receipt_in_transaction(
        tx,
        OperationalReceiptInsert {
            tenant_id: &event.tenant_id,
            namespace: &event.namespace,
            subject_kind: SubjectKind::Memory,
            subject_id: operational_receipt_subject_id(
                USAGE_EVENT_ROUTE,
                &memory_id.to_string(),
                event_type,
            ),
            event_type,
            actor_did: WRITER_DID,
            event_hlc: CREATED_AT,
            event_body_hash,
            receipt_body,
        },
    )
    .await
    .map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })?;
    rows_to_u32(rows)
}

async fn write_context_packet_rows(
    tx: &mut Transaction<'_, Postgres>,
    packet: &DagDbGraphContextPacket,
    subject_id: Hash256,
    receipt_hash: Hash256,
) -> DomainResult<u32> {
    let event_body_hash = hash_event_body(packet)?;
    let receipt_body = json!({
        "request_id": packet.request_id,
        "task_hash": packet.task_hash,
        "packet_hash": packet.packet_hash,
        "selected_memory_ref_count": packet.selected_memory_refs.len(),
        "source": "graph_context_packet_receipt",
    });
    let receipt_rows = insert_receipt(
        tx,
        ReceiptInsert {
            tenant_id: &packet.tenant_id,
            namespace: &packet.namespace,
            subject_kind: SubjectKind::ContextPacket,
            subject_id,
            event_type: ReceiptEventType::DagdbRecordAccepted,
            event_body_hash,
            receipt_hash,
            receipt_body,
        },
    )
    .await?;
    let memory_rows = insert_context_packet_memory(tx, packet, subject_id, receipt_hash).await?;
    Ok(receipt_rows.saturating_add(memory_rows))
}

struct ReceiptInsert<'a> {
    tenant_id: &'a str,
    namespace: &'a str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
    event_type: ReceiptEventType,
    event_body_hash: Hash256,
    receipt_hash: Hash256,
    receipt_body: JsonValue,
}

async fn insert_receipt(
    tx: &mut Transaction<'_, Postgres>,
    receipt: ReceiptInsert<'_>,
) -> DomainResult<u32> {
    let event_hlc = timestamp_parts(CREATED_AT)?;
    let receipt_result = sqlx::query(
        "INSERT INTO dagdb_receipts \
         (receipt_hash, tenant_id, namespace, subject_kind, subject_id, prev_receipt_hash, seq, \
          event_type, actor_did, event_hlc_physical_ms, event_hlc_logical, event_hash, receipt_body, \
          created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, $9, $10, $11, $12, $9, $10) \
         ON CONFLICT (receipt_hash) DO NOTHING",
    )
    .bind(hash_bytes(receipt.receipt_hash))
    .bind(receipt.tenant_id)
    .bind(receipt.namespace)
    .bind(subject_kind_sql(receipt.subject_kind))
    .bind(hash_bytes(receipt.subject_id))
    .bind(hash_bytes(Hash256::ZERO))
    .bind(event_type_sql(receipt.event_type))
    .bind(WRITER_DID)
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .bind(hash_bytes(receipt.event_body_hash))
    .bind(receipt.receipt_body)
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
    .bind(receipt.tenant_id)
    .bind(receipt.namespace)
    .bind(subject_kind_sql(receipt.subject_kind))
    .bind(hash_bytes(receipt.subject_id))
    .bind(hash_bytes(receipt.receipt_hash))
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;

    Ok(rows_to_u32(receipt_result.rows_affected())?
        .saturating_add(rows_to_u32(head_result.rows_affected())?))
}

async fn insert_usage_memory(
    tx: &mut Transaction<'_, Postgres>,
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
    memory_id: Hash256,
    latest_receipt_hash: Hash256,
) -> DomainResult<u32> {
    let payload_hash = hash_event_body(event)?;
    let source_hash =
        hash_from_hex("task_hash", &event.task_hash).map_err(|_| DomainError::ValidationFailed)?;
    let title = safe_metadata(
        &format!("usage event {}", event.request_id),
        &event.request_id,
    )?;
    let summary = usage_event_summary_metadata(event, metadata)?;
    let keywords = usage_event_keywords_metadata(metadata)?;
    let created_at = timestamp_parts(CREATED_AT)?;
    // PRD-D4: usage-event telemetry gets its own structural home (the dedicated
    // `usage_event` node_type) instead of masquerading as a knowledge
    // `excerpt`. Packet selection excludes this facet by structure, so the
    // read-side title-prefix heuristic is no longer the line of defense.
    let result = sqlx::query(
        "INSERT INTO dagdb_memory_objects \
         (memory_id, tenant_id, namespace, node_type, source_type, consent_purpose, payload_hash, source_hash, \
          owner_did, controller_did, submitted_by_did, title, summary, keywords, risk_class, risk_bp, \
          status, validation_status, council_status, dag_finality_status, latest_receipt_hash, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, 'usage_event', 'generated', 'retrieval', $4, $5, $6, $6, $6, $7, $8, $9, \
          'R1', 100, 'pending', 'pending', 'not_required', 'pending', $10, $11, $12, $11, $12) \
         ON CONFLICT (memory_id) DO NOTHING",
    )
    .bind(hash_bytes(memory_id))
    .bind(&event.tenant_id)
    .bind(&event.namespace)
    .bind(hash_bytes(payload_hash))
    .bind(hash_bytes(source_hash))
    .bind(WRITER_DID)
    .bind(json_value(&title)?)
    .bind(json_value(&summary)?)
    .bind(json_value(&keywords)?)
    .bind(hash_bytes(latest_receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

async fn insert_context_packet_memory(
    tx: &mut Transaction<'_, Postgres>,
    packet: &DagDbGraphContextPacket,
    memory_id: Hash256,
    latest_receipt_hash: Hash256,
) -> DomainResult<u32> {
    let payload_hash = hash_from_hex("packet_hash", &packet.packet_hash)
        .map_err(|_| DomainError::ValidationFailed)?;
    let source_hash =
        hash_from_hex("task_hash", &packet.task_hash).map_err(|_| DomainError::ValidationFailed)?;
    let title = safe_metadata(
        &format!("context packet {}", packet.request_id),
        &packet.request_id,
    )?;
    let summary = safe_metadata(
        &format!(
            "bounded packet with {} memory refs",
            packet.selected_memory_refs.len()
        ),
        &packet.packet_hash,
    )?;
    let created_at = timestamp_parts(CREATED_AT)?;
    let result = sqlx::query(
        "INSERT INTO dagdb_memory_objects \
         (memory_id, tenant_id, namespace, node_type, source_type, consent_purpose, payload_hash, source_hash, \
          owner_did, controller_did, submitted_by_did, title, summary, keywords, risk_class, risk_bp, \
          status, validation_status, council_status, dag_finality_status, latest_receipt_hash, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, 'context_packet', 'generated', 'retrieval', $4, $5, $6, $6, $6, $7, $8, '[]'::jsonb, \
          'R1', 100, 'pending', 'pending', 'not_required', 'pending', $9, $10, $11, $10, $11) \
         ON CONFLICT (memory_id) DO NOTHING",
    )
    .bind(hash_bytes(memory_id))
    .bind(&packet.tenant_id)
    .bind(&packet.namespace)
    .bind(hash_bytes(payload_hash))
    .bind(hash_bytes(source_hash))
    .bind(WRITER_DID)
    .bind(json_value(&title)?)
    .bind(json_value(&summary)?)
    .bind(hash_bytes(latest_receipt_hash))
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    rows_to_u32(result.rows_affected())
}

fn compute_usage_event_receipt_hash(
    event: &DagDbGraphContextSelectionResponse,
) -> DomainResult<Hash256> {
    let memory_id = usage_event_memory_id(event)?;
    let event_body_hash = hash_event_body(event)?;
    ReceiptHashMaterial {
        tenant_id: event.tenant_id.clone(),
        namespace: event.namespace.clone(),
        subject_kind: SubjectKind::Memory,
        subject_id: memory_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type: ReceiptEventType::IntakeCreated,
        actor_did: WRITER_DID.to_owned(),
        event_hlc: CREATED_AT,
        event_body_hash,
    }
    .hash()
    .map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

fn compute_context_packet_receipt_hash(packet: &DagDbGraphContextPacket) -> DomainResult<Hash256> {
    let subject_id = context_packet_subject_id(packet)?;
    let event_body_hash = hash_event_body(packet)?;
    ReceiptHashMaterial {
        tenant_id: packet.tenant_id.clone(),
        namespace: packet.namespace.clone(),
        subject_kind: SubjectKind::ContextPacket,
        subject_id,
        prev_receipt_hash: Hash256::ZERO,
        seq: 1,
        event_type: ReceiptEventType::DagdbRecordAccepted,
        actor_did: WRITER_DID.to_owned(),
        event_hlc: CREATED_AT,
        event_body_hash,
    }
    .hash()
    .map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

fn usage_event_idempotency_key(event: &DagDbGraphContextSelectionResponse) -> DomainResult<String> {
    Ok(stable_hash(
        "exo.dagdb.graph_context_selection.usage_event.idempotency_key",
        &[
            &event.tenant_id,
            &event.namespace,
            &event.request_id,
            &event.task_hash,
        ],
    )
    .map_err(import_error)?
    .to_string())
}

fn context_packet_idempotency_key(packet: &DagDbGraphContextPacket) -> DomainResult<String> {
    Ok(stable_hash(
        "exo.dagdb.graph_context_selection.context_packet_receipt.idempotency_key",
        &[
            &packet.tenant_id,
            &packet.namespace,
            &packet.request_id,
            &packet.packet_hash,
        ],
    )
    .map_err(import_error)?
    .to_string())
}

fn usage_event_request_hash(
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
) -> DomainResult<Hash256> {
    // The searchable `summary_text` and `knowledge_class` are persisted at the
    // same trust level as the event but travel out-of-band in `metadata`, so they
    // are not part of `serde_json::to_vec(event)`. Bind them into the idempotency
    // request hash (stable field order) so a replayed request that reuses the same
    // event but mutates the searchable metadata fails the idempotency conflict
    // check instead of silently overwriting the persisted summary.
    let request_body = UsageEventRequestBody {
        event,
        summary_text: metadata.and_then(|metadata| metadata.summary_text.as_deref()),
        knowledge_class: metadata.and_then(|metadata| metadata.knowledge_class.as_deref()),
    };
    RequestHashMaterial {
        route_name: USAGE_EVENT_ROUTE.to_owned(),
        tenant_id: event.tenant_id.clone(),
        namespace: event.namespace.clone(),
        canonical_redacted_request_body: serde_json::to_vec(&request_body).map_err(|error| {
            DomainError::HashMaterial {
                reason: format!("usage_event_request_json: {error}"),
            }
        })?,
    }
    .hash()
    .map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

/// Deterministic request-hash body that binds the out-of-band searchable
/// metadata (`summary_text`, `knowledge_class`) alongside the usage event.
#[derive(Serialize)]
struct UsageEventRequestBody<'a> {
    event: &'a DagDbGraphContextSelectionResponse,
    summary_text: Option<&'a str>,
    knowledge_class: Option<&'a str>,
}

fn context_packet_request_hash(packet: &DagDbGraphContextPacket) -> DomainResult<Hash256> {
    RequestHashMaterial {
        route_name: PACKET_RECEIPT_ROUTE.to_owned(),
        tenant_id: packet.tenant_id.clone(),
        namespace: packet.namespace.clone(),
        canonical_redacted_request_body: serde_json::to_vec(packet).map_err(|error| {
            DomainError::HashMaterial {
                reason: format!("context_packet_request_json: {error}"),
            }
        })?,
    }
    .hash()
    .map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

fn usage_event_memory_id(event: &DagDbGraphContextSelectionResponse) -> DomainResult<Hash256> {
    stable_hash(
        "exo.dagdb.graph_context_selection.usage_event.memory_id",
        &[
            &event.tenant_id,
            &event.namespace,
            &event.request_id,
            &event.task_hash,
        ],
    )
    .map_err(import_error)
}

fn context_packet_subject_id(packet: &DagDbGraphContextPacket) -> DomainResult<Hash256> {
    hash_from_hex("packet_hash", &packet.packet_hash).map_err(|_| DomainError::ValidationFailed)
}

fn validate_selection_response(event: &DagDbGraphContextSelectionResponse) -> DomainResult<()> {
    if event.tenant_id.trim().is_empty() || event.namespace.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    if event.request_id.trim().is_empty() || event.task_hash.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    hash_from_hex("task_hash", &event.task_hash).map_err(|_| DomainError::ValidationFailed)?;
    for selected in &event.selected_memory_refs {
        hash_from_hex("selected.memory_id", &selected.memory_id)
            .map_err(|_| DomainError::ValidationFailed)?;
    }
    for edge in &event.selected_graph_edges {
        validate_selected_graph_edge(edge)?;
    }
    Ok(())
}

fn validate_context_packet(packet: &DagDbGraphContextPacket) -> DomainResult<()> {
    if packet.tenant_id.trim().is_empty() || packet.namespace.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    if packet.request_id.trim().is_empty()
        || packet.task_hash.trim().is_empty()
        || packet.packet_hash.trim().is_empty()
    {
        return Err(DomainError::ValidationFailed);
    }
    hash_from_hex("task_hash", &packet.task_hash).map_err(|_| DomainError::ValidationFailed)?;
    hash_from_hex("packet_hash", &packet.packet_hash).map_err(|_| DomainError::ValidationFailed)?;
    for selected in &packet.selected_memory_refs {
        hash_from_hex("selected.memory_id", &selected.memory_id)
            .map_err(|_| DomainError::ValidationFailed)?;
    }
    for edge in &packet.selected_graph_edges {
        validate_selected_graph_edge(edge)?;
    }
    Ok(())
}

fn validate_selected_graph_edge(edge: &DagDbSelectedGraphEdgeRef) -> DomainResult<()> {
    hash_from_hex("selected.graph_edge_id", &edge.graph_edge_id)
        .map_err(|_| DomainError::ValidationFailed)?;
    hash_from_hex("selected.from_memory_id", &edge.from_memory_id)
        .map_err(|_| DomainError::ValidationFailed)?;
    hash_from_hex("selected.to_memory_id", &edge.to_memory_id)
        .map_err(|_| DomainError::ValidationFailed)?;
    if edge.selection_reason.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    Ok(())
}

fn safe_metadata(text: &str, original_hash: &str) -> DomainResult<SafeMetadata> {
    Ok(SafeMetadata {
        decision: SafeMetadataDecision::Allow,
        text: text.to_owned(),
        redaction_codes: Vec::new(),
        original_hash: original_hash.to_owned(),
        truncated: false,
        byte_len: u32::try_from(text.len()).map_err(|_| DomainError::ValidationFailed)?,
    })
}

fn usage_event_summary_metadata(
    event: &DagDbGraphContextSelectionResponse,
    metadata: Option<&UsageEventMemoryMetadata>,
) -> DomainResult<SafeMetadata> {
    if let Some(summary_text) = metadata
        .and_then(|metadata| metadata.summary_text.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return sanitize_runtime_metadata(MetadataField::Summary, summary_text)
            .map_err(metadata_error);
    }
    safe_metadata(
        &format!(
            "selected {} memory refs for task {}",
            event.selected_memory_refs.len(),
            event.task_hash
        ),
        &event.task_hash,
    )
}

/// Build the keyword list persisted for a usage-event memory row.
///
/// When the writeback metadata carries a typed-knowledge class, the class is
/// persisted as a single deterministic `knowledge:<class>` keyword so recall
/// can search and surface it. Classless writebacks persist no keywords, exactly
/// as before. The class describes WHAT the memory is and is never consulted for
/// placement/organization.
fn usage_event_keywords_metadata(
    metadata: Option<&UsageEventMemoryMetadata>,
) -> DomainResult<Vec<SafeMetadata>> {
    let Some(class) = metadata
        .and_then(|metadata| metadata.knowledge_class.as_deref())
        .map(str::trim)
        .filter(|class| !class.is_empty())
    else {
        return Ok(Vec::new());
    };
    let keyword =
        sanitize_runtime_metadata(MetadataField::Keyword, &knowledge_class_keyword(class))
            .map_err(metadata_error)?;
    Ok(vec![keyword])
}

fn metadata_error(error: crate::metadata::MetadataError) -> DomainError {
    DomainError::HashMaterial {
        reason: format!("usage_event_metadata: {error}"),
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
        ReceiptEventType::DagdbApprovalRequestSubmitted => "dagdb_approval_request_submitted",
        ReceiptEventType::DagdbApprovalGranted => "dagdb_approval_granted",
        ReceiptEventType::DagdbApprovalDenied => "dagdb_approval_denied",
        ReceiptEventType::DagdbRecordAccepted => "dagdb_record_accepted",
        ReceiptEventType::DagdbImportCompleted => "dagdb_import_completed",
        ReceiptEventType::DagdbExportCompleted => "dagdb_export_completed",
        ReceiptEventType::DagdbReplayDetected => "dagdb_replay_detected",
        ReceiptEventType::DagdbIdempotencyConflict => "dagdb_idempotency_conflict",
        ReceiptEventType::DagdbRlsTenantViolation => "dagdb_rls_tenant_violation",
        ReceiptEventType::DagdbSignatureFailure => "dagdb_signature_failure",
        ReceiptEventType::DagdbCouncilOperatorDecision => "dagdb_council_operator_decision",
    }
}

fn graph_style_sql(graph_style: MemoryGraphStyle) -> &'static str {
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

fn json_value<T: Serialize>(value: &T) -> DomainResult<JsonValue> {
    serde_json::to_value(value).map_err(|error| DomainError::HashMaterial {
        reason: format!("json_value: {error}"),
    })
}

#[derive(Debug, Clone, Copy)]
struct SqlTimestamp {
    physical_ms: i64,
    logical: i32,
}

fn timestamp_parts(timestamp: Timestamp) -> DomainResult<SqlTimestamp> {
    Ok(SqlTimestamp {
        physical_ms: i64::try_from(timestamp.physical_ms)
            .map_err(|_| DomainError::ValidationFailed)?,
        logical: i32::try_from(timestamp.logical).map_err(|_| DomainError::ValidationFailed)?,
    })
}

fn rows_to_u32(rows: u64) -> DomainResult<u32> {
    u32::try_from(rows).map_err(|_| DomainError::ValidationFailed)
}

fn hash_bytes(hash: Hash256) -> Vec<u8> {
    hash.as_bytes().to_vec()
}

fn hash_from_vec(bytes: Vec<u8>) -> DomainResult<Hash256> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| DomainError::ValidationFailed)?;
    Ok(Hash256::from_bytes(bytes))
}

fn pg(source: sqlx::Error) -> DomainError {
    DomainError::HashMaterial {
        reason: format!("graph_context_selection_write_postgres: {source}"),
    }
}

fn import_error(error: crate::kg_import::KgImportError) -> DomainError {
    DomainError::HashMaterial {
        reason: error.to_string(),
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use exo_core::Hash256;
    use exo_dag_db_api::{
        DagDbGraphContextPacket, DagDbGraphContextSelectionResponse,
        DagDbGraphContextSelectionStatus, DagDbSelectedContextRef, ReceiptEventType, SafeMetadata,
        SafeMetadataDecision, SubjectKind, ValidationStatus,
    };

    use super::*;

    fn sample_event() -> DagDbGraphContextSelectionResponse {
        DagDbGraphContextSelectionResponse {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-usage-1".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            selection_status: DagDbGraphContextSelectionStatus::Selected,
            selected_memory_refs: vec![DagDbSelectedContextRef {
                memory_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into(),
                catalog_id: None,
                title: SafeMetadata {
                    decision: SafeMetadataDecision::Allow,
                    text: "title".into(),
                    redaction_codes: Vec::new(),
                    original_hash:
                        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
                    truncated: false,
                    byte_len: 5,
                },
                summary: SafeMetadata {
                    decision: SafeMetadataDecision::Allow,
                    text: "summary".into(),
                    redaction_codes: Vec::new(),
                    original_hash:
                        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into(),
                    truncated: false,
                    byte_len: 7,
                },
                catalog_path: vec!["dag-db".into()],
                document_type: "summary".into(),
                selection_reason: "fixture".into(),
                token_estimate: 100,
                validation_status: ValidationStatus::Pending,
                citation_ref: "citation:fixture".into(),
                boundary_flags: vec!["repository_test_only".into()],
            }],
            selected_graph_edges: Vec::new(),
            omitted_memory_refs: Vec::new(),
            selection_trace: Vec::new(),
            selected_token_estimate: 100,
            token_budget: 1_000,
            boundary_warnings: Vec::new(),
        }
    }

    #[test]
    fn usage_event_receipt_hash_is_deterministic() {
        let event = sample_event();
        let first = compute_usage_event_receipt_hash(&event).expect("first hash");
        let second = compute_usage_event_receipt_hash(&event).expect("second hash");
        assert_eq!(first, second);
    }

    #[test]
    fn usage_event_idempotency_key_is_deterministic() {
        let event = sample_event();
        let first = usage_event_idempotency_key(&event).expect("first key");
        let second = usage_event_idempotency_key(&event).expect("second key");
        assert_eq!(first, second);
    }

    fn sample_packet() -> DagDbGraphContextPacket {
        let event = sample_event();
        DagDbGraphContextPacket {
            schema_version: "dagdb.graph_context_packet.v1".into(),
            tenant_id: event.tenant_id.clone(),
            namespace: event.namespace.clone(),
            request_id: event.request_id.clone(),
            task: "Build packet".into(),
            task_hash: event.task_hash.clone(),
            packet_hash: compute_usage_event_receipt_hash(&event)
                .expect("hash")
                .to_string(),
            selected_memory_refs: event.selected_memory_refs.clone(),
            selected_graph_edges: Vec::new(),
            citation_refs: Vec::new(),
            packet_metrics: exo_dag_db_api::DagDbContextPacketMetrics {
                token_budget: event.token_budget,
                selected_token_estimate: event.selected_token_estimate,
                selected_memory_ref_count: 1,
                selected_graph_edge_count: 0,
                citation_ref_count: 0,
                end_to_end_savings_status: "blocked".into(),
                cost_savings_status: "blocked".into(),
            },
            boundaries: exo_dag_db_api::DagDbContextPacketBoundaries {
                repository_test_level_only: true,
                production_runtime: "blocked".into(),
                default_context_replacement: "blocked".into(),
                citation_locator_status: "omitted_citation_locator_blocked".into(),
                billing_savings: "blocked".into(),
            },
            agent_usage_instructions: Vec::new(),
            markdown: "# packet".into(),
        }
    }

    #[test]
    fn context_packet_receipt_hash_and_idempotency_are_deterministic() {
        let packet = sample_packet();
        let first_hash = compute_context_packet_receipt_hash(&packet).expect("first hash");
        let second_hash = compute_context_packet_receipt_hash(&packet).expect("second hash");
        assert_eq!(first_hash, second_hash);
        let first_key = context_packet_idempotency_key(&packet).expect("first key");
        let second_key = context_packet_idempotency_key(&packet).expect("second key");
        assert_eq!(first_key, second_key);
    }

    #[test]
    fn request_hashes_and_memory_ids_are_deterministic() {
        let event = sample_event();
        let first_request = usage_event_request_hash(&event, None).expect("first request hash");
        let second_request = usage_event_request_hash(&event, None).expect("second request hash");
        assert_eq!(first_request, second_request);
        let first_memory = usage_event_memory_id(&event).expect("first memory id");
        let second_memory = usage_event_memory_id(&event).expect("second memory id");
        assert_eq!(first_memory, second_memory);

        let packet = sample_packet();
        let first_packet_request = context_packet_request_hash(&packet).expect("packet request");
        let second_packet_request = context_packet_request_hash(&packet).expect("packet request");
        assert_eq!(first_packet_request, second_packet_request);
        let first_subject = context_packet_subject_id(&packet).expect("subject id");
        let second_subject = context_packet_subject_id(&packet).expect("subject id");
        assert_eq!(first_subject, second_subject);
    }

    #[test]
    fn validation_helpers_reject_empty_scope_and_invalid_hashes() {
        let mut event = sample_event();
        event.tenant_id.clear();
        assert_eq!(
            validate_selection_response(&event),
            Err(DomainError::ValidationFailed)
        );

        let mut packet = sample_packet();
        packet.packet_hash = "not-a-hash".into();
        assert_eq!(
            validate_context_packet(&packet),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn sql_mapping_helpers_cover_all_variants() {
        assert_eq!(subject_kind_sql(SubjectKind::Memory), "memory");
        assert_eq!(
            subject_kind_sql(SubjectKind::ContextPacket),
            "context_packet"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::ContextPacketCreated),
            "context_packet_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagFinalityCompensated),
            "dag_finality_compensated"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbApprovalRequestSubmitted),
            "dagdb_approval_request_submitted"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbApprovalGranted),
            "dagdb_approval_granted"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbApprovalDenied),
            "dagdb_approval_denied"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbRecordAccepted),
            "dagdb_record_accepted"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbImportCompleted),
            "dagdb_import_completed"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbExportCompleted),
            "dagdb_export_completed"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbReplayDetected),
            "dagdb_replay_detected"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbIdempotencyConflict),
            "dagdb_idempotency_conflict"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbRlsTenantViolation),
            "dagdb_rls_tenant_violation"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbSignatureFailure),
            "dagdb_signature_failure"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagdbCouncilOperatorDecision),
            "dagdb_council_operator_decision"
        );
    }

    #[test]
    fn timestamp_and_row_helpers_convert_deterministically() {
        let parts = timestamp_parts(CREATED_AT).expect("timestamp parts");
        assert_eq!(parts.physical_ms, 1);
        assert_eq!(parts.logical, 0);
        assert_eq!(rows_to_u32(3).expect("rows"), 3);
        let hash = Hash256::from_bytes([0xab; 32]);
        assert_eq!(hash_bytes(hash), hash.as_bytes().to_vec());
        assert_eq!(
            hash_from_vec(hash.as_bytes().to_vec()).expect("hash from vec"),
            hash
        );
    }

    #[test]
    fn safe_metadata_and_json_value_helpers_build_structures() {
        let metadata = safe_metadata(
            "summary",
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        )
        .expect("metadata");
        assert_eq!(metadata.text, "summary");
        let value = json_value(&metadata).expect("json value");
        assert!(value.is_object());
    }

    #[test]
    fn usage_event_summary_metadata_prefers_safe_writeback_summary() {
        let event = sample_event();
        let metadata = UsageEventMemoryMetadata {
            summary_text: Some(
                "PRD28 active-use planning writeback retained searchable task context".to_owned(),
            ),
            knowledge_class: None,
        };
        let summary =
            usage_event_summary_metadata(&event, Some(&metadata)).expect("summary metadata");
        assert_eq!(
            summary.text,
            "PRD28 active-use planning writeback retained searchable task context"
        );
        assert_eq!(summary.decision, SafeMetadataDecision::Allow);
    }

    #[test]
    fn usage_event_summary_metadata_falls_back_to_selection_summary() {
        let event = sample_event();
        let summary = usage_event_summary_metadata(&event, None).expect("summary metadata");
        assert!(summary.text.contains("selected 1 memory refs for task"));
        assert!(summary.text.contains(&event.task_hash));
    }

    #[test]
    fn usage_event_summary_metadata_rejects_raw_code_material() {
        let event = sample_event();
        let metadata = UsageEventMemoryMetadata {
            summary_text: Some("fn raw_payload() {}".to_owned()),
            knowledge_class: None,
        };
        assert!(usage_event_summary_metadata(&event, Some(&metadata)).is_err());
    }

    #[test]
    fn usage_event_keywords_metadata_persists_knowledge_class_tag() {
        let metadata = UsageEventMemoryMetadata {
            summary_text: Some("Recorded a finding from the implementation".to_owned()),
            knowledge_class: Some("finding".to_owned()),
        };
        let keywords =
            usage_event_keywords_metadata(Some(&metadata)).expect("knowledge class keyword");
        assert_eq!(keywords.len(), 1);
        assert_eq!(keywords[0].text, "knowledge:finding");
        assert_eq!(keywords[0].decision, SafeMetadataDecision::Allow);
    }

    #[test]
    fn usage_event_keywords_metadata_is_empty_without_class() {
        assert!(
            usage_event_keywords_metadata(None)
                .expect("no metadata keywords")
                .is_empty()
        );
        let classless = UsageEventMemoryMetadata {
            summary_text: Some("Telemetry only".to_owned()),
            knowledge_class: None,
        };
        assert!(
            usage_event_keywords_metadata(Some(&classless))
                .expect("classless keywords")
                .is_empty()
        );
        let blank = UsageEventMemoryMetadata {
            summary_text: None,
            knowledge_class: Some("   ".to_owned()),
        };
        assert!(
            usage_event_keywords_metadata(Some(&blank))
                .expect("blank class keywords")
                .is_empty()
        );
    }

    #[test]
    fn usage_event_request_hash_binds_searchable_metadata() {
        let event = sample_event();
        let baseline = usage_event_request_hash(&event, None).expect("baseline request hash");

        let with_summary = UsageEventMemoryMetadata {
            summary_text: Some("original searchable summary".to_owned()),
            knowledge_class: None,
        };
        let summary_hash =
            usage_event_request_hash(&event, Some(&with_summary)).expect("summary request hash");
        assert_ne!(
            baseline, summary_hash,
            "binding a summary must change the idempotency request hash"
        );

        // Mutating only the summary (same event, same idempotency key) must change
        // the request hash so a replayed signature cannot poison the persisted
        // searchable summary.
        let mutated_summary = UsageEventMemoryMetadata {
            summary_text: Some("attacker-mutated searchable summary".to_owned()),
            knowledge_class: None,
        };
        let mutated_hash =
            usage_event_request_hash(&event, Some(&mutated_summary)).expect("mutated request hash");
        assert_ne!(
            summary_hash, mutated_hash,
            "changing summary_text must change the request hash"
        );

        // The same idempotency key proves the binding is the request hash, not the key.
        assert_eq!(
            usage_event_idempotency_key(&event).expect("key for summary"),
            usage_event_idempotency_key(&event).expect("key for mutated"),
        );

        // knowledge_class is bound independently of summary_text.
        let with_class = UsageEventMemoryMetadata {
            summary_text: Some("original searchable summary".to_owned()),
            knowledge_class: Some("finding".to_owned()),
        };
        let class_hash =
            usage_event_request_hash(&event, Some(&with_class)).expect("class request hash");
        assert_ne!(
            summary_hash, class_hash,
            "changing knowledge_class must change the request hash"
        );
    }

    #[test]
    fn hash_from_vec_rejects_invalid_length() {
        assert_eq!(
            hash_from_vec(vec![0u8; 31]),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn subject_kind_sql_maps_every_variant() {
        use exo_dag_db_api::SubjectKind;

        assert_eq!(subject_kind_sql(SubjectKind::Memory), "memory");
        assert_eq!(subject_kind_sql(SubjectKind::Catalog), "catalog");
        assert_eq!(subject_kind_sql(SubjectKind::Route), "route");
        assert_eq!(
            subject_kind_sql(SubjectKind::ContextPacket),
            "context_packet"
        );
        assert_eq!(
            subject_kind_sql(SubjectKind::ValidationReport),
            "validation_report"
        );
        assert_eq!(
            subject_kind_sql(SubjectKind::AgentSafetyScore),
            "agent_safety_score"
        );
        assert_eq!(
            subject_kind_sql(SubjectKind::InboundAgentCredential),
            "inbound_agent_credential"
        );
        assert_eq!(
            subject_kind_sql(SubjectKind::CouncilDecision),
            "council_decision"
        );
    }

    #[test]
    fn event_type_sql_maps_every_variant() {
        use exo_dag_db_api::ReceiptEventType;

        assert_eq!(
            event_type_sql(ReceiptEventType::IntakeCreated),
            "intake_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DuplicateRejected),
            "duplicate_rejected"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::ValidationCreated),
            "validation_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::ValidationPassed),
            "validation_passed"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::ValidationFailed),
            "validation_failed"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::MemoryApproved),
            "memory_approved"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::MemoryRoutable),
            "memory_routable"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::MemoryRevoked),
            "memory_revoked"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::MemorySuperseded),
            "memory_superseded"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::RouteCreated),
            "route_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::RouteActivated),
            "route_activated"
        );
        assert_eq!(event_type_sql(ReceiptEventType::RouteStale), "route_stale");
        assert_eq!(
            event_type_sql(ReceiptEventType::RouteInvalidated),
            "route_invalidated"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::ContextPacketCreated),
            "context_packet_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::WritebackCreated),
            "writeback_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::TrustCheckCreated),
            "trust_check_created"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::CouncilDecisionRecorded),
            "council_decision_recorded"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagFinalityCommitted),
            "dag_finality_committed"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagFinalityFailed),
            "dag_finality_failed"
        );
        assert_eq!(
            event_type_sql(ReceiptEventType::DagFinalityCompensated),
            "dag_finality_compensated"
        );
    }

    #[test]
    fn validation_helpers_reject_empty_request_and_task_fields() {
        let mut event = sample_event();
        event.request_id.clear();
        assert_eq!(
            validate_selection_response(&event),
            Err(DomainError::ValidationFailed)
        );

        let mut packet = sample_packet();
        packet.task_hash.clear();
        assert_eq!(
            validate_context_packet(&packet),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn validation_helpers_reject_empty_packet_scope_fields() {
        let mut packet = sample_packet();
        packet.namespace.clear();
        assert_eq!(
            validate_context_packet(&packet),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn rows_to_u32_rejects_overflow() {
        assert_eq!(
            rows_to_u32(u64::from(u32::MAX) + 1),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn timestamp_parts_reject_overflow() {
        assert!(timestamp_parts(Timestamp::new(u64::MAX, 0)).is_err());
    }

    #[test]
    fn validation_helpers_reject_invalid_selected_memory_id() {
        let mut event = sample_event();
        event.selected_memory_refs[0].memory_id = "not-a-hash".into();
        assert_eq!(
            validate_selection_response(&event),
            Err(DomainError::ValidationFailed)
        );

        let mut packet = sample_packet();
        packet.selected_memory_refs[0].memory_id = "not-a-hash".into();
        assert_eq!(
            validate_context_packet(&packet),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn pg_and_import_error_helpers_wrap_sources() {
        let pg_error = pg(sqlx::Error::RowNotFound);
        assert!(matches!(pg_error, DomainError::HashMaterial { .. }));
        let import_error = import_error(crate::kg_import::KgImportError::InvalidHash {
            field: "task_hash".into(),
        });
        assert!(matches!(import_error, DomainError::HashMaterial { .. }));
    }
}
