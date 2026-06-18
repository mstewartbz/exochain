#![cfg(feature = "production-db")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::{Arc, RwLock};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use exo_api::dagdb::{
    ConsentPurpose, CouncilDecisionStatus, DagDbCatalogLookupRequest, DagDbContextPacketRequest,
    DagDbCouncilDecisionRequest, DagDbErrorEnvelope, DagDbExportRequest, DagDbImportRequest,
    DagDbIntakeRequest, DagDbReceiptLookupRequest, DagDbRouteLookupRequest, DagDbRouteRequest,
    DagDbTrustCheckRequest, DagDbValidateRequest, DagDbWritebackRequest, DecisionSource, RiskClass,
    SubjectKind,
};
use exo_core::{Hash256, crypto::KeyPair};
use exo_dag_db_core::hash::RequestHashMaterial;
use exo_dag_db_exchange::kg_import::{
    KG_IMPORT_CANDIDATES_SCHEMA, KG_IMPORT_DATABASE_URL_ENV, KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
    required_trace,
};
use exo_dag_db_postgres::{
    persistent_context::build_persistent_graph_context_selection,
    postgres::{
        DAGDB_GRAPH_SCHEMA_SQL, DAGDB_SCHEMA_SQL, DAGDB_TELEMETRY_FACET_NODE_TYPE_SCHEMA_SQL,
    },
};
use exo_gatekeeper::{
    ConsentEngine, DagDbConsentRecord, IdentityRegistry, sign_write_payload, types::BailmentState,
    usage_event_payload_hash,
};
use exo_gateway::{
    dagdb::{
        DagDbRouteContext, dagdb_router, selection_request_from_writeback,
        set_route_context_for_integration_tests,
    },
    server::{AppState, build_router},
};
use exo_identity::registry::LocalDidRegistry;
use serde::de::DeserializeOwned;
use serde_json::{Value as JsonValue, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

const TENANT_ID: &str = "tenant-a";
const NAMESPACE: &str = "primary";
const AGENT_DID: &str = "did:exo:agent";
const EXPORTER_DID: &str = "did:exo:exporter";
const BEARER: &str = "test-token";
const FORGED_BEARER: &str = "forged-test-token";

struct TestDb {
    admin_pool: PgPool,
    pool: PgPool,
    schema: String,
}

impl TestDb {
    async fn new(label: &str, database_url: &str) -> Self {
        let schema = format!("dagdb_route_m14_{label}_{}", std::process::id());
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
            .expect("connect admin Postgres pool");
        sqlx::query(&format!(r#"DROP SCHEMA IF EXISTS "{schema}" CASCADE"#))
            .execute(&admin_pool)
            .await
            .expect("drop isolated schema");
        sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
            .execute(&admin_pool)
            .await
            .expect("create isolated schema");

        let scoped_url = database_url_with_search_path(database_url, &schema);
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&scoped_url)
            .await
            .expect("connect scoped Postgres pool");
        sqlx::raw_sql(DAGDB_SCHEMA_SQL)
            .execute(&pool)
            .await
            .expect("apply DAG DB schema");
        sqlx::raw_sql(DAGDB_GRAPH_SCHEMA_SQL)
            .execute(&pool)
            .await
            .expect("apply DAG DB graph schema");
        sqlx::raw_sql(DAGDB_TELEMETRY_FACET_NODE_TYPE_SCHEMA_SQL)
            .execute(&pool)
            .await
            .expect("apply DAG DB telemetry-facet node_type schema");
        sqlx::raw_sql(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                did TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                roles JSONB NOT NULL DEFAULT '[]',
                tenant_id TEXT NOT NULL,
                created_at BIGINT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                pace_status TEXT NOT NULL DEFAULT 'Unenrolled',
                password_hash TEXT NOT NULL,
                salt TEXT NOT NULL,
                mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                actor_did TEXT NOT NULL,
                created_at BIGINT NOT NULL,
                expires_at BIGINT NOT NULL,
                revoked BOOLEAN NOT NULL DEFAULT FALSE
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("apply gateway auth tables");
        Self {
            admin_pool,
            pool,
            schema,
        }
    }

    async fn cleanup(self) {
        self.pool.close().await;
        sqlx::query(&format!(
            r#"DROP SCHEMA IF EXISTS "{}" CASCADE"#,
            self.schema
        ))
        .execute(&self.admin_pool)
        .await
        .expect("drop isolated schema after test");
        self.admin_pool.close().await;
    }
}

#[tokio::test]
async fn dagdb_routes_integration_contract() {
    let Some(database_url) = configured_database_url("dagdb_routes_integration_contract") else {
        return;
    };
    let db = TestDb::new("integration", &database_url).await;

    let keypair = KeyPair::generate();
    let ctx = Arc::new(DagDbRouteContext::from_pool(Some(db.pool.clone())));
    set_route_context_for_integration_tests(ctx.clone());
    insert_session_user(&db.pool, BEARER, AGENT_DID, TENANT_ID).await;

    let app = build_router(AppState::new(
        Some(db.pool.clone()),
        Arc::new(RwLock::new(LocalDidRegistry::new())),
    ));

    let import_request = DagDbImportRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-import-integration".to_owned(),
        db_set_version: "dag_db-project_memory_v3".to_owned(),
        source_hash: h(0x01),
        requester_did: AGENT_DID.to_owned(),
        import_report: base_report(),
    };
    let forged_bearer_response = app
        .clone()
        .oneshot(scoped_post_with_bearer(
            FORGED_BEARER,
            "/api/v1/dag-db/import",
            "dagdb:import",
            &import_request,
            None,
        ))
        .await
        .expect("forged bearer response");
    assert_eq!(forged_bearer_response.status(), StatusCode::UNAUTHORIZED);
    let receipts_before_missing_import_signature =
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let missing_import_signature = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/import",
            "dagdb:import",
            &import_request,
            None,
        ))
        .await
        .expect("missing import signature response");
    assert_eq!(missing_import_signature.status(), StatusCode::BAD_REQUEST);
    let missing_import_signature_body: DagDbErrorEnvelope =
        response_json(missing_import_signature).await;
    assert_eq!(
        missing_import_signature_body.error_code,
        "write_signature_required"
    );
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_missing_import_signature,
        "missing import signature must fail before DAG DB persistence"
    );

    let mut scope_mismatch_import = import_request.clone();
    scope_mismatch_import.idempotency_key = "idem-import-scope-mismatch".to_owned();
    scope_mismatch_import.import_report["tenant_id"] = json!("tenant-b");
    let scope_mismatch_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/import",
            "dagdb:import",
            &scope_mismatch_import,
            None,
        ))
        .await
        .expect("import scope mismatch response");
    assert_eq!(scope_mismatch_response.status(), StatusCode::BAD_REQUEST);
    let scope_mismatch_body: DagDbErrorEnvelope = response_json(scope_mismatch_response).await;
    assert_eq!(scope_mismatch_body.error_code, "invalid_request_shape");
    assert_no_forbidden_material(&scope_mismatch_body.message);

    let mut namespace_scope_mismatch_import = import_request.clone();
    namespace_scope_mismatch_import.idempotency_key = "idem-import-namespace-mismatch".to_owned();
    namespace_scope_mismatch_import.import_report["namespace"] = json!("secondary");
    let namespace_scope_mismatch_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/import",
            "dagdb:import",
            &namespace_scope_mismatch_import,
            None,
        ))
        .await
        .expect("import namespace scope mismatch response");
    assert_eq!(
        namespace_scope_mismatch_response.status(),
        StatusCode::BAD_REQUEST
    );
    let namespace_scope_mismatch_body: DagDbErrorEnvelope =
        response_json(namespace_scope_mismatch_response).await;
    assert_eq!(
        namespace_scope_mismatch_body.error_code,
        "invalid_request_shape"
    );
    assert_no_forbidden_material(&namespace_scope_mismatch_body.message);

    let mut empty_field_import = import_request.clone();
    empty_field_import.idempotency_key = " ".to_owned();
    let empty_field_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/import",
            "dagdb:import",
            &empty_field_import,
            None,
        ))
        .await
        .expect("empty import runtime field response");
    assert_eq!(empty_field_response.status(), StatusCode::BAD_REQUEST);
    let empty_field_body: DagDbErrorEnvelope = response_json(empty_field_response).await;
    assert_eq!(empty_field_body.error_code, "invalid_request_shape");
    assert_no_forbidden_material(&empty_field_body.message);

    let invalid_requester_export = DagDbExportRequest {
        idempotency_key: "idem-export-invalid-requester".to_owned(),
        requester_did: "not-a-did".to_owned(),
        ..export_request()
    };
    let invalid_requester_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/export",
            "dagdb:export",
            &invalid_requester_export,
            None,
        ))
        .await
        .expect("invalid export requester response");
    assert_eq!(invalid_requester_response.status(), StatusCode::BAD_REQUEST);
    let invalid_requester_body: DagDbErrorEnvelope =
        response_json(invalid_requester_response).await;
    assert_eq!(invalid_requester_body.error_code, "invalid_request_shape");
    assert_no_forbidden_material(&invalid_requester_body.message);

    ctx.install_gatekeeper_profile(
        ConsentEngine::default(),
        IdentityRegistry::default().with_public_key(AGENT_DID, *keypair.public_key().as_bytes()),
    );
    let denied_import_signature = import_signature(&db.pool, &keypair, &import_request).await;
    let denied_import = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/import",
            "dagdb:import",
            &import_request,
            Some(denied_import_signature),
        ))
        .await
        .expect("consent denied import response");
    assert_eq!(denied_import.status(), StatusCode::FORBIDDEN);
    let denied_import_body: DagDbErrorEnvelope = response_json(denied_import).await;
    assert_eq!(denied_import_body.error_code, "consent_denied");
    assert_no_forbidden_material(&denied_import_body.message);

    ctx.install_gatekeeper_profile(
        active_consent_engine(),
        IdentityRegistry::default()
            .with_public_key(AGENT_DID, *keypair.public_key().as_bytes())
            .with_public_key(EXPORTER_DID, *keypair.public_key().as_bytes()),
    );
    let receipts_before_import_consent_gap = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let idempotency_before_import_consent_gap =
        idempotency_count(&db.pool, TENANT_ID, NAMESPACE, "dagdb.import").await;
    let initial_import_signature = import_signature(&db.pool, &keypair, &import_request).await;
    let import_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/import",
            "dagdb:import",
            &import_request,
            Some(initial_import_signature),
        ))
        .await
        .expect("import response");
    assert_eq!(import_response.status(), StatusCode::FORBIDDEN);
    let import_body: DagDbErrorEnvelope = response_json(import_response).await;
    assert_eq!(import_body.error_code, "consent_denied");
    assert!(
        import_body
            .message
            .contains("distinct import/export consent"),
        "import denial must identify the missing distinct consent"
    );
    assert_no_forbidden_material(&import_body.message);
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_import_consent_gap,
        "writeback consent must not let import append DAG DB rows"
    );
    assert_eq!(
        idempotency_count(&db.pool, TENANT_ID, NAMESPACE, "dagdb.import").await,
        idempotency_before_import_consent_gap,
        "writeback consent must not leave an import idempotency reservation"
    );
    let import_seed_json =
        serde_json::to_string(&import_request.import_report).expect("seed import report json");
    exo_dag_db_postgres::postgres::kg_import::persist_kg_import_report(&db.pool, &import_seed_json)
        .await
        .expect("seed DAG DB graph rows for writeback/context contract");

    let packet_request_denied = DagDbContextPacketRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-packet-2".to_owned(),
        request_id: "request-2".to_owned(),
        route_id: h(0x40),
        task_hash: h(0x51),
        requesting_agent_did: AGENT_DID.to_owned(),
        token_budget: 2048,
        force_revalidate: None,
        max_memory_refs: None,
        task: None,
        layered_mode: None,
        max_layer_depth: None,
        require_layer_evidence: None,
        drilldown_reserve_bp: None,
    };
    let unauthenticated = app
        .clone()
        .oneshot(unauthenticated_post(
            "/api/v1/dag-db/context-packet",
            &packet_request_denied,
        ))
        .await
        .expect("unauthenticated response");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let unauth_body: DagDbErrorEnvelope = response_json(unauthenticated).await;
    assert_eq!(unauth_body.error_code, "unauthenticated");

    // A forged bearer with self-asserted tenant/namespace/scope headers must
    // not reach persisted tenant data on the signature-free read path: the
    // token must resolve to a live DB-backed session for the tenant.
    let forged_bearer_packet = app
        .clone()
        .oneshot(scoped_post_with_bearer(
            FORGED_BEARER,
            "/api/v1/dag-db/context-packet",
            "dagdb:context_packet",
            &packet_request_denied,
            None,
        ))
        .await
        .expect("forged bearer context packet response");
    assert_eq!(forged_bearer_packet.status(), StatusCode::UNAUTHORIZED);
    let forged_packet_body: DagDbErrorEnvelope = response_json(forged_bearer_packet).await;
    assert_eq!(forged_packet_body.error_code, "unauthenticated");

    ctx.install_gatekeeper_profile(
        ConsentEngine::default(),
        IdentityRegistry::default().with_public_key(AGENT_DID, *keypair.public_key().as_bytes()),
    );
    let writeback_request_denied = DagDbWritebackRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-2".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x81),
        route_id: h(0x91),
        context_packet_id: h(0xa0),
        validation_report_id: h(0xb1),
        summary_text: Some("Safe answer summary".to_owned()),
        citation_hashes: None,
        safety_score_id: None,
        keyword_texts: None,
        knowledge_class: None,
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
    };
    let selection_denied = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&writeback_request_denied)
            .expect("selection request for denied writeback"),
    )
    .await
    .expect("selection for signature");
    let signature_denied = sign_write_payload(
        &keypair,
        &usage_event_payload_hash(&selection_denied.selection).expect("hash"),
    )
    .expect("signature");
    let denied = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &writeback_request_denied,
            Some(signature_denied),
        ))
        .await
        .expect("consent denied response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let denied_body: DagDbErrorEnvelope = response_json(denied).await;
    assert_eq!(denied_body.error_code, "consent_denied");

    let writeback_request_missing_signature = DagDbWritebackRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-missing-signature".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x82),
        route_id: h(0x92),
        context_packet_id: h(0xa1),
        validation_report_id: h(0xb2),
        summary_text: Some("Safe answer summary".to_owned()),
        citation_hashes: None,
        safety_score_id: None,
        keyword_texts: None,
        knowledge_class: None,
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
    };
    let missing_signature = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &writeback_request_missing_signature,
            None,
        ))
        .await
        .expect("missing write signature response");
    assert_eq!(missing_signature.status(), StatusCode::BAD_REQUEST);
    let missing_signature_body: DagDbErrorEnvelope = response_json(missing_signature).await;
    assert_eq!(
        missing_signature_body.error_code,
        "write_signature_required"
    );

    ctx.install_gatekeeper_profile(
        active_consent_engine(),
        IdentityRegistry::default()
            .with_public_key(AGENT_DID, *keypair.public_key().as_bytes())
            .with_public_key(EXPORTER_DID, *keypair.public_key().as_bytes()),
    );

    let packet_request = DagDbContextPacketRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-packet-1".to_owned(),
        request_id: "request-1".to_owned(),
        route_id: h(0x40),
        task_hash: h(0x50),
        requesting_agent_did: AGENT_DID.to_owned(),
        token_budget: 2048,
        force_revalidate: Some(false),
        max_memory_refs: Some(8),
        task: Some("Implement live gateway context packet selection for DAG DB MCP".to_owned()),
        layered_mode: None,
        max_layer_depth: None,
        require_layer_evidence: None,
        drilldown_reserve_bp: None,
    };
    let packet_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/context-packet",
            "dagdb:context_packet",
            &packet_request,
            None,
        ))
        .await
        .expect("context packet response");
    assert_eq!(packet_response.status(), StatusCode::OK);
    let packet_body: JsonValue = response_json(packet_response).await;
    assert_eq!(packet_body["tenant_id"], TENANT_ID);
    assert!(!packet_body["memory_refs"].as_array().unwrap().is_empty());
    if packet_body["validation_status"] == "passed" {
        for memory_ref in packet_body["memory_refs"].as_array().unwrap() {
            assert_ne!(
                memory_ref["latest_receipt_hash"],
                JsonValue::String(exo_core::Hash256::ZERO.to_string())
            );
        }
    } else {
        assert_eq!(packet_body["validation_status"], "failed");
        assert!(
            packet_body["selection_warning"]
                .as_str()
                .unwrap_or_default()
                .contains("receipt hash unavailable")
        );
    }
    assert_no_forbidden_material(&packet_body.to_string());

    let context_packet_id = packet_body["context_packet_id"]
        .as_str()
        .expect("context_packet_id")
        .to_owned();
    let receipts_before = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let writeback_request = DagDbWritebackRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-1".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x80),
        route_id: h(0x90),
        context_packet_id,
        validation_report_id: h(0xb0),
        summary_text: Some("Safe answer summary".to_owned()),
        citation_hashes: Some(vec![h(0xc0)]),
        safety_score_id: None,
        keyword_texts: Some(vec!["answer".to_owned()]),
        knowledge_class: None,
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
    };
    // Sign over the same selection material the gateway reconstructs from the
    // writeback request (selection_request_from_writeback binds summary_text /
    // knowledge_class into the signed task hash), so the operator signature
    // cryptographically covers the persisted searchable metadata.
    let selection = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&writeback_request)
            .expect("selection request for writeback"),
    )
    .await
    .expect("selection for signature");
    let payload_hash = usage_event_payload_hash(&selection.selection).expect("payload hash");
    let signature = sign_write_payload(&keypair, &payload_hash).expect("signature");

    let receipts_before_metadata_relay = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let memory_objects_before_metadata_relay =
        memory_object_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let mutated_summary_relay_request = DagDbWritebackRequest {
        summary_text: Some("Attacker-mutated searchable summary".to_owned()),
        ..writeback_request.clone()
    };
    let mutated_summary_relay = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &mutated_summary_relay_request,
            Some(signature.clone()),
        ))
        .await
        .expect("mutated summary relay response");
    assert_eq!(
        mutated_summary_relay.status(),
        StatusCode::FORBIDDEN,
        "a writeback signature must not authorize a mutated summary_text"
    );
    let mutated_summary_relay_body: DagDbErrorEnvelope = response_json(mutated_summary_relay).await;
    assert_eq!(mutated_summary_relay_body.error_code, "provenance_denied");
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_metadata_relay,
        "mutated summary relay must not persist DAG DB receipts"
    );
    assert_eq!(
        memory_object_count(&db.pool, TENANT_ID, NAMESPACE).await,
        memory_objects_before_metadata_relay,
        "mutated summary relay must not persist DAG DB memory rows"
    );

    let mutated_knowledge_class_relay_request = DagDbWritebackRequest {
        knowledge_class: Some("finding".to_owned()),
        ..writeback_request.clone()
    };
    let mutated_knowledge_class_relay = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &mutated_knowledge_class_relay_request,
            Some(signature.clone()),
        ))
        .await
        .expect("mutated knowledge class relay response");
    assert_eq!(
        mutated_knowledge_class_relay.status(),
        StatusCode::FORBIDDEN,
        "a writeback signature must not authorize a mutated knowledge_class"
    );
    let mutated_knowledge_class_relay_body: DagDbErrorEnvelope =
        response_json(mutated_knowledge_class_relay).await;
    assert_eq!(
        mutated_knowledge_class_relay_body.error_code,
        "provenance_denied"
    );
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_metadata_relay,
        "mutated knowledge_class relay must not persist DAG DB receipts"
    );
    assert_eq!(
        memory_object_count(&db.pool, TENANT_ID, NAMESPACE).await,
        memory_objects_before_metadata_relay,
        "mutated knowledge_class relay must not persist DAG DB memory rows"
    );

    let writeback_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &writeback_request,
            Some(signature.clone()),
        ))
        .await
        .expect("writeback response");
    assert_eq!(writeback_response.status(), StatusCode::OK);
    let writeback_body: JsonValue = response_json(writeback_response).await;
    assert!(!writeback_body["receipt_hash"].as_str().unwrap().is_empty());
    assert_no_forbidden_material(&writeback_body.to_string());
    let receipts_after = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    assert!(
        receipts_after > receipts_before,
        "writeback must append at least one dagdb_receipts row"
    );

    // Q3-S1 typed knowledge writebacks: an invalid class fails closed before any
    // persistence, an empty summary on a knowledge class is rejected, a valid
    // class persists a searchable `knowledge:<class>` keyword, and a classless
    // writeback persists no knowledge keyword (telemetry unchanged).
    let receipts_before_invalid_class = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let invalid_class_request = DagDbWritebackRequest {
        idempotency_key: "idem-writeback-invalid-class".to_owned(),
        answer_hash: h(0x84),
        route_id: h(0x94),
        validation_report_id: h(0xb4),
        summary_text: Some("Has a summary but an invalid class".to_owned()),
        knowledge_class: Some("rumor".to_owned()),
        ..writeback_request.clone()
    };
    let invalid_class_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &invalid_class_request,
            Some(signature.clone()),
        ))
        .await
        .expect("invalid knowledge class response");
    assert_eq!(
        invalid_class_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let invalid_class_body: DagDbErrorEnvelope = response_json(invalid_class_response).await;
    assert_eq!(invalid_class_body.error_code, "invalid_knowledge_class");
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_invalid_class,
        "invalid knowledge class must fail before DAG DB persistence"
    );

    let empty_summary_class_request = DagDbWritebackRequest {
        idempotency_key: "idem-writeback-empty-summary-class".to_owned(),
        answer_hash: h(0x85),
        route_id: h(0x95),
        validation_report_id: h(0xb5),
        summary_text: Some("   ".to_owned()),
        knowledge_class: Some("finding".to_owned()),
        ..writeback_request.clone()
    };
    let empty_summary_class_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &empty_summary_class_request,
            Some(signature.clone()),
        ))
        .await
        .expect("empty summary knowledge class response");
    assert_eq!(
        empty_summary_class_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let empty_summary_class_body: DagDbErrorEnvelope =
        response_json(empty_summary_class_response).await;
    assert_eq!(
        empty_summary_class_body.error_code,
        "knowledge_class_requires_summary"
    );
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_invalid_class,
        "empty-summary knowledge writeback must fail before DAG DB persistence"
    );

    let knowledge_writeback_request = DagDbWritebackRequest {
        idempotency_key: "idem-writeback-knowledge-finding".to_owned(),
        answer_hash: h(0x86),
        route_id: h(0x96),
        validation_report_id: h(0xb6),
        summary_text: Some("Typed knowledge writeback persisted a finding".to_owned()),
        knowledge_class: Some("finding".to_owned()),
        ..writeback_request.clone()
    };
    let knowledge_selection = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&knowledge_writeback_request)
            .expect("knowledge selection request"),
    )
    .await
    .expect("knowledge selection for signature");
    let knowledge_signature = sign_write_payload(
        &keypair,
        &usage_event_payload_hash(&knowledge_selection.selection).expect("knowledge payload hash"),
    )
    .expect("knowledge signature");
    let knowledge_writeback_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &knowledge_writeback_request,
            Some(knowledge_signature),
        ))
        .await
        .expect("knowledge writeback response");
    assert_eq!(knowledge_writeback_response.status(), StatusCode::OK);
    let knowledge_keywords = memory_keyword_texts(
        &db.pool,
        TENANT_ID,
        NAMESPACE,
        &knowledge_selection.selection,
    )
    .await;
    assert!(
        knowledge_keywords
            .iter()
            .any(|keyword| keyword == "knowledge:finding"),
        "knowledge writeback must persist a searchable knowledge:finding keyword, got {knowledge_keywords:?}"
    );

    let classless_writeback_request = DagDbWritebackRequest {
        idempotency_key: "idem-writeback-classless".to_owned(),
        answer_hash: h(0x87),
        route_id: h(0x97),
        validation_report_id: h(0xb7),
        summary_text: Some("Plain telemetry writeback with no knowledge class".to_owned()),
        knowledge_class: None,
        ..writeback_request.clone()
    };
    let classless_selection = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&classless_writeback_request)
            .expect("classless selection request"),
    )
    .await
    .expect("classless selection for signature");
    let classless_signature = sign_write_payload(
        &keypair,
        &usage_event_payload_hash(&classless_selection.selection).expect("classless payload hash"),
    )
    .expect("classless signature");
    let classless_writeback_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &classless_writeback_request,
            Some(classless_signature),
        ))
        .await
        .expect("classless writeback response");
    assert_eq!(classless_writeback_response.status(), StatusCode::OK);
    let classless_keywords = memory_keyword_texts(
        &db.pool,
        TENANT_ID,
        NAMESPACE,
        &classless_selection.selection,
    )
    .await;
    assert!(
        !classless_keywords
            .iter()
            .any(|keyword| keyword.starts_with("knowledge:")),
        "classless writeback must not persist any knowledge keyword, got {classless_keywords:?}"
    );

    // The layered writeback target must be bound into the signed usage-event
    // payload: a relayed signature minted for one writeback shape must not
    // authorize first-submission of a mutated layer target, and a signature
    // minted for a flat writeback must not authorize adding a layer target.
    let layered_writeback_request = DagDbWritebackRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-layered-1".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x83),
        route_id: h(0x93),
        context_packet_id: writeback_request.context_packet_id.clone(),
        validation_report_id: h(0xb3),
        summary_text: Some("Safe answer summary".to_owned()),
        citation_hashes: None,
        safety_score_id: None,
        keyword_texts: None,
        knowledge_class: None,
        layered_mode: Some("auto".to_owned()),
        target_layer_path: Some("root/codex/runtime".to_owned()),
        target_layer_depth: Some(2),
        target_layer_reason: Some("layer target binding regression".to_owned()),
    };
    let layered_selection = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&layered_writeback_request)
            .expect("layered selection request"),
    )
    .await
    .expect("layered selection for signature");
    let layered_signature = sign_write_payload(
        &keypair,
        &usage_event_payload_hash(&layered_selection.selection).expect("layered payload hash"),
    )
    .expect("layered signature");
    let flat_shape_request = DagDbWritebackRequest {
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
        ..layered_writeback_request.clone()
    };
    let flat_shape_selection = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&flat_shape_request).expect("flat selection request"),
    )
    .await
    .expect("flat selection for signature");
    let flat_shape_signature = sign_write_payload(
        &keypair,
        &usage_event_payload_hash(&flat_shape_selection.selection).expect("flat payload hash"),
    )
    .expect("flat shape signature");

    let receipts_before_layer_relay = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let mutated_layer_relay_request = DagDbWritebackRequest {
        target_layer_path: Some("root/codex/exfiltration".to_owned()),
        target_layer_reason: Some("mutated layer target relay".to_owned()),
        ..layered_writeback_request.clone()
    };
    let mutated_layer_relay = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &mutated_layer_relay_request,
            Some(layered_signature.clone()),
        ))
        .await
        .expect("mutated layer relay response");
    assert_eq!(
        mutated_layer_relay.status(),
        StatusCode::FORBIDDEN,
        "a writeback signature must not authorize a mutated layer target"
    );
    let mutated_layer_relay_body: DagDbErrorEnvelope = response_json(mutated_layer_relay).await;
    assert_eq!(mutated_layer_relay_body.error_code, "provenance_denied");
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_layer_relay,
        "mutated layer target relay must not persist DAG DB rows"
    );

    let layered_escalation = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &layered_writeback_request,
            Some(flat_shape_signature),
        ))
        .await
        .expect("layered escalation response");
    assert_eq!(
        layered_escalation.status(),
        StatusCode::FORBIDDEN,
        "a flat writeback signature must not authorize adding a layer target"
    );
    let layered_escalation_body: DagDbErrorEnvelope = response_json(layered_escalation).await;
    assert_eq!(layered_escalation_body.error_code, "provenance_denied");
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_layer_relay,
        "flat signature layer escalation must not persist DAG DB rows"
    );

    let layered_writeback = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &layered_writeback_request,
            Some(layered_signature),
        ))
        .await
        .expect("layered writeback response");
    assert_eq!(
        layered_writeback.status(),
        StatusCode::OK,
        "a correctly signed layered writeback must still persist"
    );
    let layered_writeback_body: JsonValue = response_json(layered_writeback).await;
    assert_eq!(
        layered_writeback_body["layered_writeback_status"],
        "layer_target_recorded"
    );
    assert_eq!(
        layered_writeback_body["target_layer_path"],
        "root/codex/runtime"
    );
    assert!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await > receipts_before_layer_relay,
        "signed layered writeback must append DAG DB rows"
    );

    let export = export_request();
    let receipts_before_missing_export_signature =
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let idempotency_before_missing_export_signature =
        idempotency_count(&db.pool, TENANT_ID, NAMESPACE, "dagdb.export").await;
    let missing_export_signature = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/export",
            "dagdb:export",
            &export,
            None,
        ))
        .await
        .expect("missing export signature response");
    assert_eq!(missing_export_signature.status(), StatusCode::BAD_REQUEST);
    let missing_export_signature_body: DagDbErrorEnvelope =
        response_json(missing_export_signature).await;
    assert_eq!(
        missing_export_signature_body.error_code,
        "write_signature_required"
    );
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_missing_export_signature,
        "missing export signature must fail before DAG DB persistence"
    );
    assert_eq!(
        idempotency_count(&db.pool, TENANT_ID, NAMESPACE, "dagdb.export").await,
        idempotency_before_missing_export_signature,
        "missing export signature must fail before idempotency reservation"
    );

    let receipts_before_export_consent_gap = receipt_count(&db.pool, TENANT_ID, NAMESPACE).await;
    let idempotency_before_export_consent_gap =
        idempotency_count(&db.pool, TENANT_ID, NAMESPACE, "dagdb.export").await;
    let export_write_signature = export_signature(&db.pool, &keypair, &export).await;
    let export_response = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/export",
            "dagdb:export",
            &export,
            Some(export_write_signature),
        ))
        .await
        .expect("export response");
    assert_eq!(
        export_response.status(),
        StatusCode::FORBIDDEN,
        "writeback consent must not authorize export"
    );
    let export_body: DagDbErrorEnvelope = response_json(export_response).await;
    assert_eq!(export_body.error_code, "consent_denied");
    assert!(
        export_body
            .message
            .contains("distinct import/export consent"),
        "export denial must identify the missing distinct consent"
    );
    assert_no_forbidden_material(&export_body.message);
    assert_eq!(
        receipt_count(&db.pool, TENANT_ID, NAMESPACE).await,
        receipts_before_export_consent_gap,
        "writeback consent must not let export append DAG DB rows"
    );
    assert_eq!(
        idempotency_count(&db.pool, TENANT_ID, NAMESPACE, "dagdb.export").await,
        idempotency_before_export_consent_gap,
        "writeback consent must not leave an export idempotency reservation"
    );

    let writeback_request_tenant_mismatch = DagDbWritebackRequest {
        tenant_id: "tenant-b".to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-tenant-mismatch".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x83),
        route_id: h(0x93),
        context_packet_id: h(0xa2),
        validation_report_id: h(0xb3),
        summary_text: Some("Safe answer summary".to_owned()),
        citation_hashes: None,
        safety_score_id: None,
        keyword_texts: None,
        knowledge_class: None,
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
    };
    let tenant_mismatch = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &writeback_request_tenant_mismatch,
            Some(signature.clone()),
        ))
        .await
        .expect("tenant mismatch writeback response");
    assert_eq!(tenant_mismatch.status(), StatusCode::FORBIDDEN);
    let tenant_mismatch_body: DagDbErrorEnvelope = response_json(tenant_mismatch).await;
    assert_eq!(tenant_mismatch_body.error_code, "tenant_scope_mismatch");

    let bad_council = DagDbCouncilDecisionRequest {
        expires_at: "not-an-hlc".to_owned(),
        ..council_decision_request()
    };
    let council_error = app
        .clone()
        .oneshot(council_post(
            TENANT_ID,
            NAMESPACE,
            "dagdb:council_decision:tenant-a:primary",
            &bad_council,
        ))
        .await
        .expect("council error response");
    assert_eq!(council_error.status(), StatusCode::SERVICE_UNAVAILABLE);
    let council_error_body: DagDbErrorEnvelope = response_json(council_error).await;
    assert_eq!(council_error_body.error_code, "database_unavailable");

    let writeback_request_metadata = DagDbWritebackRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-metadata".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x83),
        route_id: h(0x93),
        context_packet_id: h(0xa2),
        validation_report_id: h(0xb3),
        summary_text: Some("fn raw_payload() {}".to_owned()),
        citation_hashes: None,
        safety_score_id: None,
        keyword_texts: None,
        knowledge_class: None,
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
    };
    let selection_metadata = build_persistent_graph_context_selection(
        &db.pool,
        &selection_request_from_writeback(&writeback_request_metadata)
            .expect("selection request for metadata writeback"),
    )
    .await
    .expect("selection for metadata rejection signature");
    let metadata_signature = sign_write_payload(
        &keypair,
        &usage_event_payload_hash(&selection_metadata.selection).expect("hash"),
    )
    .expect("signature");
    let metadata_rejected = app
        .clone()
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &writeback_request_metadata,
            Some(metadata_signature),
        ))
        .await
        .expect("metadata rejected response");
    let metadata_status = metadata_rejected.status();
    let metadata_body: DagDbErrorEnvelope = response_json(metadata_rejected).await;
    assert_eq!(
        metadata_status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unexpected error envelope: {metadata_body:?}"
    );
    assert_eq!(metadata_body.error_code, "metadata_rejected");

    let writeback_request_provenance = DagDbWritebackRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-writeback-provenance".to_owned(),
        requesting_agent_did: AGENT_DID.to_owned(),
        parent_memory_ids: vec![h(0x10)],
        answer_hash: h(0x84),
        route_id: h(0x94),
        context_packet_id: h(0xa3),
        validation_report_id: h(0xb4),
        summary_text: Some("Safe answer summary".to_owned()),
        citation_hashes: None,
        safety_score_id: None,
        keyword_texts: None,
        knowledge_class: None,
        layered_mode: None,
        target_layer_path: None,
        target_layer_depth: None,
        target_layer_reason: None,
    };
    let forged_hash = Hash256::digest(b"forged-payload");
    let forged_signature =
        sign_write_payload(&keypair, forged_hash.as_bytes()).expect("forged signature");
    let provenance_denied = app
        .oneshot(scoped_post(
            "/api/v1/dag-db/writeback",
            "dagdb:writeback",
            &writeback_request_provenance,
            Some(forged_signature),
        ))
        .await
        .expect("provenance denied response");
    assert_eq!(provenance_denied.status(), StatusCode::FORBIDDEN);
    let provenance_body: DagDbErrorEnvelope = response_json(provenance_denied).await;
    assert_eq!(provenance_body.error_code, "provenance_denied");

    // Regression (header-trust gap + synthetic scaffold success): the DAG DB
    // router itself must bind bearer tokens to live sessions when a pool is
    // configured, even when mounted WITHOUT the gateway-wide session middleware.
    // A forged bearer with self-asserted tenant/namespace/scope headers must
    // fail at auth, and a valid live session must still fail closed for routes
    // that have no governed DB-backed implementation.
    let standalone = dagdb_router::<()>();
    let fixtures = dagdb_fixtures();
    let trust_check = trust_check_request();
    let forged_trust = standalone
        .clone()
        .oneshot(scoped_post_with_bearer(
            FORGED_BEARER,
            "/api/v1/dag-db/trust-check",
            "dagdb:trust_check",
            &trust_check,
            None,
        ))
        .await
        .expect("forged bearer trust-check response");
    assert_eq!(forged_trust.status(), StatusCode::UNAUTHORIZED);
    let forged_trust_body: DagDbErrorEnvelope = response_json(forged_trust).await;
    assert_eq!(forged_trust_body.error_code, "unauthenticated");

    let live_trust = standalone
        .clone()
        .oneshot(scoped_post_with_bearer(
            BEARER,
            "/api/v1/dag-db/trust-check",
            "dagdb:trust_check",
            &trust_check,
            None,
        ))
        .await
        .expect("live bearer trust-check response");
    assert_live_scaffold_fail_closed(live_trust, "dagdb.trust_check").await;

    let intake: DagDbIntakeRequest = dagdb_fixture(&fixtures, "intake");
    let live_intake = standalone
        .clone()
        .oneshot(scoped_post_with_bearer(
            BEARER,
            "/api/v1/dag-db/intake",
            "dagdb:intake",
            &intake,
            None,
        ))
        .await
        .expect("live bearer intake response");
    assert_live_scaffold_fail_closed(live_intake, "dagdb.intake").await;

    let route: DagDbRouteRequest = dagdb_fixture(&fixtures, "route");
    let live_route = standalone
        .clone()
        .oneshot(scoped_post_with_bearer(
            BEARER,
            "/api/v1/dag-db/route",
            "dagdb:route",
            &route,
            None,
        ))
        .await
        .expect("live bearer route response");
    assert_live_scaffold_fail_closed(live_route, "dagdb.route").await;

    let validation: DagDbValidateRequest = dagdb_fixture(&fixtures, "validate");
    let live_validation = standalone
        .clone()
        .oneshot(scoped_post_with_bearer(
            BEARER,
            "/api/v1/dag-db/validate",
            "dagdb:validate",
            &validation,
            None,
        ))
        .await
        .expect("live bearer validate response");
    assert_live_scaffold_fail_closed(live_validation, "dagdb.validate").await;

    let receipt: DagDbReceiptLookupRequest = dagdb_fixture(&fixtures, "receipt_lookup");
    let live_receipt = standalone
        .clone()
        .oneshot(scoped_get(
            &format!(
                "/api/v1/dag-db/receipts/{}?tenant_id={}&namespace={}&include_body=true",
                receipt.receipt_hash, receipt.tenant_id, receipt.namespace
            ),
            "dagdb:receipt_lookup",
        ))
        .await
        .expect("live bearer receipt lookup response");
    assert_live_scaffold_fail_closed(live_receipt, "dagdb.receipt_lookup").await;

    let catalog: DagDbCatalogLookupRequest = dagdb_fixture(&fixtures, "catalog_lookup");
    let live_catalog = standalone
        .clone()
        .oneshot(scoped_get(
            &format!(
                "/api/v1/dag-db/catalog/{}?tenant_id={}&namespace={}&include_children=true&include_routes=true",
                catalog.catalog_id, catalog.tenant_id, catalog.namespace
            ),
            "dagdb:catalog_lookup",
        ))
        .await
        .expect("live bearer catalog lookup response");
    assert_live_scaffold_fail_closed(live_catalog, "dagdb.catalog_lookup").await;

    let route_lookup: DagDbRouteLookupRequest = dagdb_fixture(&fixtures, "route_lookup");
    let live_route_lookup = standalone
        .clone()
        .oneshot(scoped_get(
            &format!(
                "/api/v1/dag-db/routes/{}?tenant_id={}&namespace={}&include_memory_refs=true&include_validation=true",
                route_lookup.route_id, route_lookup.tenant_id, route_lookup.namespace
            ),
            "dagdb:route_lookup",
        ))
        .await
        .expect("live bearer route lookup response");
    assert_live_scaffold_fail_closed(live_route_lookup, "dagdb.route_lookup").await;

    let forged_council = standalone
        .clone()
        .oneshot(council_post_with_bearer(
            FORGED_BEARER,
            TENANT_ID,
            NAMESPACE,
            "dagdb:council_decision:tenant-a:primary",
            &council_decision_request(),
        ))
        .await
        .expect("forged bearer council response");
    assert_eq!(forged_council.status(), StatusCode::UNAUTHORIZED);
    let forged_council_body: DagDbErrorEnvelope = response_json(forged_council).await;
    assert_eq!(forged_council_body.error_code, "unauthenticated");

    let live_council = standalone
        .clone()
        .oneshot(council_post_with_bearer(
            BEARER,
            TENANT_ID,
            NAMESPACE,
            "dagdb:council_decision:tenant-a:primary",
            &council_decision_request(),
        ))
        .await
        .expect("live bearer council response");
    assert_live_scaffold_fail_closed(live_council, "dagdb.council_decision").await;

    db.cleanup().await;
}

