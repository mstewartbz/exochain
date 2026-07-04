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

# Ratification Slate — VCG Decision Gates (2026-07-04)

**Provenance.** This is the verbatim ratification slate issued by the principal
(Bob Stewart) on 2026-07-04, resolving VCG-004, VCG-001, and VCG-011 and
directing the heavy-lift engineering workstreams W1–W5. Ground-truth verified at
commit `2c1a8f65`; slate hardened by a two-reviewer adversarial pass
(doctrine-consistency + gameability), all blocking findings incorporated. It is
the authoritative decision record referenced by `GAP-REGISTRY.md` rows VCG-001,
VCG-004, and VCG-011, and by the Objective-Gated Pending (OGP) and Green-Light
Protocol notes there. It records a governance decision; it is not a parallel gap
registry (the Single Source Rule is preserved — `GAP-REGISTRY.md` remains the
execution ledger).

---

## Preamble — Standing Corrections and the OGP Pattern

**P1. Funding is not a constraint.** All funding-contingent hedges in the
deliberation dossier are struck. No decision below is gated, sequenced, or scoped
by affordability. Cost figures survive only as planning data for vendor
negotiation. One substantive consequence: the VCG-001 Stage-1 dissent resolves in
favor of commissioning the paid Stage-1 design review **now** — its deliverable is
the design, which exists (O-1.1); a design review does not rot when code is
subsequently written to the reviewed design, and it reserves the Stage-2 queue
slot.

**P2. The Objective-Gated Pending (OGP) pattern is codified.** The
chicken-and-egg deadlock — audit needs finished code; rows read as "blocked" while
code waits on audit status — is resolved as follows, and this pattern (including
its Green-Light Protocol) applies to all externally-gated rows going forward:

1. The row's **green objectives are elicited, made mechanism-anchored, and
   ratified NOW** (in this slate).
2. Engineering proceeds immediately and independently — row status never gates the
   build.
3. Independent processes (external review, validation, hardware testing) run
   against the frozen artifacts.
4. **Passing all ratified objectives green-lights the row without a second
   ratification round-trip**, executed exclusively through the Green-Light
   Protocol (P2.5). This slate IS the ratification event; external reports are
   evidence, never authority — "ratification precedes authority" is honored
   because the ratification precedes the audit, attached to objective criteria
   rather than to any vendor's or lane's judgment.
5. **Green-Light Protocol (mechanized pre-flip — adjudicator separation is
   mandatory):**
   - **(a) Green-Light Memo:** the lane produces a memo enumerating each ratified
     objective with its evidence artifact — frozen commit hash, test-run logs,
     report hash — one artifact per objective, no objective satisfied by
     assertion.
   - **(b) Independent CONFIRM:** a non-lane integrity verifier (an independent
     adversarial process, never the building lane) issues a written CONFIRM that
     each artifact satisfies the ratified objective text. The CONFIRM is included
     in the flip commit. The lane that builds never adjudicates its own
     objectives.
   - **(c) Notice to Bob:** the memo + CONFIRM are delivered to Bob with a defined
     objection window (default 72 hours) before the flip executes. This is notice,
     not a ratification round-trip — silence green-lights; objection halts.
   - **(d) Machine-checkable deviation triggers** (any one halts the flip and
     escalates to Bob, regardless of whether the lane objects): any diff hunk
     outside the pre-declared file set; any dependency-graph delta relative to the
     frozen audited commit; any severity **or classification** change between a
     vendor's draft and final report; any objective ambiguity or scope change.
6. **Scope carve-out (D8 preserved):** OGP green-lights **row status only**. Any
   `unaudited-*` feature-default flip, Holon promotion, or charter amendment
   embedded in a row's green path remains a distinct ratification event under the
   master doctrine and D8 — unless that specific diff is itself pre-declared and
   ratified in the objectives (as O-1.6 does for VCG-001). No such event exists in
   the three rows below.
7. **Status vocabulary (Single Source Rule):** the labels `Pending-External` and
   `Pending-Activation` land either as annotations on the existing ledger statuses
   (`Red` / `Open (Blocked-external)`) or via a Status Values amendment landed
   together with the `tools/test_gap_registry_truth.sh` guard update in the same
   change set. No orphan vocabulary.

