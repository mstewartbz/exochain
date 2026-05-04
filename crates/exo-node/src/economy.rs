//! Node API surface for the custody-native economy layer.
//!
//! Routes are merged into the gateway's axum router via the same
//! `serve_with_extra_routes` pattern used elsewhere on the node. POSTs
//! inherit bearer-token auth from the merged write guard. Reads are
//! public so quoting is free of authentication friction.
//!
//! All settlement mechanics are preserved end-to-end. During the launch
//! phase, every quote and settlement returns `charged_amount_micro_exo
//! = 0` with an explicit `zero_fee_reason`. Trust is never paywalled.
//!
//! ## Routes
//!
//! | Method | Path | Purpose |
//! |--------|------|---------|
//! | `POST` | `/api/v1/economy/quote` | Build a deterministic settlement quote. |
//! | `POST` | `/api/v1/economy/settle` | Settle a previously issued quote. |
//! | `GET`  | `/api/v1/economy/receipts/:id` | Fetch a settlement receipt. |
//! | `GET`  | `/api/v1/economy/policy/active` | Inspect the active pricing policy. |

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    routing::{get, post},
};
use exo_core::{Hash256, Signature};
use exo_economy::{
    EconomyError, EconomyStore, InMemoryEconomyStore, PricingInputs, PricingPolicy,
    SettlementContext, SettlementQuote, SettlementReceipt, quote, settle,
};
use serde::{Deserialize, Serialize};
use tower::limit::ConcurrencyLimitLayer;

const MAX_ECONOMY_API_BODY_BYTES: usize = 64 * 1024;
const MAX_ECONOMY_API_CONCURRENT_REQUESTS: usize = 64;

/// Shared state for economy route handlers.
#[derive(Clone)]
pub struct EconomyApiState {
    pub store: Arc<Mutex<InMemoryEconomyStore>>,
}

impl EconomyApiState {
    /// Construct a fresh state seeded with the zero-launch policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(InMemoryEconomyStore::new())),
        }
    }
}

impl Default for EconomyApiState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Request / Response shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub quote_id: String,
    pub inputs: PricingInputs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettleRequest {
    pub quote_hash_hex: String,
    pub context: SettlementContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyResponse {
    pub policy: PricingPolicy,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, String);
type ApiResult<T> = Result<T, ApiError>;

fn parse_hash(raw: &str) -> ApiResult<Hash256> {
    let bytes = hex::decode(raw).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "quote_hash_hex must be lowercase hex".into(),
        )
    })?;
    if bytes.len() != 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "quote_hash_hex must be 32 bytes (64 hex chars)".into(),
        ));
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    Ok(Hash256::from_bytes(buf))
}

async fn with_store_blocking<T, F>(state: Arc<EconomyApiState>, op: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce(&mut InMemoryEconomyStore) -> ApiResult<T> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut guard = state.store.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "economy store unavailable".into(),
            )
        })?;
        op(&mut guard)
    })
    .await
    .map_err(|err| {
        tracing::error!(err = %err, "economy store task failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "economy store task failed".into(),
        )
    })?
}

