#![cfg(feature = "postgres")]
#![allow(clippy::expect_used, clippy::panic)]

use std::{process, str::FromStr};

use exo_dag_db_postgres::postgres::{
    DAGDB_EXPORT_SCHEMA_SQL, DAGDB_GRAPH_SCHEMA_SQL, DAGDB_PRD17_CONTEXT_PACKET_SCHEMA_SQL,
    DAGDB_PRD17_DEFAULT_ROUTE_SCHEMA_SQL, DAGDB_PRD17_LIFECYCLE_SCHEMA_SQL, DAGDB_SCHEMA_SQL,
    DAGDB_TENANT_RLS_SCHEMA_SQL, begin_tenant_transaction,
};
use serde_json::json;
use sqlx::{
    Connection, PgConnection, PgPool, Postgres, Transaction,
    postgres::{PgConnectOptions, PgPoolOptions},
};

const TEST_DATABASE_URL_ENV: &str = "EXO_DAGDB_TEST_DATABASE_URL";
const CI_DATABASE_URL_ENV: &str = "DATABASE_URL";
const RLS_TEST_ROLE_PASSWORD: &str = "dagdb_rls_live_path_password";

struct TenantTable {
    name: &'static str,
    expected_tenant_a_rows: i64,
}

const TENANT_TABLES: &[TenantTable] = &[
    TenantTable {
        name: "dagdb_receipts",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_subject_receipt_heads",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_memory_objects",
        expected_tenant_a_rows: 2,
    },
    TenantTable {
        name: "dagdb_catalog_entries",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_route_receipts",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_context_packets",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_validation_reports",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_idempotency_keys",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_dag_outbox",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_graph_nodes",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_graph_edges",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_graph_layers",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_exports",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_default_routes",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_context_packet_records",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_lifecycle_actions",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_route_invalidation_events",
        expected_tenant_a_rows: 1,
    },
    TenantTable {
        name: "dagdb_continuation_records",
        expected_tenant_a_rows: 1,
    },
];

#[tokio::test]
async fn rls_requires_bound_tenant_context_for_live_path_tables() {
    let Some(db) = TestDb::maybe_new("rls_live_missing_context").await else {
        return;
    };
    apply_live_path_schema(&db.pool).await;
    let rls_pool = RlsCheckedPool::new(&db, "missing_context").await;
    seed_live_path_rows(&rls_pool.pool, "tenant-a")
        .await
        .expect("seed tenant rows through tenant-bound transaction");

    for table in TENANT_TABLES {
        let query = format!("SELECT count(*) FROM {}", table.name);
        let result = sqlx::query_scalar::<_, i64>(&query)
            .fetch_one(&rls_pool.pool)
            .await;
        assert!(
            result.is_err(),
            "unbound tenant context must fail closed for {}",
            table.name
        );
    }

    rls_pool.cleanup(&db.pool).await;
}

#[tokio::test]
async fn rls_blocks_cross_tenant_reads_and_writes_for_live_path_tables() {
    let Some(db) = TestDb::maybe_new("rls_live_cross_tenant").await else {
        return;
    };
    apply_live_path_schema(&db.pool).await;
    let rls_pool = RlsCheckedPool::new(&db, "cross_tenant").await;
    seed_live_path_rows(&rls_pool.pool, "tenant-a")
        .await
        .expect("seed tenant rows through tenant-bound transaction");

    for table in TENANT_TABLES {
        let mut tenant_a_tx = begin_tenant_transaction(&rls_pool.pool, "tenant-a")
            .await
            .expect("begin tenant-a transaction");
        let tenant_a_count = count_tenant_rows(&mut tenant_a_tx, table.name, "tenant-a").await;
        tenant_a_tx
            .commit()
            .await
            .expect("commit tenant-a visibility check");
        assert_eq!(
            tenant_a_count, table.expected_tenant_a_rows,
            "tenant-a must see its own seeded rows for {}",
            table.name
        );

        let mut tenant_b_tx = begin_tenant_transaction(&rls_pool.pool, "tenant-b")
            .await
            .expect("begin tenant-b transaction");
        let tenant_b_read_count = count_tenant_rows(&mut tenant_b_tx, table.name, "tenant-a").await;
        assert_eq!(
            tenant_b_read_count, 0,
            "tenant-b must not read tenant-a rows for {}",
            table.name
        );
        let update_query = format!(
            "UPDATE {} SET tenant_id = tenant_id WHERE tenant_id = 'tenant-a'",
            table.name
        );
        let updated = sqlx::query(&update_query)
            .execute(&mut *tenant_b_tx)
            .await
            .expect("cross-tenant update is filtered by RLS")
            .rows_affected();
        tenant_b_tx
            .commit()
            .await
            .expect("commit tenant-b isolation check");
        assert_eq!(
            updated, 0,
            "tenant-b must not update tenant-a rows for {}",
            table.name
        );
    }

    assert_tenant_mismatch_insert_is_rejected(&rls_pool.pool).await;
    rls_pool.cleanup(&db.pool).await;
}

