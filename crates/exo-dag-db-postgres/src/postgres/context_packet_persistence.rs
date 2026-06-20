//! PRD17B Postgres adapter for context packet persistence.

use serde_json::to_value;
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::context_packet_persistence::{
    ContextPacketAcceptanceEvidence, ContextPacketError, ContextPacketRecord,
    accept_context_packet_record, validate_context_packet_record,
};

/// Persist a PRD17B context packet record in a serializable transaction.
pub async fn persist_context_packet_record(
    pool: &PgPool,
    record: &ContextPacketRecord,
) -> Result<u64, ContextPacketPostgresError> {
    validate_context_packet_record(record).map_err(ContextPacketPostgresError::Contract)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(ContextPacketPostgresError::Sqlx)?;
    let result = persist_context_packet_record_in_transaction(&mut tx, record).await;
    match result {
        Ok(rows) => {
            tx.commit()
                .await
                .map_err(ContextPacketPostgresError::Sqlx)?;
            Ok(rows)
        }
        Err(error) => {
            if let Err(rollback_error) = tx.rollback().await {
                tracing::warn!(
                    operation = "persist_context_packet_record",
                    tenant_id = %record.tenant_id,
                    project_id = %record.project_id,
                    memory_namespace = %record.memory_namespace,
                    packet_id = %record.packet_id,
                    route_id = %record.route_id,
                    error = %rollback_error,
                    "failed to rollback transaction after context packet persistence error"
                );
            }
            Err(error)
        }
    }
}

/// Accept and persist a PRD17B context packet after approval/finality gates pass.
pub async fn persist_accepted_context_packet_record(
    pool: &PgPool,
    record: &ContextPacketRecord,
    evidence: &ContextPacketAcceptanceEvidence,
) -> Result<u64, ContextPacketPostgresError> {
    let accepted = accept_context_packet_record(record, evidence)
        .map_err(ContextPacketPostgresError::Contract)?;
    persist_context_packet_record(pool, &accepted).await
}

