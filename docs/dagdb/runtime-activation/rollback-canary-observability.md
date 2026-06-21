# DAG DB Enterprise Runtime Closure Plan

## Summary

This plan is the active source of truth for closing EXOCHAIN PR #695 final-round DAG DB runtime feedback. It makes the production claim truthful by keeping only the five governed, DB-backed, observable REST endpoints listed below mounted and by requiring external production finality, tenant RLS, consent-separated import/export, MCP/SDK live proxy evidence, exact coverage claims, rollback/canary evidence, and test-first execution.

Exit criterion: This section identifies PR #695 final-round DAG DB runtime closure as the single active objective and bounds the production claim to mounted live surfaces.

## Locked Decisions

- Active source of truth path: `docs/dagdb/runtime-activation/rollback-canary-observability.md`.
- GitHub PR target: `exochain/exochain#695`.
- Branch delivery model: dedicated PR branch only; no direct commit to `main`, no merge, and no auto-merge.
- Production-mounted REST endpoints are exactly `POST /api/v1/dag-db/route`, `POST /api/v1/dag-db/context-packet`, `POST /api/v1/dag-db/writeback`, `POST /api/v1/dag-db/import`, and `POST /api/v1/dag-db/export`.
- Reserved DTO-only surfaces are exactly `POST /api/v1/dag-db/intake`, `POST /api/v1/dag-db/validate`, `POST /api/v1/dag-db/trust-check`, `POST /api/v1/dag-db/council/decision`, `GET /api/v1/dag-db/receipts/{hash}`, `GET /api/v1/dag-db/catalog/{id}`, and `GET /api/v1/dag-db/routes/{id}`; these remain unmounted from the production router until governed persistence and tests exist.
- External production finality is mandatory for accepted or approved D5 state; gateway-local construction, requester self-approval, a merely registered non-requester DID, caller-controlled JSON, and shaped placeholder evidence must not graduate records to accepted or approved.
- External finality verification must bind tenant ID, namespace, actor DID, route or packet or action identity, request ID or idempotency key, canonical payload hash, authority DID, authority signature, timestamp, and route purpose.
- Finality authority must reuse existing EXOCHAIN consent, identity, AVC, council, or gatekeeper authority mechanics; no new ad hoc allowlist, hard-coded production DID, fake authority, or parallel governance system is permitted.
- Import, export, and writeback are separate consent purposes; `dag-db:writeback:{tenant_id}` must not authorize import or export, `dag-db:import:{tenant_id}:{namespace}` must not authorize export, and `dag-db:export:{tenant_id}:{namespace}` must not authorize import.
- RLS is mandatory on every tenant-owned live DAG DB table; all production query paths must use tenant-bound transactions and fail closed without `exo.tenant_id`.
- MCP and SDK are live proxy surfaces only when configured with gateway URL, bearer token, tenant ID, namespace, actor DID, authority scope, and route-specific signature material; missing configuration or missing signatures fail before HTTP.
- Coverage claims must name the exact command, crate/package set, exclusions, denominator, and live-Postgres prerequisites that produced the number.
- No production behavior may be changed without a red-green-refactor record: failing test first, production change second, focused green test third, broader suite fourth.
- DAG DB local memory retrieval is not proof for this PR unless `/ready` is healthy and the repository memory scripts exist; missing memory runtime is recorded as a tooling blocker, not as a code claim.

Exit criterion: Every ambiguous execution, route, finality, consent, RLS, MCP/SDK, coverage, and delivery decision is fixed with literal paths and route names.

## Deferred Phases

| deferred item | activation trigger |
| --- | --- |
| Mount reserved intake, validate, trust-check, council decision, receipt lookup, catalog lookup, and route lookup endpoints | Each endpoint has DB-backed governed persistence or lookup, tenant-bound transaction tests, idempotency or lookup replay tests, durable audit events, MCP/SDK contract updates where applicable, OpenAPI updates, and independent review verdict `Ship`. |
| Namespace-bound PostgreSQL RLS policy in addition to tenant RLS | Every live DAG DB transaction binds both `exo.tenant_id` and `exo.namespace`, migration adds namespace-scoped policies, and cross-namespace read/write/update/delete/import/export/lookup denial tests pass under a non-bypass role. |
| Production canary traffic promotion | A target deployment exists with operator-controlled traffic shifting, live Postgres, tenant authority records, configured MCP proxy, metrics scrape, and rollback target image. |