fn trust_check_request() -> DagDbTrustCheckRequest {
    DagDbTrustCheckRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-trust-check-standalone".to_owned(),
        agent_did: AGENT_DID.to_owned(),
        operator_did: "did:exo:operator".to_owned(),
        model_name: "governed-model".to_owned(),
        model_version: "1.0".to_owned(),
        provider_or_builder: "exochain".to_owned(),
        requested_action: "memory:read".to_owned(),
        requested_scope_hash: h(0xd0),
        purpose: ConsentPurpose::TrustCheck,
        autonomy_level: "supervised".to_owned(),
        nonce: h(0xd1),
        expires_at: "2000:0".to_owned(),
        signature: "trust-check-signature".to_owned(),
        checkpoint_hash: None,
        attestation_hash: None,
        evidence_receipt_hashes: None,
        prior_trust_receipt_hash: None,
    }
}

fn council_decision_request() -> DagDbCouncilDecisionRequest {
    DagDbCouncilDecisionRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-council-integration".to_owned(),
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

fn council_post<T>(
    tenant_header: &str,
    namespace_header: &str,
    authority_scope: &str,
    body: &T,
) -> Request<Body>
where
    T: serde::Serialize,
{
    council_post_with_bearer(
        BEARER,
        tenant_header,
        namespace_header,
        authority_scope,
        body,
    )
}

fn council_post_with_bearer<T>(
    bearer: &str,
    tenant_header: &str,
    namespace_header: &str,
    authority_scope: &str,
    body: &T,
) -> Request<Body>
where
    T: serde::Serialize,
{
    Request::builder()
        .method("POST")
        .uri("/api/v1/dag-db/council/decision")
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header("x-exo-tenant-id", tenant_header)
        .header("x-exo-namespace", namespace_header)
        .header("x-exo-authority-scope", authority_scope)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(body).expect("serialize body"),
        ))
        .expect("request")
}

