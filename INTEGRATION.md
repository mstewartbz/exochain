<!--
Copyright 2026 Exochain Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at:

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

SPDX-License-Identifier: Apache-2.0
-->

# EXOCHAIN WASM Integration Contract

## Overview

The EXOCHAIN WASM bridge compiles the Rust constitutional trust fabric into a
WebAssembly package for JavaScript consumers. The current public bridge is
source-counted by CI Gate 22 at **157 Rust `#[wasm_bindgen]` exports** and
smoke-tested by the Node bridge verification harness before the aggregator gate
can pass.

The bridge is a core runtime adapter. Consumers may present EXOCHAIN trust
claims only when they call the relevant WASM or core API path and tests prove
fail-closed behavior when the adapter rejects, times out, or is unavailable.

## Source And Artifacts

- Rust source: `crates/exochain-wasm/src/`
- Generated Node package: `packages/exochain-wasm/wasm/`
- Bridge verification harness: `packages/exochain-wasm/test/bridge_verification.mjs`
- CI gates: `.github/workflows/ci.yml` Gates 20, 21, and 22

## Verification

```bash
cargo test -p exochain-wasm
wasm-pack build crates/exochain-wasm --target nodejs --out-dir ../../packages/exochain-wasm/wasm
node packages/exochain-wasm/test/bridge_verification.mjs
```

## Adapter Boundary

WASM consumers must not mint, cache, or simulate consent, authority,
provenance, governance outcomes, settlement authority, or constitutional
invariant results outside the Rust adapter. Adjacent surfaces such as
CommandBase and ExoForge remain adjacent unless the runtime path invokes the
tested adapter and the surface has its own fail-closed tests.

## Governance Monitoring Attestation

Continuous governance monitoring uses the Rust governance-monitor verifier
through the WASM bridge:

- `wasm_governance_findings_digest(findings_json)` computes the canonical
  findings digest.
- `wasm_verify_governance_attestation(signer_did, findings_json,
  signature_json, signer_public_key_hex)` verifies that the signed envelope
  matches the submitted findings before ingestion.

The audit API rejects missing, mismatched, or invalid attestations before any
database write. This completes the T-14 adapter path and aligns the threat
matrix with the implementation.

## DAG DB Adapter Contract (split `exo-dag-db-*` crates)

**Status: in progress** — tracked as GAP-012 in `GAP-REGISTRY.md`.
This section is the integration contract for the split `exo-dag-db-*`
graph-governed agent-memory crates; it is not yet a closure claim.

### Adapter boundary

`exo-dag-db-*` is an **opt-in adapter surface**: the default node does not serve it. The
node binary (`exo-node`) depends on `exo-gateway` with **default features**, so
it is built *without* `production-db`. The `/api/v1/dag-db/*` routes are merged
into the router unconditionally, but the gated DB-persistence path only compiles
under the `exo-gateway/production-db` feature; in a default node build the write
handlers have no DB-persistence branch and **fail closed (503)**. A functional,
governed dag-db surface therefore exists only when `exo-gateway` is built with
`production-db` **and** a Postgres pool is configured — that explicit build is
the opt-in boundary. The served writeback persistence path is routed through the
`DagDbGatekeeperService` (`crates/exo-gatekeeper/src/dagdb_gate.rs`) consent →
Ed25519 → invariant chain. Import/export routes fail closed (`403`,
`consent_denied`) until distinct import/export consent is configured, so
writeback-only consent cannot authorize them.
Consumers must not write the `dagdb_*` tables directly; the raw
`exo_dag_db_postgres::postgres::*` functions are not a public,
governance-bearing surface.

The four PRD-D5 gate methods (`persist_lifecycle_action`, `persist_default_route`,
`persist_continuation_record`, `persist_context_packet_record`) are **GATED but
DORMANT**: they enforce the full chain at the method boundary (proven by the
`dagdb_gate` route-contract tests) but no served `/api/v1/dag-db/*` endpoint
invokes them yet. Wiring them to REST endpoints is deferred (no requirement
drives them). See their `DORMANT` doc-comments and the
`gatekeeper-lifecycle-surfaces-gated` security-regression check.

### Fail-closed guarantees

- Writes are authorized only against real `exo-consent` / `exo-identity` state; an
  unconfigured resolver fails closed (no dev fabricated identity, no deterministic
  signing key in the shipping path). (T1/T2 — landed.)
- The three served mutation surfaces (writeback / import / export) fail closed
  (503, `database_unavailable`) when no database pool is configured. Writeback
  was previously a synthetic 201 scaffold; it now matches import/export. (T6 —
  landed.)
