# LiveSafe QR Activation Model

## Source Basis

- `docs/EXOCHAIN_APP_BOUNDARY.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`
- `context/canon/2026-05-24-phase-5-exo-safe-card-runtime.md`
- `context/canon/2026-05-24-phase-8-ice-card-images.md`
- `src/qr_pointer.rs`
- `src/qr_activation.rs`
- `tests/qr_pointer.rs`
- `tests/qr_activation.rs`

## Ground Truth

LiveSafe currently supports only a fail-closed QR activation contract. The repo
does not yet prove a live EXOCHAIN-backed responder or network activation path.

Current source evidence supports these bounded statements only:

- the printable card and QR flow use a retrieval or activation pointer rather
  than raw sensitive payloads,
- responder and network activation stay inactive until a verified adapter path
  returns permit,
- responder landing remains limited to the approved emergency subset,
- expanded responder disclosure remains Bob-only and blocked by default,
- QR activation routes need a disablement or rotation reference.

This model is an adjacent-surface contract. It does not authorize a live
first-responder disclosure route, public trust claim, or EXOCHAIN-rooted proof.

## Current Contract Coverage

`src/qr_activation.rs` now enforces:

- synthetic token, policy, and session references for activation fixtures,
- dependency on a current QR pointer policy that already passed fail-closed
  validation,
- metadata-only activation payloads with no raw sensitive records, direct
  contact values, or location traces,
- disablement or rotation references before activation can be shown,
- fail-closed responder and P.A.C.E. network activation until a verified adapter path returns permit,
- emergency-subset-only responder landing,
- denial of expanded responder disclosure until Bob approves live scope,
- denial of verified EXOCHAIN activation claims without verified permit state.

`tests/qr_activation.rs` proves:

- missing references and non-synthetic fixtures fail closed,
- responder and network activation deny when pointer validation or permit-state
  evidence is missing,
- raw sensitive, direct-contact, and location-trace payloads are blocked,
- expanded responder scope and missing disablement references are blocked,
- owner setup preview can remain inactive without making verified claims,
- verified EXOCHAIN activation copy is blocked unless permit-state evidence
  exists.

## Pointer And Runtime Boundary

- `src/qr_pointer.rs` remains the prerequisite contract for token metadata,
  current endpoint policy, stale-target denial, and rotation handling.
- `src/qr_activation.rs` adds the landing-state and activation-path boundary on
  top of that pointer contract.
- `docs/EXOCHAIN_APP_BOUNDARY.md` still blocks LiveSafe from claiming runtime
  EXOCHAIN enforcement until a verified adapter path exists and is tested fail
  closed.

## Responder Scope Boundary

The current approved QR activation disclosure posture is narrow:

- owner setup preview may exist as an inactive setup state,
- responder scan may project only the approved emergency subset,
- expanded responder disclosure remains blocked,
- public or customer-facing copy must not imply verified EXOCHAIN activation
  while runtime evidence is inactive.

This keeps the activation vocabulary aligned with existing emergency-profile and
trust-state boundaries without turning on live responder access.

## Disablement And Rollback

- Path classification: adjacent surface domain contract and control document.
- Trust posture: inactive; no public EXOCHAIN-backed activation claim is active
  in this repo.
- Data posture: synthetic references only; no raw sensitive QR, contact,
  location, or responder data is stored here.
- Disablement path: keep `src/qr_activation.rs` unwired from runtime routes or
  remove the `qr_activation` export from `src/lib.rs`.