**P3. Heavy-lift engineering is the directive, not the residue.** The dossier's
honest bottom line — no mechanism moved and none will until engineering hours are
allocated — is accepted and answered in Ruling R4. Ledger edits, criterion
rewording, and hygiene are necessary but are not progress; the workstreams in R4
are.

---

## RULING R1 — VCG-004: MCP Mutation Authority

**R1.1 — Option B and "both (B interim → A later)" are REJECTED**, on the
following recorded grounds:

- Consensus adjudication is authenticity-and-ordering only: every honest validator
  auto-votes any structurally valid proposal (`reactor.rs:1097-1114`). No merits
  deliberation exists at any validator-set size.
- Default configuration self-commits at quorum 1 (`main.rs:147-151`).
- The BFT min-4 removal floor lives only in the HTTP handler (`api.rs:643-650`)
  and is bypassed by direct `submit_proposal` callers.
- The MCP caller is structurally invisible behind the node signature
  (`reactor.rs:1511-1531`) — the same defect class refuted in the prior fake
  (commits `fbd675e8`/`50753c3b`).
- A node-attached mutation interim exceeds ratified D2's read-scoped,
  process-separated end state — an un-sunsettable authority ratchet.
- B would be the first default-build production caller of `submit_proposal`, a
  production-authority posture change presented as "buildable now."

**Reopening conditions (recorded, not expected):** B may be re-proposed only if ALL
of the following hold: a deliberative or human co-sign gate on validator-change
proposals; the min-4 floor enforced at the validate layer; end-to-end
authenticated caller identity; a deployed multi-validator quorum. Noted for the
record: under those conditions B costs as much as A while still granting the wrong
authority class.

**R1.2 — Red-with-reason is the ratified standing state** for VCG-004 until its
objectives (R1.4) are met. This is the ledger functioning, not failing.

**R1.3 — A-narrow is the committed design:** a single `DecisionCreate` consensus
payload variant; mirrored canonical wire type in exo-node (no dependency on the
decision-forum crate), guarded by a cross-crate round-trip test; strict canonical
CBOR validation arm parallel to `validate_governance_proposal_payload`;
apply-on-commit arm writing to a decision-application store; `cast_vote` and
`advance_decision` excluded from v1.

**R1.4 — Green objectives for VCG-004 (mutation half), ratified now under OGP:**

- **O-4.1** A-narrow implemented per R1.3.
- **O-4.2 Attribution (strict bar, ratified):** the mutation half goes green only
  when the MCP caller's identity is cryptographically present in the committed
  payload's provenance — caller signs the decision payload with its own DID key;
  the node signature is transport-only — carried over a write-capable authenticated
  transport. **Explicit precondition: green additionally requires the W2
  write-scope amendment ratified by Bob; until that ratification lands, completion
  of O-4.1 through O-4.3 is recorded progress, never green.** Node-attributed
  `DecisionCreate` without caller identity is recorded progress, never green.
- **O-4.3 Mechanism-anchored closure gate** (replaces the test-name criterion): a
  ≥4-validator harness in which an MCP-originated `DecisionCreate` commits at
  quorum and is independently applied on a second node; the committed payload
  carries the verified caller DID; the min-validator floor is asserted at the
  validate layer; both prior refutation defects (misattribution, no-op mutation
  theater) exist as named, passing lock tests. **Harness independence:** harness
  nodes share no store or mutable state; node 2's apply must occur via its own
  commit path driven by network-delivered consensus messages; the closing
  assertion reads the decision from node 2's independent store.
- **O-4.4 CGR half:** greens only when `exochain_verify_cgr_proof` dispatches to
  the VCG-001-audited verifier with its own passing wire-through tests (positive
  verify + fail-closed negative), and the VCG-001 semantic-boundary sentence is
  updated to record this as the first production consumer. Inheriting VCG-001's
  green alone — with an MCP CGR tool that still refuses every call — does not green
  this half. Full row green requires both halves.

**R1.5 — Immediate directives (option-independent, execute under R4):**

- Push the BFT min-4 removal floor from `api.rs:643-650` into
  `validate_governance_proposal_payload`/`submit_proposal`, coordinated with the
  VCG-014 concurrent-remove race so the fix lands once.
- Quarantine ALL refuted branches — three refs: local
  `vcg/004b-mcp-mutation-effect-work`, local `vcg/004b-mcp-mutation-effect`, and
  `origin/vcg/004b-mcp-mutation-effect` — tagged `refuted-do-not-merge`; salvage
  only the `NodeContext.net_handle` design notes.
