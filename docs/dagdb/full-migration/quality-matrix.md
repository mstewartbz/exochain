# Full DAG DB Migration Quality Matrix

Schema: `dagdb_full_migration_quality_matrix_v1`

This file is an auditable RED/GREEN ledger for the full DAG DB migration. A row
is complete only when its `red_command`, `red_failure`, `green_command`,
`artifact`, and `commit` fields contain evidence from this branch. A `commit`
value of `not-claimed` means the row is an active contract and must not be
reported as complete.

Definition of 100 percent TDD for this migration: every production change,
migration, adapter, source guard, route, and adjacent-surface persistence change
gets a RED test first, expected failure recorded here, then the smallest
implementation, then GREEN evidence recorded here.

## Summary Matrix

| ID | Surface | Classification | RED requirement | GREEN acceptance |
|---|---|---|---|---|
| QM-00 | Fresh base | EXOCHAIN core process | Prove original checkout dirty/gone and clean worktree from `origin/main`. | Clean branch from `origin/main` with trace docs committed first. |
| QM-01 | Matrix enforcement | Core runtime adapter | Guard fails when any matrix row lacks required evidence fields. | `cargo test -p exo-gateway quality_matrix_is_complete` passes. |
| QM-02 | DAG DB schema | EXOCHAIN core | Migration contract fails for missing root-bundle receipt and migrated state families. | Postgres migrations cover all new state families and root-bundle receipts. |
| QM-03 | Root bundle | EXOCHAIN core | Invalid, missing, or tampered root bundle refuses DAG DB record creation. | `exo-root` and AVC tests pass with immutable root-bundle receipt semantics. |
| QM-04 | Node DAG store | EXOCHAIN core | Source guard fails on production `SqliteDagStore::open(data_dir)`. | DAG DB node store persists DAG, parents, commits, certificates, receipts, economy state. |
| QM-05 | 0dentity | EXOCHAIN core | Restart test fails because memory-only state disappears. | DAG DB-backed 0dentity reloads every record and `persistence_ready()` is true. |
| QM-06 | Gateway state | Core runtime adapter | Guard fails on production public-schema gateway writes. | Gateway state writes/readbacks resolve through DAG DB store interfaces. |
| QM-07 | Full REST surface | Core runtime adapter | Router test fails because only five routes are mounted. | All documented routes mount; auth/session gates run; pool-backed promoted routes persist/read DAG DB tables; no-pool paths fail closed. |
| QM-08 | Finality | EXOCHAIN core | Self-approved route/import/export/council tests fail. | Accepted/approved state requires independent finality authority. |
| QM-09 | RLS | EXOCHAIN core | Tenant mismatch live test fails for each unlisted tenant table. | Every tenant table is RLS-protected and live mismatch test passes. |
| QM-10 | Idempotency/replay | Core runtime adapter | Duplicate key and mismatched request-hash tests fail for new routes. | Replay is stable; mismatched replay rejects mutation with receipt evidence. |
| QM-11 | SDK/MCP parity | Core runtime adapter | Compile/source tests fail for missing route helpers/tools. | SDK and MCP expose all REST routes with typed requests and fail-closed errors. |
| QM-12 | CommandBase | Adjacent surface | Guard fails on production SQLite and durable browser state. | CommandBase durable state uses DAG DB adapter; SQLite removed from production. |
| QM-13 | Demo | Adjacent surface | Guard fails on direct demo service `pg.Pool` writes. | Demo services call DAG DB/gateway adapters; old SQL is test fixture or gone. |
| QM-14 | Site | Adjacent surface | Guard fails on contact public tables and `CONTACT_DATABASE_URL`. | Contact intake writes DAG DB namespace records; disclosure/rate-limit tests pass. |
| QM-15 | Web durable state | Adjacent surface | Tests fail for durable product state in `localStorage`. | Durable UI state writes server/DAG DB; ephemeral preferences are classified. |
| QM-16 | CyberMedica | Adjacent surface | Adapter test fails without DAG DB/gateway evidence and simulated-trust denial. | `npm run quality` passes with DAG DB adapter evidence and no trust claim by proximity. |
| QM-17 | Source hygiene | All owned surfaces | Repo-wide guard fails on production SQLite, legacy writes, scaffold routes, or raw secrets. | Clippy, source guards, JS guards, and secret scans pass. |
| QM-18 | Coverage | All owned surfaces | Coverage gate fails if touched migration code lacks tests. | Rust coverage remains at policy; adjacent packages run their coverage gates. |
| QM-19 | Live proof | Core plus adapters | Runtime smoke fails without live DAG DB evidence. | Live Postgres proof covers write/read/lookup, RLS, unavailable DB, replay, finality. |

