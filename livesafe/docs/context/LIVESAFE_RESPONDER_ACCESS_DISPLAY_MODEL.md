# LiveSafe Responder Access Display Model

## Source Basis

- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_EMERGENCY_PROFILE_MODEL.md`
- `docs/context/LIVESAFE_QR_ACTIVATION_MODEL.md`
- `docs/context/LIVESAFE_VITALLOCK_VAULT_MODEL.md`
- `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`
- `src/responder_access_display.rs`
- `tests/responder_access_display.rs`

## Ground Truth

LiveSafe now defines a dedicated adjacent-surface contract for responder-facing
display behavior. Current repo truth still keeps responder access fail-closed:
the display may show only the approved emergency subset, remains synthetic and
metadata-only, and must not imply live EXOCHAIN-backed responder authority.

This model does not approve expanded responder disclosure, does not activate a
runtime responder route, and does not authorize direct contact, location,
medical-record export, or broader vault disclosure.

## Current Contract Coverage

`src/responder_access_display.rs` now enforces:

- synthetic responder session, policy, and disablement references,
- accessible and machine-readable responder status fields,
- synthetic-only fixtures with no raw sensitive payloads,
- denial of direct contact values and location traces,
- emergency-subset-only responder panel inventory,
- blocked expanded responder scope pending Bob approval,
- dependency on the emergency-profile contract for emergency panels,
- dependency on the QR activation contract for QR-linked panels,
- dependency on the VitalLock vault contract for responder vault panels,
- verified-claim denial unless the responder path is in `VerifiedPermit`.

`tests/responder_access_display.rs` proves:

- missing references and missing accessibility or machine-state fields deny,
- non-synthetic, raw-sensitive, direct-contact, and location-trace payloads
  deny,
- inactive emergency-subset display remains allowed without making verified
  claims,
- expanded responder scope and unapproved panels deny,
- QR-linked, vault-linked, and verified responder claims deny without the
  required contract evidence or verified permit state.

## Emergency Subset Boundary

The current approved responder display boundary is narrow:

- responder-facing panels may show only emergency identity, emergency medical
  summary, QR activation status, and VitalLock emergency badge status,
- P.A.C.E. contact summaries remain excluded from responder display output,
- full vault export remains excluded from responder display output,
- Expanded responder access displays remain blocked until Bob approves live
  scope.

This keeps responder display behavior aligned with the existing emergency
profile, QR activation, and VitalLock vault contracts without activating a
broader responder-access path.

## Trust And Runtime Boundary

- Classification: adjacent surface domain contract and control document.
- Trust posture: inactive by default; responder status must remain explicitly
  not-yet-verified unless a verified permit path exists.
- Data posture: synthetic references only; no raw medical, identity, direct
  contact, location, or vault-export payloads.
- Runtime posture: no responder route is activated by this contract alone, and
  no EXOCHAIN-backed responder claim is authorized here.

## Disablement And Rollback

- Keep `src/responder_access_display.rs` unwired from runtime routes until a
  verified responder authorization path exists.
- Remove `responder_access_display` from `src/lib.rs` to disable the contract
  export.
- Disable any future responder-facing route unless it has a current
  disablement reference and verified permit evidence.
