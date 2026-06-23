# Full DAG DB Migration Code Trace

Schema: `dagdb_full_migration_code_trace_v1`

Captured from clean branch `bob-stewart/dagdb-full-migration-20260623` at
`97e234dc46e483b254d0f82aed5fb2d5669b1ba5`, equal to `origin/main` on
2026-06-23. Max's merged PR #695 is present as merge commit `5ea25f6c`, and the
current base also includes later DAG DB finality hardening through merge #703.

## Baseline

The source checkout at `/Users/bobstewart/dev/exochain` is not a safe migration
base. It is on branch
`bob-stewart/cybermedica-adjacent-surface-contracts-20260526`, whose upstream is
gone, and it contains modified and untracked files across CyberMedica, site, SDK
dist-test output, and adjacent documents. The migration branch was therefore
created in the isolated worktree
`/Users/bobstewart/dev/exochain-dagdb-full-migration` from `origin/main`.

Command evidence:

```text
git fetch origin main --prune
git worktree add -b bob-stewart/dagdb-full-migration-20260623 ../exochain-dagdb-full-migration origin/main
git status --short --branch
## bob-stewart/dagdb-full-migration-20260623...origin/main
git rev-parse HEAD
97e234dc46e483b254d0f82aed5fb2d5669b1ba5
git rev-parse origin/main
97e234dc46e483b254d0f82aed5fb2d5669b1ba5
```

## Classification

| Path | Classification | Current durable-state finding |
|---|---|---|
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs` | EXOCHAIN core | QM-04 moved production start and status commands to `DagDbNodeStore::open` with required `DATABASE_URL`, `EXO_DAGDB_TENANT_ID`, and `EXO_DAGDB_NAMESPACE`. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/store.rs` | EXOCHAIN core | QM-04 introduced a DAG DB-backed node store while retaining legacy SQLite construction only for direct test/dev compatibility. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/zerodentity/store.rs` | EXOCHAIN core | QM-05 moved production 0dentity startup to DAG DB-backed persistence for claims, scores, OTP state, sessions, attestations, emitted DAG nodes, and trust receipts. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/db.rs` | Core runtime adapter | QM-06 splits gateway migrations from runtime serving: public migrations still run as rollback/history, but the returned production pool uses DAGDB-first `search_path` so gateway table contracts resolve in the `dagdb` schema. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs` | Core runtime adapter | DAG DB REST router currently mounts five live routes. The remaining documented routes are present as test-only handlers or DTO fixtures, not live production routes. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres` | EXOCHAIN core | Dedicated Postgres DAG DB schema, migrator, tenant transaction binding, and 69 traced table contracts exist after the QM-06 gateway-state migrations. Missing production state families continue to be added here first. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exochain-sdk/src/dagdb.rs` | Core runtime adapter | SDK exposes the same five-route subset. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/tools/dagdb.rs` | Core runtime adapter | MCP exposes four agent-facing DAG DB tools, not the full REST surface. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base` | Adjacent surface | Production CommandBase uses `better-sqlite3`, `the_team.db`, `task_forces.db`, many SQLite DDL blocks, and browser durable state. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo` | Adjacent surface | Demo services create direct `pg.Pool` instances against `DATABASE_URL` and initialize demo-owned Postgres schemas. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/site` | Adjacent surface | Contact intake owns direct `CONTACT_DATABASE_URL` tables and rate-limit state. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/web` | Adjacent surface | Council, feedback, layout templates, onboarding, and auth compatibility paths use `localStorage`. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica` | Adjacent surface | Trust adapter/runtime configuration code records evidence boundaries but is not a live DB owner on `origin/main`. |

## Core Node Store

QM-04 implementation replaced the production node DAG store startup path:

```text
crates/exo-node/src/main.rs:513
let gateway_pool = gateway_pool_from_env().await?;
let (dagdb_tenant_id, dagdb_namespace) = dagdb_node_scope_from_env()?;
let dag_store =
    store::DagDbNodeStore::open(gateway_pool.clone(), dagdb_tenant_id, dagdb_namespace).await?;
```

`exochain status` now reads the same DAG DB-backed height:

```text
crates/exo-node/src/main.rs:1260
let gateway_pool = gateway_pool_from_env().await?;
let (dagdb_tenant_id, dagdb_namespace) = dagdb_node_scope_from_env()?;
let dag_store =
    store::DagDbNodeStore::open(gateway_pool, dagdb_tenant_id, dagdb_namespace).await?;
```

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/store.rs:132`
through `:175` is now a compatibility handle with two backends:

```text
SqliteDagStore { backend: NodeStoreBackend }
NodeStoreBackend::DagDb(PostgresDagNodeStore)
DagDbNodeStore::open(pool, tenant_id, namespace)
```

