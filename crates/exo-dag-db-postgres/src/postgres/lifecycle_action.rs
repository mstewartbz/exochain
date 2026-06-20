//! Feature-gated Postgres adapter for PRD17C lifecycle actions.

use serde_json::Value as JsonValue;
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;

use crate::lifecycle_action::{
    LifecycleAction, LifecycleActionError, LifecycleApplyResult, LifecycleTerminalState,
    ProductionLifecycleApproval, ProductionLifecycleApprovalEvidence,
};

/// Errors raised by PRD17C lifecycle Postgres persistence.
#[derive(Debug, Error)]
pub enum LifecycleActionPostgresError {
    #[error(transparent)]
    Lifecycle(#[from] LifecycleActionError),
    #[error("prd17_lifecycle_postgres_failed")]
    Postgres {
        #[source]
        source: sqlx::Error,
    },
    #[error("prd17_lifecycle_json_failed: {reason}")]
    Json { reason: String },
}

/// Result alias for lifecycle Postgres persistence.
pub type Result<T> = std::result::Result<T, LifecycleActionPostgresError>;

/// Persist a validated lifecycle action and rollback ref.
pub async fn persist_lifecycle_action(
    pool: &PgPool,
    action: &LifecycleAction,
) -> Result<LifecycleApplyResult> {
    if action.terminal_state == LifecycleTerminalState::Accepted
        || action.production_lifecycle_approval == ProductionLifecycleApproval::Approved
    {
        return Err(LifecycleActionError::ProductionApprovalMissing {
            action_id: action.action_id.clone(),
        }
        .into());
    }
    persist_lifecycle_action_checked(pool, action, "persist_lifecycle_action").await
}

async fn persist_lifecycle_action_checked(
    pool: &PgPool,
    action: &LifecycleAction,
    operation: &'static str,
) -> Result<LifecycleApplyResult> {
    action.validate()?;
    let idempotency_key = action.idempotency_key()?;
    let action_body = serde_json::to_value(action).map_err(json)?;
    let rollback_body = serde_json::to_value(&action.rollback_ref).map_err(json)?;

    let mut tx = pool.begin().await.map_err(pg)?;
    let result = persist_lifecycle_action_in_transaction(
        &mut tx,
        action,
        &idempotency_key,
        action_body,
        rollback_body,
    )
    .await;
    match result {
        Ok(result) => {
            tx.commit().await.map_err(pg)?;
            Ok(result)
        }
        Err(error) => {
            if let Err(rollback_error) = tx.rollback().await {
                tracing::warn!(
                    operation = operation,
                    tenant_id = %action.tenant_id,
                    project_id = %action.project_id,
                    memory_namespace = %action.memory_namespace,
                    action_id = %action.action_id,
                    rollback_id = %action.rollback_ref.rollback_id,
                    idempotency_key = %idempotency_key,
                    error = %rollback_error,
                    "failed to rollback transaction after lifecycle action persistence error"
                );
            }
            Err(error)
        }
    }
}

/// Accept and persist a lifecycle action after production approval/finality evidence is bound.
pub async fn persist_approved_lifecycle_action(
    pool: &PgPool,
    action: &LifecycleAction,
    approval: &ProductionLifecycleApprovalEvidence,
) -> Result<LifecycleApplyResult> {
    let accepted = action.approved_with_evidence(approval)?;
    persist_lifecycle_action_checked(pool, &accepted, "persist_approved_lifecycle_action").await
}

async fn persist_lifecycle_action_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    action: &LifecycleAction,
    idempotency_key: &str,
    action_body: JsonValue,
    rollback_body: JsonValue,
) -> Result<LifecycleApplyResult> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(pg)?;
    super::bind_tenant_context(tx, &action.tenant_id)
        .await
        .map_err(pg)?;

    if let Some(row) = sqlx::query(
        "SELECT action_id, action_body FROM dagdb_lifecycle_actions \
         WHERE idempotency_key = $1 AND tenant_id = $2 AND project_id = $3 \
           AND memory_namespace = $4",
    )
    .bind(idempotency_key)
    .bind(&action.tenant_id)
    .bind(&action.project_id)
    .bind(&action.memory_namespace)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?
    {
        let existing_action_id = row.get::<String, _>("action_id");
        let existing_body = row.get::<JsonValue, _>("action_body");
        if existing_body == action_body {
            return Ok(LifecycleApplyResult {
                action_id: existing_action_id,
                idempotency_key: idempotency_key.to_owned(),
                replayed: true,
                terminal_state: action.terminal_state,
                route_invalidation_event_count: u32::try_from(
                    action.route_invalidation_event_ids.len(),
                )
                .map_err(|_| LifecycleActionError::CountOutOfRange)?,
            });
        }
        return Err(LifecycleActionError::DuplicateUnsafeReplay {
            idempotency_key: idempotency_key.to_owned(),
        }
        .into());
    }

    sqlx::query(
        "INSERT INTO dagdb_lifecycle_rollbacks \
         (rollback_id, action_id, inverse_action_type, before_refs, after_refs, validation_ref, operator_required, rollback_body) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&action.rollback_ref.rollback_id)
    .bind(&action.action_id)
    .bind(action.rollback_ref.inverse_action_type.as_str())
    .bind(serde_json::to_value(&action.rollback_ref.before_refs).map_err(json)?)
    .bind(serde_json::to_value(&action.rollback_ref.after_refs).map_err(json)?)
    .bind(&action.rollback_ref.validation_ref)
    .bind(action.rollback_ref.operator_required)
    .bind(rollback_body)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;

    sqlx::query(
        "INSERT INTO dagdb_lifecycle_actions \
         (action_id, action_type, tenant_id, project_id, memory_namespace, actor_id, source_packet_id, source_receipt_id, target_memory_ids, parent_memory_ids, validation_report_id, policy_ref, rollback_id, route_invalidation_event_ids, evidence_refs, terminal_state, production_lifecycle_approval, idempotency_key, action_body, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
    )
    .bind(&action.action_id)
    .bind(action.action_type.as_str())
    .bind(&action.tenant_id)
    .bind(&action.project_id)
    .bind(&action.memory_namespace)
    .bind(&action.actor_id)
    .bind(&action.source_packet_id)
    .bind(&action.source_receipt_id)
    .bind(serde_json::to_value(&action.target_memory_ids).map_err(json)?)
    .bind(serde_json::to_value(&action.parent_memory_ids).map_err(json)?)
    .bind(&action.validation_report_id)
    .bind(&action.policy_ref)
    .bind(&action.rollback_ref.rollback_id)
    .bind(serde_json::to_value(&action.route_invalidation_event_ids).map_err(json)?)
    .bind(serde_json::to_value(&action.evidence_refs).map_err(json)?)
    .bind(terminal_state(action.terminal_state))
    .bind(match action.production_lifecycle_approval {
        crate::lifecycle_action::ProductionLifecycleApproval::Approved => "approved",
        crate::lifecycle_action::ProductionLifecycleApproval::OperatorDeferred => {
            "operator_deferred"
        }
    })
    .bind(idempotency_key)
    .bind(action_body)
    .bind(&action.created_at)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;

    Ok(LifecycleApplyResult {
        action_id: action.action_id.clone(),
        idempotency_key: idempotency_key.to_owned(),
        replayed: false,
        terminal_state: action.terminal_state,
        route_invalidation_event_count: u32::try_from(action.route_invalidation_event_ids.len())
            .map_err(|_| LifecycleActionError::CountOutOfRange)?,
    })
}

fn terminal_state(state: LifecycleTerminalState) -> &'static str {
    match state {
        LifecycleTerminalState::Accepted => "accepted",
        LifecycleTerminalState::HonestBlocked => "honest_blocked",
        LifecycleTerminalState::OperatorDeferred => "operator_deferred",
        LifecycleTerminalState::FailedValidation => "failed_validation",
    }
}

fn pg(source: sqlx::Error) -> LifecycleActionPostgresError {
    LifecycleActionPostgresError::Postgres { source }
}

fn json(source: serde_json::Error) -> LifecycleActionPostgresError {
    LifecycleActionPostgresError::Json {
        reason: source.to_string(),
    }
}