async fn count_tenant_rows(
    tx: &mut Transaction<'_, Postgres>,
    table_name: &str,
    tenant_id: &str,
) -> i64 {
    let query = format!("SELECT count(*) FROM {table_name} WHERE tenant_id = $1");
    sqlx::query_scalar::<_, i64>(&query)
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .expect("count tenant rows")
}

async fn assert_tenant_mismatch_insert_is_rejected(pool: &PgPool) {
    let mut tx = begin_tenant_transaction(pool, "tenant-b")
        .await
        .expect("begin tenant-b mismatch insert transaction");
    let result = sqlx::query(
        "INSERT INTO dagdb_default_routes \
         (tenant_id, project_id, memory_namespace, route_id, status, route_source, policy_ref, \
          freshness_ref, policy_allowed, freshness_status, invalidated, \
          production_default_route_approval_status, packet_quality_review_status, \
          selected_memory_refs, selected_memory_ref_count, created_at, updated_at) \
         VALUES ('tenant-a', 'project-rls', 'default', 'route-mismatch', 'active', 'persisted', \
                 'policy-rls', 'freshness-rls', true, 'current', false, 'approved', 'approved', \
                 '[]'::jsonb, 0, '2026-06-19T00:00:00Z', '2026-06-19T00:00:00Z')",
    )
    .execute(&mut *tx)
    .await;
    assert!(
        result.is_err(),
        "WITH CHECK must reject inserting tenant-a rows while bound as tenant-b"
    );
}

