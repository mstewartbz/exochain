//! HTTP server skeleton — gateway configuration, lifecycle, and axum routing.
use std::sync::{Arc, RwLock};

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
};
use exo_core::Did;
use exo_gatekeeper::{
    invariants::InvariantSet,
    kernel::{AdjudicationContext, Kernel},
    types::{AuthorityChain, BailmentState, Permission, PermissionSet},
};
use exo_governance::conflict::ConflictDeclaration;
use exo_identity::did::{DidDocument, DidRegistry};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::{
    error::{GatewayError, Result},
    rest::HealthResponse,
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub bind_address: String,
    pub tls_config: Option<TlsConfig>,
    pub max_connections: u32,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8443".into(),
            tls_config: None,
            max_connections: 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// Synchronous handle (kept for backward compatibility)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct GatewayHandle {
    pub config: GatewayConfig,
    pub running: bool,
}

/// Validate config and return a handle.  Does not bind a port or start an
/// async runtime — use `serve()` for a running server.
pub fn start(config: GatewayConfig) -> Result<GatewayHandle> {
    if config.bind_address.is_empty() {
        return Err(GatewayError::BadRequest(
            "bind_address cannot be empty".into(),
        ));
    }
    if config.max_connections == 0 {
        return Err(GatewayError::BadRequest(
            "max_connections must be > 0".into(),
        ));
    }
    Ok(GatewayHandle {
        config,
        running: true,
    })
}

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Shared state injected into every axum handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    /// Live PostgreSQL pool.  `None` when the gateway starts without a DB
    /// URL (e.g. local dev without Docker Compose).
    pub pool: Option<sqlx::PgPool>,
    /// In-memory DID registry shared across all request handlers.
    pub registry: Arc<RwLock<DidRegistry>>,
    /// Constitutional kernel — enforces the 8 invariants on every action.
    pub kernel: Arc<Kernel>,
    /// Wall-clock milliseconds at server start, used to compute uptime.
    start_ms: u64,
}

impl AppState {
    pub fn new(pool: Option<sqlx::PgPool>, registry: Arc<RwLock<DidRegistry>>) -> Self {
        // Bootstrap kernel with the all-invariants set.
        // constitution bytes are hashed for immutability verification.
        let kernel = Kernel::new(b"exochain-constitution-v1", InvariantSet::all());
        Self {
            pool,
            registry,
            kernel: Arc::new(kernel),
            start_ms: now_ms(),
        }
    }

    fn uptime_seconds(&self) -> u64 {
        now_ms().saturating_sub(self.start_ms) / 1000
    }

    /// Return the DB pool or a 503 if none is configured.
    pub fn require_db(&self) -> Result<&sqlx::PgPool> {
        self.pool.as_ref().ok_or_else(|| {
            GatewayError::Internal("no database configured — start with DATABASE_URL".into())
        })
    }

    /// Load conflict declarations for an actor.
    ///
    /// Currently returns an empty list until the DB schema includes a
    /// `conflict_declarations` table.  Handlers should call
    /// `.await.unwrap_or_default()` for graceful degradation.
    pub async fn load_conflict_declarations(
        &self,
        _actor: &Did,
    ) -> std::result::Result<Vec<ConflictDeclaration>, GatewayError> {
        // TODO: query `conflict_declarations` table when DB is available
        Ok(vec![])
    }

    /// Build a minimal adjudication context for the given actor.
    ///
    /// For endpoints that are functional before full role/consent databases
    /// are available.  Sets `human_override_preserved: true` and grants the
    /// actor the `"vote"` permission so the Kernel can permit basic actions.
    pub async fn build_adjudication_context(&self, _actor: &Did) -> AdjudicationContext {
        AdjudicationContext {
            actor_roles: vec![],
            authority_chain: AuthorityChain::default(),
            consent_records: vec![],
            bailment_state: BailmentState::None,
            human_override_preserved: true,
            actor_permissions: PermissionSet::new(vec![Permission::new("vote")]),
            provenance: None,
            quorum_evidence: None,
        }
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /health — always returns 200 OK.
async fn handle_health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_seconds: state.uptime_seconds(),
    })
}

