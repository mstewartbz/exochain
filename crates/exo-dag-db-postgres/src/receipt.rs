//! PostgreSQL-backed DAG DB receipt append and reconstruction.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{ReceiptEventType, SubjectKind};
use sqlx::{PgPool, Postgres, Row, Transaction, types::JsonValue};
use thiserror::Error;

use crate::hash::ReceiptHashMaterial;

/// Receipt append request material stored in `dagdb_receipts`.
#[derive(Debug, Clone)]
pub struct ReceiptAppendRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub expected_prev_receipt_hash: Hash256,
    pub event_type: ReceiptEventType,
    pub actor_did: String,
    pub event_hlc: Timestamp,
    pub event_body_hash: Hash256,
    pub receipt_body: JsonValue,
}

/// Result of appending or replaying a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptAppendResult {
    pub receipt_hash: Hash256,
    pub prev_receipt_hash: Hash256,
    pub seq: u64,
    pub created_new: bool,
}

/// Receipt row reconstructed from the latest subject head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptRecord {
    pub receipt_hash: Hash256,
    pub prev_receipt_hash: Hash256,
    pub seq: u64,
}

#[derive(Debug, Clone, Copy)]
struct ReceiptHead {
    latest_receipt_hash: Hash256,
    latest_seq: u64,
}

/// Errors from receipt storage.
#[derive(Debug, Error)]
pub enum ReceiptStoreError {
    #[error("stale_previous_receipt_hash")]
    StalePreviousReceiptHash,
    #[error("receipt_chain_not_found")]
    ReceiptChainNotFound,
    #[error("receipt_chain_broken")]
    ReceiptChainBroken,
    #[error("receipt hash column had invalid length")]
    InvalidHashLength,
    #[error("receipt timestamp is out of SQL range")]
    TimestampOutOfRange,
    #[error("receipt sequence is out of range")]
    SequenceOutOfRange,
    #[error("receipt hash serialization failed: {0}")]
    Hash(String),
    #[error("postgres receipt operation failed")]
    Postgres {
        #[source]
        source: sqlx::Error,
    },
}

/// Receipt storage result alias.
pub type Result<T> = std::result::Result<T, ReceiptStoreError>;

/// Append a receipt with per-subject compare-and-set semantics.
pub async fn append_receipt(
    pool: &PgPool,
    request: &ReceiptAppendRequest,
) -> Result<ReceiptAppendResult> {
    let first = append_receipt_once(pool, request).await;
    if matches!(&first, Err(error) if is_serialization_failure(error)) {
        return append_receipt_once(pool, request).await;
    }
    first
}

async fn append_receipt_once(
    pool: &PgPool,
    request: &ReceiptAppendRequest,
) -> Result<ReceiptAppendResult> {
    let mut tx = pool.begin().await.map_err(pg)?;
    let result = append_receipt_in_transaction(&mut tx, request).await;
    match result {
        Ok(result) => {
            tx.commit().await.map_err(pg)?;
            Ok(result)
        }
        Err(error) => {
            let _ = tx.rollback().await;
            Err(error)
        }
    }
}

async fn append_receipt_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
) -> Result<ReceiptAppendResult> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(pg)?;
    lock_subject(tx, request).await?;

    let head = fetch_head(tx, request).await?;
    let result = match head {
        None => append_genesis(tx, request).await?,
        Some(head) => append_after_head(tx, request, head).await?,
    };

    Ok(result)
}

fn is_serialization_failure(error: &ReceiptStoreError) -> bool {
    match error {
        ReceiptStoreError::Postgres {
            source: sqlx::Error::Database(database_error),
        } => database_error.code().as_deref() == Some("40001"),
        _ => false,
    }
}