fn export_request() -> DagDbExportRequest {
    DagDbExportRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        idempotency_key: "idem-export-integration".to_owned(),
        db_set_version: "dag_db-project_memory_v3".to_owned(),
        // The export requester must be the authenticated session actor: a
        // writeback-authorized agent cannot self-assert a different
        // requester_did to reach the export adapter (the cross-actor case is
        // covered by the bind_requester_to_session_actor unit tests).
        requester_did: AGENT_DID.to_owned(),
        included_memory_ids: Vec::new(),
        included_graph_styles: Vec::new(),
        included_writeback_idempotency_keys: Vec::new(),
        source_commit_or_repo_ref: Some("c706242d36f1c275e05d8a132778491da08f61c7".to_owned()),
        include_preview_context: false,
    }
}

fn active_consent_engine() -> ConsentEngine {
    ConsentEngine::default()
        .with_bailment(
            TENANT_ID,
            BailmentState::Active {
                bailor: exo_core::Did::new("did:exo:bailor").expect("bailor"),
                bailee: exo_core::Did::new(AGENT_DID).expect("bailee"),
                scope: "dag-db:writeback".into(),
            },
        )
        .with_consent_record(DagDbConsentRecord {
            tenant_id: TENANT_ID.to_owned(),
            agent_did: AGENT_DID.to_owned(),
            purpose: ConsentPurpose::Writeback,
            active: true,
        })
        .with_consent_record(DagDbConsentRecord {
            tenant_id: TENANT_ID.to_owned(),
            agent_did: EXPORTER_DID.to_owned(),
            purpose: ConsentPurpose::Writeback,
            active: true,
        })
}

