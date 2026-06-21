//! PostgreSQL-backed DAG finality outbox.

use exo_core::{Did, Hash256, Signature, Timestamp};
use exo_dag::{
    dag::{DagNode, compute_node_hash},
    store::DagStore,
};
use exo_dag_db_api::{DagFinalityStatus, ReceiptEventType, SubjectKind};
use serde_json::{Map, Value};
use sqlx::{PgPool, Row};
use thiserror::Error;

use crate::receipt::{
    ReceiptAppendRequest, ReceiptStoreError, append_receipt, reconstruct_receipt_chain,
};

/// Outbox retry budget fixed by the DAG DB MVP contract.
pub const MAX_OUTBOX_ATTEMPTS: i32 = 6;

/// Retry delays in seconds for attempts 1 through 6.
pub const RETRY_BACKOFF_SECONDS: [u64; 6] = [1, 5, 30, 120, 600, 1800];

/// Lease window applied when a worker claims a due outbox row. While the lease
/// holds, the row is not due for other workers; a crashed worker's row becomes
/// due again once the lease expires.
const OUTBOX_CLAIM_LEASE_MS: u64 = 30_000;

const OUTBOX_DAG_CREATOR_DID: &str = "did:exo:dagdb-outbox";

/// Durable outbox enqueue material.
#[derive(Debug, Clone)]
pub struct OutboxEnqueueRequest {
    pub outbox_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub dag_write_id: String,
    pub dag_payload_hash: Hash256,
    pub created_at: Timestamp,
}

/// Worker mode used by tests and recovery callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagWriteMode {
    Normal,
    FailBeforeDagWrite { error_code: String },
    FailAfterDagCommit,
}

/// Result of one outbox processing attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxProcessResult {
    Committed {
        outbox_id: Hash256,
        dag_receipt_hash: Hash256,
        receipt_hash: Hash256,
    },
    ScheduledRetry {
        outbox_id: Hash256,
        attempt_count: i32,
        next_attempt_at: Timestamp,
    },
    Compensated {
        outbox_id: Hash256,
        compensation_receipt_hash: Hash256,
    },
    AlreadyTerminal {
        outbox_id: Hash256,
        status: DagFinalityStatus,
    },
}

#[derive(Debug, Clone)]
struct OutboxRow {
    outbox_id: Hash256,
    tenant_id: String,
    namespace: String,
    subject_kind: SubjectKind,
    subject_id: Hash256,
    dag_write_id: String,
    dag_payload_hash: Hash256,
    dag_finality_status: DagFinalityStatus,
    attempt_count: i32,
    max_attempts: i32,
    created_at: Timestamp,
}

struct FinalityReceiptRequest<'a> {
    event_type: ReceiptEventType,
    status: DagFinalityStatus,
    now: Timestamp,
    actor_did: &'a str,
    dag_receipt_hash: Option<Hash256>,
    error_code: Option<&'a str>,
}

/// Errors from outbox processing.
#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("outbox row not found")]
    OutboxNotFound,
    #[error("no due outbox row")]
    NoDueOutboxRow,
    #[error("compensated outbox rows are terminal")]
    CompensatedRowsAreTerminal,
    #[error("outbox timestamp is out of SQL range")]
    TimestampOutOfRange,
    #[error("outbox hash column had invalid length")]
    InvalidHashLength,
    #[error("outbox attempt count is out of range")]
    AttemptOutOfRange,
    #[error("invalid outbox enum value")]
    InvalidEnumValue,
    #[error("invalid outbox DID: {0}")]
    InvalidDid(String),
    #[error("DAG write failed: {0}")]
    DagWrite(String),
    #[error("simulated postgres update failure after DAG commit")]
    PostgresUpdateFailedAfterDagCommit,
    #[error("postgres outbox operation failed")]
    Postgres {
        #[source]
        source: sqlx::Error,
    },
    #[error("receipt operation failed")]
    Receipt {
        #[from]
        source: ReceiptStoreError,
    },
}

/// Outbox result alias.
pub type Result<T> = std::result::Result<T, OutboxError>;

/// Insert a durable pending outbox row.
pub async fn enqueue_outbox(pool: &PgPool, request: &OutboxEnqueueRequest) -> Result<bool> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &request.tenant_id)
        .await
        .map_err(pg)?;
    let rows = sqlx::query(
        "INSERT INTO dagdb_dag_outbox \
         (outbox_id, tenant_id, namespace, subject_kind, subject_id, dag_write_id, dag_payload_hash, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $8, $9) \
         ON CONFLICT (tenant_id, namespace, subject_kind, subject_id, dag_write_id) DO NOTHING",
    )
    .bind(hash_bytes(request.outbox_id))
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(subject_kind_sql(request.subject_kind))
    .bind(hash_bytes(request.subject_id))
    .bind(&request.dag_write_id)
    .bind(hash_bytes(request.dag_payload_hash))
    .bind(timestamp_i64(request.created_at.physical_ms)?)
    .bind(timestamp_i32(request.created_at.logical)?)
    .execute(&mut *tx)
    .await
    .map_err(pg)?;
    tx.commit().await.map_err(pg)?;

    Ok(rows.rows_affected() == 1)
}