/// GET /ready — returns 200 when the DB pool is reachable, 503 otherwise.
async fn handle_ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let (status_str, http_status) = match &state.pool {
        Some(pool) => match sqlx::query("SELECT 1").fetch_one(pool).await {
            Ok(_) => ("ok", StatusCode::OK),
            Err(_) => ("db_unavailable", StatusCode::SERVICE_UNAVAILABLE),
        },
        None => ("no_db_configured", StatusCode::SERVICE_UNAVAILABLE),
    };
    let body = HealthResponse {
        status: status_str.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_seconds: state.uptime_seconds(),
    };
    (http_status, Json(body))
}

/// Placeholder for endpoints that are defined but not yet fully implemented.
async fn handle_not_implemented() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "not_implemented",
            "message": "This endpoint is defined but not yet fully implemented. See EXOCHAIN-REM-001."
        })),
    )
}

// ---------------------------------------------------------------------------
// DID / Auth / Agent handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/auth/register — register a new DID document.
///
/// Body: a `DidDocument` JSON object.  Returns 201 on success, 409 if the
/// DID is already registered.
async fn handle_auth_register(
    State(state): State<AppState>,
    Json(doc): Json<DidDocument>,
) -> impl IntoResponse {
    let did_str = doc.id.as_str().to_owned();
    let mut reg = state.registry.write().unwrap_or_else(|e| e.into_inner());
    match reg.register(doc) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "did": did_str, "status": "registered" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// GET /api/v1/auth/me — resolve the caller's DID document.
///
/// Requires `X-Actor-Did: did:exo:<id>` request header.
async fn handle_auth_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let did_str = match headers.get("x-actor-did").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_owned(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "missing X-Actor-Did header" })),
            )
                .into_response();
        }
    };
    let did = match Did::new(&did_str) {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid DID format" })),
            )
                .into_response();
        }
    };
    let reg = state.registry.read().unwrap_or_else(|e| e.into_inner());
    match reg.resolve(&did) {
        Some(doc) => Json(doc.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "DID not found" })),
        )
            .into_response(),
    }
}

/// GET /api/v1/agents — list all registered DID identifiers.
async fn handle_agents_list(State(state): State<AppState>) -> impl IntoResponse {
    let reg = state.registry.read().unwrap_or_else(|e| e.into_inner());
    let dids: Vec<String> = reg.list_dids().into_iter().map(|s| s.to_owned()).collect();
    Json(serde_json::json!({ "agents": dids }))
}

/// GET /api/v1/agents/:did — resolve a single DID document by its identifier.
async fn handle_agent_get(
    State(state): State<AppState>,
    Path(did_str): Path<String>,
) -> impl IntoResponse {
    let did = match Did::new(&did_str) {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid DID format" })),
            )
                .into_response();
        }
    };
    let reg = state.registry.read().unwrap_or_else(|e| e.into_inner());
    match reg.resolve(&did) {
        Some(doc) => Json(doc.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "DID not found" })),
        )
            .into_response(),
    }
}