The DAG DB backend persists DAG nodes, parents, committed heights, consensus
round/votes, commit certificates, validators, trust receipts, economy objects,
economy anchors, and economy metadata in tenant-scoped Postgres tables. Each
operation binds `exo.tenant_id` in a transaction before reading or writing.

Legacy SQLite remains directly constructible through
`SqliteDagStore::open(data_dir)` at
`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/store.rs:1087`
through `:1098`, but production `start_node` and `status` no longer call it.

### Baseline Finding Before QM-04

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs:495`
through `:501` opens the local DAG store and then opens the 0dentity store:

```text
495 // Open local DAG store.
496 let dag_store = store::SqliteDagStore::open(data_dir)?;
497 let height = dag_store.committed_height_value()?;
...
500 // Open 0dentity store (shares the same dag.db, applies zerodentity migration).
501 let mut zerodentity_store = zerodentity::store::ZerodentityStore::open(data_dir)?;
```

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/main.rs:1264`
through `:1270` also opens SQLite for `exochain status`:

```text
1264 Command::Status { data_dir } => {
1265     let data_dir = config::resolve_data_dir(data_dir)?;
1266     let node_identity = identity::load_or_create(&data_dir)?;
1267     let dag_store = store::SqliteDagStore::open(&data_dir)?;
```

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/store.rs:138`
through `:236` defines `SqliteDagStore::open` and creates the SQLite tables
`dag_nodes`, `dag_parents`, `committed`, `consensus_meta`, `consensus_votes`,
`commit_certificates`, `validators`, `trust_receipts`, `economy_objects`,
`economy_anchors`, and `economy_meta`.

## 0dentity DAG DB Store

QM-05 implementation moved production 0dentity startup to DAG DB:

```text
crates/exo-node/src/main.rs:524
let mut zerodentity_store = zerodentity::store::ZerodentityStore::open_dagdb(
    gateway_pool.clone(),
    dagdb_tenant_id.clone(),
    dagdb_namespace.clone(),
)
.await?;
```

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/zerodentity/store.rs`
now exposes `ZerodentityStore::open_dagdb(pool, tenant_id, namespace)`, verifies
`dagdb_zerodentity_records`, reloads rows in deterministic order, and keeps the
in-memory view synchronized with DAG DB writes.

The new tenant-owned record table is:

```text
crates/exo-dag-db-postgres/migrations/20260623000003_create_zerodentity_record_schema.sql
dagdb_zerodentity_records
```

Its schema-enforced `state_family` values are:

```text
claim
score
previous_score
score_history
device_fingerprint
behavioral_sample
otp_challenge
otp_lockout
attestation
identity_session
session_nonce
dag_node
trust_receipt
```

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`
now includes `dagdb_zerodentity_records`, so production rows are covered by the
same forced tenant policy as the node-store and DAG DB tables.

`ZerodentityStore::open(data_dir)` remains only as a test/dev compatibility
entry point. Production `start_node` no longer calls it.

### Baseline Finding Before QM-05

Before QM-05, `crates/exo-node/src/zerodentity/store.rs` declared persistence
not ready and stored claims, sessions, OTP challenges, attestations, scores, DAG
nodes, and trust receipts only in `BTreeMap`, `BTreeSet`, and `Vec` fields. Its
`open` method ignored `data_dir` and returned `Self::new()`.

## Gateway DAG DB State

QM-06 keeps the gateway's existing table-shaped helper API but changes the
production resolution boundary. `crates/exo-gateway/src/db.rs` now uses a
migration pool with `public,dagdb` so the existing gateway migration ledger stays
isolated, then closes it and returns a runtime pool with `dagdb,public`.

The DAG DB schema now owns two gateway state migrations:

```text
crates/exo-dag-db-postgres/migrations/20260623000004_create_gateway_state_records_schema.sql
crates/exo-dag-db-postgres/migrations/20260623000005_create_gateway_legacy_table_contracts.sql
```

`dagdb_gateway_state_records` is the closed family ledger for gateway state
families:

```text
did_document
session
user
agent
decision
delegation
audit_entry
constitution
identity_score
enrollment
livesafe_identity
scan_receipt
consent_anchor
trustee_shard
agent_role
consent_record
authority_chain
layout_template
feedback_issue
conflict_declaration
avc_registry_state
hlc_counter
```

The DAG DB legacy table-contract migration also creates DAGDB-schema copies of
the gateway's production table contracts (`users`, `agents`, `decisions`,
`audit_entries`, `sessions`, `did_documents`, `feedback_issues`, and the rest)
with the final deterministic shapes, including `odentity_composite_basis_points`
instead of the old floating LiveSafe score column.

The public-schema gateway migrations still run for rollback/history and for
older deployments, but production traffic receives the DAGDB-first runtime pool.
The source guard `production_gateway_state_has_no_explicit_public_schema_writes`
prevents future route code from bypassing that boundary with explicit
`public.<table>` writes.

## DAG DB Foundation

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/src/postgres/mod.rs:101`
through `:114` defines the dedicated `dagdb` schema and explains the separate
SQLx migration ledger. `:173` through `:203` exposes the migrator and default
pool initialization. `:205` through `:226` binds `exo.tenant_id` for RLS-protected
tenant transactions. `:249` through `:290` provisions the schema through the
canonical ledgered migration runner.