### QM-00

surface: Fresh base
classification: EXOCHAIN core process
red_command: `cd /Users/bobstewart/dev/exochain && git status --short --branch`
red_failure: Original checkout reports branch `bob-stewart/cybermedica-adjacent-surface-contracts-20260526...origin/bob-stewart/cybermedica-adjacent-surface-contracts-20260526 [gone]` with many modified and untracked files.
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration && git status --short --branch && git rev-parse HEAD && git rev-parse origin/main`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/docs/dagdb/full-migration/code-trace.md`
commit: `1de3bb04`

### QM-01

surface: Matrix enforcement
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway quality_matrix_is_complete`
red_failure: `quality matrix must exist at /Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/../../docs/dagdb/full-migration/quality-matrix.md: No such file or directory (os error 2)`
green_command: `cargo test -p exo-gateway quality_matrix_is_complete`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: `1de3bb04`

### QM-02

surface: DAG DB schema
classification: EXOCHAIN core
red_command: `cargo test -p exo-dag-db-postgres --features postgres root_bundle_receipts_are_global_immutable_schema_contract`
red_failure: `assertion failed: lower.contains("create table if not exists dagdb_root_bundle_receipts")`
green_command: `cargo test -p exo-dag-db-postgres --features postgres root_bundle_receipts_are_global_immutable_schema_contract` and `cargo test -p exo-dag-db-postgres --features postgres --test migration_contract`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260623000001_create_root_bundle_receipt_schema.sql`
commit: `31971776`

### QM-03

surface: Root bundle
classification: EXOCHAIN core
red_command: `cargo test -p exo-node avc_root_trust_loader_records_dagdb_receipt_after_verification_before_registry_commit`
red_failure: `root bundle DAG DB receipt must be recorded before the issuer registry commit`
green_command: `cargo test -p exo-node avc_root_trust` and `cargo test -p exo-root`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/avc.rs`
commit: `6b33a1b7`

### QM-04

surface: Node DAG store
classification: EXOCHAIN core
red_command: `cargo test -p exo-dag-db-postgres --features postgres node_store_tables_are_dagdb_schema_contract` and `cargo test -p exo-node node_production_startup_uses_dagdb_store_not_sqlite_dag_db`
red_failure: `DAG DB schema must include node-store table dagdb_node_dag_nodes`; `start_node must not open the legacy SQLite dag.db store in production`
green_command: `cargo test -p exo-dag-db-postgres --features postgres --test migration_contract`; `cargo test -p exo-node node_production_startup_uses_dagdb_store_not_sqlite_dag_db`; `cargo test -p exo-node store::tests`; `cargo clippy -p exo-node --all-targets -- -D warnings`; `cargo clippy -p exo-dag-db-postgres --features postgres --all-targets -- -D warnings`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/store.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260623000002_create_dagdb_node_store_schema.sql`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`
commit: `ea556e61`

### QM-05

surface: 0dentity
classification: EXOCHAIN core
red_command: `cargo test -p exo-dag-db-postgres --features postgres zerodentity_records_are_dagdb_schema_contract` and `cargo test -p exo-node zerodentity_restart_persists_dagdb_state`
red_failure: `DAG DB schema must include the 0dentity durable record table`; `production node startup must open 0dentity through the DAG DB-backed store`
green_command: `cargo test -p exo-dag-db-postgres --features postgres zerodentity_records_are_dagdb_schema_contract`; `cargo test -p exo-dag-db-postgres --features postgres --test migration_contract`; `cargo test -p exo-node zerodentity_restart_persists_dagdb_state`; `cargo test -p exo-node zerodentity`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/zerodentity/store.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260623000003_create_zerodentity_record_schema.sql`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`
commit: `26c5f35b`

### QM-06

surface: Gateway state
classification: Core runtime adapter
red_command: `cargo test -p exo-dag-db-postgres --features postgres gateway_state_records_are_dagdb_schema_contract` and `cargo test -p exo-gateway production_gateway_state_resolves_legacy_tables_in_dagdb_schema`
red_failure: `DAG DB schema must enumerate gateway state family did_document`; `couldn't read .../20260623000005_create_gateway_legacy_table_contracts.sql: No such file or directory`
green_command: `cargo test -p exo-dag-db-postgres --features postgres gateway_state_records_are_dagdb_schema_contract`; `cargo test -p exo-dag-db-postgres --features postgres --test migration_contract`; `cargo test -p exo-gateway production_gateway_state`; `cargo test -p exo-gateway quality_matrix_is_complete`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/db.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260623000004_create_gateway_state_records_schema.sql`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260623000005_create_gateway_legacy_table_contracts.sql`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/src/postgres/mod.rs`
commit: `24fcdd99`

### QM-07

surface: Full REST surface
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway dagdb_router_mounts_full_rest_surface_fail_closed_without_db`
red_failure: `thread 'dagdb::tests::dagdb_router_mounts_full_rest_surface_fail_closed_without_db' panicked ... assertion 'left == right' failed ... left: 404 right: 503` for POST `/api/v1/dag-db/intake`; the first RED compile also failed until the new GET assertion helper existed.
green_command: `cargo test -p exo-gateway dagdb_router && cargo test -p exo-gateway dagdb_full_rest_surface_has_governed_persistence_handlers && cargo test -p exo-gateway dagdb_handlers_cover_authorized_and_denied_branches_directly && cargo clippy -p exo-gateway --all-targets -- -D warnings && cargo fmt --all -- --check`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: `69d1360f`