async fn import_signature(
    pool: &PgPool,
    keypair: &KeyPair,
    request: &DagDbImportRequest,
) -> String {
    let selection =
        build_persistent_graph_context_selection(pool, &selection_request_for_import(request))
            .await
            .expect("selection for import signature");
    sign_write_payload(
        keypair,
        &usage_event_payload_hash(&selection.selection).expect("import payload hash"),
    )
    .expect("import signature")
}

async fn export_signature(
    pool: &PgPool,
    keypair: &KeyPair,
    request: &DagDbExportRequest,
) -> String {
    let selection =
        build_persistent_graph_context_selection(pool, &selection_request_for_export(request))
            .await
            .expect("selection for export signature");
    sign_write_payload(
        keypair,
        &usage_event_payload_hash(&selection.selection).expect("export payload hash"),
    )
    .expect("export signature")
}

fn selection_request_for_import(
    request: &DagDbImportRequest,
) -> exo_api::dagdb::DagDbGraphContextSelectionRequest {
    exo_api::dagdb::DagDbGraphContextSelectionRequest {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.idempotency_key.clone(),
        task: format!("import:{}", request.source_hash),
        task_hash: request.source_hash.clone(),
        token_budget: 2_048,
        max_memory_refs: 1,
        catalog_hints: Vec::new(),
        requested_memory_ids: Vec::new(),
        force_revalidate: false,
    }
}

