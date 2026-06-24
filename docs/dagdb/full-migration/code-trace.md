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
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-gateway/src/dagdb.rs` | Core runtime adapter | QM-07 mounts all twelve documented REST routes; QM-10 wraps promoted write routes in DAG DB idempotency replay/conflict guards. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-dag-db-postgres` | EXOCHAIN core | Dedicated Postgres DAG DB schema, migrator, tenant transaction binding, and 69 traced table contracts exist after the QM-06 gateway-state migrations. Missing production state families continue to be added here first. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exochain-sdk/src/dagdb.rs` | Core runtime adapter | QM-11 exposes typed spec helpers and HTTP client methods for all twelve DAG DB REST routes, including import/export finality headers and auth-only lookup methods. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/tools/dagdb.rs` | Core runtime adapter | QM-11 exposes twelve MCP tools bound to canonical DAG DB DTO fixtures, with route-specific signature/finality carrier fields and fail-closed gateway proxy dispatch. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/tools/mod.rs` | Core runtime adapter | QM-11 registers and dispatches all twelve DAG DB MCP tools through the production `ToolRegistry`. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/crates/exo-node/src/mcp/resources/tools_summary.rs` | Core runtime adapter | QM-11 categorizes all twelve DAG DB MCP tools as `dagdb` in the tool-summary resource. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base` | Adjacent surface | QM-12 routes production CommandBase persistence through a DAG DB intake adapter, moves `better-sqlite3` to dev/test compatibility only, and sends durable dashboard UI state through the server adapter. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo` | Adjacent surface | QM-13 routes demo service persistence through `@exochain/shared`'s DAG DB adapter, removes service `pg` production dependencies, and marks legacy SQL init files as fixture-only. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/site` | Adjacent surface | QM-14 routes contact intake through the gateway DAG DB intake adapter and removes production `CONTACT_DATABASE_URL`/direct `pg` ownership. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/web` | Adjacent surface | QM-15 routes durable council, feedback, layout-template, and APE onboarding state through the DAG DB durable-state adapter; only classified ephemeral compatibility keys remain browser-local. |
| `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica` | Adjacent surface | QM-16 requires CyberMedica trust activation evidence to name the DAG DB gateway intake route and fail closed on missing, simulated, cached, or overridden DAG DB trust evidence. |

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

QM-09 expanded the live-path RLS contract metadata and seed rows to cover every
new tenant table introduced by this migration wave:

```text
dagdb_gateway_state_records
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
dagdb_zerodentity_records
```

The RED contract failed because these tables were present in the RLS migration
but absent from the live-path metadata. GREEN evidence now covers source
enumeration, missing-tenant-context denial, and cross-tenant read/write
isolation for the expanded table set.

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

## DAG DB Idempotency And Replay Boundary

QM-10 extends DAG DB idempotency guards from import/export to the newly promoted
write routes:

```text
dagdb.intake
dagdb.validate
dagdb.trust_check
dagdb.council_decision
```

Each wrapper computes the same deterministic request hash as its persistence
function, reserves `(tenant_id, namespace, route_name, idempotency_key)` in
`dagdb_idempotency_keys`, and only then runs the existing route-specific
mutation. A completed duplicate request replays the cached response body with
`idempotency_status: replayed`; a reused key with a different request hash
returns `409 idempotency_key_conflict` and emits operational receipt evidence.
If persistence fails after reservation, the wrapper removes only the matching
reserved row so the caller can retry without leaving a poisoned in-progress key.

The RED source guard failed because `persist_idempotent_intake_response` and the
other wrappers did not exist. The GREEN guard verifies that every promoted route
has an explicit route constant, calls `reserve_gateway_idempotency_key`, stores
with `store_gateway_idempotency_response`, and routes cleanup through the shared
reservation delete helper.

## DAG DB SDK and MCP Parity

QM-11 expands the typed SDK and MCP surfaces to match the twelve REST routes.
`crates/exochain-sdk/src/dagdb.rs:65` through `:163` now exposes typed
`DagDbClient` request specs for:

```text
intake
route
context_packet
validate
writeback
dagdb_import
dagdb_export
trust_check
council_decision
receipt_lookup
catalog_lookup
route_lookup
```