Exit criterion: Deferred work is explicitly unmounted or operator-gated and has concrete activation triggers.

## Requirements Specification

### Functional

- The five mounted REST endpoints persist or read only through governed DAG DB runtime code paths backed by Postgres when database, tenant/session authority, consent, signatures, and finality evidence are configured.
- `/api/v1/dag-db/route` persists a default-route record only after requester write authorization and externally verified production finality approval pass.
- `/api/v1/dag-db/context-packet` persists a context-packet record only after requester write authorization and externally verified production finality approval pass.
- `/api/v1/dag-db/writeback` persists memory, lifecycle, and continuation records only after writeback consent, Ed25519 requester signatures, lifecycle approval evidence, continuation approval evidence, idempotency checks, and D5 validation pass.
- `/api/v1/dag-db/import` persists imported material only under import consent, import-bound signature payload, tenant/namespace scope, idempotency replay safety, and real import persistence.
- `/api/v1/dag-db/export` builds export material only under export consent, export-bound signature payload, tenant/namespace scope, idempotency replay safety, and real export readback.
- Accepted or approved D5 state is impossible from gateway-local self-approval, caller-controlled JSON, registered-but-unauthorized DID signatures, forged signatures, mismatched tenant, mismatched namespace, mismatched payload hash, mismatched actor, mismatched route, mismatched request ID, or stale timestamp evidence.
- RLS blocks cross-tenant read, write, update, delete, import, export, lookup, and idempotency access under a non-superuser/non-bypass role.
- MCP configured proxy calls the live gateway contract through SDK request construction, not a local fake path.
- Missing MCP config, missing route-specific signatures, mismatched tenant/namespace, and gateway denial return typed fail-closed statuses.
- Durable receipts or audit records exist for approval request submitted, approval granted or denied, record accepted, import completed, export completed, replay detected, idempotency conflict, RLS/tenant violation, signature failure, and council/operator decision where the mounted code path can produce the event.

### Non-Functional

- Idempotency is deterministic: same route, tenant, namespace, idempotency key, and request hash replays the stored response; same key with different material returns `409 idempotency_key_conflict`.
- Failure paths must fail before durable mutation when authority, consent, signature, tenant context, finality, or idempotency checks fail.
- Tenant-bound transactions must bind tenant context before any table read or write.
- No mounted production route returns synthetic success, placeholder success, or database-unavailable from a helper that skipped an available persistence closure.
- Structured status and logs must not expose raw secrets, bearer tokens, private keys, or raw signatures.
- Rollback by disabling DB configuration or reverting to the previous gateway image must return fail-closed `503 database_unavailable` for persistence routes within 15 minutes.
- Canary promotion requires DAG DB route 5xx rate below 1 percent excluding intentional fail-closed probes, p95 writeback and context-packet latency below 750 ms, zero cross-tenant isolation violations, and zero signature-bypass findings for the full step window.

### Compatibility

- Public REST prefix remains `/api/v1/dag-db`.
- Response `schema_version` constants remain the v1 constants owned by `crates/exo-api/src/dagdb.rs`.
- Existing DTO structs in `exo-api` remain the wire source of truth.
- Existing tenant ID plus namespace storage scope remains unchanged.
- Reserved DTO-only surfaces remain available as DTO types but are not production router claims.
- Existing graph-governed benchmark and thesis caveats remain unchanged; this plan does not claim billing savings or thesis acceptance.
- Existing CI gate names remain authoritative: Gate 2 workspace tests, Gate 3 scoped coverage, Gate 12 gateway integration, and Gate 13 production-db integration.

### Observability