### QM-08

surface: Finality
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway --features production-db dagdb_finality_requires_independent_authority`
red_failure: `error[E0425]: cannot find function 'import_finality_payload_hash' in this scope`; same RED compile failed for missing `export_finality_payload_hash` and `validate_council_decision_finality`.
green_command: `cargo test -p exo-gateway --features production-db dagdb_finality_requires_independent_authority && cargo test -p exo-gateway --features production-db import_export_authorization && cargo test -p exo-gateway --features production-db dagdb_council_decision && cargo clippy -p exo-gateway --features production-db --all-targets -- -D warnings && cargo fmt --all -- --check`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: `c88d16b9`

### QM-09

surface: RLS
classification: EXOCHAIN core
red_command: `cargo test -p exo-dag-db-postgres --features postgres rls_migration_tenant_table_list_matches_test_metadata`
red_failure: `missing from tests: ["dagdb_gateway_state_records", "dagdb_node_commit_certificates", "dagdb_node_committed", "dagdb_node_consensus_meta", "dagdb_node_consensus_votes", "dagdb_node_dag_nodes", "dagdb_node_dag_parents", "dagdb_node_economy_anchors", "dagdb_node_economy_meta", "dagdb_node_economy_objects", "dagdb_node_trust_receipts", "dagdb_node_validators", "dagdb_zerodentity_records"]`
green_command: `cargo test -p exo-dag-db-postgres --features postgres rls_migration_tenant_table_list_matches_test_metadata && cargo test -p exo-dag-db-postgres --features postgres rls_migration_source_enables_forced_tenant_policy_for_expected_tables && cargo test -p exo-dag-db-postgres --features postgres rls_requires_bound_tenant_context_for_live_path_tables && cargo test -p exo-dag-db-postgres --features postgres rls_blocks_cross_tenant_reads_and_writes_for_live_path_tables && cargo clippy -p exo-dag-db-postgres --features postgres --all-targets -- -D warnings && cargo fmt --all -- --check`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/tests/dagdb_tenant_rls_live_path_contract.rs`
commit: `7d6ca65b`

### QM-10

surface: Idempotency/replay
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway dagdb_idempotency_replay_contract`
red_failure: `persist_idempotent_intake_response must wrap dagdb.intake writes`
green_command: `cargo test -p exo-gateway dagdb_idempotency_replay_contract && cargo test -p exo-gateway --features production-db dagdb_idempotency_replay_contract && cargo test -p exo-gateway --features production-db idempotency_error_helpers_return_stable_envelopes && cargo test -p exo-gateway --features production-db idempotency_db_error_and_short_circuit_paths_fail_closed && cargo test -p exo-gateway --features production-db dagdb_router && cargo clippy -p exo-gateway --features production-db --all-targets -- -D warnings && cargo fmt --all -- --check && cargo test -p exo-gateway quality_matrix_is_complete`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/docs/dagdb/full-migration/code-trace.md`
commit: `5d691fec`

### QM-11