- Amend the GAP-REGISTRY line calling the refuted prototype's infrastructure
  "sound and reusable" to record that its attribution defect is structural.
- Correct the decision record's Option A con: strike "touches the consensus core";
  record "extends exo-node's payload layer; BFT engine unchanged" so future D1
  audit scoping inherits the accurate blast radius.

---

## RULING R2 — VCG-001: External Cryptographic Review

**R2.1 — Status codified: `Pending-External (objectives ratified)`** under OGP
(vocabulary per P2.7). The row is not "blocked" — the build proceeds now (R4/W1);
the review runs against the frozen artifact; green fires through the Green-Light
Protocol on objectives.

**R2.2 — The pass criterion is REWORDED.** "Auditor certifies the RISC Zero
verifier is sound" is struck: no reputable firm issues that artifact, and demanding
it forces either permanent red or a proof-shaped letter. The certificate→promotion
automation is also struck — the report is evidence; this slate is the ratification.

**R2.3 — Green objectives for VCG-001, ratified now under OGP:**

- **O-1.1 Seam binding fixed before freeze:** the `RiscZeroReceiptVerifier` seam
  carries (or journal-digest-binds) `domain_separator`, `commitment_roots`, and
  `statement_kind` through to verification. The current seam
  (`envelope.rs:268-272`) drops all three; freezing it would certify a sound
  receipt-verifier of an unbound statement.
- **O-1.2 VCG-001c built and frozen:** pinned risc0 toolchain vendored into the
  cargo-deny perimeter **pre-freeze**; Groth16-wrapped receipt verifier implemented
  behind the seam; commit frozen and tagged. The audited artifact and the shipped
  dependency graph are the same graph.
- **O-1.3 Upstream-coverage verification — independent, never lane-self-confirmed:**
  the confirmation that RISC Zero's published third-party audits cover the exact
  pinned release must come from the independent process — either the Stage-2 vendor
  countersigns the coverage claim for the pinned release, or the audited-commit →
  pinned-version delta is reviewed inside the engagement. The lane browsing upstream
  audit lists and writing "confirmed" in the row is not a satisfying path. The delta
  (or countersignature) is recorded in the row.
- **O-1.4 Independent review passed at the frozen commit** by a firm meeting the
  certificate standard (R2.4), with:
  - **Zero unresolved findings in the protected class, defined by mechanism, not
    report label:** any finding whose exploit path results in a receipt or journal
    being accepted under a `domain_separator`, `commitment_roots`, or
    `statement_kind` other than the one verified — regardless of the vendor's own
    severity or classification. Class membership is not negotiable.
  - Zero unresolved high/critical findings elsewhere.
  - **"Resolved" is defined:** a code/config fix verified in the fix-verification
    addendum, OR a Bob-ratified risk acceptance recorded in the row. Nothing else
    resolves a finding.
  - Any severity or classification change between the vendor's draft and final
    report escalates to Bob (P2.5.d), whether or not the lane objects.
- **O-1.5 Scope boundaries held:** pedagogical feature-on blake3 surface out of
  scope except a production-build gate-bypass check (`guard_unaudited` and cfg
  gating themselves IN scope); supply-chain scope is the pinning discipline (exact
  pins, checksums, deny/vet policy, advisory subscription, re-review trigger), not a
  transitive-tree line review; no re-audit of risc0 internals (duplicates ~19
  published engagements — D1 violation; P1 funding-unconstrained may not be cited to
  reopen this).
- **O-1.6 Pre-declared green diff, executed as one reviewed change with zero
  dependency-graph delta:** components named by test/function (line numbers will
  drift by freeze): registry promotion of the RiscZero descriptor to
  `ProductionReviewed`; seam swap from `FailClosedRiscZeroVerifier` to the audited
  verifier in `ProofEnvelope::verify()`; un-ignore the standing red
  `production_backend_variant_executes_without_unaudited_flag`; the two
  anti-overclaim locks replaced with pre-declared successor assertions —
  `anti_overclaim_default_registry_has_zero_production_reviewed_backends` becomes
  *"default_registry() contains exactly one ProductionReviewed backend — RiscZero —
  with named review evidence referenced at the pinned commit"*, and
  `riscz_backend_is_never_marked_production_reviewed` becomes *"RiscZero is
  ProductionReviewed only while the in-repo review evidence file exists"* — with
  lock 1's doc comment updated in the same diff so ratified text and code agree.
  **The green diff carries zero Cargo.lock or deny.toml delta relative to the
  frozen audited commit; any dependency hunk is a P2.5.d deviation** (risc0 lands
  pre-freeze under O-1.2, never post-audit). This diff shape is pre-authorized here
  so the test edits are never mistaken for lock-gaming.
