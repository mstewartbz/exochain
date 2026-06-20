# Summary

This plan remediates GitHub issues #696 through #700 for the AVC trust-receipt path by fixing receipt signature verification, adding truthful durability guardrails, binding receipts to action content commitments, adding hash-linkage for receipt ordering, and documenting the remaining external timestamp/anchor boundary. The plan preserves the existing `/api/v1/avc/*` API surface, existing credential and revocation semantics, and backward compatibility for previously serialized receipts.

Exit criterion: The remediation objective, user outcome, and preserved compatibility boundaries are stated in this section.

# Locked Decisions

1. Execution branch: `codex/avc-trust-receipt-issues-696-700`.
2. Base branch: `exochain/main`.
3. Canonical plan file: `docs/audit/AVC-TRUST-RECEIPT-ISSUES-696-700-PLAN.md`.
4. Issue #696 writes are scoped to `crates/exo-avc/src/registry.rs` unless node startup revalidation is required.
5. Issue #700 writes are scoped to `crates/exo-node/src/avc.rs`, `crates/exo-node/src/main.rs`, `deploy/NODE-ZERO.md`, `deploy/RAILWAY-HANDOFF.md`, and `docs/guides/production-deployment.md`.
6. Issues #697, #698, and #699 share receipt schema and emit-path ownership and execute after the architecture scout returns a non-overlapping implementation map.
7. No fake RFC-3161, blockchain, DAG, BCTS, or external anchor provider will be added.
8. If no real external timestamp or anchor provider exists in the current repository, the fix will record truthful timestamp provenance and defer external anchoring with an activation trigger.
9. Existing signed receipts remain deserializable by using optional new fields with `serde(default)` when receipt schema fields are added.
10. Receipt signing payload changes must be deterministic, canonical-CBOR encoded, and covered by regression tests.
11. The open DAG DB PR #695 remains untouched by this issue set.
12. GitHub issues #696 through #700 remain the external tracker until a maintainer closes or supersedes them.

Exit criterion: Every ambiguous execution, path, compatibility, and scope decision is pinned to a literal value.

# Deferred Phases

1. RFC-3161 TSA integration is deferred until maintainers provide approved TSA endpoint configuration, trust policy, timeout budget, retry budget, and acceptance-test credentials.
2. Public blockchain anchoring is deferred until maintainers select a chain, transaction format, key custody model, confirmation policy, fee policy, and replay-safe testnet validation path.
3. Full BCTS lifecycle integration for every AVC receipt is deferred until maintainers decide whether AVC receipt emission creates a BCTS transaction or attaches to a caller-supplied BCTS correlation ID.
4. Persisting public-key trust anchors is deferred until maintainers decide whether startup configuration remains the sole trust-anchor source or durable state becomes an authenticated trust-anchor store.

Exit criterion: Every deferred item has a concrete activation trigger and no deferred item is required for the in-scope code to be truthful.

# Requirements Specification

## Functional

1. `InMemoryAvcRegistry::put_receipt` verifies receipt signatures against `receipt.validator_did` when a validator public key is registered.
2. Durable receipt import rejects malformed receipt IDs, empty signatures, unknown credentials, and duplicate durable keys.
3. Live durable-state application rejects forged receipt signatures when startup trust anchors are available.
4. AVC receipt emission binds the receipt to an action content commitment derived from the submitted `AvcActionRequest`.
5. AVC receipt storage records enough linkage to detect deletion or reordering within the implemented receipt-chain scope.
6. AVC startup emits an operator-visible warning when Postgres-backed registry durability is unavailable.
7. Deployment documentation states that `DATABASE_URL` is required for production AVC registry durability.
8. Documentation states that public-key trust anchors are restored from verified startup configuration, not from ordinary durable registry state.

## Non-Functional

1. Receipt validation fails closed on unresolved validator keys in live storage paths.
2. Receipt schema changes are backward-compatible with legacy receipts.
3. Hashes are deterministic across platforms by using existing canonical structured hashing or canonical CBOR patterns.
4. The implementation introduces no new network dependency for timestamping or anchoring.
5. Tests run without real external services except existing optional Postgres tests that already skip when the configured database URL is absent.
6. Logging avoids secrets, signatures, raw private payloads, and database URLs.

## Compatibility