async fn seed_live_path_rows(pool: &PgPool, tenant_id: &str) -> sqlx::Result<()> {
    let mut tx = begin_tenant_transaction(pool, tenant_id).await?;
    let empty_refs = json!([]);
    let empty_object = json!({});
    let safe_text = json!({
        "decision": "allow",
        "text": "safe",
        "redaction_codes": [],
        "original_hash": "caac13844969e521bb8bfcf8bc706ad54bcce3e3f260368eda31bdb0542d00e1",
        "truncated": false,
        "byte_len": 4
    });

    sqlx::query(
        "INSERT INTO dagdb_receipts \
         (receipt_hash, tenant_id, namespace, subject_kind, subject_id, prev_receipt_hash, seq, \
          event_type, actor_did, event_hlc_physical_ms, event_hlc_logical, event_hash, \
          receipt_body, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, 'default', 'memory', $3, $4, 1, 'intake_created', 'did:exo:rls', \
                 1, 0, $5, $6, 1, 0)",
    )
    .bind(hb(0x01))
    .bind(tenant_id)
    .bind(hb(0x21))
    .bind(vec![0_u8; 32])
    .bind(hb(0x31))
    .bind(json!({"event": "intake_created"}))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_subject_receipt_heads \
         (tenant_id, namespace, subject_kind, subject_id, latest_receipt_hash, latest_seq, \
          updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, 'default', 'memory', $2, $3, 1, 1, 0)",
    )
    .bind(tenant_id)
    .bind(hb(0x21))
    .bind(hb(0x01))
    .execute(&mut *tx)
    .await?;
    for (memory_id, payload_hash, source_hash) in [(0x40, 0x41, 0x42), (0x43, 0x44, 0x45)] {
        sqlx::query(
            "INSERT INTO dagdb_memory_objects \
             (memory_id, tenant_id, namespace, node_type, source_type, consent_purpose, \
              payload_hash, source_hash, owner_did, controller_did, submitted_by_did, title, \
              summary, keywords, risk_class, risk_bp, status, validation_status, council_status, \
              latest_receipt_hash, created_at_physical_ms, created_at_logical, \
              updated_at_physical_ms, updated_at_logical) \
             VALUES ($1, $2, 'default', 'source', 'public_web', 'retrieval', $3, $4, \
                     'did:example:owner', 'did:example:controller', 'did:example:submitter', \
                     $5, $5, $6, 'R0', 0, 'routable', 'passed', 'not_required', $7, 1, 0, 1, 0)",
        )
        .bind(hb(memory_id))
        .bind(tenant_id)
        .bind(hb(payload_hash))
        .bind(hb(source_hash))
        .bind(&safe_text)
        .bind(&empty_refs)
        .bind(hb(0x01))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO dagdb_catalog_entries \
         (catalog_id, tenant_id, namespace, memory_id, catalog_level, title, summary, keywords, \
          payload_hash, source_hash, status, validation_status, council_status, latest_receipt_hash, \
          created_at_physical_ms, created_at_logical, updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, 'default', $3, 0, $4, $4, $5, $6, $7, 'routable', 'passed', \
                 'not_required', $8, 1, 0, 1, 0)",
    )
    .bind(hb(0x50))
    .bind(tenant_id)
    .bind(hb(0x40))
    .bind(&safe_text)
    .bind(&empty_refs)
    .bind(hb(0x51))
    .bind(hb(0x52))
    .bind(hb(0x01))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_route_receipts \
         (route_id, tenant_id, namespace, requesting_agent_did, task_signature_hash, \
          approved_scope_hash, candidate_memory_ids, selected_memory_ids, route_score_bp, \
          token_budget, token_estimate, risk_bp, status, validation_status, council_status, \
          stale_at_physical_ms, stale_at_logical, latest_receipt_hash, created_at_physical_ms, \
          created_at_logical) \
         VALUES ($1, $2, 'default', 'did:example:agent', $3, $4, $5, $5, 9000, 4096, 256, \
                 0, 'active', 'passed', 'not_required', 90, 0, $6, 1, 0)",
    )
    .bind(hb(0x60))
    .bind(tenant_id)
    .bind(hb(0x61))
    .bind(hb(0x62))
    .bind(&empty_refs)
    .bind(hb(0x01))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_context_packets \
         (context_packet_id, tenant_id, namespace, request_id, route_id, task_hash, \
          requesting_agent_did, memory_refs, packet_hash, token_budget, token_estimate, \
          validation_status, council_status, latest_receipt_hash, created_at_physical_ms, \
          created_at_logical) \
         VALUES ($1, $2, 'default', 'request-rls', $3, $4, 'did:example:agent', $5, $6, \
                 4096, 256, 'passed', 'not_required', $7, 1, 0)",
    )
    .bind(hb(0x70))
    .bind(tenant_id)
    .bind(hb(0x60))
    .bind(hb(0x71))
    .bind(&empty_refs)
    .bind(hb(0x72))
    .bind(hb(0x01))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_validation_reports \
         (validation_report_id, tenant_id, namespace, subject_kind, subject_id, validator_did, \
          input_hash, policy_hash, validation_status, risk_class, risk_bp, decision, notes, \
          contradictory_report_ids, latest_receipt_hash, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, 'default', 'memory', $3, 'did:example:validator', $4, $5, 'passed', \
                 'R0', 0, 'allow', $6, $7, $8, 1, 0)",
    )
    .bind(hb(0x80))
    .bind(tenant_id)
    .bind(hb(0x40))
    .bind(hb(0x81))
    .bind(hb(0x82))
    .bind(&safe_text)
    .bind(&empty_refs)
    .bind(hb(0x01))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_idempotency_keys \
         (tenant_id, namespace, route_name, idempotency_key, request_hash, response_hash, \
          response_body, status_code, cached_failure, created_at_physical_ms, created_at_logical, \
          expires_at_physical_ms, expires_at_logical) \
         VALUES ($1, 'default', 'dagdb:import', 'idempotency-rls', $2, $3, $4, 201, false, 1, 0, 2, 0)",
    )
    .bind(tenant_id)
    .bind(hb(0x90))
    .bind(hb(0x91))
    .bind(json!({"status": "created"}))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_dag_outbox \
         (outbox_id, tenant_id, namespace, subject_kind, subject_id, dag_write_id, \
          dag_payload_hash, dag_finality_status, created_at_physical_ms, created_at_logical, \
          updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, 'default', 'memory', $3, 'dag-write-rls', $4, 'pending', 1, 0, 1, 0)",
    )
    .bind(hb(0x95))
    .bind(tenant_id)
    .bind(hb(0x40))
    .bind(hb(0x96))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_graph_nodes \
         (graph_node_id, tenant_id, namespace, memory_id, graph_style, node_kind, metadata, \
          created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, 'default', $3, 'canonical_memory_graph', 'canonical', $4, 1, 0)",
    )
    .bind(hb(0xa0))
    .bind(tenant_id)
    .bind(hb(0x40))
    .bind(&empty_object)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_graph_edges \
         (graph_edge_id, tenant_id, namespace, graph_style, from_memory_id, to_memory_id, \
          edge_kind, created_at_physical_ms, created_at_logical) \
         VALUES ($1, $2, 'default', 'canonical_memory_graph', $3, $4, 'related_to', 1, 0)",
    )
    .bind(hb(0xa1))
    .bind(tenant_id)
    .bind(hb(0x40))
    .bind(hb(0x43))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_graph_layers \
         (layer_id, tenant_id, namespace, root_memory_id, layer_depth, layer_kind, graph_style, \
          layer_path, metadata, created_at_physical_ms, created_at_logical, \
          updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, 'default', $3, 0, 'root', 'canonical_memory_graph', 'root', $4, 1, 0, 1, 0)",
    )
    .bind(hb(0xa2))
    .bind(tenant_id)
    .bind(hb(0x40))
    .bind(&empty_object)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_exports \
         (export_id, tenant_id, namespace, schema_version, export_scope_hash, included_memory_ids_hash, \
          included_receipt_heads_hash, section_hashes, section_counts, citation_index_hash, \
          provenance_index_hash, redaction_summary_hash, omission_summary_hash, verification_hash, \
          whole_export_hash, export_status, requester_did, created_at_physical_ms, created_at_logical, \
          updated_at_physical_ms, updated_at_logical) \
         VALUES ($1, $2, 'default', 'dagdb_kg_portable_export_v1', $3, $4, $5, $6, $6, $7, $8, \
                 $9, $10, $11, $12, 'verified', 'did:example:requester', 1, 0, 1, 0)",
    )
    .bind(hb(0xb0))
    .bind(tenant_id)
    .bind(hb(0xb1))
    .bind(hb(0xb2))
    .bind(hb(0xb3))
    .bind(&empty_object)
    .bind(hb(0xb4))
    .bind(hb(0xb5))
    .bind(hb(0xb6))
    .bind(hb(0xb7))
    .bind(hb(0xb8))
    .bind(hb(0xb9))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_default_routes \
         (tenant_id, project_id, memory_namespace, route_id, status, route_source, policy_ref, \
          freshness_ref, policy_allowed, freshness_status, invalidated, \
          production_default_route_approval_status, packet_quality_review_status, selected_memory_refs, \
          selected_memory_ref_count, created_at, updated_at) \
         VALUES ($1, 'project-rls', 'default', 'route-rls', 'active', 'persisted', 'policy-rls', \
                 'freshness-rls', true, 'current', false, 'approved', 'approved', $2, 0, \
                 '2026-06-19T00:00:00Z', '2026-06-19T00:00:00Z')",
    )
    .bind(tenant_id)
    .bind(&empty_refs)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_context_packet_records \
         (packet_id, route_id, query_hash, tenant_id, project_id, memory_namespace, selected_memory_ids, \
          selected_edge_ids, token_budget, token_estimate, context_quality, citation_coverage_bp, \
          validation_coverage_bp, freshness_status, validation_status, source_proof_refs, \
          idempotency_key, persistence_status, production_default_route_approval_status, \
          packet_quality_review_status, created_at) \
         VALUES ('packet-rls', 'route-rls', 'query-rls', $1, 'project-rls', 'default', $2, $2, \
                 4096, 256, 'usable_context', 10000, 10000, 'current', 'passed', $2, \
                 'packet-idempotency-rls', 'persisted', 'approved', 'approved', \
                 '2026-06-19T00:00:00Z')",
    )
    .bind(tenant_id)
    .bind(&empty_refs)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_lifecycle_rollbacks \
         (rollback_id, action_id, inverse_action_type, before_refs, after_refs, validation_ref, \
          operator_required, rollback_body) \
         VALUES ('rollback-rls', 'action-rls', 'writeback', $1, $1, 'validation-rls', false, $2)",
    )
    .bind(&empty_refs)
    .bind(json!({"rollback": "noop"}))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_lifecycle_actions \
         (action_id, action_type, tenant_id, project_id, memory_namespace, actor_id, source_packet_id, \
          source_receipt_id, target_memory_ids, parent_memory_ids, validation_report_id, policy_ref, \
          rollback_id, route_invalidation_event_ids, evidence_refs, terminal_state, \
          production_lifecycle_approval, idempotency_key, action_body, created_at) \
         VALUES ('action-rls', 'writeback', $1, 'project-rls', 'default', 'did:example:actor', \
                 'packet-rls', 'receipt-rls', $2, $2, 'validation-rls', 'policy-rls', \
                 'rollback-rls', $2, $2, 'accepted', 'approved', 'lifecycle-idempotency-rls', \
                 $3, '2026-06-19T00:00:00Z')",
    )
    .bind(tenant_id)
    .bind(&empty_refs)
    .bind(json!({"action": "writeback"}))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_route_invalidation_events \
         (event_id, tenant_id, project_id, memory_namespace, route_id, source_action_id, \
          impacted_memory_ids, reason, invalidated_packet_ids, freshness_state_before, \
          freshness_state_after, retrieval_readiness_impact, validation_report_id, rollback_ref, \
          idempotency_key, event_body, created_at) \
         VALUES ('route-invalidation-rls', $1, 'project-rls', 'default', 'route-rls', \
                 'action-rls', $2, 'memory changed', $2, 'current', 'stale', \
                 'reject_until_rebuilt', 'validation-rls', 'rollback-rls', \
                 'route-invalidation-idempotency-rls', $3, '2026-06-19T00:00:00Z')",
    )
    .bind(tenant_id)
    .bind(&empty_refs)
    .bind(json!({"event": "route_invalidated"}))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO dagdb_continuation_records \
         (continuation_id, task_id, tenant_id, project_id, memory_namespace, summary_ref, memory_refs, \
          blocker_refs, validation_refs, expiry_epoch_seconds, later_retrieval_status, \
          production_lifecycle_approval, idempotency_key, record_body, created_at) \
         VALUES ('continuation-rls', 'task-rls', $1, 'project-rls', 'default', 'summary-rls', \
                 $2, $2, $2, 1, 'pending', 'approved', 'continuation-idempotency-rls', \
                 $3, '2026-06-19T00:00:00Z')",
    )
    .bind(tenant_id)
    .bind(&empty_refs)
    .bind(json!({"continuation": "pending"}))
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

