//! Feature-gated Postgres adapter for PRD17C route invalidation events.

use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use thiserror::Error;

use crate::route_invalidation::{
    RouteFreshnessState, RouteInvalidationApplyResult, RouteInvalidationError,
    RouteInvalidationEvent,
};

/// Errors raised by PRD17C route invalidation Postgres persistence.
#[derive(Debug, Error)]
pub enum RouteInvalidationPostgresError {
    #[error(transparent)]
    RouteInvalidation(#[from] RouteInvalidationError),
    #[error("prd17_route_invalidation_postgres_failed")]
    Postgres {
        #[source]
        source: sqlx::Error,
    },
    #[error("prd17_route_invalidation_json_failed: {reason}")]
    Json { reason: String },
}

/// Result alias for route invalidation Postgres persistence.
pub type Result<T> = std::result::Result<T, RouteInvalidationPostgresError>;

/// Persist a validated route invalidation event.
pub async fn persist_route_invalidation_event(
    pool: &PgPool,
    event: &RouteInvalidationEvent,
) -> Result<RouteInvalidationApplyResult> {
    event.validate()?;
    let idempotency_key = event.idempotency_key()?;
    let rollback_idempotency_key = idempotency_key.clone();
    let event_body = serde_json::to_value(event).map_err(json)?;

    let mut tx = pool.begin().await.map_err(pg)?;
    let result = async {
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await
            .map_err(pg)?;

        if let Some(row) = sqlx::query(
            "SELECT event_id, event_body FROM dagdb_route_invalidation_events \
             WHERE idempotency_key = $1 AND tenant_id = $2 AND project_id = $3 \
               AND memory_namespace = $4",
        )
        .bind(&idempotency_key)
        .bind(&event.tenant_id)
        .bind(&event.project_id)
        .bind(&event.memory_namespace)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg)?
        {
            let existing_event_id = row.get::<String, _>("event_id");
            let existing_body = row.get::<JsonValue, _>("event_body");
            if existing_body == event_body {
                return Ok(RouteInvalidationApplyResult {
                    event_id: existing_event_id,
                    route_id: event.route_id.clone(),
                    idempotency_key,
                    replayed: true,
                    freshness_state_after: RouteFreshnessState::Stale,
                });
            }
            return Err(RouteInvalidationError::DuplicateUnsafeReplay { idempotency_key }.into());
        }

        sqlx::query(
            "INSERT INTO dagdb_route_invalidation_events \
             (event_id, tenant_id, project_id, memory_namespace, route_id, source_action_id, impacted_memory_ids, reason, invalidated_packet_ids, freshness_state_before, freshness_state_after, retrieval_readiness_impact, validation_report_id, rollback_ref, idempotency_key, event_body, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'current', 'stale', 'reject_until_rebuilt', $10, $11, $12, $13, $14)",
        )
        .bind(&event.event_id)
        .bind(&event.tenant_id)
        .bind(&event.project_id)
        .bind(&event.memory_namespace)
        .bind(&event.route_id)
        .bind(&event.source_action_id)
        .bind(serde_json::to_value(&event.impacted_memory_ids).map_err(json)?)
        .bind(&event.reason)
        .bind(serde_json::to_value(&event.invalidated_packet_ids).map_err(json)?)
        .bind(&event.validation_report_id)
        .bind(&event.rollback_ref)
        .bind(&idempotency_key)
        .bind(event_body)
        .bind(&event.created_at)
        .execute(&mut *tx)
        .await
        .map_err(pg)?;

        Ok(RouteInvalidationApplyResult {
            event_id: event.event_id.clone(),
            route_id: event.route_id.clone(),
            idempotency_key,
            replayed: false,
            freshness_state_after: RouteFreshnessState::Stale,
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
                    operation = "persist_route_invalidation_event",
                    tenant_id = %event.tenant_id,
                    project_id = %event.project_id,
                    memory_namespace = %event.memory_namespace,
                    route_id = %event.route_id,
                    event_id = %event.event_id,
                    source_action_id = %event.source_action_id,
                    idempotency_key = %rollback_idempotency_key,
                    error = %rollback_error,
                    "failed to rollback transaction after route invalidation persistence error"
                );
            }
            Err(error)
        }
    }
}

fn pg(source: sqlx::Error) -> RouteInvalidationPostgresError {
    RouteInvalidationPostgresError::Postgres { source }
}

fn json(source: serde_json::Error) -> RouteInvalidationPostgresError {
    RouteInvalidationPostgresError::Json {
        reason: source.to_string(),
    }
}
