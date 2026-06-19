//! Additive DAG DB gateway scaffolding and narrow council ingress.

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{MethodRouter, get, post},
};
// `ConsentPurpose` is consumed only by the production-db consent verification and
// the `#[cfg(debug_assertions)]` dev gatekeeper profile; gate it to those
// configurations to avoid an unused-import warning in a release build without
// `production-db`.
#[cfg(any(feature = "production-db", debug_assertions))]
use exo_api::dagdb::ConsentPurpose;
// `DagDbWritebackResponse` is constructed only by the production-db persisted
// writeback path and the `#[cfg(test)]` response-shape builder; gate the import
// to those configurations so the default build is unused-import clean.
#[cfg(any(test, feature = "production-db"))]
use exo_api::dagdb::DagDbWritebackResponse;
use exo_api::dagdb::{
    CatalogEntryResponse, ContextPacketLayerBudgetReport, ContextPacketLayerEdgeRef,
    ContextPacketLayerRef, ContextPacketMemoryRef, CouncilReviewStatus, CredentialStatus,
    DagDbCatalogLookupRequest, DagDbCatalogLookupResponse, DagDbContextPacketRequest,
    DagDbContextPacketResponse, DagDbCouncilDecisionRequest, DagDbErrorEnvelope,
    DagDbExportRequest, DagDbImportRequest, DagDbIntakeRequest, DagDbIntakeResponse,
    DagDbReceiptLookupRequest, DagDbReceiptLookupResponse, DagDbRouteLookupRequest,
    DagDbRouteLookupResponse, DagDbRouteRequest, DagDbRouteResponse, DagDbTrustCheckRequest,
    DagDbTrustCheckResponse, DagDbValidateRequest, DagDbValidateResponse, DagDbWritebackRequest,
    DagFinalityStatus, MemoryStatus, ReceiptEventType, RiskClass, RouteStatus, SafeMetadata,
    SubjectKind, ValidationDecision, ValidationStatus,
};
#[cfg(feature = "production-db")]
use exo_api::dagdb::{DagDbExportResponse, DagDbImportResponse};
#[cfg(feature = "production-db")]
use exo_api::dagdb::{
    DagDbGraphContextPacketBuildRequest, DagDbGraphContextSelectionRequest,
    DagDbGraphContextSelectionResponse, DagDbGraphContextSelectionStatus,
};
use exo_core::Hash256;
use exo_dag_db_core::{
    hash::RequestHashMaterial,
    metadata::{MetadataField, sanitize_keywords, sanitize_runtime_metadata},
};
use exo_dag_db_domain::council::{CouncilError, build_council_decision_response};
#[cfg(feature = "production-db")]
use exo_dag_db_domain::scoring::DomainError;
#[cfg(feature = "production-db")]
use exo_dag_db_domain::{
    context_packet_persistence::{
        ContextPacketRecord, ContextPacketRequest, ContextPacketRouteBinding,
        DefaultContextQuality, PacketFreshnessStatus, PacketPersistenceStatus,
        PacketValidationStatus, build_context_packet_record,
    },
    continuation_persistence::{
        ContinuationRecord, ContinuationRetrievalStatus, PRD17_CONTINUATION_RECORD_SCHEMA,
    },
    default_route::{
        DEFAULT_ROUTE_SCHEMA_VERSION, DefaultRouteMemoryRef, DefaultRouteRecord,
        DefaultRouteSource, DefaultRouteStatus, RouteFreshnessStatus,
    },
    lifecycle_action::{
        LifecycleAction, LifecycleActionType, LifecycleEvidenceRef, LifecycleMemoryRef,
        LifecycleRollbackRef, LifecycleTerminalState, PRD17_LIFECYCLE_ACTION_SCHEMA,
        ProductionLifecycleApproval,
    },
};
#[cfg(feature = "production-db")]
use exo_dag_db_exchange::{kg_export::KgExportError, kg_import::KgImportError};
use exo_dag_db_exchange::{kg_export::KgExportScope, kg_import::KgImportDryRunReport};
#[cfg(feature = "production-db")]
use exo_dag_db_postgres::{
    persistent_context::{
        build_persistent_graph_context_packet,
        build_persistent_graph_context_packet_with_layered_drilldown,
        build_persistent_graph_context_selection,
    },
    postgres::{
        begin_tenant_transaction, kg_context_selection_write::UsageEventMemoryMetadata,
        kg_import::KgImportPersistenceError,
    },
};
// `DagDbConsentRecord`/`BailmentState` are consumed only by the production-db DB
// resolver and the `#[cfg(debug_assertions)]` dev gatekeeper profile; gate the
// import to those configurations so a release build without `production-db` does
// not warn on unused imports (T1 gated the dev profile out of release).
#[cfg(feature = "production-db")]
use exo_gatekeeper::invariants::InvariantContext;
use exo_gatekeeper::{ConsentEngine, IdentityRegistry};
#[cfg(any(feature = "production-db", debug_assertions))]
use exo_gatekeeper::{DagDbConsentRecord, types::BailmentState};
#[cfg(feature = "production-db")]
use exo_gatekeeper::{DagDbGatekeeperService, GatekeeperError, types::DAGDB_WRITEBACK_SCOPE};
#[cfg(feature = "production-db")]
use exo_gatekeeper::{usage_event_payload_hash, verify_write_consent, verify_write_signature};
use serde::Serialize;
use serde_json::{Value, json};
#[cfg(feature = "production-db")]
use sqlx::{Postgres, Row, Transaction};
#[cfg(any(feature = "production-db", debug_assertions))]
use tracing::info;
use tracing::warn;

/// Public REST prefix reserved for ExoChain DAG DB.
pub const DAGDB_REST_PREFIX: &str = "/api/v1/dag-db";

const TENANT_HEADER: &str = "x-exo-tenant-id";
const NAMESPACE_HEADER: &str = "x-exo-namespace";
const AUTHORITY_SCOPE_HEADER: &str = "x-exo-authority-scope";
#[cfg(feature = "production-db")]
const WRITE_SIGNATURE_HEADER: &str = "x-exo-write-signature";
#[cfg(feature = "production-db")]
const LIFECYCLE_SIGNATURE_HEADER: &str = "x-exo-lifecycle-signature";
#[cfg(feature = "production-db")]
const CONTINUATION_SIGNATURE_HEADER: &str = "x-exo-continuation-signature";
#[cfg(feature = "production-db")]
const WRITEBACK_CONTINUATION_EXPIRY_EPOCH_SECONDS: u64 = 4_102_444_800;
#[cfg(feature = "production-db")]
const IMPORT_ROUTE_IDEMPOTENCY_NAME: &str = "dagdb.import";
#[cfg(feature = "production-db")]
const EXPORT_ROUTE_IDEMPOTENCY_NAME: &str = "dagdb.export";
#[cfg(feature = "production-db")]
const RESERVED_IDEMPOTENCY_BODY_STATUS: &str = "reserved";
#[cfg(feature = "production-db")]
const GATEWAY_IDEMPOTENCY_RESERVATION_TTL_MS: i64 = 86_400_000;
#[cfg(feature = "production-db")]
const GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD: &str = "_gateway_authorization_payload_hash";
/// Environment variable that enables the repository local-dev gatekeeper profile.
///
/// Only consulted by the `#[cfg(debug_assertions)]` dev-profile mount; the
/// constant itself is retained unconditionally so the binary and tooling can
/// name it, but a release build never reads it because the mount is compiled out.
pub const LOCAL_DEV_GATEKEEPER_ENV: &str = "DAGDB_LOCAL_DEV_GATEKEEPER";
#[cfg(debug_assertions)]
const LOCAL_DEV_AGENT_DID: &str = "did:exo:cursor-mcp-agent";
// Canonical local-dev tenant/namespace sourced from the single DAG DB constant
// so the gateway can never diverge from the runtime's tenant identity (GAP-012
// P1-E).
#[cfg(debug_assertions)]
const LOCAL_DEV_TENANT_ID: &str = exo_dag_db_core::tenant::LOCAL_DEV_TENANT_ID;
#[cfg(debug_assertions)]
const LOCAL_DEV_NAMESPACE: &str = exo_dag_db_core::tenant::LOCAL_DEV_NAMESPACE;
#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
const DAGDB_LAYERED_MODES: &[&str] = &["off", "auto", "required"];
#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
const DAGDB_MAX_LAYER_DEPTH: u32 = 8;
// D1-S4 gateway bound for the depth-on-demand reserve, in basis points. Mirrors
// the runtime's `LAYERED_DRILLDOWN_MAX_RESERVE_BP` (the breadth pass always keeps
// a majority share). Defined locally so the validator compiles without the
// production-db feature, which gates the exo-dag-db import.
// Consumed only by `validate_drilldown_reserve_bp`, itself reached solely from
// the production-db persisted context-packet path, so gate it identically.
#[cfg(feature = "production-db")]
const DAGDB_MAX_DRILLDOWN_RESERVE_BP: u32 = 5_000;
/// Allowed typed-knowledge classes for a writeback. The list is closed: any
/// other value is rejected (fail closed). The class describes WHAT the memory
/// is; it never influences deterministic placement/organization.
const DAGDB_KNOWLEDGE_CLASSES: &[&str] = &["decision", "finding", "fix", "constraint", "handoff"];
#[cfg(feature = "production-db")]
const DAGDB_WRITEBACK_SIGNED_TASK_HASH_DOMAIN: &str =
    "exo.dagdb.gateway.writeback.signed_task_hash.v2";
#[cfg(debug_assertions)]
const LOCAL_DEV_KEY_SEED_REL: &str = "crates/exo-gatekeeper/tests/fixtures/dev_private_key.seed";
#[cfg(debug_assertions)]
const LOCAL_DEV_KEY_SOURCE_EXPLICIT_SEED: &str = "explicit_seed_file";
#[cfg(debug_assertions)]
const LOCAL_DEV_KEY_SOURCE_DETERMINISTIC_FALLBACK: &str = "deterministic_local_dev_fallback";

type QueryParams = BTreeMap<String, String>;

/// Route-scoped DAG DB dependencies injected by the gateway server merge.
#[derive(Clone)]
pub struct DagDbRouteContext {
    pub pool: Option<sqlx::PgPool>,
    /// Explicitly-installed in-memory gatekeeper profile.
    ///
    /// `None` is the default production posture: the write gate hydrates its
    /// consent/identity state from the live database per request (see
    /// [`resolve_gatekeeper_service_from_db`]). A `Some(_)` profile is installed
    /// ONLY by integration tests (`install_gatekeeper_profile`) or the explicit
    /// repository dev profile (`install_local_dev_gatekeeper_profile`, gated on
    /// `DAGDB_LOCAL_DEV_GATEKEEPER`); when present it overrides the DB resolver.
    /// Ticket T1 will gate/remove the dev fallback.
    gatekeeper: Arc<RwLock<Option<DagDbGatekeeperProfile>>>,
}

#[derive(Clone)]
struct DagDbGatekeeperProfile {
    // Constructed in every build (the dev/test profile installer compiles in the
    // default build) but read only on the production-db gatekeeper_service path;
    // suppress the not-read warning without gating the field, which would split
    // its constructor across configs.
    #[cfg_attr(not(feature = "production-db"), allow(dead_code))]
    consent_engine: Arc<ConsentEngine>,
    #[cfg_attr(not(feature = "production-db"), allow(dead_code))]
    identity_registry: Arc<IdentityRegistry>,
}

impl DagDbRouteContext {
    /// Build route context from the gateway application pool.
    #[must_use]
    pub fn from_pool(pool: Option<sqlx::PgPool>) -> Self {
        Self {
            pool,
            gatekeeper: Arc::new(RwLock::new(None)),
        }
    }

    /// Install in-memory consent and identity registries for integration tests.
    pub fn install_gatekeeper_profile(
        &self,
        consent_engine: ConsentEngine,
        identity_registry: IdentityRegistry,
    ) {
        if let Ok(mut profile) = self.gatekeeper.write() {
            *profile = Some(DagDbGatekeeperProfile {
                consent_engine: Arc::new(consent_engine),
                identity_registry: Arc::new(identity_registry),
            });
        }
    }

    /// Install local-dev consent and identity for repository MCP writeback dogfood.
    ///
    /// Gated on `#[cfg(debug_assertions)]` so it is never compiled into a release
    /// binary (T1). In release the write gate reaches `gatekeeper_service` with
    /// no installed profile and authorizes only via the live DB resolver
    /// (`resolve_gatekeeper_service_from_db`) or fails closed; there is no
    /// fabricated dev identity in the shipping path.
    #[cfg(debug_assertions)]
    pub fn install_local_dev_gatekeeper_profile(&self) {
        self.install_local_dev_gatekeeper_profile_from_seed_path(&local_dev_key_seed_path());
    }

    #[cfg(debug_assertions)]
    fn install_local_dev_gatekeeper_profile_from_seed_path(&self, seed_path: &str) {
        let Ok(local_keypair) = load_local_dev_keypair_from_seed_path(seed_path) else {
            return;
        };
        let keypair = local_keypair.keypair;
        let Ok(bailor) = exo_core::Did::new("did:exo:local-dev-bailor")
            .or_else(|_| exo_core::Did::new("did:exo:bailor"))
        else {
            return;
        };
        let bailee = exo_core::Did::new(LOCAL_DEV_AGENT_DID).unwrap_or_else(|_| bailor.clone());
        let consent_engine = ConsentEngine::default()
            .with_bailment(
                LOCAL_DEV_TENANT_ID,
                BailmentState::Active {
                    bailor,
                    bailee,
                    scope: "dag-db:writeback".into(),
                },
            )
            .with_consent_record(DagDbConsentRecord {
                tenant_id: LOCAL_DEV_TENANT_ID.to_owned(),
                agent_did: LOCAL_DEV_AGENT_DID.to_owned(),
                purpose: ConsentPurpose::Writeback,
                active: true,
            });
        let identity_registry = IdentityRegistry::default()
            .with_public_key(LOCAL_DEV_AGENT_DID, *keypair.public_key().as_bytes());
        self.install_gatekeeper_profile(consent_engine, identity_registry);
        info!(
            tenant_id = LOCAL_DEV_TENANT_ID,
            namespace = LOCAL_DEV_NAMESPACE,
            agent_did = LOCAL_DEV_AGENT_DID,
            key_source = local_keypair.source,
            "Installed local dev DAG DB gatekeeper profile"
        );
    }

    /// Snapshot the explicitly-installed gatekeeper profile, if any.
    ///
    /// Returns `None` both when no profile was installed AND when the lock is
    /// poisoned. A poisoned lock is treated as "no installed profile" so the
    /// caller falls through to the DB resolver (fail-closed real authorization)
    /// rather than to a silently-empty default that would deny-all and mask the
    /// fault. This replaces the prior `.unwrap_or_default()` poisoned-lock
    /// silent-empty fallback.
    #[cfg(feature = "production-db")]
    fn installed_gatekeeper_profile(&self) -> Option<DagDbGatekeeperProfile> {
        self.gatekeeper.read().ok().and_then(|guard| guard.clone())
    }

    /// Resolve the gatekeeper service that authorizes a single write.
    ///
    /// When an in-memory profile is explicitly installed (integration tests or
    /// the gated repository dev profile) it is used directly. Otherwise — the
    /// default production posture — the service is hydrated from live database
    /// state for `(agent_did, tenant_id)` via [`resolve_gatekeeper_service_from_db`].
    ///
    /// Fails closed with [`GatekeeperError::AuthorityResolverUnavailable`] when
    /// the DB resolver cannot establish authorization state (query failure). It
    /// NEVER falls back to empty registries.
    #[cfg(feature = "production-db")]
    async fn gatekeeper_service(
        &self,
        pool: &sqlx::PgPool,
        agent_did: &str,
        tenant_id: &str,
    ) -> Result<DagDbGatekeeperService, GatekeeperError> {
        if let Some(profile) = self.installed_gatekeeper_profile() {
            return Ok(DagDbGatekeeperService::new(
                pool.clone(),
                profile.consent_engine,
                profile.identity_registry,
            ));
        }
        resolve_gatekeeper_service_from_db(pool, agent_did, tenant_id).await
    }
}

/// Hydrate a [`DagDbGatekeeperService`] from live ExoChain consent/identity
/// database state for a single `(agent_did, tenant_id)` write authorization.
///
/// This is the default production authority resolver: it translates the
/// gateway-owned, DB-backed `consent_records` rows and the agent's DID document
/// into the gate's in-memory [`ConsentEngine`]/[`IdentityRegistry`] shapes. The
/// row→gate-type translation lives here in the gateway crate so `exo-gatekeeper`
/// stays database-free.
///
/// ## Tenant-qualified scope convention
///
/// `consent_records` has no tenant column, so the tenant is encoded into the
/// existing `scope` string. A writeback grant for `tenant_id` is stored as
/// [`tenant_writeback_scope`]'s `dag-db:writeback:{tenant_id}`. The resolver
/// filters the loaded rows on that exact scope string, so a grant scoped to a
/// different tenant cannot authorize this tenant's writeback. The gate's own
/// bailment uses the canonical [`DAGDB_WRITEBACK_SCOPE`] (`dag-db:writeback`),
/// which is what `BailmentState::authorizes_writeback` checks; the
/// tenant-qualification is enforced by the row filter, not the bailment scope.
///
/// ## Fail-closed contract
///
/// Returns [`GatekeeperError::AuthorityResolverUnavailable`] when a database
/// query fails (consent load, trusted-clock lookup, or DID-document lookup) or
/// when a registered DID document carries no trustable Ed25519 key. It never
/// returns a service backed by empty registries on error — an empty registry
/// silently denies-all and masks misconfiguration, so it is reserved for the
/// genuinely-unregistered agent (no rows), which then fails closed inside the
/// gate's consent/provenance checks with a policy denial.
#[cfg(feature = "production-db")]
async fn resolve_gatekeeper_service_from_db(
    pool: &sqlx::PgPool,
    agent_did: &str,
    tenant_id: &str,
) -> Result<DagDbGatekeeperService, GatekeeperError> {
    let now_ms = trusted_resolver_epoch_ms(pool).await?;
    let expected_scope = tenant_writeback_scope(tenant_id);

    let mut consent_engine = ConsentEngine::default();
    let consent_rows = crate::db::load_consent_records(pool, agent_did, now_ms)
        .await
        .map_err(|error| {
            GatekeeperError::AuthorityResolverUnavailable(format!(
                "consent records lookup failed: {error}"
            ))
        })?;
    for row in consent_rows {
        // The loader already filters to active + non-expired rows. Translate
        // only the rows scoped to THIS tenant's writeback into gate state.
        if row.scope != expected_scope {
            continue;
        }
        // The bailor (subject) entrusts the acting agent (bailee) with the
        // canonical writeback scope. `authorizes_writeback` checks the bailee
        // equals the agent and the scope equals `DAGDB_WRITEBACK_SCOPE`; the
        // tenant qualification has already been enforced by the row filter.
        let Ok(bailor) = exo_core::Did::new(&row.subject_did) else {
            continue;
        };
        let Ok(bailee) = exo_core::Did::new(agent_did) else {
            continue;
        };
        consent_engine = consent_engine
            .with_bailment(
                tenant_id,
                BailmentState::Active {
                    bailor,
                    bailee,
                    scope: DAGDB_WRITEBACK_SCOPE.to_owned(),
                },
            )
            .with_consent_record(DagDbConsentRecord {
                tenant_id: tenant_id.to_owned(),
                agent_did: agent_did.to_owned(),
                purpose: ConsentPurpose::Writeback,
                active: true,
            });
    }

    let mut identity_registry = IdentityRegistry::default();
    if let Some(doc) = crate::db::find_did_document(pool, agent_did)
        .await
        .map_err(|error| {
            GatekeeperError::AuthorityResolverUnavailable(format!(
                "DID document lookup failed: {error}"
            ))
        })?
    {
        let keys = crate::server::active_did_document_ed25519_keys(&doc, "dag-db writeback agent")
            .map_err(|error| {
                GatekeeperError::AuthorityResolverUnavailable(format!(
                    "DID document key validation failed: {error}"
                ))
            })?;
        for key in keys {
            let Ok(key_bytes) = <[u8; 32]>::try_from(key.as_slice()) else {
                continue;
            };
            identity_registry = identity_registry.with_public_key(agent_did, key_bytes);
        }
    }

    Ok(DagDbGatekeeperService::new(
        pool.clone(),
        Arc::new(consent_engine),
        Arc::new(identity_registry),
    ))
}

/// Canonical tenant-qualified DAG DB writeback scope string used as the
/// `consent_records.scope` filter value.
///
/// `consent_records` has no tenant column; the tenant is encoded into the scope
/// so a grant cannot be replayed across tenants. Built from the gate's canonical
/// [`DAGDB_WRITEBACK_SCOPE`] (`dag-db:writeback`) as `dag-db:writeback:{tenant_id}`.
#[cfg(feature = "production-db")]
fn tenant_writeback_scope(tenant_id: &str) -> String {
    format!("{DAGDB_WRITEBACK_SCOPE}:{tenant_id}")
}

/// Trusted current-time milliseconds from the database clock for the resolver's
/// active/non-expired consent filter. Never caller time; fails closed on error.
#[cfg(feature = "production-db")]
async fn trusted_resolver_epoch_ms(pool: &sqlx::PgPool) -> Result<i64, GatekeeperError> {
    sqlx::query_scalar::<_, i64>(
        "SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        GatekeeperError::AuthorityResolverUnavailable(format!(
            "resolver clock lookup failed: {error}"
        ))
    })
}

static ROUTE_CONTEXT_OVERRIDE: std::sync::OnceLock<Arc<DagDbRouteContext>> =
    std::sync::OnceLock::new();

/// Override the route context used by DAG DB handlers (integration tests only).
pub fn set_route_context_for_integration_tests(ctx: Arc<DagDbRouteContext>) {
    let _ = ROUTE_CONTEXT_OVERRIDE.set(ctx);
}

fn resolve_route_context(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
) -> Arc<DagDbRouteContext> {
    if let Some(ctx) = ROUTE_CONTEXT_OVERRIDE.get() {
        return ctx.clone();
    }
    extension
        .map(|Extension(ctx)| ctx)
        .unwrap_or_else(|| Arc::new(DagDbRouteContext::from_pool(None)))
}

/// Build the additive public ExoChain DAG DB router.
pub fn dagdb_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let routes: [(&str, MethodRouter<S>); 12] = [
        ("/api/v1/dag-db/intake", post(handle_dagdb_intake)),
        ("/api/v1/dag-db/route", post(handle_dagdb_route)),
        (
            "/api/v1/dag-db/context-packet",
            post(handle_dagdb_context_packet),
        ),
        ("/api/v1/dag-db/validate", post(handle_dagdb_validate)),
        ("/api/v1/dag-db/writeback", post(handle_dagdb_writeback)),
        ("/api/v1/dag-db/import", post(handle_dagdb_import)),
        ("/api/v1/dag-db/export", post(handle_dagdb_export)),
        ("/api/v1/dag-db/trust-check", post(handle_dagdb_trust_check)),
        (
            "/api/v1/dag-db/council/decision",
            post(handle_dagdb_council_decision),
        ),
        (
            "/api/v1/dag-db/receipts/:receipt_hash",
            get(handle_dagdb_receipt_lookup),
        ),
        (
            "/api/v1/dag-db/catalog/:catalog_id",
            get(handle_dagdb_catalog_lookup),
        ),
        (
            "/api/v1/dag-db/routes/:route_id",
            get(handle_dagdb_route_lookup),
        ),
    ];
    routes
        .into_iter()
        .fold(Router::new(), |router, (path, route)| {
            router.route(path, route)
        })
}

async fn handle_dagdb_intake(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbIntakeRequest>,
) -> Response {
    let ctx = resolve_route_context(extension);
    dagdb_authorized_response(
        &ctx,
        &headers,
        request.tenant_id.clone(),
        request.namespace.clone(),
        "dagdb:intake",
        "dagdb.intake",
        || created_json_response(intake_response_from_request(request, "dagdb.intake")),
    )
    .await
}

async fn handle_dagdb_route(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbRouteRequest>,
) -> Response {
    if let Some(denied) = verify_dagdb_authority(
        &headers,
        &request.tenant_id,
        &request.namespace,
        "dagdb:route",
    ) {
        return denied;
    }
    let ctx = resolve_route_context(extension);
    #[cfg(feature = "production-db")]
    if let Err(denied) =
        verify_dagdb_session_authority(&ctx, &headers, "dagdb.route", &request.tenant_id).await
    {
        return denied;
    }
    route_handler(&ctx, &headers, request).await
}

async fn handle_dagdb_context_packet(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbContextPacketRequest>,
) -> Response {
    if let Some(denied) = verify_dagdb_authority(
        &headers,
        &request.tenant_id,
        &request.namespace,
        "dagdb:context_packet",
    ) {
        return denied;
    }
    let ctx = resolve_route_context(extension);
    #[cfg(feature = "production-db")]
    if let Err(denied) =
        verify_dagdb_session_authority(&ctx, &headers, "dagdb.context_packet", &request.tenant_id)
            .await
    {
        return denied;
    }
    gated_context_packet_handler(&ctx, &headers, request).await
}

async fn handle_dagdb_validate(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbValidateRequest>,
) -> Response {
    let ctx = resolve_route_context(extension);
    dagdb_authorized_response(
        &ctx,
        &headers,
        request.tenant_id.clone(),
        request.namespace.clone(),
        "dagdb:validate",
        "dagdb.validate",
        || created_json_response(validate_response_from_request(request, "dagdb.validate")),
    )
    .await
}

async fn handle_dagdb_writeback(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbWritebackRequest>,
) -> Response {
    if let Some(denied) = verify_dagdb_authority(
        &headers,
        &request.tenant_id,
        &request.namespace,
        "dagdb:writeback",
    ) {
        return denied;
    }
    let ctx = resolve_route_context(extension);
    #[cfg(feature = "production-db")]
    if let Err(denied) =
        verify_dagdb_session_authority(&ctx, &headers, "dagdb.writeback", &request.tenant_id).await
    {
        return denied;
    }
    writeback_handler(&ctx, &headers, request).await
}

async fn handle_dagdb_import(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    request: Result<Json<DagDbImportRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            return dagdb_invalid_json_request_response("dagdb.import", &rejection);
        }
    };
    if let Some(denied) = verify_dagdb_authority(
        &headers,
        &request.tenant_id,
        &request.namespace,
        "dagdb:import",
    ) {
        log_dagdb_authority_denial(
            "dagdb.import",
            &headers,
            &request.tenant_id,
            &request.namespace,
            "dagdb:import",
        );
        return denied;
    }
    let ctx = resolve_route_context(extension);
    #[cfg(feature = "production-db")]
    {
        let session_actor = match verify_dagdb_session_authority(
            &ctx,
            &headers,
            "dagdb.import",
            &request.tenant_id,
        )
        .await
        {
            Ok(actor) => actor,
            Err(denied) => return denied,
        };
        if let Err(denied) = bind_requester_to_session_actor(
            &session_actor,
            "dagdb.import",
            &request.tenant_id,
            &request.requester_did,
        ) {
            return *denied;
        }
    }
    import_handler(&ctx, &headers, request).await
}

async fn handle_dagdb_export(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    request: Result<Json<DagDbExportRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            return dagdb_invalid_json_request_response("dagdb.export", &rejection);
        }
    };
    if let Some(denied) = verify_dagdb_authority(
        &headers,
        &request.tenant_id,
        &request.namespace,
        "dagdb:export",
    ) {
        log_dagdb_authority_denial(
            "dagdb.export",
            &headers,
            &request.tenant_id,
            &request.namespace,
            "dagdb:export",
        );
        return denied;
    }
    let ctx = resolve_route_context(extension);
    #[cfg(feature = "production-db")]
    {
        let session_actor = match verify_dagdb_session_authority(
            &ctx,
            &headers,
            "dagdb.export",
            &request.tenant_id,
        )
        .await
        {
            Ok(actor) => actor,
            Err(denied) => return denied,
        };
        if let Err(denied) = bind_requester_to_session_actor(
            &session_actor,
            "dagdb.export",
            &request.tenant_id,
            &request.requester_did,
        ) {
            return *denied;
        }
    }
    export_handler(&ctx, &headers, request).await
}

async fn handle_dagdb_trust_check(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbTrustCheckRequest>,
) -> Response {
    let ctx = resolve_route_context(extension);
    dagdb_authorized_response(
        &ctx,
        &headers,
        request.tenant_id.clone(),
        request.namespace.clone(),
        "dagdb:trust_check",
        "dagdb.trust_check",
        || {
            created_json_response(trust_check_response_from_request(
                request,
                "dagdb.trust_check",
            ))
        },
    )
    .await
}

async fn handle_dagdb_council_decision(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Json(request): Json<DagDbCouncilDecisionRequest>,
) -> Response {
    let ctx = resolve_route_context(extension);
    council_authorized_response(&ctx, &headers, request, |request| {
        match build_council_decision_response(request) {
            Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
            Err(error) => council_error_response(error),
        }
    })
    .await
}

async fn handle_dagdb_receipt_lookup(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Path(receipt_hash): Path<String>,
    Query(query): Query<QueryParams>,
) -> Response {
    let ctx = resolve_route_context(extension);
    let tenant_id = required_query_text(&query, "tenant_id");
    let namespace = required_query_text(&query, "namespace");
    dagdb_authorized_response(
        &ctx,
        &headers,
        tenant_id.clone(),
        namespace.clone(),
        "dagdb:receipt_lookup",
        "dagdb.receipt_lookup",
        || {
            let request = DagDbReceiptLookupRequest {
                receipt_hash,
                tenant_id,
                namespace,
                include_body: optional_query_bool(&query, "include_body"),
            };
            (StatusCode::OK, Json(receipt_lookup_response(request))).into_response()
        },
    )
    .await
}

async fn handle_dagdb_catalog_lookup(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
    Query(query): Query<QueryParams>,
) -> Response {
    let ctx = resolve_route_context(extension);
    let tenant_id = required_query_text(&query, "tenant_id");
    let namespace = required_query_text(&query, "namespace");
    dagdb_authorized_response(
        &ctx,
        &headers,
        tenant_id.clone(),
        namespace.clone(),
        "dagdb:catalog_lookup",
        "dagdb.catalog_lookup",
        || {
            let request = DagDbCatalogLookupRequest {
                catalog_id,
                tenant_id,
                namespace,
                include_children: optional_query_bool(&query, "include_children"),
                include_routes: optional_query_bool(&query, "include_routes"),
            };
            (StatusCode::OK, Json(catalog_lookup_response(request))).into_response()
        },
    )
    .await
}

async fn handle_dagdb_route_lookup(
    extension: Option<Extension<Arc<DagDbRouteContext>>>,
    headers: HeaderMap,
    Path(route_id): Path<String>,
    Query(query): Query<QueryParams>,
) -> Response {
    let ctx = resolve_route_context(extension);
    let tenant_id = required_query_text(&query, "tenant_id");
    let namespace = required_query_text(&query, "namespace");
    dagdb_authorized_response(
        &ctx,
        &headers,
        tenant_id.clone(),
        namespace.clone(),
        "dagdb:route_lookup",
        "dagdb.route_lookup",
        || {
            let request = DagDbRouteLookupRequest {
                route_id,
                tenant_id,
                namespace,
                include_memory_refs: optional_query_bool(&query, "include_memory_refs"),
                include_validation: optional_query_bool(&query, "include_validation"),
            };
            (StatusCode::OK, Json(route_lookup_response(request))).into_response()
        },
    )
    .await
}

async fn route_handler(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    request: DagDbRouteRequest,
) -> Response {
    #[cfg(not(feature = "production-db"))]
    let _ = headers;
    #[cfg(feature = "production-db")]
    if let Some(pool) = &ctx.pool {
        let signature = match header_text(headers, WRITE_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "write_signature_required",
                    "DAG DB route persistence requires x-exo-write-signature header",
                    false,
                );
            }
        };
        let service = match ctx
            .gatekeeper_service(pool, &request.requesting_agent_did, &request.tenant_id)
            .await
        {
            Ok(service) => service,
            Err(error) => {
                let handler_error = DagDbHandlerError::from_gatekeeper(error);
                warn!(
                    route = "dagdb.route",
                    status = handler_error.status().as_u16(),
                    error_code = %handler_error.error_code(),
                    gatekeeper_error_class = %handler_error.class(),
                    "DAG DB default route authority resolver failed closed"
                );
                return handler_error.into_response();
            }
        };
        match gated_route_response(&service, &request, &signature).await {
            Ok(response) => {
                info!(
                    route = "dagdb.route",
                    status = 201,
                    tenant_id = %response.tenant_id,
                    namespace = %response.namespace,
                    route_id = %response.route_id,
                    "DAG DB default route persisted"
                );
                return (StatusCode::CREATED, Json(response)).into_response();
            }
            Err(error) => {
                warn!(
                    route = "dagdb.route",
                    status = error.status().as_u16(),
                    error_code = %error.error_code(),
                    gatekeeper_error_class = %error.class(),
                    "DAG DB default route persistence failed closed"
                );
                return error.into_response();
            }
        }
    }
    #[cfg(not(feature = "production-db"))]
    let _ = &request;
    warn!(
        route = "dagdb.route",
        status = 503,
        "DAG DB default route rejected because no governed route persistence is configured"
    );
    dagdb_route_database_unavailable_response("dagdb.route")
}

async fn gated_context_packet_handler(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    request: DagDbContextPacketRequest,
) -> Response {
    #[cfg(not(feature = "production-db"))]
    let _ = headers;
    #[cfg(feature = "production-db")]
    if let Some(pool) = &ctx.pool {
        let signature = match header_text(headers, WRITE_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "write_signature_required",
                    "DAG DB context packet persistence requires x-exo-write-signature header",
                    false,
                );
            }
        };
        let service = match ctx
            .gatekeeper_service(pool, &request.requesting_agent_did, &request.tenant_id)
            .await
        {
            Ok(service) => service,
            Err(error) => {
                let handler_error = DagDbHandlerError::from_gatekeeper(error);
                warn!(
                    route = "dagdb.context_packet",
                    status = handler_error.status().as_u16(),
                    error_code = %handler_error.error_code(),
                    gatekeeper_error_class = %handler_error.class(),
                    "DAG DB context packet authority resolver failed closed"
                );
                return handler_error.into_response();
            }
        };
        match gated_context_packet_response(&service, pool, &request, &signature).await {
            Ok(response) => {
                info!(
                    route = "dagdb.context_packet",
                    status = 200,
                    tenant_id = %response.tenant_id,
                    namespace = %response.namespace,
                    context_packet_id = %response.context_packet_id,
                    "DAG DB context packet persisted"
                );
                return (StatusCode::OK, Json(response)).into_response();
            }
            Err(error) => {
                warn!(
                    route = "dagdb.context_packet",
                    status = error.status().as_u16(),
                    error_code = %error.error_code(),
                    gatekeeper_error_class = %error.class(),
                    "DAG DB context packet persistence failed closed"
                );
                return error.into_response();
            }
        }
    }
    context_packet_handler(ctx, request).await
}

async fn context_packet_handler(
    ctx: &DagDbRouteContext,
    request: DagDbContextPacketRequest,
) -> Response {
    #[cfg(feature = "production-db")]
    {
        if let Some(pool) = &ctx.pool {
            match persistent_context_packet_response(pool, &request).await {
                Ok(response) => {
                    let mode = response
                        .context_packet_mode
                        .clone()
                        .unwrap_or_else(|| "database".to_owned());
                    if response.memory_refs.is_empty() {
                        info!(
                            route = "dagdb.context_packet",
                            status = 200,
                            mode = %mode,
                            tenant_id = %response.tenant_id,
                            namespace = %response.namespace,
                            "DAG DB context packet served with empty selection"
                        );
                    } else {
                        info!(
                            route = "dagdb.context_packet",
                            status = 200,
                            mode = %mode,
                            tenant_id = %response.tenant_id,
                            namespace = %response.namespace,
                            memory_ref_count = response.memory_refs.len(),
                            token_estimate = response.token_estimate,
                            "DAG DB context packet served from database"
                        );
                    }
                    return (StatusCode::OK, Json(response)).into_response();
                }
                Err(error) => {
                    warn!(
                        route = "dagdb.context_packet",
                        status = error.status().as_u16(),
                        error_code = %error.error_code(),
                        "DAG DB context packet database path failed closed"
                    );
                    return error.into_response();
                }
            }
        }
    }
    let _ = ctx;
    if let Err(response) = context_packet_layered_fields(&request, 0, false) {
        return *response;
    }
    warn!(
        route = "dagdb.context_packet",
        status = 503,
        "DAG DB context packet rejected because no governed database pool is configured"
    );
    dagdb_route_database_unavailable_response("dagdb.context_packet")
}