/// Return true when the subject has committed DAG finality.
pub async fn subject_has_committed_finality(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> Result<bool> {
    let Some(status) =
        subject_finality_status(pool, tenant_id, namespace, subject_kind, subject_id).await?
    else {
        return Ok(false);
    };
    Ok(status == DagFinalityStatus::Committed)
}

/// Route eligibility is fail-closed on non-committed DAG finality.
pub async fn subject_is_route_eligible(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> Result<bool> {
    subject_has_committed_finality(pool, tenant_id, namespace, subject_kind, subject_id).await
}

/// Context-packet eligibility is fail-closed on non-committed DAG finality.
pub async fn subject_is_context_eligible(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> Result<bool> {
    subject_has_committed_finality(pool, tenant_id, namespace, subject_kind, subject_id).await
}

/// Process the next due pending or failed outbox row.
///
/// The due row is claimed atomically (`FOR UPDATE SKIP LOCKED` plus a lease on
/// `next_attempt_at`) so concurrent workers never double-process one row.
/// Selection is restricted to the subject kinds this worker can parse;
/// `export` rows belong to the dedicated export-finality path and must never
/// be claimed here.
pub async fn process_next_due_outbox(
    pool: &PgPool,
    tenant_id: &str,
    store: &mut impl DagStore,
    now: Timestamp,
    actor_did: &str,
) -> Result<Option<OutboxProcessResult>> {
    let lease_until_physical_ms = now
        .physical_ms
        .checked_add(OUTBOX_CLAIM_LEASE_MS)
        .ok_or(OutboxError::TimestampOutOfRange)?;
    let mut tx = crate::postgres::begin_tenant_transaction(pool, tenant_id)
        .await
        .map_err(pg)?;
    let outbox_id = sqlx::query_scalar::<_, Vec<u8>>(
        "UPDATE dagdb_dag_outbox \
         SET next_attempt_at_physical_ms = $3, next_attempt_at_logical = $2, \
             updated_at_physical_ms = $1, updated_at_logical = $2 \
         WHERE tenant_id = $4 AND outbox_id = ( \
             SELECT outbox_id FROM dagdb_dag_outbox \
             WHERE tenant_id = $4 \
               AND dag_finality_status IN ('pending','failed') \
               AND subject_kind IN ('memory','catalog','route','context_packet','validation_report', \
                                    'agent_safety_score','inbound_agent_credential','council_decision') \
               AND attempt_count < max_attempts \
               AND (next_attempt_at_physical_ms IS NULL \
                    OR next_attempt_at_physical_ms < $1 \
                    OR (next_attempt_at_physical_ms = $1 AND next_attempt_at_logical <= $2)) \
             ORDER BY next_attempt_at_physical_ms ASC NULLS FIRST, next_attempt_at_logical ASC NULLS FIRST, attempt_count ASC \
             LIMIT 1 \
             FOR UPDATE SKIP LOCKED) \
         RETURNING outbox_id",
    )
    .bind(timestamp_i64(now.physical_ms)?)
    .bind(timestamp_i32(now.logical)?)
    .bind(timestamp_i64(lease_until_physical_ms)?)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg)?;
    tx.commit().await.map_err(pg)?;

    match outbox_id {
        Some(bytes) => process_outbox_by_id(
            pool,
            tenant_id,
            store,
            hash_from_vec(bytes)?,
            now,
            actor_did,
            DagWriteMode::Normal,
        )
        .await
        .map(Some),
        None => Ok(None),
    }
}

/// Process a specific outbox row.
pub async fn process_outbox_by_id(
    pool: &PgPool,
    tenant_id: &str,
    store: &mut impl DagStore,
    outbox_id: Hash256,
    now: Timestamp,
    actor_did: &str,
    mode: DagWriteMode,
) -> Result<OutboxProcessResult> {
    let row = load_outbox(pool, tenant_id, outbox_id).await?;
    match row.dag_finality_status {
        DagFinalityStatus::Committed | DagFinalityStatus::Compensated => {
            return Ok(OutboxProcessResult::AlreadyTerminal {
                outbox_id,
                status: row.dag_finality_status,
            });
        }
        DagFinalityStatus::Pending | DagFinalityStatus::Failed => {}
    }

    if let DagWriteMode::FailBeforeDagWrite { error_code } = mode {
        return record_outbox_failure(pool, &row, now, actor_did, &error_code).await;
    }

    let dag_receipt_hash = commit_to_exo_dag(store, &row).await?;
    if mode == DagWriteMode::FailAfterDagCommit {
        return Err(OutboxError::PostgresUpdateFailedAfterDagCommit);
    }

    let receipt_hash = append_finality_receipt(
        pool,
        &row,
        FinalityReceiptRequest {
            event_type: ReceiptEventType::DagFinalityCommitted,
            status: DagFinalityStatus::Committed,
            now,
            actor_did,
            dag_receipt_hash: Some(dag_receipt_hash),
            error_code: None,
        },
    )
    .await?;
    update_outbox_committed(pool, &row, dag_receipt_hash, now).await?;
    update_subject_finality(pool, &row, DagFinalityStatus::Committed, receipt_hash, now).await?;

    Ok(OutboxProcessResult::Committed {
        outbox_id,
        dag_receipt_hash,
        receipt_hash,
    })
}

/// Compensated rows are terminal; operators must create a new recovery row.
pub async fn operator_retry_compensated_row(
    pool: &PgPool,
    tenant_id: &str,
    outbox_id: Hash256,
) -> Result<()> {
    let row = load_outbox(pool, tenant_id, outbox_id).await?;
    if row.dag_finality_status == DagFinalityStatus::Compensated {
        return Err(OutboxError::CompensatedRowsAreTerminal);
    }
    Ok(())
}

/// Reconstruct the subject receipt chain after outbox recovery.
pub async fn reconstruct_subject_receipts_after_recovery(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> Result<Vec<crate::receipt::ReceiptRecord>> {
    reconstruct_receipt_chain(pool, tenant_id, namespace, subject_kind, subject_id)
        .await
        .map_err(OutboxError::from)
}

async fn commit_to_exo_dag(store: &mut impl DagStore, row: &OutboxRow) -> Result<Hash256> {
    let node = dag_node_for_row(row)?;
    if !store.contains(&node.hash).await.map_err(dag)? {
        store.put(node.clone()).await.map_err(dag)?;
        let next_height = store
            .committed_height()
            .await
            .map_err(dag)?
            .checked_add(1)
            .ok_or(OutboxError::AttemptOutOfRange)?;
        store
            .mark_committed(&node.hash, next_height)
            .await
            .map_err(dag)?;
    }
    Ok(node.hash)
}

fn dag_node_for_row(row: &OutboxRow) -> Result<DagNode> {
    let creator_did =
        Did::new(OUTBOX_DAG_CREATOR_DID).map_err(|err| OutboxError::InvalidDid(err.to_string()))?;
    let hash = compute_node_hash(&[], &row.dag_payload_hash, &creator_did, &row.created_at)
        .map_err(|err| OutboxError::DagWrite(err.to_string()))?;
    Ok(DagNode {
        hash,
        parents: Vec::new(),
        payload_hash: row.dag_payload_hash,
        creator_did,
        timestamp: row.created_at,
        signature: deterministic_outbox_signature(hash, &row.dag_write_id),
    })
}

fn deterministic_outbox_signature(hash: Hash256, dag_write_id: &str) -> Signature {
    let mut material = Vec::new();
    material.extend_from_slice(hash.as_bytes());
    material.extend_from_slice(dag_write_id.as_bytes());
    let left = Hash256::digest(&material);
    material.push(1);
    let right = Hash256::digest(&material);
    let mut bytes = [0_u8; 64];
    bytes[..32].copy_from_slice(left.as_bytes());
    bytes[32..].copy_from_slice(right.as_bytes());
    Signature::from_bytes(bytes)
}

async fn record_outbox_failure(
    pool: &PgPool,
    row: &OutboxRow,
    now: Timestamp,
    actor_did: &str,
    error_code: &str,
) -> Result<OutboxProcessResult> {
    let next_attempt_count = row
        .attempt_count
        .checked_add(1)
        .ok_or(OutboxError::AttemptOutOfRange)?;
    if next_attempt_count >= row.max_attempts {
        let compensation_receipt_hash = append_finality_receipt(
            pool,
            row,
            FinalityReceiptRequest {
                event_type: ReceiptEventType::DagFinalityCompensated,
                status: DagFinalityStatus::Compensated,
                now,
                actor_did,
                dag_receipt_hash: None,
                error_code: Some(error_code),
            },
        )
        .await?;
        update_outbox_compensated(pool, row, error_code, compensation_receipt_hash, now).await?;
        update_subject_finality(
            pool,
            row,
            DagFinalityStatus::Compensated,
            compensation_receipt_hash,
            now,
        )
        .await?;
        return Ok(OutboxProcessResult::Compensated {
            outbox_id: row.outbox_id,
            compensation_receipt_hash,
        });
    }

    let failure_receipt_hash = append_finality_receipt(
        pool,
        row,
        FinalityReceiptRequest {
            event_type: ReceiptEventType::DagFinalityFailed,
            status: DagFinalityStatus::Failed,
            now,
            actor_did,
            dag_receipt_hash: None,
            error_code: Some(error_code),
        },
    )
    .await?;
    let next_attempt_at = retry_at(now, next_attempt_count)?;
    let recorded_attempt_count =
        update_outbox_failed(pool, row, next_attempt_at, error_code, now).await?;
    update_subject_finality(
        pool,
        row,
        DagFinalityStatus::Failed,
        failure_receipt_hash,
        now,
    )
    .await?;

    Ok(OutboxProcessResult::ScheduledRetry {
        outbox_id: row.outbox_id,
        attempt_count: recorded_attempt_count,
        next_attempt_at,
    })
}

fn retry_at(now: Timestamp, attempt_count: i32) -> Result<Timestamp> {
    let index = usize::try_from(attempt_count.saturating_sub(1))
        .map_err(|_| OutboxError::AttemptOutOfRange)?;
    let delay_secs = RETRY_BACKOFF_SECONDS
        .get(index)
        .ok_or(OutboxError::AttemptOutOfRange)?;
    let delay_ms = delay_secs
        .checked_mul(1000)
        .ok_or(OutboxError::AttemptOutOfRange)?;
    let physical_ms = now
        .physical_ms
        .checked_add(delay_ms)
        .ok_or(OutboxError::AttemptOutOfRange)?;
    Ok(Timestamp::new(physical_ms, now.logical))
}

async fn append_finality_receipt(
    pool: &PgPool,
    row: &OutboxRow,
    request: FinalityReceiptRequest<'_>,
) -> Result<Hash256> {
    let expected_prev_receipt_hash = latest_subject_receipt_hash(pool, row).await?;
    let event_body_hash = finality_event_body_hash(
        row,
        request.status,
        request.dag_receipt_hash,
        request.error_code,
    );
    // Crash-window replay guard: if a previous run already appended this exact
    // terminal finality event but crashed before finalizing the outbox row,
    // reuse the head receipt instead of appending a duplicate at seq + 1.
    if matches!(
        request.status,
        DagFinalityStatus::Committed | DagFinalityStatus::Compensated
    ) && head_receipt_matches_event(pool, row, expected_prev_receipt_hash, event_body_hash)
        .await?
    {
        return Ok(expected_prev_receipt_hash);
    }
    let receipt_body = finality_receipt_body(
        row,
        request.status,
        request.dag_receipt_hash,
        request.error_code,
    );
    let receipt = append_receipt(
        pool,
        &ReceiptAppendRequest {
            tenant_id: row.tenant_id.clone(),
            namespace: row.namespace.clone(),
            subject_kind: row.subject_kind,
            subject_id: row.subject_id,
            expected_prev_receipt_hash,
            event_type: request.event_type,
            actor_did: request.actor_did.to_owned(),
            event_hlc: request.now,
            event_body_hash,
            receipt_body,
        },
    )
    .await?;
    Ok(receipt.receipt_hash)
}

fn finality_event_body_hash(
    row: &OutboxRow,
    status: DagFinalityStatus,
    dag_receipt_hash: Option<Hash256>,
    error_code: Option<&str>,
) -> Hash256 {
    let mut material = Vec::new();
    material.extend_from_slice(row.outbox_id.as_bytes());
    material.extend_from_slice(row.dag_write_id.as_bytes());
    material.extend_from_slice(finality_status_sql(status).as_bytes());
    if let Some(hash) = dag_receipt_hash {
        material.extend_from_slice(hash.as_bytes());
    }
    if let Some(code) = error_code {
        material.extend_from_slice(code.as_bytes());
    }
    Hash256::digest(&material)
}

fn finality_receipt_body(
    row: &OutboxRow,
    status: DagFinalityStatus,
    dag_receipt_hash: Option<Hash256>,
    error_code: Option<&str>,
) -> Value {
    let mut body = Map::new();
    body.insert("event".to_owned(), Value::String("dag_finality".to_owned()));
    body.insert(
        "status".to_owned(),
        Value::String(finality_status_sql(status).to_owned()),
    );
    body.insert(
        "outbox_id".to_owned(),
        Value::String(row.outbox_id.to_string()),
    );
    body.insert(
        "dag_write_id".to_owned(),
        Value::String(row.dag_write_id.clone()),
    );
    if let Some(hash) = dag_receipt_hash {
        body.insert(
            "dag_receipt_hash".to_owned(),
            Value::String(hash.to_string()),
        );
    }
    if let Some(code) = error_code {
        body.insert("last_error_code".to_owned(), Value::String(code.to_owned()));
    }
    Value::Object(body)
}

/// True when the current head receipt already records the given finality
/// event body (the event hash binds outbox_id, dag_write_id, status, and the
/// DAG receipt hash, so a match is exactly this terminal event).
async fn head_receipt_matches_event(
    pool: &PgPool,
    row: &OutboxRow,
    head_receipt_hash: Hash256,
    event_body_hash: Hash256,
) -> Result<bool> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &row.tenant_id)
        .await
        .map_err(pg)?;
    let matched = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dagdb_receipts \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = $3 AND subject_id = $4 \
           AND receipt_hash = $5 AND event_hash = $6",
    )
    .bind(&row.tenant_id)
    .bind(&row.namespace)
    .bind(subject_kind_sql(row.subject_kind))
    .bind(hash_bytes(row.subject_id))
    .bind(hash_bytes(head_receipt_hash))
    .bind(hash_bytes(event_body_hash))
    .fetch_one(&mut *tx)
    .await
    .map_err(pg)?;
    tx.commit().await.map_err(pg)?;
    Ok(matched == 1)
}

