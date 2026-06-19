# DAG DB Runtime Activation Rollback, Canary, and Observability Plan

## Summary

This plan defines the production-runtime evidence path for the DAG DB gateway REST
surface served under `/api/v1/dag-db/*` plus the default-compiled MCP proxy evidence
path. It covers rollback, canary rollout, observability, and failure-mode proof
without claiming billing savings or thesis acceptance.

Exit criterion: The document states the exact runtime claim and the claims it excludes.

## Locked Decisions

- Runtime claim: gateway REST paths for default route, context packet build,
  writeback, import, and export when Postgres, tenant/session authority, and
  write signatures are configured.
- MCP claim: `dagdb-gateway-proxy` builds a configured SDK proxy that fails
  closed without complete gateway auth/scope.
- RLS claim: tenant-scoped DAG DB tables have an RLS migration and transaction
  tenant binding; namespace remains enforced by query predicates.
- Rollback mechanism: route traffic to the previous gateway image or remove the
  gateway `DATABASE_URL` from the DAG DB route context; either path must produce
  fail-closed `503 database_unavailable` responses for persistence routes.
- Canary sequence: 1 percent for 30 minutes, 10 percent for 60 minutes, 50
  percent for 120 minutes, then 100 percent only after every gate in this plan
  is green.
- Direct table writes are never an activation or rollback mechanism.
- Main integration owns the final proof commands and CI/build/test status.

Exit criterion: The rollout and rollback choices are fixed and do not require operator interpretation.

## Deferred Phases

This section lists work excluded from the REST activation claim.

| item | activation trigger |
| --- | --- |
| Namespace-bound RLS policy | RLS policy includes both `tenant_id` and `namespace` after every transaction binds both settings. |
| Additional route persistence | Intake, validate, trust-check, council-decision, receipt lookup, catalog lookup, and route lookup have governed persistence paths and failure-mode tests. |

Exit criterion: Excluded work has concrete activation triggers and is not described as current production evidence.

## Requirements Specification

### Functional

- `/api/v1/dag-db/route` persists a governed default route only after tenant
  authority and write signature checks pass.
- `/api/v1/dag-db/context-packet` builds governed context from persisted DAG DB
  rows when the pool and authority gates are configured.
- `/api/v1/dag-db/writeback` persists a governed memory only after tenant-scoped
  consent, Ed25519 provenance, and invariant checks pass.
- `/api/v1/dag-db/import` and `/api/v1/dag-db/export` require distinct
  import/export consent and idempotency checks before persistence.
- Missing DB pool returns `503 database_unavailable`; missing write signature
  returns `400 invalid_request_shape`; denied consent returns `403 consent_denied`.

### Non-Functional

- Canary promotion requires DAG DB route 5xx rate below 1 percent over the
  current step window, excluding intentional fail-closed probes.
- Canary promotion requires p95 DAG DB route latency below 750 ms for writeback
  and context-packet requests over the current step window.
- Rollback must complete within 15 minutes from gate breach detection.
- Idempotency conflict responses must remain deterministic: same idempotency key
  plus different request hash returns `409`.

### Compatibility

- Public REST prefix remains `/api/v1/dag-db`.
- Response `schema_version` constants stay the v1 values documented in
  `INTEGRATION.md`.
- Existing `tenant_id` + `namespace` storage scope remains unchanged.
- Existing graph-governed benchmark/thesis caveats remain unchanged.

### Observability

- Required counters: `dagdb_route_requests_total`,
  `dagdb_route_fail_closed_total{reason}`, `dagdb_writeback_persisted_total`,
  `dagdb_import_persisted_total`, `dagdb_export_built_total`,
  `dagdb_consent_denied_total`, `dagdb_signature_missing_total`,
  `dagdb_idempotency_conflict_total`, and `dagdb_db_unavailable_total`.
- Required latency metrics: `dagdb_writeback_latency_ms_p95`,
  `dagdb_context_packet_latency_ms_p95`, `dagdb_import_latency_ms_p95`, and
  `dagdb_export_latency_ms_p95`.
- Required log fields: `route`, `tenant_id`, `namespace`, `status`,
  `error_code`, `idempotency_key`, `receipt_hash` when produced, and
  `requires_council_review`.

Exit criterion: Functional, non-functional, compatibility, and observability requirements are concrete and measurable.

## Production Runtime Activation Contract

| claim | scope | proof command or check |
| --- | --- | --- |
| Gateway builds the DAG DB persistence path by default | `crates/exo-gateway/Cargo.toml`, `crates/exo-node/Cargo.toml` | `rg -n "default = \\[\"production-db\"\\]|default = \\[\"exo-gateway/default\"\\]" crates/exo-gateway/Cargo.toml crates/exo-node/Cargo.toml` |
| Gateway serves the REST prefix | `crates/exo-gateway/src/server.rs`, `crates/exo-gateway/src/dagdb.rs` | `rg -n "dagdb_router\\(\\)|/api/v1/dag-db" crates/exo-gateway/src/server.rs crates/exo-gateway/src/dagdb.rs` |
| D5 persistence methods are reached by served routes | `crates/exo-gateway/src/dagdb.rs` | `rg -n "persist_default_route|persist_context_packet_record|persist_lifecycle_action|persist_continuation_record" crates/exo-gateway/src/dagdb.rs` |
| RLS migration and tenant binding are present | `crates/exo-dag-db-postgres` | `test -f crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql && rg -n "ENABLE ROW LEVEL SECURITY|bind_tenant_context|begin_tenant_transaction" crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql crates/exo-dag-db-postgres/src` |
| MCP gateway proxy dispatches through SDK when configured | `crates/exo-node/src/mcp/tools/dagdb.rs` | `cargo test -p exo-node --features dagdb-gateway-proxy configured_gateway_proxies_all_dagdb_mcp_tools_with_auth_and_tenant_scope` |
| No-pool runtime fails closed | gateway DAG DB route tests | `cargo test -p exo-gateway --features production-db dagdb_default_router_returns_explicit_runtime_failure_for_every_get_and_post_route` |
| Cross-tenant access fails closed | gateway DAG DB route tests | `cargo test -p exo-gateway --features production-db dagdb_cross_tenant_denies_every_get_and_post_route` |
| Live DB writeback uses real DB authority | gateway live-Postgres test | `EXO_DAGDB_TEST_DATABASE_URL=<operator-set> cargo test -p exo-gateway --features production-db writeback_authorizes_against_real_db_consent_and_identity_state` |
| OpenAPI remains DTO-compatible | API sync test | `cargo test -p exo-api --test openapi_sync` |