fn selection_request_for_export(
    request: &DagDbExportRequest,
) -> exo_api::dagdb::DagDbGraphContextSelectionRequest {
    let max_memory_refs = u32::try_from(request.included_memory_ids.len())
        .unwrap_or(u32::MAX)
        .clamp(1, 64);
    exo_api::dagdb::DagDbGraphContextSelectionRequest {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.idempotency_key.clone(),
        task: format!("export:{}", request.db_set_version),
        task_hash: export_request_hash(request).to_string(),
        token_budget: 2_048,
        max_memory_refs,
        catalog_hints: request.included_graph_styles.clone(),
        requested_memory_ids: request.included_memory_ids.clone(),
        force_revalidate: false,
    }
}

fn export_request_hash(request: &DagDbExportRequest) -> Hash256 {
    let body = serde_json::to_value(request).expect("export request json");
    let mut canonical_body = Vec::new();
    ciborium::ser::into_writer(&body, &mut canonical_body).expect("canonical export request");
    RequestHashMaterial {
        route_name: "dagdb.export".to_owned(),
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        canonical_redacted_request_body: canonical_body,
    }
    .hash()
    .expect("export request hash")
}

/// Collect the persisted keyword texts for the usage-event memory row a
/// writeback created, keyed by the selection's deterministic `task_hash`
/// (the usage memory row stores `source_hash = decode(task_hash, 'hex')`).
async fn memory_keyword_texts(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    selection: &exo_api::dagdb::DagDbGraphContextSelectionResponse,
) -> Vec<String> {
    let source_hash = hex::decode(&selection.task_hash).expect("decode selection task hash");
    let rows: Vec<JsonValue> = sqlx::query_scalar(
        "SELECT keywords FROM dagdb_memory_objects \
         WHERE tenant_id = $1 AND namespace = $2 AND source_hash = $3",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(source_hash)
    .fetch_all(pool)
    .await
    .expect("fetch usage memory keywords");
    rows.iter()
        .flat_map(|keywords| keywords.as_array().cloned().unwrap_or_default())
        .filter_map(|keyword| {
            keyword
                .get("text")
                .and_then(JsonValue::as_str)
                .map(str::to_owned)
        })
        .collect()
}

async fn receipt_count(pool: &PgPool, tenant_id: &str, namespace: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM dagdb_receipts WHERE tenant_id = $1 AND namespace = $2",
    )
    .bind(tenant_id)
    .bind(namespace)
    .fetch_one(pool)
    .await
    .expect("count receipts")
}

