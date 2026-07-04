# LiveSafe Consent And Revocation Receipts

## Source Basis

- `docs/EXOCHAIN_APP_BOUNDARY.md`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_TO_EXOCHAIN_INTEGRATION_MAP.md`
- `src/consent_revocation_receipt.rs`
- `tests/consent_revocation_receipt.rs`

## Ground Truth

Consent and revocation proof is inactive in LiveSafe today because the consent adapter is not wired.
LiveSafe remains an adjacent surface and must not mint, cache, or simulate consent or revocation receipt outcomes outside EXOCHAIN.

Current repo evidence allows a fail-closed domain contract only: synthetic
evaluation of whether a receipt path stays inactive, adapter-backed, and
metadata-safe. It does not authorize runtime consent enforcement, public proof
claims, or substitute receipt issuance.

## Current Contract Coverage

`src/consent_revocation_receipt.rs` now enforces:

- consent and revocation proof remains inactive until a verified EXOCHAIN
  consent adapter is wired,
- receipt provenance must be a verified EXOCHAIN adapter path rather than a
  LiveSafe synthetic or cached receipt outcome,
- receipt metadata is limited to commitments, references, policy ids, and
  hashes only,
- product copy cannot claim verified consent or revocation proof without
  verified code and policy evidence.

`tests/consent_revocation_receipt.rs` proves:

- fail-closed denial while the adapter is not wired,
- denial when LiveSafe tries to mint or simulate local receipt outcomes,
- denial when receipt metadata includes raw sensitive payloads,
- allowance only for verified-adapter receipt provenance with safe metadata,
- copy-review denial for unsupported verified-proof claims.

## EXOCHAIN Consent Boundary

The written and executable boundary is unchanged:

- `docs/EXOCHAIN_APP_BOUNDARY.md` says LiveSafe must not mint, cache, or
  simulate consent, revocation, or receipt outcomes outside EXOCHAIN.
- `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md` keeps consent
  and revocation proof inactive until a verified adapter exists.
- `docs/context/LIVESAFE_TO_EXOCHAIN_INTEGRATION_MAP.md` records the current
  runtime posture as unwired and fail-closed.

This contract is therefore an adjacent-surface guardrail, not proof that a
runtime consent adapter exists.

## Receipt Metadata Boundary

Consent and revocation receipts may contain commitments, references, policy ids, and hashes only.
Raw medical, identity, contact, location, QR, payment, or other sensitive
payloads stay out of receipt metadata, logs, fixtures, and EXOCHAIN anchors.

## Activation And Copy Gates

- Runtime activation remains blocked until a verified adapter path is wired and
  tested fail closed.
- Public or customer-facing copy must not claim verified consent or revocation
  proof until verified code and policy evidence exists.
- Internal development may refer to the inactive requirement and the adapter
  gate, but not to a completed proof state.

## Disablement And Rollback

- Path classification: adjacent surface domain contract and control document.
- Trust posture: inactive; no EXOCHAIN-backed consent or revocation proof claim
  is active in this repo.
- Data posture: synthetic references only; no raw sensitive consent payloads or
  receipt contents are stored here.
- Disablement path: keep `src/consent_revocation_receipt.rs` unwired from
  runtime routes or remove the `consent_revocation_receipt` export from
  `src/lib.rs`.