Exit criterion: Each runtime claim maps to a runnable proof command or observable check.

## Implementation Slices

### Slice 1: Evidence Wiring

goal: collect and publish the exact proof outputs from the main integration run.
allowed write scope: `docs/dagdb/runtime-activation/**`, `INTEGRATION.md`,
`README.md`, `AGENTS.md`, `governance/threat_matrix.md`,
`governance/traceability_matrix.md`.
requirements: record command, environment prerequisite, exit status, and log
artifact path; never convert skipped live-DB tests into pass claims.
specification: append an evidence table under this directory after main
integration runs the commands in the Production Runtime Activation Contract.
test plan: run `git diff --check` and the stale-phrase `rg` proof after the
evidence table update.
exit criterion: The evidence table names only commands that actually ran.

### Slice 2: Canary And Rollback Run

goal: execute the canary sequence and rollback drill against the target
environment.
allowed write scope: `docs/dagdb/runtime-activation/**`.
requirements: capture step window, traffic percentage, metrics, rollback trigger
decision, and final promotion/rollback verdict.
specification: promotion requires every metric gate in Requirements
Specification to pass for the full step window.
test plan: perform one rollback drill during the 1 percent canary and verify
`503 database_unavailable` on persistence routes after disablement.
exit criterion: The runbook contains a dated canary record with a promotion or rollback verdict.

Exit criterion: The implementation slices are bounded to docs and operator evidence.

## Test Plan

### Baseline Commands

- `rg -n -i "o[p]t-in|d[e]fault-off|d[o]rmant|n[o]t wired|production runtime block[e]d|RLS deferr[e]d|gated-but-unus[e]d" README.md AGENTS.md INTEGRATION.md governance/threat_matrix.md governance/traceability_matrix.md docs/dagdb/api/openapi.json docs/dagdb/runtime-activation`
- `git diff --check`

### Per-Slice Commands

- Slice 1: run every command in the Production Runtime Activation Contract and
  paste only actual outcomes into the evidence table.
- Slice 2: run the canary/rollback checks in the target environment and record
  metrics for each traffic step.

### Final Agent-Runnable Sequence

1. `rg -n -i "o[p]t-in|d[e]fault-off|d[o]rmant|n[o]t wired|production runtime block[e]d|RLS deferr[e]d|gated-but-unus[e]d" README.md AGENTS.md INTEGRATION.md governance/threat_matrix.md governance/traceability_matrix.md docs/dagdb/api/openapi.json docs/dagdb/runtime-activation`
2. `git diff --check`

### Operator-Only Steps

- Provide `EXO_DAGDB_TEST_DATABASE_URL` for live-Postgres tests.
- Execute production canary traffic shifting.
- Confirm rollback target image or database-disablement action in the deployment platform.

Exit criterion: Agent-runnable checks and operator-only checks are separated.

## Definition of Done

- README describes DAG DB as a runtime adapter, not a disabled overlay.
- AGENTS.md describes the REST runtime and default-compiled MCP proxy status.
- INTEGRATION.md states that gateway/node defaults include the production DB path.
- INTEGRATION.md states that RLS implementation is present and evidence pending.
- INTEGRATION.md states that D5 persistence methods are reached by served routes.
- Threat and traceability matrices describe T-17/DAGDB-001 as REST activation
  evidence pending until main integration runs.
- OpenAPI server description does not call the REST surface disabled.
- This runbook exists under `docs/dagdb/runtime-activation/`.
- Stale-phrase `rg` proof has no rejected framing outside this plan's required
  `Deferred Phases` heading.
- `git diff --check` passes.
- No CI/build/test pass is claimed unless main integration supplies the command
  output.

Exit criterion: Every done item is checkable from the worktree or main integration evidence.

## Post-Implementation Review

When the Definition of Done is met and the checkpoint commit is created, a post-implementation review pass runs before push or deployment. The pass walks code review, test coverage, hardening, end-to-end verification, and documentation. Blockers found in the review are fixed and the relevant layers re-run before ship.

Review scope: `README.md`, `AGENTS.md`, `INTEGRATION.md`, `governance/threat_matrix.md`, `governance/traceability_matrix.md`, `docs/dagdb/api/openapi.json`, `docs/dagdb/runtime-activation/**`.
Review trigger: Definition of Done met and checkpoint commit created.
Review verdict gate: Ship | Fix blockers and re-run | Hand back to planning.

Exit criterion: The post-implementation review scope, trigger, and verdict gate are explicit and require review before push or deployment.