Current inventory:

- 19 SQL migration files exist under
  `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations`.
- Those migrations contain 69 `CREATE TABLE IF NOT EXISTS` table contracts.
- Tenant RLS is centralized in
  `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`.

The current DAG DB table contracts are:

```text
dagdb_receipts
dagdb_root_bundle_receipts
dagdb_subject_receipt_heads
dagdb_memory_objects
dagdb_memory_edges
dagdb_node_commit_certificates
dagdb_node_committed
dagdb_node_consensus_meta
dagdb_node_consensus_votes
dagdb_node_dag_nodes
dagdb_node_dag_parents
dagdb_node_economy_anchors
dagdb_node_economy_meta
dagdb_node_economy_objects
dagdb_node_trust_receipts
dagdb_node_validators
dagdb_catalog_entries
dagdb_route_receipts
dagdb_context_packets
dagdb_validation_reports
dagdb_agent_safety_scores
dagdb_inbound_agent_credentials
dagdb_council_decisions
dagdb_idempotency_keys
dagdb_dag_outbox
dagdb_benchmark_runs
dagdb_graph_nodes
dagdb_graph_edges
dagdb_graph_similarity_results
dagdb_graph_canonicalization_decisions
dagdb_graph_views
dagdb_graph_placement_traces
dagdb_graph_route_invalidations
dagdb_exports
dagdb_export_challenges
dagdb_graph_edge_tombstones
dagdb_graph_layers
dagdb_graph_layer_memberships
dagdb_graph_layer_edges
dagdb_default_routes
dagdb_context_packet_records
dagdb_lifecycle_rollbacks
dagdb_lifecycle_actions
dagdb_route_invalidation_events
dagdb_continuation_records
dagdb_zerodentity_records
dagdb_gateway_state_records
```

The same DAG DB schema also owns these gateway runtime compatibility table
contracts so the gateway's existing bare SQL resolves inside `dagdb`:

```text
users
agents
decisions
delegations
audit_entries
constitutions
identity_scores
enrollment_log
hlc_state
livesafe_identities
scan_receipts
consent_anchors
trustee_shard_status
sessions
agent_roles
consent_records
authority_chains
layout_templates
feedback_issues
conflict_declarations
did_documents
avc_registry_state
```

## DAG DB REST, SDK, MCP Surface

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
now mounts the twelve documented DAG DB routes:

```text
/api/v1/dag-db/intake
/api/v1/dag-db/route
/api/v1/dag-db/context-packet
/api/v1/dag-db/validate
/api/v1/dag-db/writeback
/api/v1/dag-db/import
/api/v1/dag-db/export
/api/v1/dag-db/trust-check
/api/v1/dag-db/council/decision
/api/v1/dag-db/receipts/:receipt_hash
/api/v1/dag-db/catalog/:catalog_id
/api/v1/dag-db/routes/:route_id
```

The RED test
`dagdb_router_mounts_full_rest_surface_fail_closed_without_db` observed a 404
for POST `/api/v1/dag-db/intake` before the route surface was expanded. The
GREEN route test now proves all twelve routes are mounted, auth-gated, and
fail closed with 503 when no governed DAG DB pool is configured.

The promoted route handlers no longer use generic scaffold authorization
responses. They call route-specific persistence and lookup paths:

- intake writes `dagdb_receipts`, `dagdb_subject_receipt_heads`,
  `dagdb_memory_objects`, and parent edges in `dagdb_memory_edges`;
- validate writes `dagdb_receipts`, `dagdb_subject_receipt_heads`, and
  `dagdb_validation_reports`;
- trust-check writes `dagdb_receipts`, `dagdb_subject_receipt_heads`,
  `dagdb_inbound_agent_credentials`, and `dagdb_agent_safety_scores`;
- council decision writes `dagdb_receipts`, `dagdb_subject_receipt_heads`, and
  `dagdb_council_decisions`;
- receipt, catalog, and route lookups read `dagdb_receipts`,
  `dagdb_catalog_entries`, and `dagdb_route_receipts`.

