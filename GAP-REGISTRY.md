# EXOCHAIN Gap Registry — Honest Audit

**Generated:** 2026-04-15
**Rule:** No gap closes until tests pass and the stub is deleted. Re-audit after each completion.

---

## Critical Gaps (Existential — product claims depend on these)

### GAP-001: DAG Persistence Layer
- **Status:** STUB
- **Location:** `crates/exo-node/src/reactor.rs:374` ("Phase 4"), `crates/exo-node/src/passport.rs:348,359` ("planned for Phase 4")
- **What's missing:** No durable storage backend for DAG events. Everything is in-memory. Process restart = data loss.
- **What breaks without it:** "Immutable ledger" claim is false. Evidence Bundles can't reference persisted events. Bailment contracts exist only in RAM.
- **Depends on:** Nothing — this is foundation.
- **Severity:** **CRITICAL**

### GAP-002: Evidence Bundle Export
- **Status:** NOT BUILT
- **Location:** Described in spec §9.4, §10, referenced in `exo-legal/src/evidence.rs` (single-event evidence, not bundle)
- **What's missing:** No packager that assembles a complete, self-contained, offline-verifiable bundle (event chain + state proofs + consent proofs + validator sigs + verification script).
- **What breaks without it:** The entire "forensic-grade" claim. GCs can't hand anything to a judge. The Board Book has no Evidence Bundle Reference that actually resolves.
- **Depends on:** GAP-001 (need persisted events to bundle)
- **Severity:** **CRITICAL**

### GAP-003: Multi-Model AI Consensus Engine
- **Status:** NOT BUILT
- **Location:** Frontend has `CouncilAIPanel.tsx` (UI shell only). No backend code calls any LLM.
- **What's missing:** The engine that convenes independent AI panels (Claude, GPT, Gemini, etc.), runs structured deliberation rounds, measures convergence, generates minority reports, Devil's Advocate.
- **What breaks without it:** decision.forum's entire differentiator. Without this, it's a form that produces PDFs, not a deliberation platform.
- **Depends on:** GAP-001 (deliberation records need persistence)
- **Severity:** **CRITICAL**

---

## High Gaps (Production-blocking — can't ship without these)

### GAP-004: Identity Verification (0dentity)
- **Status:** PARTIAL STUB
- **Location:** `crates/exo-node/src/zerodentity/` — onboarding UI exists, OTP challenge exists, but `store.rs:5` self-describes as "stub"
- **What's missing:** No real DID resolution against a registry. No WebAuthn. No proof-of-identity ceremony that a court would accept. Behavioral fingerprinting marked as "stubs" in `onboarding_ui.rs:397`.
- **What breaks without it:** Every DID on every bailment contract is an unverified string. Legal admissibility of identity claims is zero.
- **Depends on:** GAP-001 (identity records need persistence)
- **Severity:** **HIGH**

### GAP-005: Gateway Authentication & Authorization
- **Status:** PARTIAL
- **Location:** `crates/exo-gateway/src/server.rs:142` (TODO: conflict declarations), `graphql.rs:673` (placeholder caller DID), `graphql.rs:440` (deterministic stub for proof verification)
- **What's missing:** No RBAC mapping (GC vs board member vs observer). Proof verification is a fake one-liner. Conflict declaration check is a TODO.
- **What breaks without it:** Anyone with a JWT can see/modify anything. No tenant isolation at the HTTP layer.
- **Depends on:** GAP-004 (need real identity to authorize against)
- **Severity:** **HIGH**

### GAP-006: Custom Constraint Evaluation in Constitution
- **Status:** STUB
- **Location:** `crates/exo-governance/src/constitution.rs:271-274` ("Custom constraint evaluation not yet implemented")
- **What's missing:** Custom constitutional predicates can't be evaluated at runtime. Only built-in constraints work.
- **What breaks without it:** Per-tenant constitutional customization is aspirational. Every tenant gets the same invariants with no extension.
- **Depends on:** Nothing
- **Severity:** **HIGH**