- **Constitutional invariant enforcement on the served writeback path enforces
  the *constructible* invariant subset**, not the full
  `InvariantEngine::all()`. The enforced set (`dagdb_invariant_set` in
  `dagdb_gate.rs`) is: `ConsentRequired`, `SeparationOfPowers`, `NoSelfGrant`,
  `HumanOverride`, `KernelImmutability`, `QuorumLegitimate`. Two invariants are
  deliberately **not** run through the engine on this path:
  - `ProvenanceVerifiable` — enforced directly and unconditionally via the gate's
    Ed25519 signature check (`verify_write_signature`) over the canonical payload
    hash, i.e. the same cryptographic binding, just not re-run through the engine.
  - `AuthorityChainValid` — **documented-as-future**: the dag-db consent schema
    stores a bailment + consent grant, not a per-link Ed25519-signed delegation
    chain, so an `InvariantContext` built from it has an empty authority chain.
    Running this invariant would fail-closed-block *every* legitimate dag-db
    write (a deadlock, not enforcement). Authorization on this path is instead
    established by the tenant-scoped consent grant (`ConsentRequired`) plus the
    route-layer session-authority binding. Loading a signed authority chain into
    the resolver to enable engine-level `AuthorityChainValid` is a follow-up.

  This is narrower than the prior claim that "all mutation surfaces enforce the
  constitutional `InvariantEngine`" — that claim is corrected here to the honest
  enforced subset.

### Tenant isolation (GAP-012 P1-E)

Tenant isolation is enforced at the storage layer by a `tenant_id` + `namespace`
pair carried on every row. Content-addressed rows (`dagdb_receipts.receipt_hash`,
`dagdb_memory_objects.memory_id`, `dagdb_catalog_entries.catalog_id`,
`dagdb_context_packet_records.packet_id`, …) use the **global 32-byte hash as the
primary key**, so a given hash maps to exactly one row owned by exactly one
tenant. P1-E hardened three layers:

- **By-hash read predicates (landed).** The data-returning by-hash reads now
  carry an explicit `AND tenant_id = $ AND namespace = $` (or project scope for
  packets) so a caller presenting another tenant's hash gets *not found* rather
  than fetching the cross-tenant row: `verify_export_record_row`,
  `verify_export_receipt_row`, and the export evidence memory read in
  `crates/exo-dag-db-postgres/src/postgres/kg_export.rs`, plus the context-packet replay
  guard in `crates/exo-dag-db-postgres/src/postgres/context_packet_persistence.rs`. The
  cross-tenant case is proven fail-closed by
  `export_evidence_read_is_tenant_scoped_cross_tenant_by_hash_fails_closed`
  (`tests/kg_export_persistence_contract.rs`).
  - The `ensure_*_match` / `row_mismatch` write-consistency guards in
    `kg_import` / `kg_writeback` intentionally read by the **global hash only**
    (no tenant predicate) and then compare the full scope+content in Rust. This
    is *already* fail-closed: a cross-tenant hash collision is rejected with a
    `Conflict`, and the row contents never reach the caller. Adding a tenant
    predicate there would convert that rejection into a silent `None → Ok →
    ON CONFLICT DO NOTHING` no-op write — a regression — so those reads are left
    as the global-hash consistency check by design. The residual is a weak
    existence-oracle (a caller can tell a hash *exists* under some tenant via the
    `Conflict` vs. proceed distinction), tracked as a follow-up below.
- **Write-time identity validation (landed).** `tenant_id` and `namespace` are
  validated and required to be in canonical, charset-safe form
  (`[A-Za-z0-9_:.-]`, non-empty, ≤128 bytes, no untrimmed whitespace) at the
  import and writeback write entrypoints
  (`KgImportDryRunReport::validate`, `KgWritebackDryRunReport::validate_for_persistence`),
  routed through `exo_dag_db_core::tenant::normalize_tenant_id`. Malformed or ambiguous
  identities fail closed before any write, preventing *new* divergence.
- **Canonical tenant constant (landed).** `exo_dag_db_core::tenant::LOCAL_DEV_TENANT_ID
  = "dag_db-local"` and `LOCAL_DEV_NAMESPACE = "dag_db"` are the single source of
  truth. The underscore form was chosen as canonical because it is what the
  shipping write paths (`exo_gateway::dagdb` local-dev mount and
  `continuation_packet`) already persist. Both `continuation_packet` and the
  gateway local-dev constants now route through this `const`. The hyphen form
  `dag-db-local` only ever
  appears in test fixtures and one smoke binary
  (`bin/dagdb_agent_brain_writeback_growth_smoke.rs`); no shipping write path
  emits it.

#### Tracked follow-ups