Live Postgres write/read proof is still gated by QM-19 and requires
`EXO_DAGDB_TEST_DATABASE_URL`.

## DAG DB Finality Boundary

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs`
now keeps write authorization separate from independent finality approval for
production DAG DB import and export routes:

- import requests must carry `x-exo-import-approval-signature`,
  `x-exo-import-approval-did`, and `x-exo-import-approval-timestamp`;
- export requests must carry `x-exo-export-approval-signature`,
  `x-exo-export-approval-did`, and `x-exo-export-approval-timestamp`;
- the approval DID is loaded into the gatekeeper service and must be different
  from the requesting DID;
- finality signatures bind a deterministic operation-finality payload derived
  from tenant, namespace, requester, idempotency key, DB set version, request
  hash/source hash, authorization payload hash, finality authority DID, and
  approval timestamp;
- missing finality headers or self-approval fail closed before import/export
  persistence;
- approved council decision persistence rejects direct self-approval when
  `subject_id` is the same DID as `approver_did`.

Route, context-packet, and writeback finality already used independent
approval-authority checks. QM-08 extends that same boundary to import/export
and council-decision persistence. Live signed Postgres proof remains covered by
QM-19.

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exochain-sdk/src/dagdb.rs:64`
through `:99` exposes SDK helpers for the same five routes.

`/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/tools/dagdb.rs:1`
through `:39` documents four MCP tools:
`dagdb_get_context_packet`, `dagdb_submit_writeback`, `dagdb_import`, and
`dagdb_export`.

## Root Bundle Preservation Boundary

The only mutable-state exception for fresh start is the root trust bundle:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-root/src/bundle.rs:35`
  defines `RootTrustBundle`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/avc.rs:80`
  defines `EXO_AVC_ROOT_TRUST_BUNDLE`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/avc.rs:781`
  through `:893` loads, verifies, pins, and registers the root trust issuer.

DAG DB may persist verification receipts for the root bundle, but those rows are
global bootstrap evidence and must be immutable. Tenant-owned rows must remain
tenant-scoped and RLS-protected.

## Adjacent Surfaces

CommandBase:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/server.js:20`
  and `:50` open `better-sqlite3` against `the_team.db`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/db.js:19`
  and `:22` do the same for the shared app DB.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/task-force-db.js:29`
  and `:33` open a separate `task_forces.db`.
- `rg` finds 156 `CREATE TABLE IF NOT EXISTS` statements under
  `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/public/app.js`
  persists dashboard widgets, locks, presets, grid layout, command history,
  notification preferences, mode selection, and collapse state in `localStorage`.

Demo:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/*/src/index.js`
  services open `new pg.Pool({ connectionString: process.env.DATABASE_URL })`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/infra/postgres/init`
  initializes direct demo schemas for users, agents, decisions, delegations,
  audit entries, constitutions, identity scores, LifeSafe, VitalLock,
  governance health, and CrossChecked tables.

Site:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/src/lib/contact-submissions.ts:60`
  names `CONTACT_DATABASE_URL`.
- `:124` through `:163` creates `site_contact_submissions` and
  `site_contact_rate_limits`.
- `:182` through `:228` inserts contact submissions.
- `:231` through `:268` mutates rate-limit rows.

Web:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/council.ts:279`
  through `:295` writes council tickets and conversations to `localStorage`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/stores/feedbackStore.ts:64`
  through `:79` stores feedback issues in `localStorage`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/stores/layoutTemplateStore.ts:34`
  through `:58` stores layout templates and active template state in
  `localStorage`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/pages/APE/OnboardPage.tsx:203`
  through `:212` stores onboarding state in `localStorage`.
- Auth token compatibility keys and theme preferences also use `localStorage`;
  durable product state must move first, while ephemeral display preferences can
  remain browser-local if explicitly classified.

CyberMedica:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica/src/trust-adapter.mjs`
  evaluates trust evidence source boundaries and blocks local simulation, cached
  outcomes, payload disclosure, and unverified activation.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica/src/runtime-configuration-source.mjs`
  evaluates runtime configuration sources and adapters.

CyberMedica remains adjacent until a tested DAG DB or gateway call path proves
the boundary. It must not claim EXOCHAIN constitutional enforcement by proximity.

## Migration Rule

Fresh-start all mutable durable state. Preserve only verified
`exo_root::RootTrustBundle` / `EXO_AVC_ROOT_TRUST_BUNDLE` bootstrap evidence.
Legacy SQLite, public-schema, direct demo Postgres, site contact Postgres, and
browser durable state can be retained only as rollback evidence or test fixtures,
not as production writers.