1. Existing `/api/v1/avc/issue`, `/api/v1/avc/validate`, `/api/v1/avc/receipts/emit`, `/api/v1/avc/receipts/:hash`, `/api/v1/avc/receipts`, `/api/v1/avc/protocol`, `/api/v1/avc/delegate`, `/api/v1/avc/revoke`, `/api/v1/avc/:id`, and `/api/v1/agents/:did/avcs` routes remain present.
2. Existing credential, delegation, revocation, policy reference, consent reference, and authority-chain validation semantics remain unchanged.
3. Existing legacy `AvcTrustReceipt` records without new optional fields remain deserializable.
4. Existing root trust bundle verification constants remain unchanged.
5. Existing file-based durable registry fallback remains available for non-production use.

## Observability

1. Startup logs identify AVC durability mode as Postgres-backed or non-Postgres-backed without printing secret values.
2. Signature verification failures surface through existing `AvcError::InvalidInput` reason text.
3. Tests prove forged receipt signatures do not increment `receipt_count`.
4. Tests prove action content commitment and receipt-chain fields are included in receipt ID/signing behavior when implemented.
5. Documentation identifies external timestamp and anchoring gaps without claiming fake capability.

Exit criterion: Functional, non-functional, compatibility, and observability requirements are explicit and testable.

# AVC Remediation Contract

| claim | scope | proof command or check |
| --- | --- | --- |
| Receipt signatures are verified on live storage. | `crates/exo-avc/src/registry.rs` | `cargo test -p exo-avc receipt -- --nocapture` |
| Forged receipt signatures are rejected without storage. | `crates/exo-avc/src/registry.rs` | Test named for forged receipt signature rejection. |
| Durable receipt records remain structurally validated before trust anchors load. | `crates/exo-avc/src/registry.rs` | Existing durable-state receipt tests plus new live-signature test. |
| AVC production durability requires Postgres configuration. | `crates/exo-node/src/main.rs`, deployment docs | Source assertion test or `rg -n "DATABASE_URL.*AVC|AVC.*DATABASE_URL" crates/exo-node deploy docs/guides`. |
| Trust anchors are not falsely claimed to be ordinary durable records. | `crates/exo-avc/src/registry.rs`, docs | `rg -n "trust anchors" crates/exo-avc/src/registry.rs deploy docs/guides`. |
| Action content commitment binds receipts to submitted action details. | `crates/exo-avc/src/receipt.rs`, `crates/exo-node/src/avc.rs` | Test proves changing action content changes receipt ID or action commitment. |
| Receipt ordering is tamper-evident within the implemented scope. | `crates/exo-avc/src/receipt.rs`, `crates/exo-avc/src/registry.rs` | Test proves second receipt records previous receipt hash and tampering breaks verification. |
| External timestamp and external anchor providers are not stubbed. | Repository-wide | `rg -n "RFC-3161|TimestampService|ExternalChain|TODO|stub|fake" crates docs` review finds no false implementation claim. |

Definitions: `AvcTrustReceipt` is the signed receipt struct in `crates/exo-avc/src/receipt.rs`; `AvcActionRequest` is the submitted action payload in `crates/exo-avc/src/validation.rs`; `AvcRegistryDurableState` is the persisted AVC runtime state in `crates/exo-avc/src/registry.rs`; `DATABASE_URL` is the existing Postgres configuration environment variable consumed by `crates/exo-node/src/main.rs`.

Exit criterion: Every claim used later has a scope and proof command or observable check.

# Implementation Slices

Sub-Agent Delegation Protocol: Each sub-agent receives an SOP-adapted brief with objective, allowed write scope, forbidden write scope, reuse check, no-stub rule, tests, acceptance criteria, and reporting format. The main orchestrator reviews all outputs before integration. Work that violates write boundaries, duplicates an existing system, fabricates capability, or lacks test evidence is rejected or returned for correction. Sub-agent standards are equivalent to local standards.

## Slice 1: Issue #696 Receipt Signature Verification

Goal: Verify AVC receipt signatures in live receipt storage and live durable-state application.

Allowed write scope: `crates/exo-avc/src/registry.rs`.

Requirements: Use `exo_core::crypto::verify`, `AvcTrustReceipt::signing_payload`, and existing registry public-key resolution.

Specification: Add signature verification to live receipt validation; preserve structural durable import checks before trust anchors load; add focused valid and forged receipt tests.

Test plan: Run `cargo test -p exo-avc receipt -- --nocapture`.

Exit criterion: A forged receipt signature fails closed and no receipt is stored.

## Slice 2: Issue #700 Durability Guardrails

Goal: Make non-Postgres AVC durability visible to operators and documented as non-production.