surface: SDK/MCP parity
classification: Core runtime adapter
red_command: `cargo test -p exochain-sdk dagdb_sdk_helpers_cover_every_route_without_shape_drift`; `cargo test -p exochain-sdk --features http-client import_export_require_independent_finality_signature_sets_before_http`; `cargo test -p exo-node mcp_dagdb_tool_surface_covers_full_rest_parity`
red_failure: SDK helper parity RED failed for missing `DagDbClient` methods `intake`, `validate`, `trust_check`, `council_decision`, `receipt_lookup`, `catalog_lookup`, and `route_lookup`; HTTP RED failed for missing import/export finality header constructors and route-specific HTTP methods; MCP RED failed for missing constants, definitions, and executors for the full REST parity tool set.
green_command: `cargo test -p exochain-sdk --features http-client dagdb && cargo test -p exo-node --features dagdb-gateway-proxy dagdb && cargo test -p exo-node --features dagdb-gateway-proxy registry_registers_and_lists && cargo test -p exo-node --features dagdb-gateway-proxy registry_get_existing && cargo clippy -p exochain-sdk --features http-client --all-targets -- -D warnings && cargo clippy -p exo-node --features dagdb-gateway-proxy --all-targets -- -D warnings && cargo fmt --all -- --check && cargo test -p exo-gateway quality_matrix_is_complete`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exochain-sdk/src/dagdb.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/tools/dagdb.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/tools/mod.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/resources/tools_summary.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/docs/dagdb/full-migration/code-trace.md`
commit: `238a2d82`

### QM-12

surface: CommandBase
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app && npm test`
red_failure: RED failed in `commandbase-dagdb-adapter.test.js` because `better-sqlite3` was still present in production `dependencies` and `public/index.html` did not load `dagdb-durable-state.js` before `app.js`; the package also lacked a complete real test-script gate before QM-12.
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app && node --test commandbase-dagdb-adapter.test.js && npm test`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/commandbase-dagdb-adapter.test.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/commandbase-dagdb-adapter.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/commandbase-db-factory.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/sqlite-dev-db.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/commandbase-ui-state.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/public/dagdb-durable-state.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/public/app.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/public/index.html`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/server.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/package.json`
commit: `b905a847`

### QM-13

surface: Demo
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/demo && npm test && npm run test:services && npm run test:react`
red_failure: Focused RED `npm run test:services -- dagdb-adapter-contract` failed because `demo/services/audit-api/src/index.js` still imported `pg` directly, constructed `new pg.Pool({ connectionString: process.env.DATABASE_URL })`, and the legacy SQL init fixture notice did not exist.
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/demo && npm run test:services -- dagdb-adapter-contract && npm run test:services && npm test && npm run test:react`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/packages/shared/src/dagdb-adapter.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/packages/shared/src/index.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/gateway-api/src/dagdb-adapter-contract.test.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/*/src/index.js`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/*/package.json`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/infra/docker-compose.yml`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/infra/postgres/init/README.md`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/package-lock.json`
commit: `ba1008fb`

### QM-14

surface: Site
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/site && npm run security:contact-intake && npm run security:contact-disclosure`
red_failure: Focused RED `npm run security:contact-intake` failed with `AssertionError [ERR_ASSERTION]: contact storage must not open direct Postgres or read legacy database URLs` because `site/src/lib/contact-submissions.ts` still imported `pg`, read `CONTACT_DATABASE_URL`/`DATABASE_URL`, and created `site_contact_submissions` plus `site_contact_rate_limits`.
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/site && npm run typecheck && npm run build && npm run security:contact-intake && npm run security:contact-disclosure`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/src/lib/contact-submissions.ts`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/scripts/test-contact-intake-policy.mjs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/scripts/assert-no-contact-submission-disclosure.mjs`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/package.json`; `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/package-lock.json`
commit: not-claimed

### QM-15

surface: Web durable state
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/web && npm test`
red_failure: not-claimed
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/web && npm run build && npm test`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/web`
commit: not-claimed

### QM-16

surface: CyberMedica
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica && npm run quality`
red_failure: not-claimed
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica && npm run quality`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica/src/trust-adapter.mjs`
commit: not-claimed

### QM-17

surface: Source hygiene
classification: All owned surfaces
red_command: `cargo clippy --workspace --all-targets -- -D warnings`
red_failure: not-claimed
green_command: `cargo clippy --workspace --all-targets -- -D warnings`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration`
commit: not-claimed

### QM-18

surface: Coverage
classification: All owned surfaces
red_command: `cargo tarpaulin --workspace --all-targets --fail-under 90`
red_failure: not-claimed
green_command: `cargo tarpaulin --workspace --all-targets --fail-under 90`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/target`
commit: not-claimed

### QM-19

surface: Live proof
classification: Core plus adapters
red_command: `EXO_DAGDB_TEST_DATABASE_URL=... cargo test -p exo-gateway --features production-db dagdb`
red_failure: not-claimed
green_command: `EXO_DAGDB_TEST_DATABASE_URL=... cargo test -p exo-gateway --features production-db dagdb`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/docs/dagdb/full-migration/live-proof.md`
commit: not-claimed