async fn memory_object_count(pool: &PgPool, tenant_id: &str, namespace: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM dagdb_memory_objects WHERE tenant_id = $1 AND namespace = $2",
    )
    .bind(tenant_id)
    .bind(namespace)
    .fetch_one(pool)
    .await
    .expect("count memory objects")
}

async fn idempotency_count(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    route_name: &str,
) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM dagdb_idempotency_keys \
         WHERE tenant_id = $1 AND namespace = $2 AND route_name = $3",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(route_name)
    .fetch_one(pool)
    .await
    .expect("count idempotency keys")
}

fn database_url_with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { "&" } else { "?" };
    format!("{database_url}{separator}options=-csearch_path%3D{schema}%2Cpublic")
}

fn configured_database_url(test_name: &str) -> Option<String> {
    match std::env::var(KG_IMPORT_DATABASE_URL_ENV) {
        Ok(database_url) => Some(database_url),
        Err(std::env::VarError::NotPresent) => {
            eprintln!(
                "skipping {test_name}: {KG_IMPORT_DATABASE_URL_ENV} is unset; live DAG DB gateway integration coverage not run"
            );
            None
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            panic!("{KG_IMPORT_DATABASE_URL_ENV} must be valid Unicode")
        }
    }
}

fn scoped_post<T>(
    path: &str,
    action: &str,
    body: &T,
    write_signature: Option<String>,
) -> Request<Body>
where
    T: serde::Serialize,
{
    scoped_post_with_bearer(BEARER, path, action, body, write_signature)
}