async fn writeback_handler(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    request: DagDbWritebackRequest,
) -> Response {
    // Fail closed on the optional typed-knowledge class before any persistence
    // or signature handling, identically in the production-db and scaffold
    // paths. A classless writeback is unchanged telemetry.
    if let Err(response) = validate_writeback_knowledge_class(&request) {
        return *response;
    }
    #[cfg(not(feature = "production-db"))]
    let _ = (ctx, headers);
    #[cfg(feature = "production-db")]
    if let Some(pool) = &ctx.pool {
        let signature = match header_text(headers, WRITE_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "write_signature_required",
                    "DAG DB writeback requires x-exo-write-signature header",
                    false,
                );
            }
        };
        let lifecycle_signature = match header_text(headers, LIFECYCLE_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "lifecycle_signature_required",
                    "DAG DB writeback lifecycle persistence requires x-exo-lifecycle-signature header",
                    false,
                );
            }
        };
        let continuation_signature = match header_text(headers, CONTINUATION_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "continuation_signature_required",
                    "DAG DB writeback continuation persistence requires x-exo-continuation-signature header",
                    false,
                );
            }
        };
        let service = match ctx
            .gatekeeper_service(pool, &request.requesting_agent_did, &request.tenant_id)
            .await
        {
            Ok(service) => service,
            Err(error) => {
                let handler_error = DagDbHandlerError::from_gatekeeper(error);
                warn!(
                    route = "dagdb.writeback",
                    status = handler_error.status().as_u16(),
                    error_code = %handler_error.error_code(),
                    gatekeeper_error_class = %handler_error.class(),
                    "DAG DB writeback authority resolver failed closed"
                );
                return handler_error.into_response();
            }
        };
        match gated_writeback_response(
            &service,
            pool,
            &request,
            &signature,
            &lifecycle_signature,
            &continuation_signature,
        )
        .await
        {
            Ok(response) => {
                info!(
                    route = "dagdb.writeback",
                    status = 200,
                    tenant_id = %response.tenant_id,
                    namespace = %response.namespace,
                    receipt_hash = %response.receipt_hash,
                    "DAG DB writeback persisted"
                );
                return (StatusCode::OK, Json(response)).into_response();
            }
            Err(error) => {
                warn!(
                    route = "dagdb.writeback",
                    status = error.status().as_u16(),
                    error_code = %error.error_code(),
                    gatekeeper_error_class = %error.class(),
                    "DAG DB writeback gate failed closed"
                );
                return error.into_response();
            }
        }
    }
    // Fail closed when no production database pool is configured: a writeback is
    // a governed graph mutation and must never return a synthetic success or a
    // fabricated receipt. This matches the import/export no-pool behavior (503,
    // `database_unavailable`) so all three mutation surfaces are uniformly
    // fail-closed off the production-db path.
    #[cfg(not(feature = "production-db"))]
    let _ = &request;
    warn!(
        route = "dagdb.writeback",
        status = 503,
        "DAG DB writeback rejected because no production database pool is configured"
    );
    dagdb_runtime_database_unavailable_response("writeback")
}

async fn import_handler(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    request: DagDbImportRequest,
) -> Response {
    #[cfg(not(feature = "production-db"))]
    let _ = (ctx, headers);
    let report_json = match validated_import_report_json(&request) {
        Ok(report_json) => report_json,
        Err(response) => return *response,
    };
    #[cfg(feature = "production-db")]
    let request_hash = match import_route_request_hash(&request) {
        Ok(request_hash) => request_hash,
        Err(response) => return *response,
    };
    #[cfg(not(feature = "production-db"))]
    let _ = &report_json;
    #[cfg(feature = "production-db")]
    if let Some(pool) = &ctx.pool {
        let signature = match header_text(headers, WRITE_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                warn!(
                    route = "dagdb.import",
                    status = 400,
                    error_code = "write_signature_required",
                    tenant_id = %request.tenant_id,
                    namespace = %request.namespace,
                    "DAG DB import rejected because write signature is missing"
                );
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "write_signature_required",
                    "DAG DB import requires x-exo-write-signature header",
                    false,
                );
            }
        };
        let replayed_response = match reserve_gateway_idempotency_key(
            pool,
            &request.tenant_id,
            &request.namespace,
            IMPORT_ROUTE_IDEMPOTENCY_NAME,
            &request.idempotency_key,
            request_hash,
            "import",
        )
        .await
        {
            Ok(GatewayIdempotencyDecision::Reserved) => None,
            Ok(GatewayIdempotencyDecision::Replayed(response)) => Some(response),
            Err(response) => return *response,
        };
        let service = match ctx
            .gatekeeper_service(pool, &request.requester_did, &request.tenant_id)
            .await
        {
            Ok(service) => service,
            Err(error) => {
                if replayed_response.is_none() {
                    if let Err(cleanup_error) =
                        cleanup_gateway_idempotency_reservation(pool, &request, request_hash).await
                    {
                        return *cleanup_error;
                    }
                }
                let handler_error = DagDbHandlerError::from_gatekeeper(error);
                warn!(
                    route = "dagdb.import",
                    status = handler_error.status().as_u16(),
                    error_code = %handler_error.error_code(),
                    gatekeeper_error_class = %handler_error.class(),
                    tenant_id = %request.tenant_id,
                    namespace = %request.namespace,
                    "DAG DB import authority resolver failed closed"
                );
                return handler_error.into_response();
            }
        };
        let authorization_payload_hash = match gated_import_authorization(
            &service,
            pool,
            &request,
            &signature,
            replayed_response.is_none(),
            replayed_response
                .as_ref()
                .and_then(|response| response.authorization_payload_hash),
        )
        .await
        {
            Ok(authorization_payload_hash) => authorization_payload_hash,
            Err(error) => {
                if replayed_response.is_none() {
                    if let Err(cleanup_error) =
                        cleanup_gateway_idempotency_reservation(pool, &request, request_hash).await
                    {
                        return *cleanup_error;
                    }
                }
                warn!(
                    route = "dagdb.import",
                    status = error.status().as_u16(),
                    error_code = %error.error_code(),
                    gatekeeper_error_class = %error.class(),
                    tenant_id = %request.tenant_id,
                    namespace = %request.namespace,
                    "DAG DB import gate failed closed"
                );
                return error.into_response();
            }
        };
        if let Some(response) = replayed_response {
            return response.response;
        }
        match exo_dag_db_postgres::postgres::kg_import::persist_kg_import_report(pool, &report_json)
            .await
        {
            Ok(summary) => {
                let status = if summary.replayed {
                    "replayed"
                } else {
                    "persisted"
                };
                match import_response_from_summary(request.clone(), summary, status) {
                    Ok(response) => {
                        if let Err(error) = store_gateway_idempotency_response(
                            pool,
                            &request.tenant_id,
                            &request.namespace,
                            IMPORT_ROUTE_IDEMPOTENCY_NAME,
                            &request.idempotency_key,
                            request_hash,
                            StatusCode::OK,
                            serde_json::to_value(&response)
                                .map_err(|_| import_idempotency_unavailable_response()),
                            Some(authorization_payload_hash),
                            "import",
                        )
                        .await
                        {
                            return *error;
                        }
                        info!(
                            route = "dagdb.import",
                            status = 200,
                            tenant_id = %response.tenant_id,
                            namespace = %response.namespace,
                            import_status = %response.import_status,
                            "DAG DB import persisted"
                        );
                        return (StatusCode::OK, Json(response)).into_response();
                    }
                    Err(response) => {
                        if let Err(cleanup_error) =
                            cleanup_gateway_idempotency_reservation(pool, &request, request_hash)
                                .await
                        {
                            return *cleanup_error;
                        }
                        return *response;
                    }
                }
            }
            Err(error) => {
                if let Err(cleanup_error) =
                    cleanup_gateway_idempotency_reservation(pool, &request, request_hash).await
                {
                    return *cleanup_error;
                }
                return dagdb_import_adapter_error_response(&request, &error);
            }
        }
    }
    warn!(
        route = "dagdb.import",
        status = 503,
        "DAG DB import rejected because no production database pool is configured"
    );
    dagdb_runtime_database_unavailable_response("import")
}

async fn export_handler(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    request: DagDbExportRequest,
) -> Response {
    #[cfg(not(feature = "production-db"))]
    let _ = (ctx, headers);
    let scope = match export_scope_from_request(&request) {
        Ok(scope) => scope,
        Err(response) => return *response,
    };
    #[cfg(not(feature = "production-db"))]
    let _ = &scope;
    #[cfg(feature = "production-db")]
    if let Some(pool) = &ctx.pool {
        let signature = match header_text(headers, WRITE_SIGNATURE_HEADER) {
            Some(signature) => signature.to_owned(),
            None => {
                warn!(
                    route = "dagdb.export",
                    status = 400,
                    error_code = "write_signature_required",
                    tenant_id = %request.tenant_id,
                    namespace = %request.namespace,
                    "DAG DB export rejected because write signature is missing"
                );
                return dagdb_error_response(
                    StatusCode::BAD_REQUEST,
                    "write_signature_required",
                    "DAG DB export requires x-exo-write-signature header",
                    false,
                );
            }
        };
        let request_hash = match export_route_request_hash(&request) {
            Ok(request_hash) => request_hash,
            Err(response) => return *response,
        };
        let replayed_response = match reserve_gateway_idempotency_key(
            pool,
            &request.tenant_id,
            &request.namespace,
            EXPORT_ROUTE_IDEMPOTENCY_NAME,
            &request.idempotency_key,
            request_hash,
            "export",
        )
        .await
        {
            Ok(GatewayIdempotencyDecision::Reserved) => None,
            Ok(GatewayIdempotencyDecision::Replayed(response)) => Some(response),
            Err(response) => return *response,
        };
        let service = match ctx
            .gatekeeper_service(pool, &request.requester_did, &request.tenant_id)
            .await
        {
            Ok(service) => service,
            Err(error) => {
                if replayed_response.is_none() {
                    if let Err(cleanup_error) =
                        cleanup_export_idempotency_reservation(pool, &request, request_hash).await
                    {
                        return *cleanup_error;
                    }
                }
                let handler_error = DagDbHandlerError::from_gatekeeper(error);
                warn!(
                    route = "dagdb.export",
                    status = handler_error.status().as_u16(),
                    error_code = %handler_error.error_code(),
                    gatekeeper_error_class = %handler_error.class(),
                    tenant_id = %request.tenant_id,
                    namespace = %request.namespace,
                    "DAG DB export authority resolver failed closed"
                );
                return handler_error.into_response();
            }
        };
        let authorization_payload_hash = match gated_export_authorization(
            &service,
            pool,
            &request,
            request_hash,
            &signature,
            replayed_response.is_none(),
            replayed_response
                .as_ref()
                .and_then(|response| response.authorization_payload_hash),
        )
        .await
        {
            Ok(authorization_payload_hash) => authorization_payload_hash,
            Err(error) => {
                if replayed_response.is_none() {
                    if let Err(cleanup_error) =
                        cleanup_export_idempotency_reservation(pool, &request, request_hash).await
                    {
                        return *cleanup_error;
                    }
                }
                warn!(
                    route = "dagdb.export",
                    status = error.status().as_u16(),
                    error_code = %error.error_code(),
                    gatekeeper_error_class = %error.class(),
                    tenant_id = %request.tenant_id,
                    namespace = %request.namespace,
                    "DAG DB export gate failed closed"
                );
                return error.into_response();
            }
        };
        if let Some(response) = replayed_response {
            return response.response;
        }
        match exo_dag_db_postgres::postgres::kg_export::build_kg_portable_export(pool, &scope, &[])
            .await
        {
            Ok(export) => match export_response_from_portable(request.clone(), export) {
                Ok(response) => {
                    if let Err(error) = store_gateway_idempotency_response(
                        pool,
                        &response.tenant_id,
                        &response.namespace,
                        EXPORT_ROUTE_IDEMPOTENCY_NAME,
                        &response.idempotency_key,
                        request_hash,
                        StatusCode::OK,
                        serde_json::to_value(&response)
                            .map_err(|_| export_idempotency_unavailable_response()),
                        Some(authorization_payload_hash),
                        "export",
                    )
                    .await
                    {
                        return *error;
                    }
                    info!(
                        route = "dagdb.export",
                        status = 200,
                        tenant_id = %response.tenant_id,
                        namespace = %response.namespace,
                        export_status = %response.export_status,
                        "DAG DB export built from database"
                    );
                    return (StatusCode::OK, Json(response)).into_response();
                }
                Err(response) => {
                    if let Err(cleanup_error) =
                        cleanup_export_idempotency_reservation(pool, &request, request_hash).await
                    {
                        return *cleanup_error;
                    }
                    return *response;
                }
            },
            Err(error) => {
                if let Err(cleanup_error) =
                    cleanup_export_idempotency_reservation(pool, &request, request_hash).await
                {
                    return *cleanup_error;
                }
                return dagdb_export_adapter_error_response(&request, &error);
            }
        }
    }
    warn!(
        route = "dagdb.export",
        status = 503,
        "DAG DB export rejected because no production database pool is configured"
    );
    dagdb_runtime_database_unavailable_response("export")
}

#[cfg(feature = "production-db")]
async fn persistent_context_packet_response(
    pool: &sqlx::PgPool,
    request: &DagDbContextPacketRequest,
) -> Result<DagDbContextPacketResponse, DagDbHandlerError> {
    // D1-S4: validate the layered mode and the depth reserve the same way the
    // depth param is validated, then dispatch. When `layered_mode != off` the
    // handler calls the membership-triggered DRILLDOWN entrypoint with the
    // reserve; off-mode stays on the breadth-only path and is byte-identical.
    let mode = layered_mode_value(&request.layered_mode)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    let max_layer_depth = validate_max_layer_depth(request.max_layer_depth)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    let drilldown_reserve_bp = validate_drilldown_reserve_bp(request.drilldown_reserve_bp)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;

    let build_request = graph_context_packet_build_request(request);
    let persistent = if mode == "off" {
        build_persistent_graph_context_packet(pool, &build_request)
            .await
            .map_err(DagDbHandlerError::from_domain)?
    } else {
        build_persistent_graph_context_packet_with_layered_drilldown(
            pool,
            &build_request,
            Some(mode),
            Some(max_layer_depth),
            drilldown_reserve_bp,
        )
        .await
        .map_err(DagDbHandlerError::from_domain)?
    };
    context_packet_response_from_persistent(request, &persistent)
        .map_err(|response| DagDbHandlerError::from_response(*response))
}

#[cfg(feature = "production-db")]
async fn gated_route_response(
    service: &DagDbGatekeeperService,
    request: &DagDbRouteRequest,
    signature: &str,
) -> Result<DagDbRouteResponse, DagDbHandlerError> {
    let response = route_response_from_request(request.clone(), "dagdb.route")
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    let record = default_route_record_from_response(request, &response)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    let invariant_context =
        service.dagdb_invariant_context(&request.tenant_id, &request.requesting_agent_did);
    service
        .persist_default_route(
            &record,
            &request.requesting_agent_did,
            signature,
            invariant_context.as_ref(),
        )
        .await
        .map_err(DagDbHandlerError::from_gatekeeper)?;
    Ok(response)
}

#[cfg(feature = "production-db")]
async fn gated_context_packet_response(
    service: &DagDbGatekeeperService,
    pool: &sqlx::PgPool,
    request: &DagDbContextPacketRequest,
    signature: &str,
) -> Result<DagDbContextPacketResponse, DagDbHandlerError> {
    let response = persistent_context_packet_response(pool, request).await?;
    let record = context_packet_record_from_response(request, &response)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    let invariant_context =
        service.dagdb_invariant_context(&request.tenant_id, &request.requesting_agent_did);
    service
        .persist_context_packet_record(
            &record,
            &request.requesting_agent_did,
            signature,
            invariant_context.as_ref(),
        )
        .await
        .map_err(DagDbHandlerError::from_gatekeeper)?;
    Ok(response)
}

#[cfg(feature = "production-db")]
async fn gated_writeback_response(
    service: &DagDbGatekeeperService,
    pool: &sqlx::PgPool,
    request: &DagDbWritebackRequest,
    signature: &str,
    lifecycle_signature: &str,
    continuation_signature: &str,
) -> Result<DagDbWritebackResponse, DagDbHandlerError> {
    let selection_request = selection_request_from_writeback(request).map_err(|error| {
        DagDbHandlerError::from_domain(DomainError::HashMaterial {
            reason: error.to_string(),
        })
    })?;
    let selection = build_persistent_graph_context_selection(pool, &selection_request)
        .await
        .map_err(DagDbHandlerError::from_domain)?;
    let memory_metadata = writeback_usage_event_metadata(request);
    let invariant_context = service.dagdb_invariant_context(
        &selection.selection.tenant_id,
        &request.requesting_agent_did,
    );
    let lifecycle_action = lifecycle_action_from_writeback(request)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    let continuation = continuation_record_from_writeback(request)
        .map_err(|response| DagDbHandlerError::from_response(*response))?;
    prevalidate_writeback_d5_gates(
        service,
        &selection.selection,
        lifecycle_action.tenant_id.as_str(),
        &request.requesting_agent_did,
        &lifecycle_action,
        &continuation,
        signature,
        lifecycle_signature,
        continuation_signature,
    )?;
    let summary = service
        .persist_usage_event_with_metadata(
            &selection.selection,
            &request.requesting_agent_did,
            signature,
            invariant_context.as_ref(),
            Some(&memory_metadata),
        )
        .await
        .map_err(DagDbHandlerError::from_gatekeeper)?;
    persist_writeback_d5_surfaces(
        service,
        pool,
        request,
        WritebackD5Surfaces {
            lifecycle_action: &lifecycle_action,
            continuation: &continuation,
            lifecycle_signature,
            continuation_signature,
            invariant_context: invariant_context.as_ref(),
        },
    )
    .await?;
    match writeback_response_from_persisted(request, &summary, !summary.replayed) {
        Ok(response) => Ok(response),
        Err(response) => Err(DagDbHandlerError::from_response(*response)),
    }
}

#[cfg(feature = "production-db")]
struct WritebackD5Surfaces<'a> {
    lifecycle_action: &'a LifecycleAction,
    continuation: &'a ContinuationRecord,
    lifecycle_signature: &'a str,
    continuation_signature: &'a str,
    invariant_context: Option<&'a InvariantContext>,
}

#[cfg(feature = "production-db")]
async fn persist_writeback_d5_surfaces(
    service: &DagDbGatekeeperService,
    pool: &sqlx::PgPool,
    request: &DagDbWritebackRequest,
    surfaces: WritebackD5Surfaces<'_>,
) -> Result<(), DagDbHandlerError> {
    service
        .persist_lifecycle_action(
            surfaces.lifecycle_action,
            &request.requesting_agent_did,
            surfaces.lifecycle_signature,
            surfaces.invariant_context,
        )
        .await
        .map_err(DagDbHandlerError::from_gatekeeper)?;

    let now_epoch_seconds = trusted_gateway_epoch_seconds(pool).await?;
    service
        .persist_continuation_record(
            surfaces.continuation,
            now_epoch_seconds,
            &request.requesting_agent_did,
            surfaces.continuation_signature,
            surfaces.invariant_context,
        )
        .await
        .map_err(DagDbHandlerError::from_gatekeeper)?;
    Ok(())
}

#[cfg(feature = "production-db")]
#[allow(clippy::too_many_arguments)]
fn prevalidate_writeback_d5_gates(
    service: &DagDbGatekeeperService,
    selection: &DagDbGraphContextSelectionResponse,
    tenant_id: &str,
    agent_did: &str,
    lifecycle_action: &LifecycleAction,
    continuation: &ContinuationRecord,
    signature: &str,
    lifecycle_signature: &str,
    continuation_signature: &str,
) -> Result<(), DagDbHandlerError> {
    let writeback_payload_hash =
        usage_event_payload_hash(selection).map_err(DagDbHandlerError::from_gatekeeper)?;
    validate_gateway_write_payload(
        service,
        &selection.tenant_id,
        agent_did,
        &writeback_payload_hash,
        signature,
    )?;

    let lifecycle_payload_hash =
        exo_gatekeeper::dagdb_gate::lifecycle_action_payload_hash(lifecycle_action)
            .map_err(DagDbHandlerError::from_gatekeeper)?;
    validate_gateway_write_payload(
        service,
        tenant_id,
        agent_did,
        &lifecycle_payload_hash,
        lifecycle_signature,
    )?;

    let continuation_payload_hash =
        exo_gatekeeper::dagdb_gate::continuation_record_payload_hash(continuation)
            .map_err(DagDbHandlerError::from_gatekeeper)?;
    validate_gateway_write_payload(
        service,
        &continuation.tenant_id,
        agent_did,
        &continuation_payload_hash,
        continuation_signature,
    )
}

#[cfg(feature = "production-db")]
fn validate_gateway_write_payload(
    service: &DagDbGatekeeperService,
    tenant_id: &str,
    agent_did: &str,
    payload_hash: &[u8; 32],
    signature: &str,
) -> Result<(), DagDbHandlerError> {
    match verify_write_consent(
        service.consent_engine.as_ref(),
        tenant_id,
        agent_did,
        ConsentPurpose::Writeback,
    ) {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            return Err(DagDbHandlerError::from_gatekeeper(
                GatekeeperError::InvariantViolation("ConsentRequired".to_owned()),
            ));
        }
    }

    match verify_write_signature(
        service.identity_registry.as_ref(),
        payload_hash,
        signature,
        agent_did,
    ) {
        Ok(true) => Ok(()),
        Ok(false) | Err(_) => Err(DagDbHandlerError::from_gatekeeper(
            GatekeeperError::InvariantViolation("ProvenanceVerifiable".to_owned()),
        )),
    }
}

#[cfg(feature = "production-db")]
async fn gated_import_authorization(
    service: &DagDbGatekeeperService,
    pool: &sqlx::PgPool,
    request: &DagDbImportRequest,
    signature: &str,
    persist_usage_event: bool,
    replay_authorization_payload_hash: Option<Hash256>,
) -> Result<Hash256, DagDbHandlerError> {
    let _ = (
        service,
        pool,
        request,
        signature,
        persist_usage_event,
        replay_authorization_payload_hash,
    );
    Err(import_export_consent_not_configured_error("import"))
}

#[cfg(feature = "production-db")]
async fn gated_export_authorization(
    service: &DagDbGatekeeperService,
    pool: &sqlx::PgPool,
    request: &DagDbExportRequest,
    request_hash: Hash256,
    signature: &str,
    persist_usage_event: bool,
    replay_authorization_payload_hash: Option<Hash256>,
) -> Result<Hash256, DagDbHandlerError> {
    let _ = (
        service,
        pool,
        request,
        request_hash,
        signature,
        persist_usage_event,
        replay_authorization_payload_hash,
    );
    Err(import_export_consent_not_configured_error("export"))
}

#[cfg(feature = "production-db")]
fn import_export_consent_not_configured_error(operation: &'static str) -> DagDbHandlerError {
    DagDbHandlerError {
        status: StatusCode::FORBIDDEN,
        error_code: "consent_denied",
        class: "consent",
        message: format!(
            "DAG DB {operation} requires distinct import/export consent, which is not configured or supported yet"
        ),
        requires_council_review: true,
    }
}

#[cfg(feature = "production-db")]
fn graph_context_packet_build_request(
    request: &DagDbContextPacketRequest,
) -> DagDbGraphContextPacketBuildRequest {
    let placeholder_selection = DagDbGraphContextSelectionResponse {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task_hash: request.task_hash.clone(),
        selection_status: DagDbGraphContextSelectionStatus::Empty,
        selected_memory_refs: Vec::new(),
        selected_graph_edges: Vec::new(),
        omitted_memory_refs: Vec::new(),
        selection_trace: Vec::new(),
        selected_token_estimate: 0,
        token_budget: request.token_budget,
        boundary_warnings: Vec::new(),
    };
    let task = request
        .task
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("route:{}", request.route_id));
    DagDbGraphContextPacketBuildRequest {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task,
        task_hash: request.task_hash.clone(),
        audit_id: request.idempotency_key.clone(),
        token_budget: request.token_budget,
        selection: placeholder_selection,
        import_tracking_status: None,
    }
}

/// Derive the selection task hash signed for a DAG DB writeback usage event.
///
/// Flat legacy writebacks with no searchable metadata sign the raw
/// `answer_hash`. When searchable metadata (`summary_text`, `knowledge_class`)
/// or any layered writeback field is present, all optional metadata and layer
/// fields are bound into the signed material with explicit absent/present
/// encoding so a relayed signature cannot be replayed with mutated searchable
/// metadata or a mutated layer target.
#[cfg(feature = "production-db")]
pub fn writeback_signed_task_hash(
    request: &DagDbWritebackRequest,
) -> Result<String, KgImportError> {
    let metadata_fields_present =
        request.summary_text.is_some() || request.knowledge_class.is_some();
    let layer_fields_present = request.layered_mode.is_some()
        || request.target_layer_path.is_some()
        || request.target_layer_depth.is_some()
        || request.target_layer_reason.is_some();
    if !metadata_fields_present && !layer_fields_present {
        return Ok(request.answer_hash.clone());
    }
    let summary_text =
        writeback_signed_task_hash_part("summary_text", request.summary_text.as_deref());
    let knowledge_class =
        writeback_signed_task_hash_part("knowledge_class", request.knowledge_class.as_deref());
    let layered_mode =
        writeback_signed_task_hash_part("layered_mode", request.layered_mode.as_deref());
    let target_layer_path =
        writeback_signed_task_hash_part("target_layer_path", request.target_layer_path.as_deref());
    let target_layer_depth_value = request.target_layer_depth.map(|depth| depth.to_string());
    let target_layer_depth =
        writeback_signed_task_hash_part("target_layer_depth", target_layer_depth_value.as_deref());
    let target_layer_reason = writeback_signed_task_hash_part(
        "target_layer_reason",
        request.target_layer_reason.as_deref(),
    );
    Ok(exo_dag_db_exchange::kg_import::stable_hash(
        DAGDB_WRITEBACK_SIGNED_TASK_HASH_DOMAIN,
        &[
            &request.answer_hash,
            &summary_text,
            &knowledge_class,
            &layered_mode,
            &target_layer_path,
            &target_layer_depth,
            &target_layer_reason,
        ],
    )?
    .to_string())
}

/// Encode an optional writeback signing field so absent and empty values bind
/// to distinct signed material.
#[cfg(feature = "production-db")]
fn writeback_signed_task_hash_part(field: &str, value: Option<&str>) -> String {
    match value {
        Some(value) => format!("{field}:present:{value}"),
        None => format!("{field}:absent"),
    }
}

/// Build the graph-context selection request whose persisted selection forms
/// the signed payload for a DAG DB writeback.
///
/// Shared with the `dagdb_writeback_sign` binary so the signer and the gateway
/// derive identical signing material, including searchable metadata and layered
/// target binding.
#[cfg(feature = "production-db")]
pub fn selection_request_from_writeback(
    request: &DagDbWritebackRequest,
) -> Result<DagDbGraphContextSelectionRequest, KgImportError> {
    let max_memory_refs = u32::try_from(request.parent_memory_ids.len())
        .unwrap_or(u32::MAX)
        .clamp(1, 64);
    Ok(DagDbGraphContextSelectionRequest {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.idempotency_key.clone(),
        task: format!("writeback:{}", request.context_packet_id),
        task_hash: writeback_signed_task_hash(request)?,
        token_budget: 2_048,
        max_memory_refs,
        catalog_hints: Vec::new(),
        requested_memory_ids: request.parent_memory_ids.clone(),
        force_revalidate: false,
    })
}

#[cfg(feature = "production-db")]
pub fn writeback_lifecycle_payload_hash(
    request: &DagDbWritebackRequest,
) -> Result<[u8; 32], String> {
    let action = lifecycle_action_from_writeback(request)
        .map_err(|_| "lifecycle action request rejected".to_owned())?;
    exo_gatekeeper::dagdb_gate::lifecycle_action_payload_hash(&action)
        .map_err(|error| error.to_string())
}

#[cfg(feature = "production-db")]
pub fn writeback_continuation_payload_hash(
    request: &DagDbWritebackRequest,
) -> Result<[u8; 32], String> {
    let record = continuation_record_from_writeback(request)
        .map_err(|_| "continuation request rejected".to_owned())?;
    exo_gatekeeper::dagdb_gate::continuation_record_payload_hash(&record)
        .map_err(|error| error.to_string())
}

#[cfg(feature = "production-db")]
fn default_route_record_from_response(
    request: &DagDbRouteRequest,
    response: &DagDbRouteResponse,
) -> Result<DefaultRouteRecord, Box<Response>> {
    let memory_ids = request.requested_memory_ids.clone().unwrap_or_default();
    if memory_ids.is_empty() {
        return Err(d5_record_rejected_response(
            "default route",
            "selected memory refs are required for default-route persistence",
        ));
    }
    let selected_memory_refs = sorted_strings(memory_ids)
        .into_iter()
        .map(|memory_id| {
            Ok(DefaultRouteMemoryRef {
                latest_receipt_hash: hash_hex(
                    "dagdb.gateway.default_route.memory_receipt",
                    &(&response.route_id, &memory_id),
                )?,
                citation_ref: hash_hex(
                    "dagdb.gateway.default_route.citation",
                    &(&response.route_id, &memory_id),
                )?,
                validation_status: "passed".to_owned(),
                memory_id,
            })
        })
        .collect::<Result<Vec<_>, Box<Response>>>()?;
    Ok(DefaultRouteRecord {
        schema_version: DEFAULT_ROUTE_SCHEMA_VERSION.to_owned(),
        route_id: response.route_id.clone(),
        tenant_id: request.tenant_id.clone(),
        project_id: gateway_project_id(&request.namespace),
        memory_namespace: request.namespace.clone(),
        status: DefaultRouteStatus::Active,
        route_source: DefaultRouteSource::Persisted,
        policy_ref: request.approved_scope_hash.clone(),
        freshness_ref: request.task_signature_hash.clone(),
        policy_allowed: true,
        freshness_status: RouteFreshnessStatus::Current,
        invalidated: false,
        production_default_route_approval_status: "operator_deferred".to_owned(),
        packet_quality_review_status: "operator_deferred".to_owned(),
        selected_memory_refs,
        created_at: gateway_record_stamp("dagdb.route.created_at", &request.idempotency_key)?,
        updated_at: gateway_record_stamp("dagdb.route.updated_at", &request.idempotency_key)?,
    })
}

#[cfg(feature = "production-db")]
fn context_packet_record_from_response(
    request: &DagDbContextPacketRequest,
    response: &DagDbContextPacketResponse,
) -> Result<ContextPacketRecord, Box<Response>> {
    let selected_memory_ids = sorted_strings(
        response
            .memory_refs
            .iter()
            .map(|memory_ref| memory_ref.memory_id.clone())
            .collect(),
    );
    let selected_edge_ids = sorted_strings(
        response
            .selected_graph_edges
            .iter()
            .map(|edge| edge.graph_edge_id.clone())
            .collect(),
    );
    let source_proof_refs = sorted_strings(
        response
            .memory_refs
            .iter()
            .map(|memory_ref| memory_ref.latest_receipt_hash.clone())
            .collect(),
    );
    let binding = ContextPacketRouteBinding {
        route_id: response.route_id.clone(),
        tenant_id: response.tenant_id.clone(),
        project_id: gateway_project_id(&response.namespace),
        memory_namespace: response.namespace.clone(),
        production_default_route_approval_status: "operator_deferred".to_owned(),
        packet_quality_review_status: "operator_deferred".to_owned(),
        route_freshness_status: PacketFreshnessStatus::Current,
    };
    let validation_passed = response.validation_status == ValidationStatus::Passed;
    let packet_request = ContextPacketRequest {
        packet_id: response.context_packet_id.clone(),
        query_hash: request.task_hash.clone(),
        selected_memory_ids,
        selected_edge_ids,
        token_budget: response.token_budget,
        token_estimate: response.token_estimate,
        citation_coverage_bp: if validation_passed { 10_000 } else { 0 },
        validation_coverage_bp: if validation_passed { 10_000 } else { 0 },
        source_proof_refs,
        context_quality: if response.memory_refs.is_empty() {
            DefaultContextQuality::EmptyContext
        } else {
            DefaultContextQuality::UsableContext
        },
        freshness_status: PacketFreshnessStatus::Current,
        validation_status: if validation_passed {
            PacketValidationStatus::Passed
        } else {
            PacketValidationStatus::Failed
        },
        persistence_status: PacketPersistenceStatus::ProofBound,
        fallback_reason: response.selection_warning.clone(),
        raw_body_present: false,
        created_at: gateway_record_stamp(
            "dagdb.context_packet.created_at",
            &request.idempotency_key,
        )?,
    };
    build_context_packet_record(&binding, packet_request)
        .map_err(|error| d5_record_rejected_response("context packet", error))
}

#[cfg(feature = "production-db")]
fn lifecycle_action_from_writeback(
    request: &DagDbWritebackRequest,
) -> Result<LifecycleAction, Box<Response>> {
    let parent_memory_ids = sorted_strings(request.parent_memory_ids.clone());
    if parent_memory_ids.is_empty() {
        return Err(d5_record_rejected_response(
            "lifecycle action",
            "parent memory refs are required for lifecycle persistence",
        ));
    }
    let parent_memory_refs =
        lifecycle_refs_from_memory_ids(&request.tenant_id, &request.namespace, parent_memory_ids);
    let target_memory_id = writeback_target_memory_id(request)?;
    let target_memory_refs = lifecycle_refs_from_memory_ids(
        &request.tenant_id,
        &request.namespace,
        vec![target_memory_id.clone()],
    );
    let action_id = hash_hex(
        "dagdb.gateway.lifecycle_action",
        &(&request.idempotency_key, &request.answer_hash),
    )?;
    Ok(LifecycleAction {
        schema_version: PRD17_LIFECYCLE_ACTION_SCHEMA.to_owned(),
        action_id: action_id.clone(),
        action_type: LifecycleActionType::Writeback,
        tenant_id: request.tenant_id.clone(),
        project_id: gateway_project_id(&request.namespace),
        memory_namespace: request.namespace.clone(),
        actor_id: request.requesting_agent_did.clone(),
        source_packet_id: request.context_packet_id.clone(),
        source_receipt_id: request.validation_report_id.clone(),
        parent_memory_ids: parent_memory_refs.clone(),
        target_memory_ids: target_memory_refs.clone(),
        validation_report_id: request.validation_report_id.clone(),
        policy_ref: request.route_id.clone(),
        rollback_ref: LifecycleRollbackRef {
            rollback_id: hash_hex("dagdb.gateway.lifecycle.rollback", &action_id)?,
            action_id: action_id.clone(),
            inverse_action_type: LifecycleActionType::Archive,
            before_refs: parent_memory_refs,
            after_refs: target_memory_refs,
            validation_ref: request.validation_report_id.clone(),
            operator_required: true,
        },
        route_invalidation_event_ids: vec![hash_hex(
            "dagdb.gateway.lifecycle.route_invalidation",
            &(&request.route_id, &request.context_packet_id),
        )?],
        evidence_refs: vec![LifecycleEvidenceRef {
            evidence_id: hash_hex("dagdb.gateway.lifecycle.evidence", &request.answer_hash)?,
            receipt_id: request.validation_report_id.clone(),
            digest: request.answer_hash.clone(),
            summary_ref: hash_hex(
                "dagdb.gateway.lifecycle.summary_ref",
                &(&request.idempotency_key, &target_memory_id),
            )?,
            preserved: true,
        }],
        terminal_state: LifecycleTerminalState::OperatorDeferred,
        production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
        created_at: gateway_record_stamp(
            "dagdb.lifecycle_action.created_at",
            &request.idempotency_key,
        )?,
    })
}

#[cfg(feature = "production-db")]
fn continuation_record_from_writeback(
    request: &DagDbWritebackRequest,
) -> Result<ContinuationRecord, Box<Response>> {
    let mut memory_ids = request.parent_memory_ids.clone();
    memory_ids.push(writeback_target_memory_id(request)?);
    let memory_refs = lifecycle_refs_from_memory_ids(
        &request.tenant_id,
        &request.namespace,
        sorted_strings(memory_ids),
    );
    Ok(ContinuationRecord {
        schema_version: PRD17_CONTINUATION_RECORD_SCHEMA.to_owned(),
        continuation_id: hash_hex(
            "dagdb.gateway.continuation",
            &(&request.idempotency_key, &request.context_packet_id),
        )?,
        task_id: request.idempotency_key.clone(),
        tenant_id: request.tenant_id.clone(),
        project_id: gateway_project_id(&request.namespace),
        memory_namespace: request.namespace.clone(),
        summary_ref: writeback_target_memory_id(request)?,
        memory_refs,
        blocker_refs: vec!["production_lifecycle_approval_deferred".to_owned()],
        validation_refs: vec![request.validation_report_id.clone()],
        expiry_epoch_seconds: WRITEBACK_CONTINUATION_EXPIRY_EPOCH_SECONDS,
        later_retrieval_status: ContinuationRetrievalStatus::Pending,
        production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
        created_at: gateway_record_stamp(
            "dagdb.continuation.created_at",
            &request.idempotency_key,
        )?,
    })
}

#[cfg(feature = "production-db")]
async fn trusted_gateway_epoch_seconds(pool: &sqlx::PgPool) -> Result<u64, DagDbHandlerError> {
    let seconds =
        sqlx::query_scalar::<_, i64>("SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::BIGINT")
            .fetch_one(pool)
            .await
            .map_err(|_| DagDbHandlerError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: "database_unavailable",
                class: "database",
                message: "DAG DB database operation failed".to_owned(),
                requires_council_review: false,
            })?;
    u64::try_from(seconds).map_err(|_| DagDbHandlerError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        error_code: "database_unavailable",
        class: "database",
        message: "DAG DB database operation failed".to_owned(),
        requires_council_review: false,
    })
}

#[cfg(feature = "production-db")]
fn writeback_target_memory_id(request: &DagDbWritebackRequest) -> Result<String, Box<Response>> {
    hash_hex(
        "dagdb.gateway.writeback.target_memory",
        &(&request.idempotency_key, &request.answer_hash),
    )
}

#[cfg(feature = "production-db")]
fn lifecycle_refs_from_memory_ids(
    tenant_id: &str,
    namespace: &str,
    memory_ids: Vec<String>,
) -> Vec<LifecycleMemoryRef> {
    memory_ids
        .into_iter()
        .map(|memory_id| LifecycleMemoryRef {
            tenant_id: tenant_id.to_owned(),
            project_id: gateway_project_id(namespace),
            memory_namespace: namespace.to_owned(),
            memory_id,
        })
        .collect()
}

#[cfg(feature = "production-db")]
fn gateway_project_id(namespace: &str) -> String {
    namespace.to_owned()
}

#[cfg(feature = "production-db")]
fn gateway_record_stamp(domain: &str, idempotency_key: &str) -> Result<String, Box<Response>> {
    hash_hex(domain, &idempotency_key)
}

