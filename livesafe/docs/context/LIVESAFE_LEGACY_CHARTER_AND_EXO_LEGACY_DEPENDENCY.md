# LiveSafe Legacy Charter And Exo-Legacy Dependency

## Source Basis

- `AGENTS.md`
- `docs/EXOCHAIN_APP_BOUNDARY.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/LIVESAFE_EXO_LEGACY_REQUIREMENTS.md`
- `context/canon/2026-05-24-phase-11-exo-legacy-build-package.md`
- `src/legacy_dependency.rs`
- `tests/legacy_dependency.rs`

## Ground Truth

LiveSafe has an executable adjacent-surface contract for legacy dependency
evaluation in `src/legacy_dependency.rs`, but it does not have a live runtime
route, adapter, or EXOCHAIN-core primitive that can activate legacy-charter,
posthumous-representation, genetic-unveiling, mausoleum-export, or erasure
claims. Current repo truth is that `crates/exo-legacy` is a proposed EXOCHAIN
core crate, not a verified dependency in this LiveSafe workspace.

The controlling posture is fail-closed:

- `crates/exo-legacy` is treated as pending EXOCHAIN evidence only,
- legacy capability labels remain inactive until a verified adapter response
  marks them active,
- emergency access remains separate from posthumous and quorum-governed
  behavior,
- receipt metadata may carry commitments, hashes, and policy references only,
  never raw charter, genetic, or interaction-memory content, and
- product copy may describe inactive requirements, but not verified legacy,
  genetic, posthumous, or erasure guarantees.

This document maps the existing Rust contract and current control docs to the
required LiveSafe boundary for any legacy-adjacent product surface.

## Current Contract Coverage

- Path classification: adjacent surface documentation and domain-contract
  mapping.
- `src/legacy_dependency.rs` currently defines:
  - dependency presence and adapter verification state for `crates/exo-legacy`,
  - charter-hash and invariant-validation checks,
  - separate legacy operation classes for `EmergencyTier0Read`,
    `LegacyCharterReview`, `PosthumousRepresentation`,
    `GeneticUnveiling`, and `Erasure`,
  - receipt metadata handling rules that deny raw charter contents, genetic
    payloads, and interaction-memory text,
  - capability activation state that keeps labels inactive until verified, and
  - copy-review checks that block unsupported public guarantees.
- `tests/legacy_dependency.rs` proves:
  - missing dependency evidence or missing adapter verification keeps legacy
    capabilities inactive,
  - missing charter hash or failed invariant validation yields denial,
  - charter contents, genetic payloads, and interaction-memory text are denied
    from receipt metadata,
  - Emergency Tier-0 reads depend on their own authorization boundary rather
    than quorum or payment state,
  - erasure posture requires a key-destruction receipt instead of a storage
    deletion guarantee, and
  - public copy blocks posthumous, genetic, and erasure guarantees without
    verified code and policy evidence.

## EXOCHAIN Dependency Boundary

The current EXOCHAIN boundary doc and canon Phase 11 record the same repo truth:

- the proposed dependency path is `crates/exo-legacy`,
- the local EXOCHAIN checkout referenced by the canon did not contain that crate
  when verified on 2026-05-24 at commit `7a4137f7`,
- LiveSafe is not allowed to substitute EXOCHAIN-core authority, receipt
  semantics, invariant validation, or charter activation while the crate is
  absent,
- no LiveSafe route, UI, or receipt may claim active `exo-legacy`
  verification until the crate exists, passes EXOCHAIN gates, and exposes a
  verified adapter.

That means legacy-adjacent behavior in this repo stays at inactive requirement
state only. Any future runtime integration must still satisfy the general
runtime claim gate in `docs/EXOCHAIN_APP_BOUNDARY.md` plus the legacy-specific
acceptance gate in `docs/TEST_PLAN.md`.

## Emergency Versus Posthumous Boundary

Legacy and emergency behavior must not collapse into a single authority path.
The current source-backed boundary is:

- Emergency Tier-0 access is a responder-facing safety path and must rely on
  its own authorization evidence.
- Emergency Tier-0 access must not consult payment state or quorum state.
- Posthumous representation, legacy-charter activation, genetic unveiling,
  self-retirement, and erasure remain non-emergency operations.
- Non-emergency legacy operations stay blocked when dependency evidence,
  adapter verification, policy state, quorum state, or invariant validation are
  not sufficient.

This separation preserves the LiveSafe safety-mesh posture without inventing
legacy authority that the repo cannot verify today.

## Data And Receipt Boundary

The current legacy contract, EXOCHAIN boundary, and test plan align on a strict
data posture:

- raw charter text stays out of LiveSafe receipt metadata,
- genetic payloads stay out of LiveSafe receipt metadata,
- interaction-memory text stays out of LiveSafe receipt metadata,
- emergency, medical, identity, and other raw sensitive records stay off-chain
  and outside legacy receipt metadata,
- erasure state may be represented only by key-destruction receipt evidence,
  not by a storage-provider deletion promise.

LiveSafe may model only safe metadata such as commitments, hashes, policy
references, adapter-derived activation state, and other redacted evidence
fields that satisfy the adjacent-surface boundary.

## Activation And Copy Gates

Before any legacy-adjacent feature can be shown as active, current source truth
requires:

1. Verified `crates/exo-legacy` dependency presence in EXOCHAIN evidence.
2. A verified adapter response for the requested legacy capability.
3. Present charter-hash evidence.
4. Passing invariant-validation evidence.
5. Safe receipt metadata only.
6. Capability labels marked active only by verified adapter response.
7. Product copy and API output that avoid unsupported posthumous, genetic, or
   erasure guarantees.

Legacy capabilities remain inactive until those gates are satisfied. This
includes legacy-charter activation, posthumous representation, genetic
unveiling, self-retirement, and any related surface copy or badge state.

## Disablement And Rollback

- Disablement path: keep legacy features at inactive requirement state unless
  verified dependency evidence and adapter responses exist.
- Runtime rollback path: if a future route is added, deny before persistence,
  badge activation, or external writes whenever dependency, charter, invariant,
  or adapter checks fail.
- Copy rollback path: revert to inactive requirement language if verified code
  or approved policy evidence is missing.
- Data rollback path: reject any receipt or metadata shape that attempts to add
  raw charter, genetic, interaction-memory, or other raw sensitive content.
