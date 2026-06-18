//! PRD17B Postgres adapter for default route records.
//!
//! This adapter is intentionally scoped to route readiness persistence. Route
//! invalidation mutation remains PRD17C-owned.

use serde_json::to_value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::default_route::{DefaultRouteError, DefaultRouteRecord, validate_default_route_record};

/// Persist a PRD17B default route in a serializable transaction.
pub async fn persist_default_route(
    pool: &PgPool,
    route: &DefaultRouteRecord,
) -> Result<u64, DefaultRoutePostgresError> {
    validate_default_route_record(route).map_err(DefaultRoutePostgresError::Contract)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(DefaultRoutePostgresError::Sqlx)?;
    let result = persist_default_route_in_transaction(&mut tx, route).await;
    match result {
        Ok(rows) => {
            tx.commit().await.map_err(DefaultRoutePostgresError::Sqlx)?;
            Ok(rows)
        }
        Err(error) => {
            if let Err(rollback_error) = tx.rollback().await {
                tracing::warn!(
                    operation = "persist_default_route",
                    tenant_id = %route.tenant_id,
                    project_id = %route.project_id,
                    memory_namespace = %route.memory_namespace,
                    route_id = %route.route_id,
                    error = %rollback_error,
                    "failed to rollback transaction after default route persistence error"
                );
            }
            Err(error)
        }
    }
}

/// Persist a PRD17B default route using an existing transaction.
pub async fn persist_default_route_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    route: &DefaultRouteRecord,
) -> Result<u64, DefaultRoutePostgresError> {
    validate_default_route_record(route).map_err(DefaultRoutePostgresError::Contract)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(DefaultRoutePostgresError::Sqlx)?;
    let selected_memory_refs =
        to_value(&route.selected_memory_refs).map_err(DefaultRoutePostgresError::Json)?;
    let result = sqlx::query(
        r#"
        INSERT INTO dagdb_default_routes (
          tenant_id,
          project_id,
          memory_namespace,
          route_id,
          status,
          route_source,
          policy_ref,
          freshness_ref,
          policy_allowed,
          freshness_status,
          invalidated,
          production_default_route_approval_status,
          packet_quality_review_status,
          selected_memory_refs,
          selected_memory_ref_count,
          created_at,
          updated_at
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
          $14, $15, $16, $17
        )
        ON CONFLICT (tenant_id, project_id, memory_namespace, route_id)
        DO UPDATE SET
          status = EXCLUDED.status,
          route_source = EXCLUDED.route_source,
          policy_ref = EXCLUDED.policy_ref,
          freshness_ref = EXCLUDED.freshness_ref,
          policy_allowed = EXCLUDED.policy_allowed,
          freshness_status = EXCLUDED.freshness_status,
          invalidated = EXCLUDED.invalidated,
          production_default_route_approval_status =
            EXCLUDED.production_default_route_approval_status,
          packet_quality_review_status = EXCLUDED.packet_quality_review_status,
          selected_memory_refs = EXCLUDED.selected_memory_refs,
          selected_memory_ref_count = EXCLUDED.selected_memory_ref_count,
          updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(&route.tenant_id)
    .bind(&route.project_id)
    .bind(&route.memory_namespace)
    .bind(&route.route_id)
    .bind(serde_label(&route.status)?)
    .bind(serde_label(&route.route_source)?)
    .bind(&route.policy_ref)
    .bind(&route.freshness_ref)
    .bind(route.policy_allowed)
    .bind(serde_label(&route.freshness_status)?)
    .bind(route.invalidated)
    .bind(&route.production_default_route_approval_status)
    .bind(&route.packet_quality_review_status)
    .bind(selected_memory_refs)
    .bind(i32::try_from(route.selected_memory_refs.len()).unwrap_or(i32::MAX))
    .bind(&route.created_at)
    .bind(&route.updated_at)
    .execute(&mut **tx)
    .await
    .map_err(DefaultRoutePostgresError::Sqlx)?;
    Ok(result.rows_affected())
}

/// Errors raised by the PRD17B default-route Postgres adapter.
#[derive(Debug, thiserror::Error)]
pub enum DefaultRoutePostgresError {
    /// Route contract failed.
    #[error("default_route_contract_failed")]
    Contract(#[source] DefaultRouteError),
    /// JSON serialization failed.
    #[error("default_route_json_failed")]
    Json(#[source] serde_json::Error),
    /// SQL execution failed.
    #[error("default_route_sql_failed")]
    Sqlx(#[source] sqlx::Error),
}

fn serde_label<T: serde::Serialize>(value: &T) -> Result<String, DefaultRoutePostgresError> {
    let label = serde_json::to_value(value).map_err(DefaultRoutePostgresError::Json)?;
    label
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| DefaultRoutePostgresError::Json(label_to_string_error()))
}

fn label_to_string_error() -> serde_json::Error {
    serde::de::Error::custom("enum label did not serialize to a string")
}