async fn latest_subject_receipt_hash(pool: &PgPool, row: &OutboxRow) -> Result<Hash256> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &row.tenant_id)
        .await
        .map_err(pg)?;
    let receipt_hash = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT latest_receipt_hash FROM dagdb_subject_receipt_heads \
         WHERE tenant_id = $1 AND namespace = $2 AND subject_kind = $3 AND subject_id = $4",
    )
    .bind(&row.tenant_id)
    .bind(&row.namespace)
    .bind(subject_kind_sql(row.subject_kind))
    .bind(hash_bytes(row.subject_id))
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg)?
    .ok_or(OutboxError::OutboxNotFound)?;
    tx.commit().await.map_err(pg)?;
    hash_from_vec(receipt_hash)
}

async fn update_outbox_committed(
    pool: &PgPool,
    row: &OutboxRow,
    dag_receipt_hash: Hash256,
    now: Timestamp,
) -> Result<()> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &row.tenant_id)
        .await
        .map_err(pg)?;
    sqlx::query(
        "UPDATE dagdb_dag_outbox \
         SET dag_finality_status = 'committed', dag_receipt_hash = $1, last_error_code = NULL, \
             next_attempt_at_physical_ms = NULL, next_attempt_at_logical = NULL, \
             updated_at_physical_ms = $2, updated_at_logical = $3 \
         WHERE tenant_id = $4 AND outbox_id = $5",
    )
    .bind(hash_bytes(dag_receipt_hash))
    .bind(timestamp_i64(now.physical_ms)?)
    .bind(timestamp_i32(now.logical)?)
    .bind(&row.tenant_id)
    .bind(hash_bytes(row.outbox_id))
    .execute(&mut *tx)
    .await
    .map_err(pg)?;
    tx.commit().await.map_err(pg)?;
    Ok(())
}