/// Reconstruct a subject receipt chain from its current head.
pub async fn reconstruct_receipt_chain(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> Result<Vec<ReceiptRecord>> {
    let subject_kind = subject_kind_sql(subject_kind);
    let subject_id_bytes = hash_bytes(subject_id);
    let row = sqlx::query(
        "SELECT latest_receipt_hash FROM dagdb_subject_receipt_heads \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = $3 AND subject_id = $4",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(subject_kind)
    .bind(subject_id_bytes)
    .fetch_optional(pool)
    .await
    .map_err(pg)?
    .ok_or(ReceiptStoreError::ReceiptChainNotFound)?;

    let mut current = hash_from_vec(row.try_get("latest_receipt_hash").map_err(pg)?)?;
    let mut records = Vec::new();
    for _ in 0..10_000_u32 {
        let row = sqlx::query(
            "SELECT receipt_hash, prev_receipt_hash, seq FROM dagdb_receipts \
             WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = $3 \
             AND subject_id = $4 AND receipt_hash = $5",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(subject_kind)
        .bind(hash_bytes(subject_id))
        .bind(hash_bytes(current))
        .fetch_optional(pool)
        .await
        .map_err(pg)?
        .ok_or(ReceiptStoreError::ReceiptChainBroken)?;

        let record = ReceiptRecord {
            receipt_hash: hash_from_vec(row.try_get("receipt_hash").map_err(pg)?)?,
            prev_receipt_hash: hash_from_vec(row.try_get("prev_receipt_hash").map_err(pg)?)?,
            seq: seq_from_i64(row.try_get("seq").map_err(pg)?)?,
        };
        current = record.prev_receipt_hash;
        let is_genesis = current == Hash256::ZERO;
        records.push(record);
        if is_genesis {
            records.reverse();
            return Ok(records);
        }
    }

    Err(ReceiptStoreError::ReceiptChainBroken)
}

/// Advisory lock key from the Slice 4 subject material contract.
#[must_use]
pub fn advisory_lock_key(
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> i64 {
    let mut material = Vec::new();
    material.extend_from_slice(tenant_id.as_bytes());
    material.push(0);
    material.extend_from_slice(namespace.as_bytes());
    material.push(0);
    material.extend_from_slice(subject_kind_sql(subject_kind).as_bytes());
    material.push(0);
    material.extend_from_slice(subject_id.to_string().as_bytes());
    let digest = Hash256::digest(&material);
    let mut key = [0_u8; 8];
    key.copy_from_slice(&digest.as_bytes()[..8]);
    i64::from_be_bytes(key)
}

async fn append_genesis(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
) -> Result<ReceiptAppendResult> {
    if request.expected_prev_receipt_hash != Hash256::ZERO {
        return Err(ReceiptStoreError::StalePreviousReceiptHash);
    }
    let result = insert_receipt(tx, request, Hash256::ZERO, 1).await?;
    insert_head(tx, request, result.receipt_hash, 1).await?;
    Ok(result)
}

async fn append_after_head(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
    head: ReceiptHead,
) -> Result<ReceiptAppendResult> {
    if request.expected_prev_receipt_hash != head.latest_receipt_hash {
        if let Some(replay) = find_existing_event_receipt(tx, request).await? {
            return Ok(replay);
        }
        return Err(ReceiptStoreError::StalePreviousReceiptHash);
    }

    let next_seq = head
        .latest_seq
        .checked_add(1)
        .ok_or(ReceiptStoreError::SequenceOutOfRange)?;
    let result = insert_receipt(tx, request, head.latest_receipt_hash, next_seq).await?;
    update_head(
        tx,
        request,
        head.latest_receipt_hash,
        result.receipt_hash,
        next_seq,
    )
    .await?;
    Ok(result)
}

async fn lock_subject(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
) -> Result<()> {
    let key = advisory_lock_key(
        &request.tenant_id,
        &request.namespace,
        request.subject_kind,
        request.subject_id,
    );
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key)
        .execute(&mut **tx)
        .await
        .map_err(pg)?;
    Ok(())
}

async fn fetch_head(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
) -> Result<Option<ReceiptHead>> {
    let row = sqlx::query(
        "SELECT latest_receipt_hash, latest_seq FROM dagdb_subject_receipt_heads \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = $3 AND subject_id = $4 \
         FOR UPDATE",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(subject_kind_sql(request.subject_kind))
    .bind(hash_bytes(request.subject_id))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;

    row.map(|row| {
        Ok(ReceiptHead {
            latest_receipt_hash: hash_from_vec(row.try_get("latest_receipt_hash").map_err(pg)?)?,
            latest_seq: seq_from_i64(row.try_get("latest_seq").map_err(pg)?)?,
        })
    })
    .transpose()
}

async fn insert_receipt(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
    prev_receipt_hash: Hash256,
    seq: u64,
) -> Result<ReceiptAppendResult> {
    let event_hlc = timestamp_parts(request.event_hlc)?;
    let seq_sql = i64::try_from(seq).map_err(|_| ReceiptStoreError::SequenceOutOfRange)?;
    let receipt_hash = ReceiptHashMaterial {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        subject_kind: request.subject_kind,
        subject_id: request.subject_id,
        prev_receipt_hash,
        seq,
        event_type: request.event_type,
        actor_did: request.actor_did.clone(),
        event_hlc: request.event_hlc,
        event_body_hash: request.event_body_hash,
    }
    .hash()
    .map_err(|err| ReceiptStoreError::Hash(err.to_string()))?;

    sqlx::query(
        "INSERT INTO dagdb_receipts \
         (receipt_hash, tenant_id, namespace, subject_kind, subject_id, prev_receipt_hash, seq, \
          event_type, actor_did, event_hlc_physical_ms, event_hlc_logical, event_hash, receipt_body, \
          created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $10, $11)",
    )
    .bind(hash_bytes(receipt_hash))
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(subject_kind_sql(request.subject_kind))
    .bind(hash_bytes(request.subject_id))
    .bind(hash_bytes(prev_receipt_hash))
    .bind(seq_sql)
    .bind(event_type_sql(request.event_type))
    .bind(&request.actor_did)
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .bind(hash_bytes(request.event_body_hash))
    .bind(request.receipt_body.clone())
    .execute(&mut **tx)
    .await
    .map_err(pg)?;

    Ok(ReceiptAppendResult {
        receipt_hash,
        prev_receipt_hash,
        seq,
        created_new: true,
    })
}

async fn insert_head(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
    receipt_hash: Hash256,
    seq: u64,
) -> Result<()> {
    let event_hlc = timestamp_parts(request.event_hlc)?;
    let seq_sql = i64::try_from(seq).map_err(|_| ReceiptStoreError::SequenceOutOfRange)?;
    sqlx::query(
        "INSERT INTO dagdb_subject_receipt_heads \
         (tenant_id, namespace, subject_kind, subject_id, latest_receipt_hash, latest_seq, \
          updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(subject_kind_sql(request.subject_kind))
    .bind(hash_bytes(request.subject_id))
    .bind(hash_bytes(receipt_hash))
    .bind(seq_sql)
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    Ok(())
}

async fn update_head(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
    expected_prev_receipt_hash: Hash256,
    receipt_hash: Hash256,
    seq: u64,
) -> Result<()> {
    let event_hlc = timestamp_parts(request.event_hlc)?;
    let seq_sql = i64::try_from(seq).map_err(|_| ReceiptStoreError::SequenceOutOfRange)?;
    let result = sqlx::query(
        "UPDATE dagdb_subject_receipt_heads \
         SET latest_receipt_hash = $1, latest_seq = $2, updated_at_physical_ms = $3, updated_at_logical = $4 \
         WHERE tenant_id = $5 AND namespace = $6 AND subject_kind = $7 AND subject_id = $8 \
         AND latest_receipt_hash = $9",
    )
    .bind(hash_bytes(receipt_hash))
    .bind(seq_sql)
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(subject_kind_sql(request.subject_kind))
    .bind(hash_bytes(request.subject_id))
    .bind(hash_bytes(expected_prev_receipt_hash))
    .execute(&mut **tx)
    .await
    .map_err(pg)?;

    if result.rows_affected() == 1 {
        Ok(())
    } else {
        Err(ReceiptStoreError::StalePreviousReceiptHash)
    }
}

async fn find_existing_event_receipt(
    tx: &mut Transaction<'_, Postgres>,
    request: &ReceiptAppendRequest,
) -> Result<Option<ReceiptAppendResult>> {
    let event_hlc = timestamp_parts(request.event_hlc)?;
    let row = sqlx::query(
        "SELECT receipt_hash, prev_receipt_hash, seq FROM dagdb_receipts \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = $3 AND subject_id = $4 \
         AND prev_receipt_hash = $5 AND event_type = $6 AND actor_did = $7 \
         AND event_hlc_physical_ms = $8 AND event_hlc_logical = $9 AND event_hash = $10 \
         ORDER BY seq ASC LIMIT 1",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(subject_kind_sql(request.subject_kind))
    .bind(hash_bytes(request.subject_id))
    .bind(hash_bytes(request.expected_prev_receipt_hash))
    .bind(event_type_sql(request.event_type))
    .bind(&request.actor_did)
    .bind(event_hlc.physical_ms)
    .bind(event_hlc.logical)
    .bind(hash_bytes(request.event_body_hash))
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;

    row.map(|row| {
        Ok(ReceiptAppendResult {
            receipt_hash: hash_from_vec(row.try_get("receipt_hash").map_err(pg)?)?,
            prev_receipt_hash: hash_from_vec(row.try_get("prev_receipt_hash").map_err(pg)?)?,
            seq: seq_from_i64(row.try_get("seq").map_err(pg)?)?,
            created_new: false,
        })
    })
    .transpose()
}

#[derive(Debug, Clone, Copy)]
struct SqlTimestamp {
    physical_ms: i64,
    logical: i32,
}

fn timestamp_parts(timestamp: Timestamp) -> Result<SqlTimestamp> {
    Ok(SqlTimestamp {
        physical_ms: i64::try_from(timestamp.physical_ms)
            .map_err(|_| ReceiptStoreError::TimestampOutOfRange)?,
        logical: i32::try_from(timestamp.logical)
            .map_err(|_| ReceiptStoreError::TimestampOutOfRange)?,
    })
}

fn seq_from_i64(seq: i64) -> Result<u64> {
    u64::try_from(seq).map_err(|_| ReceiptStoreError::SequenceOutOfRange)
}

fn hash_bytes(hash: Hash256) -> Vec<u8> {
    hash.as_bytes().to_vec()
}

fn hash_from_vec(bytes: Vec<u8>) -> Result<Hash256> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ReceiptStoreError::InvalidHashLength)?;
    Ok(Hash256::from_bytes(bytes))
}

fn pg(source: sqlx::Error) -> ReceiptStoreError {
    ReceiptStoreError::Postgres { source }
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