- Required durable audit event categories: `dagdb_approval_request_submitted`, `dagdb_approval_granted`, `dagdb_approval_denied`, `dagdb_record_accepted`, `dagdb_import_completed`, `dagdb_export_completed`, `dagdb_replay_detected`, `dagdb_idempotency_conflict`, `dagdb_rls_tenant_violation`, `dagdb_signature_failure`, and `dagdb_council_operator_decision`.
- Required counters or structured status fields: `dagdb_route_requests_total`, `dagdb_route_fail_closed_total{reason}`, `dagdb_writeback_persisted_total`, `dagdb_import_persisted_total`, `dagdb_export_built_total`, `dagdb_consent_denied_total`, `dagdb_signature_missing_total`, `dagdb_signature_failure_total`, `dagdb_external_finality_denied_total`, `dagdb_idempotency_replay_total`, `dagdb_idempotency_conflict_total`, `dagdb_rls_tenant_violation_total`, and `dagdb_db_unavailable_total`.
- Required health states: `dagdb_active`, `dagdb_degraded`, and `dagdb_unavailable`, with clear reason codes for missing DB config, failed DB pool, missing tenant context, missing authority, missing MCP config, and route-level denial.
- Required structured log fields: `route`, `tenant_id`, `namespace`, `status`, `error_code`, `idempotency_ref`, `receipt_hash` when produced, `authority_did` when safe, and `requires_council_review`.
- Required PR evidence fields: command, exit status, checked crates, package exclusions, line coverage numerator and denominator, branch coverage result when available, live-Postgres prerequisite, and skipped-provider reason when a live test is not run.

Exit criterion: Functional, non-functional, compatibility, and observability requirements cover every final-round feedback category with checkable behavior.

## Enterprise Runtime Closure Contract

| claim | scope | proof command or check |
| --- | --- | --- |
| Only live governed surfaces are mounted | `crates/exo-gateway/src/dagdb.rs`, `INTEGRATION.md`, `docs/dagdb/api/openapi.json` | `rg -n "route\\(\"/api/v1/dag-db/(intake|validate|trust-check|council|receipts|catalog|routes)" crates/exo-gateway/src/dagdb.rs crates/exo-gateway/src/server.rs` returns no production mount. |
| Route finality is external and authority-scoped | `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs` | `cargo test -p exo-gateway --features production-db default_route_finality_rejects_requester_self_approval_before_persistence` and `cargo test -p exo-gateway --features production-db default_route_finality_rejects_registered_non_authority_before_persistence` |
| Context-packet finality is external and authority-scoped | `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs` | `cargo test -p exo-gateway --features production-db context_packet_finality_rejects_registered_non_authority_before_persistence` |
| D5 domain constructors reject shaped placeholder approval evidence | `crates/exo-dag-db-domain/src/default_route.rs`, `crates/exo-dag-db-domain/src/context_packet_persistence.rs`, `crates/exo-dag-db-domain/src/lifecycle_action.rs`, `crates/exo-dag-db-domain/src/continuation_persistence.rs` | `RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-domain --test prd17_default_retrieval_contract --test prd17_lifecycle_contract` |
| Import/export/writeback consent purposes are separated | `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs` | `cargo test -p exo-gateway --features production-db import_export_authorization_deny_cross_purpose_consent_and_signature` |
| Import/export idempotency detects replay and material conflict | `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs` | `cargo test -p exo-gateway --features production-db --test dagdb_route_integration_contract dagdb_routes_integration_contract` |
| RLS blocks tenant-bound live paths under non-bypass role | `crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`, `crates/exo-dag-db-postgres/tests/dagdb_tenant_rls_live_path_contract.rs` | `RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-postgres --features postgres --test dagdb_tenant_rls_live_path_contract -- --nocapture` |
| MCP configured proxy uses live gateway contract | `crates/exo-node/src/mcp/tools/dagdb.rs`, `crates/exochain-sdk/src/dagdb.rs` | `RUSTFLAGS='-D warnings' cargo test -p exo-node dagdb --features dagdb-gateway-proxy` and `RUSTFLAGS='-D warnings' cargo test -p exochain-sdk dagdb --features http-client` |
| Default-on runtime reports DAG DB active/degraded/unavailable honestly | `crates/exo-gateway/src/server.rs`, `crates/exo-gateway/src/dagdb.rs`, docs | `rg -n "dagdb_active|dagdb_degraded|dagdb_unavailable|database_unavailable" crates/exo-gateway/src crates/exo-node/src docs/dagdb/runtime-activation` plus gateway health tests. |
| Audit and operational status expose finality, replay, RLS, and signature outcomes | `crates/exo-gateway/src/dagdb.rs`, `crates/exo-dag-db-postgres/src/postgres/*`, docs | tests assert durable rows or structured status for required event categories; `rg -n "dagdb_approval_request_submitted|dagdb_idempotency_conflict|dagdb_rls_tenant_violation|dagdb_signature_failure" crates docs`. |
| Coverage claims are exact | PR comment, `tarpaulin.toml`, `tools/test_coverage_policy.sh` | `bash tools/test_coverage_policy.sh` and the PR body/comment names exact tarpaulin command, denominator, exclusions, and live-Postgres prerequisites. |