- **Existing-data reconciliation (no destructive rewrite).** Corpus memory is
  **append-only** — rows already written under a non-canonical `tenant_id` (e.g.
  a stray `dag-db-local`) must **not** be rewritten/deleted. Reconciliation is a
  forward-only operation: (1) audit `SELECT DISTINCT tenant_id, namespace FROM
  dagdb_memory_objects` (and the other scoped tables) to enumerate any divergent
  partitions; (2) if a divergent partition exists, supersede its rows into the
  canonical tenant via the normal append/supersession path (new rows + receipts),
  never an in-place `UPDATE`. The local dev stack currently writes only the
  canonical `dag_db-local`, so no reconciliation is outstanding there; this
  procedure is the contract for any environment that already wrote the hyphen
  form.
- **Row-Level Security (deferred — design below).** RLS is the strong
  defense-in-depth layer and is **not** applied in P1-E because the gateway uses a
  single shared `PgPool` (`max_connections = 10`) whose `search_path` is baked
  into the connect options with **no per-request tenant binding**
  (`exo_gateway::db::init_pool`). Applying RLS requires a per-request tenant GUC,
  which is a connection-model refactor larger than this ticket and dangerous to
  half-apply: enabling RLS with a `USING (tenant_id =
  current_setting('app.tenant_id', true))` policy while any code path forgets to
  set the GUC makes `current_setting` return NULL and **denies all rows on that
  path** (fail-closed but functionally broken). The intended design, to be landed
  as a dedicated slice:
  1. Migration (idempotent): for each tenant-scoped table,
     `ALTER TABLE <t> ENABLE ROW LEVEL SECURITY;`
     `ALTER TABLE <t> FORCE ROW LEVEL SECURITY;`
     `CREATE POLICY tenant_isolation ON <t> USING (tenant_id =
     current_setting('app.tenant_id', true) AND namespace =
     current_setting('app.namespace', true));`
  2. App wiring: every request acquires a transaction and, as its first
     statement, runs `SELECT set_config('app.tenant_id', $1, true),
     set_config('app.namespace', $2, true)` (transaction-local `is_local = true`,
     so the setting cannot leak across pool checkouts). All DAG DB queries on that
     request must run inside that transaction.
  3. Rollout gate: the GUC must be threaded through **every** `&PgPool` read/write
     path before the policy is enabled, otherwise unmigrated paths deny all rows.
     This is why P1-E lands predicate hardening + validation + the canonical split
     fix and defers RLS rather than half-applying it.

### Provisioning

The dag-db schema is applied by a single ledgered migrator on gateway startup
(T4): `exo_gateway::db::init_pool` runs the gateway's own migrations, then calls
`exo_dag_db_postgres::postgres::run_migrations_in_schema` to provision the dag-db tables
into a dedicated `dagdb` Postgres schema. The migration SQL is **embedded in the
binary** at compile time by `sqlx::migrate!`, so the deploy image needs **no
Dockerfile change** to copy `crates/exo-dag-db-postgres/migrations/` — provisioning is
purely from the compiled binary. A fresh container must answer one
`/api/v1/dag-db/*` call after startup; if the dag-db migration fails, startup
aborts (fail closed) so the gateway never serves dag-db routes against an
unprovisioned schema.

The dedicated `dagdb` schema holds the dag-db tables **and** their own
`_sqlx_migrations` ledger. This is required because sqlx 0.8 hardcodes the
migration-tracking table name and the gateway and dag-db crates reuse the same
integer migration versions (`20260505000001`, `20260602000001`) for different
SQL; a shared `public._sqlx_migrations` would collide on version with a
mismatched checksum and abort startup. The gateway pool's `search_path` is set to
`public,dagdb` so bare-named gateway queries resolve in `public` and bare-named
dag-db queries resolve in `dagdb`. Local launch flows no longer apply a divergent
psql glob for the dag-db schema; the gateway binary is the single authoritative
provisioning path.

**Existing-store cutover (operational residual).** A store previously provisioned
by the old psql glob holds its dag-db tables in `public`. After this change the
migrator creates an empty `dagdb` copy, and because `search_path` lists `public`
first, bare dag-db queries keep resolving to the existing `public` data — no read
data loss, but the `dagdb` copy stays unused until a one-time data migration moves
the rows (or the deployment is recreated on a fresh database). Fresh deploys have
no `public` dag-db tables and resolve cleanly to `dagdb`. The offline dev/benchmark
tools (`kg_export` / `kg_import` / `writeback_sign`) use the dag-db crate's own
`init_pool` without the gateway's `search_path`, so they operate on `public`; this
divergence on fresh deploys is tracked as a follow-up, out of scope here.