#[cfg(feature = "production-db")]
fn sorted_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

#[cfg(feature = "production-db")]
fn d5_record_rejected_response(
    surface: &'static str,
    detail: impl std::fmt::Display,
) -> Box<Response> {
    Box::new(dagdb_error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "metadata_rejected",
        format!("DAG DB {surface} persistence request was rejected: {detail}"),
        true,
    ))
}

#[cfg(feature = "production-db")]
fn writeback_usage_event_metadata(request: &DagDbWritebackRequest) -> UsageEventMemoryMetadata {
    UsageEventMemoryMetadata {
        summary_text: request.summary_text.clone(),
        knowledge_class: request.knowledge_class.clone(),
    }
}

#[cfg(feature = "production-db")]
fn context_packet_response_from_persistent(
    request: &DagDbContextPacketRequest,
    persistent: &exo_dag_db_postgres::persistent_context::PersistentGraphContextPacket,
) -> Result<DagDbContextPacketResponse, Box<Response>> {
    let memory_refs: Vec<ContextPacketMemoryRef> = persistent
        .packet
        .selected_memory_refs
        .iter()
        .map(|memory_ref| ContextPacketMemoryRef {
            memory_id: memory_ref.memory_id.clone(),
            title: memory_ref.title.clone(),
            summary: memory_ref.summary.clone(),
            keywords: Vec::new(),
            latest_receipt_hash: persistent
                .selection
                .selected_memory_receipt_hashes
                .get(&memory_ref.memory_id)
                .cloned()
                .unwrap_or_else(|| Hash256::ZERO.to_string()),
        })
        .collect();
    let token_estimate = persistent.packet.packet_metrics.selected_token_estimate;
    let zero_receipt_hash = Hash256::ZERO.to_string();
    let missing_selected_receipt_hash = !memory_refs.is_empty()
        && memory_refs
            .iter()
            .any(|memory_ref| memory_ref.latest_receipt_hash == zero_receipt_hash);
    let (context_packet_mode, selection_warning) = if memory_refs.is_empty() {
        (
            Some("empty_selection".to_owned()),
            Some("no memory references selected for this task and database state".to_owned()),
        )
    } else if missing_selected_receipt_hash {
        (
            Some("database".to_owned()),
            Some(
                "selected memory reference receipt hash unavailable; validation failed closed"
                    .to_owned(),
            ),
        )
    } else {
        (Some("database".to_owned()), None)
    };
    let layered = context_packet_layered_fields(request, memory_refs.len(), false)?;
    Ok(DagDbContextPacketResponse {
        schema_version: exo_api::dagdb::DAGDB_CONTEXT_PACKET_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        idempotency_key: request.idempotency_key.clone(),
        context_packet_id: persistent.packet.packet_hash.clone(),
        route_id: request.route_id.clone(),
        receipt_hash: hash_hex(
            "dagdb.gateway.receipt",
            &(
                "dagdb.context_packet",
                persistent.packet.packet_hash.as_str(),
            ),
        )
        .unwrap_or_else(|_| Hash256::ZERO.to_string()),
        validation_status: if missing_selected_receipt_hash {
            ValidationStatus::Failed
        } else {
            ValidationStatus::Passed
        },
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Pending,
        memory_refs,
        packet_hash: persistent.packet.packet_hash.clone(),
        token_budget: request.token_budget,
        token_estimate,
        created_new: true,
        validation_report_id: None,
        council_decision_id: None,
        context_packet_mode,
        selection_warning,
        layered_mode: layered.layered_mode,
        selected_layers: layered.selected_layers,
        selected_layer_edges: layered.selected_layer_edges,
        layer_budget_report: layered.layer_budget_report,
        flat_fallback_used: layered.flat_fallback_used,
        layered_status: layered.layered_status,
        selected_graph_edges: persistent.packet.selected_graph_edges.clone(),
        citation_refs: persistent.packet.citation_refs.clone(),
        packet_metrics: Some(persistent.packet.packet_metrics.clone()),
        boundaries: Some(persistent.packet.boundaries.clone()),
        packet_markdown: Some(persistent.packet.markdown.clone()),
    })
}

#[cfg(feature = "production-db")]
fn writeback_response_from_persisted(
    request: &DagDbWritebackRequest,
    summary: &exo_dag_db_postgres::postgres::kg_context_selection_write::DbWriteSummary,
    created_new: bool,
) -> Result<DagDbWritebackResponse, Box<Response>> {
    let summary_meta = request
        .summary_text
        .as_deref()
        .map(|text| sanitize_metadata(MetadataField::Summary, text))
        .transpose()?;
    let keywords = request
        .keyword_texts
        .as_deref()
        .map(|texts| sanitize_keyword_texts(Some(texts)))
        .transpose()?;
    let layered = writeback_layered_fields(request)?;
    Ok(DagDbWritebackResponse {
        schema_version: exo_api::dagdb::DAGDB_WRITEBACK_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        idempotency_key: request.idempotency_key.clone(),
        memory_id: hash_hex(
            "dagdb.gateway.writeback.memory",
            &(summary.receipt_hash.as_str(), request.answer_hash.as_str()),
        )?,
        receipt_hash: summary.receipt_hash.clone(),
        validation_status: ValidationStatus::Passed,
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Pending,
        risk_class: RiskClass::R1,
        risk_bp: 1_000,
        created_new,
        validation_report_id: Some(request.validation_report_id.clone()),
        council_decision_id: None,
        summary: summary_meta,
        keywords,
        target_layer_path: layered.target_layer_path,
        target_layer_depth: layered.target_layer_depth,
        target_layer_reason: layered.target_layer_reason,
        created_child_layer_id: layered.created_child_layer_id,
        layered_writeback_status: layered.layered_writeback_status,
    })
}

#[cfg(feature = "production-db")]
struct DagDbHandlerError {
    status: StatusCode,
    error_code: &'static str,
    class: &'static str,
    message: String,
    requires_council_review: bool,
}

#[cfg(feature = "production-db")]
impl DagDbHandlerError {
    fn from_domain(error: DomainError) -> Self {
        let (status, error_code, message, requires_council_review) = match &error {
            DomainError::TenantScopeMismatch { .. } => (
                StatusCode::FORBIDDEN,
                "tenant_scope_mismatch",
                error.to_string(),
                false,
            ),
            DomainError::Metadata(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "metadata_rejected",
                error.to_string(),
                true,
            ),
            DomainError::HashMaterial { .. } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB database operation failed".into(),
                false,
            ),
            _ => (
                StatusCode::BAD_REQUEST,
                "invalid_request_shape",
                error.to_string(),
                false,
            ),
        };
        Self {
            status,
            error_code,
            class: "domain",
            message,
            requires_council_review,
        }
    }

    fn from_gatekeeper(error: GatekeeperError) -> Self {
        let failure = GatekeeperFailure::from_error(&error);
        Self {
            status: failure.status,
            error_code: failure.error_code,
            class: failure.class,
            message: failure.message.to_owned(),
            requires_council_review: failure.requires_council_review,
        }
    }

    fn from_response(response: Response) -> Self {
        let status = response.status();
        let (error_code, requires_council_review) = if status == StatusCode::UNPROCESSABLE_ENTITY {
            ("metadata_rejected", true)
        } else {
            ("invalid_request_shape", false)
        };
        Self {
            status,
            error_code,
            class: "response",
            message: "DAG DB writeback request was rejected".into(),
            requires_council_review,
        }
    }

    fn status(&self) -> StatusCode {
        self.status
    }

    fn error_code(&self) -> &str {
        self.error_code
    }

    fn class(&self) -> &str {
        self.class
    }

    fn into_response(self) -> Response {
        dagdb_error_response(
            self.status,
            self.error_code,
            &self.message,
            self.requires_council_review,
        )
    }
}

#[cfg(feature = "production-db")]
struct GatekeeperFailure {
    status: StatusCode,
    error_code: &'static str,
    class: &'static str,
    message: &'static str,
    requires_council_review: bool,
}

#[cfg(feature = "production-db")]
impl GatekeeperFailure {
    fn from_error(error: &GatekeeperError) -> Self {
        match error {
            GatekeeperError::InvariantViolation(detail) if detail.contains("ConsentRequired") => {
                Self {
                    status: StatusCode::FORBIDDEN,
                    error_code: "consent_denied",
                    class: "consent",
                    message: "DAG DB writeback consent was denied",
                    requires_council_review: true,
                }
            }
            GatekeeperError::InvariantViolation(detail)
                if detail.contains("ProvenanceVerifiable") =>
            {
                Self {
                    status: StatusCode::FORBIDDEN,
                    error_code: "provenance_denied",
                    class: "provenance",
                    message: "DAG DB writeback provenance could not be verified",
                    requires_council_review: true,
                }
            }
            GatekeeperError::McpTypedSignatureEncodingFailed { .. } => Self {
                status: StatusCode::FORBIDDEN,
                error_code: "provenance_denied",
                class: "provenance",
                message: "DAG DB writeback provenance could not be verified",
                requires_council_review: true,
            },
            // Metadata sanitization rejections surface inside hash-material
            // assembly; classify them before the database-failure arm so a
            // policy rejection is never reported as database unavailability.
            GatekeeperError::InvariantViolation(detail) if detail.contains("metadata rejected") => {
                Self {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    error_code: "metadata_rejected",
                    class: "metadata",
                    message: "DAG DB writeback metadata was rejected",
                    requires_council_review: true,
                }
            }
            GatekeeperError::InvariantViolation(detail)
                if is_gatekeeper_database_failure(detail) =>
            {
                Self {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    error_code: "database_unavailable",
                    class: "database",
                    message: "DAG DB database operation failed",
                    requires_council_review: false,
                }
            }
            GatekeeperError::CapabilityDenied(_) => Self {
                status: StatusCode::FORBIDDEN,
                error_code: "writeback_denied",
                class: "capability",
                message: "DAG DB writeback was denied",
                requires_council_review: true,
            },
            GatekeeperError::InvariantViolation(_) => Self {
                status: StatusCode::FORBIDDEN,
                error_code: "writeback_denied",
                class: "invariant",
                message: "DAG DB writeback was denied",
                requires_council_review: true,
            },
            GatekeeperError::Timeout(_) => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: "database_unavailable",
                class: "runtime",
                message: "DAG DB database operation failed",
                requires_council_review: false,
            },
            // The authority resolver could not establish consent/identity state
            // (pool absent or a resolver query failed). This is an availability
            // /misconfiguration fault, not a policy denial: surface 5xx so the
            // caller retries rather than treating it as a hard deny.
            GatekeeperError::AuthorityResolverUnavailable(_) => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: "database_unavailable",
                class: "resolver",
                message: "DAG DB authorization state is unavailable",
                requires_council_review: false,
            },
            GatekeeperError::KernelIntegrityFailure { .. }
            | GatekeeperError::CombinatorError(_)
            | GatekeeperError::HolonError(_)
            | GatekeeperError::McpViolation(_)
            | GatekeeperError::TeeError(_)
            | GatekeeperError::CheckpointError(_)
            | GatekeeperError::Core(_)
            | GatekeeperError::McpAuditChainBroken { .. }
            | GatekeeperError::McpAuditInvalidRecord { .. }
            | GatekeeperError::McpAuditHashEncodingFailed { .. } => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: "database_unavailable",
                class: "runtime",
                message: "DAG DB database operation failed",
                requires_council_review: false,
            },
        }
    }
}

#[cfg(feature = "production-db")]
fn is_gatekeeper_database_failure(detail: &str) -> bool {
    detail.contains("dagdb write blocked")
        && (detail.contains("hash_material_failed")
            || detail.contains("graph_context_selection_write_postgres")
            // PRD-D5: the four lifecycle/persistence surfaces emit this marker
            // when their backing transaction fails, so a DB outage on those
            // surfaces is classified 503 rather than a policy rejection.
            || detail.contains("surface_database_unavailable"))
}

fn verify_council_authority(
    headers: &HeaderMap,
    request: &DagDbCouncilDecisionRequest,
) -> Option<Response> {
    let Some(auth) = header_text(headers, header::AUTHORIZATION.as_str()) else {
        return Some(council_unauthenticated_response());
    };
    if !auth.starts_with("Bearer ") {
        return Some(council_unauthenticated_response());
    }

    let Some(tenant) = header_text(headers, TENANT_HEADER) else {
        return Some(council_tenant_scope_mismatch_response());
    };
    let Some(namespace) = header_text(headers, NAMESPACE_HEADER) else {
        return Some(council_tenant_scope_mismatch_response());
    };
    if tenant != request.tenant_id || namespace != request.namespace {
        return Some(council_tenant_scope_mismatch_response());
    }

    let required_scope = format!(
        "dagdb:council_decision:{}:{}",
        request.tenant_id, request.namespace
    );
    let has_scope = header_text(headers, AUTHORITY_SCOPE_HEADER)
        .map(|value| value.split([',', ' ']).any(|scope| scope == required_scope))
        .unwrap_or(false);
    if !has_scope {
        return Some(council_authority_required_response());
    }

    None
}

fn verify_dagdb_authority(
    headers: &HeaderMap,
    tenant_id: &str,
    namespace: &str,
    action: &str,
) -> Option<Response> {
    let denial = dagdb_authority_denial(headers, tenant_id, namespace, action)?;
    Some(match denial.error_code {
        "unauthenticated" => dagdb_unauthenticated_response(false),
        "tenant_scope_mismatch" => dagdb_tenant_scope_mismatch_response(false),
        "authority_denied" => dagdb_authority_required_response(false),
        _ => dagdb_authority_required_response(false),
    })
}

/// Identity established by binding a bearer token to a live DB-backed session.
///
/// `NoPool` means no pool is configured, so there is no persisted actor to bind
/// to. Callers must still fail closed before returning any claim-producing
/// response. `Authenticated(actor_did)` is the cryptographically-established
/// session actor that downstream authorization must bind request-supplied
/// identity and scope against.
#[cfg(feature = "production-db")]
enum DagDbSessionActor {
    /// No pool configured; no persisted session identity exists.
    NoPool,
    /// Bearer token resolved to a live session owned by this actor DID.
    Authenticated(String),
}

/// Bind the bearer token to a live DB-backed session whose user owns the
/// requested tenant. The header-shape gate alone accepts any `Bearer` string;
/// whenever a pool is configured (persisted tenant data is reachable), the
/// token must resolve to an unrevoked, unexpired `sessions` row whose user
/// profile matches the requested `tenant_id`. Session expiry is judged by the
/// trusted database clock, never caller time. Fails closed on lookup errors.
///
/// On success returns the session's authenticated `actor_did` so callers can
/// bind request-supplied identity (`requester_did`) and capability scope to the
/// principal that actually holds the session, instead of trusting
/// caller-controlled request fields or headers.
#[cfg(feature = "production-db")]
async fn verify_dagdb_session_authority(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    route_name: &'static str,
    tenant_id: &str,
) -> Result<DagDbSessionActor, Response> {
    let Some(pool) = ctx.pool.as_ref() else {
        return Ok(DagDbSessionActor::NoPool);
    };
    let token = header_text(headers, header::AUTHORIZATION.as_str())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty());
    let Some(token) = token else {
        warn!(
            route = route_name,
            tenant_id = %tenant_id,
            "DAG DB session binding failed closed: empty bearer token"
        );
        return Err(dagdb_unauthenticated_response(false));
    };
    let row = sqlx::query(
        "SELECT s.actor_did, u.tenant_id AS user_tenant_id \
         FROM sessions s \
         LEFT JOIN users u ON u.did = s.actor_did \
         WHERE s.token = $1 \
           AND s.revoked = FALSE \
           AND s.expires_at > FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT",
    )
    .bind(token)
    .fetch_optional(pool)
    .await;
    match row {
        Err(_) => {
            warn!(
                route = route_name,
                tenant_id = %tenant_id,
                "DAG DB session binding failed closed: session lookup unavailable"
            );
            Err(dagdb_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "session_lookup_unavailable",
                "DAG DB session validation is unavailable",
                false,
            ))
        }
        Ok(None) => {
            warn!(
                route = route_name,
                tenant_id = %tenant_id,
                "DAG DB session binding failed closed: no live session for bearer token"
            );
            Err(dagdb_unauthenticated_response(false))
        }
        Ok(Some(row)) => {
            let user_tenant: Option<String> = row.try_get("user_tenant_id").ok();
            if user_tenant.as_deref() != Some(tenant_id) {
                warn!(
                    route = route_name,
                    tenant_id = %tenant_id,
                    "DAG DB session binding failed closed: session user tenant mismatch"
                );
                return Err(dagdb_tenant_scope_mismatch_response(false));
            }
            let actor_did: Option<String> = row.try_get("actor_did").ok();
            match actor_did.filter(|did| !did.is_empty()) {
                Some(actor_did) => Ok(DagDbSessionActor::Authenticated(actor_did)),
                None => {
                    warn!(
                        route = route_name,
                        tenant_id = %tenant_id,
                        "DAG DB session binding failed closed: session row missing actor_did"
                    );
                    Err(dagdb_unauthenticated_response(false))
                }
            }
        }
    }
}

/// Reject when the request-supplied `requester_did` does not match the
/// cryptographically-established session actor. The session actor (not a
/// caller-controlled request field or `x-exo-authority-scope` header) is the
/// only trusted principal; a writeback-authorized agent must not be able to
/// self-assert another `requester_did` to reach the import/export adapters.
///
/// When no pool is configured there is no persisted session identity to bind
/// against. The caller must still reject before returning any claim-producing
/// response from the no-pool path.
#[cfg(feature = "production-db")]
fn bind_requester_to_session_actor(
    session_actor: &DagDbSessionActor,
    route_name: &'static str,
    tenant_id: &str,
    requester_did: &str,
) -> Result<(), Box<Response>> {
    let DagDbSessionActor::Authenticated(actor_did) = session_actor else {
        return Ok(());
    };
    // Shape validation precedes authorization: a malformed requester_did is a
    // bad request (400 invalid_request_shape from the handler's own
    // validation), not a forbidden actor mismatch. Skip the actor binding for
    // a requester_did that is not a well-formed DID and let the downstream
    // handler reject it; a well-formed DID that does not match the session
    // actor is still bound/denied below.
    if exo_core::Did::new(requester_did).is_err() {
        return Ok(());
    }
    if actor_did == requester_did {
        return Ok(());
    }
    warn!(
        route = route_name,
        tenant_id = %tenant_id,
        "DAG DB authority failed closed: requester_did does not match session actor"
    );
    Err(Box::new(dagdb_error_response(
        StatusCode::FORBIDDEN,
        "requester_actor_mismatch",
        "DAG DB requester_did does not match the authenticated session actor",
        false,
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DagDbAuthorityDenial {
    status: StatusCode,
    error_code: &'static str,
}

fn dagdb_authority_denial(
    headers: &HeaderMap,
    tenant_id: &str,
    namespace: &str,
    action: &str,
) -> Option<DagDbAuthorityDenial> {
    let Some(auth) = header_text(headers, header::AUTHORIZATION.as_str()) else {
        return Some(DagDbAuthorityDenial {
            status: StatusCode::UNAUTHORIZED,
            error_code: "unauthenticated",
        });
    };
    if !auth.starts_with("Bearer ") {
        return Some(DagDbAuthorityDenial {
            status: StatusCode::UNAUTHORIZED,
            error_code: "unauthenticated",
        });
    }

    let Some(tenant) = header_text(headers, TENANT_HEADER) else {
        return Some(DagDbAuthorityDenial {
            status: StatusCode::FORBIDDEN,
            error_code: "tenant_scope_mismatch",
        });
    };
    let Some(header_namespace) = header_text(headers, NAMESPACE_HEADER) else {
        return Some(DagDbAuthorityDenial {
            status: StatusCode::FORBIDDEN,
            error_code: "tenant_scope_mismatch",
        });
    };
    if tenant != tenant_id || header_namespace != namespace {
        return Some(DagDbAuthorityDenial {
            status: StatusCode::FORBIDDEN,
            error_code: "tenant_scope_mismatch",
        });
    }

    let required_scope = format!("{action}:{tenant_id}:{namespace}");
    let has_scope = header_text(headers, AUTHORITY_SCOPE_HEADER)
        .map(|value| value.split([',', ' ']).any(|scope| scope == required_scope))
        .unwrap_or(false);
    if !has_scope {
        return Some(DagDbAuthorityDenial {
            status: StatusCode::FORBIDDEN,
            error_code: "authority_denied",
        });
    }

    None
}

fn log_dagdb_authority_denial(
    route_name: &'static str,
    headers: &HeaderMap,
    tenant_id: &str,
    namespace: &str,
    action: &str,
) {
    if let Some(denial) = dagdb_authority_denial(headers, tenant_id, namespace, action) {
        warn!(
            route = route_name,
            status = denial.status.as_u16(),
            error_code = denial.error_code,
            tenant_id = %tenant_id,
            namespace = %namespace,
            "DAG DB authority check failed closed"
        );
    }
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn required_query_text(query: &QueryParams, name: &str) -> String {
    query.get(name).cloned().unwrap_or_default()
}

fn optional_query_bool(query: &QueryParams, name: &str) -> Option<bool> {
    query.get(name).map(|value| value == "true")
}

/// Session-binding gate shared by every DAG DB route. With `production-db`
/// this defers to [`verify_dagdb_session_authority`] so header-shape authority
/// alone can never reach a pool-backed route. No-pool routes still fail closed
/// before returning claim-producing responses.
#[cfg(feature = "production-db")]
async fn verify_dagdb_session_gate(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    route_name: &'static str,
    tenant_id: &str,
) -> Option<Response> {
    verify_dagdb_session_authority(ctx, headers, route_name, tenant_id)
        .await
        .err()
}

#[cfg(not(feature = "production-db"))]
async fn verify_dagdb_session_gate(
    _ctx: &DagDbRouteContext,
    _headers: &HeaderMap,
    _route_name: &'static str,
    _tenant_id: &str,
) -> Option<Response> {
    None
}

async fn dagdb_authorized_response(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    tenant_id: String,
    namespace: String,
    action: &str,
    route_name: &'static str,
    _authorized: impl FnOnce() -> Response,
) -> Response {
    if let Some(denied) = verify_dagdb_authority(headers, &tenant_id, &namespace, action) {
        return denied;
    }
    if let Some(denied) = verify_dagdb_session_gate(ctx, headers, route_name, &tenant_id).await {
        return denied;
    }
    if !dagdb_governed_pool_available(ctx) {
        warn!(
            route = route_name,
            status = 503,
            tenant_id = %tenant_id,
            namespace = %namespace,
            "DAG DB scaffold route rejected because no governed database pool is configured"
        );
        return dagdb_route_database_unavailable_response(route_name);
    }
    warn!(
        route = route_name,
        status = 503,
        tenant_id = %tenant_id,
        namespace = %namespace,
        "DAG DB scaffold route rejected because no governed route persistence is configured"
    );
    dagdb_route_database_unavailable_response(route_name)
}

async fn council_authorized_response(
    ctx: &DagDbRouteContext,
    headers: &HeaderMap,
    request: DagDbCouncilDecisionRequest,
    _authorized: impl FnOnce(DagDbCouncilDecisionRequest) -> Response,
) -> Response {
    if let Some(denied) = verify_council_authority(headers, &request) {
        return denied;
    }
    if let Some(denied) =
        verify_dagdb_session_gate(ctx, headers, "dagdb.council_decision", &request.tenant_id).await
    {
        return denied;
    }
    if !dagdb_governed_pool_available(ctx) {
        warn!(
            route = "dagdb.council_decision",
            status = 503,
            tenant_id = %request.tenant_id,
            namespace = %request.namespace,
            "DAG DB council decision rejected because no governed database pool is configured"
        );
        return dagdb_route_database_unavailable_response("dagdb.council_decision");
    }
    warn!(
        route = "dagdb.council_decision",
        status = 503,
        tenant_id = %request.tenant_id,
        namespace = %request.namespace,
        "DAG DB council decision rejected because no governed route persistence is configured"
    );
    dagdb_route_database_unavailable_response("dagdb.council_decision")
}

fn created_json_response<T: Serialize>(result: Result<T, Box<Response>>) -> Response {
    match result {
        Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
        Err(response) => *response,
    }
}

fn dagdb_unauthenticated_response(requires_council_review: bool) -> Response {
    dagdb_error_response(
        StatusCode::UNAUTHORIZED,
        "unauthenticated",
        "DAG DB route requires bearer authentication",
        requires_council_review,
    )
}

fn dagdb_tenant_scope_mismatch_response(requires_council_review: bool) -> Response {
    dagdb_error_response(
        StatusCode::FORBIDDEN,
        "tenant_scope_mismatch",
        "DAG DB tenant or namespace does not match the authorized scope",
        requires_council_review,
    )
}

fn dagdb_authority_required_response(requires_council_review: bool) -> Response {
    dagdb_error_response(
        StatusCode::FORBIDDEN,
        "authority_denied",
        "DAG DB route requires matching authority scope",
        requires_council_review,
    )
}

fn dagdb_invalid_json_request_response(
    route_name: &'static str,
    rejection: &JsonRejection,
) -> Response {
    warn!(
        route = route_name,
        status = 400,
        error_code = "invalid_request_shape",
        rejection_category = dagdb_json_rejection_category(rejection),
        "DAG DB JSON request rejected"
    );
    dagdb_error_response(
        StatusCode::BAD_REQUEST,
        "invalid_request_shape",
        "DAG DB request JSON body is malformed or contains unsupported fields",
        false,
    )
}

fn dagdb_json_rejection_category(rejection: &JsonRejection) -> &'static str {
    match rejection {
        JsonRejection::JsonDataError(_) => "json_data_error",
        JsonRejection::JsonSyntaxError(_) => "json_syntax_error",
        JsonRejection::MissingJsonContentType(_) => "missing_json_content_type",
        JsonRejection::BytesRejection(_) => "bytes_rejection",
        _ => "json_rejection",
    }
}

fn intake_response_from_request(
    request: DagDbIntakeRequest,
    route_name: &str,
) -> Result<DagDbIntakeResponse, Box<Response>> {
    let title = sanitize_metadata(MetadataField::Title, &request.title_text)?;
    let summary = sanitize_metadata(MetadataField::Summary, &request.summary_text)?;
    let keywords = sanitize_keyword_texts(request.keyword_texts.as_deref())?;
    let mut redacted_body = request_json(&request)?;
    replace_metadata(
        &mut redacted_body,
        "title_text",
        "title",
        request_json(&title)?,
    )?;
    replace_metadata(
        &mut redacted_body,
        "summary_text",
        "summary",
        request_json(&summary)?,
    )?;
    replace_metadata(
        &mut redacted_body,
        "keyword_texts",
        "keywords",
        request_json(&keywords)?,
    )?;
    let request_hash = request_hash(
        route_name,
        &request.tenant_id,
        &request.namespace,
        &redacted_body,
    )?;
    let memory_id = hash_hex(
        "dagdb.gateway.memory",
        &(&request_hash, &request.payload_hash),
    )?;
    Ok(DagDbIntakeResponse {
        schema_version: exo_api::dagdb::DAGDB_INTAKE_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        memory_id,
        receipt_hash: receipt_hash(route_name, request_hash)?,
        validation_status: ValidationStatus::Pending,
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Pending,
        risk_class: RiskClass::R1,
        risk_bp: 1_000,
        created_new: true,
        title,
        summary,
        keywords,
        validation_report_id: None,
        council_decision_id: None,
        duplicate_of_memory_id: None,
    })
}

fn route_response_from_request(
    request: DagDbRouteRequest,
    route_name: &str,
) -> Result<DagDbRouteResponse, Box<Response>> {
    let redacted_body = request_json(&request)?;
    let request_hash = request_hash(
        route_name,
        &request.tenant_id,
        &request.namespace,
        &redacted_body,
    )?;
    let selected_memory_ids = request.requested_memory_ids.clone().unwrap_or_default();
    Ok(DagDbRouteResponse {
        schema_version: exo_api::dagdb::DAGDB_ROUTE_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        route_id: hash_hex(
            "dagdb.gateway.route",
            &(&request_hash, &request.task_signature_hash),
        )?,
        receipt_hash: receipt_hash(route_name, request_hash)?,
        validation_status: ValidationStatus::Passed,
        council_status: CouncilReviewStatus::NotRequired,
        route_status: RouteStatus::Active,
        dag_finality_status: DagFinalityStatus::Pending,
        selected_memory_ids,
        route_score_bp: 0,
        token_budget: request.token_budget,
        token_estimate: 0,
        stale_at: "86400000:0".to_owned(),
        created_new: true,
        validation_report_id: None,
        council_decision_id: None,
        rejected_memory_ids: Some(Vec::new()),
    })
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
struct LayeredContextFields {
    layered_mode: Option<String>,
    selected_layers: Option<Vec<ContextPacketLayerRef>>,
    selected_layer_edges: Option<Vec<ContextPacketLayerEdgeRef>>,
    layer_budget_report: Option<ContextPacketLayerBudgetReport>,
    flat_fallback_used: Option<bool>,
    layered_status: Option<String>,
}

#[cfg(any(test, feature = "production-db"))]
struct LayeredWritebackFields {
    target_layer_path: Option<String>,
    target_layer_depth: Option<u32>,
    target_layer_reason: Option<String>,
    created_child_layer_id: Option<String>,
    layered_writeback_status: Option<String>,
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn no_layered_context_fields() -> LayeredContextFields {
    LayeredContextFields {
        layered_mode: None,
        selected_layers: None,
        selected_layer_edges: None,
        layer_budget_report: None,
        flat_fallback_used: None,
        layered_status: None,
    }
}

#[cfg(any(test, feature = "production-db"))]
fn no_layered_writeback_fields() -> LayeredWritebackFields {
    LayeredWritebackFields {
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
        created_child_layer_id: None,
        layered_writeback_status: None,
    }
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn invalid_layered_request(error_code: &'static str, message: &'static str) -> Box<Response> {
    Box::new(dagdb_error_response(
        StatusCode::BAD_REQUEST,
        error_code,
        message,
        false,
    ))
}

/// Validate the optional typed-knowledge class on a writeback, failing closed.
///
/// A writeback with no `knowledge_class` is plain usage-event telemetry and is
/// accepted unchanged. When a class is present it must be one of the closed
/// `DAGDB_KNOWLEDGE_CLASSES` set and the writeback must carry a non-empty
/// `summary_text` (knowledge writebacks must be content-bearing). The class
/// describes WHAT the memory is and never influences placement/organization.
fn validate_writeback_knowledge_class(
    request: &DagDbWritebackRequest,
) -> Result<(), Box<Response>> {
    let Some(class) = request.knowledge_class.as_deref() else {
        return Ok(());
    };
    if !DAGDB_KNOWLEDGE_CLASSES.contains(&class) {
        return Err(Box::new(dagdb_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_knowledge_class",
            "DAG DB knowledge_class must be one of decision, finding, fix, constraint, handoff",
            false,
        )));
    }
    let summary_is_content_bearing = request
        .summary_text
        .as_deref()
        .map(|text| !text.trim().is_empty())
        .unwrap_or(false);
    if !summary_is_content_bearing {
        return Err(Box::new(dagdb_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "knowledge_class_requires_summary",
            "DAG DB knowledge writebacks require a non-empty summary_text",
            false,
        )));
    }
    Ok(())
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn layered_mode_value(layered_mode: &Option<String>) -> Result<&str, Box<Response>> {
    let mode = layered_mode.as_deref().unwrap_or("off");
    if DAGDB_LAYERED_MODES.contains(&mode) {
        Ok(mode)
    } else {
        Err(invalid_layered_request(
            "invalid_layered_mode",
            "DAG DB layered_mode must be off, auto, or required",
        ))
    }
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn validate_max_layer_depth(max_layer_depth: Option<u32>) -> Result<u32, Box<Response>> {
    let depth = max_layer_depth.unwrap_or(2);
    if depth <= DAGDB_MAX_LAYER_DEPTH {
        Ok(depth)
    } else {
        Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB max_layer_depth exceeds gateway limit",
        ))
    }
}

/// Validate the depth-on-demand reserve (D1-S4), like the depth param.
///
/// Absent defaults to `0` (no reserve, byte-identical to the leftover-budget
/// path). A reserve above the runtime's half-budget bound fails closed so the
/// breadth pass always keeps a majority of the token budget.
#[cfg(feature = "production-db")]
fn validate_drilldown_reserve_bp(drilldown_reserve_bp: Option<u32>) -> Result<u32, Box<Response>> {
    let reserve = drilldown_reserve_bp.unwrap_or(0);
    if reserve <= DAGDB_MAX_DRILLDOWN_RESERVE_BP {
        Ok(reserve)
    } else {
        Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB drilldown_reserve_bp exceeds gateway limit",
        ))
    }
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn layer_path_depth(layer_path: &str) -> Result<u32, Box<Response>> {
    validate_layer_path(layer_path)?;
    u32::try_from(layer_path.split('/').count().saturating_sub(1)).map_err(|_| {
        invalid_layered_request(
            "invalid_layer_path_depth",
            "DAG DB layer_path depth exceeds gateway limit",
        )
    })
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn validate_layer_path(layer_path: &str) -> Result<(), Box<Response>> {
    if layer_path.is_empty()
        || layer_path.starts_with('/')
        || layer_path.ends_with('/')
        || layer_path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.chars().any(|character| {
                    character.is_control() || character.is_whitespace() || character == '\\'
                })
        })
    {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB target_layer_path must be a relative layer path",
        ));
    }
    Ok(())
}

#[cfg(any(test, feature = "production-db"))]
fn validate_layer_reason(reason: &str) -> Result<(), Box<Response>> {
    if reason.trim().is_empty()
        || reason.len() > 128
        || reason.chars().any(|character| character.is_control())
    {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB target_layer_reason must be a bounded non-empty reason",
        ));
    }
    Ok(())
}

#[cfg_attr(not(any(test, feature = "production-db")), allow(dead_code))]
fn context_packet_layered_fields(
    request: &DagDbContextPacketRequest,
    selected_ref_count: usize,
    verified_layer_evidence: bool,
) -> Result<LayeredContextFields, Box<Response>> {
    let explicit_layer_request = request.layered_mode.is_some()
        || request.max_layer_depth.is_some()
        || request.require_layer_evidence.unwrap_or(false);
    if !explicit_layer_request {
        return Ok(no_layered_context_fields());
    }

    let mode = layered_mode_value(&request.layered_mode)?;
    let max_layer_depth = validate_max_layer_depth(request.max_layer_depth)?;
    let require_layer_evidence =
        request.require_layer_evidence.unwrap_or(false) || mode == "required";
    if mode == "off" {
        if require_layer_evidence {
            return Err(invalid_layered_request(
                "invalid_request_shape",
                "DAG DB required layer evidence conflicts with layered_mode off",
            ));
        }
        return Ok(LayeredContextFields {
            layered_mode: Some("off".to_owned()),
            selected_layers: Some(Vec::new()),
            selected_layer_edges: Some(Vec::new()),
            layer_budget_report: Some(ContextPacketLayerBudgetReport {
                layered_mode: "off".to_owned(),
                max_layer_depth,
                required_layer_evidence: false,
                budget_status: "not_requested".to_owned(),
            }),
            flat_fallback_used: Some(false),
            layered_status: Some("off".to_owned()),
        });
    }

    if selected_ref_count == 0 || !verified_layer_evidence {
        if require_layer_evidence {
            return Err(invalid_layered_request(
                "required_layer_evidence_missing",
                "DAG DB required layered context requires selected layer evidence",
            ));
        }
        return Ok(LayeredContextFields {
            layered_mode: Some(mode.to_owned()),
            selected_layers: Some(Vec::new()),
            selected_layer_edges: Some(Vec::new()),
            layer_budget_report: Some(ContextPacketLayerBudgetReport {
                layered_mode: mode.to_owned(),
                max_layer_depth,
                required_layer_evidence: false,
                budget_status: "flat_fallback_no_layer_evidence".to_owned(),
            }),
            flat_fallback_used: Some(true),
            layered_status: Some("flat_fallback_no_layer_evidence".to_owned()),
        });
    }

    let root_path = "root".to_owned();
    let child_path = format!("root/context-packet/{}", request.route_id);
    let child_depth = layer_path_depth(&child_path)?;
    if child_depth > max_layer_depth {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB layered context exceeds max_layer_depth",
        ));
    }
    let root_layer_id = hash_hex("dagdb.gateway.layer", &root_path)?;
    let child_layer_id = hash_hex("dagdb.gateway.layer", &child_path)?;
    let layer_edge_id = hash_hex(
        "dagdb.gateway.layer_edge",
        &(&root_layer_id, &child_layer_id, "contains_subgraph"),
    )?;
    let selected_ref_count = u32::try_from(selected_ref_count).unwrap_or(u32::MAX);
    Ok(LayeredContextFields {
        layered_mode: Some(mode.to_owned()),
        selected_layers: Some(vec![
            ContextPacketLayerRef {
                layer_id: root_layer_id.clone(),
                layer_path: root_path,
                layer_depth: 0,
                layer_kind: "root".to_owned(),
                selected_ref_count,
            },
            ContextPacketLayerRef {
                layer_id: child_layer_id.clone(),
                layer_path: child_path,
                layer_depth: child_depth,
                layer_kind: "context_packet".to_owned(),
                selected_ref_count,
            },
        ]),
        selected_layer_edges: Some(vec![ContextPacketLayerEdgeRef {
            layer_edge_id,
            from_layer_id: root_layer_id,
            to_layer_id: child_layer_id,
            edge_kind: "contains_subgraph".to_owned(),
        }]),
        layer_budget_report: Some(ContextPacketLayerBudgetReport {
            layered_mode: mode.to_owned(),
            max_layer_depth,
            required_layer_evidence: require_layer_evidence,
            budget_status: "within_layer_budget".to_owned(),
        }),
        flat_fallback_used: Some(false),
        layered_status: Some("layered_evidence_selected".to_owned()),
    })
}