async fn apply_live_path_schema(pool: &PgPool) {
    for migration_sql in [
        DAGDB_GRAPH_SCHEMA_SQL,
        DAGDB_EXPORT_SCHEMA_SQL,
        DAGDB_PRD17_DEFAULT_ROUTE_SCHEMA_SQL,
        DAGDB_PRD17_CONTEXT_PACKET_SCHEMA_SQL,
        DAGDB_PRD17_LIFECYCLE_SCHEMA_SQL,
        DAGDB_TENANT_RLS_SCHEMA_SQL,
    ] {
        sqlx::raw_sql(migration_sql)
            .execute(pool)
            .await
            .expect("apply DAG DB live-path migration");
    }
}

fn hb(byte: u8) -> Vec<u8> {
    vec![byte; 32]
}

struct TestDb {
    pool: PgPool,
    schema: String,
    database_url: String,
}

impl TestDb {
    async fn maybe_new(label: &str) -> Option<Self> {
        let Some((database_url, source_env)) = test_database_url() else {
            eprintln!(
                "skipping DAG DB tenant RLS live-path postgres test: neither {TEST_DATABASE_URL_ENV} nor {CI_DATABASE_URL_ENV} is set"
            );
            return None;
        };
        eprintln!("running DAG DB tenant RLS live-path postgres test with {source_env}");
        Some(Self::new_with_database_url(label, &database_url).await)
    }

