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

# EXOCHAIN Systemic Integrity Ledger

**Generated:** 2026-07-02
**Amended:** 2026-07-02 — scout-sweep corrections, ratified decisions D1-D8, and
`v0.2.0-beta` release evidence folded in by the remediation coordinator.
**Authority:** single source of truth for critical EXOCHAIN gaps, priority, TDD
remediation entry points, closure gates, and execution progress.
**Source snapshot:** clean `origin/main` at `2d4baec1fc84bc5e71a9d5b9d1c35e7bff4aeee1`
(signed tag `v0.2.0-beta`, verified with GPG key
`96B889DAE73CD7C511CCDE28897119B7198789EC`)
**Runtime snapshot:** `https://exochain.io` (also
`https://exochain-production.up.railway.app`)
**Guard:** `bash tools/test_gap_registry_truth.sh`

## Single Source Rule

This file is the execution tool. Do not create parallel critical-gap registries,
date-stamped current registries, separate systemic-integrity plans, or proof-gap
tracking files. Supporting PRs, issues, and design notes may exist, but this file
owns priority, status, next test, closure gate, and current execution order.

If a worker discovers a new critical gap, add it here before remediation. If a
worker closes a gap, update this file in the same change set as the closing
tests and implementation. A gap is not closed by prose, intent, green CI alone,
or a healthy runtime endpoint alone.

## Execution Protocol

1. Select the lowest-numbered open P0. If no P0 remains, select the
   lowest-numbered open P1. P2 work waits unless it blocks a P0 or P1 closure.
2. Start from clean `origin/main` or the exact target PR branch under review.
3. Classify every touched path as EXOCHAIN core, core runtime adapter, adjacent
   surface, imported evidence, or third-party/vendor.
4. Write the red test or source guard named in the selected row before changing
   production code or claim text.
5. Make the smallest source change that turns the red test green.
6. Run the row's closure gate commands plus any touched-crate gates.
7. Update the row status, evidence, commit/PR, runtime proof when applicable, and
   claim boundary.
8. Re-run `bash tools/test_gap_registry_truth.sh`.

Multi-lane execution (2026-07-02): independent-crate lanes may run in parallel under a
single coordinator that owns this ledger, every `tools/test_*.sh` guard, and the merge
queue. Priority order P0 > P1 > P2 is preserved across lanes. Lane branches are named
`vcg/NNN-<slug>`. Every lane rebases on latest `origin/main` and re-runs its closure gate
before merge. Ledger row updates ship in the same change set as the closing tests and are
authored only by the coordinator; workers never edit this ledger or any guard script.

## Status Values

- **Open:** source truth shows the capability or claim is not complete.
- **Red:** failing test or guard exists and proves the gap.
- **Green-local:** implementation passes local closure gate, but CI/runtime or
  claim evidence is not complete.
- **Green-ci:** required CI is green on the relevant branch or main SHA.
- **Runtime-verified:** live or recorded-provider evidence proves the deployed
  boundary.
- **Closed:** source, tests, CI, runtime where applicable, docs, and this ledger
  all agree.
- **Blocked-external:** qualified external review or external account/action is
  required; the internal fail-closed boundary remains tested.

## Current Evidence Stack

Commands used to establish the current ledger:

```bash
git fetch origin main
git worktree add --detach /tmp/exochain-origin-main-gap-audit-20260702.Xi4QXP origin/main
git status --short
git rev-parse HEAD
git log -1 --oneline --decorate
rg -n "(unaudited|pedagogical|not production|fake|scaffold|placeholder|simulated|mock|dev-mode|not implemented|skeleton)" crates --glob '*.rs' --glob '!**/tests/**'
rg -n "unaudited-.*= \\[\\]|allow-simulated-tee|production-db" Cargo.toml crates/*/Cargo.toml .github/workflows/ci.yml
rg -n "SNARK|STARK|zkML|zero-knowledge|proof system|GraphQL|MCP|simulation|unaudited|first-touch|Holon|TEE|quote verifier|CrossChecked|HLC|tenant isolation|billing" README.md crates docs governance GAP-REGISTRY.md .github Cargo.toml
gh pr list --state open --json number,title,headRefName,baseRefName,isDraft,updatedAt,url --limit 100
gh issue list --state open --limit 100 --json number,title,labels,updatedAt,url
gh run list --branch main --limit 20 --json databaseId,workflowName,headSha,status,conclusion,createdAt,updatedAt,url
/usr/bin/curl -sS -m 15 -w '\nHTTP %{http_code}\n' https://exochain-production.up.railway.app/health
/usr/bin/curl -sS -m 15 -w '\nHTTP %{http_code}\n' https://exochain-production.up.railway.app/ready
/usr/bin/curl -sS -m 15 -w '\nHTTP %{http_code}\n' https://exochain-production.up.railway.app/health/db
```

Current external state at ledger creation, updated at amendment:

- `origin/main` is `2d4baec1` (merge of PR `#738`); PR `#738` is merged, no PRs open.
- Release `v0.2.0-beta` is published: release workflow run `28590796514` completed
  with conclusion success (34 jobs, 0 failures); signed tag verified; 64 release
  assets; npm `@exochain/exochain-wasm@0.2.0-beta` live with Sigstore provenance;
  all 31 crates in the signed tag's publish list return 200 on crates.io at
  `0.2.0-beta`, including `exochain-node`, `exochain-proofs`, and `exochain-wasm`.