#[cfg(any(test, feature = "production-db"))]
fn writeback_layered_fields(
    request: &DagDbWritebackRequest,
) -> Result<LayeredWritebackFields, Box<Response>> {
    let explicit_layer_request = request.layered_mode.is_some()
        || request.target_layer_path.is_some()
        || request.target_layer_depth.is_some()
        || request.target_layer_reason.is_some();
    if !explicit_layer_request {
        return Ok(no_layered_writeback_fields());
    }

    let mode = layered_mode_value(&request.layered_mode)?;
    let target_present = request.target_layer_path.is_some()
        || request.target_layer_depth.is_some()
        || request.target_layer_reason.is_some();
    if mode == "off" {
        if target_present {
            return Err(invalid_layered_request(
                "invalid_request_shape",
                "DAG DB target layer fields require layered_mode auto or required",
            ));
        }
        return Ok(no_layered_writeback_fields());
    }

    let Some(target_layer_path) = request.target_layer_path.as_deref() else {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB layered writeback requires target_layer_path",
        ));
    };
    let Some(target_layer_depth) = request.target_layer_depth else {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB layered writeback requires target_layer_depth",
        ));
    };
    let Some(target_layer_reason) = request.target_layer_reason.as_deref() else {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB layered writeback requires target_layer_reason",
        ));
    };

    let calculated_depth = layer_path_depth(target_layer_path)?;
    if calculated_depth != target_layer_depth || target_layer_depth > DAGDB_MAX_LAYER_DEPTH {
        return Err(invalid_layered_request(
            "invalid_request_shape",
            "DAG DB target_layer_depth must match target_layer_path",
        ));
    }
    validate_layer_reason(target_layer_reason)?;

    Ok(LayeredWritebackFields {
        target_layer_path: Some(target_layer_path.to_owned()),
        target_layer_depth: Some(target_layer_depth),
        target_layer_reason: Some(target_layer_reason.to_owned()),
        created_child_layer_id: Some(hash_hex(
            "dagdb.gateway.writeback.layer",
            &target_layer_path,
        )?),
        layered_writeback_status: Some("layer_target_recorded".to_owned()),
    })
}

#[allow(dead_code)]
fn context_packet_response_from_request(
    request: DagDbContextPacketRequest,
    route_name: &str,
) -> Result<DagDbContextPacketResponse, Box<Response>> {
    let layered = context_packet_layered_fields(&request, 0, false)?;
    let redacted_body = request_json(&request)?;
    let request_hash = request_hash(
        route_name,
        &request.tenant_id,
        &request.namespace,
        &redacted_body,
    )?;
    Ok(DagDbContextPacketResponse {
        schema_version: exo_api::dagdb::DAGDB_CONTEXT_PACKET_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        context_packet_id: hash_hex(
            "dagdb.gateway.context_packet",
            &(&request_hash, &request.route_id),
        )?,
        route_id: request.route_id,
        receipt_hash: receipt_hash(route_name, request_hash)?,
        validation_status: ValidationStatus::Pending,
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Pending,
        memory_refs: Vec::new(),
        packet_hash: hash_hex("dagdb.gateway.packet", &request_hash)?,
        token_budget: request.token_budget,
        token_estimate: 0,
        created_new: true,
        validation_report_id: None,
        council_decision_id: None,
        context_packet_mode: Some("scaffold".to_owned()),
        selection_warning: Some(
            "gateway database mode unavailable; scaffold packet has no selected memory refs"
                .to_owned(),
        ),
        layered_mode: layered.layered_mode,
        selected_layers: layered.selected_layers,
        selected_layer_edges: layered.selected_layer_edges,
        layer_budget_report: layered.layer_budget_report,
        flat_fallback_used: layered.flat_fallback_used,
        layered_status: layered.layered_status,
        selected_graph_edges: Vec::new(),
        citation_refs: Vec::new(),
        packet_metrics: None,
        boundaries: None,
        packet_markdown: None,
    })
}

fn validate_response_from_request(
    request: DagDbValidateRequest,
    route_name: &str,
) -> Result<DagDbValidateResponse, Box<Response>> {
    let notes = sanitize_optional_metadata(
        MetadataField::ValidationNotes,
        request.validation_notes_text.as_deref(),
    )?;
    let mut redacted_body = request_json(&request)?;
    replace_metadata(
        &mut redacted_body,
        "validation_notes_text",
        "validation_notes",
        request_json(&notes)?,
    )?;
    let request_hash = request_hash(
        route_name,
        &request.tenant_id,
        &request.namespace,
        &redacted_body,
    )?;
    Ok(DagDbValidateResponse {
        schema_version: exo_api::dagdb::DAGDB_VALIDATE_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        validation_report_id: hash_hex(
            "dagdb.gateway.validation",
            &(&request_hash, &request.subject_id),
        )?,
        subject_kind: request.subject_kind,
        subject_id: request.subject_id,
        receipt_hash: receipt_hash(route_name, request_hash)?,
        validation_status: request.requested_status.unwrap_or(ValidationStatus::Passed),
        council_status: CouncilReviewStatus::NotRequired,
        risk_class: RiskClass::R1,
        risk_bp: 1_000,
        decision: ValidationDecision::Allow,
        created_new: true,
        council_decision_id: request.council_decision_id,
        contradictory_report_ids: Some(Vec::new()),
        notes,
    })
}

// Builds the synthetic writeback response shape. The runtime no-pool path now
// fails closed (503) instead of returning this scaffold, so this builder is
// retained only for the response-shape unit tests that assert its layered /
// metadata handling.
#[cfg(test)]
fn writeback_response_from_request(
    request: DagDbWritebackRequest,
    route_name: &str,
) -> Result<DagDbWritebackResponse, Box<Response>> {
    let layered = writeback_layered_fields(&request)?;
    let summary =
        sanitize_optional_metadata(MetadataField::Summary, request.summary_text.as_deref())?;
    let keywords = request
        .keyword_texts
        .as_deref()
        .map(|texts| sanitize_keyword_texts(Some(texts)))
        .transpose()?;
    let mut redacted_body = request_json(&request)?;
    replace_metadata(
        &mut redacted_body,
        "summary_text",
        "summary",
        request_json(&summary)?,
    )?;
    replace_metadata(
        &mut redacted_body,
        "keyword_texts",
        "keywords",
        request_json(&keywords)?,
    )?;
    let request_hash = request_hash(
        route_name,
        &request.tenant_id,
        &request.namespace,
        &redacted_body,
    )?;
    Ok(DagDbWritebackResponse {
        schema_version: exo_api::dagdb::DAGDB_WRITEBACK_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        memory_id: hash_hex(
            "dagdb.gateway.writeback",
            &(&request_hash, &request.answer_hash),
        )?,
        receipt_hash: receipt_hash(route_name, request_hash)?,
        validation_status: ValidationStatus::Pending,
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Pending,
        risk_class: RiskClass::R1,
        risk_bp: 1_000,
        created_new: true,
        validation_report_id: Some(request.validation_report_id),
        council_decision_id: None,
        summary,
        keywords,
        target_layer_path: layered.target_layer_path,
        target_layer_depth: layered.target_layer_depth,
        target_layer_reason: layered.target_layer_reason,
        created_child_layer_id: layered.created_child_layer_id,
        layered_writeback_status: layered.layered_writeback_status,
    })
}

fn validated_import_report_json(request: &DagDbImportRequest) -> Result<String, Box<Response>> {
    validate_runtime_request_basics(
        "dagdb.import",
        &request.tenant_id,
        &request.namespace,
        &request.idempotency_key,
        &request.db_set_version,
        &request.requester_did,
    )?;
    exo_dag_db_exchange::kg_import::hash_from_hex("source_hash", &request.source_hash).map_err(
        |_| {
            invalid_runtime_request(
                "dagdb.import",
                "import_source_hash_invalid",
                "DAG DB import source_hash must be 64-character hex",
            )
        },
    )?;
    let report_json = serde_json::to_string(&request.import_report).map_err(|_| {
        invalid_runtime_request(
            "dagdb.import",
            "import_report_encode_failed",
            "DAG DB import report could not be encoded as JSON",
        )
    })?;
    let report = KgImportDryRunReport::parse_json(&report_json).map_err(|_| {
        invalid_runtime_request(
            "dagdb.import",
            "import_report_invalid_or_unsafe",
            "DAG DB import report is invalid or unsafe",
        )
    })?;
    if report.tenant_id != request.tenant_id || report.namespace != request.namespace {
        return Err(invalid_runtime_request(
            "dagdb.import",
            "import_report_scope_mismatch",
            "DAG DB import report tenant or namespace does not match request scope",
        ));
    }
    // Persisted receipts derive their actor from the report, and every receipt
    // intent must match the report actor; bind that declared actor to the
    // signature-verified requester so an import cannot attribute receipts to a
    // principal that never signed the request.
    if report.actor_did != request.requester_did {
        return Err(invalid_runtime_request(
            "dagdb.import",
            "import_report_actor_mismatch",
            "DAG DB import report actor_did does not match the requester",
        ));
    }
    Ok(report_json)
}

#[cfg(feature = "production-db")]
fn import_route_request_hash(request: &DagDbImportRequest) -> Result<Hash256, Box<Response>> {
    let body = request_json(request)?;
    request_hash(
        IMPORT_ROUTE_IDEMPOTENCY_NAME,
        &request.tenant_id,
        &request.namespace,
        &body,
    )
}

#[cfg(feature = "production-db")]
fn export_route_request_hash(request: &DagDbExportRequest) -> Result<Hash256, Box<Response>> {
    let body = request_json(request)?;
    request_hash(
        EXPORT_ROUTE_IDEMPOTENCY_NAME,
        &request.tenant_id,
        &request.namespace,
        &body,
    )
}

#[cfg(feature = "production-db")]
enum GatewayIdempotencyDecision {
    Reserved,
    Replayed(CachedGatewayIdempotencyResponse),
}

#[cfg(feature = "production-db")]
struct CachedGatewayIdempotencyResponse {
    response: Response,
    authorization_payload_hash: Option<Hash256>,
}

#[cfg(feature = "production-db")]
async fn reserve_gateway_idempotency_key(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
    operation: &'static str,
) -> Result<GatewayIdempotencyDecision, Box<Response>> {
    let response_body = json!({
        "idempotency_status": RESERVED_IDEMPOTENCY_BODY_STATUS,
        "route_name": route_name,
    });
    let response_hash = gateway_idempotency_response_hash(
        &response_body,
        route_name,
        "reserve",
        operation,
        tenant_id,
        namespace,
        idempotency_key,
    )?;
    let mut tx = begin_gateway_idempotency_transaction(
        pool,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        "reserve",
        operation,
    )
    .await?;
    let reserved = insert_gateway_idempotency_reservation(
        &mut tx,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        request_hash,
        response_hash,
        &response_body,
        operation,
    )
    .await?
        || (reclaim_expired_gateway_idempotency_reservation(
            &mut tx,
            tenant_id,
            namespace,
            route_name,
            idempotency_key,
            request_hash,
            operation,
        )
        .await?
            && insert_gateway_idempotency_reservation(
                &mut tx,
                tenant_id,
                namespace,
                route_name,
                idempotency_key,
                request_hash,
                response_hash,
                &response_body,
                operation,
            )
            .await?);
    let decision = if reserved {
        GatewayIdempotencyDecision::Reserved
    } else {
        replay_gateway_idempotency_response_in_transaction(
            &mut tx,
            tenant_id,
            namespace,
            route_name,
            idempotency_key,
            request_hash,
            operation,
        )
        .await?
    };
    commit_gateway_idempotency_transaction(
        tx,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        "reserve",
        operation,
    )
    .await?;
    Ok(decision)
}

#[cfg(feature = "production-db")]
async fn begin_gateway_idempotency_transaction<'a>(
    pool: &'a sqlx::PgPool,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    phase: &'static str,
    operation: &'static str,
) -> Result<Transaction<'a, Postgres>, Box<Response>> {
    begin_tenant_transaction(pool, tenant_id)
        .await
        .map_err(|_| {
            idempotency_unavailable_response_logged(
                route_name,
                phase,
                operation,
                tenant_id,
                namespace,
                idempotency_key,
            )
        })
}

#[cfg(feature = "production-db")]
async fn commit_gateway_idempotency_transaction(
    tx: Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    phase: &'static str,
    operation: &'static str,
) -> Result<(), Box<Response>> {
    tx.commit().await.map_err(|_| {
        idempotency_unavailable_response_logged(
            route_name,
            phase,
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        )
    })
}

/// Insert a gateway idempotency reservation stamped by the trusted database
/// clock (the gateway's approved DB-backed time source; AGENTS.md forbids
/// `SystemTime` in production code).
#[cfg(feature = "production-db")]
#[allow(clippy::too_many_arguments)]
async fn insert_gateway_idempotency_reservation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
    response_hash: Hash256,
    response_body: &Value,
    operation: &'static str,
) -> Result<bool, Box<Response>> {
    let inserted = sqlx::query(
        "INSERT INTO dagdb_idempotency_keys \
         (tenant_id, namespace, route_name, idempotency_key, request_hash, response_hash, response_body, \
          status_code, cached_failure, created_at_physical_ms, created_at_logical, \
          expires_at_physical_ms, expires_at_logical) \
         SELECT $1, $2, $3, $4, $5, $6, $7, 202, false, trusted_now.now_ms, 0, \
                trusted_now.now_ms + $8, 0 \
         FROM (SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT AS now_ms) \
              AS trusted_now \
         ON CONFLICT (tenant_id, namespace, route_name, idempotency_key) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .bind(idempotency_key)
    .bind(request_hash.as_bytes().to_vec())
    .bind(response_hash.as_bytes().to_vec())
    .bind(response_body.clone())
    .bind(GATEWAY_IDEMPOTENCY_RESERVATION_TTL_MS)
    .execute(&mut **tx)
    .await
    .map_err(|_| {
        idempotency_unavailable_response_logged(
            route_name,
            "reserve",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        )
    })?;
    Ok(inserted.rows_affected() == 1)
}

/// Delete an expired reserved row for the same request hash so the retry can
/// re-reserve; completed responses and foreign-request reservations are never
/// touched.
#[cfg(feature = "production-db")]
async fn reclaim_expired_gateway_idempotency_reservation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
    operation: &'static str,
) -> Result<bool, Box<Response>> {
    let reclaimed = sqlx::query(
        "DELETE FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4 \
           AND request_hash = $5 \
           AND response_body->>'idempotency_status' = $6 \
           AND expires_at_physical_ms < FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .bind(idempotency_key)
    .bind(request_hash.as_bytes().to_vec())
    .bind(RESERVED_IDEMPOTENCY_BODY_STATUS)
    .execute(&mut **tx)
    .await
    .map_err(|_| {
        idempotency_unavailable_response_logged(
            route_name,
            "reserve",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        )
    })?;
    Ok(reclaimed.rows_affected() == 1)
}

#[cfg(feature = "production-db")]
#[allow(dead_code)]
async fn replay_gateway_idempotency_response(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
    operation: &'static str,
) -> Result<GatewayIdempotencyDecision, Box<Response>> {
    let mut tx = begin_gateway_idempotency_transaction(
        pool,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        "replay",
        operation,
    )
    .await?;
    let decision = replay_gateway_idempotency_response_in_transaction(
        &mut tx,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        request_hash,
        operation,
    )
    .await?;
    commit_gateway_idempotency_transaction(
        tx,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        "replay",
        operation,
    )
    .await?;
    Ok(decision)
}

#[cfg(feature = "production-db")]
async fn replay_gateway_idempotency_response_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
    operation: &'static str,
) -> Result<GatewayIdempotencyDecision, Box<Response>> {
    let unavailable = || {
        idempotency_unavailable_response_logged(
            route_name,
            "replay",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        )
    };
    let row = sqlx::query(
        "SELECT request_hash, response_body, status_code FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .bind(idempotency_key)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| unavailable())?;

    let existing_hash = hash_from_idempotency_row(
        row.try_get("request_hash").map_err(|_| unavailable())?,
        operation,
    )
    .inspect_err(|_| {
        log_idempotency_failure(
            route_name,
            "replay",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
            StatusCode::SERVICE_UNAVAILABLE,
        );
    })?;
    if existing_hash != request_hash {
        return Err(idempotency_conflict_response(operation));
    }

    let mut body: Value = row.try_get("response_body").map_err(|_| unavailable())?;
    if body.get("idempotency_status").and_then(Value::as_str)
        == Some(RESERVED_IDEMPOTENCY_BODY_STATUS)
    {
        return Err(idempotency_in_progress_response(operation));
    }
    let authorization_payload_hash = gateway_authorization_payload_hash_from_cached_body(
        route_name,
        operation,
        tenant_id,
        namespace,
        idempotency_key,
        &mut body,
    )?;

    let status = status_from_idempotency_row(
        row.try_get("status_code").map_err(|_| unavailable())?,
        operation,
    )
    .inspect_err(|_| {
        log_idempotency_failure(
            route_name,
            "replay",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
            StatusCode::SERVICE_UNAVAILABLE,
        );
    })?;
    Ok(GatewayIdempotencyDecision::Replayed(
        CachedGatewayIdempotencyResponse {
            response: (status, Json(body)).into_response(),
            authorization_payload_hash,
        },
    ))
}

#[cfg(feature = "production-db")]
fn gateway_authorization_payload_hash_from_cached_body(
    route_name: &str,
    operation: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
    body: &mut Value,
) -> Result<Option<Hash256>, Box<Response>> {
    let Some(value) = body.get(GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD) else {
        return Ok(None);
    };
    let Some(hash_hex) = value.as_str() else {
        log_idempotency_failure(
            route_name,
            "replay",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
            StatusCode::SERVICE_UNAVAILABLE,
        );
        return Err(idempotency_unavailable_response(operation));
    };
    let authorization_payload_hash = exo_dag_db_exchange::kg_import::hash_from_hex(
        GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD,
        hash_hex,
    )
    .map_err(|_| {
        log_idempotency_failure(
            route_name,
            "replay",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
            StatusCode::SERVICE_UNAVAILABLE,
        );
        idempotency_unavailable_response(operation)
    })?;
    if let Value::Object(fields) = body {
        fields.remove(GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD);
    }
    Ok(Some(authorization_payload_hash))
}

#[cfg(feature = "production-db")]
fn insert_gateway_authorization_payload_hash(
    response_body: &mut Value,
    authorization_payload_hash: Option<Hash256>,
    route_name: &str,
    operation: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
) -> Result<(), Box<Response>> {
    let Some(authorization_payload_hash) = authorization_payload_hash else {
        return Ok(());
    };
    let Value::Object(fields) = response_body else {
        log_idempotency_failure(
            route_name,
            "store",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
            StatusCode::SERVICE_UNAVAILABLE,
        );
        return Err(idempotency_unavailable_response(operation));
    };
    fields.insert(
        GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD.to_owned(),
        json!(authorization_payload_hash.to_string()),
    );
    Ok(())
}

#[cfg(feature = "production-db")]
#[allow(clippy::too_many_arguments)]
async fn store_gateway_idempotency_response(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
    status: StatusCode,
    response_body: Result<Value, Box<Response>>,
    authorization_payload_hash: Option<Hash256>,
    operation: &'static str,
) -> Result<(), Box<Response>> {
    let mut response_body = response_body.inspect_err(|_| {
        log_idempotency_failure(
            route_name,
            "store",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
            StatusCode::SERVICE_UNAVAILABLE,
        );
    })?;
    insert_gateway_authorization_payload_hash(
        &mut response_body,
        authorization_payload_hash,
        route_name,
        operation,
        tenant_id,
        namespace,
        idempotency_key,
    )?;
    let response_hash = gateway_idempotency_response_hash(
        &response_body,
        route_name,
        "store",
        operation,
        tenant_id,
        namespace,
        idempotency_key,
    )?;
    let mut tx = begin_gateway_idempotency_transaction(
        pool,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        "store",
        operation,
    )
    .await?;
    let updated = sqlx::query(
        "UPDATE dagdb_idempotency_keys \
         SET response_hash = $5, response_body = $6, status_code = $7, cached_failure = false \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4 \
           AND request_hash = $8",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .bind(idempotency_key)
    .bind(response_hash.as_bytes().to_vec())
    .bind(response_body)
    .bind(i32::from(status.as_u16()))
    .bind(request_hash.as_bytes().to_vec())
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        idempotency_unavailable_response_logged(
            route_name,
            "store",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        )
    })?;

    if updated.rows_affected() == 1 {
        commit_gateway_idempotency_transaction(
            tx,
            tenant_id,
            namespace,
            route_name,
            idempotency_key,
            "store",
            operation,
        )
        .await
    } else {
        Err(idempotency_unavailable_response_logged(
            route_name,
            "store",
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        ))
    }
}

#[cfg(feature = "production-db")]
async fn delete_gateway_idempotency_reservation(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
) -> Result<u64, sqlx::Error> {
    let mut tx = begin_tenant_transaction(pool, tenant_id).await?;
    let rows_affected = delete_gateway_idempotency_reservation_in_transaction(
        &mut tx,
        tenant_id,
        namespace,
        route_name,
        idempotency_key,
        request_hash,
    )
    .await?;
    tx.commit().await?;
    Ok(rows_affected)
}

#[cfg(feature = "production-db")]
async fn delete_gateway_idempotency_reservation_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
    idempotency_key: &str,
    request_hash: Hash256,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "DELETE FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4 \
           AND request_hash = $5 \
           AND response_body->>'idempotency_status' = $6",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .bind(idempotency_key)
    .bind(request_hash.as_bytes().to_vec())
    .bind(RESERVED_IDEMPOTENCY_BODY_STATUS)
    .execute(&mut **tx)
    .await
    .map(|result| result.rows_affected())
}

#[cfg(feature = "production-db")]
async fn cleanup_gateway_idempotency_reservation(
    pool: &sqlx::PgPool,
    request: &DagDbImportRequest,
    request_hash: Hash256,
) -> Result<(), Box<Response>> {
    match delete_gateway_idempotency_reservation(
        pool,
        &request.tenant_id,
        &request.namespace,
        IMPORT_ROUTE_IDEMPOTENCY_NAME,
        &request.idempotency_key,
        request_hash,
    )
    .await
    {
        Ok(rows_affected) if idempotency_reservation_cleanup_removed(rows_affected) => Ok(()),
        Ok(rows_affected) => {
            log_idempotency_cleanup_row_mismatch(
                "dagdb.import",
                "import",
                &request.tenant_id,
                &request.namespace,
                &request.idempotency_key,
                rows_affected,
            );
            Err(idempotency_unavailable_response("import"))
        }
        Err(_) => Err(idempotency_unavailable_response_logged(
            "dagdb.import",
            "cleanup",
            "import",
            &request.tenant_id,
            &request.namespace,
            &request.idempotency_key,
        )),
    }
}

#[cfg(feature = "production-db")]
async fn cleanup_export_idempotency_reservation(
    pool: &sqlx::PgPool,
    request: &DagDbExportRequest,
    request_hash: Hash256,
) -> Result<(), Box<Response>> {
    match delete_gateway_idempotency_reservation(
        pool,
        &request.tenant_id,
        &request.namespace,
        EXPORT_ROUTE_IDEMPOTENCY_NAME,
        &request.idempotency_key,
        request_hash,
    )
    .await
    {
        Ok(rows_affected) if idempotency_reservation_cleanup_removed(rows_affected) => Ok(()),
        Ok(rows_affected) => {
            log_idempotency_cleanup_row_mismatch(
                "dagdb.export",
                "export",
                &request.tenant_id,
                &request.namespace,
                &request.idempotency_key,
                rows_affected,
            );
            Err(idempotency_unavailable_response("export"))
        }
        Err(_) => Err(idempotency_unavailable_response_logged(
            "dagdb.export",
            "cleanup",
            "export",
            &request.tenant_id,
            &request.namespace,
            &request.idempotency_key,
        )),
    }
}

#[cfg(feature = "production-db")]
fn idempotency_unavailable_response_logged(
    route_name: &str,
    stage: &'static str,
    operation: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
) -> Box<Response> {
    log_idempotency_failure(
        route_name,
        stage,
        operation,
        tenant_id,
        namespace,
        idempotency_key,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    idempotency_unavailable_response(operation)
}

#[cfg(feature = "production-db")]
fn log_idempotency_failure(
    route_name: &str,
    stage: &'static str,
    operation: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
    status: StatusCode,
) {
    warn!(
        route = route_name,
        status = status.as_u16(),
        stage,
        operation,
        tenant_id = %tenant_id,
        namespace = %namespace,
        idempotency_ref = %idempotency_ref(idempotency_key),
        "DAG DB idempotency guard failed closed"
    );
}

#[cfg(feature = "production-db")]
fn log_idempotency_cleanup_row_mismatch(
    route_name: &str,
    operation: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
    rows_affected: u64,
) {
    warn!(
        route = route_name,
        status = StatusCode::SERVICE_UNAVAILABLE.as_u16(),
        stage = "cleanup",
        operation,
        tenant_id = %tenant_id,
        namespace = %namespace,
        idempotency_ref = %idempotency_ref(idempotency_key),
        rows_affected,
        "DAG DB idempotency reservation cleanup removed unexpected row count"
    );
}

#[cfg(feature = "production-db")]
fn idempotency_reservation_cleanup_removed(rows_affected: u64) -> bool {
    rows_affected == 1
}

#[cfg(feature = "production-db")]
fn idempotency_ref(idempotency_key: &str) -> String {
    Hash256::digest(idempotency_key.as_bytes()).to_string()
}

#[cfg(feature = "production-db")]
fn hash_from_idempotency_row(
    bytes: Vec<u8>,
    operation: &'static str,
) -> Result<Hash256, Box<Response>> {
    let Ok(bytes) = <[u8; 32]>::try_from(bytes) else {
        return Err(idempotency_unavailable_response(operation));
    };
    Ok(Hash256::from_bytes(bytes))
}

#[cfg(feature = "production-db")]
fn status_from_idempotency_row(
    status_code: i32,
    operation: &'static str,
) -> Result<StatusCode, Box<Response>> {
    let Ok(status_code) = u16::try_from(status_code) else {
        return Err(idempotency_unavailable_response(operation));
    };
    StatusCode::from_u16(status_code).map_err(|_| idempotency_unavailable_response(operation))
}

#[cfg(feature = "production-db")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdapterFailure {
    status: StatusCode,
    error_code: &'static str,
    class: &'static str,
    message: &'static str,
}

#[cfg(feature = "production-db")]
fn import_adapter_failure(error: &KgImportPersistenceError) -> AdapterFailure {
    match error {
        KgImportPersistenceError::Report(report_error) => match report_error {
            KgImportError::InvalidJson { .. }
            | KgImportError::InvalidReport { .. }
            | KgImportError::InvalidHash { .. } => AdapterFailure {
                status: StatusCode::BAD_REQUEST,
                error_code: "import_rejected",
                class: "validation",
                message: "DAG DB import request was rejected by the import adapter",
            },
            KgImportError::Hash { .. } => import_runtime_adapter_failure("runtime"),
        },
        KgImportPersistenceError::Conflict { .. } => AdapterFailure {
            status: StatusCode::CONFLICT,
            error_code: "import_rejected",
            class: "conflict",
            message: "DAG DB import request conflicted with existing adapter state",
        },
        KgImportPersistenceError::UnsupportedSection { .. } => AdapterFailure {
            status: StatusCode::BAD_REQUEST,
            error_code: "import_rejected",
            class: "unsupported",
            message: "DAG DB import request used an unsupported adapter section",
        },
        // PRD-D2 (D2-S3): the import-time aggregate distillation rejected a layer
        // member (e.g. forbidden material), so the request content is rejected —
        // a validation failure, not a runtime fault.
        KgImportPersistenceError::LayerAggregate { .. } => AdapterFailure {
            status: StatusCode::BAD_REQUEST,
            error_code: "import_rejected",
            class: "validation",
            message: "DAG DB import request was rejected by the layer aggregate distiller",
        },
        KgImportPersistenceError::Postgres { .. } => import_runtime_adapter_failure("postgres"),
        KgImportPersistenceError::MissingDatabaseUrl { .. }
        | KgImportPersistenceError::Init { .. }
        | KgImportPersistenceError::Json { .. }
        | KgImportPersistenceError::TimestampOutOfRange
        | KgImportPersistenceError::CountOutOfRange
        | KgImportPersistenceError::Hash { .. } => import_runtime_adapter_failure("runtime"),
    }
}

#[cfg(feature = "production-db")]
fn import_runtime_adapter_failure(class: &'static str) -> AdapterFailure {
    AdapterFailure {
        status: StatusCode::SERVICE_UNAVAILABLE,
        error_code: "database_unavailable",
        class,
        message: "DAG DB import adapter is temporarily unavailable",
    }
}

#[cfg(feature = "production-db")]
fn export_adapter_failure(error: &KgExportError) -> AdapterFailure {
    match error {
        KgExportError::InvalidScope { .. } | KgExportError::ForbiddenMaterial { .. } => {
            AdapterFailure {
                status: StatusCode::BAD_REQUEST,
                error_code: "export_rejected",
                class: "validation",
                message: "DAG DB export request was rejected by the export adapter",
            }
        }
        KgExportError::ImportHash(import_error) => match import_error {
            KgImportError::InvalidJson { .. }
            | KgImportError::InvalidReport { .. }
            | KgImportError::InvalidHash { .. } => AdapterFailure {
                status: StatusCode::BAD_REQUEST,
                error_code: "export_rejected",
                class: "validation",
                message: "DAG DB export request was rejected by the export adapter",
            },
            KgImportError::Hash { .. } => export_runtime_adapter_failure("runtime"),
        },
        KgExportError::Conflict { .. } | KgExportError::IncompatibleCachedResponse { .. } => {
            AdapterFailure {
                status: StatusCode::CONFLICT,
                error_code: "export_rejected",
                class: "conflict",
                message: "DAG DB export request conflicted with existing adapter state",
            }
        }
        KgExportError::UnsupportedPersistenceTarget { .. } => AdapterFailure {
            status: StatusCode::BAD_REQUEST,
            error_code: "export_rejected",
            class: "unsupported",
            message: "DAG DB export request used an unsupported adapter target",
        },
        KgExportError::Postgres { .. } => export_runtime_adapter_failure("postgres"),
        KgExportError::MissingDatabaseUrl { .. }
        | KgExportError::TimestampOutOfRange
        | KgExportError::CountOutOfRange
        | KgExportError::Hash { .. }
        | KgExportError::Io { .. }
        | KgExportError::Json { .. }
        | KgExportError::Init { .. } => export_runtime_adapter_failure("runtime"),
    }
}

#[cfg(feature = "production-db")]
fn export_runtime_adapter_failure(class: &'static str) -> AdapterFailure {
    AdapterFailure {
        status: StatusCode::SERVICE_UNAVAILABLE,
        error_code: "database_unavailable",
        class,
        message: "DAG DB export adapter is temporarily unavailable",
    }
}

#[cfg(feature = "production-db")]
fn dagdb_import_adapter_error_response(
    request: &DagDbImportRequest,
    error: &KgImportPersistenceError,
) -> Response {
    let failure = import_adapter_failure(error);
    warn!(
        route = "dagdb.import",
        status = failure.status.as_u16(),
        error_code = failure.error_code,
        adapter_error_class = failure.class,
        tenant_id = %request.tenant_id,
        namespace = %request.namespace,
        "DAG DB import adapter failed closed"
    );
    dagdb_error_response(failure.status, failure.error_code, failure.message, false)
}

#[cfg(feature = "production-db")]
fn dagdb_export_adapter_error_response(
    request: &DagDbExportRequest,
    error: &KgExportError,
) -> Response {
    let failure = export_adapter_failure(error);
    warn!(
        route = "dagdb.export",
        status = failure.status.as_u16(),
        error_code = failure.error_code,
        adapter_error_class = failure.class,
        tenant_id = %request.tenant_id,
        namespace = %request.namespace,
        "DAG DB export adapter failed closed"
    );
    dagdb_error_response(failure.status, failure.error_code, failure.message, false)
}

#[cfg(feature = "production-db")]
fn idempotency_conflict_response(operation: &'static str) -> Box<Response> {
    Box::new(dagdb_error_response(
        StatusCode::CONFLICT,
        "idempotency_key_conflict",
        idempotency_message(
            operation,
            "idempotency key was already used with a different request body",
        ),
        false,
    ))
}

#[cfg(feature = "production-db")]
fn idempotency_in_progress_response(operation: &'static str) -> Box<Response> {
    Box::new(dagdb_error_response(
        StatusCode::CONFLICT,
        "idempotency_key_in_progress",
        idempotency_message(operation, "idempotency key is currently being processed"),
        false,
    ))
}

#[cfg(feature = "production-db")]
fn idempotency_unavailable_response(operation: &'static str) -> Box<Response> {
    Box::new(dagdb_error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        idempotency_message(operation, "idempotency guard could not be checked"),
        false,
    ))
}

#[cfg(feature = "production-db")]
fn import_idempotency_unavailable_response() -> Box<Response> {
    idempotency_unavailable_response("import")
}

#[cfg(feature = "production-db")]
fn export_idempotency_unavailable_response() -> Box<Response> {
    idempotency_unavailable_response("export")
}

fn dagdb_runtime_database_unavailable_response(operation: &'static str) -> Response {
    dagdb_error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        idempotency_message(operation, "requires a configured production database"),
        false,
    )
}

fn dagdb_governed_pool_available(ctx: &DagDbRouteContext) -> bool {
    #[cfg(feature = "production-db")]
    {
        ctx.pool.is_some()
    }
    #[cfg(not(feature = "production-db"))]
    {
        let _ = ctx;
        false
    }
}

fn dagdb_route_database_unavailable_response(route_name: &'static str) -> Response {
    let operation = route_name.strip_prefix("dagdb.").unwrap_or(route_name);
    dagdb_error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        format!("DAG DB {operation} requires a configured production database"),
        false,
    )
}

fn idempotency_message(operation: &'static str, detail: &str) -> String {
    format!("DAG DB {operation} {detail}")
}

#[cfg(feature = "production-db")]
fn import_response_from_summary(
    request: DagDbImportRequest,
    summary: exo_dag_db_exchange::kg_import::KgImportPersistedSummary,
    import_status: &str,
) -> Result<DagDbImportResponse, Box<Response>> {
    let imported_record_count = summary
        .inserted_memory_count
        .saturating_add(summary.inserted_catalog_count)
        .saturating_add(summary.inserted_graph_node_count)
        .saturating_add(summary.inserted_graph_edge_count)
        .saturating_add(summary.inserted_layer_count)
        .saturating_add(summary.inserted_layer_membership_count)
        .saturating_add(summary.inserted_layer_edge_count)
        .saturating_add(summary.inserted_validation_report_count)
        .saturating_add(summary.inserted_placement_decision_count)
        .saturating_add(summary.inserted_placement_trace_count)
        .saturating_add(summary.inserted_receipt_count);
    Ok(DagDbImportResponse {
        schema_version: exo_api::dagdb::DAGDB_IMPORT_RESPONSE_SCHEMA_VERSION.to_owned(),
        operation_id: runtime_operation_id(
            "dagdb.gateway.import.operation",
            &request.tenant_id,
            &request.namespace,
            &request.idempotency_key,
            &request.db_set_version,
            &request.source_hash,
        )?,
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        db_set_version: request.db_set_version,
        import_status: import_status.to_owned(),
        import_receipt_id: Some(summary.idempotency_key),
        source_hash: request.source_hash,
        imported_record_count,
        receipt_path: None,
        non_claims: runtime_non_claims(),
    })
}

fn export_scope_from_request(request: &DagDbExportRequest) -> Result<KgExportScope, Box<Response>> {
    validate_runtime_request_basics(
        "dagdb.export",
        &request.tenant_id,
        &request.namespace,
        &request.idempotency_key,
        &request.db_set_version,
        &request.requester_did,
    )?;
    for memory_id in &request.included_memory_ids {
        exo_dag_db_exchange::kg_import::hash_from_hex("included_memory_ids", memory_id).map_err(
            |_| {
                invalid_runtime_request(
                    "dagdb.export",
                    "export_included_memory_id_invalid",
                    "DAG DB export included_memory_ids entries must be 64-character hex hashes",
                )
            },
        )?;
    }
    let scope = KgExportScope {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        included_memory_ids: request.included_memory_ids.clone(),
        included_graph_styles: request.included_graph_styles.clone(),
        included_writeback_idempotency_keys: request.included_writeback_idempotency_keys.clone(),
        source_commit_or_repo_ref: request.source_commit_or_repo_ref.clone(),
        include_preview_context: request.include_preview_context,
    };
    scope.validate().map_err(|_| {
        invalid_runtime_request(
            "dagdb.export",
            "export_scope_invalid_or_unsafe",
            "DAG DB export scope is invalid or unsafe",
        )
    })?;
    Ok(scope)
}

#[cfg(feature = "production-db")]
fn export_response_from_portable(
    request: DagDbExportRequest,
    export: exo_dag_db_exchange::kg_export::KgPortableExport,
) -> Result<DagDbExportResponse, Box<Response>> {
    let exported_record_count = export_record_count(&export);
    Ok(DagDbExportResponse {
        schema_version: exo_api::dagdb::DAGDB_EXPORT_RESPONSE_SCHEMA_VERSION.to_owned(),
        operation_id: runtime_operation_id(
            "dagdb.gateway.export.operation",
            &request.tenant_id,
            &request.namespace,
            &request.idempotency_key,
            &request.db_set_version,
            export.hashes.whole_export_hash.as_str(),
        )?,
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        db_set_version: request.db_set_version,
        export_status: "built".to_owned(),
        export_artifact_id: Some(export.export_id),
        export_hash: Some(export.hashes.whole_export_hash),
        exported_record_count,
        report_path: None,
        non_claims: runtime_non_claims(),
    })
}

