//! PostgreSQL-backed mutation idempotency replay and conflict handling.

use exo_core::{Hash256, Timestamp};
use sqlx::{PgPool, Row, types::JsonValue};
use thiserror::Error;

/// Request to persist a cacheable idempotency response.
#[derive(Debug, Clone)]
pub struct IdempotencyRecordRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub route_name: String,
    pub idempotency_key: String,
    pub request_hash: Hash256,
    pub response_body: JsonValue,
    pub status_code: u16,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Stored response returned for idempotency replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedIdempotencyResponse {
    pub response_hash: Hash256,
    pub response_body: JsonValue,
    pub status_code: u16,
    pub cached_failure: bool,
}

/// Idempotency store decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyDecision {
    Stored(CachedIdempotencyResponse),
    Replayed(CachedIdempotencyResponse),
    Conflict { error_code: &'static str },
    NotCached { reason: &'static str },
}

/// Errors from idempotency storage.
#[derive(Debug, Error)]
pub enum IdempotencyStoreError {
    #[error("idempotency hash column had invalid length")]
    InvalidHashLength,
    #[error("idempotency timestamp is out of SQL range")]
    TimestampOutOfRange,
    #[error("idempotency status code is out of range")]
    StatusCodeOutOfRange,
    #[error("postgres idempotency operation failed")]
    Postgres {
        #[source]
        source: sqlx::Error,
    },
}

/// Idempotency result alias.
pub type Result<T> = std::result::Result<T, IdempotencyStoreError>;

/// Store or replay a cacheable mutation response for an idempotency key.
pub async fn store_idempotency_response(
    pool: &PgPool,
    request: &IdempotencyRecordRequest,
) -> Result<IdempotencyDecision> {
    let Some(cached_failure) = cacheable_failure_flag(request) else {
        return Ok(IdempotencyDecision::NotCached {
            reason: "uncacheable_failure",
        });
    };

    let mut tx = pool.begin().await.map_err(pg)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await
        .map_err(pg)?;
    crate::postgres::bind_tenant_context(&mut tx, &request.tenant_id)
        .await
        .map_err(pg)?;

    if let Some(existing) = fetch_existing(request, &mut tx).await? {
        tx.commit().await.map_err(pg)?;
        if existing.request_hash == request.request_hash {
            return Ok(IdempotencyDecision::Replayed(existing.cached));
        }
        return Ok(IdempotencyDecision::Conflict {
            error_code: "idempotency_key_conflict",
        });
    }

    let cached = cached_response(request, cached_failure)?;
    insert_response(request, &cached, &mut tx).await?;
    tx.commit().await.map_err(pg)?;
    Ok(IdempotencyDecision::Stored(cached))
}

fn cacheable_failure_flag(request: &IdempotencyRecordRequest) -> Option<bool> {
    if (200..300).contains(&request.status_code) {
        return Some(false);
    }
    if request.status_code == 409
        && request
            .response_body
            .get("error_code")
            .and_then(JsonValue::as_str)
            == Some("duplicate_active_memory")
    {
        return Some(true);
    }
    None
}

#[derive(Debug, Clone)]
struct ExistingIdempotencyRow {
    request_hash: Hash256,
    cached: CachedIdempotencyResponse,
}

async fn fetch_existing(
    request: &IdempotencyRecordRequest,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<Option<ExistingIdempotencyRow>> {
    let row = sqlx::query(
        "SELECT request_hash, response_hash, response_body, status_code, cached_failure \
         FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4 \
         FOR UPDATE",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(&request.route_name)
    .bind(&request.idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg)?;

    row.map(|row| {
        let status_code: i32 = row.try_get("status_code").map_err(pg)?;
        Ok(ExistingIdempotencyRow {
            request_hash: hash_from_vec(row.try_get("request_hash").map_err(pg)?)?,
            cached: CachedIdempotencyResponse {
                response_hash: hash_from_vec(row.try_get("response_hash").map_err(pg)?)?,
                response_body: row.try_get("response_body").map_err(pg)?,
                status_code: u16::try_from(status_code)
                    .map_err(|_| IdempotencyStoreError::StatusCodeOutOfRange)?,
                cached_failure: row.try_get("cached_failure").map_err(pg)?,
            },
        })
    })
    .transpose()
}

async fn insert_response(
    request: &IdempotencyRecordRequest,
    cached: &CachedIdempotencyResponse,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<()> {
    let created_at = timestamp_parts(request.created_at)?;
    let expires_at = timestamp_parts(request.expires_at)?;
    let status_code = i32::from(request.status_code);
    sqlx::query(
        "INSERT INTO dagdb_idempotency_keys \
         (tenant_id, namespace, route_name, idempotency_key, request_hash, response_hash, response_body, \
          status_code, cached_failure, created_at_physical_ms, created_at_logical, \
          expires_at_physical_ms, expires_at_logical) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&request.tenant_id)
    .bind(&request.namespace)
    .bind(&request.route_name)
    .bind(&request.idempotency_key)
    .bind(hash_bytes(request.request_hash))
    .bind(hash_bytes(cached.response_hash))
    .bind(cached.response_body.clone())
    .bind(status_code)
    .bind(cached.cached_failure)
    .bind(created_at.physical_ms)
    .bind(created_at.logical)
    .bind(expires_at.physical_ms)
    .bind(expires_at.logical)
    .execute(&mut **tx)
    .await
    .map_err(pg)?;
    Ok(())
}

fn cached_response(
    request: &IdempotencyRecordRequest,
    cached_failure: bool,
) -> Result<CachedIdempotencyResponse> {
    Ok(CachedIdempotencyResponse {
        response_hash: Hash256::digest(request.response_body.to_string().as_bytes()),
        response_body: request.response_body.clone(),
        status_code: request.status_code,
        cached_failure,
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
            .map_err(|_| IdempotencyStoreError::TimestampOutOfRange)?,
        logical: i32::try_from(timestamp.logical)
            .map_err(|_| IdempotencyStoreError::TimestampOutOfRange)?,
    })
}

fn hash_bytes(hash: Hash256) -> Vec<u8> {
    hash.as_bytes().to_vec()
}

fn hash_from_vec(bytes: Vec<u8>) -> Result<Hash256> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| IdempotencyStoreError::InvalidHashLength)?;
    Ok(Hash256::from_bytes(bytes))
}

fn pg(source: sqlx::Error) -> IdempotencyStoreError {
    IdempotencyStoreError::Postgres { source }
}