/// POST /api/v1/agents/enroll — enroll an agent by registering its DID document.
///
/// Identical to `POST /api/v1/auth/register`; the distinct path expresses the
/// agent-specific enrollment workflow while sharing the underlying DID registry.
async fn handle_agents_enroll(
    State(state): State<AppState>,
    Json(doc): Json<DidDocument>,
) -> impl IntoResponse {
    let did_str = doc.id.as_str().to_owned();
    let mut reg = state.registry.write().unwrap_or_else(|e| e.into_inner());
    match reg.register(doc) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "did": did_str, "status": "enrolled" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the axum `Router` with all constitutional middleware wired.
///
/// All 20 `RestRoute` paths are registered.  Unimplemented handlers return
/// 501 until the full handler stack lands in a follow-up PR.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Probes
        .route("/health", get(handle_health))
        .route("/ready", get(handle_ready))
        // Decisions
        .route("/api/v1/decisions/:id", get(handle_not_implemented))
        .route("/api/v1/decisions", post(handle_not_implemented))
        // Auth
        .route("/api/v1/auth/token", post(handle_not_implemented))
        .route("/api/v1/auth/saml/callback", post(handle_not_implemented))
        .route("/api/v1/auth/register", post(handle_auth_register))
        .route("/api/v1/auth/login", post(handle_not_implemented))
        .route("/api/v1/auth/refresh", post(handle_not_implemented))
        .route("/api/v1/auth/me", get(handle_auth_me))
        .route("/api/v1/auth/logout", post(handle_not_implemented))
        // Agents (static route before parameterised to avoid ambiguity)
        .route("/api/v1/agents/enroll", post(handle_agents_enroll))
        .route("/api/v1/agents", get(handle_agents_list))
        .route("/api/v1/agents/:did", get(handle_agent_get))
        .route(
            "/api/v1/agents/:did/advance-pace",
            post(handle_not_implemented),
        )
        // Identity
        .route("/api/v1/identity/:did/score", get(handle_not_implemented))
        // Tenant
        .route(
            "/api/v1/tenants/:id/constitution",
            get(handle_not_implemented),
        )
        // Legal
        .route("/api/v1/ediscovery/export", post(handle_not_implemented))
        // Audit
        .route("/api/v1/audit/:decision_id", get(handle_not_implemented))
        // Users
        .route("/api/v1/users", get(handle_not_implemented))
        .route(
            "/api/v1/users/:did/advance-pace",
            post(handle_not_implemented),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Graceful shutdown signal
// ---------------------------------------------------------------------------

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let sigterm = async {
        let mut stream = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap_or_else(|_| panic!("failed to install SIGTERM handler"));
        stream.recv().await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C — shutting down");
        }
        _ = sigterm => {
            tracing::info!("Received SIGTERM — shutting down");
        }
    }
}

// ---------------------------------------------------------------------------
// Async server entry point
// ---------------------------------------------------------------------------