/// Record a retryable failure with a relative attempt increment so concurrent
/// or duplicated workers can never under-count the bounded retry budget.
async fn update_outbox_failed(
    pool: &PgPool,
    row: &OutboxRow,
    next_attempt_at: Timestamp,
    error_code: &str,
    now: Timestamp,
) -> Result<i32> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &row.tenant_id)
        .await
        .map_err(pg)?;
    let attempt_count = sqlx::query_scalar::<_, i32>(
        "UPDATE dagdb_dag_outbox \
         SET dag_finality_status = 'failed', attempt_count = attempt_count + 1, \
             last_error_code = $1, \
             next_attempt_at_physical_ms = $2, next_attempt_at_logical = $3, \
             updated_at_physical_ms = $4, updated_at_logical = $5 \
         WHERE tenant_id = $6 AND outbox_id = $7 AND attempt_count < max_attempts \
         RETURNING attempt_count",
    )
    .bind(error_code)
    .bind(timestamp_i64(next_attempt_at.physical_ms)?)
    .bind(timestamp_i32(next_attempt_at.logical)?)
    .bind(timestamp_i64(now.physical_ms)?)
    .bind(timestamp_i32(now.logical)?)
    .bind(&row.tenant_id)
    .bind(hash_bytes(row.outbox_id))
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg)?
    .ok_or(OutboxError::AttemptOutOfRange)?;
    tx.commit().await.map_err(pg)?;
    Ok(attempt_count)
}