- Crate packages publish under the `exochain-` prefix (crates.io namespace
  collision with an unrelated project's `exo-*` names); directories under
  `crates/` keep their `exo-*` names. Every cargo command in this ledger uses
  package names, not directory names.
- Open issues in the current integrity queue: `#734`, `#735`, `#736`, `#737`.
- Live runtime probes (2026-07-02, coordinator re-verified at amendment):
  - `https://exochain.io/ready`: HTTP 200, `status=ok`, `version=0.2.0-beta`,
    `dagdb_runtime_status=dagdb_active`, `dagdb_runtime_reason=db_probe_ok`.
  - `https://exochain-production.up.railway.app/health/db`: HTTP 200,
    `db=connected`, `status=ok`.

Amendment baseline (2026-07-02, local run on clean `origin/main` at `2d4baec1`):

- System Closure Gate cargo commands all exit 0: `build --workspace --release`,
  `test --workspace`, `clippy --workspace --all-targets -- -D warnings`,
  `fmt --all -- --check`, `doc --workspace --no-deps`, `audit`, `deny check`.
- The workspace itself is fully green. Every open VCG row concerns gated,
  unwired, or overclaimed surfaces, not broken builds - which is why generic
  gate commands alone (see VCG-004) can never stamp a closure.
- Guard evidence for this amendment: the rewritten
  `tools/test_gap_registry_truth.sh` passes against this ledger, fails against
  the prior committed registry (missing the ledger title), and the prior
  committed guard fails against this ledger (missing old-format markers). The
  guard rewrite therefore ships in the same change set as this ledger.
- Unaudited feature matrix (CI Gate 23) pre-verified locally: all eight
  `unaudited-*` features compile and pass tests in isolation
  (`exochain-node` x6, `exochain-gateway` graphql, `exochain-proofs`
  pedagogical), each `cargo test -p <pkg> --features <feature>` exit 0. The
  gated code is healthy; before Gate 23 it was simply never compiled in CI.

## Execution Board

| ID | Priority | Status | Classification | Owner role | Blocked claim | Next red test or guard | Closure gate |
|----|----------|--------|----------------|------------|---------------|------------------------|--------------|
| VCG-001 | P0 | Red | EXOCHAIN core | Proof architecture | Production SNARK/STARK/ZKML soundness | `crates/exo-proofs` production backend absence tests | `cargo test -p exochain-proofs` plus backend feature gates |
| VCG-002 | P0 | Green-local | Governance/docs | Claim integrity | Accurate proof and constitutional claims | `tools/check_systemic_integrity_claims.sh` | claim guard plus docs source scan |
| VCG-003 | P0 | Red | Core runtime adapter | Gateway | Production GraphQL governance/API execution | GraphQL no-fabrication resolver tests | `cargo test -p exochain-gateway graphql --features production-db` |
| VCG-004 | P0 | Open | Core runtime adapter | MCP runtime | MCP tools as constitutional runtime actions | MCP mutation-effect and CGR verifier tests | `cargo test -p exochain-node mcp` |
| VCG-005 | P1 | Open | Core runtime adapter | Governance runtime | Complete validator-set lifecycle | proposal-vote-commit application tests | `cargo test -p exochain-node governance` |
| VCG-006 | P1 | Open | Core runtime adapter | AVC runtime | Civilizational-class AVC closure | issues `#734`-`#737` regression tests | node/gateway AVC tests plus runtime probes |
| VCG-007 | P1 | Open | Adjacent adapter | CrossChecked boundary | Verified CrossChecked provenance | external receipt authority proof tests | `cargo test -p exochain-node crosschecked` |
| VCG-008 | P1 | Open | Core runtime adapter | 0dentity | Public first-touch claims | proof-of-possession negative tests | `cargo test -p exochain-node zerodentity` |
| VCG-009 | P1 | Open | Core runtime adapter | 0dentity | Device/behavior trust inputs | consented sample ingestion tests | `cargo test -p exochain-node zerodentity` |
| VCG-010 | P1 | Open | Core runtime adapter | Holon runtime | Holons as trusted actors | signed authority/provenance tests | `cargo test -p exochain-node holon` |
| VCG-011 | P1 | Open | EXOCHAIN core | TEE integration | Hardware TEE attestation | platform quote verifier tests | `cargo test -p exochain-gatekeeper tee` |
| VCG-012 | P2 | Open | EXOCHAIN core | Distributed time | Multi-node causal finality | multi-node HLC partition tests | `cargo test -p exochain-core hlc` |
| VCG-013 | P2 | Open | EXOCHAIN core | Tenant platform | SaaS tenant ops and billing | tenant metering and billing export tests | `cargo test -p exochain-tenant` |
| VCG-014 | P2 | Open | Governance/legal | Council/legal | Constitutional completeness | unresolved Sybil/no-admin traceability guard | governance guard plus legal sign-off record |

## VCG-001 - Production ZK Proof Backend Absent

**Priority:** P0
**Status:** Red
**Classification:** EXOCHAIN core
**Owner role:** Proof architecture

Lane record (2026-07-02, branch `vcg/001a-proof-envelope`):

- Red evidence: `tests/envelope.rs` (documented compile-red, then behavioral)
  and the ignored standing red
  `production_backend_variant_executes_without_unaudited_flag`, independently
  verified to fail for the documented reasons (red commit `4f4c4b66`; hardened
  `d64184a4`).
- Green-local for the envelope remediation item: statement registry +
  `ProofEnvelope` with canonical CBOR; `verify()` fails closed for every
  backend (no success path exists in the match); negative fixtures for
  truncated CBOR, spliced unknown statement-kind, and garbage bytes; gates
  exit 0 in both feature configurations plus clippy `-D warnings`.
- Adversarial review: first pass refuted (major - success-shaped `verify()`,
  missing fixtures, inert standing red); hardening applied; re-refutation
  verdict NOT-REFUTED.
- Forward coupling for VCG-001b: `default_registry()`/`AuditStatus` exist so
  the standing red asserts against reality. VCG-001b must add a genuine
  `BackendId` variant with a wired verifier; flipping an `AuditStatus` tag
  cannot turn the standing red green because `verify()` has no success arm
  for the pedagogical id.
- Row stays Red, not Green: this lane delivers the envelope/registry
  remediation item only; production backend soundness (D1: RISC Zero,
  server-side) and external cryptographic review remain open.

Evidence:

- `crates/exo-proofs/src/lib.rs:17-24` states that the crate is unaudited,
  pedagogical, and not production cryptography.
- `crates/exo-proofs/src/lib.rs:26-31` says every public entry point refuses
  unless `unaudited-pedagogical-proofs` is enabled.
- `crates/exo-proofs/src/lib.rs:38-42` labels SNARK, STARK, and ZKML modules as
  skeletons.
- `crates/exo-proofs/src/lib.rs:68-77` implements the default refusal guard.
- `crates/exo-proofs/Cargo.toml:35-43` says the feature only unlocks tests,
  demos, and classroom use and must not be enabled in production.
- `.github/workflows/ci.yml:96-104` excludes `exo-proofs` from the default
  tarpaulin pass because the default build compiles to refusal paths.

Failure mode:

Any unqualified claim that EXOCHAIN has production SNARK, STARK, or ZKML
soundness is false until a real backend and review path are in place.

Next red test:

- Add tests proving production proof calls refuse when no production backend is
  registered.
- Add proof-envelope tests binding statement kind, backend id, version, public
  inputs, commitment roots, verifier key or image id, and domain separator.
- Add negative fixtures for tampered proof bytes, wrong public inputs, wrong key,
  malformed envelope, and disabled backend.

Remediation track:

- Define a versioned proof statement registry covering governance compliance,
  DAG inclusion, execution receipt, model inference, and compatibility-only
  pedagogical proofs.
- Wire one production backend slice end to end before expanding proof claims.
- Treat RISC Zero receipts as execution proofs and RISC Zero Groth16 wrapping as
  receipt compression, not a generic replacement for every SNARK claim.
- Select custom SNARK, STARK, and ZKML backends per statement type after the
  statement registry exists.
- Ratified decision D1 (2026-07-02): RISC Zero is the selected backend family,
  with Groth16 wrapping as receipt compression; proving is server-side only (no
  browser/WASM prover). Verifier minimalism applies: the verifier stays small,
  in-workspace, and pinned, and carries the external audit budget; the proving
  toolchain is vendored, pinned, and enters the `cargo deny` perimeter as a
  reviewed dependency authority.

Scout corrections (2026-07-02):

- No ZK backend dependency (risc0, arkworks, halo2, plonky, winterfell) exists
  anywhere in the workspace; adding one is a supply-chain event.
- `crates/exo-proofs/src/circuit.rs` (R1CS) is sound structural shape and
  stays; `snark.rs`, `stark.rs`, and `zkml.rs` internals are replaced, never
  patched.
- `exochain-wasm` declares but never uses `exo-proofs` (cargo-machete ignore);
  a superficial call site there is not integration evidence.
- Non-closure boundary: cosmetic API reshaping (an envelope around the existing
  blake3 stand-ins) is explicitly not closure for this row.

Closure gate:

```bash
cargo test -p exochain-proofs
cargo test -p exochain-proofs --features unaudited-pedagogical-proofs
cargo clippy -p exochain-proofs --all-targets -- -D warnings
cargo audit
cargo deny check
```

Closure requires external cryptographic review before public or governance
claims are upgraded.

## VCG-002 - Proof and Constitutional Claim Drift

**Priority:** P0
**Status:** Green-local
**Classification:** Governance/docs
**Owner role:** Claim integrity

Lane record (2026-07-02, branch `vcg/002-claim-integrity`):

- Red evidence: the coordinator-authored guard
  `tools/check_systemic_integrity_claims.sh` failed against the pre-lane tree
  at `README.md:214` (red commit `affa16b8`); an independent verifier
  reproduced both full-extent candidate scans exactly.
- Guard evolution, coordinator-authored and hash-pinned throughout: v1
  `781e45c5...3722056` (five claim files) to v2 `dd9d2863...b00896d3`
  (adds THREAT-MODEL, COUNCIL-ASSESSMENT, SYSTEM-DOCUMENTATION; a
  maturity-language scan for 'cryptographic level' and 'formal proofs'; the
  `--packages` variant) to v3
  `bff75fef3804e6ba056800bd1204e8daf904234036daa2eb68b9a3f8d8a6ae1f`
  (adds ARCHITECTURE, USER-MANUAL, developer-onboarding). Each version was
  proven red against the then-current tree before workers made it pass
  honestly.
- Green-local evidence: eleven claim files rewritten downward-only (the
  flagship ASI-safety section split into delivered type/runtime enforcement
  vs roadmap cryptographic enforcement; THREAT-MODEL Threat 6 downgraded from
  live control to design intent; the five-tuple flagship invariant landed);
  living command surfaces moved to `exochain-*` package names; the guard is
  wired into the ci.yml hygiene job; both guards exit 0 (commits `a40f9980`,
  `e9fe496a`, `45d9b425`).
- Adversarial review: first green refuted (major, twice - untouched flagship
  claim; THREAT-MODEL live-control overstatement; repo-wide gate residue;
  `--packages` regex gap); two hardening passes; final fresh-eyes
  re-refutation across five lenses returned NOT-REFUTED with sweep residue
  zero outside exempt records.
- Residual recorded, outside this row's charter: stale `-p exo-*` and
  `--packages exo-*` command examples remain in living operational docs the
  guard's LIVING_SURFACES list does not cover (docs/dagdb/*,
  docs/avc/root-trust-install-intake.md,
  docs/grant/CODEX-CYBERSECURITY-GRANT-CLAIMS.md) and in ratified resolutions
  CR-003/CR-004 (records: annotate, never rewrite). Command-accuracy
  follow-up, not proof-maturity drift.