Definitions: `mounted production endpoint` means one of the five production routes registered by `exo-gateway` in Locked Decisions; `reserved DTO-only surface` means an `exo-api` request/response type that is not registered by the production router; `external production finality` means verified approval/finality evidence issued by a non-requester authority through existing EXOCHAIN consent, identity, AVC, council, or gatekeeper authority mechanics; `idempotency_ref` means a non-secret stable reference derived from the idempotency key; `tenant-bound transaction` means a Postgres transaction that calls `bind_tenant_context` or an equivalent helper before accessing tenant-owned DAG DB tables; `live-Postgres test` means a test that uses `EXO_DAGDB_TEST_DATABASE_URL` or CI `DATABASE_URL` against a real Postgres service.

Exit criterion: Every runtime claim maps to a concrete proof command or observable code check and every contract term is defined.

## Implementation Slices

Sub-Agent Delegation Protocol: Every implementation worker receives this plan, the maintainer final-round feedback, exact allowed and forbidden write scopes, and the red-green-refactor requirement. Workers must report failing-test command output before production edits, changed file paths, focused green commands, broader verification commands, coverage evidence, and remaining risks. Reviewer agents may not be implementation agents. Any worker that edits outside its scope, skips red evidence, creates a stub, fabricates proof, or duplicates a parallel system is rejected.

### Slice 1: Finality Authority and D5 Construction

goal: make accepted/approved route, context-packet, lifecycle, and continuation state require externally verified production finality authority, not gateway-local evidence or registered non-requester DID signatures alone.
allowed write scope: `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gatekeeper/src/dagdb_gate.rs`, `crates/exo-dag-db-domain/src/default_route.rs`, `crates/exo-dag-db-domain/src/context_packet_persistence.rs`, `crates/exo-dag-db-domain/src/lifecycle_action.rs`, `crates/exo-dag-db-domain/src/continuation_persistence.rs`, `crates/exo-dag-db-domain/tests/prd17_default_retrieval_contract.rs`, `crates/exo-dag-db-domain/tests/prd17_lifecycle_contract.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs`.
requirements: write failing tests first for self-approval, registered non-authority approval, forged approval, mismatched tenant, mismatched namespace, mismatched payload hash, mismatched actor, mismatched route, mismatched request ID, stale timestamp when a timestamp bound exists, and valid external approval graduation.
specification: reuse existing consent, identity, AVC, council, or gatekeeper authority primitives; add no new hard-coded authority DID and no new governance allowlist; leave unsupported finality unavailable paths proposed, pending, or operator_deferred.
test plan: red commands must include focused gateway and domain tests named in the Enterprise Runtime Closure Contract; green commands must rerun those tests plus `RUSTFLAGS='-D warnings' cargo check -p exo-gateway --features production-db`.
exit criterion: all Slice 1 finality tests fail before implementation, pass after implementation, and no accepted/approved path remains constructible from caller/gateway-local evidence alone.

### Slice 2: Mounted Route and Import/Export Runtime Contract