/// Record the terminal compensation with a relative, budget-capped attempt
/// increment in the same statement as the status write.
async fn update_outbox_compensated(
    pool: &PgPool,
    row: &OutboxRow,
    error_code: &str,
    compensation_receipt_hash: Hash256,
    now: Timestamp,
) -> Result<()> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &row.tenant_id)
        .await
        .map_err(pg)?;
    sqlx::query(
        "UPDATE dagdb_dag_outbox \
         SET dag_finality_status = 'compensated', \
             attempt_count = LEAST(attempt_count + 1, max_attempts), last_error_code = $1, \
             next_attempt_at_physical_ms = NULL, next_attempt_at_logical = NULL, \
             compensation_receipt_hash = $2, updated_at_physical_ms = $3, updated_at_logical = $4 \
         WHERE tenant_id = $5 AND outbox_id = $6",
    )
    .bind(error_code)
    .bind(hash_bytes(compensation_receipt_hash))
    .bind(timestamp_i64(now.physical_ms)?)
    .bind(timestamp_i32(now.logical)?)
    .bind(&row.tenant_id)
    .bind(hash_bytes(row.outbox_id))
    .execute(&mut *tx)
    .await
    .map_err(pg)?;
    tx.commit().await.map_err(pg)?;
    Ok(())
}