- **O-1.7 Standing pin-and-monitor, machine-enforced at promotion:** rz-security
  advisory subscription; exact-version pin + checksums; and a CI check that fails on
  any risc0 version change lacking a linked re-review record — the re-review trigger
  cannot silently decay.
- **Green semantic boundary (recorded in the row):** green attests crate-boundary
  verifier soundness, feature-off, at the pinned commit; no production component yet
  produces or consumes proofs (updated when O-4.4 lands the first consumer);
  end-to-end production proof flow is a later row. Conditionality sentence recorded
  per O-1.3.

**R2.4 — Certificate standard codified:** named firm/reviewer with a verifiable
ZK-verification track record; written report pinned to the frozen commit hash;
explicit scope statement **matching the ratified O-1.5 in/out list verbatim** (the
commissioning letter may not narrow it); fix-verification addendum; archivable
in-repo as named review evidence; publication or reference rights.

**R2.5 — Commissioning directives (execute now, funding unconstrained):**

- Commission **Stage 1 immediately**: a paid design review by the selected (or
  shortlisted) firm covering the seam-binding design (O-1.1) and the
  envelope/registry surface, reserving the Stage-2 queue slot.
- Issue the 3–4 firm RFQ now (zkSecurity / Veridise / Hexens archetype + one
  Trail-of-Bits-class generalist for the supply-chain dimension) with the full
  scope packet: frozen-commit intent, threat model (adversarial proof submitter,
  server-side prover per D1), D1 text, crate map including the standing red and both
  anti-overclaim locks, explicit in/out list per O-1.5, invocation
  `cargo test -p exochain-proofs` in both feature configurations.
- Sign the Stage-2 SOW on schedule fit alone; all vendor quotes replace the
  dossier's planning figures on receipt.

---

## RULING R3 — VCG-011: TEE Attestation

**R3.1 — The commissioning brief is REJECTED as shaped.** No external party is
engaged to *supply* a verifier: quote verification is a maintained ecosystem
capability (Intel QVL Rust bindings; Phala `dcap-qvl`; VirTEE `sev`), and a bespoke
verifier is a depreciating fork against ~30-day collateral expiry and TCB recovery
events. This rejection is shape-based, not budget-based, and survives P1.

**R3.2 — Status codified: `Pending-Activation (trigger + objectives ratified)`**
under OGP (vocabulary per P2.7). The row records affirmatively: refusal mechanism
VERIFIED (41/41 tests at `2c1a8f65`; hardware quotes always rejected without a
verifier; simulated refused in production even when manually allowed); remaining gap
is dormant-capability activation, not a live vulnerability. The row stays un-green.

**R3.3 — Activation trigger (authorization and ratification are split):** W5
authorizes *designing and building toward* the candidate consumer (holon tenant
isolation per PANEL-4 ARCH-007). The trigger itself fires only on a **separate
one-line Bob ratification of the named consumer at design-complete**, which must
state what production surface exercises the flow (or reference the consumer's own
ratified row with acceptance criteria). If the lane selects a different consumer
than ARCH-007, that selection returns to Bob. Mechanism progress comes from creating
a real consumer — never from validating a verifier against nothing, and never from
consumer-theater built solely to fire this trigger.

**R3.4 — Green objectives for VCG-011, ratified now under OGP:**

- **O-11.1** A named production verifier type (the closure blanket impl at
  `tee.rs:250-261` sealed or excluded from the production path) wired into ≥1
  **real** consumer flow — where *real* is defined as: exercised by a production
  surface, or carrying its own ratified acceptance row; never satisfied by a flow
  whose only caller is the VCG-011 validation harness.
- **O-11.2** Platform scope inherits ratified D4 exactly: SGX/DCAP slice one;
  TrustZone as vendor-plugin interface. Any TDX-first design (adding the missing
  `Tdx` variant; `dcap-qvl` covers both under DCAP) requires its own one-line
  ratification before entering scope — pragmatism does not silently amend D4. "And/or
  SEV-SNP" is struck.