    async fn new_with_database_url(label: &str, database_url: &str) -> Self {
        let schema = format!("dagdb_{label}_{}", process::id());
        let mut admin = PgConnection::connect(database_url)
            .await
            .expect("connect to configured DAG DB test database URL");
        sqlx::raw_sql(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
            .execute(&mut admin)
            .await
            .expect("drop existing RLS live-path test schema");
        sqlx::raw_sql(&format!("CREATE SCHEMA {schema}"))
            .execute(&mut admin)
            .await
            .expect("create RLS live-path test schema");

        let scoped_url = database_url_with_search_path(database_url, &schema);
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&scoped_url)
            .await
            .expect("connect RLS live-path test pool");
        sqlx::raw_sql(DAGDB_SCHEMA_SQL)
            .execute(&pool)
            .await
            .expect("apply DAG DB base schema");
        Self {
            pool,
            schema,
            database_url: database_url.to_owned(),
        }
    }
}

fn test_database_url() -> Option<(String, &'static str)> {
    for env_name in [TEST_DATABASE_URL_ENV, CI_DATABASE_URL_ENV] {
        match std::env::var(env_name) {
            Ok(value) if !value.trim().is_empty() => return Some((value, env_name)),
            Ok(_) | Err(_) => {}
        }
    }
    None
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let schema = self.schema.clone();
        let database_url = self.database_url.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("create cleanup runtime");
            runtime.block_on(async move {
                let mut conn = PgConnection::connect(&database_url)
                    .await
                    .expect("connect for RLS live-path cleanup");
                sqlx::raw_sql(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
                    .execute(&mut conn)
                    .await
                    .expect("drop RLS live-path test schema");
            });
        })
        .join()
        .expect("join cleanup thread");
    }
}