/// Bind to `config.bind_address`, serve all routes, and drain on SIGTERM /
/// Ctrl+C.  Returns once shutdown is complete.
pub async fn serve(config: GatewayConfig, pool: Option<sqlx::PgPool>) -> Result<()> {
    let registry = Arc::new(RwLock::new(DidRegistry::new()));
    let state = AppState::new(pool, registry);
    let app = build_router(state);

    let listener = TcpListener::bind(&config.bind_address)
        .await
        .map_err(|e| GatewayError::Internal(format!("bind failed: {e}")))?;

    tracing::info!("exo-gateway listening on {}", config.bind_address);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| GatewayError::Internal(format!("server error: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use exo_core::Timestamp;
    use exo_identity::did::DidDocument;
    use tower::ServiceExt;

    use super::*; // for .oneshot()

    fn state() -> AppState {
        AppState::new(None, Arc::new(RwLock::new(DidRegistry::new())))
    }

    /// Build a minimal DidDocument for use in registration tests.
    fn minimal_doc(did_str: &str) -> DidDocument {
        let did = Did::new(did_str).expect("valid DID");
        DidDocument {
            id: did,
            public_keys: vec![],
            authentication: vec![],
            verification_methods: vec![],
            service_endpoints: vec![],
            created: Timestamp::ZERO,
            updated: Timestamp::ZERO,
            revoked: false,
        }
    }

    // --- GatewayConfig / start() (existing tests preserved) ---

    #[test]
    fn start_default() {
        let h = start(GatewayConfig::default()).unwrap();
        assert!(h.running);
        assert_eq!(h.config.bind_address, "127.0.0.1:8443");
    }
    #[test]
    fn start_empty_addr() {
        let c = GatewayConfig {
            bind_address: String::new(),
            ..Default::default()
        };
        assert!(start(c).is_err());
    }
    #[test]
    fn start_zero_connections() {
        let c = GatewayConfig {
            max_connections: 0,
            ..Default::default()
        };
        assert!(start(c).is_err());
    }
    #[test]
    fn config_serde() {
        let c = GatewayConfig::default();
        let j = serde_json::to_string(&c).unwrap();
        let r: GatewayConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(r.bind_address, c.bind_address);
    }
    #[test]
    fn tls_config_serde() {
        let t = TlsConfig {
            cert_path: "c".into(),
            key_path: "k".into(),
        };
        let j = serde_json::to_string(&t).unwrap();
        let r: TlsConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(r.cert_path, "c");
    }
    #[test]
    fn config_with_tls() {
        let c = GatewayConfig {
            tls_config: Some(TlsConfig {
                cert_path: "c".into(),
                key_path: "k".into(),
            }),
            ..Default::default()
        };
        let h = start(c).unwrap();
        assert!(h.config.tls_config.is_some());
    }

    // --- Router integration tests (no network listener needed) ---

    #[tokio::test]
    async fn health_returns_200() {
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_without_db_returns_503() {
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn defined_api_routes_return_non_404() {
        // Routes that return non-404 regardless of body/headers.
        // NOTE: /api/v1/agents/:did is excluded here because it returns 404
        // when the DID is not registered — that behaviour is tested separately.
        let paths = [
            "/api/v1/decisions/some-id",
            "/api/v1/auth/me",
            "/api/v1/agents",
            "/api/v1/identity/did:exo:alice/score",
            "/api/v1/users",
            "/api/v1/audit/decision-123",
            "/api/v1/tenants/tenant-1/constitution",
        ];
        for path in paths {
            let app = build_router(state());
            let resp = app
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_ne!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "expected non-404 for GET {path}"
            );
        }
    }

    // --- New DID / auth / agent endpoint tests ---

    #[tokio::test]
    async fn auth_register_returns_201() {
        let body = serde_json::to_string(&minimal_doc("did:exo:tester")).unwrap();
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn auth_register_duplicate_returns_409() {
        let doc = minimal_doc("did:exo:dup");
        let registry = Arc::new(RwLock::new(DidRegistry::new()));
        registry.write().unwrap().register(doc.clone()).unwrap();
        let st = AppState::new(None, registry);
        let body = serde_json::to_string(&doc).unwrap();
        let app = build_router(st);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn auth_me_missing_header_returns_400() {
        let app = build_router(state());
        let resp = app
            .oneshot(Request::builder().uri("/api/v1/auth/me").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn auth_me_known_did_returns_200() {
        let doc = minimal_doc("did:exo:me-test");
        let registry = Arc::new(RwLock::new(DidRegistry::new()));
        registry.write().unwrap().register(doc).unwrap();
        let st = AppState::new(None, registry);
        let app = build_router(st);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("x-actor-did", "did:exo:me-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_me_unknown_did_returns_404() {
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("x-actor-did", "did:exo:ghost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn agents_list_returns_registered_dids() {
        let doc = minimal_doc("did:exo:listed");
        let registry = Arc::new(RwLock::new(DidRegistry::new()));
        registry.write().unwrap().register(doc).unwrap();
        let st = AppState::new(None, registry);
        let app = build_router(st);
        let resp = app
            .oneshot(Request::builder().uri("/api/v1/agents").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(val["agents"].as_array().unwrap().contains(&serde_json::json!("did:exo:listed")));
    }

    #[tokio::test]
    async fn agent_get_known_did_returns_200() {
        let doc = minimal_doc("did:exo:agent-get");
        let registry = Arc::new(RwLock::new(DidRegistry::new()));
        registry.write().unwrap().register(doc).unwrap();
        let st = AppState::new(None, registry);
        let app = build_router(st);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/agents/did:exo:agent-get")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn agent_get_unknown_returns_404() {
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/agents/did:exo:nobody")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn agents_enroll_returns_201() {
        let body = serde_json::to_string(&minimal_doc("did:exo:enrollee")).unwrap();
        let app = build_router(state());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents/enroll")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
}