Allowed write scope: `crates/exo-node/src/avc.rs`, `crates/exo-node/src/main.rs`, `deploy/NODE-ZERO.md`, `deploy/RAILWAY-HANDOFF.md`, `docs/guides/production-deployment.md`.

Requirements: Reuse existing `DATABASE_URL`, `AvcRegistryDurability`, and deployment documentation patterns.

Specification: Add startup warning or existing-pattern guard for non-Postgres AVC durability; document production requirement and trust-anchor reload design.

Test plan: Run targeted node AVC startup/durability tests or source assertion tests discovered in `crates/exo-node/src/avc.rs` and `crates/exo-node/src/main.rs`.

Exit criterion: Production operators can discover that `DATABASE_URL` is required for durable AVC registry operation before deployment.

## Slice 3: Issues #697, #698, and #699 Architecture Map

Goal: Prevent schema/time/anchor implementation from guessing or overlapping with earlier slices.

Allowed write scope: none.

Requirements: Inspect existing receipt schema, BCTS chain, economy anchor pattern, DAG append path, timestamp source, and deployment docs.

Specification: Return exact implementation slices for content commitment, hash-linkage, and truthful timestamp provenance.

Test plan: Read-only file:line report reviewed by main orchestrator.

Exit criterion: The next write slices have exact non-overlapping file scopes and no fake provider dependency.

## Slice 4: Issue #699 Action Content Commitment

Goal: Bind emitted AVC receipts to the submitted action content without storing raw payloads.

Allowed write scope: To be finalized by Slice 3.

Requirements: Preserve legacy receipt deserialization and canonical signing.

Specification: Add optional content commitment derived from `AvcActionRequest` or a caller-supplied content hash if existing API shape supports it.

Test plan: Add tests proving action-field changes alter the signed receipt commitment and legacy receipts remain valid.

Exit criterion: A receipt can be cryptographically tied to the action details that produced it.

## Slice 5: Issue #697 Receipt Hash Linkage

Goal: Add tamper-evident ordering within an explicit receipt-chain scope.

Allowed write scope: To be finalized by Slice 3.

Requirements: Reuse existing hash-chain or anchor patterns where possible.

Specification: Add previous-receipt linkage or existing anchor commitment without claiming full external anchoring.

Test plan: Add tests proving chain linkage is recorded and tampering is detected.

Exit criterion: Receipts have an implemented ordering proof within the selected chain scope.

## Slice 6: Issue #698 Timestamp Provenance Boundary

Goal: Make timestamp trust level explicit and avoid legal-admissibility overclaiming.

Allowed write scope: To be finalized by Slice 3.

Requirements: Reuse `AvcReceiptTimestampSource` and existing Postgres clock path.

Specification: Record or document whether a receipt timestamp came from Postgres-backed runtime time or local HLC, and defer RFC-3161/blockchain anchoring until real provider configuration exists.

Test plan: Add tests proving emitted receipts use trusted node timestamp source rather than caller-supplied `validation.now`, and documentation grep proves no false external timestamp claim.

Exit criterion: The system no longer hides timestamp trust source or claims an unimplemented external timestamp provider.

Exit criterion: Every implementation slice has goal, write scope, requirements, specification, test plan, and observable completion condition.

# Test Plan

## Baseline Commands

1. `git status --short --branch`
2. `cargo test -p exo-avc receipt -- --nocapture`
3. `cargo test -p exo-node avc -- --nocapture`

## Per-Slice Commands

1. Slice 1: `cargo test -p exo-avc receipt -- --nocapture`
2. Slice 2: `cargo test -p exo-node avc -- --nocapture`
3. Slice 3: `rg -n "AvcTrustReceipt|AvcActionRequest|BctsTransition|EconomyRecordAnchor|AvcReceiptTimestampSource" crates/exo-avc crates/exo-node crates/exo-core`
4. Slice 4: Commands to be finalized after Slice 3.
5. Slice 5: Commands to be finalized after Slice 3.
6. Slice 6: Commands to be finalized after Slice 3.

## Final Agent-Runnable Sequence

1. `git status --short --branch`
2. `cargo fmt --check`
3. `cargo test -p exo-avc receipt -- --nocapture`
4. `cargo test -p exo-node avc -- --nocapture`
5. `rg -n "RFC-3161|TimestampService|ExternalChain|fake|stub|placeholder" crates/exo-avc crates/exo-node docs deploy`
6. `git diff --stat`

## Operator-Only Steps

1. Configure `DATABASE_URL` in production deployment.
2. Select real RFC-3161 and blockchain anchoring providers if external legal timestamping is required.
3. Close or comment on GitHub issues #696 through #700 after maintainer review.