async fn update_subject_finality(
    pool: &PgPool,
    row: &OutboxRow,
    status: DagFinalityStatus,
    receipt_hash: Hash256,
    now: Timestamp,
) -> Result<()> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, &row.tenant_id)
        .await
        .map_err(pg)?;
    let status = finality_status_sql(status);
    let subject_id = hash_bytes(row.subject_id);
    let receipt_hash = hash_bytes(receipt_hash);
    match row.subject_kind {
        SubjectKind::Memory => {
            sqlx::query(
                "UPDATE dagdb_memory_objects \
                 SET dag_finality_status = $1, latest_receipt_hash = $2, \
                     updated_at_physical_ms = $3, updated_at_logical = $4 \
                 WHERE tenant_id = $5 AND namespace = $6 AND memory_id = $7",
            )
            .bind(status)
            .bind(receipt_hash)
            .bind(timestamp_i64(now.physical_ms)?)
            .bind(timestamp_i32(now.logical)?)
            .bind(&row.tenant_id)
            .bind(&row.namespace)
            .bind(subject_id)
            .execute(&mut *tx)
            .await
            .map_err(pg)?;
        }
        SubjectKind::Catalog => {
            sqlx::query(
                "UPDATE dagdb_catalog_entries \
                 SET dag_finality_status = $1, latest_receipt_hash = $2, \
                     updated_at_physical_ms = $3, updated_at_logical = $4 \
                 WHERE tenant_id = $5 AND namespace = $6 AND catalog_id = $7",
            )
            .bind(status)
            .bind(receipt_hash)
            .bind(timestamp_i64(now.physical_ms)?)
            .bind(timestamp_i32(now.logical)?)
            .bind(&row.tenant_id)
            .bind(&row.namespace)
            .bind(subject_id)
            .execute(&mut *tx)
            .await
            .map_err(pg)?;
        }
        SubjectKind::Route => {
            sqlx::query(
                "UPDATE dagdb_route_receipts \
                 SET dag_finality_status = $1, latest_receipt_hash = $2 \
                 WHERE tenant_id = $3 AND namespace = $4 AND route_id = $5",
            )
            .bind(status)
            .bind(receipt_hash)
            .bind(&row.tenant_id)
            .bind(&row.namespace)
            .bind(subject_id)
            .execute(&mut *tx)
            .await
            .map_err(pg)?;
        }
        SubjectKind::ContextPacket => {
            sqlx::query(
                "UPDATE dagdb_context_packets \
                 SET dag_finality_status = $1, latest_receipt_hash = $2 \
                 WHERE tenant_id = $3 AND namespace = $4 AND context_packet_id = $5",
            )
            .bind(status)
            .bind(receipt_hash)
            .bind(&row.tenant_id)
            .bind(&row.namespace)
            .bind(subject_id)
            .execute(&mut *tx)
            .await
            .map_err(pg)?;
        }
        SubjectKind::ValidationReport
        | SubjectKind::AgentSafetyScore
        | SubjectKind::InboundAgentCredential
        | SubjectKind::CouncilDecision => {}
    }
    tx.commit().await.map_err(pg)?;
    Ok(())
}

