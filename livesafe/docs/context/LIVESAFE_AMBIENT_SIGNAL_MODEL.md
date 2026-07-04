# LiveSafe Ambient Signal Model

## Source Basis

- `docs/context/LIVESAFE_CONTEXT_SEED.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`
- `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`
- `context/canon/2026-05-24-phase-9-enterprise-onboarding-commercial-architecture.md`
- `src/ambient_signal.rs`
- `src/entitlement_marketplace.rs`
- `tests/ambient_signal.rs`

## Ground Truth

Ambient is a LiveSafe-adjacent context layer. Current repo and canon evidence
do not verify a broader runtime authority role for Ambient, so Ambient behavior
must stay fail-closed, synthetic, and metadata-only until a verified adapter
path exists.

The current source set only proves these Ambient-specific concepts:

- Ambient is part of the current product boundary vocabulary.
- Ambient context exists as a marketplace template category.
- Ambient signal acknowledgement is an explicit consent concept.
- Ambient runtime trust claims remain inactive until verified permit evidence
  exists.

## Current Contract Coverage

- `src/ambient_signal.rs` defines:
  - synthetic signal, policy, and session references,
  - owner preview versus context-pack dispatch modes,
  - metadata-only versus recipient-visible visibility,
  - marketplace-template and consent acknowledgement checks,
  - raw-sensitive, direct-contact, and location-trace denial,
  - verified-permit and disablement requirements for recipient-visible output,
  - verified-claim gating for Ambient trust language.
- `tests/ambient_signal.rs` proves:
  - missing references and non-synthetic fixtures deny,
  - Ambient context dispatch requires a declared marketplace template,
  - Ambient context dispatch requires acknowledged Ambient consent,
  - raw-sensitive, direct-contact, and location-trace payloads deny,
  - recipient-visible delivery denies without verified permit and disablement,
  - owner preview remains allowed while trust is inactive,
  - verified Ambient trust claims deny without verified permit state.

## Marketplace And Consent Boundary

Ambient dispatch is only modeled here as a context-pack behavior that depends
on explicit marketplace-template declaration and explicit Ambient signal
acknowledgement. This repo does not currently prove broader Ambient automation,
authority, or responder behavior.

## Runtime Boundary

- Ambient remains an adjacent surface, not EXOCHAIN core.
- recipient-visible Ambient delivery remains inactive until a verified adapter
  path returns permit.
- Ambient payloads stay metadata-only and synthetic.
- Ambient payloads may not embed raw sensitive records, direct contact values,
  or location-trace data.

## Disablement And Rollback

- Keep Ambient behavior limited to unwired Rust contract evaluation until a
  verified runtime path exists.
- Remove `ambient_signal` from `src/lib.rs` to disable the contract export.
- Disable any future recipient-visible route unless it has a current
  disablement reference and verified permit evidence.