goal: ensure every mounted route is live, governed, DB-backed, tenant-bound, consent-separated, idempotent, and covered; keep unsupported DTO-only surfaces unmounted and unclaimed.
allowed write scope: `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs`, `crates/exo-gateway/tests/dagdb_cross_tenant.rs`, `crates/exo-api/tests/openapi_sync.rs`, `docs/dagdb/api/openapi.json`.
requirements: write failing tests first for import/export cross-purpose denial, replay equality, same-key changed-material conflict, persistence success, reserved route unmounted behavior, and no helper skipping an available persistence closure.
specification: mounted route handlers must call real persistence or readback closures when DB pool is configured; reserved DTO-only routes must not be mounted or described as served production routes.
test plan: red and green commands must include `cargo test -p exo-gateway --features production-db import_export_authorization_deny_cross_purpose_consent_and_signature`, route integration contract tests, `cargo test -p exo-api --test openapi_sync`, and `git diff --check`.
exit criterion: all mounted route contract tests pass and reserved DTO-only surfaces remain unmounted from the production router.

### Slice 3: RLS, Audit, and Observability Proof

goal: prove tenant isolation and operator-visible runtime status across live paths, including deletion, import/export, lookup/readback, finality denial, signature failure, replay, and idempotency conflict.
allowed write scope: `crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`, `crates/exo-dag-db-postgres/src/postgres/**`, `crates/exo-dag-db-postgres/tests/dagdb_tenant_rls_live_path_contract.rs`, `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gateway/src/server.rs`, `crates/exo-gateway/tests/dagdb_route_integration_contract.rs`, `docs/dagdb/runtime-activation/rollback-canary-observability.md`.
requirements: write failing tests first for cross-tenant delete denial, missing tenant context failure, non-bypass role enforcement, RLS violation classification, signature failure status, idempotency conflict status, replay status, and health states `dagdb_active`, `dagdb_degraded`, and `dagdb_unavailable`.
specification: all tenant-owned tables have `ENABLE ROW LEVEL SECURITY` and `FORCE ROW LEVEL SECURITY`; failure logs and status use safe references and never log raw signatures or bearer tokens.
test plan: red and green commands must include the live RLS contract test, gateway route integration contract tests, and any health/status test added by the worker.
exit criterion: RLS, audit/status, and health checks pass under a non-bypass role and expose the required status without leaking secrets.

### Slice 4: MCP/SDK Live Contract

goal: ensure SDK request construction and MCP tools invoke the live gateway contract and fail closed before HTTP when required configuration or signatures are absent.
allowed write scope: `crates/exochain-sdk/src/dagdb.rs`, `crates/exo-node/src/mcp/tools/dagdb.rs`, node and SDK tests colocated with those files.
requirements: write failing tests first for missing proxy configuration, missing route/context/writeback/import/export signatures, tenant/namespace mismatch, configured proxy request dispatch, and gateway denial propagation.
specification: SDK/MCP must emit the same route-specific signature and authority headers expected by the gateway and must not introduce local fake success responses.
test plan: red and green commands must include `RUSTFLAGS='-D warnings' cargo test -p exochain-sdk dagdb --features http-client` and `RUSTFLAGS='-D warnings' cargo test -p exo-node dagdb --features dagdb-gateway-proxy`.
exit criterion: MCP/SDK tests prove live gateway dispatch and fail-closed local validation.

### Slice 5: Documentation, Evidence, and PR Readiness

goal: align docs, traceability, threat matrix, OpenAPI, PR evidence, coverage claims, and runbook language with the final code behavior.
allowed write scope: `README.md`, `INTEGRATION.md`, `AGENTS.md`, `governance/threat_matrix.md`, `governance/traceability_matrix.md`, `docs/dagdb/api/openapi.json`, `docs/dagdb/runtime-activation/**`.
requirements: update docs only after code and tests define the true mounted surface; remove stale claims that import/export fail closed when consent exists; never describe scoped coverage as universal coverage.
specification: docs must say exactly which routes are mounted, which DTO surfaces are reserved, which tests require live Postgres, and what CI/local coverage commands proved.
test plan: run `rg` stale-claim checks, `jq empty docs/dagdb/api/openapi.json`, `bash tools/test_repo_truth.sh`, `bash tools/test_coverage_policy.sh`, and `git diff --check`.
exit criterion: docs and PR evidence match the code, no stale runtime claim remains, and repo-truth plus coverage-policy checks pass.

