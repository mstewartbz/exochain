# EXOCHAIN Gap Registry — Honest Audit

**Generated:** 2026-04-15
**Last Updated:** 2026-04-15 21:15 EDT
**Rule:** No gap closes until tests pass and the stub is deleted. Re-audit after each completion.

---

## CLOSED Gaps

### ✅ GAP-001: DAG Persistence Layer — CLOSED `d6b9e2d`
- **Closed:** 2026-04-15
- **What was built:** `PostgresStore` (560 LOC, `postgres` feature flag) + `SqliteDagStore` (952 LOC, 16 tests) in `exo-node`. Reactor now restores persisted consensus state on startup. Phase 4 stubs removed from `reactor.rs` and `passport.rs`.

### ✅ GAP-002: Evidence Bundle Export — CLOSED `0685ded`
- **Closed:** 2026-04-15
- **What was built:** `crates/exo-legal/src/bundle.rs` — `EvidenceBundle` with `assemble()`, `verify()`, `render_json()`, `render_markdown_summary()`, `sign()`, `compute_bundle_hash()`. Offline-verifiable BLAKE3 hash chain. FRE 901/803(6)/902(13/14)/Daubert compliant. 16 tests, 126 total in exo-legal.

### ✅ GAP-003: Multi-Model AI Consensus Engine — CLOSED `789c324`
- **Closed:** 2026-04-15
- **What was built:** New crate `crates/exo-consensus/`. Commit-reveal scheme for model independence. Convergence scoring in basis points. `PanelConfidenceIndex`: model agreement (50%) + convergence speed (30%) + devil's advocate (20%). `MinorityReport` generation. `DeliberationResult` maps to `EvidenceBundle`. `MockLlmClient` for deterministic testing. LLM providers feature-gated (anthropic/openai/google). 14 tests.

### ✅ GAP-004: Identity Verification (0dentity) — CLOSED `7746c2a`
- **Closed:** 2026-04-15
- **What was built:** `crates/exo-identity/src/registry.rs` (`LocalDidRegistry`) + `crates/exo-identity/src/verification.rs` (`VerificationCeremony`, `IdentityProof`, `calculate_risk_score()`). Stub comments removed from `zerodentity/store.rs`. 12 tests.

### ✅ GAP-005: Gateway Authentication & Authorization — CLOSED `34ce160`
- **Closed:** 2026-04-15
- **What was built:** `Authenticator` struct using `LocalDidRegistry` in `exo-gateway/src/auth.rs`. `Role` enum (Admin, ExecutiveChair, BoardMember, Observer) + `Permission` enum + RBAC `has_permission()`. `verify_request()` validates JWT, resolves DID, enforces risk score threshold. Conflict declaration TODO replaced with real implementation.

### ✅ GAP-006: Custom Constraint Evaluation in Constitution — CLOSED `34ce160`
- **Closed:** 2026-04-15
- **What was built:** `CustomConstraintEvaluator` in `exo-governance/src/constitution.rs` — deterministic AST evaluator (`Expr::Eq`, `Expr::GreaterThan`, `Expr::Contains`) against `DeterministicMap` context. Stub at lines 271-274 removed. Tenant-specific governance rules now enforceable mathematically.

### ✅ GAP-007: LiveSafe Integration — CLOSED `8fb0a2a`
- **Closed:** 2026-04-15
- **What was built:** Production resolvers now require real `PgPool` — mock fallback is a compile error, not a silent return. 12 PACE integration tests rewritten against current API (PaceConfig, state transitions, escalation, Shamir split/reconstruct). Mock functions retained as `FOR TESTING ONLY`.

### ✅ GAP-011: ExoForge Signal Collection & Onboarding — CLOSED `8ec43c6`
- **Closed:** 2026-04-15
- **What was built:** Behavioral JS collector implemented (keystroke dynamics, mouse velocity, touch pressure, scroll histograms). Fingerprint collector expanded from 8 to all 15 `FingerprintSignal` variants (AudioContext, CanvasRendering, WebGL, WebRTC, FontEnumeration, BatteryStatus, + originals). ExoForge task registry updated to reflect actual completion. Phase 4/5 stubs removed.

---

## Open Gaps — Layer 5: Scale

### GAP-008: Contract Clause Legal Review
- **Status:** NEEDS ATTORNEY REVIEW
- **Location:** `crates/exo-consent/src/contract.rs` — all clause templates
- **What's missing:** No attorney has reviewed the bailment contract clause library. Architecture is correct, content is unvetted.
- **What breaks without it:** Contracts may not hold up in court.
- **Depends on:** Andrew Sacks or equivalent attorney review
- **Severity:** **MEDIUM** — not a code task

### GAP-009: Distributed HLC Sync
- **Status:** NOT BUILT
- **Location:** `crates/exo-core/src/hlc.rs` works single-node only
- **What's missing:** Multi-party HLC sync protocol for causal ordering across nodes in different locations.
- **What breaks without it:** Multi-party decisions may have ambiguous ordering under network partition.
- **Depends on:** GAP-001 ✅
- **Severity:** **MEDIUM**

### GAP-010: Tenant Isolation & Billing
- **Status:** MINIMAL
- **Location:** `crates/exo-tenant/` (482 LOC, thinnest crate)
- **What's missing:** Usage metering, billing integration, subscription management, tenant data isolation at storage layer.
- **What breaks without it:** Can't charge customers. Can't run multi-tenant SaaS.
- **Depends on:** GAP-001 ✅, GAP-005 ✅
- **Severity:** **MEDIUM**

---

## Additional Work Completed (Not in Original Registry)

### Bailment Contract Engine — `d036327`
- `crates/exo-consent/src/contract.rs` — clause composition, breach assessment, amendments, 16 tests

### Decision Forum GC Interface Ultraplan — `3523073`
- `gap/ULTRAPLAN-DECISION-FORUM-GC.md` — Board Book artifact spec, 4-phase implementation plan

### GAP Incubator Layer — `e301451` (merged PR)
- `gap/` directory — CEO onboarding, agentic teams, Syntaxis protocols, doctrine encoding, Decision Forum integration

### Railway Node 0 Deployment — `012dcab`
- `railway.json` — Dockerfile auto-detection, real health endpoint (`GET /health`)
- `deploy/NODE-ZERO.md` — Genesis runbook, exact CLI flags, multi-node join pattern

---

## Build Order Status

```
Layer 0: PERSISTENCE   ✅ GAP-001 CLOSED
Layer 1: PROOF         ✅ GAP-002 CLOSED  ✅ GAP-004 CLOSED
Layer 2: INTELLIGENCE  ✅ GAP-003 CLOSED
Layer 3: ACCESS        ✅ GAP-005 CLOSED  ✅ GAP-006 CLOSED
Layer 4: INTEGRATION   ✅ GAP-007 CLOSED  ✅ GAP-011 CLOSED
Layer 5: SCALE         ⏳ GAP-008 (attorney)  ⏳ GAP-009  ⏳ GAP-010
```

## Process

1. No gap closes until tests pass and all stubs in that layer are deleted.
2. After each layer completion, re-run stub audit and update this registry.
3. New features are BLOCKED until the current layer is complete.
4. Every gap closure gets: ultraplan → tests first → implementation → verification → stub deletion → re-audit.

---

*This registry is itself a governed artifact. Updates require a commit with evidence of completion.*