`crates/exochain-sdk/src/dagdb.rs:989` through `:1351` exposes matching
`DagDbHttpClient` methods. Mutating routes fail closed before HTTP when required
gateway signatures are missing. Import and export additionally require
independent finality approval signatures, DIDs, and timestamps before HTTP
dispatch. Receipt, catalog, and route lookups are auth/tenant/namespace scoped
GET requests and do not accept mutation signature carriers.

`crates/exo-node/src/mcp/tools/dagdb.rs:84` through `:95` defines the twelve MCP
tool names, `:1417` through `:1881` defines strict object schemas, and `:1924`
through `:1988` executes each tool through the shared fail-closed gateway proxy.
The schema helpers bind new write-route tools to canonical
`exo-dag-db-api/fixtures/json/all_dto_fixtures.json` request fixtures. MCP strips
signature carrier fields before DTO deserialization and forwards them only as
gateway headers.

`crates/exo-node/src/mcp/tools/mod.rs:97` through `:164` registers all twelve
DAG DB tools in the production `ToolRegistry`, and `:260` through `:276`
dispatches them. `crates/exo-node/src/mcp/resources/tools_summary.rs:89` through
`:100` categorizes all twelve tools under `dagdb`.

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

- QM-12 implementation files:
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/commandbase-db-factory.js`
    chooses the production DAG DB adapter when `NODE_ENV=production` and
    `COMMAND_BASE_ALLOW_DEV_SQLITE` is not set to `1`.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/commandbase-dagdb-adapter.js`
    requires `COMMAND_BASE_DAGDB_GATEWAY_URL`,
    `COMMAND_BASE_DAGDB_AUTH_TOKEN`, `COMMAND_BASE_DAGDB_TENANT_ID`,
    `COMMAND_BASE_DAGDB_NAMESPACE`, owner/controller/submitted-by DIDs, and
    `COMMAND_BASE_DAGDB_WRITE_SIGNATURE`. It records production operations to
    `POST /api/v1/dag-db/intake` with tenant, namespace, idempotency,
    authority-scope, and write-signature headers. SQL-shaped `.run`, `.get`,
    and `.all` calls refuse to synthesize row or row-id results; the gateway
    must return an explicit `commandbase_result`, or the adapter fails closed.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/sqlite-dev-db.js`
    is the only direct `better-sqlite3` opener and is reached only through the
    factory's development/test compatibility branch.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/lib/commandbase-ui-state.js`
    mounts `/api/dagdb/commandbase/ui-state` and records durable UI-state
    mutations through the active adapter.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/public/dagdb-durable-state.js`
    provides the browser facade that replaces direct durable dashboard
    `localStorage` calls.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/command-base/app/services/wasm-loader.js`
    fails closed if the EXOCHAIN WASM package is unavailable instead of
    pretending trust evidence is present.
- QM-12 RED evidence: `cd command-base/app && npm test` initially failed the new
  `commandbase-dagdb-adapter.test.js` guard because `better-sqlite3` remained a
  production dependency and the durable-state script was not mounted before
  `app.js`.
- QM-12 GREEN evidence: `node --test commandbase-dagdb-adapter.test.js` passed
  3 tests, including a child-process gateway proof that adapter reads use real
  `commandbase_result` bodies and fail closed when absent. `npm test` passed 52
  tests for the CommandBase package.

Baseline before QM-12:

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