fn scoped_post_with_bearer<T>(
    bearer: &str,
    path: &str,
    action: &str,
    body: &T,
    write_signature: Option<String>,
) -> Request<Body>
where
    T: serde::Serialize,
{
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header("x-exo-tenant-id", TENANT_ID)
        .header("x-exo-namespace", NAMESPACE)
        .header(
            "x-exo-authority-scope",
            format!("{action}:{TENANT_ID}:{NAMESPACE}"),
        )
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(signature) = write_signature {
        builder = builder.header("x-exo-write-signature", signature);
    }
    builder
        .body(Body::from(
            serde_json::to_vec(body).expect("serialize body"),
        ))
        .expect("request")
}

fn scoped_get(path: &str, action: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {BEARER}"))
        .header("x-exo-tenant-id", TENANT_ID)
        .header("x-exo-namespace", NAMESPACE)
        .header(
            "x-exo-authority-scope",
            format!("{action}:{TENANT_ID}:{NAMESPACE}"),
        )
        .body(Body::empty())
        .expect("request")
}

async fn insert_session_user(pool: &PgPool, token: &str, did: &str, tenant_id: &str) {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await
        .expect("delete test session");
    sqlx::query("DELETE FROM users WHERE did = $1")
        .bind(did)
        .execute(pool)
        .await
        .expect("delete test user");
    sqlx::query(
        "INSERT INTO users \
         (did, display_name, email, roles, tenant_id, created_at, status, pace_status, \
          password_hash, salt, mfa_enabled) \
         VALUES ($1, $2, $3, '[]'::jsonb, $4, $5, 'Active', 'Unenrolled', 'hash', 'salt', false)",
    )
    .bind(did)
    .bind("DAG DB Agent")
    .bind(format!("{did}@example.invalid"))
    .bind(tenant_id)
    .bind(10_000_i64)
    .execute(pool)
    .await
    .expect("insert test user");
    sqlx::query(
        "INSERT INTO sessions (token, actor_did, created_at, expires_at, revoked) \
         VALUES ($1, $2, $3, $4, false)",
    )
    .bind(token)
    .bind(did)
    .bind(10_000_i64)
    .bind(4_102_444_800_000_i64)
    .execute(pool)
    .await
    .expect("insert test session");
}

fn unauthenticated_post<T>(path: &str, body: &T) -> Request<Body>
where
    T: serde::Serialize,
{
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(body).expect("serialize body"),
        ))
        .expect("request")
}

async fn response_json<T: DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

async fn assert_live_scaffold_fail_closed(response: axum::response::Response, route_name: &str) {
    let status = response.status();
    let body: DagDbErrorEnvelope = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{route_name} must fail closed instead of returning scaffold success: {body:?}"
    );
    assert_eq!(body.error_code, "database_unavailable");
    assert!(
        body.message
            .contains("requires a configured production database"),
        "{route_name} error message must identify missing governed persistence: {}",
        body.message
    );
    assert!(!body.requires_council_review);
    assert_no_forbidden_material(&body.message);
}

