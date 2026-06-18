//! Feature-gated Postgres adapter for PRD17C continuation records.

use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use thiserror::Error;

use crate::continuation_persistence::{
    ContinuationPersistResult, ContinuationPersistenceError, ContinuationRecord,
};

/// Errors raised by PRD17C continuation Postgres persistence.
#[derive(Debug, Error)]
pub enum ContinuationPersistencePostgresError {
    #[error(transparent)]
    Continuation(#[from] ContinuationPersistenceError),
    #[error("prd17_continuation_postgres_failed")]
    Postgres {
        #[source]
        source: sqlx::Error,
    },
    #[error("prd17_continuation_json_failed: {reason}")]
    Json { reason: String },
}

/// Result alias for continuation Postgres persistence.
pub type Result<T> = std::result::Result<T, ContinuationPersistencePostgresError>;

/// Persist a validated continuation record.
pub async fn persist_continuation_record(
    pool: &PgPool,
    record: &ContinuationRecord,
    now_epoch_seconds: u64,
) -> Result<ContinuationPersistResult> {
    record.validate(now_epoch_seconds)?;
    let idempotency_key = record.idempotency_key()?;
    let rollback_idempotency_key = idempotency_key.clone();
    let record_body = serde_json::to_value(record).map_err(json)?;

    let mut tx = pool.begin().await.map_err(pg)?;
    let result = async {
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await
            .map_err(pg)?;

        if let Some(row) = sqlx::query(
            "SELECT continuation_id, record_body FROM dagdb_continuation_records \
             WHERE idempotency_key = $1 AND tenant_id = $2 AND project_id = $3 \
               AND memory_namespace = $4",
        )
        .bind(&idempotency_key)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.memory_namespace)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg)?
        {
            let existing_id = row.get::<String, _>("continuation_id");
            let existing_body = row.get::<JsonValue, _>("record_body");
            if existing_body == record_body {
                return Ok(ContinuationPersistResult {
                    continuation_id: existing_id,
                    idempotency_key,
                    replayed: true,
                    later_retrieval_status: record.later_retrieval_status,
                });
            }
            return Err(
                ContinuationPersistenceError::DuplicateUnsafeReplay { idempotency_key }.into(),
            );
        }

        sqlx::query(
             "INSERT INTO dagdb_continuation_records \
             (continuation_id, task_id, tenant_id, project_id, memory_namespace, summary_ref, memory_refs, blocker_refs, validation_refs, expiry_epoch_seconds, later_retrieval_status, production_lifecycle_approval, idempotency_key, record_body, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(&record.continuation_id)
        .bind(&record.task_id)
        .bind(&record.tenant_id)
        .bind(&record.project_id)
        .bind(&record.memory_namespace)
        .bind(&record.summary_ref)
        .bind(serde_json::to_value(&record.memory_refs).map_err(json)?)
        .bind(serde_json::to_value(&record.blocker_refs).map_err(json)?)
        .bind(serde_json::to_value(&record.validation_refs).map_err(json)?)
        .bind(i64::try_from(record.expiry_epoch_seconds).map_err(|_| {
            ContinuationPersistenceError::InvalidRecord {
                reason: "expiry_epoch_seconds out of range".to_owned(),
            }
        })?)
        .bind(match record.later_retrieval_status {
            crate::continuation_persistence::ContinuationRetrievalStatus::Pending => "pending",
            crate::continuation_persistence::ContinuationRetrievalStatus::Retrieved => "retrieved",
            crate::continuation_persistence::ContinuationRetrievalStatus::ExpiredRejected => {
                "expired_rejected"
            }
        })
        .bind(match record.production_lifecycle_approval {
            crate::lifecycle_action::ProductionLifecycleApproval::Approved => "approved",
            crate::lifecycle_action::ProductionLifecycleApproval::OperatorDeferred => {
                "operator_deferred"
            }
        })
        .bind(&idempotency_key)
        .bind(record_body)
        .bind(&record.created_at)
        .execute(&mut *tx)
        .await
        .map_err(pg)?;

        Ok(ContinuationPersistResult {
            continuation_id: record.continuation_id.clone(),
            idempotency_key,
            replayed: false,
            later_retrieval_status: record.later_retrieval_status,
        })
    }
    .await;

    match result {
        Ok(result) => {
            tx.commit().await.map_err(pg)?;
            Ok(result)
        }
        Err(error) => {
            if let Err(rollback_error) = tx.rollback().await {
                tracing::warn!(
                    operation = "persist_continuation_record",
                    tenant_id = %record.tenant_id,
                    project_id = %record.project_id,
                    memory_namespace = %record.memory_namespace,
                    continuation_id = %record.continuation_id,
                    task_id = %record.task_id,
                    idempotency_key = %rollback_idempotency_key,
                    error = %rollback_error,
                    "failed to rollback transaction after continuation persistence error"
                );
            }
            Err(error)
        }
    }
}

fn pg(source: sqlx::Error) -> ContinuationPersistencePostgresError {
    ContinuationPersistencePostgresError::Postgres { source }
}

fn json(source: serde_json::Error) -> ContinuationPersistencePostgresError {
    ContinuationPersistencePostgresError::Json {
        reason: source.to_string(),
    }
}