async fn subject_finality_status(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    subject_kind: SubjectKind,
    subject_id: Hash256,
) -> Result<Option<DagFinalityStatus>> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, tenant_id)
        .await
        .map_err(pg)?;
    let subject_id = hash_bytes(subject_id);
    let status = match subject_kind {
        SubjectKind::Memory => sqlx::query_scalar::<_, String>(
            "SELECT dag_finality_status FROM dagdb_memory_objects \
                 WHERE tenant_id = $1 AND namespace = $2 AND memory_id = $3",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(subject_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg)?,
        SubjectKind::Catalog => sqlx::query_scalar::<_, String>(
            "SELECT dag_finality_status FROM dagdb_catalog_entries \
                 WHERE tenant_id = $1 AND namespace = $2 AND catalog_id = $3",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(subject_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg)?,
        SubjectKind::Route => sqlx::query_scalar::<_, String>(
            "SELECT dag_finality_status FROM dagdb_route_receipts \
                 WHERE tenant_id = $1 AND namespace = $2 AND route_id = $3",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(subject_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg)?,
        SubjectKind::ContextPacket => sqlx::query_scalar::<_, String>(
            "SELECT dag_finality_status FROM dagdb_context_packets \
                 WHERE tenant_id = $1 AND namespace = $2 AND context_packet_id = $3",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(subject_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg)?,
        SubjectKind::ValidationReport
        | SubjectKind::AgentSafetyScore
        | SubjectKind::InboundAgentCredential
        | SubjectKind::CouncilDecision => None,
    };
    let status = status
        .map(|value| parse_finality_status(&value))
        .transpose()?;
    tx.commit().await.map_err(pg)?;
    Ok(status)
}

async fn load_outbox(pool: &PgPool, tenant_id: &str, outbox_id: Hash256) -> Result<OutboxRow> {
    let mut tx = crate::postgres::begin_tenant_transaction(pool, tenant_id)
        .await
        .map_err(pg)?;
    let row = sqlx::query(
        "SELECT outbox_id, tenant_id, namespace, subject_kind, subject_id, dag_write_id, \
                dag_payload_hash, dag_finality_status, attempt_count, max_attempts, \
                created_at_physical_ms, created_at_logical \
         FROM dagdb_dag_outbox WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(tenant_id)
    .bind(hash_bytes(outbox_id))
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg)?
    .ok_or(OutboxError::OutboxNotFound)?;
    tx.commit().await.map_err(pg)?;

    Ok(OutboxRow {
        outbox_id: hash_from_vec(row.try_get("outbox_id").map_err(pg)?)?,
        tenant_id: row.try_get("tenant_id").map_err(pg)?,
        namespace: row.try_get("namespace").map_err(pg)?,
        subject_kind: parse_subject_kind(row.try_get::<String, _>("subject_kind").map_err(pg)?)?,
        subject_id: hash_from_vec(row.try_get("subject_id").map_err(pg)?)?,
        dag_write_id: row.try_get("dag_write_id").map_err(pg)?,
        dag_payload_hash: hash_from_vec(row.try_get("dag_payload_hash").map_err(pg)?)?,
        dag_finality_status: parse_finality_status(
            &row.try_get::<String, _>("dag_finality_status")
                .map_err(pg)?,
        )?,
        attempt_count: row.try_get("attempt_count").map_err(pg)?,
        max_attempts: row.try_get("max_attempts").map_err(pg)?,
        created_at: Timestamp::new(
            u64_from_i64(row.try_get("created_at_physical_ms").map_err(pg)?)?,
            u32_from_i32(row.try_get("created_at_logical").map_err(pg)?)?,
        ),
    })
}

fn parse_subject_kind(value: String) -> Result<SubjectKind> {
    match value.as_str() {
        "memory" => Ok(SubjectKind::Memory),
        "catalog" => Ok(SubjectKind::Catalog),
        "route" => Ok(SubjectKind::Route),
        "context_packet" => Ok(SubjectKind::ContextPacket),
        "validation_report" => Ok(SubjectKind::ValidationReport),
        "agent_safety_score" => Ok(SubjectKind::AgentSafetyScore),
        "inbound_agent_credential" => Ok(SubjectKind::InboundAgentCredential),
        "council_decision" => Ok(SubjectKind::CouncilDecision),
        _ => Err(OutboxError::InvalidEnumValue),
    }
}

fn parse_finality_status(value: &str) -> Result<DagFinalityStatus> {
    match value {
        "pending" => Ok(DagFinalityStatus::Pending),
        "committed" => Ok(DagFinalityStatus::Committed),
        "failed" => Ok(DagFinalityStatus::Failed),
        "compensated" => Ok(DagFinalityStatus::Compensated),
        _ => Err(OutboxError::InvalidEnumValue),
    }
}

fn subject_kind_sql(subject_kind: SubjectKind) -> &'static str {
    match subject_kind {
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

fn finality_status_sql(status: DagFinalityStatus) -> &'static str {
    match status {
        DagFinalityStatus::Pending => "pending",
        DagFinalityStatus::Committed => "committed",
        DagFinalityStatus::Failed => "failed",
        DagFinalityStatus::Compensated => "compensated",
    }
}

fn hash_bytes(hash: Hash256) -> Vec<u8> {
    hash.as_bytes().to_vec()
}

fn hash_from_vec(bytes: Vec<u8>) -> Result<Hash256> {
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| OutboxError::InvalidHashLength)?;
    Ok(Hash256::from_bytes(array))
}

fn timestamp_i64(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| OutboxError::TimestampOutOfRange)
}

fn timestamp_i32(value: u32) -> Result<i32> {
    i32::try_from(value).map_err(|_| OutboxError::TimestampOutOfRange)
}

fn u64_from_i64(value: i64) -> Result<u64> {
    u64::try_from(value).map_err(|_| OutboxError::TimestampOutOfRange)
}

fn u32_from_i32(value: i32) -> Result<u32> {
    u32::try_from(value).map_err(|_| OutboxError::TimestampOutOfRange)
}

fn pg(source: sqlx::Error) -> OutboxError {
    OutboxError::Postgres { source }
}

fn dag(source: exo_dag::error::DagError) -> OutboxError {
    OutboxError::DagWrite(source.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn row(subject_kind: SubjectKind) -> OutboxRow {
        OutboxRow {
            outbox_id: h(0x01),
            tenant_id: "tenant-a".to_owned(),
            namespace: "default".to_owned(),
            subject_kind,
            subject_id: h(0x02),
            dag_write_id: "dag-write-unit".to_owned(),
            dag_payload_hash: h(0x03),
            dag_finality_status: DagFinalityStatus::Pending,
            attempt_count: 0,
            max_attempts: MAX_OUTBOX_ATTEMPTS,
            created_at: Timestamp::new(123, 4),
        }
    }

    #[test]
    fn enum_sql_vectors_are_total_for_outbox_contract() {
        let subject_vectors = [
            ("memory", SubjectKind::Memory),
            ("catalog", SubjectKind::Catalog),
            ("route", SubjectKind::Route),
            ("context_packet", SubjectKind::ContextPacket),
            ("validation_report", SubjectKind::ValidationReport),
            ("agent_safety_score", SubjectKind::AgentSafetyScore),
            (
                "inbound_agent_credential",
                SubjectKind::InboundAgentCredential,
            ),
            ("council_decision", SubjectKind::CouncilDecision),
        ];
        for (label, kind) in subject_vectors {
            assert_eq!(subject_kind_sql(kind), label);
            assert_eq!(
                parse_subject_kind(label.to_owned()).expect("parse kind"),
                kind
            );
        }
        assert!(matches!(
            parse_subject_kind("unknown".to_owned()),
            Err(OutboxError::InvalidEnumValue)
        ));

        let finality_vectors = [
            ("pending", DagFinalityStatus::Pending),
            ("committed", DagFinalityStatus::Committed),
            ("failed", DagFinalityStatus::Failed),
            ("compensated", DagFinalityStatus::Compensated),
        ];
        for (label, status) in finality_vectors {
            assert_eq!(finality_status_sql(status), label);
            assert_eq!(parse_finality_status(label).expect("parse status"), status);
        }
        assert!(matches!(
            parse_finality_status("unknown"),
            Err(OutboxError::InvalidEnumValue)
        ));
    }

    #[test]
    fn deterministic_dag_node_and_receipt_body_vectors_are_stable() {
        let row = row(SubjectKind::Memory);
        let node = dag_node_for_row(&row).expect("dag node");
        assert_eq!(node.parents, Vec::<Hash256>::new());
        assert_eq!(node.payload_hash, row.dag_payload_hash);
        assert_eq!(node.timestamp, row.created_at);
        assert!(!node.signature.is_empty());
        assert_eq!(
            deterministic_outbox_signature(node.hash, &row.dag_write_id),
            node.signature
        );
        assert_ne!(
            deterministic_outbox_signature(node.hash, "different-write"),
            node.signature
        );

        let committed_hash =
            finality_event_body_hash(&row, DagFinalityStatus::Committed, Some(h(0x04)), None);
        let failed_hash =
            finality_event_body_hash(&row, DagFinalityStatus::Failed, None, Some("dag_down"));
        assert_ne!(committed_hash, failed_hash);

        let committed_body =
            finality_receipt_body(&row, DagFinalityStatus::Committed, Some(h(0x04)), None);
        assert_eq!(committed_body["event"], "dag_finality");
        assert_eq!(committed_body["status"], "committed");
        assert_eq!(committed_body["dag_receipt_hash"], h(0x04).to_string());

        let failed_body =
            finality_receipt_body(&row, DagFinalityStatus::Failed, None, Some("dag_down"));
        assert_eq!(failed_body["status"], "failed");
        assert_eq!(failed_body["last_error_code"], "dag_down");
    }

    #[test]
    fn retry_and_scalar_bounds_fail_closed() {
        assert_eq!(
            retry_at(Timestamp::new(1_000, 7), 1).expect("first retry"),
            Timestamp::new(2_000, 7)
        );
        assert_eq!(
            retry_at(Timestamp::new(1_000, 7), 6).expect("sixth retry"),
            Timestamp::new(1_801_000, 7)
        );
        assert!(matches!(
            retry_at(Timestamp::new(1_000, 0), 7),
            Err(OutboxError::AttemptOutOfRange)
        ));
        assert!(matches!(
            retry_at(Timestamp::new(u64::MAX, 0), 1),
            Err(OutboxError::AttemptOutOfRange)
        ));
        assert!(matches!(
            hash_from_vec(vec![0; 31]),
            Err(OutboxError::InvalidHashLength)
        ));
        assert!(matches!(
            timestamp_i64(u64::MAX),
            Err(OutboxError::TimestampOutOfRange)
        ));
        assert!(matches!(
            timestamp_i32(u32::MAX),
            Err(OutboxError::TimestampOutOfRange)
        ));
        assert!(matches!(
            u64_from_i64(-1),
            Err(OutboxError::TimestampOutOfRange)
        ));
        assert!(matches!(
            u32_from_i32(-1),
            Err(OutboxError::TimestampOutOfRange)
        ));
    }
}
