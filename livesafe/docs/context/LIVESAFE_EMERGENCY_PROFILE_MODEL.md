# LiveSafe Emergency Profile Model

## Source Basis

- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md`
- `src/emergency_profile.rs`
- `tests/emergency_profile.rs`

## Product Rule

LiveSafe emergency-profile contracts must keep profile data on synthetic
references only, restrict field names to the validated emergency-profile
inventory, block direct contact, location-trace, and QR-secret metadata, and
fail closed when responder disclosure requests fields outside the approved
emergency subset or any expanded responder scope.

## Domain Terms

- `ResponderEmergency`: approved minimal responder disclosure scope.
- `ResponderExpandedRequest`: blocked expanded responder disclosure scope that
  remains fail-closed until Bob approves live responder-access boundaries.
- `EmergencyCore`: responder-projected fields inside the approved emergency
  subset.
- `ReleaseBoundEmergency`: responder-projected release-bound fields that
  require explicit acceptance plus effective-date and revocation references.

## Implemented Contract

The emergency-profile contract now enforces:

- an allowlisted emergency-profile field-name inventory,
- synthetic value references instead of inline sensitive data,
- no raw sensitive payloads, direct contact details, location traces, or QR
  secrets in fixtures or metadata,
- responder emergency projection limited to the approved core emergency subset,
- release-bound emergency fields allowed only when explicit acceptance,
  effective-date, and revocation references are present,
- expanded responder disclosure blocked pending Bob-approved live scope,
- fail-closed projection when requested fields are missing or outside the
  responder emergency subset.

## Test Evidence

- `cargo test --test emergency_profile`
- `npm run quality`

## Boundaries

- Classification: adjacent surface domain contract.
- Trust posture: inactive; no live responder route, EXOCHAIN enforcement, or
  public trust claim.
- Data posture: synthetic references only; no raw identity, contact, medical,
  location, or QR-activation payloads.
- Disablement path: keep `src/emergency_profile.rs` unwired from runtime routes
  or remove the `emergency_profile` export from `src/lib.rs`.