fn map_economy_error(err: EconomyError) -> ApiError {
    tracing::warn!(?err, "economy operation rejected");
    match err {
        EconomyError::EmptyField { .. }
        | EconomyError::BasisPointOutOfRange { .. }
        | EconomyError::FloorAboveCeiling { .. }
        | EconomyError::InvalidInput { .. } => (StatusCode::BAD_REQUEST, err.to_string()),
        EconomyError::QuoteExpired => (StatusCode::CONFLICT, err.to_string()),
        EconomyError::QuoteNotFound | EconomyError::UnknownPolicy { .. } => {
            (StatusCode::NOT_FOUND, err.to_string())
        }
        EconomyError::QuoteHashMismatch
        | EconomyError::ReceiptHashMismatch
        | EconomyError::RevenueShareOverAllocated { .. }
        | EconomyError::SettlementOverAllocated { .. } => {
            (StatusCode::UNPROCESSABLE_ENTITY, err.to_string())
        }
        EconomyError::Serialization { .. } | EconomyError::ZeroLaunchInvariantViolated { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, "economy error".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_quote(
    State(state): State<Arc<EconomyApiState>>,
    Json(payload): Json<QuoteRequest>,
) -> ApiResult<Json<SettlementQuote>> {
    let result = with_store_blocking(state, move |store| {
        let policy = store.get_active_policy().map_err(map_economy_error)?;
        let quote_record =
            quote(&policy, &payload.inputs, payload.quote_id).map_err(map_economy_error)?;
        // Allow re-quoting the same canonical inputs by ignoring duplicate-hash
        // errors on the put: the stored record is identical to the new one.
        if let Err(EconomyError::InvalidInput { .. }) = store.put_quote(quote_record.clone()) {
            // Existing canonical quote already stored — no-op.
        }
        Ok(quote_record)
    })
    .await?;
    Ok(Json(result))
}

async fn handle_settle(
    State(state): State<Arc<EconomyApiState>>,
    Json(payload): Json<SettleRequest>,
) -> ApiResult<Json<SettlementReceipt>> {
    let quote_hash = parse_hash(&payload.quote_hash_hex)?;
    let context = payload.context;
    let receipt = with_store_blocking(state, move |store| {
        let stored = store
            .get_quote(&quote_hash)
            .map_err(map_economy_error)?
            .ok_or_else(|| map_economy_error(EconomyError::QuoteNotFound))?;
        let receipt =
            settle(&stored, &context, |_| Signature::empty()).map_err(map_economy_error)?;
        store
            .put_receipt(receipt.clone())
            .map_err(map_economy_error)?;
        Ok(receipt)
    })
    .await?;
    Ok(Json(receipt))
}

async fn handle_get_receipt(
    State(state): State<Arc<EconomyApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<SettlementReceipt>> {
    let receipt = with_store_blocking(state, move |store| {
        store
            .get_receipt(&id)
            .map_err(map_economy_error)?
            .ok_or((StatusCode::NOT_FOUND, "receipt not found".into()))
    })
    .await?;
    Ok(Json(receipt))
}

async fn handle_active_policy(
    State(state): State<Arc<EconomyApiState>>,
) -> ApiResult<Json<PolicyResponse>> {
    let policy = with_store_blocking(state, move |store| {
        store.get_active_policy().map_err(map_economy_error)
    })
    .await?;
    Ok(Json(PolicyResponse { policy }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn economy_router(state: Arc<EconomyApiState>) -> Router {
    Router::new()
        .route("/api/v1/economy/quote", post(handle_quote))
        .route("/api/v1/economy/settle", post(handle_settle))
        .route("/api/v1/economy/receipts/:id", get(handle_get_receipt))
        .route("/api/v1/economy/policy/active", get(handle_active_policy))
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_ECONOMY_API_BODY_BYTES))
        .layer(ConcurrencyLimitLayer::new(
            MAX_ECONOMY_API_CONCURRENT_REQUESTS,
        ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{self, Body};
    use axum::http::{Method, Request};
    use exo_core::{Did, Timestamp};
    use exo_economy::{ActorClass, AssuranceClass, EventClass, ZeroFeeReason};
    use tower::ServiceExt;

    fn fresh_state() -> Arc<EconomyApiState> {
        Arc::new(EconomyApiState::new())
    }

    fn baseline_inputs() -> PricingInputs {
        PricingInputs {
            actor_did: Did::new("did:exo:agent").unwrap(),
            actor_class: ActorClass::Holon,
            event_class: EventClass::HolonCommercialAction,
            assurance_class: AssuranceClass::Standard,
            declared_value_micro_exo: Some(1_000_000),
            realized_value_micro_exo: None,
            compute_units: 100,
            storage_bytes: 4_096,
            verification_ops: 5,
            network_load_bp: 10_000,
            risk_bp: 1_500,
            market_domain: None,
            timestamp: Timestamp::new(1_000_000, 0),
        }
    }

    async fn read_body(response: axum::response::Response) -> Vec<u8> {
        body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    async fn quote_returns_zero_amount_with_zero_fee_reason() {
        let state = fresh_state();
        let app = economy_router(Arc::clone(&state));
        let payload = QuoteRequest {
            quote_id: "q-1".into(),
            inputs: baseline_inputs(),
        };
        let body = serde_json::to_vec(&payload).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/quote")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let parsed: SettlementQuote = serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed.charged_amount_micro_exo, 0);
        assert!(parsed.zero_fee_reason.is_some());
    }

    #[tokio::test]
    async fn settle_creates_zero_priced_receipt() {
        let state = fresh_state();
        let app = economy_router(Arc::clone(&state));

        // Step 1: quote.
        let payload = QuoteRequest {
            quote_id: "q-1".into(),
            inputs: baseline_inputs(),
        };
        let body = serde_json::to_vec(&payload).unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/quote")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let issued: SettlementQuote = serde_json::from_slice(&read_body(response).await).unwrap();

        // Step 2: settle.
        let settle_request = SettleRequest {
            quote_hash_hex: format!("{}", issued.quote_hash),
            context: SettlementContext {
                receipt_id: "rec-1".into(),
                custody_transaction_hash: Hash256::from_bytes([0x33; 32]),
                prev_settlement_receipt: Hash256::ZERO,
                now: Timestamp::new(1_010_000, 0),
            },
        };
        let body = serde_json::to_vec(&settle_request).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/settle")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let receipt: SettlementReceipt =
            serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(receipt.charged_amount_micro_exo, 0);
        assert!(receipt.zero_fee_reason.is_some());
    }

    #[tokio::test]
    async fn settle_returns_404_for_unknown_quote() {
        let state = fresh_state();
        let app = economy_router(Arc::clone(&state));
        let settle_request = SettleRequest {
            quote_hash_hex: "11".repeat(32),
            context: SettlementContext {
                receipt_id: "rec-1".into(),
                custody_transaction_hash: Hash256::ZERO,
                prev_settlement_receipt: Hash256::ZERO,
                now: Timestamp::new(1_010_000, 0),
            },
        };
        let body = serde_json::to_vec(&settle_request).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/settle")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn settle_returns_400_for_invalid_quote_hash_hex() {
        let state = fresh_state();
        let app = economy_router(Arc::clone(&state));
        let settle_request = SettleRequest {
            quote_hash_hex: "not-hex".into(),
            context: SettlementContext {
                receipt_id: "rec-1".into(),
                custody_transaction_hash: Hash256::ZERO,
                prev_settlement_receipt: Hash256::ZERO,
                now: Timestamp::new(1_010_000, 0),
            },
        };
        let body = serde_json::to_vec(&settle_request).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/settle")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_receipt_404_when_unknown() {
        let state = fresh_state();
        let app = economy_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/economy/receipts/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn active_policy_returns_zero_launch_default() {
        let state = fresh_state();
        let app = economy_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/economy/policy/active")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let parsed: PolicyResponse = serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed.policy.id, "exo.economy.zero-launch");
        assert_eq!(parsed.policy.compute_unit_price_micro_exo, 0);
    }

    #[tokio::test]
    async fn quote_returns_400_for_blank_quote_id() {
        let state = fresh_state();
        let app = economy_router(state);
        let payload = QuoteRequest {
            quote_id: "   ".into(),
            inputs: baseline_inputs(),
        };
        let body = serde_json::to_vec(&payload).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/quote")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn quote_zero_fee_reason_for_human_avc_validate_is_agent_validation() {
        let state = fresh_state();
        let app = economy_router(state);
        let mut inputs = baseline_inputs();
        inputs.actor_class = ActorClass::Human;
        inputs.event_class = EventClass::AvcValidate;
        let payload = QuoteRequest {
            quote_id: "q-h".into(),
            inputs,
        };
        let body = serde_json::to_vec(&payload).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/economy/quote")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let parsed: SettlementQuote = serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(parsed.zero_fee_reason, Some(ZeroFeeReason::AgentValidation));
    }

    #[test]
    fn router_uses_blocking_store_access_and_local_admission_control() {
        let source = include_str!("economy.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert!(
            production.contains("tokio::task::spawn_blocking"),
            "economy handlers must isolate sync store access from Tokio workers"
        );
        assert!(
            production.contains("DefaultBodyLimit::max(MAX_ECONOMY_API_BODY_BYTES)"),
            "economy router must cap body size locally"
        );
        assert!(
            production.contains("ConcurrencyLimitLayer::new("),
            "economy router must apply local admission control"
        );
    }
}