- QM-13 implementation files:
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/packages/shared/src/dagdb-adapter.js`
    defines the shared demo DAG DB store. Production requires
    `EXO_DEMO_DAGDB_GATEWAY_URL`, gateway bearer token, tenant/namespace,
    owner/controller/submitted-by DIDs, and write signature. It posts every
    query-shaped operation to `POST /api/v1/dag-db/intake` with tenant,
    namespace, idempotency, authority-scope, and write-signature headers, then
    refuses to synthesize query results unless the gateway returns explicit
    `demo_result.rows`.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/packages/shared/src/index.js`
    exports `createDemoServiceStore` and keeps `getPool()` as a DAG DB adapter
    alias for older shared callers.
  - Every `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/*/src/index.js`
    entrypoint now uses `createDemoServiceStore(<service-name>)` instead of
    constructing `pg.Pool` from `DATABASE_URL`.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/infra/docker-compose.yml`
    injects `EXO_DEMO_DAGDB_*` settings into services and places the legacy
    Postgres container behind the `legacy-postgres-fixture` profile.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/infra/postgres/init/README.md`
    states the SQL init directory is fixture-only and must not be mounted as a
    production writer.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/gateway-api/src/dagdb-adapter-contract.test.js`
    is the QM-13 source guard covering service entrypoints, shared adapter
    exports, and the fixture-only SQL notice.
- QM-13 RED evidence: `npm run test:services -- dagdb-adapter-contract` failed
  because `demo/services/audit-api/src/index.js` still imported `pg` directly
  and constructed `new pg.Pool({ connectionString: process.env.DATABASE_URL })`;
  the fixture-only README was also missing.
- QM-13 GREEN evidence: `npm run test:services -- dagdb-adapter-contract`
  passed 2 tests; `npm run test:services` passed 173 tests; `npm test` passed
  183 tests; `npm run test:react` passed 10 tests. The React suite emitted its
  existing `act(...)` and `--localstorage-file` warnings.

Baseline before QM-13:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/services/*/src/index.js`
  services open `new pg.Pool({ connectionString: process.env.DATABASE_URL })`.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/demo/infra/postgres/init`
  initializes direct demo schemas for users, agents, decisions, delegations,
  audit entries, constitutions, identity scores, LifeSafe, VitalLock,
  governance health, and CrossChecked tables.

Site:

- QM-14 implementation files:
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/src/lib/contact-submissions.ts`
    now requires `SITE_DAGDB_GATEWAY_URL`, `SITE_DAGDB_AUTH_TOKEN`,
    tenant/namespace, owner/controller/submitted-by DIDs, and
    `SITE_DAGDB_WRITE_SIGNATURE`. Every submission, rate-limit, notification,
    and recent-list operation posts to `POST /api/v1/dag-db/intake` with tenant,
    namespace, idempotency, authority-scope, and write-signature headers.
  - The contact adapter refuses to synthesize durable state. Submission creation
    requires `site_contact_result.submission`; rate limiting requires
    `site_contact_result.request_count`; notification updates require
    `site_contact_result.notification_updated`; recent listing requires
    `site_contact_result.submissions`.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/package.json`
    no longer declares `pg` or `@types/pg`.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/scripts/test-contact-intake-policy.mjs`
    guards against direct Postgres imports, legacy public contact tables,
    legacy contact DB environment variables, and missing DAG DB intake evidence.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/scripts/assert-no-contact-submission-disclosure.mjs`
    keeps internal support disclosure decoupled from contact submissions and
    guards that the contact backend remains DAG DB-backed.
- QM-14 RED evidence: `npm run security:contact-intake` failed with
  `AssertionError [ERR_ASSERTION]: contact storage must not open direct
  Postgres or read legacy database URLs`.
- QM-14 GREEN evidence: `npm run security:contact-intake`,
  `npm run security:contact-disclosure`, `npm run typecheck`, and
  `npm run build` passed. The lockfile refresh reported the existing site npm
  audit findings: 7 vulnerabilities (3 moderate, 4 high).

Baseline before QM-14:

- `/Users/bobstewart/dev/exochain-dagdb-full-migration/site/src/lib/contact-submissions.ts:60`
  names `CONTACT_DATABASE_URL`.
- `:124` through `:163` creates `site_contact_submissions` and
  `site_contact_rate_limits`.
- `:182` through `:228` inserts contact submissions.
- `:231` through `:268` mutates rate-limit rows.

Web:

- QM-15 implementation files:
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/dagdbDurableState.ts`
    defines the shared browser durable-state adapter for
    `council-tickets`, `council-conversations`, `feedback-issues`,
    `layout-templates`, and `ape-onboarding`. It records every family through
    `POST /api/v1/dag-db/intake` with tenant, namespace, idempotency,
    authority-scope, and token-derived authorization headers. The adapter
    refuses to accept write/read/delete confirmations unless the response body
    contains `web_durable_state_result`.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/council.ts`
    now persists tickets and conversations via the DAG DB durable-state adapter.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/CouncilContext.tsx`
    hydrates tickets and conversations from the DAG DB durable-state adapter
    after mount.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/stores/feedbackStore.ts`
    persists mandated-reporter feedback through the `feedback-issues` durable
    family.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/stores/layoutTemplateStore.ts`
    persists user layout templates and active template selection through the
    `layout-templates` durable family.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/apeOnboardingState.ts`,
    `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/pages/APE/OnboardPage.tsx`,
    `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/pages/APE/APEDashboardPage.tsx`,
    and `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/auth.tsx`
    move onboarding data to the `ape-onboarding` durable family. Auth token,
    dev-bypass, and theme compatibility keys remain browser-local by explicit
    classification.
  - `/Users/bobstewart/dev/exochain-dagdb-full-migration/web/src/lib/dagdbDurableState.test.ts`
    guards that durable product-state keys no longer appear in production web
    source and that every durable family is represented in the DAG DB adapter.
- QM-15 RED evidence: `npm test -- dagdbDurableState` failed because the
  durable source still contained `df_council_tickets` and
  `df_council_conversations`, and `web/src/lib/dagdbDurableState.ts` did not
  exist.
- QM-15 GREEN evidence: `npm test -- dagdbDurableState` passed 2 tests;
  `npm test` passed 377 tests across 10 files; `npm run build` completed the
  Vite production build. The build retained the existing chunk-size warning.

Baseline before QM-15:

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
  now evaluates a DAG DB gateway call-path evidence object in addition to root,
  gateway, receipt, privacy, and Decision Forum evidence. Verified activation
  requires `source: exochain_dagdb_gateway`, `routePath: /api/v1/dag-db/intake`,
  `method: POST`, tenant and namespace binding, the
  `x-exo-authority-scope` authority header, unavailable-gateway fail-closed
  evidence, no-simulation policy evidence, and digest-shaped route, request,
  and receipt hashes.
- The same adapter blocks `dagdb_gateway_call_path_absent`,
  `dagdb_gateway_local_simulation_forbidden`,
  `dagdb_gateway_cached_outcome_forbidden`, and
  `dagdb_gateway_override_forbidden`, so CyberMedica cannot lift a production
  trust claim by naming EXOCHAIN generally or by replaying local/cached trust.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica/src/runtime-configuration-source.mjs`
  evaluates runtime configuration sources and adapters.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica/scripts/source-adapter-contract-guard.mjs`
  adds MAC-006, binding the integration map, adapter source, and adapter tests
  to the DAG DB gateway evidence route.
- `/Users/bobstewart/dev/exochain-dagdb-full-migration/cybermedica/docs/context/EXOCHAIN_TO_CYBERMEDICA_INTEGRATION_MAP.md`
  now includes the DAG DB gateway evidence path and the minimum adapter
  contract requirement that the path fail closed when missing, simulated,
  cached, overridden, unavailable, or not bound to tenant and namespace.
- QM-16 RED evidence: `node --test tests/trust-adapter.test.mjs` failed because
  the adapter output lacked `dagDbGatewayCallPathSource`, allowed missing DAG DB
  evidence, and allowed simulated DAG DB trust; `npm run guard:adapter-contracts`
  failed MAC-006 coverage for the integration map, adapter source, and tests;
  the package-level `npm run quality` then failed older fixtures that treated
  root/gateway/receipt/privacy/Decision Forum evidence as sufficient without
  DAG DB call-path evidence.
- QM-16 GREEN evidence: `node --test tests/trust-adapter.test.mjs` passed 9
  tests; `npm run guard:adapter-contracts` passed with `findingsCount: 0`;
  `npm run quality` passed CyberMedica lint/typecheck, dependency audit, source
  hazard scan, source secret scan, guard suite, build artifact generation,
  1005 tests, and coverage.

CyberMedica remains adjacent. It may name a verified EXOCHAIN production trust
claim only when runtime evidence includes the tested DAG DB gateway call path;
otherwise it stays inactive or denied and must not claim constitutional
enforcement by proximity.

## Migration Rule

Fresh-start all mutable durable state. Preserve only verified
`exo_root::RootTrustBundle` / `EXO_AVC_ROOT_TRUST_BUNDLE` bootstrap evidence.
Legacy SQLite, public-schema, direct demo Postgres, site contact Postgres, and
browser durable state can be retained only as rollback evidence or test fixtures,
not as production writers.