### Versioned v1 REST wire contract

Every consumer-facing `/api/v1/dag-db/*` **response** body now carries a stable
`schema_version` string so a non-Rust integrator can detect the wire-contract
version directly from the response. The constants are owned by `exo-api`
(`crates/exo-api/src/dagdb.rs`, `DAGDB_*_RESPONSE_SCHEMA_VERSION`) and are the
single source of truth:

| Endpoint | Response DTO | `schema_version` |
| --- | --- | --- |
| `POST /intake` | `DagDbIntakeResponse` | `dagdb_intake_response_v1` |
| `POST /route` | `DagDbRouteResponse` | `dagdb_route_response_v1` |
| `POST /context-packet` | `DagDbContextPacketResponse` | `dagdb_context_packet_response_v1` |
| `POST /validate` | `DagDbValidateResponse` | `dagdb_validate_response_v1` |
| `POST /writeback` | `DagDbWritebackResponse` | `dagdb_writeback_response_v1` |
| `POST /import` | `DagDbImportResponse` | `dagdb_import_response_v1` |
| `POST /export` | `DagDbExportResponse` | `dagdb_export_response_v1` |
| `POST /trust-check` | `DagDbTrustCheckResponse` | `dagdb_trust_check_response_v1` |
| `POST /council/decision` | `DagDbCouncilDecisionResponse` | `dagdb_council_decision_response_v1` |
| `GET /receipts/{hash}` | `DagDbReceiptLookupResponse` | `dagdb_receipt_lookup_response_v1` |
| `GET /catalog/{id}` | `DagDbCatalogLookupResponse` | `dagdb_catalog_lookup_response_v1` |
| `GET /routes/{id}` | `DagDbRouteLookupResponse` | `dagdb_route_lookup_response_v1` |

Request bodies and the shared `DagDbErrorEnvelope` are **not** versioned in v1.

#### Machine contract (codegen source)

`docs/dagdb/api/openapi.json` is an OpenAPI 3.1 document covering every route,
request body, response body, and the error envelope. It is the artifact a
non-Rust integrator codegens from. It is hand-authored from the `exo-api` DTOs
and **CI-asserted to stay in sync** by `crates/exo-api/tests/openapi_sync.rs`:
every fixture in `crates/exo-dag-db-api/fixtures/json/all_dto_fixtures.json`
validates against its component schema (and each fixture is independently
round-trip-asserted against its Rust DTO, so the spec's field set cannot drift
from the DTO's), and each documented `schema_version` `const` equals both the
Rust constant and the fixture value. Response schemas use
`additionalProperties: false` to mirror `#[serde(deny_unknown_fields)]`.

#### v1 `DagDbContextPacketResponse` vs. the internal `DagDbGraphContextPacket`

The REST `/context-packet` response (`DagDbContextPacketResponse`) is the
**canonical, versioned v1 contract**. The internal builder emits a richer,
separately-versioned `DagDbGraphContextPacket` (`dagdb_graph_context_packet_v1`)
that is never returned over HTTP. To close the previously-undocumented
divergence, the v1 REST response now surfaces the load-bearing rich fields that
the governed (persistent) path already has in scope
(`context_packet_response_from_persistent`,
`crates/exo-gateway/src/dagdb.rs`):

- `selected_graph_edges` — the selected graph edges (`DagDbSelectedGraphEdgeRef`).
- `citation_refs` — the packet's citation references.
- `packet_metrics` — token-budget / selection / savings-status metrics.
- `boundaries` — the blocked-claim boundaries (repository-test-level flags).
- `packet_markdown` — the rendered agent-facing markdown.

These are populated on the governed `production-db` path and are empty/`null` on
the no-database **scaffold** path (which has no built packet); they are
optional+`skip_serializing_if` so a scaffold response omits them. The v1 REST
contract therefore exposes the full internal packet's user-facing surface except
`agent_usage_instructions` and the packet's own `schema_version`/`task` echo,
which are **documented-as-follow-up**: surfacing `agent_usage_instructions` over
REST is a tracked addition for a future minor (additive, non-breaking) revision.
Consumers that need the byte-exact internal packet should treat
`DagDbGraphContextPacket` as the internal contract and the REST response as the
governed projection of it.

### Honest scope

`exo-dag-db` delivers deterministic, graph-governed cross-agent retention/recall
with measured context compression. It does **not** yet claim to be cheaper *and*
better than raw context: the rigorous benchmark fails cost-vs-neutral and the 80%
token-reduction floor, and the proof gate returns not-accepted. See `T3` and the
shipped DAG DB docs under `docs/dagdb/`.