### GAP-007: LiveSafe Integration Tests
- **Status:** PLACEHOLDER
- **Location:** `crates/exo-identity/tests/livesafe_integration.rs:13` ("TODO: Rewrite integration tests"), `crates/exo-gateway/src/livesafe.rs:1,176` ("resolver stubs", "mock data")
- **What's missing:** LiveSafe GraphQL resolvers return mock data. Integration tests are empty placeholders.
- **What breaks without it:** LiveSafe claims EXOCHAIN integration but nothing is actually wired.
- **Depends on:** GAP-001, GAP-005
- **Severity:** **HIGH**

---

## Medium Gaps (Quality/completeness — should fix before enterprise customers)

### GAP-008: Contract Clause Legal Review
- **Status:** NEEDS REVIEW
- **Location:** `crates/exo-consent/src/contract.rs` — all clause templates
- **What's missing:** No attorney has reviewed the bailment contract clause library. Templates are structurally correct but legally unvetted.
- **What breaks without it:** Contracts may not hold up in court despite correct architecture.
- **Depends on:** Andrew Sacks or equivalent attorney review
- **Severity:** **MEDIUM**

### GAP-009: Distributed HLC Sync
- **Status:** NOT BUILT
- **Location:** `crates/exo-core/src/hlc.rs` works single-node. No multi-party sync protocol.
- **What's missing:** When parties in different locations act on the same decision, causal ordering is local only.
- **What breaks without it:** Multi-party decisions may have ambiguous ordering under network partition.
- **Depends on:** GAP-001
- **Severity:** **MEDIUM**

### GAP-010: Tenant Isolation & Billing
- **Status:** MINIMAL
- **Location:** `crates/exo-tenant/` (482 LOC, thinnest crate)
- **What's missing:** No usage metering, no billing integration, no subscription management, no tenant data isolation at the storage layer.
- **What breaks without it:** Can't charge customers. Can't run multi-tenant SaaS.
- **Depends on:** GAP-001, GAP-005
- **Severity:** **MEDIUM**

### GAP-011: ExoForge Signal Collection & Onboarding API
- **Status:** PHASE 4/5 STUBS
- **Location:** `crates/exo-node/src/exoforge.rs:431,457` ("Phase 4: Signal Collection", "Phase 5: Onboarding API")
- **What's missing:** ExoForge integration points for automated signal collection and client onboarding are stubbed.
- **What breaks without it:** GAP engagement automation pipeline is manual.
- **Depends on:** GAP-001, GAP-005
- **Severity:** **MEDIUM**

---

## Foundation-First Build Order

```
Layer 0: PERSISTENCE (everything else is vapor without this)
  └─ GAP-001: DAG persistence layer

Layer 1: PROOF (can't prove anything without persisted data)
  ├─ GAP-002: Evidence Bundle export
  └─ GAP-004: Identity verification (0dentity)

Layer 2: INTELLIGENCE (the differentiator)
  └─ GAP-003: Multi-model AI consensus engine

Layer 3: ACCESS CONTROL (who can do what)
  ├─ GAP-005: Gateway auth & RBAC
  └─ GAP-006: Custom constitutional constraints

Layer 4: INTEGRATION (wire it all together)
  ├─ GAP-007: LiveSafe integration
  └─ GAP-011: ExoForge signals & onboarding

Layer 5: SCALE (enterprise readiness)
  ├─ GAP-008: Legal review of contract clauses
  ├─ GAP-009: Distributed HLC sync
  └─ GAP-010: Tenant isolation & billing
```

## Process

1. Start at Layer 0. Do not advance until all tests pass and all stubs in that layer are deleted.
2. After each layer completion, re-run the full stub audit (`grep -rn "TODO\|FIXME\|todo!\|unimplemented!\|stub\|placeholder\|Phase 4\|planned for"`) and update this registry.
3. New features are BLOCKED until the current layer is complete.
4. Every gap closure gets: ultraplan → tests first → implementation → verification → stub deletion → re-audit.

---

*This registry is itself a governed artifact. Updates require a commit with evidence of completion.*
