# LiveSafe Genesis Development Trust

## Source Basis

- `AGENTS.md`
- `docs/EXOCHAIN_APP_BOUNDARY.md`
- `docs/GENESIS_DEVELOPMENT_TRUST.md`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md`
- `context/canon/2026-05-25-phase-14-genesis-development-trust.md`
- `src/genesis-trust.ts`
- `src/genesis_development_trust.rs`
- `tests/genesis-trust.test.ts`
- `tests/genesis_development_trust.rs`

## Ground Truth

LiveSafe already has executable genesis development trust policy in both
TypeScript and Rust, but it does not have completed internal proof, a completed
7-of-13 FROST ceremony, or a verified runtime adapter that would allow public
or customer-facing EXOCHAIN or root-backed trust claims. Repo truth today is:

- ExoForge is permitted for internal development work during genesis,
- source provenance is required for trusted development input,
- third-party development input requires classification before internal use,
- external trust signaling is denied until proof and adapter gates pass, and
- public trust posture stays inactive, genesis-pending, or internal-proof only
  until the exact claim is verified.

This document maps the current canon, boundary docs, and executable contracts
to the LiveSafe control posture for genesis development trust. It does not
activate any runtime authority, production trust signal, or EXOCHAIN-backed
customer claim.

## Current Contract Coverage

- Path classification: adjacent surface documentation and domain-contract
  mapping.
- `src/genesis-trust.ts` currently defines:
  - internal development, private review, customer, and public audience states,
  - development-planning, implementation, internal-validation,
    external-trust-signal, and customer-runtime-claim use cases,
  - the executable FROST profile constant with threshold `7` and participants
    `13`,
  - source-provenance, internal-proof, runtime-adapter, and
    externally-visible-signal gates.
- `tests/genesis-trust.test.ts` proves:
  - ExoForge is allowed for internal development during genesis,
  - external trust signaling is denied before internal proof and completed
    FROST evidence,
  - the exact 7-of-13 FROST profile is required for external signaling,
  - development trust is denied when source provenance is missing.
- `src/genesis_development_trust.rs` currently defines:
  - executable source classes for Bob direction, ExoForge, verified EXOCHAIN
    runtime, and third-party input,
  - internal versus externally visible use checks,
  - exact FROST threshold and participant requirements,
  - required evidence messages for provenance, classification, proof, ceremony,
    and adapter state.
- `tests/genesis_development_trust.rs` proves:
  - ExoForge internal implementation is allowed when provenance is recorded,
  - external trust signaling denies until internal proof, completed 7-of-13
    FROST evidence, and verified adapter evidence all exist,
  - third-party internal-development use fails without classification,
  - all genesis development trust requires source provenance.

## Allowed Genesis Development Uses

Current source-backed genesis trust allows these internal uses now:

- ExoForge planning, implementation, validation support, and bounded execution
  for private development.
- TDD slices built from repo evidence and canon records.
- Synthetic fixtures, inactive trust-state UI, fail-closed adapter contracts,
  and internal review artifacts.
- Private validation reports that do not claim public or customer-facing
  verification.

These allowed uses are limited to internal development and private review. They
do not authorize legal, medical, billing, governance, custody, consent,
provenance, or EXOCHAIN runtime claims by proximity.

## External Trust Signal Gate

External trust signaling remains disabled until every required proof exists for
the exact claim surface:

1. Source provenance for the development input.
2. Completed internal proof gate.
3. Completed 7-of-13 FROST keygen ceremony evidence.
4. Verified runtime adapter for the specific LiveSafe claim.
5. Fail-closed tests proving deny, timeout, and unavailable behavior.
6. Raw sensitive data remains off-chain, out of anchors, and out of exported
   artifacts.

Until those gates pass, LiveSafe must not claim active EXOCHAIN enforcement,
root-backed authority, verified custody proof, verified consent proof,
authority-chain enforcement, or any other public trust-bearing guarantee.

## FROST Ceremony Profile

The current executable and written genesis ceremony profile is:

- Scheme: FROST
- Threshold: 7
- Participants: 13
- Current repo state: no completion transcript or participant attestations in
  this workspace
- External trust signaling state: disabled until completion and verification

This matches the current canon record, `docs/GENESIS_DEVELOPMENT_TRUST.md`,
`src/genesis-trust.ts`, and `src/genesis_development_trust.rs`.

## Disablement And Rollback

- Disablement path: keep public and customer-facing trust-bearing output in
  inactive, genesis-pending, or internal-proof states until exact claim gates
  pass.
- Runtime rollback path: if a future route attempts to signal verified trust
  without proof or adapter evidence, deny the output before rendering,
  persistence, or external writes.
- Review rollback path: reject any development artifact that lacks source
  provenance or uses unclassified third-party material for trusted internal
  work.
- Proof rollback path: if FROST, internal-proof, or adapter evidence becomes
  incomplete or unavailable, revert the affected claim surface to non-verified
  status immediately.