/// Persist a PRD17B context packet record using an existing transaction.
pub async fn persist_context_packet_record_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    record: &ContextPacketRecord,
) -> Result<u64, ContextPacketPostgresError> {
    validate_context_packet_record(record).map_err(ContextPacketPostgresError::Contract)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut **tx)
        .await
        .map_err(ContextPacketPostgresError::Sqlx)?;
    super::bind_tenant_context(tx, &record.tenant_id)
        .await
        .map_err(ContextPacketPostgresError::Sqlx)?;
    let selected_memory_ids =
        to_value(&record.selected_memory_ids).map_err(ContextPacketPostgresError::Json)?;
    let selected_edge_ids =
        to_value(&record.selected_edge_ids).map_err(ContextPacketPostgresError::Json)?;
    let source_proof_refs =
        to_value(&record.source_proof_refs).map_err(ContextPacketPostgresError::Json)?;

    // Replay guard mirroring the PRD17C lifecycle adapter: an existing
    // packet_id may only be re-persisted with the exact same scope and body.
    // Anything else (cross-tenant clobber, mutated proof-bound contents) is an
    // unsafe replay and fails closed.
    if let Some(existing) = sqlx::query(
        "SELECT \
           (tenant_id = $2 AND project_id = $3 AND memory_namespace = $4) AS scope_matches, \
           (route_id = $5 AND query_hash = $6 AND selected_memory_ids = $7 \
            AND selected_edge_ids = $8 AND token_budget = $9 AND token_estimate = $10 \
            AND context_quality = $11 AND citation_coverage_bp = $12 \
            AND validation_coverage_bp = $13 AND freshness_status = $14 \
            AND validation_status = $15 AND source_proof_refs = $16 \
            AND fallback_reason IS NOT DISTINCT FROM $17 AND idempotency_key = $18 \
            AND persistence_status = $19 \
            AND production_default_route_approval_status = $20 \
            AND packet_quality_review_status = $21 AND created_at = $22) AS body_matches \
         FROM dagdb_context_packet_records \
         WHERE packet_id = $1 AND tenant_id = $2 AND project_id = $3 AND memory_namespace = $4",
    )
    .bind(&record.packet_id)
    .bind(&record.tenant_id)
    .bind(&record.project_id)
    .bind(&record.memory_namespace)
    .bind(&record.route_id)
    .bind(&record.query_hash)
    .bind(&selected_memory_ids)
    .bind(&selected_edge_ids)
    .bind(i32::try_from(record.token_budget).unwrap_or(i32::MAX))
    .bind(i32::try_from(record.token_estimate).unwrap_or(i32::MAX))
    .bind(serde_label(&record.context_quality)?)
    .bind(i32::from(record.citation_coverage_bp))
    .bind(i32::from(record.validation_coverage_bp))
    .bind(serde_label(&record.freshness_status)?)
    .bind(serde_label(&record.validation_status)?)
    .bind(&source_proof_refs)
    .bind(&record.fallback_reason)
    .bind(&record.idempotency_key)
    .bind(serde_label(&record.persistence_status)?)
    .bind(&record.production_default_route_approval_status)
    .bind(&record.packet_quality_review_status)
    .bind(&record.created_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ContextPacketPostgresError::Sqlx)?
    {
        let scope_matches: bool = existing
            .try_get("scope_matches")
            .map_err(ContextPacketPostgresError::Sqlx)?;
        let body_matches: bool = existing
            .try_get("body_matches")
            .map_err(ContextPacketPostgresError::Sqlx)?;
        if scope_matches && body_matches {
            // Exact idempotent replay: nothing is rewritten.
            return Ok(0);
        }
        return Err(ContextPacketPostgresError::UnsafeReplay {
            packet_id: record.packet_id.clone(),
        });
    }

    let result = sqlx::query(
        r#"
        INSERT INTO dagdb_context_packet_records (
          packet_id,
          route_id,
          query_hash,
          tenant_id,
          project_id,
          memory_namespace,
          selected_memory_ids,
          selected_edge_ids,
          token_budget,
          token_estimate,
          context_quality,
          citation_coverage_bp,
          validation_coverage_bp,
          freshness_status,
          validation_status,
          source_proof_refs,
          fallback_reason,
          idempotency_key,
          persistence_status,
          production_default_route_approval_status,
          packet_quality_review_status,
          created_at
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
          $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
        )
        ON CONFLICT (packet_id)
        DO UPDATE SET
          route_id = EXCLUDED.route_id,
          query_hash = EXCLUDED.query_hash,
          selected_memory_ids = EXCLUDED.selected_memory_ids,
          selected_edge_ids = EXCLUDED.selected_edge_ids,
          token_budget = EXCLUDED.token_budget,
          token_estimate = EXCLUDED.token_estimate,
          context_quality = EXCLUDED.context_quality,
          citation_coverage_bp = EXCLUDED.citation_coverage_bp,
          validation_coverage_bp = EXCLUDED.validation_coverage_bp,
          freshness_status = EXCLUDED.freshness_status,
          validation_status = EXCLUDED.validation_status,
          source_proof_refs = EXCLUDED.source_proof_refs,
          fallback_reason = EXCLUDED.fallback_reason,
          idempotency_key = EXCLUDED.idempotency_key,
          persistence_status = EXCLUDED.persistence_status,
          production_default_route_approval_status =
            EXCLUDED.production_default_route_approval_status,
          packet_quality_review_status = EXCLUDED.packet_quality_review_status
        WHERE dagdb_context_packet_records.tenant_id = EXCLUDED.tenant_id
          AND dagdb_context_packet_records.project_id = EXCLUDED.project_id
          AND dagdb_context_packet_records.memory_namespace = EXCLUDED.memory_namespace
        "#,
    )
    .bind(&record.packet_id)
    .bind(&record.route_id)
    .bind(&record.query_hash)
    .bind(&record.tenant_id)
    .bind(&record.project_id)
    .bind(&record.memory_namespace)
    .bind(selected_memory_ids)
    .bind(selected_edge_ids)
    .bind(i32::try_from(record.token_budget).unwrap_or(i32::MAX))
    .bind(i32::try_from(record.token_estimate).unwrap_or(i32::MAX))
    .bind(serde_label(&record.context_quality)?)
    .bind(i32::from(record.citation_coverage_bp))
    .bind(i32::from(record.validation_coverage_bp))
    .bind(serde_label(&record.freshness_status)?)
    .bind(serde_label(&record.validation_status)?)
    .bind(source_proof_refs)
    .bind(&record.fallback_reason)
    .bind(&record.idempotency_key)
    .bind(serde_label(&record.persistence_status)?)
    .bind(&record.production_default_route_approval_status)
    .bind(&record.packet_quality_review_status)
    .bind(&record.created_at)
    .execute(&mut **tx)
    .await
    .map_err(ContextPacketPostgresError::Sqlx)?;
    if result.rows_affected() == 0 {
        // A concurrent conflicting row blocked the scope-guarded upsert.
        return Err(ContextPacketPostgresError::UnsafeReplay {
            packet_id: record.packet_id.clone(),
        });
    }
    Ok(result.rows_affected())
}

/// Errors raised by the PRD17B context-packet Postgres adapter.
#[derive(Debug, thiserror::Error)]
pub enum ContextPacketPostgresError {
    /// Packet contract failed.
    #[error("context_packet_contract_failed")]
    Contract(#[source] ContextPacketError),
    /// JSON serialization failed.
    #[error("context_packet_json_failed")]
    Json(#[source] serde_json::Error),
    /// Replay of an existing packet_id with a different scope or body.
    #[error("context_packet_unsafe_replay: {packet_id}")]
    UnsafeReplay {
        /// Packet identifier whose replay was rejected.
        packet_id: String,
    },
    /// SQL execution failed.
    #[error("context_packet_sql_failed")]
    Sqlx(#[source] sqlx::Error),
}

fn serde_label<T: serde::Serialize>(value: &T) -> Result<String, ContextPacketPostgresError> {
    let label = serde_json::to_value(value).map_err(ContextPacketPostgresError::Json)?;
    label
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ContextPacketPostgresError::Json(label_to_string_error()))
}

fn label_to_string_error() -> serde_json::Error {
    serde::de::Error::custom("enum label did not serialize to a string")
}