fn validate_runtime_request_basics(
    route_name: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
    db_set_version: &str,
    requester_did: &str,
) -> Result<(), Box<Response>> {
    for (field, value) in [
        ("tenant_id", tenant_id),
        ("namespace", namespace),
        ("idempotency_key", idempotency_key),
        ("db_set_version", db_set_version),
        ("requester_did", requester_did),
    ] {
        if value.trim().is_empty() {
            return Err(invalid_runtime_request(
                route_name,
                "runtime_required_field_empty",
                "DAG DB runtime request fields must not be empty",
            ));
        }
        exo_dag_db_exchange::kg_export::reject_forbidden_string(field, value).map_err(|_| {
            invalid_runtime_request(
                route_name,
                "runtime_field_unsafe",
                "DAG DB runtime request field is unsafe",
            )
        })?;
    }
    if !requester_did.starts_with("did:") {
        return Err(invalid_runtime_request(
            route_name,
            "runtime_requester_did_invalid",
            "DAG DB runtime requester_did must be a DID",
        ));
    }
    Ok(())
}

#[cfg(feature = "production-db")]
fn runtime_operation_id(
    domain: &str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
    db_set_version: &str,
    material_hash: &str,
) -> Result<String, Box<Response>> {
    hash_hex(
        domain,
        &(
            tenant_id,
            namespace,
            idempotency_key,
            db_set_version,
            material_hash,
        ),
    )
}

#[cfg(feature = "production-db")]
fn runtime_non_claims() -> Vec<String> {
    vec![
        "m60_not_approved".to_owned(),
        "operator_runtime_approval_not_present".to_owned(),
        "not_final_evidence".to_owned(),
    ]
}

#[cfg(feature = "production-db")]
fn export_record_count(export: &exo_dag_db_exchange::kg_export::KgPortableExport) -> u32 {
    [
        export.memory_records.len(),
        export.catalog_entries.len(),
        export.graph_nodes.len(),
        export.graph_edges.len(),
        export.similarity_results.len(),
        export.canonicalization_decisions.len(),
        export.placement_traces.len(),
        export.validation_reports.len(),
        export.receipts.len(),
        export.subject_receipt_heads.len(),
        export.context_packet_previews.len(),
        export.context_packet_records.len(),
        export.route_receipts.len(),
        export.writeback_summaries.len(),
        export.idempotency_references.len(),
        export.citation_index.len(),
        export.provenance_index.len(),
    ]
    .into_iter()
    .fold(0_u32, |total, count| {
        total.saturating_add(u32::try_from(count).unwrap_or(u32::MAX))
    })
}

fn invalid_runtime_request(
    route_name: &'static str,
    category: &'static str,
    message: &'static str,
) -> Box<Response> {
    warn!(
        route = route_name,
        status = 400,
        error_code = "invalid_request_shape",
        category,
        "DAG DB runtime request rejected"
    );
    Box::new(dagdb_error_response(
        StatusCode::BAD_REQUEST,
        "invalid_request_shape",
        message,
        false,
    ))
}

fn trust_check_response_from_request(
    request: DagDbTrustCheckRequest,
    route_name: &str,
) -> Result<DagDbTrustCheckResponse, Box<Response>> {
    let mut redacted_body = request_json(&request)?;
    let signature_hash = Hash256::digest(request.signature.as_bytes()).to_string();
    replace_metadata(
        &mut redacted_body,
        "signature",
        "signature_hash",
        request_json(&signature_hash)?,
    )?;
    let request_hash = request_hash(
        route_name,
        &request.tenant_id,
        &request.namespace,
        &redacted_body,
    )?;
    Ok(DagDbTrustCheckResponse {
        schema_version: exo_api::dagdb::DAGDB_TRUST_CHECK_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        idempotency_key: request.idempotency_key,
        credential_id: hash_hex(
            "dagdb.gateway.credential",
            &(&request_hash, &request.agent_did),
        )?,
        safety_score_id: hash_hex(
            "dagdb.gateway.safety_score",
            &(&request_hash, &request.operator_did),
        )?,
        receipt_hash: receipt_hash(route_name, request_hash)?,
        validation_status: ValidationStatus::Passed,
        council_status: CouncilReviewStatus::NotRequired,
        credential_status: CredentialStatus::Active,
        total_score_bp: 10_000,
        created_new: true,
        block_reason: None,
        expires_at: Some(request.expires_at),
    })
}

fn receipt_lookup_response(request: DagDbReceiptLookupRequest) -> DagDbReceiptLookupResponse {
    DagDbReceiptLookupResponse {
        schema_version: exo_api::dagdb::DAGDB_RECEIPT_LOOKUP_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        receipt_hash: request.receipt_hash,
        subject_kind: SubjectKind::Memory,
        subject_id: Hash256::ZERO.to_string(),
        prev_receipt_hash: Hash256::ZERO.to_string(),
        seq: 1,
        event_type: ReceiptEventType::IntakeCreated,
        actor_did: "did:exo:gateway".to_owned(),
        event_hlc: "0:0".to_owned(),
        created_at: "0:0".to_owned(),
        receipt_body: request.include_body.unwrap_or(false).then(|| {
            json!({
                "summary": safe_gateway_metadata("receipt lookup body")
            })
        }),
        validation_report_id: None,
    }
}

fn catalog_lookup_response(request: DagDbCatalogLookupRequest) -> DagDbCatalogLookupResponse {
    let title = safe_gateway_metadata("catalog");
    let summary = safe_gateway_metadata("catalog summary");
    DagDbCatalogLookupResponse {
        schema_version: exo_api::dagdb::DAGDB_CATALOG_LOOKUP_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        catalog_id: request.catalog_id,
        catalog_level: 0,
        title: title.clone(),
        summary: summary.clone(),
        keywords: Vec::new(),
        status: MemoryStatus::Routable,
        validation_status: ValidationStatus::Passed,
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Committed,
        latest_receipt_hash: Hash256::ZERO.to_string(),
        memory_id: None,
        parent_catalog_id: None,
        children: request.include_children.unwrap_or(false).then(|| {
            vec![CatalogEntryResponse {
                catalog_id: Hash256::digest(b"child-catalog").to_string(),
                title,
                summary,
            }]
        }),
        routes: request
            .include_routes
            .unwrap_or(false)
            .then(|| vec![Hash256::digest(b"catalog-route").to_string()]),
    }
}

fn route_lookup_response(request: DagDbRouteLookupRequest) -> DagDbRouteLookupResponse {
    DagDbRouteLookupResponse {
        schema_version: exo_api::dagdb::DAGDB_ROUTE_LOOKUP_RESPONSE_SCHEMA_VERSION.to_owned(),
        tenant_id: request.tenant_id,
        namespace: request.namespace,
        route_id: request.route_id,
        route_status: RouteStatus::Active,
        validation_status: ValidationStatus::Passed,
        council_status: CouncilReviewStatus::NotRequired,
        dag_finality_status: DagFinalityStatus::Committed,
        selected_memory_ids: Vec::new(),
        route_score_bp: 0,
        token_budget: 0,
        token_estimate: 0,
        stale_at: "86400000:0".to_owned(),
        latest_receipt_hash: Hash256::ZERO.to_string(),
        memory_refs: request.include_memory_refs.unwrap_or(false).then(|| {
            vec![ContextPacketMemoryRef {
                memory_id: Hash256::digest(b"route-memory").to_string(),
                title: safe_gateway_metadata("memory"),
                summary: safe_gateway_metadata("memory summary"),
                keywords: Vec::new(),
                latest_receipt_hash: Hash256::ZERO.to_string(),
            }]
        }),
        validation_report: None,
    }
}

fn sanitize_metadata(field: MetadataField, text: &str) -> Result<SafeMetadata, Box<Response>> {
    sanitize_runtime_metadata(field, text).map_err(|_| {
        Box::new(dagdb_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "metadata_rejected",
            "DAG DB metadata was rejected",
            true,
        ))
    })
}

fn sanitize_optional_metadata(
    field: MetadataField,
    text: Option<&str>,
) -> Result<Option<SafeMetadata>, Box<Response>> {
    text.map(|value| sanitize_metadata(field, value))
        .transpose()
}

fn sanitize_keyword_texts(texts: Option<&[String]>) -> Result<Vec<SafeMetadata>, Box<Response>> {
    let empty = Vec::new();
    let keyword_texts = texts.unwrap_or(&empty);
    sanitize_keywords(keyword_texts).map_err(|_| {
        Box::new(dagdb_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "metadata_rejected",
            "DAG DB keyword metadata was rejected",
            true,
        ))
    })
}

fn request_json<T: Serialize>(request: &T) -> Result<Value, Box<Response>> {
    serde_json::to_value(request).map_err(|_| {
        Box::new(dagdb_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
            "DAG DB request could not be canonicalized",
            false,
        ))
    })
}

fn replace_metadata(
    body: &mut Value,
    inbound_field: &str,
    stored_field: &str,
    value: Value,
) -> Result<(), Box<Response>> {
    let Some(object) = body.as_object_mut() else {
        return Err(Box::new(dagdb_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
            "DAG DB request body must be a JSON object",
            false,
        )));
    };
    object.remove(inbound_field);
    object.insert(stored_field.to_owned(), value);
    Ok(())
}

fn request_hash<T: Serialize>(
    route_name: &str,
    tenant_id: &str,
    namespace: &str,
    redacted_body: &T,
) -> Result<Hash256, Box<Response>> {
    let mut canonical_body = Vec::new();
    ciborium::ser::into_writer(redacted_body, &mut canonical_body).map_err(|_| {
        Box::new(dagdb_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
            "DAG DB request body could not be encoded canonically",
            false,
        ))
    })?;
    RequestHashMaterial {
        route_name: route_name.to_owned(),
        tenant_id: tenant_id.to_owned(),
        namespace: namespace.to_owned(),
        canonical_redacted_request_body: canonical_body,
    }
    .hash()
    .map_err(|_| {
        Box::new(dagdb_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
            "DAG DB request hash could not be computed",
            false,
        ))
    })
}

fn hash_hex<T: Serialize>(domain: &str, value: &T) -> Result<String, Box<Response>> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&(domain, value), &mut bytes).map_err(|_| {
        Box::new(dagdb_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
            "DAG DB response hash could not be computed",
            false,
        ))
    })?;
    Ok(Hash256::digest(&bytes).to_string())
}

#[cfg(feature = "production-db")]
fn gateway_idempotency_response_hash<T: Serialize>(
    response_body: &T,
    route_name: &str,
    stage: &'static str,
    operation: &'static str,
    tenant_id: &str,
    namespace: &str,
    idempotency_key: &str,
) -> Result<Hash256, Box<Response>> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(
        &("dagdb.gateway.idempotency_response", response_body),
        &mut bytes,
    )
    .map_err(|_| {
        idempotency_unavailable_response_logged(
            route_name,
            stage,
            operation,
            tenant_id,
            namespace,
            idempotency_key,
        )
    })?;
    Ok(Hash256::digest(&bytes))
}

fn receipt_hash(route_name: &str, request_hash: Hash256) -> Result<String, Box<Response>> {
    hash_hex("dagdb.gateway.receipt", &(route_name, request_hash))
}

fn safe_gateway_metadata(text: &str) -> SafeMetadata {
    SafeMetadata {
        decision: exo_api::dagdb::SafeMetadataDecision::Allow,
        text: text.to_owned(),
        redaction_codes: Vec::new(),
        original_hash: Hash256::digest(text.as_bytes()).to_string(),
        truncated: false,
        byte_len: u32::try_from(text.len()).unwrap_or(u32::MAX),
    }
}

fn council_unauthenticated_response() -> Response {
    dagdb_error_response(
        StatusCode::UNAUTHORIZED,
        "unauthenticated",
        "DAG DB council decision requires bearer authentication",
        false,
    )
}

fn council_tenant_scope_mismatch_response() -> Response {
    dagdb_error_response(
        StatusCode::FORBIDDEN,
        "tenant_scope_mismatch",
        "DAG DB tenant or namespace does not match the authorized scope",
        false,
    )
}

fn council_authority_required_response() -> Response {
    dagdb_error_response(
        StatusCode::FORBIDDEN,
        "council_authority_required",
        "DAG DB council decision requires council authority scope",
        true,
    )
}

fn council_error_response(error: CouncilError) -> Response {
    match error {
        CouncilError::InvalidRequestShape(_) | CouncilError::Hash(_) => dagdb_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
            "DAG DB council decision request is invalid",
            false,
        ),
        CouncilError::Metadata(_) => dagdb_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "metadata_rejected",
            "DAG DB council decision metadata was rejected",
            true,
        ),
        CouncilError::ApprovalScopeMismatch => dagdb_error_response(
            StatusCode::CONFLICT,
            "approval_scope_mismatch",
            "DAG DB council decision scope does not match the subject",
            true,
        ),
        CouncilError::ApprovalDenied => dagdb_error_response(
            StatusCode::FORBIDDEN,
            "approval_denied",
            "DAG DB council decision was denied",
            true,
        ),
        CouncilError::CouncilEscalationRequired => dagdb_error_response(
            StatusCode::FORBIDDEN,
            "council_escalation_required",
            "DAG DB council decision requires escalation",
            true,
        ),
        CouncilError::ApprovalRequired => dagdb_error_response(
            StatusCode::FORBIDDEN,
            "approval_required",
            "DAG DB council approval is required",
            true,
        ),
    }
}

fn dagdb_error_response(
    status: StatusCode,
    error_code: &str,
    message: impl Into<String>,
    requires_council_review: bool,
) -> Response {
    (
        status,
        Json(DagDbErrorEnvelope {
            error_code: error_code.to_owned(),
            message: message.into(),
            receipt_hash: None,
            validation_report_id: None,
            requires_council_review,
        }),
    )
        .into_response()
}

// T1: the local-dev keypair loader (including the deterministic [0..31] seed
// fallback) is compiled ONLY in debug builds. A release binary has no path that
// can materialize a publicly-derivable signing key for the dev profile; the
// signing identity in release comes exclusively from a provisioned seed file or
// the canonical gatekeeper signer, which fail closed when absent.
#[cfg(debug_assertions)]
struct LocalDevKeypair {
    keypair: exo_core::crypto::KeyPair,
    source: &'static str,
}

#[cfg(debug_assertions)]
fn local_dev_key_seed_path() -> String {
    std::env::var("DAGDB_DEV_KEY_SEED").unwrap_or_else(|_| LOCAL_DEV_KEY_SEED_REL.to_owned())
}

#[cfg(debug_assertions)]
fn load_local_dev_keypair_from_seed_path(seed_path: &str) -> Result<LocalDevKeypair, String> {
    load_local_dev_keypair_from_seed_path_with_source(
        seed_path,
        LOCAL_DEV_KEY_SOURCE_EXPLICIT_SEED,
        local_dev_fallback_enabled(),
    )
}

#[cfg(debug_assertions)]
fn local_dev_fallback_enabled() -> bool {
    std::env::var(LOCAL_DEV_GATEKEEPER_ENV)
        .map(|value| value == "1")
        .unwrap_or(false)
}

#[cfg(debug_assertions)]
fn load_local_dev_keypair_from_seed_path_with_source(
    seed_path: &str,
    seed_source: &'static str,
    fallback_enabled: bool,
) -> Result<LocalDevKeypair, String> {
    use std::path::Path;

    use exo_core::crypto::KeyPair;

    let (seed_bytes, source) = match std::fs::read(Path::new(&seed_path)) {
        Ok(bytes) if bytes.len() >= 32 => {
            let mut seed = [0_u8; 32];
            seed.copy_from_slice(&bytes[..32]);
            (seed, seed_source)
        }
        Ok(_) => return Err(format!("dev key seed at {seed_path} is too short")),
        Err(error) => {
            if !fallback_enabled {
                return Err(format!(
                    "dev key seed unavailable at {seed_path}: {error}; set {LOCAL_DEV_GATEKEEPER_ENV}=1 to permit deterministic local-dev fallback"
                ));
            }
            (
                core::array::from_fn(|index| u8::try_from(index).unwrap_or(0)),
                LOCAL_DEV_KEY_SOURCE_DETERMINISTIC_FALLBACK,
            )
        }
    };
    let keypair = KeyPair::from_secret_bytes(seed_bytes).map_err(|error| error.to_string())?;
    Ok(LocalDevKeypair { keypair, source })
}

#[cfg(test)]
mod tests {
    #![allow(unexpected_cfgs)]

    use axum::{
        body::Body,
        extract::{Path, Query},
        http::{HeaderMap, HeaderValue, Request, StatusCode, header},
    };
    use exo_api::dagdb::{
        CouncilDecisionStatus, DagDbCatalogLookupRequest, DagDbContextPacketRequest,
        DagDbCouncilDecisionRequest, DagDbErrorEnvelope, DagDbExportRequest, DagDbImportRequest,
        DagDbIntakeRequest, DagDbReceiptLookupRequest, DagDbRouteLookupRequest, DagDbRouteRequest,
        DagDbTrustCheckRequest, DagDbValidateRequest, DagDbWritebackRequest, DecisionSource,
        RiskClass, SubjectKind,
    };
    use exo_core::Hash256;
    use serde::{Serialize, Serializer, de::DeserializeOwned};
    use tower::ServiceExt;

    use super::*;

    fn dagdb_app() -> Router {
        dagdb_router::<()>()
    }

    #[test]
    fn dagdb_rest_prefix_is_pinned() {
        assert_eq!(DAGDB_REST_PREFIX, "/api/v1/dag-db");
    }

    #[test]
    fn dagdb_json_fixtures() {
        let fixtures = fixtures();
        assert_fixture::<DagDbIntakeRequest>(&fixtures, "requests", "intake");
        assert_fixture::<DagDbRouteRequest>(&fixtures, "requests", "route");
        assert_fixture::<DagDbContextPacketRequest>(&fixtures, "requests", "context_packet");
        assert_fixture::<DagDbValidateRequest>(&fixtures, "requests", "validate");
        assert_fixture::<DagDbWritebackRequest>(&fixtures, "requests", "writeback");
        assert_fixture::<DagDbTrustCheckRequest>(&fixtures, "requests", "trust_check");
        assert_fixture::<DagDbCouncilDecisionRequest>(&fixtures, "requests", "council_decision");
        assert_fixture::<DagDbReceiptLookupRequest>(&fixtures, "requests", "receipt_lookup");
        assert_fixture::<DagDbCatalogLookupRequest>(&fixtures, "requests", "catalog_lookup");
        assert_fixture::<DagDbRouteLookupRequest>(&fixtures, "requests", "route_lookup");
        assert_fixture::<DagDbErrorEnvelope>(&fixtures, "errors", "tenant_scope_mismatch");
    }