Exit criterion: Each implementation slice has a goal, allowed write scope, requirements, specification, test plan, and exit criterion with test-first sequencing.

## Test Plan

### Baseline Commands

- `git status --short --branch`
- `gh pr view 695 --repo exochain/exochain --json url,number,title,state,isDraft,headRefOid,statusCheckRollup,reviewDecision,mergeStateStatus,comments,reviews,mergedAt,closed`
- `gh pr checks 695 --repo exochain/exochain --watch=false`
- `cargo fmt --all -- --check`
- `git diff --check`
- `bash tools/test_repo_truth.sh`
- `bash tools/test_coverage_policy.sh`

### Per-Slice Commands

- Slice 1: `RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-domain --test prd17_default_retrieval_contract --test prd17_lifecycle_contract`; `cargo test -p exo-gateway --features production-db default_route_finality_rejects_requester_self_approval_before_persistence`; `cargo test -p exo-gateway --features production-db default_route_finality_rejects_registered_non_authority_before_persistence`; `cargo test -p exo-gateway --features production-db context_packet_finality_rejects_registered_non_authority_before_persistence`; `RUSTFLAGS='-D warnings' cargo check -p exo-gateway --features production-db`.
- Slice 2: `cargo test -p exo-gateway --features production-db import_export_authorization_deny_cross_purpose_consent_and_signature`; `cargo test -p exo-gateway --features production-db --test dagdb_route_integration_contract dagdb_routes_integration_contract`; `cargo test -p exo-api --test openapi_sync`.
- Slice 3: `RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-postgres --features postgres --test dagdb_tenant_rls_live_path_contract -- --nocapture`; gateway health/status tests added by the slice; `rg -n "dagdb_approval_request_submitted|dagdb_idempotency_conflict|dagdb_rls_tenant_violation|dagdb_signature_failure" crates docs`.
- Slice 4: `RUSTFLAGS='-D warnings' cargo test -p exochain-sdk dagdb --features http-client`; `RUSTFLAGS='-D warnings' cargo test -p exo-node dagdb --features dagdb-gateway-proxy`.
- Slice 5: stale-claim checks, `jq empty docs/dagdb/api/openapi.json`, `bash tools/test_repo_truth.sh`, `bash tools/test_coverage_policy.sh`, and `git diff --check`.

### Final Agent-Runnable Sequence

1. `cargo fmt --all -- --check`
2. `git diff --check`
3. `bash tools/test_repo_truth.sh`
4. `bash tools/test_coverage_policy.sh`
5. `RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-domain --test prd17_default_retrieval_contract --test prd17_lifecycle_contract`
6. `RUSTFLAGS='-D warnings' cargo test -p exo-gateway dagdb --features production-db`
7. `RUSTFLAGS='-D warnings' cargo test -p exochain-sdk dagdb --features http-client`
8. `RUSTFLAGS='-D warnings' cargo test -p exo-node dagdb --features dagdb-gateway-proxy`
9. `RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-postgres --features postgres --test dagdb_tenant_rls_live_path_contract -- --nocapture`
10. `RUSTFLAGS='-D warnings' cargo build --workspace --all-targets`
11. `cargo clippy --workspace --all-targets -- -D warnings`
12. `cargo tarpaulin --workspace --exclude exochain-wasm --exclude exo-proofs --out xml --output-dir coverage --engine llvm --timeout 300 --fail-under 93`
13. `gh pr checks 695 --repo exochain/exochain --watch=false`

### Operator-Only Steps

- Provide `EXO_DAGDB_TEST_DATABASE_URL` when local live-Postgres tests are required outside CI.
- Approve GitHub Actions runs for fork updates when GitHub marks a run `action_required`.
- Execute production canary traffic shifting.
- Confirm rollback target image or database-disablement action in the deployment platform.
- Decide whether PR #695 may be merged after independent review and required CI pass.