Exit criterion: Baseline, per-slice, final, and operator-only checks are listed as runnable commands or explicit operator actions.

# Definition of Done

1. Issue #696 has a code path that verifies receipt signatures against validator public keys.
2. Issue #696 has a forged-signature regression test.
3. Valid receipts store successfully with registered validator keys.
4. Forged receipts do not increment registry receipt count.
5. Durable receipt structural checks still reject invalid IDs.
6. Durable receipt structural checks still reject empty signatures.
7. Durable receipt structural checks still reject unknown credentials.
8. Live durable-state application rejects forged receipt signatures when trust anchors are present.
9. Issue #700 startup warning or guard exists for non-Postgres AVC durability.
10. Deployment docs require `DATABASE_URL` for production AVC registry durability.
11. Deployment docs state trust anchors reload from verified startup configuration.
12. No documentation claims trust anchors are ordinary durable registry records.
13. Issue #699 receipt output contains an action content commitment or equivalent signed content binding.
14. Issue #699 tests prove action content affects the receipt binding.
15. Legacy receipts without new optional fields remain deserializable.
16. Issue #697 receipts contain implemented hash linkage or documented existing anchor commitment.
17. Issue #697 tests prove linkage detects tampering or ordering changes within scope.
18. Issue #698 does not add fake external timestamp providers.
19. Issue #698 records or documents the timestamp trust source for emitted receipts.
20. Existing AVC routes remain present.
21. `cargo fmt --check` passes.
22. Targeted `exo-avc` tests pass.
23. Targeted `exo-node` AVC tests pass or an external dependency blocker is documented with exact command output.
24. Git status contains only issue-scoped files.
25. The PR/commit message references GitHub issues #696 through #700.

Exit criterion: All listed Definition of Done items are independently checkable.

# Post-Implementation Review

When the Definition of Done is met and the checkpoint commit is created, a post-implementation review pass runs before push or deployment. The pass walks code review, test coverage, hardening, end-to-end verification, and documentation. Blockers found in the review are fixed and the relevant layers re-run before ship.

Review scope: Slices 1 through 6 and files under `crates/exo-avc/src/registry.rs`, `crates/exo-avc/src/receipt.rs`, `crates/exo-node/src/avc.rs`, `crates/exo-node/src/main.rs`, `deploy/NODE-ZERO.md`, `deploy/RAILWAY-HANDOFF.md`, `docs/guides/production-deployment.md`, and this plan file.
Review trigger: Definition of Done met and checkpoint commit created.
Review verdict gate: Ship | Fix blockers and re-run | Hand back to planning.

Completed review evidence recorded before PR:

- Code Review: Two reviewer passes and one silent-failure pass reported no remaining high-confidence blockers after scoped receipt-validator keys replaced generic public-key receipt verification.
- Test Coverage: `cargo +nightly llvm-cov -p exo-avc -p exo-node --branch --summary-only --json --output-path /tmp/exochain-avc-coverage-branch-final5.json --no-fail-fast` passed with `exo-avc` 194/194 and `exo-node` 1219/1219 tests passing. Compared with base coverage from `/tmp/exochain-avc-coverage-base.json`, total line coverage moved from 89.9613% to 90.0238%, total branch coverage moved from 67.6080% to 67.9745%, `crates/exo-avc/src/registry.rs` moved from 98.3535%/81.4286% to 98.3871%/81.5385%, `crates/exo-node/src/avc.rs` moved from 89.3953%/67.1053% to 90.0774%/67.7778%, and `crates/exo-node/src/main.rs` moved from 41.6981%/70.0000% to 43.1616%/70.0000%.
- Hardening: Receipt signatures verify with scoped validator keys only, durable receipt import rejects malformed chains and duplicate action commitments, idempotency conflicts fail closed, and `EXO_AVC_REQUIRE_POSTGRES_DURABILITY` can fail startup without `DATABASE_URL`.
- End-to-End Verification: Targeted `cargo test -p exo-avc registry -- --nocapture` passed 56 tests and targeted `cargo test -p exo-node avc -- --nocapture` passed 79 tests after final coverage tests.
- Documentation and Handoff: Production deployment docs and deploy handoffs now require Postgres-backed AVC durability for production and state that key trust anchors reload from verified startup configuration, not durable state.

Review verdict: Ship.

Exit criterion: The post-implementation review scope, trigger, and verdict gate are explicit and require review before push or deployment.