- Status Green-local: local closure gate passes; Green-ci on lane PR CI;
  Closed after merge when source, tests, CI, docs, and this ledger agree.

Evidence:

- `README.md:214` describes `exo-proofs` as SNARK, STARK, and ZKML proof
  systems.
- `governance/sub_agents.md:61` says SNARK, STARK, ZKML proof systems and
  verifier infrastructure are complete.
- `governance/traceability_matrix.md:115-118` marks SNARK, STARK, ZKML, and
  unified verifier rows green.
- `docs/reference/CRATE-REFERENCE.md:355-388` describes `exo-proofs` as a
  zero-knowledge proof system with proof generation and verification APIs.
- `docs/ASI-REPORT-FEATURE.md:180-216` overstates proof-backed constitutional
  guarantees.
- Scout sweep (2026-07-02) found additional unqualified claim sites beyond the
  five above: `README.md:118,140`;
  `docs/reference/CRATE-REFERENCE.md:419-421,1172,1189`;
  `docs/ASI-REPORT-FEATURE.md:48,81,208,216,235,243`; plus stale numeric claims
  (57 tests cited vs 118 actual behind the feature flag).
- Honest qualifying language already exists in
  `docs/council/PANEL-2-LEGAL.md:217`, `PANEL-3-ARCHITECTURE.md:77-90`,
  `PANEL-5-OPERATIONS.md:328,847`, and `docs/council/OPTIMIZED-SPEC.md:206`;
  the fix propagates that language outward.

Failure mode:

Public, governance, and onboarding materials can cause agents and stakeholders
to plan from false proof maturity.

Next red test:

- Add `tools/check_systemic_integrity_claims.sh` or extend
  `tools/test_gap_registry_truth.sh` so it fails on unqualified proof completion
  claims while VCG-001 is open.

Remediation track:

- Rewrite proof claims to distinguish structural API shape, fail-closed refusal
  behavior, and production cryptographic readiness.
- Update traceability rows so proof entries cannot appear complete until VCG-001
  has closure evidence.
- Ratified claim architecture (2026-07-02): public claims state that EXOCHAIN
  makes power constitutional, not models aligned - the safety property is the
  governed channel, not the mind. Safety formulations are rewritten as
  invariant five-tuples: invariant, adversary, evidence, detection, failure
  mode. The socio-technical scope condition (the ledger governs what routes
  through it) is stated openly.
- Package rename hygiene: fix `-p exo-*` cargo commands in living surfaces
  (docs/guides, docs/reference, governance matrices); the claim guard forbids
  old package names outside allowlisted historical records (docs/audit,
  docs/proof, dated validation reports), which are never rewritten.
