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
| QM-07 | Full REST surface | Core runtime adapter | Router test fails because only five routes are mounted. | All documented routes mount, auth gate, persist, and fail closed without DB. |
| QM-08 | Finality | EXOCHAIN core | Self-approved route/import/export/council tests fail. | Accepted/approved state requires independent finality authority. |
| QM-09 | RLS | EXOCHAIN core | Tenant mismatch live test fails for each unlisted tenant table. | Every tenant table is RLS-protected and live mismatch test passes. |
| QM-10 | Idempotency/replay | EXOCHAIN core | Duplicate key and mismatched request-hash tests fail for new routes. | Replay is stable; mismatched replay rejects mutation with receipt evidence. |
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
commit: `76a2e327`

### QM-01

surface: Matrix enforcement
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway quality_matrix_is_complete`
red_failure: `quality matrix must exist at /Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/../../docs/dagdb/full-migration/quality-matrix.md: No such file or directory (os error 2)`
green_command: `cargo test -p exo-gateway quality_matrix_is_complete`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: `76a2e327`

### QM-02

surface: DAG DB schema
classification: EXOCHAIN core
red_command: `EXO_DAGDB_TEST_DATABASE_URL=... cargo test -p exo-dag-db-postgres --features postgres migration_contract`
red_failure: not-claimed
green_command: `EXO_DAGDB_TEST_DATABASE_URL=... cargo test -p exo-dag-db-postgres --features postgres migration_contract`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations`
commit: not-claimed

### QM-03

surface: Root bundle
classification: EXOCHAIN core
red_command: `cargo test -p exo-root && cargo test -p exo-node avc::tests`
red_failure: not-claimed
green_command: `cargo test -p exo-root && cargo test -p exo-node avc::tests`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/avc.rs`
commit: not-claimed

### QM-04

surface: Node DAG store
classification: EXOCHAIN core
red_command: `cargo test -p exo-node dagdb_node_store_source_guard`
red_failure: not-claimed
green_command: `cargo test -p exo-node --features dagdb-gateway-proxy dagdb`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs`
commit: not-claimed

### QM-05

surface: 0dentity
classification: EXOCHAIN core
red_command: `cargo test -p exo-node zerodentity_restart_persists_dagdb_state`
red_failure: not-claimed
green_command: `cargo test -p exo-node zerodentity`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/zerodentity/store.rs`
commit: not-claimed

### QM-06

surface: Gateway state
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway gateway_legacy_public_schema_writes_are_blocked`
red_failure: not-claimed
green_command: `cargo test -p exo-gateway --features production-db gateway_state`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/db.rs`
commit: not-claimed

### QM-07

surface: Full REST surface
classification: Core runtime adapter
red_command: `cargo test -p exo-gateway dagdb_router_mounts_full_rest_surface`
red_failure: not-claimed
green_command: `cargo test -p exo-gateway --features production-db dagdb`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: not-claimed

### QM-08

surface: Finality
classification: EXOCHAIN core
red_command: `cargo test -p exo-gateway dagdb_finality_requires_independent_authority`
red_failure: not-claimed
green_command: `cargo test -p exo-gateway dagdb_finality`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: not-claimed

### QM-09

surface: RLS
classification: EXOCHAIN core
red_command: `EXO_DAGDB_TEST_DATABASE_URL=... cargo test -p exo-dag-db-postgres --features postgres dagdb_tenant_rls_live_path_contract`
red_failure: not-claimed
green_command: `EXO_DAGDB_TEST_DATABASE_URL=... cargo test -p exo-dag-db-postgres --features postgres dagdb_tenant_rls_live_path_contract`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`
commit: not-claimed

### QM-10

surface: Idempotency/replay
classification: EXOCHAIN core
red_command: `cargo test -p exo-gateway dagdb_idempotency_replay_contract`
red_failure: not-claimed
green_command: `cargo test -p exo-gateway dagdb_idempotency_replay_contract`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
commit: not-claimed

### QM-11

surface: SDK/MCP parity
classification: Core runtime adapter
red_command: `cargo test -p exochain-sdk --features http-client dagdb && cargo test -p exo-node --features dagdb-gateway-proxy dagdb`
red_failure: not-claimed
green_command: `cargo test -p exochain-sdk --features http-client dagdb && cargo test -p exo-node --features dagdb-gateway-proxy dagdb`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exochain-sdk/src/dagdb.rs`
commit: not-claimed

### QM-12

surface: CommandBase
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app && npm test`
red_failure: not-claimed
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app && npm test`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app`
commit: not-claimed

### QM-13

surface: Demo
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/demo && npm test && npm run test:services && npm run test:react`
red_failure: not-claimed
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/demo && npm test && npm run test:services && npm run test:react`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo`
commit: not-claimed

### QM-14

surface: Site
classification: Adjacent surface
red_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/site && npm run security:contact-intake && npm run security:contact-disclosure`
red_failure: not-claimed
green_command: `cd /Users/bobstewart/dev/exochain-dagdb-full-migration/site && npm run typecheck && npm run build && npm run security:contact-intake && npm run security:contact-disclosure`
artifact: `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/src/lib/contact-submissions.ts`
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
