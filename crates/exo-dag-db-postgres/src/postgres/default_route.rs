//! PRD17B Postgres adapter for default route records.
//!
//! This adapter is intentionally scoped to route readiness persistence. Route
//! invalidation mutation remains PRD17C-owned.

use exo_core::Timestamp;
use exo_dag_db_api::{ReceiptEventType, SubjectKind};
use serde_json::{json, to_value};
use sqlx::{PgPool, Postgres, Transaction};

use crate::{
    default_route::{
        DefaultRouteAcceptanceEvidence, DefaultRouteError, DefaultRouteRecord,
        accept_default_route_record, validate_default_route_record,
    },
    receipt::{
        OperationalReceiptInsert, ReceiptStoreError, insert_operational_receipt_in_transaction,
        operational_receipt_subject_id,
    },
    scoring::hash_event_body,
};

const DEFAULT_ROUTE_AUDIT_ACTOR_DID: &str = "did:exo:dagdb-default-route-writer";
const DEFAULT_ROUTE_AUDIT_ROUTE_NAME: &str = "dagdb.route";
const CREATED_AT: Timestamp = Timestamp::new(1, 0);

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

/// Accept and persist a PRD17B default route after approval/finality gates pass.
pub async fn persist_accepted_default_route(
    pool: &PgPool,
    route: &DefaultRouteRecord,
    evidence: &DefaultRouteAcceptanceEvidence,
    updated_at: String,
) -> Result<u64, DefaultRoutePostgresError> {
    let accepted = accept_default_route_record(route, evidence, updated_at)
        .map_err(DefaultRoutePostgresError::Contract)?;
    persist_default_route(pool, &accepted).await
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
    super::bind_tenant_context(tx, &route.tenant_id)
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
    insert_default_route_approval_receipts(tx, route).await?;
    insert_default_route_record_accepted_receipt(tx, route).await?;
    Ok(result.rows_affected())
}

async fn insert_default_route_approval_receipts(
    tx: &mut Transaction<'_, Postgres>,
    route: &DefaultRouteRecord,
) -> Result<u64, DefaultRoutePostgresError> {
    let mut inserted = 0_u64;
    for event_type in [
        ReceiptEventType::DagdbApprovalRequestSubmitted,
        ReceiptEventType::DagdbApprovalGranted,
    ] {
        let receipt_body = json!({
            "route_name": DEFAULT_ROUTE_AUDIT_ROUTE_NAME,
            "route_id": route.route_id,
            "project_id": route.project_id,
            "source": "default_route_persistence_adapter",
        });
        let event_body_hash =
            hash_event_body(&receipt_body).map_err(DefaultRoutePostgresError::ReceiptHash)?;
        inserted = inserted.saturating_add(
            insert_operational_receipt_in_transaction(
                tx,
                OperationalReceiptInsert {
                    tenant_id: &route.tenant_id,
                    namespace: &route.memory_namespace,
                    subject_kind: SubjectKind::Route,
                    subject_id: operational_receipt_subject_id(
                        DEFAULT_ROUTE_AUDIT_ROUTE_NAME,
                        &route.route_id,
                        event_type,
                    ),
                    event_type,
                    actor_did: DEFAULT_ROUTE_AUDIT_ACTOR_DID,
                    event_hlc: CREATED_AT,
                    event_body_hash,
                    receipt_body,
                },
            )
            .await?,
        );
    }
    Ok(inserted)
}

async fn insert_default_route_record_accepted_receipt(
    tx: &mut Transaction<'_, Postgres>,
    route: &DefaultRouteRecord,
) -> Result<u64, DefaultRoutePostgresError> {
    if route.production_default_route_approval_status != "accepted"
        || route.packet_quality_review_status != "accepted"
    {
        return Ok(0);
    }
    let event_type = ReceiptEventType::DagdbRecordAccepted;
    let receipt_body = json!({
        "route_name": DEFAULT_ROUTE_AUDIT_ROUTE_NAME,
        "route_id": route.route_id,
        "request_id": route.request_id,
        "source": "default_route_persistence_adapter",
    });
    let event_body_hash =
        hash_event_body(&receipt_body).map_err(DefaultRoutePostgresError::ReceiptHash)?;
    insert_operational_receipt_in_transaction(
        tx,
        OperationalReceiptInsert {
            tenant_id: &route.tenant_id,
            namespace: &route.memory_namespace,
            subject_kind: SubjectKind::Route,
            subject_id: operational_receipt_subject_id(
                DEFAULT_ROUTE_AUDIT_ROUTE_NAME,
                &route.route_id,
                event_type,
            ),
            event_type,
            actor_did: DEFAULT_ROUTE_AUDIT_ACTOR_DID,
            event_hlc: CREATED_AT,
            event_body_hash,
            receipt_body,
        },
    )
    .await
    .map_err(DefaultRoutePostgresError::Receipt)
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
    /// Receipt hash material failed.
    #[error("default_route_receipt_hash_failed")]
    ReceiptHash(#[source] crate::scoring::DomainError),
    /// Receipt audit write failed.
    #[error("default_route_receipt_failed")]
    Receipt(#[from] ReceiptStoreError),
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