- Disambiguate protocol formal proofs (`docs/proofs/CONSTITUTIONAL-PROOFS.md`)
  from SNARK/STARK/ZKML cryptography so one fix does not cast doubt on the
  other.

Closure gate:

```bash
bash tools/test_gap_registry_truth.sh
bash tools/check_systemic_integrity_claims.sh
rg -n "SNARK|STARK|ZKML|zero-knowledge|formal proofs|cryptographic level" README.md docs governance
cargo fmt --all -- --check
```

## VCG-003 - GraphQL Surface Is Default-Off and Unaudited

**Priority:** P0
**Status:** Red
**Classification:** Core runtime adapter
**Owner role:** Gateway

Lane record (2026-07-02, branch `vcg/003-graphql-actor-context`):

- Premise correction discovered in RED: the nine hardcoded actor literals were
  dead code behind an unconditional mutation kill-switch
  (`guard_graphql_mutation_execution` refused every mutation regardless of the
  feature flag). Red tests were adapted honestly: refusals must carry a
  dedicated `missing_authenticated_actor` code, and injecting a real actor
  must be provably observable (red commit `1ebc48ac`, independently verified).
- First green (`58ac5b11`) REFUTED, two criticals: it removed the kill-switch
  (unscoped security-posture change self-authorized by the lane's own test),
  `cast_vote` fabricated placeholder decisions for nonexistent ids, and
  `advance_decision` carried a decorative actor check while caller-controlled
  `reason` became the audit actor.
- Corrective (`59ea1604`): kill-switch restored as the final unconditional
  gate, sequenced `guard_graphql_execution` then `require_authenticated_actor`
  then refusal, so error types are honest per configuration; fabrication
  reverted to fail-closed not-found; `advance_decision` binds the checked
  actor DID and `reason` never flows into identity; meta-test strengthened to
  count the refusal call in all nine resolvers. Re-refutation across six
  lenses: NOT-REFUTED. Gates: feature-off 18/18, feature-on 31/31 plus one
  ignored, clippy both configs `-D warnings` clean.
- Delivered: genuine per-request `AuthenticatedActor` middleware (real Ed25519
  verification via `auth::authenticate`), all nine literals eliminated,
  `evaluateConsent` routed through the deny-by-default kernel path, standing
  red `mutations_execute_with_actor_after_adjudication_wiring` (ignored) that
  fails at the kill-switch.
- Independent convergence: a parallel corrective derivation reached the same
  unique guard ordering, recorded as corroborating evidence.
- Residuals for the follow-on adjudication lane: (a) the standing red is the
  only tripwire against removing the kill-switch without real core-backed
  wiring - a PR deleting `refuse_graphql_mutation_execution` without
  un-ignoring and passing that test is suspect by construction; (b)
  `revoke_delegation` and `amend_constitution` discard the checked actor
  because those resolvers have no audit call yet - identity binding must be
  completed there when mutations actually execute.
- Row stays Red: actor plumbing and no-fabrication are delivered; the row's
  full goal (every resolver routed through core-backed consent, authority,
  and proof services, with mutations executing) remains open, gated on the
  VCG-001 envelope path and core adjudication wiring.

Evidence:

- `crates/exo-gateway/Cargo.toml:97-102` says GraphQL includes unaudited
  governance mutations, fabricated consent evaluation, and proof-verification
  scaffolding.
- `crates/exo-gateway/src/graphql.rs:504-514` names GraphQL consent fabrication
  and proof verifier initiatives.
- `crates/exo-gateway/src/graphql.rs:1328-1331` says GraphQL is refused by
  default to avoid placeholder caller identity, fabricated consent, proof
  scaffolding, and unauthenticated playground HTML.
- `crates/exo-gateway/src/graphql.rs:2089-2096` verifies that proof query output
  reports proof storage and verification as unwired.

Failure mode:

If enabled without full wiring, GraphQL can present governance, consent, or proof
truth that did not come from the core runtime.

Next red test:

- GraphQL resolver tests proving caller identity, consent, authority, proof
  validity, and playground access cannot be fabricated, including with
  `unaudited-gateway-graphql-api` enabled.

Remediation track:

- Route every resolver through authenticated actor context and the same
  core-backed consent, authority, and proof services used by REST.
- Proof queries call the VCG-001 verifier or return a typed production refusal.

Scout corrections (2026-07-02):

- `verifyProof` and `evaluateConsent` already fail closed; the open fabrication
  surface is nine hardcoded actor literals (`did:exo:caller`, `system`) at
  `graphql.rs:822,904,909,952,969,1029,1094,1120,1128`.
- Reuse targets: `exo_gateway::auth::{authenticate, AuthenticatedActor}` and
  `exo_gatekeeper::kernel` via the `build_adjudication_context` pattern at
  `server.rs:812-852`; per-request actor injection into the schema is new
  plumbing in `server.rs`.
- The in-file `include_str!` meta-tests assert structural split boundaries;
  refactors must preserve them.

Closure gate:

```bash
cargo test -p exochain-gateway graphql --features production-db
cargo test -p exochain-gateway graphql --features "production-db unaudited-gateway-graphql-api"
cargo clippy -p exochain-gateway --features production-db --all-targets -- -D warnings
```

## VCG-004 - MCP Runtime Actions and CGR Proof Verification Are Not Fully Wired

**Priority:** P0
**Status:** Open
**Classification:** Core runtime adapter
**Owner role:** MCP runtime

Evidence:

- `crates/exo-node/Cargo.toml:189-211` says several MCP tools return
  truth-shaped JSON without persistence, reactor invocation, DAG append,
  messaging, or consent/escalation state, and are gated behind
  `unaudited-mcp-simulation-tools`.
- `crates/exo-node/src/mcp/tools/proofs.rs:724-786` defines
  `exochain_verify_cgr_proof` as fail-closed because proof bytes, public inputs,
  checkpoint root, validator signatures, and a production verifier are not
  wired.

Failure mode:

Agents can mistake no-effect tool output for constitutional runtime effects.

Next red test:

- Source guard and unit tests fail when a state-changing MCP tool returns
  success-shaped output without a live persistence, reactor, DAG, message,
  consent, or escalation effect.
- CGR proof tests fail until proof bytes, public inputs, checkpoint root,
  validator signatures, and production verifier are consumed.

Remediation track:

- Reclassify read-only helpers as read-only and route mutations through governed
  runtime paths.
- Use the VCG-001 proof envelope for CGR proof verification.
- Ratified decision D2 (2026-07-02): `exochain mcp` remains a standalone
  process connected to the node over an authenticated, read-scoped RPC bridge -
  adjudicator and adjudicated never share a process boundary. Any interim
  node-attached mode runs under a named capability profile with the bridge as
  the committed end state.

Scout corrections (2026-07-02):

- The standalone `exochain mcp` command never populates `reactor_state`,
  `store`, or a network handle, and `NodeContext` has no `net_handle` field -
  the bridge plus context extension is the core of this row, ahead of per-tool
  handler edits.
- Eight read-only tools (authority-chain verification, permission checks,
  ledger reads, Merkle proof computation, threat evaluation, node status) are
  already real and must not be reclassified as simulation; messaging tools fail
  closed unconditionally and keep that posture.
- Real mutation paths to route through: `reactor::submit_proposal`
  (`reactor.rs:1275`), `broadcast_governance_event` (`reactor.rs:1399`),
  `SqliteDagStore` persistence primitives, and the hardened zerodentity store.

Closure gate:

The generic commands below passed on untouched `origin/main` at amendment time
(416 and 428 tests green, clippy clean), so they are necessary but not
sufficient. Closure additionally requires the named red tests from this row to
exist and pass; a green run of the generic commands alone is not closure
evidence.

```bash
cargo test -p exochain-node mcp
cargo test -p exochain-node mcp --features unaudited-mcp-simulation-tools
cargo test -p exochain-node mcp_mutation_effect
cargo test -p exochain-node cgr_proof_fail_closed
cargo clippy -p exochain-node --all-targets -- -D warnings
```

## VCG-005 - Admin Governance Shortcut Is Proposal-Only

**Priority:** P1
**Status:** Open
**Classification:** Core runtime adapter
**Owner role:** Governance runtime

Evidence:

- `crates/exo-node/Cargo.toml:164-175` says the route submits a canonical
  `ValidatorSetChange` proposal, remains default-off, and is not a complete
  governance lifecycle.
- `governance/resolutions/CR-001-AEGIS-SYBIL-AUTHENTIC-PLURALITY.md:218`
  records no-admin preservation as partial.
- GitHub issue `#737` (scoping `/receipts/emit` authorization away from an
  admin bearer-token pattern) is an AVC surface tracked under VCG-006; it
  shares the admin-bearer-authority theme with this row but is not this row's
  evidence (scout correction, 2026-07-02).

Failure mode:

HTTP admin/proposal surfaces may be treated as complete governance when they are
only proposal entry points.

Next red test:

- Proposal-vote-commit tests proving validator-set changes cannot apply without
  quorum, authority chain, persistence, and audit receipt.

Remediation track:

- Complete validator-set lifecycle from HTTP proposal through quorum vote,
  commit application, persistence, and audit receipt.
- Remove admin bearer-token authority from effects that require subject or
  quorum authority.

Scout corrections (2026-07-02): full BFT propose/vote/commit machinery exists
(`exo-dag` consensus via `propose_verified`/`vote_verified`/`check_and_commit`),
and `store.save_validator_set` (`store.rs:1543`) is loaded at startup but never
called after a commit. The fix point is decoding the committed payload in
`handle_commit`/`check_and_commit` (`reactor.rs`) and applying a
`ValidatorSetChange` to live consensus config plus persistence. The
minimum-validator BFT floor is checked at proposal time only; concurrent remove
proposals that individually pass but jointly drop below the floor are an open
design point the red tests must cover.

Closure gate:

```bash
cargo test -p exochain-node governance
cargo test -p exochain-node avc
cargo clippy -p exochain-node --all-targets -- -D warnings
```

## VCG-006 - AVC Runtime Integrity Issues Remain Open

**Priority:** P1
**Status:** Open
**Classification:** Core runtime adapter
**Owner role:** AVC runtime

Evidence:

- Live `/ready` returned HTTP 200 with `dagdb_runtime_status=dagdb_active`.
- Live `/health/db` returned HTTP 200 with `db=connected`.
- `docs/guides/production-deployment.md:109-113` says local AVC file fallback is
  not a substitute for Postgres-backed production durability and recommends
  `EXO_AVC_REQUIRE_POSTGRES_DURABILITY=true`.
- GitHub issue `#734` tracks conformance-test root support.
- GitHub issue `#735` tracks making the AVC registry durably Postgres-backed by
  default.
- GitHub issue `#736` tracks runtime issuer allow-list registration/rotation.
- GitHub issue `#737` tracks subject-signature scoping for `/receipts/emit`.

Failure mode:

Live health is green, but civilizational-class AVC claims remain incomplete
while durability, issuer rotation, auth scoping, and conformance root issues are
open.

Next red test:

- `#735`: production startup fails when required Postgres durability is absent.
- `#737`: `/receipts/emit` rejects admin bearer-token-only authority where
  subject signature is required.
- `#736`: issuer allow-list registration/rotation cannot require manual gateway
  redeploy.
- `#734`: conformance roots cannot weaken production root trust.

Remediation track:

- Close the four AVC issues as one integrity lane with source, CI, and runtime
  proof recorded here - as four closure sub-records (`#734`, `#735`, `#736`,
  `#737`), each with its own red-test evidence. The row does not close on a
  subset.

Scout corrections (2026-07-02):

- `#735`: `avc_require_postgres_durability_from_env()` is called at
  `main.rs:514` and its result is discarded - the flag is structurally inert
  today. The fix threads the parsed requirement into
  `AvcApiState::with_durable_registry` so startup fails closed.
- `#737`: subject-signature verification is fully implemented and invoked in
  `handle_emit_receipt`; the missing piece is a router-middleware carve-out
  (mirror `is_zerodentity_local_signed_write`). Red tests must exercise the
  full router with `require_bearer_on_writes` layered on - direct handler
  calls prove nothing about the wired stack.
- `#736`: no issuer registration surface exists; persistence today is a
  single-row CBOR blob, so per-issuer shape is new design. Registration
  authority follows the D3 one-authority-model rule (DelegationRegistry).
- `#734`: guard that production `AVC_ROOT_TRUST_*` constants compile
  identically with the conformance feature off.
- `cargo test -p exochain-gateway avc --features production-db` currently only
  exercises route-discovery strings and migrations; behavioral AVC coverage
  lives in `exochain-node`.

Closure gate:

```bash
cargo test -p exochain-node avc
cargo test -p exochain-gateway avc --features production-db
cargo clippy -p exochain-node --all-targets -- -D warnings
cargo clippy -p exochain-gateway --features production-db --all-targets -- -D warnings
/usr/bin/curl -sS https://exochain-production.up.railway.app/ready
/usr/bin/curl -sS https://exochain-production.up.railway.app/health/db
```

## VCG-007 - CrossChecked Receipt Anchor Is Unaudited

**Priority:** P1
**Status:** Open
**Classification:** Adjacent adapter
**Owner role:** CrossChecked boundary

Evidence:

- `crates/exo-node/Cargo.toml:177-187` says the CrossChecked route mints a
  node-signed TrustReceipt from external metadata but lacks a trusted authority
  resolver, proof fetcher, and tenant/workspace authorization contract.
- Scout correction (2026-07-02): the route (`api.rs:605-626`) is an
  unconditional 403 refusal with no minting path in any feature configuration -
  commit `c730a0e4` removed enabled-path minting. The feature name appears only
  in the error payload and test attributes, never as a compile-time branch. The
  gap is a missing intake implementation, not a partially-guarded mint.

Failure mode:

External metadata can be mistaken for verified EXOCHAIN provenance.

Next red test:

- CrossChecked anchor rejects external metadata without trusted authority
  resolution, proof fetch, tenant/workspace auth, and signature verification.

Remediation track:

- Add adjacent-surface intake for the CrossChecked adapter.
- Validate external receipt proof and authority before minting any EXOCHAIN
  receipt.
- Ratified decision D3 (2026-07-02): trusted-authority resolution reuses
  `exo-authority` (`AuthorityChain`, `DelegationRegistry`) and
  `exo-identity::LocalDidRegistry` - both already dependencies of
  `exochain-node`, both unimported by this route today. Registry entries are
  the single authority species for humans, models, and external verifiers, so
  a CrossChecked authority is a delegated verification seat under the same
  law.
- `exo-tenant` is not an `exochain-node` dependency and has no workspace
  concept; the tenant/workspace authorization scope for this route is decided
  explicitly, not silently invented.

Closure gate:

```bash
cargo test -p exochain-node crosschecked
cargo clippy -p exochain-node --all-targets -- -D warnings
```

## VCG-008 - 0dentity First-Touch Onboarding Is Gated

**Priority:** P1
**Status:** Open
**Classification:** Core runtime adapter
**Owner role:** 0dentity

Evidence:

- `crates/exo-node/Cargo.toml:213-230` says first-touch claim creation needs an
  approved proof-of-possession contract before exposure.
- `crates/exo-node/src/zerodentity/onboarding.rs:374-388` returns a forbidden
  refusal until the proof-of-possession design lands.

Failure mode:

A bearer token alone could be confused with cryptographic proof that a caller
controls the claimed DID.

Next red test:

- First-touch claim creation with only a node write bearer token cannot create a
  pending claim for an arbitrary DID.
- DID derivation mismatch, unsigned bootstrap payload, replayed OTP session, and
  wrong public key all fail.

Remediation track:

- Bind DID, bootstrap public key, signed bootstrap payload, OTP session, claim
  record, and HLC metadata.

Scout corrections (2026-07-02): a complete proof-of-possession contract already
exists behind the feature flag and passes 34 tests (domain-separated canonical
CBOR signing, DID derivation from the public key with mismatch rejection,
Ed25519 verification, zero-signature rejection, exact-payload replay dedup).
The genuinely open pieces are narrower than this row's original framing:

- `created_ms` has no freshness/skew window against the trusted HLC - an
  arbitrarily old signed payload is accepted when byte-identical replay dedup
  does not catch it. Red test: stale `created_ms` rejection outside a bounded
  window (mirror `ZERODENTITY_ERASURE_MAX_FUTURE_SKEW_MS`).
- No test exercises the full wired stack (onboarding router merged behind
  `auth::require_bearer_on_writes`); every existing test builds the router in
  isolation. Red test: a bearer-only request with no proof-of-possession is
  rejected through the production middleware composition.
- Enabling the feature default is a ratification event with named review
  evidence (D8), never a test outcome.

Closure gate:

```bash
cargo test -p exochain-node zerodentity
cargo test -p exochain-node zerodentity --features unaudited-zerodentity-first-touch-onboarding
cargo clippy -p exochain-node --all-targets -- -D warnings
```

## VCG-009 - 0dentity Device and Behavioral Axes Are Unwired

**Priority:** P1
**Status:** Open
**Classification:** Core runtime adapter
**Owner role:** 0dentity

Evidence:

- `crates/exo-node/Cargo.toml:243-252` says deterministic helpers exist but the
  public ingestion path does not persist client-collected samples.
- `crates/exo-node/src/zerodentity/mod.rs:35-44` says `device_trust` and
  `behavioral_signature` are disabled by default.
- `crates/exo-node/src/zerodentity/api.rs:516-533` returns a forbidden refusal
  for the unaudited device/behavioral surface.
- `docs/0DENTITY-APP-SPEC.md:31-35` describes complete collection that current
  runtime gates do not expose.

Failure mode:

Device and behavior signals may be treated as production trust inputs even
though ingestion and persistence are not wired.

Next red test:

- Device and behavioral sample fields are rejected unless consent-scoped,
  privacy-reviewed, persisted, replay-safe, and scored from stored evidence.

Remediation track:

- Implement bounded sample ingestion and scoring through stored, consented
  evidence.
- Rewrite docs to match the consent and privacy boundary.

Scout corrections (2026-07-02): this row understates the gap. No ingestion
producer exists at all (`put_fingerprint`/`put_behavioral` at
`store.rs:1036,1053` have zero HTTP call sites), `SubmitClaimRequest` lacks the
spec section 7.1 sample fields, the client-side collection layer from
`docs/0DENTITY-APP-SPEC.md` section 3 is entirely unimplemented, and
`exo-consent` is declared but never referenced in the zerodentity module -
consent scoping is new integration, not plumbing. The scoring engine
(`scoring.rs:181-187`) already consumes stored samples once they exist. Same
worker as VCG-008, sequential (shared request struct, store, and test harness).

Closure gate:

```bash
cargo test -p exochain-node zerodentity
cargo test -p exochain-node zerodentity --features unaudited-zerodentity-device-behavioral-axes
cargo clippy -p exochain-node --all-targets -- -D warnings
```

## VCG-010 - Infrastructure Holons Are Gated

**Priority:** P1
**Status:** Open
**Classification:** Core runtime adapter
**Owner role:** Holon runtime

Evidence:

- `crates/exo-node/Cargo.toml:232-241` says infrastructure Holons use sentinel
  authority/provenance signatures and no public key.
- `crates/exo-node/src/holons.rs:34-39` says the runtime background manager is
  disabled by default behind `unaudited-infrastructure-holons`.
- Scout correction (2026-07-02): the sentinel-signature claim is stale - real
  Ed25519 authority/provenance signing is wired (`holons.rs:358-472`) with
  production configuration at `main.rs:783-798`. The live residual gaps are:
  (a) the trust check is tautological - `config.root_public_key` is trusted
  because it is the key that signed (`holons.rs:435-440`); (b) the Scaling
  Holon submits a real `ValidatorSetChange` proposal for a fabricated candidate
  DID (`holons.rs:787-860`) while module docs call the manager
  recommendation-only, and no test exercises that path; (c) the Cargo.toml
  comment cites sentinel bytes that no longer exist.

Failure mode:

Recommendation-only automation can be mistaken for trusted adjudication actors.

Next red test:

- Infrastructure Holons cannot start with sentinel signatures, missing public
  keys, or recommendation-only authority if state-changing behavior is enabled.

Remediation track:

- Ratified decision D5 (2026-07-02): a self-issued key trusted by itself is the
  recursive self-authorization failure this architecture exists to prevent.
  Holon root authority is legitimate only with witnessed ceremony, external
  attestation, and lineage. Scaling-Holon auto-promotion is
  recommendation-only, full stop: promotion is a ratification event with named
  evidence.
- Red tests: (a) holon startup rejects self-issued root authority (grantor key
  equals signer key with no external delegation chain); (b) the Scaling Holon
  emits recommendation objects and submits zero `ValidatorSetChange` proposals.
- Holon adjudication contexts carry real signed authority and provenance chains.
- Recommendation-only behavior and state-changing behavior remain separate.

Closure gate:

```bash
cargo test -p exochain-node holon
cargo clippy -p exochain-node --all-targets -- -D warnings
```

## VCG-011 - Hardware TEE Quote Verification Requires Production Integration

**Priority:** P1
**Status:** Open
**Classification:** EXOCHAIN core
**Owner role:** TEE integration

Evidence:

- `crates/exo-gatekeeper/src/tee.rs:39-53` distinguishes simulated testing from
  production policy.
- `crates/exo-gatekeeper/src/tee.rs:145-151` rejects simulated platforms outside
  testing.
- `crates/exo-gatekeeper/src/tee.rs:158-176` generates deterministic simulated
  attestation fixtures and says hardware platforms require quote verification.
- `crates/exo-gatekeeper/src/tee.rs:342-372` fails closed for hardware quotes
  unless a `TeeQuoteVerifier` is supplied.

Failure mode:

Hardware-rooted trust claims can outrun the actual platform quote verifier.

Next red test:

- Production hardware TEE attestation rejects hardware quotes without a platform
  verifier and rejects simulated fixtures outside testing.

Remediation track:

- Selected platform verifier validates quote evidence, measurement, signer
  chain, revocation state, and freshness.
- Ratified decision D4 (2026-07-02): slice one is SGX/DCAP; TrustZone is
  descoped to a vendor-plugin interface. TEE attestations are an evidence
  class, never a standalone trust root. Red test addition: attestation
  revocation automatically and visibly downgrades dependent claims as DAG
  evidence objects - never silently.
- Scout notes (2026-07-02): the `TeeQuoteVerifier` trait and 40 fail-closed
  tests are the right foundation; current test verifiers are toys (fixed-byte
  signatures). No DCAP/SEV dependency exists in the workspace. The unused
  `allow-simulated-tee` feature is removed or wired, not left dangling.

Closure gate:

```bash
cargo test -p exochain-gatekeeper tee
cargo clippy -p exochain-gatekeeper --all-targets -- -D warnings
```

Closure requires platform-specific security review before hardware-rooted trust
claims are upgraded.

## VCG-012 - Distributed HLC Sync Protocol Is Not Built

**Priority:** P2
**Status:** Open
**Classification:** EXOCHAIN core
**Owner role:** Distributed time

Evidence:

- `crates/exo-core/src/hlc.rs:42-56` implements a local `HybridClock`.
- `crates/exo-core/src/hlc.rs:128-167` merges a received remote timestamp with
  drift checks.
- The historical April registry recorded missing multi-party HLC sync for causal
  ordering across nodes.

Failure mode:

Local HLC merge can be overread as full multi-node finality under partitions.

Next red test:

- Multi-node HLC fixtures expose partition, drift, replay, and merge ambiguity
  cases that current local merge does not settle.

Remediation track:

- Define and implement peer sync protocol, partition evidence, and deterministic
  conflict behavior.
- Ratified decision D6 (2026-07-02): HLC timestamps piggyback on the existing
  gossipsub DAG-sync channel - time rides the channel that already carries
  causality. Partition recovery is quorum-median with mandatory flag-and-alert;
  silent accept-max is forbidden (one bad clock must not steer history
  ordering). Time anomalies are constitutional events recorded as DAG evidence
  objects, because deliberation order is legitimacy-relevant.
- Scout notes (2026-07-02): local merge math is complete and tested (25 tests);
  the gap is wire-protocol only. Transport primitives exist in `exo-node`
  (`network.rs` gossipsub, `sync.rs` state sync). The red test includes a real
  `NetworkHandle` round trip, not in-process merge calls alone.

Closure gate:

```bash
cargo test -p exochain-core hlc
cargo clippy -p exochain-core --all-targets -- -D warnings
```

## VCG-013 - Tenant Metering, Billing, and Product Operations Are Structural

**Priority:** P2
**Status:** Open
**Classification:** EXOCHAIN core
**Owner role:** Tenant platform

Evidence:

- `crates/exo-tenant/src/lib.rs:17-27` exposes tenant, store, sharding, and cold
  storage modules.
- `crates/exo-tenant/src/store.rs:34-123` implements an in-memory tenant-aware
  key-value store.
- `crates/exo-tenant/src/tenant.rs:44-180` implements tenant config and
  lifecycle registry.
- `rg --files crates/exo-tenant/src` shows no metering, subscription, invoice,
  or billing module.

Failure mode:

Structural tenant primitives can be overstated as complete SaaS tenant
operations.

Next red test:

- Tenant metering tests fail because usage, subscription, and billing export
  records are absent or not tenant-bound.

Remediation track:

- Implement tenant usage metering, subscription state, billing export, and
  storage/runtime policy binding.
- Scout correction (2026-07-02): the crate is 1827 LOC across seven modules and
  per-tenant isolation with quota fields is already built (`store.rs`
  tenant-consistency checks); the open surface is exactly metering, billing
  export, and subscription state.
- Ratified decision D7 (2026-07-02): metering and billing live in isolated
  `exo-tenant` modules, not in `exo-economy`. Metering observes and never
  gates: usage reconciles against actual store state, aggregation windows use
  HLC time, invoices are deterministic, and no settlement or charge fires by
  default. A dependency-direction guard makes this machine-checked: no
  trust-path crate may import `exochain-tenant`.

Closure gate:

```bash
cargo test -p exochain-tenant
cargo clippy -p exochain-tenant --all-targets -- -D warnings
```

## VCG-014 - Governance and Legal Completeness Rows Remain Partial

**Priority:** P2
**Status:** Open
**Classification:** Governance/legal
**Owner role:** Council/legal

Evidence:

- `governance/resolutions/CR-001-AEGIS-SYBIL-AUTHENTIC-PLURALITY.md:137-142`
  leaves six Sybil sub-threat rows unresolved.
- `governance/resolutions/CR-001-AEGIS-SYBIL-AUTHENTIC-PLURALITY.md:208-218`
  (implementation tracking) marks eight work-order rows partial (8.1 through
  8.7 and 8.9) with only 8.8 release gating implemented; Section 9 has six
  unchecked release-blocking boxes (scout correction, 2026-07-02 - the original
  four-item framing undercounted).
- The historical April registry listed legal review, distributed HLC sync, and
  tenant scale as open scale-layer work.

Failure mode:

Constitutional completeness claims can outrun unresolved threat, legal, and
traceability rows.

Next red test:

- Governance source guard fails on unresolved Sybil, no-admin, and traceability
  rows when a release-complete claim is made.

Remediation track:

- Each unresolved row maps to source enforcement, tests, legal sign-off, or an
  explicit non-claim.
- Guard first (coordinator-authored): a companion work-order guard pins the six
  Sybil sub-threat rows, the eight partial-row caveat substrings, and the
  Section 9 checkbox states, so none can silently change without explicit guard
  maintenance. Substantive closure of individual sub-threats proceeds as
  separate lanes after the guard exists.
- `tools/test_cr001_status.sh` guards only the DRAFT status; any change set
  touching that guard and CR-001 status together is flagged for coordinator
  plus principal review.

Closure gate:

```bash
bash tools/test_gap_registry_truth.sh
cargo test -p exochain-gatekeeper invariants
cargo test -p exochain-governance
cargo clippy -p exochain-gatekeeper --all-targets -- -D warnings
cargo clippy -p exochain-governance --all-targets -- -D warnings
```

## Ratified Decisions

Ratified by the principal on 2026-07-02.
Master doctrine: **ratification precedes authority; authority follows evidence.**
Feature-default flips, Holon promotion, and charter amendments are ratification
events with named review evidence, never lane outcomes.

- **D1 - Proof backend:** RISC Zero, server-side proving only, Groth16 wrapping
  as receipt compression; verifier minimalism (small, in-workspace, pinned,
  audit-budgeted); toolchain vendored and pinned inside the `cargo deny`
  perimeter. Applies to VCG-001, VCG-003, VCG-004.
- **D2 - MCP topology:** standalone process plus authenticated, read-scoped RPC
  bridge; adjudicator and adjudicated never share a process boundary. Applies
  to VCG-004.
- **D3 - One authority model:** `exo-authority` DelegationRegistry entries are
  the single authority species for humans, models, and external verifiers.
  Applies to VCG-007 and issue `#736`.
- **D4 - TEE scope:** SGX/DCAP first; TrustZone as vendor plugin; attestation
  is an evidence class, never a trust root; revocation visibly downgrades
  dependent claims. Applies to VCG-011.
- **D5 - Root legitimacy:** witnessed ceremony plus external attestation plus
  lineage; self-issued roots are rejected; Scaling-Holon promotion is
  recommendation-only. Applies to VCG-010.
- **D6 - Distributed time:** HLC rides gossipsub DAG-sync; quorum-median
  recovery with mandatory flag-and-alert; time anomalies are DAG evidence.
  Applies to VCG-012.
- **D7 - Metering:** isolated `exo-tenant`; metering observes, never gates,
  never charges by default; a dependency-direction guard keeps trust-path
  crates free of `exochain-tenant`. Applies to VCG-013.
- **D8 - Doctrine:** every `unaudited-*` default flip is a ratification event
  with named review evidence. Applies to all feature-gated rows.

Decision queue (not ratified): D9 - AI-IRB council charter. Design frozen at
`governance/proposals/D9-COUNCIL-CHARTER-PROPOSAL.md`, canonical BLAKE3
`c1e89db47a30849d41e6db9c4c23d52d9dfbf3a820f2695dcdbcade6d42bd6af`; additive,
blocked-by D3/D4/D5 evidence maturity. Landing the proposal object in this
repository is not enactment; enactment requires explicit principal
ratification recorded against that hash. The coordinator's own seat record and
loop charter, including the amendment ratchet, live at
`governance/proposals/SEAT-000-COORDINATOR-RECORD.md`.

Claim frame for VCG-002 and all public materials: EXOCHAIN makes power
constitutional; the safety property is the governed channel, not the model
mind. Safety claims are stated as invariant five-tuples (invariant, adversary,
evidence, detection, failure mode), and the socio-technical scope condition is
stated openly.

## Explicit Corrections

- eDiscovery export is not an open origin-main gap. Current source routes
  authenticated requests through `exo-legal` eDiscovery search and includes
  source guards rejecting enterprise 501 markers.
- Gateway `production-db` is enabled by default on origin-main. Missing runtime
  DB state remains fail-closed, and live Railway probes currently report DB
  readiness.
- `EXOCHAIN_REAL_PROOF_BACKEND_HEAVY_LIFT_PLAN.md`,
  `GAP-REGISTRY-CURRENT-2026-07-02.md`,
  `GAP-REGISTRY-ORIGIN-MAIN-VERIFIED-2026-07-02.md`, and
  `docs/superpowers/plans/2026-07-02-systemic-integrity-tdd-remediation.md`
  were consolidated into this ledger and removed to eliminate split tracking.

## System Closure Gate

Run these before claiming systemic integrity closure:

```bash
bash tools/test_gap_registry_truth.sh
cargo build --workspace --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo doc --workspace --no-deps
cargo audit
cargo deny check
/usr/bin/curl -sS https://exochain.io/ready
/usr/bin/curl -sS https://exochain-production.up.railway.app/ready
/usr/bin/curl -sS https://exochain-production.up.railway.app/health/db
```

## Closure Record

No VCG row is closed at ledger creation. Each closure record must name:

- closing commit and PR;
- path classification;
- red test evidence;
- green local command evidence;
- GitHub CI evidence;
- runtime evidence when the row affects deployment;
- docs and governance claim updates;
- remaining risk or external review boundary.