Exit criterion: Baseline, per-slice, final, and operator-only checks are explicitly separated and runnable.

## Definition of Done

- PR #695 remains open on a dedicated branch and is not merged by agents.
- The active source-of-truth plan is this file.
- No second active DAG DB final-round plan exists.
- Every executable behavior change has red-green-refactor evidence in the worker report.
- Only the five locked mounted routes are served by the production router.
- Reserved DTO-only endpoints are not production-mounted or described as served production routes.
- Gateway self-approval cannot produce accepted or approved state.
- Registered non-requester DID signatures without production finality authority cannot produce accepted or approved state.
- Forged approval receipts or signatures are rejected before persistence.
- Mismatched tenant, namespace, actor, route, payload hash, request ID, or timestamp finality evidence is rejected before persistence.
- Valid external finality evidence graduates proposed/pending/operator_deferred state to accepted or approved state.
- Import, export, and writeback consent purposes are separated by tests.
- Import/export replay and changed-material idempotency conflict are tested.
- Every live tenant-owned DAG DB table has forced RLS.
- RLS denial covers cross-tenant read, write, update, delete, import, export, lookup, and idempotency paths under a non-bypass role.
- Missing tenant context fails closed.
- MCP configured proxy calls the live gateway/SDK path.
- MCP and SDK missing signature/configuration paths fail before HTTP where the client can prove them.
- Default-on production runtime health reports `dagdb_active`, `dagdb_degraded`, or `dagdb_unavailable`.
- Durable audit or structured status evidence exists for every event category listed in Observability.
- Logs and status do not expose bearer tokens, private keys, or raw signatures.
- README, INTEGRATION, threat matrix, traceability matrix, OpenAPI, and runtime runbook match the implemented mounted surface.
- Coverage claims name exact commands, denominators, exclusions, and live-Postgres prerequisites.
- `bash tools/test_repo_truth.sh` passes.
- `bash tools/test_coverage_policy.sh` passes.
- Focused slice tests pass.
- Workspace build, clippy, and tarpaulin commands in the final sequence pass or the blocker is explicitly outside agent control.
- GitHub CI gates are checked after push and no in-scope failure remains.
- A separate independent reviewer returns verdict `Ship`.

Exit criterion: Every done item is independently verifiable by worktree inspection, command output, CI status, or independent review.

## Post-Implementation Review

When the Definition of Done is met and the checkpoint commit is created, a post-implementation review pass runs before push or deployment. The pass walks code review, test coverage, hardening, end-to-end verification, and documentation. Blockers found in the review are fixed and the relevant layers re-run before ship.

Review scope: `crates/exo-gateway/src/dagdb.rs`, `crates/exo-gatekeeper/src/dagdb_gate.rs`, `crates/exo-dag-db-domain/src/default_route.rs`, `crates/exo-dag-db-domain/src/context_packet_persistence.rs`, `crates/exo-dag-db-domain/src/lifecycle_action.rs`, `crates/exo-dag-db-domain/src/continuation_persistence.rs`, `crates/exo-dag-db-postgres/migrations/20260619000001_enable_dagdb_tenant_rls.sql`, `crates/exo-dag-db-postgres/src/postgres/**`, `crates/exo-dag-db-postgres/tests/dagdb_tenant_rls_live_path_contract.rs`, `crates/exo-node/src/mcp/tools/dagdb.rs`, `crates/exochain-sdk/src/dagdb.rs`, `crates/exo-gateway/tests/**`, `README.md`, `INTEGRATION.md`, `AGENTS.md`, `governance/threat_matrix.md`, `governance/traceability_matrix.md`, `docs/dagdb/api/openapi.json`, and `docs/dagdb/runtime-activation/**`.
Review trigger: Definition of Done met and checkpoint commit created.
Review verdict gate: Ship | Fix blockers and re-run | Hand back to planning.

Exit criterion: The post-implementation review scope, trigger, and verdict gate are explicit and require review before push or deployment.