fn dagdb_fixtures() -> JsonValue {
    serde_json::from_str(include_str!(
        "../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
    ))
    .expect("parse complete DAG DB fixture set")
}

fn dagdb_fixture<T>(fixtures: &JsonValue, name: &str) -> T
where
    T: DeserializeOwned,
{
    serde_json::from_value(
        fixtures
            .get("requests")
            .and_then(|requests| requests.get(name))
            .unwrap_or_else(|| panic!("missing fixture requests.{name}"))
            .clone(),
    )
    .unwrap_or_else(|err| panic!("parse fixture requests.{name}: {err}"))
}

fn assert_no_forbidden_material(payload: &str) {
    for forbidden in ["postgres://", "postgresql://", "fn raw_payload()"] {
        assert!(
            !payload.contains(forbidden),
            "response must not expose forbidden material ({forbidden})"
        );
    }
}

fn base_report() -> JsonValue {
    json!({
        "schema_version": KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
        "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
        "graph_root": "KnowledgeGraphs/dag-db",
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "actor_did": AGENT_DID,
        "batch_id": h(0x01),
        "dry_run_only": true,
        "postgres_writes": false,
        "raw_markdown_included": false,
        "proposed_memory_records": [
            memory(0x10, 0x20, "KnowledgeGraphs/dag-db/00_Index.md", "00_Index"),
            memory(0x11, 0x21, "KnowledgeGraphs/dag-db/01_Project_Brief.md", "01_Project_Brief")
        ],
        "proposed_catalog_entries": [
            catalog(0x30, 0x10, 0x20, "00_Index"),
            catalog(0x31, 0x11, 0x21, "01_Project_Brief")
        ],
        "proposed_graph_nodes": [
            graph_node(0x40, 0x10),
            graph_node(0x41, 0x11)
        ],
        "proposed_graph_edges": [
            {
                "graph_edge_id": h(0x50),
                "tenant_id": TENANT_ID,
                "namespace": NAMESPACE,
                "graph_style": "semantic_catalog_graph",
                "from_memory_id": h(0x10),
                "to_memory_id": h(0x11),
                "edge_kind": "related_to",
                "source_edge_kind": "wikilink",
                "receipt_intent_id": h(0x91)
            }
        ],
        "proposed_required_edges": [
            {
                "required_edge_id": h(0x50),
                "tenant_id": TENANT_ID,
                "namespace": NAMESPACE,
                "graph_style": "semantic_catalog_graph",
                "from_memory_id": h(0x10),
                "to_memory_id": h(0x11),
                "edge_kind": "related_to",
                "status": "proposed"
            }
        ],
        "proposed_placement_decisions": [
            placement(0x60, 0x10, 0xa0),
            placement(0x61, 0x11, 0xa1)
        ],
        "proposed_receipt_intents": [
            receipt(0x80, "memory", 0x10, "intake_created"),
            receipt(0x81, "memory", 0x11, "intake_created"),
            receipt(0x82, "catalog", 0x30, "memory_approved"),
            receipt(0x83, "catalog", 0x31, "memory_approved"),
            receipt(0x84, "validation_report", 0x70, "validation_created"),
            receipt(0x85, "validation_report", 0x71, "validation_created"),
            receipt(0x91, "memory", 0x50, "validation_created")
        ],
        "proposed_validation_reports": [
            validation_report(0x70, 0x10),
            validation_report(0x71, 0x11)
        ],
        "proposed_governance_reviews": [],
        "proposed_graph_view_refreshes": [],
        "proposed_route_invalidations": [],
        "proposed_subdag_boundaries": [],
        "rollback_plan": {},
        "placement_governance_summary": {},
        "review_items": [],
        "warnings": [],
        "errors": []
    })
}

fn h(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn safe(text: &str) -> JsonValue {
    json!({
        "decision": "allow",
        "text": text,
        "redaction_codes": [],
        "original_hash": h(0xef),
        "truncated": false,
        "byte_len": text.len()
    })
}

fn memory(id: u8, source: u8, path: &str, title: &str) -> JsonValue {
    json!({
        "memory_id": h(id),
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "source_path": path,
        "candidate_id": title,
        "node_type": "source",
        "source_type": "generated",
        "source_hash": h(source),
        "payload_hash": h(id.wrapping_add(0x20)),
        "owner_did": "did:exo:kg-importer",
        "controller_did": "did:exo:kg-importer",
        "submitted_by_did": "did:exo:kg-importer",
        "consent_purpose": "retrieval",
        "title": safe(title),
        "summary": safe("summary for governed memory context"),
        "keywords": [],
        "catalog_path": ["KnowledgeGraphs", "dag-db"],
        "risk_class": "R1",
        "risk_bp": 100,
        "validation_status": "pending",
        "council_status": "not_required",
        "dag_finality_status": "pending",
        "status": "pending",
        "receipt_intent_id": h(id.wrapping_add(0x70))
    })
}

fn catalog(id: u8, memory_id: u8, source: u8, title: &str) -> JsonValue {
    json!({
        "catalog_id": h(id),
        "memory_id": h(memory_id),
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "catalog_path": ["KnowledgeGraphs", "dag-db"],
        "catalog_level": 2,
        "title": safe(title),
        "summary": safe("catalog summary"),
        "payload_hash": h(id.wrapping_add(0x20)),
        "source_hash": h(source),
        "status": "pending",
        "validation_status": "pending",
        "council_status": "not_required",
        "dag_finality_status": "pending",
        "receipt_intent_id": h(id.wrapping_add(0x52))
    })
}

fn graph_node(id: u8, memory_id: u8) -> JsonValue {
    json!({
        "graph_node_id": h(id),
        "memory_id": h(memory_id),
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "graph_style": "semantic_catalog_graph",
        "node_kind": "canonical",
        "catalog_path": ["KnowledgeGraphs", "dag-db"]
    })
}

fn placement(id: u8, memory_id: u8, receipt_id: u8) -> JsonValue {
    json!({
        "placement_decision_id": h(id),
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "input_memory_id": h(memory_id),
        "placement_trace": required_trace(),
        "canonicalization_decision": {
            "decision_kind": "new_canonical",
            "decision_reason": "synthetic fixture",
            "confidence_bp": 0,
            "risk_class": "R1",
            "validator_status": "pending",
            "matched_memory_ids": [],
            "canonical_memory_id": null,
            "required_edges_to_create": []
        },
        "similarity_results": [],
        "validator_report": h(memory_id.wrapping_add(0x60)),
        "receipt_intent_id": h(receipt_id)
    })
}

fn receipt(id: u8, subject_kind: &str, subject_id: u8, event_type: &str) -> JsonValue {
    json!({
        "receipt_intent_id": h(id),
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "subject_kind": subject_kind,
        "subject_id": h(subject_id),
        "event_type": event_type,
        "actor_did": AGENT_DID,
        "reason": "synthetic fixture"
    })
}

fn validation_report(id: u8, subject_id: u8) -> JsonValue {
    json!({
        "validation_report_id": h(id),
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "subject_kind": "memory",
        "subject_id": h(subject_id),
        "validator_did": "did:exo:kg-importer",
        "input_hash": h(id.wrapping_add(0x10)),
        "policy_hash": h(id.wrapping_add(0x20)),
        "validation_status": "pending",
        "risk_class": "R1",
        "risk_bp": 100,
        "decision": "allow",
        "notes": safe("synthetic validation")
    })
}