struct RlsCheckedPool {
    pool: PgPool,
    role_name: Option<String>,
}

impl RlsCheckedPool {
    async fn new(db: &TestDb, label: &str) -> Self {
        if !pool_role_bypasses_rls(&db.pool).await {
            return Self {
                pool: db.pool.clone(),
                role_name: None,
            };
        }

        let role_name = format!("dagdb_rls_{}_{}", label, process::id());
        drop_rls_role_if_exists(&db.pool, &role_name).await;
        sqlx::raw_sql(&format!(
            "CREATE ROLE {role_name} LOGIN PASSWORD '{RLS_TEST_ROLE_PASSWORD}'"
        ))
        .execute(&db.pool)
        .await
        .expect("create RLS checked pool role");
        sqlx::raw_sql(&format!(
            "GRANT USAGE ON SCHEMA {} TO {role_name}",
            db.schema
        ))
        .execute(&db.pool)
        .await
        .expect("grant schema usage to RLS checked pool role");
        sqlx::raw_sql(&format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA {} TO {role_name}",
            db.schema
        ))
        .execute(&db.pool)
        .await
        .expect("grant table privileges to RLS checked pool role");
        sqlx::raw_sql(&format!(
            "GRANT EXECUTE ON FUNCTION {}.dagdb_current_tenant_id() TO {role_name}",
            db.schema
        ))
        .execute(&db.pool)
        .await
        .expect("grant tenant helper execution to RLS checked pool role");

        let options = PgConnectOptions::from_str(&db.database_url)
            .expect("parse RLS checked pool database URL")
            .username(&role_name)
            .password(RLS_TEST_ROLE_PASSWORD)
            .options([("search_path", format!("{},public", db.schema))]);
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .expect("connect RLS checked pool");
        Self {
            pool,
            role_name: Some(role_name),
        }
    }

    async fn cleanup(self, admin_pool: &PgPool) {
        if let Some(role_name) = self.role_name {
            self.pool.close().await;
            drop_rls_role_if_exists(admin_pool, &role_name).await;
        }
    }
}

async fn pool_role_bypasses_rls(pool: &PgPool) -> bool {
    sqlx::query_scalar("SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user")
        .fetch_one(pool)
        .await
        .expect("query pool role RLS bypass state")
}

async fn drop_rls_role_if_exists(pool: &PgPool, role_name: &str) {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = $1)")
            .bind(role_name)
            .fetch_one(pool)
            .await
            .expect("query RLS checked pool role existence");
    if !exists {
        return;
    }
    sqlx::raw_sql(&format!("DROP OWNED BY {role_name}"))
        .execute(pool)
        .await
        .expect("drop RLS checked pool role privileges");
    sqlx::raw_sql(&format!("DROP ROLE {role_name}"))
        .execute(pool)
        .await
        .expect("drop RLS checked pool role");
}

fn database_url_with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}options=-csearch_path%3D{schema}%2Cpublic")
}