    #[tokio::test]
    async fn dagdb_router_registers_every_route() {
        let fixtures = fixtures();
        let app = dagdb_app();

        assert_scaffold_post_response(
            app.clone(),
            "/api/v1/dag-db/intake",
            "dagdb:intake",
            fixture::<DagDbIntakeRequest>(&fixtures, "requests", "intake"),
        )
        .await;
        assert_scaffold_post_response(
            app.clone(),
            "/api/v1/dag-db/route",
            "dagdb:route",
            fixture::<DagDbRouteRequest>(&fixtures, "requests", "route"),
        )
        .await;
        assert_scaffold_post_response(
            app.clone(),
            "/api/v1/dag-db/context-packet",
            "dagdb:context_packet",
            fixture::<DagDbContextPacketRequest>(&fixtures, "requests", "context_packet"),
        )
        .await;
        assert_scaffold_post_response(
            app.clone(),
            "/api/v1/dag-db/validate",
            "dagdb:validate",
            fixture::<DagDbValidateRequest>(&fixtures, "requests", "validate"),
        )
        .await;
        // Writeback fails closed (503) with no production database pool,
        // identically to import/export — never a synthetic 201 scaffold receipt.
        assert_post_error(
            app.clone(),
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            fixture::<DagDbWritebackRequest>(&fixtures, "requests", "writeback"),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
        assert_post_error(
            app.clone(),
            "/api/v1/dag-db/import",
            "dagdb:import",
            import_request(),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
        assert_post_error(
            app.clone(),
            "/api/v1/dag-db/export",
            "dagdb:export",
            export_request(),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
        assert_scaffold_post_response(
            app.clone(),
            "/api/v1/dag-db/trust-check",
            "dagdb:trust_check",
            fixture::<DagDbTrustCheckRequest>(&fixtures, "requests", "trust_check"),
        )
        .await;
        assert_post_error(
            app.clone(),
            "/api/v1/dag-db/council/decision",
            "dagdb:council_decision",
            fixture::<DagDbCouncilDecisionRequest>(&fixtures, "requests", "council_decision"),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;

        let receipt: DagDbReceiptLookupRequest = fixture(&fixtures, "requests", "receipt_lookup");
        assert_scaffold_get_response(
            app.clone(),
            &format!(
                "/api/v1/dag-db/receipts/{}?tenant_id={}&namespace={}&include_body=true",
                receipt.receipt_hash, receipt.tenant_id, receipt.namespace
            ),
            "dagdb:receipt_lookup",
        )
        .await;
        let catalog: DagDbCatalogLookupRequest = fixture(&fixtures, "requests", "catalog_lookup");
        assert_scaffold_get_response(
            app.clone(),
            &format!(
                "/api/v1/dag-db/catalog/{}?tenant_id={}&namespace={}&include_children=true&include_routes=true",
                catalog.catalog_id, catalog.tenant_id, catalog.namespace
            ),
            "dagdb:catalog_lookup",
        )
        .await;
        let route: DagDbRouteLookupRequest = fixture(&fixtures, "requests", "route_lookup");
        assert_scaffold_get_response(
            app,
            &format!(
                "/api/v1/dag-db/routes/{}?tenant_id={}&namespace={}&include_memory_refs=true&include_validation=true",
                route.route_id, route.tenant_id, route.namespace
            ),
            "dagdb:route_lookup",
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_router_rejects_metadata_failures_before_success_response() {
        let fixtures = fixtures();
        let app = dagdb_app();

        let intake = DagDbIntakeRequest {
            title_text: "fn raw_payload() {}".to_owned(),
            ..fixture(&fixtures, "requests", "intake")
        };
        assert_error_response(
            app.clone()
                .oneshot(scoped_json_request(
                    "POST",
                    "/api/v1/dag-db/intake",
                    "dagdb:intake",
                    &intake,
                ))
                .await
                .expect("intake metadata rejection"),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;

        let import = DagDbImportRequest {
            import_report: json!({"raw_markdown": "forbidden"}),
            ..import_request()
        };
        assert_error_response(
            app.clone()
                .oneshot(scoped_json_request(
                    "POST",
                    "/api/v1/dag-db/import",
                    "dagdb:import",
                    &import,
                ))
                .await
                .expect("import adapter rejection"),
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
        )
        .await;

        let validate = DagDbValidateRequest {
            validation_notes_text: Some("fn raw_payload() {}".to_owned()),
            ..fixture(&fixtures, "requests", "validate")
        };
        assert_error_response(
            app.clone()
                .oneshot(scoped_json_request(
                    "POST",
                    "/api/v1/dag-db/validate",
                    "dagdb:validate",
                    &validate,
                ))
                .await
                .expect("validate metadata rejection"),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;

        // Writeback now fails closed (503) on the no-pool path BEFORE building any
        // response, so raw-code summary metadata is no longer the first rejection
        // here — the fail-closed 503 is. Summary-metadata rejection on the real
        // persisted path (with a pool) is covered by `writeback_response_from_*`
        // tests. The knowledge-class metadata rejection (422) still precedes the
        // pool check; see `dagdb_writeback_route_rejects_unknown_knowledge_class`.
        let writeback = DagDbWritebackRequest {
            summary_text: Some("fn raw_payload() {}".to_owned()),
            ..fixture(&fixtures, "requests", "writeback")
        };
        assert_error_response(
            app.oneshot(scoped_json_request(
                "POST",
                "/api/v1/dag-db/writeback",
                "dagdb:writeback",
                &writeback,
            ))
            .await
            .expect("writeback no-pool fail-closed"),
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_export_route_rejects_malformed_included_memory_ids_before_scaffold() {
        let app = dagdb_app();
        let export = DagDbExportRequest {
            included_memory_ids: vec!["not-a-hash".to_owned()],
            ..export_request()
        };
        assert_error_response(
            app.oneshot(scoped_json_request(
                "POST",
                "/api/v1/dag-db/export",
                "dagdb:export",
                &export,
            ))
            .await
            .expect("malformed export memory id response"),
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
        )
        .await;
    }

    #[test]
    fn validate_writeback_knowledge_class_accepts_each_allowed_class_with_summary() {
        let fixtures = fixtures();
        for class in ["decision", "finding", "fix", "constraint", "handoff"] {
            let request = DagDbWritebackRequest {
                summary_text: Some(format!("content-bearing {class} summary")),
                knowledge_class: Some(class.to_owned()),
                ..fixture(&fixtures, "requests", "writeback")
            };
            assert!(
                validate_writeback_knowledge_class(&request).is_ok(),
                "class {class} with a summary must validate"
            );
        }
    }

    #[test]
    fn validate_writeback_knowledge_class_accepts_classless_writeback() {
        let fixtures = fixtures();
        let request = DagDbWritebackRequest {
            knowledge_class: None,
            ..fixture(&fixtures, "requests", "writeback")
        };
        assert!(validate_writeback_knowledge_class(&request).is_ok());
    }

    #[tokio::test]
    async fn dagdb_writeback_route_accepts_known_knowledge_class() {
        // A known knowledge class passes the pre-pool validation (no 422
        // class rejection) and then fails closed (503) on the no-pool path,
        // rather than being rejected as an invalid class. This distinguishes a
        // valid-class-but-no-pool writeback (503) from an invalid class (422).
        let fixtures = fixtures();
        let app = dagdb_app();
        let writeback = DagDbWritebackRequest {
            summary_text: Some("Implemented typed knowledge writebacks".to_owned()),
            knowledge_class: Some("finding".to_owned()),
            ..fixture(&fixtures, "requests", "writeback")
        };
        assert_post_error(
            app,
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            writeback,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_writeback_route_rejects_unknown_knowledge_class() {
        let fixtures = fixtures();
        let app = dagdb_app();
        let writeback = DagDbWritebackRequest {
            summary_text: Some("Has a summary but an invalid class".to_owned()),
            knowledge_class: Some("rumor".to_owned()),
            ..fixture(&fixtures, "requests", "writeback")
        };
        assert_error_response(
            app.oneshot(scoped_json_request(
                "POST",
                "/api/v1/dag-db/writeback",
                "dagdb:writeback",
                &writeback,
            ))
            .await
            .expect("unknown knowledge class response"),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_knowledge_class",
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_writeback_route_rejects_knowledge_class_with_empty_summary() {
        let fixtures = fixtures();
        let app = dagdb_app();
        let writeback = DagDbWritebackRequest {
            summary_text: Some("   ".to_owned()),
            knowledge_class: Some("decision".to_owned()),
            ..fixture(&fixtures, "requests", "writeback")
        };
        assert_error_response(
            app.clone()
                .oneshot(scoped_json_request(
                    "POST",
                    "/api/v1/dag-db/writeback",
                    "dagdb:writeback",
                    &writeback,
                ))
                .await
                .expect("blank summary knowledge class response"),
            StatusCode::UNPROCESSABLE_ENTITY,
            "knowledge_class_requires_summary",
        )
        .await;

        let missing_summary = DagDbWritebackRequest {
            summary_text: None,
            knowledge_class: Some("decision".to_owned()),
            ..fixture(&fixtures, "requests", "writeback")
        };
        assert_error_response(
            app.oneshot(scoped_json_request(
                "POST",
                "/api/v1/dag-db/writeback",
                "dagdb:writeback",
                &missing_summary,
            ))
            .await
            .expect("missing summary knowledge class response"),
            StatusCode::UNPROCESSABLE_ENTITY,
            "knowledge_class_requires_summary",
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_import_export_json_rejections_use_sanitized_envelope() {
        let app = dagdb_app();
        let mut import = serde_json::to_value(import_request()).expect("import json");
        import
            .as_object_mut()
            .expect("import object")
            .insert("secret_api_key".to_owned(), json!("sk-prod-secret"));
        assert_sanitized_invalid_shape(
            app.clone()
                .oneshot(scoped_raw_json_request(
                    "POST",
                    "/api/v1/dag-db/import",
                    "dagdb:import",
                    import.to_string(),
                ))
                .await
                .expect("import unknown field response"),
            &["secret_api_key", "sk-prod-secret"],
        )
        .await;

        let mut export = serde_json::to_value(export_request()).expect("export json");
        export
            .as_object_mut()
            .expect("export object")
            .insert("secret_token".to_owned(), json!("bearer-secret-value"));
        assert_sanitized_invalid_shape(
            app.clone()
                .oneshot(scoped_raw_json_request(
                    "POST",
                    "/api/v1/dag-db/export",
                    "dagdb:export",
                    export.to_string(),
                ))
                .await
                .expect("export unknown field response"),
            &["secret_token", "bearer-secret-value"],
        )
        .await;

        assert_sanitized_invalid_shape(
            app.oneshot(scoped_raw_json_request(
                "POST",
                "/api/v1/dag-db/import",
                "dagdb:import",
                r#"{"tenant_id":"tenant-a","secret_api_key":"sk-prod-secret""#.to_owned(),
            ))
            .await
            .expect("import malformed json response"),
            &["secret_api_key", "sk-prod-secret"],
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_runtime_import_export_no_pool_503_error_shapes() {
        let ctx = DagDbRouteContext::from_pool(None);
        assert_error_response_shape(
            import_handler(&ctx, &authorized_headers("dagdb:import"), import_request()).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "DAG DB import requires a configured production database",
            false,
        )
        .await;
        assert_error_response_shape(
            export_handler(&ctx, &authorized_headers("dagdb:export"), export_request()).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "DAG DB export requires a configured production database",
            false,
        )
        .await;
    }

    /// P1: writeback fails closed (503, `database_unavailable`) with no
    /// production database pool — identical envelope shape to import/export,
    /// with no synthetic 201 scaffold and no fabricated receipt body.
    #[tokio::test]
    async fn dagdb_runtime_writeback_no_pool_fails_closed_503() {
        let fixtures = fixtures();
        let ctx = DagDbRouteContext::from_pool(None);
        let request: DagDbWritebackRequest = fixture(&fixtures, "requests", "writeback");
        assert_error_response_shape(
            writeback_handler(&ctx, &authorized_headers("dagdb:writeback"), request).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "DAG DB writeback requires a configured production database",
            false,
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_scaffold_routes_without_pool_fail_closed_when_authorized() {
        let fixtures = fixtures();
        let intake: DagDbIntakeRequest = fixture(&fixtures, "requests", "intake");
        assert_error_response_shape(
            handle_dagdb_intake(None, authorized_headers("dagdb:intake"), Json(intake)).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "DAG DB intake requires a configured production database",
            false,
        )
        .await;

        let catalog: DagDbCatalogLookupRequest = fixture(&fixtures, "requests", "catalog_lookup");
        assert_error_response_shape(
            handle_dagdb_catalog_lookup(
                None,
                authorized_headers("dagdb:catalog_lookup"),
                Path(catalog.catalog_id.clone()),
                Query(lookup_query(&catalog.tenant_id, &catalog.namespace, &[])),
            )
            .await,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "DAG DB catalog_lookup requires a configured production database",
            false,
        )
        .await;
    }

    #[tokio::test]
    async fn dagdb_handlers_cover_authorized_and_denied_branches_directly() {
        let fixtures = fixtures();
        let scaffold_post_status = StatusCode::SERVICE_UNAVAILABLE;
        let scaffold_get_status = StatusCode::SERVICE_UNAVAILABLE;
        let intake: DagDbIntakeRequest = fixture(&fixtures, "requests", "intake");
        assert_eq!(
            handle_dagdb_intake(
                None,
                authorized_headers("dagdb:intake"),
                Json(intake.clone())
            )
            .await
            .status(),
            scaffold_post_status
        );
        assert_eq!(
            handle_dagdb_intake(None, denied_headers("dagdb:intake"), Json(intake))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let route: DagDbRouteRequest = fixture(&fixtures, "requests", "route");
        assert_eq!(
            handle_dagdb_route(None, authorized_headers("dagdb:route"), Json(route.clone()))
                .await
                .status(),
            scaffold_post_status
        );
        assert_eq!(
            handle_dagdb_route(None, denied_headers("dagdb:route"), Json(route))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let packet: DagDbContextPacketRequest = fixture(&fixtures, "requests", "context_packet");
        assert_eq!(
            handle_dagdb_context_packet(
                None,
                authorized_headers("dagdb:context_packet"),
                Json(packet.clone()),
            )
            .await
            .status(),
            scaffold_post_status
        );
        assert_eq!(
            handle_dagdb_context_packet(
                None,
                denied_headers("dagdb:context_packet"),
                Json(packet),
            )
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let validation: DagDbValidateRequest = fixture(&fixtures, "requests", "validate");
        assert_eq!(
            handle_dagdb_validate(
                None,
                authorized_headers("dagdb:validate"),
                Json(validation.clone())
            )
            .await
            .status(),
            scaffold_post_status
        );
        assert_eq!(
            handle_dagdb_validate(None, denied_headers("dagdb:validate"), Json(validation))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let writeback: DagDbWritebackRequest = fixture(&fixtures, "requests", "writeback");
        // Authorized writeback fails closed (503) with no pool — the audit's
        // synthetic-201 scaffold is gone. Authority denial still precedes the
        // pool check, so the denied branch stays FORBIDDEN.
        assert_eq!(
            handle_dagdb_writeback(
                None,
                authorized_headers("dagdb:writeback"),
                Json(writeback.clone()),
            )
            .await
            .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            handle_dagdb_writeback(None, denied_headers("dagdb:writeback"), Json(writeback),)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let import = import_request();
        assert_eq!(
            handle_dagdb_import(
                None,
                authorized_headers("dagdb:import"),
                Ok(Json(import.clone())),
            )
            .await
            .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            handle_dagdb_import(None, denied_headers("dagdb:import"), Ok(Json(import)))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let export = export_request();
        assert_eq!(
            handle_dagdb_export(
                None,
                authorized_headers("dagdb:export"),
                Ok(Json(export.clone())),
            )
            .await
            .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            handle_dagdb_export(None, denied_headers("dagdb:export"), Ok(Json(export)))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let trust: DagDbTrustCheckRequest = fixture(&fixtures, "requests", "trust_check");
        assert_eq!(
            handle_dagdb_trust_check(
                None,
                authorized_headers("dagdb:trust_check"),
                Json(trust.clone())
            )
            .await
            .status(),
            scaffold_post_status
        );
        assert_eq!(
            handle_dagdb_trust_check(None, denied_headers("dagdb:trust_check"), Json(trust))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );

        let receipt: DagDbReceiptLookupRequest = fixture(&fixtures, "requests", "receipt_lookup");
        let receipt_query = lookup_query(
            &receipt.tenant_id,
            &receipt.namespace,
            &[("include_body", "true")],
        );
        assert_eq!(
            handle_dagdb_receipt_lookup(
                None,
                authorized_headers("dagdb:receipt_lookup"),
                Path(receipt.receipt_hash.clone()),
                Query(receipt_query)
            )
            .await
            .status(),
            scaffold_get_status
        );
        assert_eq!(
            handle_dagdb_receipt_lookup(
                None,
                denied_headers("dagdb:receipt_lookup"),
                Path(receipt.receipt_hash.clone()),
                Query(lookup_query(&receipt.tenant_id, &receipt.namespace, &[]))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );

        let catalog: DagDbCatalogLookupRequest = fixture(&fixtures, "requests", "catalog_lookup");
        let catalog_query = lookup_query(
            &catalog.tenant_id,
            &catalog.namespace,
            &[("include_children", "true"), ("include_routes", "true")],
        );
        assert_eq!(
            handle_dagdb_catalog_lookup(
                None,
                authorized_headers("dagdb:catalog_lookup"),
                Path(catalog.catalog_id.clone()),
                Query(catalog_query)
            )
            .await
            .status(),
            scaffold_get_status
        );
        assert_eq!(
            handle_dagdb_catalog_lookup(
                None,
                denied_headers("dagdb:catalog_lookup"),
                Path(catalog.catalog_id.clone()),
                Query(lookup_query(&catalog.tenant_id, &catalog.namespace, &[]))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );

        let route_lookup: DagDbRouteLookupRequest = fixture(&fixtures, "requests", "route_lookup");
        let route_query = lookup_query(
            &route_lookup.tenant_id,
            &route_lookup.namespace,
            &[
                ("include_memory_refs", "true"),
                ("include_validation", "true"),
            ],
        );
        assert_eq!(
            handle_dagdb_route_lookup(
                None,
                authorized_headers("dagdb:route_lookup"),
                Path(route_lookup.route_id.clone()),
                Query(route_query)
            )
            .await
            .status(),
            scaffold_get_status
        );
        assert_eq!(
            handle_dagdb_route_lookup(
                None,
                denied_headers("dagdb:route_lookup"),
                Path(route_lookup.route_id.clone()),
                Query(lookup_query(
                    &route_lookup.tenant_id,
                    &route_lookup.namespace,
                    &[]
                ))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn dagdb_private_gateway_vectors_cover_fail_closed_branches() {
        let fixtures = fixtures();
        let route_vectors = [
            ("dagdb.intake", "dagdb:intake"),
            ("dagdb.route", "dagdb:route"),
            ("dagdb.context_packet", "dagdb:context_packet"),
            ("dagdb.validate", "dagdb:validate"),
            ("dagdb.writeback", "dagdb:writeback"),
            ("dagdb.import", "dagdb:import"),
            ("dagdb.export", "dagdb:export"),
            ("dagdb.trust_check", "dagdb:trust_check"),
        ];
        for (route_name, action) in route_vectors {
            assert!(route_name.starts_with("dagdb."));
            assert!(action.starts_with("dagdb:"));
        }
        let query = lookup_query(
            "tenant-a",
            "primary",
            &[("include_body", "true"), ("include_routes", "false")],
        );
        assert_eq!(required_query_text(&query, "tenant_id"), "tenant-a");
        assert_eq!(optional_query_bool(&query, "include_body"), Some(true));
        assert_eq!(optional_query_bool(&query, "include_routes"), Some(false));
        assert_eq!(optional_query_bool(&query, "missing"), None);

        let mut headers = HeaderMap::new();
        assert_eq!(
            verify_dagdb_authority(&headers, "tenant-a", "primary", "dagdb:intake")
                .expect("missing auth")
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            dagdb_authority_denial(&headers, "tenant-a", "primary", "dagdb:intake"),
            Some(DagDbAuthorityDenial {
                status: StatusCode::UNAUTHORIZED,
                error_code: "unauthenticated",
            })
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic test"),
        );
        assert_eq!(
            verify_dagdb_authority(&headers, "tenant-a", "primary", "dagdb:intake")
                .expect("basic auth")
                .status(),
            StatusCode::UNAUTHORIZED
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test"),
        );
        assert_eq!(
            verify_dagdb_authority(&headers, "tenant-a", "primary", "dagdb:intake")
                .expect("missing tenant")
                .status(),
            StatusCode::FORBIDDEN
        );
        headers.insert(TENANT_HEADER, HeaderValue::from_static("tenant-a"));
        assert_eq!(
            verify_dagdb_authority(&headers, "tenant-a", "primary", "dagdb:intake")
                .expect("missing namespace")
                .status(),
            StatusCode::FORBIDDEN
        );
        headers.insert(NAMESPACE_HEADER, HeaderValue::from_static("primary"));
        assert_eq!(
            verify_dagdb_authority(&headers, "tenant-a", "primary", "dagdb:intake")
                .expect("missing authority")
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            dagdb_authority_denial(&headers, "tenant-a", "primary", "dagdb:intake"),
            Some(DagDbAuthorityDenial {
                status: StatusCode::FORBIDDEN,
                error_code: "authority_denied",
            })
        );
        let mut namespace_mismatch = headers.clone();
        namespace_mismatch.insert(NAMESPACE_HEADER, HeaderValue::from_static("other"));
        assert_eq!(
            verify_dagdb_authority(&namespace_mismatch, "tenant-a", "primary", "dagdb:intake")
                .expect("namespace mismatch")
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            dagdb_authority_denial(&namespace_mismatch, "tenant-a", "primary", "dagdb:intake"),
            Some(DagDbAuthorityDenial {
                status: StatusCode::FORBIDDEN,
                error_code: "tenant_scope_mismatch",
            })
        );
        headers.insert(
            AUTHORITY_SCOPE_HEADER,
            HeaderValue::from_static("dagdb:other:tenant-a:primary dagdb:intake:tenant-a:primary"),
        );
        assert!(verify_dagdb_authority(&headers, "tenant-a", "primary", "dagdb:intake").is_none());
        log_dagdb_authority_denial(
            "dagdb.intake",
            &headers,
            "tenant-a",
            "primary",
            "dagdb:intake",
        );

        let intake: DagDbIntakeRequest = fixture(&fixtures, "requests", "intake");
        assert!(
            intake_response_from_request(
                DagDbIntakeRequest {
                    title_text: "fn raw_payload() {}".to_owned(),
                    ..intake.clone()
                },
                "dagdb.intake",
            )
            .is_err()
        );
        assert!(
            intake_response_from_request(
                DagDbIntakeRequest {
                    keyword_texts: Some(vec!["fn raw_payload() {}".to_owned()]),
                    ..intake
                },
                "dagdb.intake",
            )
            .is_err()
        );

        let mut route: DagDbRouteRequest = fixture(&fixtures, "requests", "route");
        route.requested_memory_ids = None;
        assert!(route_response_from_request(route, "dagdb.route").is_ok());

        let validate: DagDbValidateRequest = fixture(&fixtures, "requests", "validate");
        assert!(
            validate_response_from_request(
                DagDbValidateRequest {
                    validation_notes_text: None,
                    ..validate.clone()
                },
                "dagdb.validate",
            )
            .is_ok()
        );
        assert!(
            validate_response_from_request(
                DagDbValidateRequest {
                    validation_notes_text: Some("fn raw_payload() {}".to_owned()),
                    ..validate
                },
                "dagdb.validate",
            )
            .is_err()
        );

        let writeback: DagDbWritebackRequest = fixture(&fixtures, "requests", "writeback");
        assert!(
            writeback_response_from_request(
                DagDbWritebackRequest {
                    summary_text: None,
                    keyword_texts: None,
                    ..writeback.clone()
                },
                "dagdb.writeback",
            )
            .is_ok()
        );
        assert!(
            writeback_response_from_request(
                DagDbWritebackRequest {
                    keyword_texts: Some(vec!["fn raw_payload() {}".to_owned()]),
                    ..writeback
                },
                "dagdb.writeback",
            )
            .is_err()
        );

        let import = import_request();
        assert!(
            validated_import_report_json(&DagDbImportRequest {
                source_hash: "bad-hash".to_owned(),
                ..import
            })
            .is_err()
        );

        let export = export_request();
        assert!(
            export_scope_from_request(&DagDbExportRequest {
                requester_did: "agent-without-did-prefix".to_owned(),
                ..export
            })
            .is_err()
        );

        let receipt = DagDbReceiptLookupRequest {
            include_body: None,
            ..fixture(&fixtures, "requests", "receipt_lookup")
        };
        assert!(receipt_lookup_response(receipt).receipt_body.is_none());
        let catalog = DagDbCatalogLookupRequest {
            include_children: None,
            include_routes: None,
            ..fixture(&fixtures, "requests", "catalog_lookup")
        };
        let catalog_response = catalog_lookup_response(catalog);
        assert!(catalog_response.children.is_none());
        assert!(catalog_response.routes.is_none());
        let route_lookup = DagDbRouteLookupRequest {
            include_memory_refs: None,
            include_validation: None,
            ..fixture(&fixtures, "requests", "route_lookup")
        };
        assert!(route_lookup_response(route_lookup).memory_refs.is_none());

        let mut non_object = Value::Null;
        assert!(replace_metadata(&mut non_object, "raw", "safe", json!("value")).is_err());
        assert!(request_json(&FailingSerialize).is_err());
        assert!(request_hash("dagdb.test", "tenant-a", "primary", &FailingSerialize).is_err());
        assert!(hash_hex("dagdb.test", &FailingSerialize).is_err());
        assert!(sanitize_optional_metadata(MetadataField::Summary, None).is_ok());
    }

    #[test]
    fn dagdb_layered_runtime_gateway_context_packet_is_additive_and_fail_closed() {
        let fixtures = fixtures();
        let request: DagDbContextPacketRequest = fixture(&fixtures, "requests", "context_packet");

        let legacy = context_packet_response_from_request(request.clone(), "dagdb.context_packet")
            .expect("legacy context packet");
        assert!(legacy.layered_mode.is_none());
        assert!(legacy.selected_layers.is_none());
        assert!(legacy.selected_layer_edges.is_none());
        assert!(legacy.layer_budget_report.is_none());
        assert!(legacy.flat_fallback_used.is_none());
        assert!(legacy.layered_status.is_none());

        let auto = context_packet_response_from_request(
            DagDbContextPacketRequest {
                layered_mode: Some("auto".to_owned()),
                max_layer_depth: Some(4),
                require_layer_evidence: Some(false),
                ..request.clone()
            },
            "dagdb.context_packet",
        )
        .expect("auto layered context packet");
        assert_eq!(auto.layered_mode.as_deref(), Some("auto"));
        assert_eq!(
            auto.layered_status.as_deref(),
            Some("flat_fallback_no_layer_evidence")
        );
        assert_eq!(auto.flat_fallback_used, Some(true));
        assert_eq!(
            auto.layer_budget_report
                .as_ref()
                .expect("layer budget")
                .budget_status
                .as_str(),
            "flat_fallback_no_layer_evidence"
        );
        assert_eq!(auto.selected_layers.as_ref().expect("layers").len(), 0);

        assert!(
            context_packet_response_from_request(
                DagDbContextPacketRequest {
                    layered_mode: Some("required".to_owned()),
                    max_layer_depth: Some(4),
                    require_layer_evidence: Some(true),
                    ..request.clone()
                },
                "dagdb.context_packet",
            )
            .is_err(),
            "required layered context must reject scaffold packets with no layer evidence"
        );
        assert!(
            context_packet_response_from_request(
                DagDbContextPacketRequest {
                    layered_mode: Some("sometimes".to_owned()),
                    ..request.clone()
                },
                "dagdb.context_packet",
            )
            .is_err(),
            "unsupported layered_mode must fail closed"
        );
        assert!(
            context_packet_response_from_request(
                DagDbContextPacketRequest {
                    layered_mode: Some("auto".to_owned()),
                    max_layer_depth: Some(DAGDB_MAX_LAYER_DEPTH + 1),
                    ..request
                },
                "dagdb.context_packet",
            )
            .is_err(),
            "over-budget max_layer_depth must fail closed"
        );
    }

    #[test]
    fn dagdb_layered_runtime_gateway_writeback_records_target_layers() {
        let fixtures = fixtures();
        let request: DagDbWritebackRequest = fixture(&fixtures, "requests", "writeback");

        let legacy = writeback_response_from_request(request.clone(), "dagdb.writeback")
            .expect("legacy writeback");
        assert!(legacy.target_layer_path.is_none());
        assert!(legacy.target_layer_depth.is_none());
        assert!(legacy.target_layer_reason.is_none());
        assert!(legacy.created_child_layer_id.is_none());
        assert!(legacy.layered_writeback_status.is_none());

        let layered = writeback_response_from_request(
            DagDbWritebackRequest {
                layered_mode: Some("auto".to_owned()),
                target_layer_path: Some("root/codex/runtime".to_owned()),
                target_layer_depth: Some(2),
                target_layer_reason: Some("agent_runtime_layered_mode".to_owned()),
                ..request.clone()
            },
            "dagdb.writeback",
        )
        .expect("layered writeback");
        assert_eq!(
            layered.target_layer_path.as_deref(),
            Some("root/codex/runtime")
        );
        assert_eq!(layered.target_layer_depth, Some(2));
        assert_eq!(
            layered.target_layer_reason.as_deref(),
            Some("agent_runtime_layered_mode")
        );
        assert_eq!(
            layered.layered_writeback_status.as_deref(),
            Some("layer_target_recorded")
        );
        assert_eq!(
            layered
                .created_child_layer_id
                .as_ref()
                .expect("created child layer id")
                .len(),
            64
        );

        assert!(
            writeback_response_from_request(
                DagDbWritebackRequest {
                    layered_mode: Some("required".to_owned()),
                    ..request.clone()
                },
                "dagdb.writeback",
            )
            .is_err(),
            "required writeback must include an explicit target layer"
        );
        assert!(
            writeback_response_from_request(
                DagDbWritebackRequest {
                    layered_mode: Some("off".to_owned()),
                    target_layer_path: Some("root/codex/runtime".to_owned()),
                    target_layer_depth: Some(2),
                    target_layer_reason: Some("agent_runtime_layered_mode".to_owned()),
                    ..request.clone()
                },
                "dagdb.writeback",
            )
            .is_err(),
            "target layer fields must not be accepted when layered_mode is off"
        );
        assert!(
            writeback_response_from_request(
                DagDbWritebackRequest {
                    layered_mode: Some("auto".to_owned()),
                    target_layer_path: Some("/root/codex/runtime".to_owned()),
                    target_layer_depth: Some(2),
                    target_layer_reason: Some("agent_runtime_layered_mode".to_owned()),
                    ..request.clone()
                },
                "dagdb.writeback",
            )
            .is_err(),
            "absolute target layer paths must fail closed"
        );
        assert!(
            writeback_response_from_request(
                DagDbWritebackRequest {
                    layered_mode: Some("auto".to_owned()),
                    target_layer_path: Some("root/codex/runtime".to_owned()),
                    target_layer_depth: Some(1),
                    target_layer_reason: Some("agent_runtime_layered_mode".to_owned()),
                    ..request
                },
                "dagdb.writeback",
            )
            .is_err(),
            "target_layer_depth must match target_layer_path"
        );
    }

    #[cfg(feature = "production-db")]
    #[test]
    fn dagdb_writeback_signed_task_hash_binds_layer_target() {
        let fixtures = fixtures();
        let request: DagDbWritebackRequest = fixture(&fixtures, "requests", "writeback");
        let flat_without_metadata = DagDbWritebackRequest {
            summary_text: None,
            knowledge_class: None,
            layered_mode: None,
            target_layer_path: None,
            target_layer_depth: None,
            target_layer_reason: None,
            ..request.clone()
        };
        assert_eq!(
            writeback_signed_task_hash(&flat_without_metadata).expect("flat task hash"),
            flat_without_metadata.answer_hash,
            "flat writebacks without metadata or layered fields must keep signing the raw answer_hash"
        );

        let with_summary = DagDbWritebackRequest {
            summary_text: Some("Safe answer summary".to_owned()),
            ..flat_without_metadata.clone()
        };
        let summary_hash = writeback_signed_task_hash(&with_summary).expect("summary task hash");
        assert_ne!(
            summary_hash, with_summary.answer_hash,
            "metadata-bearing writebacks must not sign the raw answer_hash"
        );
        let mutated_summary = DagDbWritebackRequest {
            summary_text: Some("Mutated searchable summary".to_owned()),
            ..with_summary.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&mutated_summary).expect("mutated summary task hash"),
            summary_hash,
            "mutating summary_text must change the signed task hash"
        );

        let empty_summary = DagDbWritebackRequest {
            summary_text: Some(String::new()),
            ..flat_without_metadata.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&empty_summary).expect("empty summary task hash"),
            writeback_signed_task_hash(&flat_without_metadata).expect("absent summary task hash"),
            "absent and empty summary_text must bind to distinct task hashes"
        );

        let with_knowledge_class = DagDbWritebackRequest {
            summary_text: Some("Typed knowledge writeback summary".to_owned()),
            knowledge_class: Some("finding".to_owned()),
            ..flat_without_metadata.clone()
        };
        let knowledge_class_hash =
            writeback_signed_task_hash(&with_knowledge_class).expect("knowledge class task hash");
        let mutated_knowledge_class = DagDbWritebackRequest {
            knowledge_class: Some("decision".to_owned()),
            ..with_knowledge_class.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&mutated_knowledge_class)
                .expect("mutated knowledge class task hash"),
            knowledge_class_hash,
            "mutating knowledge_class must change the signed task hash"
        );

        let empty_knowledge_class = DagDbWritebackRequest {
            summary_text: Some("Typed knowledge writeback summary".to_owned()),
            knowledge_class: Some(String::new()),
            ..flat_without_metadata.clone()
        };
        let absent_knowledge_class = DagDbWritebackRequest {
            summary_text: Some("Typed knowledge writeback summary".to_owned()),
            knowledge_class: None,
            ..flat_without_metadata.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&empty_knowledge_class)
                .expect("empty knowledge class task hash"),
            writeback_signed_task_hash(&absent_knowledge_class)
                .expect("absent knowledge class task hash"),
            "absent and empty knowledge_class must bind to distinct task hashes"
        );

        let layered = DagDbWritebackRequest {
            layered_mode: Some("auto".to_owned()),
            target_layer_path: Some("root/codex/runtime".to_owned()),
            target_layer_depth: Some(2),
            target_layer_reason: Some("agent_runtime_layered_mode".to_owned()),
            ..flat_without_metadata.clone()
        };
        let layered_hash = writeback_signed_task_hash(&layered).expect("layered task hash");
        assert_ne!(
            layered_hash, layered.answer_hash,
            "layered writebacks must bind the layer target into the signed task hash"
        );
        assert_eq!(
            writeback_signed_task_hash(&layered).expect("layered task hash repeat"),
            layered_hash,
            "layer target binding must be deterministic"
        );

        let mutated_path = DagDbWritebackRequest {
            target_layer_path: Some("root/codex/exfiltration".to_owned()),
            ..layered.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&mutated_path).expect("mutated path task hash"),
            layered_hash,
            "mutating target_layer_path must change the signed task hash"
        );
        let mutated_depth = DagDbWritebackRequest {
            target_layer_depth: Some(3),
            ..layered.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&mutated_depth).expect("mutated depth task hash"),
            layered_hash,
            "mutating target_layer_depth must change the signed task hash"
        );
        let mutated_reason = DagDbWritebackRequest {
            target_layer_reason: Some("mutated_reason".to_owned()),
            ..layered.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&mutated_reason).expect("mutated reason task hash"),
            layered_hash,
            "mutating target_layer_reason must change the signed task hash"
        );
        let mutated_mode = DagDbWritebackRequest {
            layered_mode: Some("required".to_owned()),
            ..layered.clone()
        };
        assert_ne!(
            writeback_signed_task_hash(&mutated_mode).expect("mutated mode task hash"),
            layered_hash,
            "mutating layered_mode must change the signed task hash"
        );

        let mode_only = DagDbWritebackRequest {
            layered_mode: Some("auto".to_owned()),
            target_layer_path: None,
            target_layer_depth: None,
            target_layer_reason: None,
            ..flat_without_metadata.clone()
        };
        let mode_only_hash = writeback_signed_task_hash(&mode_only).expect("mode-only task hash");
        assert_ne!(
            mode_only_hash, flat_without_metadata.answer_hash,
            "adding layered_mode alone must change the signed task hash"
        );
        let empty_reason = DagDbWritebackRequest {
            layered_mode: Some("auto".to_owned()),
            target_layer_path: None,
            target_layer_depth: None,
            target_layer_reason: Some(String::new()),
            ..flat_without_metadata
        };
        assert_ne!(
            writeback_signed_task_hash(&empty_reason).expect("empty reason task hash"),
            mode_only_hash,
            "absent and empty layered fields must bind to distinct task hashes"
        );
    }

    #[tokio::test]
    async fn dagdb_layered_runtime_gateway_exposes_specific_error_codes() {
        let fixtures = fixtures();
        let app = dagdb_app();
        let request: DagDbContextPacketRequest = fixture(&fixtures, "requests", "context_packet");

        assert_error_response(
            app.clone()
                .oneshot(scoped_json_request(
                    "POST",
                    "/api/v1/dag-db/context-packet",
                    "dagdb:context_packet",
                    &DagDbContextPacketRequest {
                        layered_mode: Some("sometimes".to_owned()),
                        ..request.clone()
                    },
                ))
                .await
                .expect("invalid layered mode response"),
            StatusCode::BAD_REQUEST,
            "invalid_layered_mode",
        )
        .await;

        assert_error_response(
            app.oneshot(scoped_json_request(
                "POST",
                "/api/v1/dag-db/context-packet",
                "dagdb:context_packet",
                &DagDbContextPacketRequest {
                    layered_mode: Some("required".to_owned()),
                    require_layer_evidence: Some(true),
                    ..request
                },
            ))
            .await
            .expect("required layer evidence missing response"),
            StatusCode::BAD_REQUEST,
            "required_layer_evidence_missing",
        )
        .await;
    }

    #[test]
    fn runtime_request_basics_cover_success_empty_unsafe_and_did_branches() {
        assert!(
            validate_runtime_request_basics(
                "dagdb.import",
                "tenant-a",
                "primary",
                "idem-1",
                "dag_db-project_memory_v3",
                "did:exo:agent",
            )
            .is_ok()
        );

        let invalid_vectors = [
            (
                "",
                "primary",
                "idem-1",
                "dag_db-project_memory_v3",
                "did:exo:agent",
            ),
            (
                "tenant-a",
                " ",
                "idem-1",
                "dag_db-project_memory_v3",
                "did:exo:agent",
            ),
            (
                "tenant-a",
                "primary",
                "",
                "dag_db-project_memory_v3",
                "did:exo:agent",
            ),
            ("tenant-a", "primary", "idem-1", "", "did:exo:agent"),
            (
                "tenant-a",
                "primary",
                "idem-1",
                "dag_db-project_memory_v3",
                "",
            ),
            (
                "tenant-a",
                "primary",
                "idem-1",
                "dag_db-project_memory_v3",
                "agent-without-did",
            ),
            (
                "tenant-a",
                "primary",
                "fn raw_payload() {}",
                "dag_db-project_memory_v3",
                "did:exo:agent",
            ),
        ];
        for (tenant_id, namespace, idempotency_key, db_set_version, requester_did) in
            invalid_vectors
        {
            assert!(
                validate_runtime_request_basics(
                    "dagdb.import",
                    tenant_id,
                    namespace,
                    idempotency_key,
                    db_set_version,
                    requester_did,
                )
                .is_err(),
                "expected invalid basics for {tenant_id:?} {namespace:?} {idempotency_key:?} {db_set_version:?} {requester_did:?}"
            );
        }
    }

    // Debug-only: the deterministic local-dev fallback exists ONLY in debug
    // builds (T1). Under `cargo test --release` `debug_assertions` is off and
    // these symbols are compiled out, so the tests that exercise them are gated
    // to the same configuration. The release posture is proven separately by
    // `release_build_has_no_dev_gatekeeper_or_deterministic_seed` below.
    #[cfg(debug_assertions)]
    #[test]
    fn local_dev_gatekeeper_profile_requires_explicit_fallback_seed_gate() {
        let missing_seed = "__missing_dagdb_local_dev_seed__";
        assert!(
            load_local_dev_keypair_from_seed_path_with_source(
                missing_seed,
                LOCAL_DEV_KEY_SOURCE_EXPLICIT_SEED,
                false,
            )
            .is_err()
        );
        let fallback = load_local_dev_keypair_from_seed_path_with_source(
            missing_seed,
            LOCAL_DEV_KEY_SOURCE_EXPLICIT_SEED,
            true,
        )
        .expect("explicit local-dev fallback");
        assert_eq!(fallback.source, LOCAL_DEV_KEY_SOURCE_DETERMINISTIC_FALLBACK);
        DagDbRouteContext::from_pool(None).install_local_dev_gatekeeper_profile();
    }

    #[cfg(debug_assertions)]
    #[test]
    fn local_dev_keypair_reads_seed_file_and_rejects_short_seed() {
        let temp_dir =
            std::env::temp_dir().join(format!("dagdb-local-dev-seed-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).expect("create seed temp dir");
        let full_seed = temp_dir.join("full.seed");
        let short_seed = temp_dir.join("short.seed");
        std::fs::write(&full_seed, [7_u8; 32]).expect("write full seed");
        std::fs::write(&short_seed, [3_u8; 31]).expect("write short seed");

        let full_seed = full_seed.to_str().expect("utf8 seed path");
        let short_seed = short_seed.to_str().expect("utf8 short seed path");
        let loaded = load_local_dev_keypair_from_seed_path(full_seed).expect("full seed");
        assert_eq!(loaded.source, LOCAL_DEV_KEY_SOURCE_EXPLICIT_SEED);
        assert!(load_local_dev_keypair_from_seed_path(short_seed).is_err());
        DagDbRouteContext::from_pool(None)
            .install_local_dev_gatekeeper_profile_from_seed_path(short_seed);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn import_report_validation_accepts_scope_and_rejects_scope_drift() {
        let request = import_request();
        let report_json = validated_import_report_json(&request).expect("valid import report");
        let report_value: serde_json::Value =
            serde_json::from_str(&report_json).expect("report json");
        assert_eq!(report_value["tenant_id"], "tenant-a");
        assert_eq!(report_value["namespace"], "primary");

        let mut tenant_mismatch = import_request();
        tenant_mismatch.import_report["tenant_id"] = json!("tenant-b");
        assert!(validated_import_report_json(&tenant_mismatch).is_err());

        let mut namespace_mismatch = import_request();
        namespace_mismatch.import_report["namespace"] = json!("secondary");
        assert!(validated_import_report_json(&namespace_mismatch).is_err());

        let mut invalid_report = import_request();
        invalid_report.import_report["schema_version"] = json!("unknown_schema");
        assert!(validated_import_report_json(&invalid_report).is_err());

        // The report's declared actor must be the signature-verified
        // requester: receipts derive their actor from the report, so a
        // foreign actor here would attribute receipts to a principal that
        // never signed the request.
        let mut actor_mismatch = import_request();
        actor_mismatch.import_report["actor_did"] = json!("did:exo:rogue-actor");
        for intent in actor_mismatch.import_report["proposed_receipt_intents"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            intent["actor_did"] = json!("did:exo:rogue-actor");
        }
        assert!(validated_import_report_json(&actor_mismatch).is_err());
    }

    #[test]
    fn export_scope_validation_copies_filters_and_rejects_defensive_branches() {
        let memory_id = Hash256::digest(b"memory-a").to_string();
        let request = DagDbExportRequest {
            included_memory_ids: vec![memory_id.clone()],
            included_graph_styles: vec!["semantic_catalog_graph".to_owned()],
            included_writeback_idempotency_keys: vec!["idem-writeback-1".to_owned()],
            include_preview_context: true,
            ..export_request()
        };
        let scope = export_scope_from_request(&request).expect("valid export scope");
        assert_eq!(scope.tenant_id, "tenant-a");
        assert_eq!(scope.namespace, "primary");
        assert_eq!(scope.included_memory_ids, vec![memory_id]);
        assert_eq!(
            scope.included_writeback_idempotency_keys,
            vec!["idem-writeback-1"]
        );
        assert!(scope.include_preview_context);

        assert!(
            export_scope_from_request(&DagDbExportRequest {
                included_graph_styles: vec!["style-a".to_owned(), "style-a".to_owned()],
                ..export_request()
            })
            .is_err()
        );
        assert!(
            export_scope_from_request(&DagDbExportRequest {
                source_commit_or_repo_ref: Some("".to_owned()),
                ..export_request()
            })
            .is_err()
        );
        assert!(
            export_scope_from_request(&DagDbExportRequest {
                source_commit_or_repo_ref: Some("fn raw_payload() {}".to_owned()),
                ..export_request()
            })
            .is_err()
        );
    }

    #[test]
    fn request_hash_uses_redacted_metadata_after_scope_verification() {
        let fixtures = fixtures();
        let request = DagDbIntakeRequest {
            title_text: "SSN 123-45-6789".to_owned(),
            ..fixture(&fixtures, "requests", "intake")
        };
        let raw_body = request_json(&request).expect("raw request json");
        let raw_hash = request_hash(
            "dagdb.intake",
            &request.tenant_id,
            &request.namespace,
            &raw_body,
        )
        .expect("raw request hash");

        let title = sanitize_metadata(MetadataField::Title, &request.title_text)
            .expect("SSN title is redacted");
        let summary = sanitize_metadata(MetadataField::Summary, &request.summary_text)
            .expect("summary is safe");
        let keywords =
            sanitize_keyword_texts(request.keyword_texts.as_deref()).expect("keywords are safe");
        let mut redacted_body = request_json(&request).expect("redacted body base");
        replace_metadata(
            &mut redacted_body,
            "title_text",
            "title",
            request_json(&title).expect("title json"),
        )
        .expect("replace title");
        replace_metadata(
            &mut redacted_body,
            "summary_text",
            "summary",
            request_json(&summary).expect("summary json"),
        )
        .expect("replace summary");
        replace_metadata(
            &mut redacted_body,
            "keyword_texts",
            "keywords",
            request_json(&keywords).expect("keywords json"),
        )
        .expect("replace keywords");
        let redacted_hash = request_hash(
            "dagdb.intake",
            &request.tenant_id,
            &request.namespace,
            &redacted_body,
        )
        .expect("redacted request hash");
        let encoded_redacted = serde_json::to_string(&redacted_body).expect("json string");

        assert_ne!(raw_hash, redacted_hash);
        assert!(!encoded_redacted.contains("123-45-6789"));
        assert!(encoded_redacted.contains("[REDACTED_SSN]"));
    }

    #[tokio::test]
    async fn dagdb_council_decision_route() {
        let app = dagdb_app();

        let unavailable = app
            .clone()
            .oneshot(council_request(
                "tenant-a",
                "primary",
                "dagdb:council_decision:tenant-a:primary",
            ))
            .await
            .expect("route response");
        assert_error_response(
            unavailable,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;

        let mismatch = app
            .clone()
            .oneshot(council_request(
                "tenant-b",
                "primary",
                "dagdb:council_decision:tenant-b:primary",
            ))
            .await
            .expect("tenant mismatch response");
        assert_eq!(mismatch.status(), axum::http::StatusCode::FORBIDDEN);
        let mismatch_error: exo_api::dagdb::DagDbErrorEnvelope = json_body(mismatch).await;
        assert_eq!(mismatch_error.error_code, "tenant_scope_mismatch");

        let missing_scope = app
            .oneshot(council_request("tenant-a", "primary", "dagdb:other"))
            .await
            .expect("missing scope response");
        assert_eq!(missing_scope.status(), axum::http::StatusCode::FORBIDDEN);
        let scope_error: exo_api::dagdb::DagDbErrorEnvelope = json_body(missing_scope).await;
        assert_eq!(scope_error.error_code, "council_authority_required");
    }

    #[test]
    fn council_authority_checks_fail_closed() {
        let request = council_decision_request();
        let mut headers = HeaderMap::new();
        assert_eq!(
            verify_council_authority(&headers, &request)
                .expect("missing auth")
                .status(),
            StatusCode::UNAUTHORIZED
        );

        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic test"),
        );
        assert_eq!(
            verify_council_authority(&headers, &request)
                .expect("basic auth")
                .status(),
            StatusCode::UNAUTHORIZED
        );

        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test"),
        );
        assert_eq!(
            verify_council_authority(&headers, &request)
                .expect("missing tenant")
                .status(),
            StatusCode::FORBIDDEN
        );
        headers.insert(TENANT_HEADER, HeaderValue::from_static("tenant-a"));
        assert_eq!(
            verify_council_authority(&headers, &request)
                .expect("missing namespace")
                .status(),
            StatusCode::FORBIDDEN
        );
        headers.insert(NAMESPACE_HEADER, HeaderValue::from_static("other"));
        assert_eq!(
            verify_council_authority(&headers, &request)
                .expect("namespace mismatch")
                .status(),
            StatusCode::FORBIDDEN
        );

        headers.insert(NAMESPACE_HEADER, HeaderValue::from_static("primary"));
        assert_eq!(
            verify_council_authority(&headers, &request)
                .expect("missing council authority")
                .status(),
            StatusCode::FORBIDDEN
        );

        headers.insert(
            AUTHORITY_SCOPE_HEADER,
            HeaderValue::from_static("other,dagdb:council_decision:tenant-a:primary"),
        );
        assert!(verify_council_authority(&headers, &request).is_none());
    }

    #[tokio::test]
    async fn council_error_responses_are_stable() {
        assert_error_response(
            council_error_response(
                exo_dag_db_domain::council::CouncilError::InvalidRequestShape("subject_id"),
            ),
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
        )
        .await;
        assert_error_response(
            council_error_response(exo_dag_db_domain::council::CouncilError::Hash(
                "cbor".to_owned(),
            )),
            StatusCode::BAD_REQUEST,
            "invalid_request_shape",
        )
        .await;
        let metadata_error = exo_dag_db_core::metadata::sanitize_runtime_metadata(
            exo_dag_db_core::metadata::MetadataField::CouncilNotes,
            "fn raw_payload() {}",
        )
        .expect_err("code excerpt rejected");
        assert_error_response(
            council_error_response(exo_dag_db_domain::council::CouncilError::Metadata(
                metadata_error,
            )),
            StatusCode::UNPROCESSABLE_ENTITY,
            "metadata_rejected",
        )
        .await;
        assert_error_response(
            council_error_response(exo_dag_db_domain::council::CouncilError::ApprovalScopeMismatch),
            StatusCode::CONFLICT,
            "approval_scope_mismatch",
        )
        .await;
        assert_error_response(
            council_error_response(exo_dag_db_domain::council::CouncilError::ApprovalDenied),
            StatusCode::FORBIDDEN,
            "approval_denied",
        )
        .await;
        assert_error_response(
            council_error_response(
                exo_dag_db_domain::council::CouncilError::CouncilEscalationRequired,
            ),
            StatusCode::FORBIDDEN,
            "council_escalation_required",
        )
        .await;
        assert_error_response(
            council_error_response(exo_dag_db_domain::council::CouncilError::ApprovalRequired),
            StatusCode::FORBIDDEN,
            "approval_required",
        )
        .await;
    }

    async fn assert_scaffold_post_response<T>(app: Router, path: &str, action: &str, body: T)
    where
        T: Serialize,
    {
        assert_post_error(
            app,
            path,
            action,
            body,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
    }

    async fn assert_post_error<T>(
        app: Router,
        path: &str,
        action: &str,
        body: T,
        status: StatusCode,
        error_code: &str,
    ) where
        T: Serialize,
    {
        let response = app
            .oneshot(scoped_json_request("POST", path, action, &body))
            .await
            .expect("DAG DB POST route error response");
        assert_error_response(response, status, error_code).await;
    }

    async fn assert_scaffold_get_response(app: Router, path: &str, action: &str) {
        assert_get_error(
            app,
            path,
            action,
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
        )
        .await;
    }

    async fn assert_get_error(
        app: Router,
        path: &str,
        action: &str,
        status: StatusCode,
        error_code: &str,
    ) {
        let response = app
            .oneshot(scoped_get_request(path, action))
            .await
            .expect("DAG DB GET route error response");
        assert_error_response(response, status, error_code).await;
    }

    fn scoped_json_request<T>(method: &str, uri: &str, action: &str, body: &T) -> Request<Body>
    where
        T: Serialize,
    {
        let body = serde_json::to_vec(body).expect("serialize DAG DB request");
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, "Bearer test-token")
            .header(TENANT_HEADER, "tenant-a")
            .header(NAMESPACE_HEADER, "primary")
            .header(AUTHORITY_SCOPE_HEADER, format!("{action}:tenant-a:primary"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .expect("request")
    }

    fn scoped_raw_json_request(
        method: &str,
        uri: &str,
        action: &str,
        body: String,
    ) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, "Bearer test-token")
            .header(TENANT_HEADER, "tenant-a")
            .header(NAMESPACE_HEADER, "primary")
            .header(AUTHORITY_SCOPE_HEADER, format!("{action}:tenant-a:primary"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .expect("request")
    }

    fn scoped_get_request(uri: &str, action: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .header(header::AUTHORIZATION, "Bearer test-token")
            .header(TENANT_HEADER, "tenant-a")
            .header(NAMESPACE_HEADER, "primary")
            .header(AUTHORITY_SCOPE_HEADER, format!("{action}:tenant-a:primary"))
            .body(Body::empty())
            .expect("request")
    }

    fn authorized_headers(action: &str) -> HeaderMap {
        scoped_headers(action, "tenant-a")
    }

    fn denied_headers(action: &str) -> HeaderMap {
        scoped_headers(action, "tenant-b")
    }

    fn scoped_headers(action: &str, tenant: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test"),
        );
        headers.insert(
            TENANT_HEADER,
            HeaderValue::from_str(tenant).expect("valid tenant header"),
        );
        headers.insert(NAMESPACE_HEADER, HeaderValue::from_static("primary"));
        headers.insert(
            AUTHORITY_SCOPE_HEADER,
            HeaderValue::from_str(&format!("{action}:{tenant}:primary"))
                .expect("valid authority scope header"),
        );
        headers
    }

    fn import_request() -> DagDbImportRequest {
        DagDbImportRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "primary".to_owned(),
            idempotency_key: "idem-import-1".to_owned(),
            db_set_version: "dag_db-project_memory_v3".to_owned(),
            source_hash: "1111111111111111111111111111111111111111111111111111111111111111"
                .to_owned(),
            requester_did: "did:exo:importer".to_owned(),
            import_report: json!({
                "schema_version": exo_dag_db_exchange::kg_import::KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
                "source_candidates_schema_version": exo_dag_db_exchange::kg_import::KG_IMPORT_CANDIDATES_SCHEMA,
                "graph_root": "KnowledgeGraphs/dag-db",
                "tenant_id": "tenant-a",
                "namespace": "primary",
                "actor_did": "did:exo:importer",
                "batch_id": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "dry_run_only": true,
                "postgres_writes": false,
                "raw_markdown_included": false,
                "proposed_memory_records": [],
                "proposed_catalog_entries": [],
                "proposed_graph_nodes": [],
                "proposed_graph_edges": [],
                "proposed_required_edges": [],
                "proposed_placement_decisions": [],
                "proposed_receipt_intents": [],
                "proposed_validation_reports": [],
                "proposed_governance_reviews": [],
                "proposed_graph_view_refreshes": [],
                "proposed_route_invalidations": [],
                "proposed_subdag_boundaries": [],
                "rollback_plan": {},
                "placement_governance_summary": {},
                "review_items": [],
                "warnings": []
            }),
        }
    }

    fn export_request() -> DagDbExportRequest {
        DagDbExportRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "primary".to_owned(),
            idempotency_key: "idem-export-1".to_owned(),
            db_set_version: "dag_db-project_memory_v3".to_owned(),
            requester_did: "did:exo:exporter".to_owned(),
            included_memory_ids: Vec::new(),
            included_graph_styles: Vec::new(),
            included_writeback_idempotency_keys: Vec::new(),
            source_commit_or_repo_ref: Some("c706242d36f1c275e05d8a132778491da08f61c7".to_owned()),
            include_preview_context: false,
        }
    }

    fn lookup_query(tenant_id: &str, namespace: &str, extras: &[(&str, &str)]) -> QueryParams {
        let mut query = QueryParams::new();
        query.insert("tenant_id".to_owned(), tenant_id.to_owned());
        query.insert("namespace".to_owned(), namespace.to_owned());
        for (name, value) in extras {
            query.insert((*name).to_owned(), (*value).to_owned());
        }
        query
    }

    fn fixtures() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
        ))
        .expect("parse complete DAG DB fixture set")
    }

    fn fixture<T>(fixtures: &serde_json::Value, section: &str, name: &str) -> T
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(
            fixtures
                .get(section)
                .and_then(|section| section.get(name))
                .unwrap_or_else(|| panic!("missing fixture {section}.{name}"))
                .clone(),
        )
        .unwrap_or_else(|err| panic!("parse fixture {section}.{name}: {err}"))
    }

    fn assert_fixture<T>(fixtures: &serde_json::Value, section: &str, name: &str)
    where
        T: DeserializeOwned + Serialize,
    {
        let parsed: T = fixture(fixtures, section, name);
        let serialized = serde_json::to_value(parsed)
            .unwrap_or_else(|err| panic!("serialize fixture {section}.{name}: {err}"));
        assert_eq!(
            serialized,
            fixtures
                .get(section)
                .and_then(|section| section.get(name))
                .unwrap_or_else(|| panic!("missing fixture {section}.{name}"))
                .clone(),
            "fixture {section}.{name} drifted"
        );
    }

    fn council_request(
        tenant_header: &str,
        namespace_header: &str,
        authority_scope: &str,
    ) -> Request<Body> {
        let body =
            serde_json::to_vec(&council_decision_request()).expect("serialize council request");

        Request::builder()
            .method("POST")
            .uri("/api/v1/dag-db/council/decision")
            .header(axum::http::header::AUTHORIZATION, "Bearer test-token")
            .header(TENANT_HEADER, tenant_header)
            .header(NAMESPACE_HEADER, namespace_header)
            .header(AUTHORITY_SCOPE_HEADER, authority_scope)
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .expect("request")
    }

    fn council_decision_request() -> exo_api::dagdb::DagDbCouncilDecisionRequest {
        exo_api::dagdb::DagDbCouncilDecisionRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "primary".to_owned(),
            idempotency_key: "idem-council-1".to_owned(),
            subject_kind: SubjectKind::Memory,
            subject_id: Hash256::from_bytes([0xf0; 32]).to_string(),
            requested_action: "memory:routable".to_owned(),
            approved_scope_hash: Hash256::from_bytes([0x12; 32]).to_string(),
            risk_class: RiskClass::R3,
            approver_did: "did:exo:council".to_owned(),
            decision_source: DecisionSource::Human,
            decision_status: CouncilDecisionStatus::Approved,
            reason_code: "operator_approved".to_owned(),
            created_at: "1000:0".to_owned(),
            expires_at: "2000:0".to_owned(),
            validation_report_id: None,
            route_id: None,
            context_packet_id: None,
            notes_text: Some("Safe approval notes".to_owned()),
        }
    }

    async fn json_body<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        serde_json::from_slice(&bytes).expect("json body")
    }

    async fn assert_error_response(
        response: axum::response::Response,
        expected_status: StatusCode,
        expected_code: &str,
    ) {
        assert_eq!(response.status(), expected_status);
        let envelope: exo_api::dagdb::DagDbErrorEnvelope = json_body(response).await;
        assert_eq!(envelope.error_code, expected_code);
    }

    async fn assert_sanitized_invalid_shape(
        response: axum::response::Response,
        forbidden_fragments: &[&str],
    ) {
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        let envelope: exo_api::dagdb::DagDbErrorEnvelope =
            serde_json::from_str(&body).expect("DAG DB error envelope");
        assert_eq!(envelope.error_code, "invalid_request_shape");
        for fragment in forbidden_fragments {
            assert!(
                !body.contains(fragment),
                "sanitized envelope must not contain {fragment}"
            );
        }
    }

    async fn assert_error_response_shape(
        response: axum::response::Response,
        expected_status: StatusCode,
        expected_code: &str,
        expected_message: &str,
        expected_requires_council_review: bool,
    ) {
        assert_eq!(response.status(), expected_status);
        let envelope: exo_api::dagdb::DagDbErrorEnvelope = json_body(response).await;
        assert_eq!(envelope.error_code, expected_code);
        assert_eq!(envelope.message, expected_message);
        assert_eq!(
            envelope.requires_council_review,
            expected_requires_council_review
        );
    }

    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(serde::ser::Error::custom("intentional serialize failure"))
        }
    }

    #[cfg(feature = "production-db")]
    mod production_db_tests {
        use std::{collections::BTreeMap, sync::Arc, time::Duration};

        use axum::{
            Json,
            extract::Extension,
            http::{HeaderMap, HeaderValue, StatusCode, header},
        };
        use exo_api::dagdb::DagDbContextPacketRequest;
        use exo_core::crypto::KeyPair;
        use exo_dag_db_domain::scoring::DomainError;
        use exo_gatekeeper::{
            ConsentEngine, GatekeeperError, IdentityRegistry,
            dagdb_gate::{
                context_packet_record_payload_hash, continuation_record_payload_hash,
                default_route_payload_hash, lifecycle_action_payload_hash,
            },
            sign_write_payload,
        };
        use sqlx::postgres::PgPoolOptions;

        use super::*;

        #[tokio::test]
        async fn bind_requester_to_session_actor_rejects_self_asserted_requester() {
            // A session authenticated as one actor must not be able to drive an
            // import/export attributed to a different, self-asserted requester_did.
            let session_actor =
                DagDbSessionActor::Authenticated("did:exo:session-owner".to_owned());
            let denied = bind_requester_to_session_actor(
                &session_actor,
                "dagdb.import",
                "tenant-a",
                "did:exo:other-principal",
            )
            .expect_err("mismatched requester must be rejected");
            assert_error_response(*denied, StatusCode::FORBIDDEN, "requester_actor_mismatch").await;
        }

        #[test]
        fn bind_requester_to_session_actor_accepts_matching_requester() {
            let session_actor =
                DagDbSessionActor::Authenticated("did:exo:session-owner".to_owned());
            assert!(
                bind_requester_to_session_actor(
                    &session_actor,
                    "dagdb.import",
                    "tenant-a",
                    "did:exo:session-owner",
                )
                .is_ok(),
                "requester matching the session actor must be authorized"
            );
        }

        #[test]
        fn bind_requester_to_session_actor_no_pool_skips_actor_binding() {
            // Without a pool there is no persisted session identity to bind
            // against; route handlers still fail closed before claim success.
            assert!(
                bind_requester_to_session_actor(
                    &DagDbSessionActor::NoPool,
                    "dagdb.import",
                    "tenant-a",
                    "did:exo:anyone",
                )
                .is_ok(),
                "no-pool path must not block on requester binding"
            );
        }

        #[tokio::test]
        async fn install_gatekeeper_profile_wires_gatekeeper_service() {
            // An explicitly-installed profile short-circuits the DB resolver, so
            // an unreachable lazy pool is never queried.
            let ctx = DagDbRouteContext::from_pool(None);
            ctx.install_gatekeeper_profile(ConsentEngine::default(), IdentityRegistry::default());
            let pool = PgPoolOptions::new()
                .acquire_timeout(Duration::from_millis(50))
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let _service = ctx
                .gatekeeper_service(&pool, "did:exo:agent", "tenant-a")
                .await
                .expect("installed profile resolves without touching the DB");
        }

        #[tokio::test]
        async fn gatekeeper_service_resolver_unconfigured_fails_closed() {
            // No profile installed and an unreachable pool: the DB resolver must
            // fail closed with AuthorityResolverUnavailable, never an empty
            // registry. The resolver's first DB touch is the trusted-clock query.
            let ctx = DagDbRouteContext::from_pool(None);
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let result = ctx
                .gatekeeper_service(&pool, "did:exo:agent", "tenant-a")
                .await;
            assert!(matches!(
                result,
                Err(GatekeeperError::AuthorityResolverUnavailable(_))
            ));
        }

        /// T1 release-posture regression. Compiles ONLY under `--release`
        /// (`debug_assertions` off) — exactly how CI runs the suite
        /// (`cargo test --workspace --release`). It proves the shipping binary
        /// has no fabricated dev authorization and no deterministic-key signing:
        ///
        /// * The dev gatekeeper profile installer is gated out: a release-config
        ///   build never installs the dev DID / dev consent. We assert no profile
        ///   is present after a bare `from_pool`, the only constructor available.
        /// * With no installed profile, the write gate authorizes solely via the
        ///   live DB resolver and fails closed with the typed
        ///   `AuthorityResolverUnavailable` against an unreachable pool — it never
        ///   falls back to a `[0..31]` deterministic key or an empty registry.
        ///
        /// The deterministic-seed loader and `install_local_dev_gatekeeper_*`
        /// are `#[cfg(debug_assertions)]` and do not exist in this configuration;
        /// the debug-only tests in the parent module exercise them.
        #[cfg(not(debug_assertions))]
        #[tokio::test]
        async fn release_build_has_no_dev_gatekeeper_or_deterministic_seed() {
            let ctx = DagDbRouteContext::from_pool(None);
            assert!(
                ctx.installed_gatekeeper_profile().is_none(),
                "release build must not install a fabricated dev gatekeeper profile"
            );
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let result = ctx
                .gatekeeper_service(&pool, "did:exo:agent", "tenant-a")
                .await;
            assert!(
                matches!(
                    result,
                    Err(GatekeeperError::AuthorityResolverUnavailable(_))
                ),
                "release build must fail closed with a typed error, never a deterministic-key signer"
            );
        }

        #[test]
        fn dagdb_install_gatekeeper_profile_ignores_poisoned_lock() {
            let ctx = DagDbRouteContext::from_pool(None);
            let gatekeeper = ctx.gatekeeper.clone();
            let _ = std::thread::spawn(move || {
                let _guard = gatekeeper.write().expect("write lock");
                panic!("poison gatekeeper profile for coverage");
            })
            .join();

            ctx.install_gatekeeper_profile(ConsentEngine::default(), IdentityRegistry::default());
        }

        #[test]
        fn resolve_route_context_uses_extension_when_override_unset() {
            let ctx = Arc::new(DagDbRouteContext::from_pool(None));
            let resolved = resolve_route_context(Some(Extension(ctx.clone())));
            assert!(resolved.pool.is_none());
        }

        #[test]
        fn resolve_route_context_falls_back_to_empty_context() {
            let resolved = resolve_route_context(None);
            assert!(resolved.pool.is_none());
        }

        #[test]
        fn resolve_route_context_prefers_integration_override() {
            let ctx = Arc::new(DagDbRouteContext::from_pool(None));
            set_route_context_for_integration_tests(ctx);
            let resolved = resolve_route_context(Some(Extension(Arc::new(
                DagDbRouteContext::from_pool(None),
            ))));
            assert!(Arc::ptr_eq(
                &resolved,
                ROUTE_CONTEXT_OVERRIDE.get().expect("override")
            ));
        }

        #[tokio::test]
        async fn handle_dagdb_council_decision_no_pool_precedes_build_errors() {
            let mut request = council_decision_request();
            request.expires_at = "not-an-hlc".to_owned();
            let response = handle_dagdb_council_decision(
                None,
                authorized_headers("dagdb:council_decision"),
                Json(request),
            )
            .await;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }

        #[tokio::test]
        async fn context_packet_handler_maps_database_failures_closed() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            pool.close().await;
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let request = DagDbContextPacketRequest {
                tenant_id: "tenant-a".to_owned(),
                namespace: "primary".to_owned(),
                idempotency_key: "idem-packet-db-error".to_owned(),
                request_id: "request-db-error".to_owned(),
                route_id: Hash256::digest(b"route").to_string(),
                task_hash: Hash256::digest(b"task").to_string(),
                requesting_agent_did: "did:exo:agent".to_owned(),
                token_budget: 2048,
                force_revalidate: None,
                max_memory_refs: None,
                task: None,
                layered_mode: None,
                max_layer_depth: None,
                require_layer_evidence: None,
                drilldown_reserve_bp: None,
            };
            let response = context_packet_handler(&ctx, request).await;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }

        #[test]
        fn context_packet_response_from_persistent_marks_empty_and_database_modes() {
            let request = context_packet_request("packet-mode-request");

            let empty = persistent_context_packet(Vec::new());
            let empty_response =
                context_packet_response_from_persistent(&request, &empty).expect("empty packet");
            assert_eq!(
                empty_response.context_packet_mode.as_deref(),
                Some("empty_selection")
            );
            assert!(empty_response.selection_warning.is_some());

            let selected_ref = selected_context_ref();
            let selected_receipt_hash = Hash256::digest(b"selected memory receipt").to_string();
            let selected = persistent_context_packet_with_receipts(
                vec![selected_ref.clone()],
                BTreeMap::from([(
                    selected_ref.memory_id.clone(),
                    selected_receipt_hash.clone(),
                )]),
            );
            let selected_response = context_packet_response_from_persistent(&request, &selected)
                .expect("selected packet");
            assert_eq!(
                selected_response.context_packet_mode.as_deref(),
                Some("database")
            );
            assert_eq!(
                selected_response.validation_status,
                ValidationStatus::Passed
            );
            assert_eq!(selected_response.selection_warning, None);
            assert_eq!(selected_response.memory_refs.len(), 1);
            assert_eq!(
                selected_response.memory_refs[0].latest_receipt_hash,
                selected_receipt_hash
            );

            let missing_receipt = persistent_context_packet_with_receipts(
                vec![selected_context_ref()],
                BTreeMap::new(),
            );
            let missing_receipt_response =
                context_packet_response_from_persistent(&request, &missing_receipt)
                    .expect("missing receipt packet");
            assert_eq!(
                missing_receipt_response.validation_status,
                ValidationStatus::Failed
            );
            assert_eq!(
                missing_receipt_response.selection_warning.as_deref(),
                Some(
                    "selected memory reference receipt hash unavailable; validation failed closed"
                )
            );
            assert_eq!(missing_receipt_response.memory_refs.len(), 1);
            assert_eq!(
                missing_receipt_response.memory_refs[0].latest_receipt_hash,
                Hash256::ZERO.to_string()
            );

            let required_request = DagDbContextPacketRequest {
                layered_mode: Some("required".to_owned()),
                max_layer_depth: Some(3),
                require_layer_evidence: Some(true),
                ..request.clone()
            };
            assert!(
                context_packet_response_from_persistent(&required_request, &selected).is_err(),
                "flat persistent refs must not satisfy required layered evidence"
            );

            let auto_request = DagDbContextPacketRequest {
                layered_mode: Some("auto".to_owned()),
                max_layer_depth: Some(3),
                require_layer_evidence: Some(false),
                ..request
            };
            let auto_response = context_packet_response_from_persistent(&auto_request, &selected)
                .expect("auto packet");
            assert_eq!(
                auto_response.layered_status.as_deref(),
                Some("flat_fallback_no_layer_evidence")
            );
            assert_eq!(auto_response.flat_fallback_used, Some(true));
        }

        #[tokio::test]
        async fn writeback_handler_requires_write_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let mut headers = HeaderMap::new();
            headers.insert(
                header::AUTHORIZATION,
                HeaderValue::from_static("Bearer test"),
            );
            let request = fixture_writeback_request();
            let response = writeback_handler(&ctx, &headers, request).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn route_handler_requires_default_route_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let fixtures = fixtures();
            let request: DagDbRouteRequest = fixture(&fixtures, "requests", "route");
            let response = route_handler(&ctx, &authorized_headers("dagdb:route"), request).await;
            assert_error_response(
                response,
                StatusCode::BAD_REQUEST,
                "write_signature_required",
            )
            .await;
        }

        #[tokio::test]
        async fn route_handler_persists_default_route_or_fails_closed_at_db_layer() {
            let keypair = KeyPair::generate();
            let fixtures = fixtures();
            let request: DagDbRouteRequest = fixture(&fixtures, "requests", "route");
            let response =
                route_response_from_request(request.clone(), "dagdb.route").expect("route shape");
            let record = default_route_record_from_response(&request, &response)
                .expect("default route record");
            let signature = sign_write_payload(
                &keypair,
                &default_route_payload_hash(&record).expect("default route payload hash"),
            )
            .expect("default route signature");
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            ctx.install_gatekeeper_profile(
                consent_engine_for_agent(&request.tenant_id, &request.requesting_agent_did),
                identity_registry_for_agent(&request.requesting_agent_did, &keypair),
            );
            let mut headers = authorized_headers("dagdb:route");
            headers.insert(
                WRITE_SIGNATURE_HEADER,
                HeaderValue::from_str(&signature).expect("signature header"),
            );
            let response = route_handler(&ctx, &headers, request).await;
            assert_error_response(
                response,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
            )
            .await;
        }

        #[tokio::test]
        async fn context_packet_handler_requires_record_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let response = gated_context_packet_handler(
                &ctx,
                &authorized_headers("dagdb:context_packet"),
                context_packet_request("missing-context-signature"),
            )
            .await;
            assert_error_response(
                response,
                StatusCode::BAD_REQUEST,
                "write_signature_required",
            )
            .await;
        }

        #[tokio::test]
        async fn gateway_context_packet_record_reaches_durable_persistence_layer() {
            let keypair = KeyPair::generate();
            let request = context_packet_request("context-d5-persist");
            let selected_ref = selected_context_ref();
            let persistent = persistent_context_packet(vec![selected_ref]);
            let response = context_packet_response_from_persistent(&request, &persistent)
                .expect("persistent context response");
            let record = context_packet_record_from_response(&request, &response)
                .expect("context packet record");
            let signature = sign_write_payload(
                &keypair,
                &context_packet_record_payload_hash(&record).expect("context record payload hash"),
            )
            .expect("context packet signature");
            let service = consented_service_for_agent(
                &keypair,
                &request.tenant_id,
                &request.requesting_agent_did,
            );
            let invariant_context =
                service.dagdb_invariant_context(&request.tenant_id, &request.requesting_agent_did);
            let error = service
                .persist_context_packet_record(
                    &record,
                    &request.requesting_agent_did,
                    &signature,
                    invariant_context.as_ref(),
                )
                .await
                .expect_err("unreachable pool fails at context packet DB layer");
            let handler_error = DagDbHandlerError::from_gatekeeper(error);
            assert_eq!(handler_error.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(handler_error.error_code(), "database_unavailable");
        }

        #[tokio::test]
        async fn writeback_handler_requires_d5_lifecycle_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let mut headers = authorized_headers("dagdb:writeback");
            headers.insert(WRITE_SIGNATURE_HEADER, HeaderValue::from_static("00"));
            let response = writeback_handler(&ctx, &headers, fixture_writeback_request()).await;
            assert_error_response(
                response,
                StatusCode::BAD_REQUEST,
                "lifecycle_signature_required",
            )
            .await;
        }

        #[tokio::test]
        async fn writeback_handler_requires_d5_continuation_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let mut headers = authorized_headers("dagdb:writeback");
            headers.insert(WRITE_SIGNATURE_HEADER, HeaderValue::from_static("00"));
            headers.insert(LIFECYCLE_SIGNATURE_HEADER, HeaderValue::from_static("00"));
            let response = writeback_handler(&ctx, &headers, fixture_writeback_request()).await;
            assert_error_response(
                response,
                StatusCode::BAD_REQUEST,
                "continuation_signature_required",
            )
            .await;
        }

        #[tokio::test]
        async fn gateway_writeback_lifecycle_and_continuation_records_reach_db_layer() {
            let keypair = KeyPair::generate();
            let request = fixture_writeback_request();
            let service = consented_service_for_agent(
                &keypair,
                &request.tenant_id,
                &request.requesting_agent_did,
            );

            let lifecycle = lifecycle_action_from_writeback(&request).expect("lifecycle action");
            let lifecycle_signature = sign_write_payload(
                &keypair,
                &lifecycle_action_payload_hash(&lifecycle).expect("lifecycle payload hash"),
            )
            .expect("lifecycle signature");
            let invariant_context =
                service.dagdb_invariant_context(&request.tenant_id, &request.requesting_agent_did);
            let lifecycle_error = service
                .persist_lifecycle_action(
                    &lifecycle,
                    &request.requesting_agent_did,
                    &lifecycle_signature,
                    invariant_context.as_ref(),
                )
                .await
                .expect_err("unreachable pool fails at lifecycle DB layer");
            let lifecycle_handler_error = DagDbHandlerError::from_gatekeeper(lifecycle_error);
            assert_eq!(
                lifecycle_handler_error.status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
            assert_eq!(lifecycle_handler_error.error_code(), "database_unavailable");

            let continuation =
                continuation_record_from_writeback(&request).expect("continuation record");
            let continuation_signature = sign_write_payload(
                &keypair,
                &continuation_record_payload_hash(&continuation)
                    .expect("continuation payload hash"),
            )
            .expect("continuation signature");
            let continuation_error = service
                .persist_continuation_record(
                    &continuation,
                    1,
                    &request.requesting_agent_did,
                    &continuation_signature,
                    invariant_context.as_ref(),
                )
                .await
                .expect_err("unreachable pool fails at continuation DB layer");
            let continuation_handler_error = DagDbHandlerError::from_gatekeeper(continuation_error);
            assert_eq!(
                continuation_handler_error.status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
            assert_eq!(
                continuation_handler_error.error_code(),
                "database_unavailable"
            );
        }

        #[tokio::test]
        async fn writeback_d5_preflight_rejects_bad_lifecycle_signature_before_persistence() {
            let keypair = KeyPair::generate();
            let request = fixture_writeback_request();
            let service = consented_service_for_agent(
                &keypair,
                &request.tenant_id,
                &request.requesting_agent_did,
            );
            let selection = fixture_writeback_selection_response(&request);
            let lifecycle = lifecycle_action_from_writeback(&request).expect("lifecycle action");
            let continuation =
                continuation_record_from_writeback(&request).expect("continuation record");
            let (writeback_signature, _, continuation_signature) =
                writeback_preflight_signatures(&keypair, &selection, &lifecycle, &continuation);

            let error = prevalidate_writeback_d5_gates(
                &service,
                &selection,
                &request.tenant_id,
                &request.requesting_agent_did,
                &lifecycle,
                &continuation,
                &writeback_signature,
                &"00".repeat(64),
                &continuation_signature,
            )
            .expect_err("forged lifecycle signature must fail in preflight");

            assert_eq!(error.status(), StatusCode::FORBIDDEN);
            assert_eq!(error.error_code(), "provenance_denied");
        }

        #[tokio::test]
        async fn writeback_d5_preflight_rejects_bad_continuation_signature_before_persistence() {
            let keypair = KeyPair::generate();
            let request = fixture_writeback_request();
            let service = consented_service_for_agent(
                &keypair,
                &request.tenant_id,
                &request.requesting_agent_did,
            );
            let selection = fixture_writeback_selection_response(&request);
            let lifecycle = lifecycle_action_from_writeback(&request).expect("lifecycle action");
            let continuation =
                continuation_record_from_writeback(&request).expect("continuation record");
            let (writeback_signature, lifecycle_signature, _) =
                writeback_preflight_signatures(&keypair, &selection, &lifecycle, &continuation);

            let error = prevalidate_writeback_d5_gates(
                &service,
                &selection,
                &request.tenant_id,
                &request.requesting_agent_did,
                &lifecycle,
                &continuation,
                &writeback_signature,
                &lifecycle_signature,
                &"00".repeat(64),
            )
            .expect_err("forged continuation signature must fail in preflight");

            assert_eq!(error.status(), StatusCode::FORBIDDEN);
            assert_eq!(error.error_code(), "provenance_denied");
        }

        #[tokio::test]
        async fn import_handler_requires_write_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let response =
                import_handler(&ctx, &authorized_headers("dagdb:import"), import_request()).await;
            assert_error_response(
                response,
                StatusCode::BAD_REQUEST,
                "write_signature_required",
            )
            .await;
        }

        #[tokio::test]
        async fn export_handler_requires_write_signature_when_pool_present() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let ctx = DagDbRouteContext::from_pool(Some(pool));
            let response =
                export_handler(&ctx, &authorized_headers("dagdb:export"), export_request()).await;
            assert_error_response(
                response,
                StatusCode::BAD_REQUEST,
                "write_signature_required",
            )
            .await;
        }

        #[test]
        fn import_adapter_failures_are_classified_without_collapsing_runtime_errors() {
            let validation = KgImportPersistenceError::Report(KgImportError::InvalidReport {
                reason: "bad shape".to_owned(),
            });
            assert_eq!(
                import_adapter_failure(&validation),
                AdapterFailure {
                    status: StatusCode::BAD_REQUEST,
                    error_code: "import_rejected",
                    class: "validation",
                    message: "DAG DB import request was rejected by the import adapter",
                }
            );

            let postgres = KgImportPersistenceError::Postgres {
                source: sqlx::Error::RowNotFound,
            };
            assert_eq!(
                import_adapter_failure(&postgres),
                AdapterFailure {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    error_code: "database_unavailable",
                    class: "postgres",
                    message: "DAG DB import adapter is temporarily unavailable",
                }
            );

            let conflict = KgImportPersistenceError::Conflict {
                reason: "existing row differs".to_owned(),
            };
            assert_eq!(import_adapter_failure(&conflict).class, "conflict");

            let unsupported = KgImportPersistenceError::UnsupportedSection {
                section: "raw_artifact".to_owned(),
            };
            assert_eq!(import_adapter_failure(&unsupported).class, "unsupported");
        }

        #[test]
        fn export_adapter_failures_are_classified_without_collapsing_runtime_errors() {
            let validation = KgExportError::ImportHash(KgImportError::InvalidHash {
                field: "memory_id".to_owned(),
            });
            assert_eq!(
                export_adapter_failure(&validation),
                AdapterFailure {
                    status: StatusCode::BAD_REQUEST,
                    error_code: "export_rejected",
                    class: "validation",
                    message: "DAG DB export request was rejected by the export adapter",
                }
            );

            let postgres = KgExportError::Postgres {
                source: Box::new(sqlx::Error::RowNotFound),
            };
            assert_eq!(
                export_adapter_failure(&postgres),
                AdapterFailure {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    error_code: "database_unavailable",
                    class: "postgres",
                    message: "DAG DB export adapter is temporarily unavailable",
                }
            );

            let conflict = KgExportError::Conflict {
                reason: "stored export differs".to_owned(),
            };
            assert_eq!(export_adapter_failure(&conflict).class, "conflict");

            let incompatible_cached = KgExportError::IncompatibleCachedResponse {
                route_name: "dagdb.kg_export.persisted.v1".to_owned(),
                reason: "cached schema_version mismatch".to_owned(),
            };
            assert_eq!(
                export_adapter_failure(&incompatible_cached),
                AdapterFailure {
                    status: StatusCode::CONFLICT,
                    error_code: "export_rejected",
                    class: "conflict",
                    message: "DAG DB export request conflicted with existing adapter state",
                }
            );

            let unsupported = KgExportError::UnsupportedPersistenceTarget {
                target: "raw_artifact".to_owned(),
            };
            assert_eq!(export_adapter_failure(&unsupported).class, "unsupported");
        }

        #[tokio::test]
        async fn adapter_error_responses_keep_validation_400_and_runtime_503() {
            assert_error_response(
                dagdb_import_adapter_error_response(
                    &import_request(),
                    &KgImportPersistenceError::Report(KgImportError::InvalidJson {
                        reason: "json".to_owned(),
                    }),
                ),
                StatusCode::BAD_REQUEST,
                "import_rejected",
            )
            .await;
            assert_error_response(
                dagdb_import_adapter_error_response(
                    &import_request(),
                    &KgImportPersistenceError::Postgres {
                        source: sqlx::Error::RowNotFound,
                    },
                ),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
            )
            .await;
            assert_error_response(
                dagdb_export_adapter_error_response(
                    &export_request(),
                    &KgExportError::InvalidScope {
                        reason: "scope".to_owned(),
                    },
                ),
                StatusCode::BAD_REQUEST,
                "export_rejected",
            )
            .await;
            assert_error_response(
                dagdb_export_adapter_error_response(
                    &export_request(),
                    &KgExportError::Postgres {
                        source: Box::new(sqlx::Error::RowNotFound),
                    },
                ),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
            )
            .await;
        }

        #[tokio::test]
        async fn dagdb_handler_error_mappings_cover_domain_gatekeeper_and_response_paths() {
            let scope_error = DagDbHandlerError::from_domain(DomainError::TenantScopeMismatch {
                expected_tenant_id: "tenant-a".to_owned(),
                expected_namespace: "primary".to_owned(),
                actual_tenant_id: "tenant-b".to_owned(),
                actual_namespace: "primary".to_owned(),
            });
            assert_eq!(scope_error.status(), StatusCode::FORBIDDEN);
            assert_eq!(scope_error.error_code(), "tenant_scope_mismatch");

            let metadata_error = exo_dag_db_core::metadata::sanitize_runtime_metadata(
                exo_dag_db_core::metadata::MetadataField::Summary,
                "fn raw_payload() {}",
            )
            .expect_err("metadata rejected");
            let metadata_error =
                DagDbHandlerError::from_domain(DomainError::Metadata(metadata_error));
            assert_eq!(metadata_error.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(metadata_error.error_code(), "metadata_rejected");

            let db_error = DagDbHandlerError::from_domain(DomainError::HashMaterial {
                reason: "postgres unavailable".to_owned(),
            });
            assert_eq!(db_error.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(db_error.error_code(), "database_unavailable");

            let validation_error = DagDbHandlerError::from_domain(DomainError::ValidationFailed);
            assert_eq!(validation_error.status(), StatusCode::BAD_REQUEST);
            assert_eq!(validation_error.error_code(), "invalid_request_shape");

            let consent_error = DagDbHandlerError::from_gatekeeper(
                GatekeeperError::InvariantViolation("ConsentRequired".to_owned()),
            );
            assert_eq!(consent_error.status(), StatusCode::FORBIDDEN);
            assert_eq!(consent_error.error_code(), "consent_denied");
            assert_eq!(consent_error.class(), "consent");

            let provenance_error = DagDbHandlerError::from_gatekeeper(
                GatekeeperError::InvariantViolation("ProvenanceVerifiable".to_owned()),
            );
            assert_eq!(provenance_error.status(), StatusCode::FORBIDDEN);
            assert_eq!(provenance_error.error_code(), "provenance_denied");
            assert_eq!(provenance_error.class(), "provenance");

            let gatekeeper_db_error =
                DagDbHandlerError::from_gatekeeper(GatekeeperError::InvariantViolation(
                    "dagdb write blocked: hash_material_failed: graph_context_selection_write_postgres: connection refused".to_owned(),
                ));
            assert_eq!(
                gatekeeper_db_error.status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
            assert_eq!(gatekeeper_db_error.error_code(), "database_unavailable");
            assert_eq!(gatekeeper_db_error.class(), "database");

            let denied_error = DagDbHandlerError::from_gatekeeper(
                GatekeeperError::InvariantViolation("writeback denied".to_owned()),
            );
            assert_eq!(denied_error.status(), StatusCode::FORBIDDEN);
            assert_eq!(denied_error.error_code(), "writeback_denied");
            assert_eq!(denied_error.class(), "invariant");

            let core_error =
                DagDbHandlerError::from_gatekeeper(GatekeeperError::Core("raw secret".to_owned()));
            assert_eq!(core_error.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(core_error.error_code(), "database_unavailable");
            assert_eq!(core_error.class(), "runtime");

            let tee_error =
                DagDbHandlerError::from_gatekeeper(GatekeeperError::TeeError("raw tee".to_owned()));
            assert_eq!(tee_error.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(tee_error.error_code(), "database_unavailable");
            assert_eq!(tee_error.class(), "runtime");
            assert_error_response_shape(
                tee_error.into_response(),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB database operation failed",
                false,
            )
            .await;

            let metadata_response = dagdb_error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "metadata_rejected",
                "metadata rejected",
                true,
            );
            let response_error = DagDbHandlerError::from_response(metadata_response);
            assert_eq!(response_error.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(response_error.error_code(), "metadata_rejected");

            let shape_response = dagdb_error_response(
                StatusCode::BAD_REQUEST,
                "invalid_request_shape",
                "invalid request",
                false,
            );
            let shape_error = DagDbHandlerError::from_response(shape_response);
            assert_eq!(shape_error.status(), StatusCode::BAD_REQUEST);
            assert_eq!(shape_error.error_code(), "invalid_request_shape");
        }

        #[tokio::test]
        async fn gatekeeper_responses_use_static_sanitized_messages() {
            let raw_did = "did:exo:raw-requester";
            let raw_reason = "raw actor text and database internals";
            let consent = DagDbHandlerError::from_gatekeeper(GatekeeperError::InvariantViolation(
                format!("ConsentRequired: tenant=tenant-a actor={raw_did} reason={raw_reason}"),
            ));
            assert_error_response_shape(
                consent.into_response(),
                StatusCode::FORBIDDEN,
                "consent_denied",
                "DAG DB writeback consent was denied",
                true,
            )
            .await;

            let provenance =
                DagDbHandlerError::from_gatekeeper(GatekeeperError::InvariantViolation(format!(
                    "ProvenanceVerifiable: tenant=tenant-a actor={raw_did} reason={raw_reason}"
                )));
            assert_error_response_shape(
                provenance.into_response(),
                StatusCode::FORBIDDEN,
                "provenance_denied",
                "DAG DB writeback provenance could not be verified",
                true,
            )
            .await;

            let denied = DagDbHandlerError::from_gatekeeper(GatekeeperError::CapabilityDenied(
                format!("actor={raw_did} reason={raw_reason}"),
            ));
            let response = denied.into_response();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body bytes");
            let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
            let envelope: exo_api::dagdb::DagDbErrorEnvelope =
                serde_json::from_str(&body).expect("DAG DB error envelope");
            assert_eq!(envelope.error_code, "writeback_denied");
            assert_eq!(envelope.message, "DAG DB writeback was denied");
            assert!(!body.contains(raw_did));
            assert!(!body.contains(raw_reason));
        }

        #[tokio::test]
        async fn gatekeeper_runtime_database_failures_map_to_503() {
            let raw_db_error = "password authentication failed for user did:exo:raw-db-user";
            let db_error = DagDbHandlerError::from_gatekeeper(GatekeeperError::InvariantViolation(
                format!(
                    "dagdb write blocked: hash_material_failed: graph_context_selection_write_postgres: {raw_db_error}"
                ),
            ));
            assert_error_response_shape(
                db_error.into_response(),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB database operation failed",
                false,
            )
            .await;

            let timeout = DagDbHandlerError::from_gatekeeper(GatekeeperError::Timeout(500));
            assert_error_response_shape(
                timeout.into_response(),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB database operation failed",
                false,
            )
            .await;
        }

        #[test]
        fn gatekeeper_database_failure_classifier_requires_dagdb_database_context() {
            assert!(is_gatekeeper_database_failure(
                "dagdb write blocked: hash_material_failed"
            ));
            assert!(is_gatekeeper_database_failure(
                "dagdb write blocked: graph_context_selection_write_postgres"
            ));
            assert!(!is_gatekeeper_database_failure("hash_material_failed"));
            assert!(!is_gatekeeper_database_failure(
                "dagdb write blocked: consent denied"
            ));
            // PRD-D5: the four lifecycle/persistence surfaces classify a backing
            // transaction outage as 503 via this marker, while a contract reject
            // (carrying the `metadata rejected` marker, mapped to 422 earlier in
            // GatekeeperFailure::from_error) is not a database failure here.
            assert!(is_gatekeeper_database_failure(
                "dagdb write blocked: surface_database_unavailable surface=lifecycle_action_postgres detail=prd17_lifecycle_postgres_failed"
            ));
            assert!(!is_gatekeeper_database_failure(
                "dagdb write blocked: metadata rejected surface=context_packet_record_postgres detail=context_packet_unsafe_replay: packet-d5-001"
            ));
        }

        #[test]
        fn runtime_import_export_request_hashes_are_deterministic_and_scope_sensitive() {
            let import = import_request();
            let import_hash =
                import_route_request_hash(&import).expect("first import request hash");
            assert_eq!(
                import_hash,
                import_route_request_hash(&import).expect("second import request hash")
            );
            let changed_import_hash = import_route_request_hash(&DagDbImportRequest {
                idempotency_key: "idem-import-2".to_owned(),
                ..import
            })
            .expect("changed import request hash");
            assert_ne!(import_hash, changed_import_hash);

            let export = export_request();
            let export_hash =
                export_route_request_hash(&export).expect("first export request hash");
            assert_eq!(
                export_hash,
                export_route_request_hash(&export).expect("second export request hash")
            );
            let changed_export_hash = export_route_request_hash(&DagDbExportRequest {
                include_preview_context: true,
                ..export
            })
            .expect("changed export request hash");
            assert_ne!(export_hash, changed_export_hash);
        }

        #[test]
        fn idempotency_response_hash_uses_canonical_cbor_deterministically() {
            let left = json!({
                "route_name": "dagdb.import",
                "idempotency_status": RESERVED_IDEMPOTENCY_BODY_STATUS,
            });
            let mut right_fields = serde_json::Map::new();
            right_fields.insert(
                "idempotency_status".to_owned(),
                json!(RESERVED_IDEMPOTENCY_BODY_STATUS),
            );
            right_fields.insert("route_name".to_owned(), json!("dagdb.import"));
            let right = Value::Object(right_fields);

            let left_hash = gateway_idempotency_response_hash(
                &left,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                "reserve",
                "import",
                "tenant-a",
                "primary",
                "idem-response-1",
            )
            .expect("left response hash");
            let right_hash = gateway_idempotency_response_hash(
                &right,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                "reserve",
                "import",
                "tenant-a",
                "primary",
                "idem-response-1",
            )
            .expect("right response hash");
            assert_eq!(left_hash, right_hash);

            let changed = json!({
                "route_name": "dagdb.export",
                "idempotency_status": RESERVED_IDEMPOTENCY_BODY_STATUS,
            });
            let changed_hash = gateway_idempotency_response_hash(
                &changed,
                EXPORT_ROUTE_IDEMPOTENCY_NAME,
                "reserve",
                "export",
                "tenant-a",
                "primary",
                "idem-response-1",
            )
            .expect("changed response hash");
            assert_ne!(left_hash, changed_hash);
        }

        #[tokio::test]
        async fn idempotency_response_hash_encoding_failure_fails_closed() {
            let response = gateway_idempotency_response_hash(
                &FailingSerialize,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                "store",
                "import",
                "tenant-a",
                "primary",
                "idem-response-fail",
            )
            .expect_err("response hash encoding failure");
            assert_error_response_shape(
                *response,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;
        }

        #[test]
        fn production_response_hash_paths_do_not_hash_json_text() {
            let source = include_str!("dagdb.rs");
            let forbidden_patterns = [
                ["response_body", ".to_string()", ".as_bytes()"].concat(),
                ["Hash256::digest(", "response_body", ".to_string"].concat(),
            ];
            for forbidden in forbidden_patterns {
                assert!(
                    !source.contains(&forbidden),
                    "production response hash path must use canonical CBOR, found {forbidden}"
                );
            }
        }

        #[tokio::test]
        async fn idempotency_error_helpers_return_stable_envelopes() {
            assert_error_response_shape(
                *idempotency_conflict_response("import"),
                StatusCode::CONFLICT,
                "idempotency_key_conflict",
                "DAG DB import idempotency key was already used with a different request body",
                false,
            )
            .await;
            assert_error_response_shape(
                *idempotency_in_progress_response("export"),
                StatusCode::CONFLICT,
                "idempotency_key_in_progress",
                "DAG DB export idempotency key is currently being processed",
                false,
            )
            .await;
            assert_error_response_shape(
                *idempotency_unavailable_response("import"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;
        }

        #[tokio::test]
        async fn malformed_idempotency_row_hash_fails_closed() {
            let hash = Hash256::digest(b"row hash");
            assert_eq!(
                hash_from_idempotency_row(hash.as_bytes().to_vec(), "import")
                    .expect("valid row hash"),
                hash
            );
            let response =
                hash_from_idempotency_row(vec![0; 31], "export").expect_err("malformed row hash");
            assert_error_response_shape(
                *response,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;
        }

        #[tokio::test]
        async fn malformed_idempotency_row_status_fails_closed() {
            assert_eq!(
                status_from_idempotency_row(i32::from(StatusCode::OK.as_u16()), "import")
                    .expect("valid status"),
                StatusCode::OK
            );
            for status_code in [-1, 1000] {
                let response = status_from_idempotency_row(status_code, "export")
                    .expect_err("malformed row status");
                assert_error_response_shape(
                    *response,
                    StatusCode::SERVICE_UNAVAILABLE,
                    "database_unavailable",
                    "DAG DB export idempotency guard could not be checked",
                    false,
                )
                .await;
            }
        }

        #[tokio::test]
        async fn cached_authorization_payload_hash_helpers_fail_closed_and_strip_private_field() {
            let payload_hash = Hash256::digest(b"cached authorization payload");
            let mut cached = json!({
                "ok": true,
                GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD: payload_hash.to_string(),
            });
            assert_eq!(
                gateway_authorization_payload_hash_from_cached_body(
                    IMPORT_ROUTE_IDEMPOTENCY_NAME,
                    "import",
                    "tenant-a",
                    "primary",
                    "idem-import-cache",
                    &mut cached,
                )
                .expect("cached authorization hash")
                .expect("hash is present"),
                payload_hash
            );
            assert_eq!(cached, json!({"ok": true}));

            let mut non_string = json!({
                GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD: true,
            });
            assert_error_response_shape(
                *gateway_authorization_payload_hash_from_cached_body(
                    IMPORT_ROUTE_IDEMPOTENCY_NAME,
                    "import",
                    "tenant-a",
                    "primary",
                    "idem-import-cache",
                    &mut non_string,
                )
                .expect_err("non-string cached authorization hash fails closed"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;

            let mut invalid_hex = json!({
                GATEWAY_AUTHORIZATION_PAYLOAD_HASH_FIELD: "not-hex",
            });
            assert_error_response_shape(
                *gateway_authorization_payload_hash_from_cached_body(
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    "export",
                    "tenant-a",
                    "primary",
                    "idem-export-cache",
                    &mut invalid_hex,
                )
                .expect_err("invalid cached authorization hash fails closed"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;

            let mut response_body = json!("not-an-object");
            assert_error_response_shape(
                *insert_gateway_authorization_payload_hash(
                    &mut response_body,
                    Some(payload_hash),
                    IMPORT_ROUTE_IDEMPOTENCY_NAME,
                    "import",
                    "tenant-a",
                    "primary",
                    "idem-import-cache",
                )
                .expect_err("non-object response body fails closed"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;
        }

        #[tokio::test]
        async fn import_export_authorization_deny_writeback_consent_and_signature() {
            let keypair = KeyPair::generate();
            let payload_hash = Hash256::digest(b"writeback-authorized import/export payload");
            let signature =
                sign_write_payload(&keypair, payload_hash.as_bytes()).expect("write signature");
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            let service = DagDbGatekeeperService::new(
                pool.clone(),
                Arc::new(
                    ConsentEngine::default()
                        .with_bailment(
                            "tenant-a",
                            BailmentState::Active {
                                bailor: exo_core::Did::new("did:exo:bailor")
                                    .expect("valid bailor did"),
                                bailee: exo_core::Did::new("did:exo:importer")
                                    .expect("valid bailee did"),
                                scope: "dag-db:writeback".to_owned(),
                            },
                        )
                        .with_consent_record(DagDbConsentRecord {
                            tenant_id: "tenant-a".to_owned(),
                            agent_did: "did:exo:importer".to_owned(),
                            purpose: ConsentPurpose::Writeback,
                            active: true,
                        })
                        .with_consent_record(DagDbConsentRecord {
                            tenant_id: "tenant-a".to_owned(),
                            agent_did: "did:exo:exporter".to_owned(),
                            purpose: ConsentPurpose::Writeback,
                            active: true,
                        }),
                ),
                Arc::new(
                    IdentityRegistry::default()
                        .with_public_key("did:exo:importer", *keypair.public_key().as_bytes())
                        .with_public_key("did:exo:exporter", *keypair.public_key().as_bytes()),
                ),
            );

            let import_denied = gated_import_authorization(
                &service,
                &pool,
                &import_request(),
                &signature,
                false,
                Some(payload_hash),
            )
            .await
            .expect_err("writeback consent must not authorize import");
            assert_eq!(import_denied.status(), StatusCode::FORBIDDEN);
            assert_eq!(import_denied.error_code(), "consent_denied");

            let export_by_bailee = DagDbExportRequest {
                requester_did: "did:exo:importer".to_owned(),
                ..export_request()
            };
            let export_denied = gated_export_authorization(
                &service,
                &pool,
                &export_by_bailee,
                Hash256::digest(b"unread export hash"),
                &signature,
                false,
                Some(payload_hash),
            )
            .await
            .expect_err("writeback consent must not authorize export");
            assert_eq!(export_denied.status(), StatusCode::FORBIDDEN);
            assert_eq!(export_denied.error_code(), "consent_denied");
        }

        #[test]
        fn runtime_import_response_counts_inserted_sections_and_non_claims() {
            let request = import_request();
            let response = import_response_from_summary(
                request,
                exo_dag_db_exchange::kg_import::KgImportPersistedSummary {
                    schema_version:
                        exo_dag_db_exchange::kg_import::KG_IMPORT_PERSISTED_SUMMARY_SCHEMA
                            .to_owned(),
                    tenant_id: "tenant-a".to_owned(),
                    namespace: "primary".to_owned(),
                    batch_id: "batch-a".to_owned(),
                    idempotency_key: "summary-idem-1".to_owned(),
                    replayed: false,
                    inserted_memory_count: 1,
                    inserted_catalog_count: 2,
                    inserted_graph_node_count: 3,
                    inserted_graph_edge_count: 4,
                    inserted_layer_count: 5,
                    inserted_layer_membership_count: 6,
                    inserted_layer_edge_count: 7,
                    inserted_validation_report_count: 8,
                    inserted_placement_decision_count: 9,
                    inserted_placement_trace_count: 10,
                    inserted_receipt_count: 11,
                    skipped_advisory_section_count: 12,
                },
                "persisted",
            )
            .expect("import response");

            assert_eq!(response.tenant_id, "tenant-a");
            assert_eq!(response.namespace, "primary");
            assert_eq!(response.import_status, "persisted");
            assert_eq!(
                response.import_receipt_id.as_deref(),
                Some("summary-idem-1")
            );
            // Sum of every inserted_* section count (1..=11); skipped advisory
            // sections are not imported records.
            assert_eq!(response.imported_record_count, 66);
            assert_eq!(response.non_claims, runtime_non_claims());
            assert!(response.operation_id.len() == 64);
        }

        #[test]
        fn runtime_export_response_counts_portable_sections_and_non_claims() {
            let request = export_request();
            let export = portable_export_with_counted_sections();
            let expected_export_id = export.export_id.clone();
            let expected_export_hash = export.hashes.whole_export_hash.clone();

            let response = export_response_from_portable(request, export).expect("export response");

            assert_eq!(response.tenant_id, "tenant-a");
            assert_eq!(response.namespace, "primary");
            assert_eq!(response.export_status, "built");
            assert_eq!(
                response.export_artifact_id.as_deref(),
                Some(expected_export_id.as_str())
            );
            assert_eq!(
                response.export_hash.as_deref(),
                Some(expected_export_hash.as_str())
            );
            assert_eq!(response.exported_record_count, 17);
            assert_eq!(response.non_claims, runtime_non_claims());
            assert!(response.operation_id.len() == 64);
        }

        #[test]
        fn runtime_adapter_failures_cover_runtime_classifier_branches() {
            let import_report_hash = KgImportPersistenceError::Report(KgImportError::Hash {
                reason: "hash failed".to_owned(),
            });
            assert_eq!(import_adapter_failure(&import_report_hash).class, "runtime");
            assert_eq!(
                import_adapter_failure(&KgImportPersistenceError::CountOutOfRange).class,
                "runtime"
            );

            let export_import_hash = KgExportError::ImportHash(KgImportError::Hash {
                reason: "hash failed".to_owned(),
            });
            assert_eq!(export_adapter_failure(&export_import_hash).class, "runtime");
            assert_eq!(
                export_adapter_failure(&KgExportError::CountOutOfRange).class,
                "runtime"
            );
        }

        #[tokio::test]
        async fn idempotency_db_error_and_short_circuit_paths_fail_closed() {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            pool.close().await;
            let request_hash = Hash256::digest(b"idempotency request");

            let reserve_error = match reserve_gateway_idempotency_key(
                &pool,
                "tenant-a",
                "primary",
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                "idem-import-1",
                request_hash,
                "import",
            )
            .await
            {
                Ok(_) => panic!("closed pool reserve should fail"),
                Err(response) => response,
            };
            assert_error_response_shape(
                *reserve_error,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;

            let replay_error = match replay_gateway_idempotency_response(
                &pool,
                "tenant-a",
                "primary",
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                "idem-import-1",
                request_hash,
                "import",
            )
            .await
            {
                Ok(_) => panic!("closed pool replay should fail"),
                Err(response) => response,
            };
            assert_error_response_shape(
                *replay_error,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;

            assert_error_response_shape(
                *store_gateway_idempotency_response(
                    &pool,
                    "tenant-a",
                    "primary",
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    "idem-export-1",
                    request_hash,
                    StatusCode::OK,
                    Err(export_idempotency_unavailable_response()),
                    None,
                    "export",
                )
                .await
                .expect_err("response body short-circuit"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;

            assert_error_response_shape(
                *store_gateway_idempotency_response(
                    &pool,
                    "tenant-a",
                    "primary",
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    "idem-export-1",
                    request_hash,
                    StatusCode::OK,
                    Ok(json!({"ok": true})),
                    None,
                    "export",
                )
                .await
                .expect_err("closed pool store should fail"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;

            assert!(
                delete_gateway_idempotency_reservation(
                    &pool,
                    "tenant-a",
                    "primary",
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    "idem-export-1",
                    request_hash,
                )
                .await
                .is_err()
            );

            let import_cleanup_error =
                cleanup_gateway_idempotency_reservation(&pool, &import_request(), request_hash)
                    .await
                    .expect_err("closed pool import cleanup should surface failure");
            assert_error_response_shape(
                *import_cleanup_error,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;

            let export_cleanup_error =
                cleanup_export_idempotency_reservation(&pool, &export_request(), request_hash)
                    .await
                    .expect_err("closed pool export cleanup should surface failure");
            assert_error_response_shape(
                *export_cleanup_error,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;
        }

        #[test]
        fn idempotency_cleanup_row_count_requires_exactly_one_removed() {
            assert!(!idempotency_reservation_cleanup_removed(0));
            assert!(idempotency_reservation_cleanup_removed(1));
            assert!(!idempotency_reservation_cleanup_removed(2));
        }

        #[tokio::test]
        #[allow(unexpected_cfgs)]
        async fn live_idempotency_replay_classifies_reserved_conflict_cached_and_bad_statuses() {
            #[cfg(coverage)]
            let pool = live_dagdb_pool().await.expect("live DAG DB pool");
            #[cfg(not(coverage))]
            let Some(pool) = live_dagdb_pool().await else {
                return;
            };
            let tenant_id = format!("tenant-coverage-{}", std::process::id());
            let namespace = "primary";
            delete_live_idempotency_rows(&pool, &tenant_id, namespace).await;

            let idempotency_key = "idem-live-classification";
            let request_hash = Hash256::digest(b"live idempotency request");
            let first = reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                request_hash,
                "import",
            )
            .await
            .expect("first reservation");
            assert!(matches!(first, GatewayIdempotencyDecision::Reserved));

            let in_progress = match reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                request_hash,
                "import",
            )
            .await
            {
                Ok(_) => panic!("reserved row should still be in progress"),
                Err(response) => response,
            };
            assert_error_response_shape(
                *in_progress,
                StatusCode::CONFLICT,
                "idempotency_key_in_progress",
                "DAG DB import idempotency key is currently being processed",
                false,
            )
            .await;

            let conflict = match reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                Hash256::digest(b"different live idempotency request"),
                "import",
            )
            .await
            {
                Ok(_) => panic!("different request hash should conflict"),
                Err(response) => response,
            };
            assert_error_response_shape(
                *conflict,
                StatusCode::CONFLICT,
                "idempotency_key_conflict",
                "DAG DB import idempotency key was already used with a different request body",
                false,
            )
            .await;

            store_gateway_idempotency_response(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                request_hash,
                StatusCode::OK,
                Ok(json!({"ok": true})),
                None,
                "import",
            )
            .await
            .expect("store cached response");
            let replay = reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                request_hash,
                "import",
            )
            .await
            .expect("cached response replays");
            assert!(matches!(replay, GatewayIdempotencyDecision::Replayed(_)));
            assert_eq!(idempotency_ref(idempotency_key).len(), 64);

            let live_ctx = DagDbRouteContext::from_pool(Some(pool.clone()));
            let packet_response = context_packet_handler(
                &live_ctx,
                DagDbContextPacketRequest {
                    tenant_id: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                    idempotency_key: "idem-live-context-packet".to_owned(),
                    ..context_packet_request("live-context-packet")
                },
            )
            .await;
            assert_eq!(packet_response.status(), StatusCode::OK);

            let export_denied_request = DagDbExportRequest {
                tenant_id: tenant_id.clone(),
                namespace: namespace.to_owned(),
                idempotency_key: "idem-live-export-gate-denied".to_owned(),
                ..export_request()
            };
            let export_denied_hash =
                export_route_request_hash(&export_denied_request).expect("export denied hash");
            let mut export_denied_headers = authorized_headers("dagdb:export");
            export_denied_headers.insert(
                WRITE_SIGNATURE_HEADER,
                HeaderValue::from_str(&"aa".repeat(64)).expect("signature header"),
            );
            let export_denied = export_handler(
                &live_ctx,
                &export_denied_headers,
                export_denied_request.clone(),
            )
            .await;
            assert_error_response(export_denied, StatusCode::FORBIDDEN, "consent_denied").await;
            assert!(matches!(
                reserve_gateway_idempotency_key(
                    &pool,
                    &tenant_id,
                    namespace,
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    &export_denied_request.idempotency_key,
                    export_denied_hash,
                    "export",
                )
                .await
                .expect("export gate denial cleaned reservation"),
                GatewayIdempotencyDecision::Reserved
            ));
            cleanup_export_idempotency_reservation(
                &pool,
                &export_denied_request,
                export_denied_hash,
            )
            .await
            .expect("cleanup export denial proof reservation");

            let export_response = export_handler(
                &live_ctx,
                &authorized_headers("dagdb:export"),
                DagDbExportRequest {
                    tenant_id: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                    idempotency_key: "idem-live-export-handler".to_owned(),
                    ..export_request()
                },
            )
            .await;
            assert_error_response(
                export_response,
                StatusCode::BAD_REQUEST,
                "write_signature_required",
            )
            .await;

            let cleanup_import_key = "idem-live-cleanup-import";
            let cleanup_import_hash = Hash256::digest(b"cleanup import reservation");
            assert!(matches!(
                reserve_gateway_idempotency_key(
                    &pool,
                    &tenant_id,
                    namespace,
                    IMPORT_ROUTE_IDEMPOTENCY_NAME,
                    cleanup_import_key,
                    cleanup_import_hash,
                    "import",
                )
                .await
                .expect("reserve import cleanup row"),
                GatewayIdempotencyDecision::Reserved
            ));
            cleanup_gateway_idempotency_reservation(
                &pool,
                &DagDbImportRequest {
                    idempotency_key: cleanup_import_key.to_owned(),
                    tenant_id: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                    ..import_request()
                },
                cleanup_import_hash,
            )
            .await
            .expect("import cleanup removed reservation");
            assert!(matches!(
                reserve_gateway_idempotency_key(
                    &pool,
                    &tenant_id,
                    namespace,
                    IMPORT_ROUTE_IDEMPOTENCY_NAME,
                    cleanup_import_key,
                    cleanup_import_hash,
                    "import",
                )
                .await
                .expect("import cleanup removed reservation"),
                GatewayIdempotencyDecision::Reserved
            ));

            let cleanup_export_key = "idem-live-cleanup-export";
            let cleanup_export_hash = Hash256::digest(b"cleanup export reservation");
            assert!(matches!(
                reserve_gateway_idempotency_key(
                    &pool,
                    &tenant_id,
                    namespace,
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    cleanup_export_key,
                    cleanup_export_hash,
                    "export",
                )
                .await
                .expect("reserve export cleanup row"),
                GatewayIdempotencyDecision::Reserved
            ));
            cleanup_export_idempotency_reservation(
                &pool,
                &DagDbExportRequest {
                    idempotency_key: cleanup_export_key.to_owned(),
                    tenant_id: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                    ..export_request()
                },
                cleanup_export_hash,
            )
            .await
            .expect("export cleanup removed reservation");
            assert!(matches!(
                reserve_gateway_idempotency_key(
                    &pool,
                    &tenant_id,
                    namespace,
                    EXPORT_ROUTE_IDEMPOTENCY_NAME,
                    cleanup_export_key,
                    cleanup_export_hash,
                    "export",
                )
                .await
                .expect("export cleanup removed reservation"),
                GatewayIdempotencyDecision::Reserved
            ));

            let stale_key = "idem-live-stale-store";
            let stale_hash = Hash256::digest(b"stale reservation");
            let stale = reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                stale_key,
                stale_hash,
                "import",
            )
            .await
            .expect("stale reservation");
            assert!(matches!(stale, GatewayIdempotencyDecision::Reserved));
            let stale_store = store_gateway_idempotency_response(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                stale_key,
                Hash256::digest(b"wrong stale reservation"),
                StatusCode::OK,
                Ok(json!({"ok": true})),
                None,
                "import",
            )
            .await
            .expect_err("stale reservation hash fails closed");
            assert_error_response_shape(
                *stale_store,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;

            let missing_cleanup_hash = Hash256::digest(b"missing cleanup reservation");
            let missing_import_cleanup = cleanup_gateway_idempotency_reservation(
                &pool,
                &DagDbImportRequest {
                    idempotency_key: "idem-live-missing-import-cleanup".to_owned(),
                    tenant_id: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                    ..import_request()
                },
                missing_cleanup_hash,
            )
            .await
            .expect_err("missing import cleanup should surface failure");
            assert_error_response_shape(
                *missing_import_cleanup,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;

            let missing_export_cleanup = cleanup_export_idempotency_reservation(
                &pool,
                &DagDbExportRequest {
                    idempotency_key: "idem-live-missing-export-cleanup".to_owned(),
                    tenant_id: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                    ..export_request()
                },
                missing_cleanup_hash,
            )
            .await
            .expect_err("missing export cleanup should surface failure");
            assert_error_response_shape(
                *missing_export_cleanup,
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;

            delete_live_idempotency_rows(&pool, &tenant_id, namespace).await;
        }

        #[tokio::test]
        #[allow(unexpected_cfgs)]
        async fn live_expired_reservation_is_reclaimed_on_retry() {
            use sqlx::Row as _;

            #[cfg(coverage)]
            let pool = live_dagdb_pool().await.expect("live DAG DB pool");
            #[cfg(not(coverage))]
            let Some(pool) = live_dagdb_pool().await else {
                return;
            };
            let tenant_id = format!("tenant-reclaim-{}", std::process::id());
            let namespace = "primary";
            delete_live_idempotency_rows(&pool, &tenant_id, namespace).await;

            // Simulate a crash between reserve and store: a reserved row whose
            // expiry is already in the past.
            let idempotency_key = "idem-live-expired-reclaim";
            let request_hash = Hash256::digest(b"expired reservation retry");
            insert_stale_live_reservation(
                &pool,
                &tenant_id,
                namespace,
                idempotency_key,
                request_hash,
            )
            .await;

            let retry = reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                request_hash,
                "import",
            )
            .await
            .expect("expired reservation must be reclaimable on retry");
            assert!(matches!(retry, GatewayIdempotencyDecision::Reserved));

            // The reclaimed reservation must carry real trusted-clock timestamps,
            // not the historical 1/86400001 placeholders.
            let mut tx = begin_tenant_transaction(&pool, &tenant_id)
                .await
                .expect("tenant-bound reclaimed reservation transaction");
            let row = sqlx::query(
                "SELECT created_at_physical_ms, expires_at_physical_ms FROM dagdb_idempotency_keys \
	         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3 AND idempotency_key = $4",
            )
            .bind(&tenant_id)
            .bind(namespace)
            .bind(IMPORT_ROUTE_IDEMPOTENCY_NAME)
            .bind(idempotency_key)
            .fetch_one(&mut *tx)
            .await
            .expect("fetch reclaimed reservation row");
            tx.commit()
                .await
                .expect("commit reclaimed reservation read");
            let created_at: i64 = row
                .try_get("created_at_physical_ms")
                .expect("created_at_physical_ms");
            let expires_at: i64 = row
                .try_get("expires_at_physical_ms")
                .expect("expires_at_physical_ms");
            assert!(
                created_at > 86_400_001,
                "reservation created_at must come from the trusted database clock, got {created_at}"
            );
            assert_eq!(
                expires_at,
                created_at + 86_400_000,
                "reservation expiry must be created_at plus the 24h TTL"
            );

            // A live (non-expired) reservation must still report in progress.
            let in_progress = match reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                idempotency_key,
                request_hash,
                "import",
            )
            .await
            {
                Ok(_) => panic!("non-expired reservation must stay in progress"),
                Err(response) => response,
            };
            assert_error_response_shape(
                *in_progress,
                StatusCode::CONFLICT,
                "idempotency_key_in_progress",
                "DAG DB import idempotency key is currently being processed",
                false,
            )
            .await;

            // An expired reservation under a different request hash must still
            // fail closed as a conflict instead of being reclaimed.
            let conflict_key = "idem-live-expired-conflict";
            insert_stale_live_reservation(
                &pool,
                &tenant_id,
                namespace,
                conflict_key,
                Hash256::digest(b"original expired request"),
            )
            .await;
            let conflict = match reserve_gateway_idempotency_key(
                &pool,
                &tenant_id,
                namespace,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                conflict_key,
                Hash256::digest(b"different expired request"),
                "import",
            )
            .await
            {
                Ok(_) => panic!("expired reservation with different request hash must conflict"),
                Err(response) => response,
            };
            assert_error_response_shape(
                *conflict,
                StatusCode::CONFLICT,
                "idempotency_key_conflict",
                "DAG DB import idempotency key was already used with a different request body",
                false,
            )
            .await;

            delete_live_idempotency_rows(&pool, &tenant_id, namespace).await;
        }

        async fn insert_stale_live_reservation(
            pool: &sqlx::PgPool,
            tenant_id: &str,
            namespace: &str,
            idempotency_key: &str,
            request_hash: Hash256,
        ) {
            let response_body = json!({
                "idempotency_status": RESERVED_IDEMPOTENCY_BODY_STATUS,
                "route_name": IMPORT_ROUTE_IDEMPOTENCY_NAME,
            });
            let response_hash = gateway_idempotency_response_hash(
                &response_body,
                IMPORT_ROUTE_IDEMPOTENCY_NAME,
                "reserve",
                "import",
                tenant_id,
                namespace,
                idempotency_key,
            )
            .expect("reserved response hash");
            let mut tx = begin_tenant_transaction(pool, tenant_id)
                .await
                .expect("tenant-bound stale reservation transaction");
            sqlx::query(
                "INSERT INTO dagdb_idempotency_keys \
	         (tenant_id, namespace, route_name, idempotency_key, request_hash, response_hash, \
	          response_body, status_code, cached_failure, created_at_physical_ms, \
                  created_at_logical, expires_at_physical_ms, expires_at_logical) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 202, false, 1, 0, 2, 0)",
            )
            .bind(tenant_id)
            .bind(namespace)
            .bind(IMPORT_ROUTE_IDEMPOTENCY_NAME)
            .bind(idempotency_key)
            .bind(request_hash.as_bytes().to_vec())
            .bind(response_hash.as_bytes().to_vec())
            .bind(response_body)
            .execute(&mut *tx)
            .await
            .expect("insert stale reserved row");
            tx.commit().await.expect("commit stale reserved row");
        }

        #[tokio::test]
        async fn idempotency_helper_wrappers_return_operation_specific_shapes() {
            assert_error_response_shape(
                *import_idempotency_unavailable_response(),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB import idempotency guard could not be checked",
                false,
            )
            .await;
            assert_error_response_shape(
                *export_idempotency_unavailable_response(),
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "DAG DB export idempotency guard could not be checked",
                false,
            )
            .await;
        }

        async fn live_dagdb_pool() -> Option<sqlx::PgPool> {
            let database_url = std::env::var("EXO_DAGDB_TEST_DATABASE_URL").ok()?;
            PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await
                .ok()
        }

        async fn delete_live_idempotency_rows(
            pool: &sqlx::PgPool,
            tenant_id: &str,
            namespace: &str,
        ) {
            let mut tx = begin_tenant_transaction(pool, tenant_id)
                .await
                .expect("tenant-bound idempotency cleanup transaction");
            sqlx::query(
                "DELETE FROM dagdb_idempotency_keys WHERE tenant_id = $1 AND namespace = $2",
            )
            .bind(tenant_id)
            .bind(namespace)
            .execute(&mut *tx)
            .await
            .expect("delete live idempotency rows");
            tx.commit().await.expect("commit live idempotency cleanup");
        }

        fn context_packet_request(request_id: &str) -> DagDbContextPacketRequest {
            DagDbContextPacketRequest {
                tenant_id: "tenant-a".to_owned(),
                namespace: "primary".to_owned(),
                idempotency_key: "idem-packet-mode".to_owned(),
                request_id: request_id.to_owned(),
                route_id: Hash256::digest(b"packet route").to_string(),
                task_hash: Hash256::digest(b"packet task").to_string(),
                requesting_agent_did: "did:exo:agent".to_owned(),
                token_budget: 512,
                force_revalidate: None,
                max_memory_refs: None,
                task: None,
                layered_mode: None,
                max_layer_depth: None,
                require_layer_evidence: None,
                drilldown_reserve_bp: None,
            }
        }

        fn selected_context_ref() -> exo_api::dagdb::DagDbSelectedContextRef {
            exo_api::dagdb::DagDbSelectedContextRef {
                memory_id: Hash256::digest(b"selected memory").to_string(),
                catalog_id: None,
                title: safe_gateway_metadata("Selected memory"),
                summary: safe_gateway_metadata("Selected summary"),
                catalog_path: Vec::new(),
                document_type: "note".to_owned(),
                selection_reason: "coverage fixture".to_owned(),
                token_estimate: 12,
                validation_status: ValidationStatus::Passed,
                citation_ref: "cite:selected".to_owned(),
                boundary_flags: Vec::new(),
            }
        }

        fn persistent_context_packet(
            selected_memory_refs: Vec<exo_api::dagdb::DagDbSelectedContextRef>,
        ) -> exo_dag_db_postgres::persistent_context::PersistentGraphContextPacket {
            let selected_memory_receipt_hashes = selected_memory_refs
                .iter()
                .map(|selected| {
                    (
                        selected.memory_id.clone(),
                        Hash256::digest(selected.memory_id.as_bytes()).to_string(),
                    )
                })
                .collect();
            persistent_context_packet_with_receipts(
                selected_memory_refs,
                selected_memory_receipt_hashes,
            )
        }

        fn persistent_context_packet_with_receipts(
            selected_memory_refs: Vec<exo_api::dagdb::DagDbSelectedContextRef>,
            selected_memory_receipt_hashes: BTreeMap<String, String>,
        ) -> exo_dag_db_postgres::persistent_context::PersistentGraphContextPacket {
            let selection_status = if selected_memory_refs.is_empty() {
                DagDbGraphContextSelectionStatus::Empty
            } else {
                DagDbGraphContextSelectionStatus::Selected
            };
            let selection = DagDbGraphContextSelectionResponse {
                tenant_id: "tenant-a".to_owned(),
                namespace: "primary".to_owned(),
                request_id: "packet-mode-request".to_owned(),
                task_hash: Hash256::digest(b"packet task").to_string(),
                selection_status,
                selected_memory_refs: selected_memory_refs.clone(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 12,
                token_budget: 512,
                boundary_warnings: Vec::new(),
            };
            exo_dag_db_postgres::persistent_context::PersistentGraphContextPacket {
                tenant_id: "tenant-a".to_owned(),
                namespace: "primary".to_owned(),
                selection:
                    exo_dag_db_postgres::persistent_context::PersistentGraphContextSelection {
                        tenant_id: "tenant-a".to_owned(),
                        namespace: "primary".to_owned(),
                        memory_row_count: u32::try_from(selected_memory_refs.len())
                            .unwrap_or(u32::MAX),
                        catalog_row_count: 0,
                        graph_edge_row_count: 0,
                        validation_row_count: 0,
                        receipt_row_count: 0,
                        skipped_row_count: 0,
                        selection: selection.clone(),
                        selected_memory_receipt_hashes,
                        boundary_warnings: Vec::new(),
                    },
                packet: exo_api::dagdb::DagDbGraphContextPacket {
                    schema_version: "dagdb_graph_context_packet.v1".to_owned(),
                    tenant_id: "tenant-a".to_owned(),
                    namespace: "primary".to_owned(),
                    request_id: "packet-mode-request".to_owned(),
                    task: "packet mode coverage".to_owned(),
                    task_hash: Hash256::digest(b"packet task").to_string(),
                    packet_hash: Hash256::digest(b"packet hash").to_string(),
                    selected_memory_refs,
                    selected_graph_edges: Vec::new(),
                    citation_refs: Vec::new(),
                    packet_metrics: exo_api::dagdb::DagDbContextPacketMetrics {
                        token_budget: 512,
                        selected_token_estimate: 12,
                        selected_memory_ref_count: u32::try_from(
                            selection.selected_memory_refs.len(),
                        )
                        .unwrap_or(u32::MAX),
                        selected_graph_edge_count: 0,
                        citation_ref_count: 0,
                        end_to_end_savings_status: "not_claimed".to_owned(),
                        cost_savings_status: "not_claimed".to_owned(),
                    },
                    boundaries: exo_api::dagdb::DagDbContextPacketBoundaries {
                        repository_test_level_only: true,
                        production_runtime: "not_approved".to_owned(),
                        default_context_replacement: "not_claimed".to_owned(),
                        citation_locator_status: "fixture".to_owned(),
                        billing_savings: "not_claimed".to_owned(),
                    },
                    agent_usage_instructions: Vec::new(),
                    markdown: String::new(),
                },
                boundary_warnings: Vec::new(),
            }
        }

        fn portable_export_with_counted_sections()
        -> exo_dag_db_exchange::kg_export::KgPortableExport {
            let record = |key: &str, value: &str| {
                [(key.to_owned(), json!(value))]
                    .into_iter()
                    .collect::<exo_dag_db_exchange::kg_export::KgExportRecord>()
            };
            exo_dag_db_exchange::kg_export::build_portable_export(
                exo_dag_db_exchange::kg_export::KgExportBuildInput {
                    scope: KgExportScope {
                        tenant_id: "tenant-a".to_owned(),
                        namespace: "primary".to_owned(),
                        included_memory_ids: Vec::new(),
                        included_graph_styles: Vec::new(),
                        included_writeback_idempotency_keys: Vec::new(),
                        source_commit_or_repo_ref: Some(
                            "c706242d36f1c275e05d8a132778491da08f61c7".to_owned(),
                        ),
                        include_preview_context: true,
                    },
                    memory_records: vec![record("memory_id", "memory-a")],
                    catalog_entries: vec![record("catalog_id", "catalog-a")],
                    graph_nodes: vec![record("graph_node_id", "node-a")],
                    graph_edges: vec![record("graph_edge_id", "edge-a")],
                    similarity_results: vec![record("similarity_result_id", "similarity-a")],
                    canonicalization_decisions: vec![record("decision_id", "decision-a")],
                    placement_traces: vec![record("placement_trace_id", "trace-a")],
                    validation_reports: vec![record("validation_report_id", "validation-a")],
                    receipts: vec![record("receipt_hash", "receipt-a")],
                    subject_receipt_heads: vec![record("subject_id", "memory-a")],
                    context_packet_previews: vec![record("context_packet_id", "packet-preview-a")],
                    context_packet_records: vec![record("context_packet_id", "packet-a")],
                    route_receipts: vec![record("route_id", "route-a")],
                    writeback_summaries: vec![record("idempotency_key", "idem-writeback-a")],
                    idempotency_references: vec![record("idempotency_key", "idem-a")],
                    citation_index: vec![record("citation_handle", "cite-a")],
                    provenance_index: vec![record("subject_id", "memory-a")],
                },
            )
            .expect("portable export")
        }

        fn fixture_writeback_request() -> DagDbWritebackRequest {
            let fixtures = fixtures();
            fixture(&fixtures, "requests", "writeback")
        }

        fn fixture_writeback_selection_response(
            request: &DagDbWritebackRequest,
        ) -> DagDbGraphContextSelectionResponse {
            let selection_request =
                selection_request_from_writeback(request).expect("writeback selection request");
            DagDbGraphContextSelectionResponse {
                tenant_id: request.tenant_id.clone(),
                namespace: request.namespace.clone(),
                request_id: selection_request.request_id,
                task_hash: selection_request.task_hash,
                selection_status: DagDbGraphContextSelectionStatus::Empty,
                selected_memory_refs: Vec::new(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 0,
                token_budget: selection_request.token_budget,
                boundary_warnings: Vec::new(),
            }
        }

        fn writeback_preflight_signatures(
            keypair: &KeyPair,
            selection: &DagDbGraphContextSelectionResponse,
            lifecycle: &LifecycleAction,
            continuation: &ContinuationRecord,
        ) -> (String, String, String) {
            let writeback_signature = sign_write_payload(
                keypair,
                &usage_event_payload_hash(selection).expect("writeback payload hash"),
            )
            .expect("writeback signature");
            let lifecycle_signature = sign_write_payload(
                keypair,
                &lifecycle_action_payload_hash(lifecycle).expect("lifecycle payload hash"),
            )
            .expect("lifecycle signature");
            let continuation_signature = sign_write_payload(
                keypair,
                &continuation_record_payload_hash(continuation).expect("continuation payload hash"),
            )
            .expect("continuation signature");
            (
                writeback_signature,
                lifecycle_signature,
                continuation_signature,
            )
        }

        fn consented_service_for_agent(
            keypair: &KeyPair,
            tenant_id: &str,
            agent_did: &str,
        ) -> DagDbGatekeeperService {
            let pool = PgPoolOptions::new()
                .acquire_timeout(Duration::from_millis(50))
                .connect_lazy("postgres://127.0.0.1:1/unreachable")
                .expect("lazy pool");
            DagDbGatekeeperService::new(
                pool,
                Arc::new(consent_engine_for_agent(tenant_id, agent_did)),
                Arc::new(identity_registry_for_agent(agent_did, keypair)),
            )
        }

        fn consent_engine_for_agent(tenant_id: &str, agent_did: &str) -> ConsentEngine {
            ConsentEngine::default()
                .with_bailment(
                    tenant_id,
                    BailmentState::Active {
                        bailor: exo_core::Did::new("did:exo:bailor").expect("valid bailor DID"),
                        bailee: exo_core::Did::new(agent_did).expect("valid agent DID"),
                        scope: DAGDB_WRITEBACK_SCOPE.to_owned(),
                    },
                )
                .with_consent_record(DagDbConsentRecord {
                    tenant_id: tenant_id.to_owned(),
                    agent_did: agent_did.to_owned(),
                    purpose: ConsentPurpose::Writeback,
                    active: true,
                })
        }

        fn identity_registry_for_agent(agent_did: &str, keypair: &KeyPair) -> IdentityRegistry {
            IdentityRegistry::default().with_public_key(agent_did, *keypair.public_key().as_bytes())
        }
    }
}