- **O-11.3** A **live-generated** quote from real hardware, produced during the
  validation window with collateral timestamps recorded, verifies against a pinned
  vendor trust root (self-hosted collateral path; hosted attestation services usable
  as cross-check oracle only, never as trust root). A stored fixture quote replayed
  under mocked time does not satisfy this objective.
- **O-11.4** Red tests all passing: mutated quote fails; revoked/OutOfDate TCB
  status visibly downgrades dependent claims as DAG evidence objects (D4's ratified
  red test); stale collateral (past nextUpdate) fails closed; simulated attestation
  remains refused in production.
- **O-11.5** `TeeAttestation` struct evolution (no field today for multi-KB quote +
  cert-chain material, `tee.rs:61-71`) designed and landed as internal engineering
  BEFORE external validation, so the reviewer validates a real interface.
- **O-11.6** External validation of the integration passed, under the R2.4
  certificate standard applied mutatis mutandis: named firm/reviewer with a
  verifiable TEE/attestation track record; written report pinned to the commit under
  validation; explicit scope statement matching the O-11.x in/out list verbatim
  (including the `dcap-qvl` targeted-review condition if that crate is chosen, since
  it lacks a dedicated audit — else Intel's official QVL bindings); fix-verification
  addendum; archivable in-repo as named review evidence.
- **O-11.7** Production policy hardening (freshness bound, measurement pinning) set
  at consumer-definition time, with the chosen freshness bound and pinned
  measurements **recorded in the consumer's ratification line itself** —
  real-vs-placeholder is checkable, not judged. No placeholder numerology now.

**R3.5 — The FHE footnote is STRUCK** from VCG-011 as a category error: FHE provides
confidentiality, not attestation or integrity-of-execution; it cannot produce the
evidence class this row gates; verifiable-FHE is research-grade. Any future
confidentiality-only requirement opens its own row in its own lane.

**R3.6 — Immediate hygiene directives (execute under R4/W4):** delete the dangling
`allow-simulated-tee` feature flag (`crates/exo-gatekeeper/Cargo.toml:50`, zero cfg
references); correct the stale claim at `docs/council/PANEL-4-SECURITY.md:569`; seal
or gate the closure blanket impl; claims-hygiene guard in force — no TEE /
hardware-rooted-trust language on any external surface while the row is pending.

---

## RULING R4 — Heavy-Lift Engineering Directive

The system is directed to plan and execute the following workstreams as real
mechanism progress. Every workstream carries an owner, a mechanism-anchored closure
gate, and a scheduled slot before work is considered planned. Ledger edits alone
never close a workstream.

**W1 — VCG-001c (critical path to the first externally-reviewed green).
Priority 1.** Sequence: seam-binding design + implementation (O-1.1) → vendor pinned
risc0 into the deny perimeter (O-1.2, pre-freeze) → Groth16 receipt verifier behind
the seam → upstream-coverage arrangement (O-1.3, via the engagement) → freeze + tag.
Stage-1 design review (R2.5) runs against the seam design at the head of this lane.
Closure gate: frozen tagged commit passing `cargo test -p exochain-proofs` in both
feature configurations with the new verifier integrated fail-closed.

**W2 — D2 authenticated bridge (the commercial keystone). Priority 1, parallel to
W1.** The unbuilt convergence point of VCG-003 closure, LiveSafe mutations, and
attribution-honest MCP writes (O-4.2's transport). Build the read-scoped bridge now
— it is already the ratified end state and needs no new authority decision. In
parallel, prepare the **write-scope amendment memo** (caller-signed mutation
payloads over the authenticated bridge) as a separate ratification item for Bob —
the write extension is an authority decision, is NOT pre-authorized by this slate,
and is the explicit precondition inside O-4.2. Closure gate: gateway↔node
authenticated RPC carrying read traffic in a multi-process deployment, with the
amendment memo delivered.

**W3 — VCG-004 hardening + A-narrow build-ready design. Priority 2.** Execute R1.5 in
full (floor push-down with VCG-014, three-ref branch quarantine, ledger amendments).
Produce the A-narrow implementation spec (wire type, validation arm, apply arm,
decision-application store, cross-crate round-trip guard, O-4.3 harness design
including the independence requirements) to build-ready state, so the build starts
the day W2's write-scope transport is ratified — demand pressure never forces a hasty
design. Closure gate: hardening merged with lock tests; spec reviewed.

**W4 — Hygiene + ledger sweep. Priority 3, small, bounded.** R3.6 items; the three
stale exo-proofs doc comments (`envelope.rs:43-44`, `:151-152`, `refusal.rs:111-116`);
the VCG-002-protecting semantic-boundary sentences recorded in the VCG-001 and
VCG-011 rows; the P2.7 status-vocabulary change (annotation or amendment + guard
update, one change set); a bounded ledger pass identifying which downstream rows can
honestly proceed against `PendingExternalReview` state without any status flip;
claims-hygiene guard propagated to external-surface checklists. Closure gate: all
named items merged; explicitly forbidden from being reported as mechanism progress.

**W5 — TEE consumer lane (builds toward the VCG-011 trigger). Priority 3,
authorized.** Design and build the first real TEE-attestation consumer (candidate:
ARCH-007 tenant isolation) toward the R3.3 design-complete ratification point. On
that ratification, execute VCG-011b per R3.4 objectives: integrate an established
verifier behind a named adapter; rent **SGX/DCAP-capable hardware** for live-quote
testing per the ratified platform slice (TDX hardware only after the O-11.2 one-line
ratification lands); commission the small external validation under O-11.6's
certificate standard. Closure gate: O-11.1 through O-11.7.

**Sequencing note:** W1 and W2 are the mechanism-progress core and proceed
concurrently; W3 hardening is same-week; W5 proceeds as capacity allows behind W1/W2,
never ahead of them.

---

## Adversarial Review Record

Two independent reviewers attacked the draft before finalization.
**Doctrine-consistency** (verdict: ratifiable with edits): every spot-checked
file:line receipt verified at `2c1a8f65`; the D2 read/write split and the Option B
rejection survived attack; two blocking findings (W5's TDX wording silently amending
D4; P2's forward-sweep lacking the D8 carve-out) — both fixed above.
**Gameability** (verdict: ratifiable with edits): the registry tag-flip,
certificate-automation, node-attribution-shortcut, and single-node-harness gaming
paths all confirmed closed; eight blocking gaming paths found (adjudicator
self-certification, vendor class-relabeling, green-diff dependency smuggling,
unbounded lock "relaxation", coverage self-attestation, O-4.2/W2 transport conflict,
harness store-sharing, missing O-11.6 certificate standard, consumer-theater) — all
closed above via P2.5, O-1.3/O-1.4/O-1.6, O-4.2/O-4.3/O-4.4, O-11.1/O-11.6, and R3.3.

---

## Ratification Block (as marked by the principal, 2026-07-04)

| # | Item | Ruling |
|---|------|--------|
| P1 | Funding hedges struck; Stage-1 commissions now | RATIFIED |
| P2 | OGP pattern + Green-Light Protocol (independent CONFIRM, 72h notice window, machine-checkable deviation triggers, D8 carve-out, status-vocabulary rule) codified for all externally-gated rows | RATIFIED |
| R1.1 | VCG-004 Option B + "both" rejected on recorded grounds with reopening conditions | RATIFIED |
| R1.2–R1.4 | Red-with-reason standing; A-narrow committed; strict attribution bar with W2-ratification precondition (O-4.2); mechanism-anchored independent harness (O-4.3); CGR wire-through requirement (O-4.4) | RATIFIED |
| R1.5 | VCG-004 immediate hardening directives (incl. three-ref quarantine) | RATIFIED |
| R2.1–R2.4 | VCG-001 → Pending-External; reworded criterion; objectives O-1.1…O-1.7 (mechanism-defined protected class, independent coverage confirmation, zero-dependency-delta green diff, CI-enforced pin-and-monitor); certificate standard | RATIFIED |
| R2.5 | Stage-1 commissioned now; RFQ issued now | RATIFIED |
| R3.1–R3.6 | VCG-011 → Pending-Activation; split trigger (R3.3); objectives O-11.1…O-11.7 (real-consumer definition, live-quote requirement, ported certificate standard); FHE struck; hygiene | RATIFIED |
| R4 | Heavy-lift workstreams W1–W5 with owners, gates, slots | RATIFIED |

Per Human Primacy: nothing above executes until Bob marks the block. The Green-Light
Protocol (P2.5) governs every subsequent flip; its deviation triggers escalate to Bob
unconditionally. Recorded here as marked by the principal on 2026-07-04.
