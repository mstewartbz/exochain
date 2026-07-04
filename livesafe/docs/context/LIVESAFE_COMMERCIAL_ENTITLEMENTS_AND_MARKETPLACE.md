# LiveSafe Commercial Entitlements And Marketplace

## Source Basis

- `context/canon/2026-05-24-phase-9-enterprise-onboarding-commercial-architecture.md`
- `context/canon/2026-05-25-phase-17-storage-entitlement-offering.md`
- `src/entitlement_marketplace.rs`
- `tests/entitlement_marketplace.rs`

## Doctrine

- LiveSafe basic individual accounts are free.
- Family plans, team plans, advanced capabilities, and marketplace templates
  are entitlement-gated.
- Trial and gift flows must be explicit state, not inferred from billing side
  effects.
- Frontline basic family eligibility must use deterministic metadata and must
  not store raw proof documents.
- Marketplace templates must declare rule scope, plan gate, consent,
  audit behavior, and disablement behavior.
- Stripe-backed billing references and custom-contract classification are
  configuration state only; this repo stores synthetic references only.

## Current Domain Contract

- `src/entitlement_marketplace.rs` defines:
  - free, family, team, and frontline-basic-family plan states,
  - explicit trial and gift states,
  - paid-capability gating,
  - deterministic frontline cohort validation,
  - marketplace template catalog entries for golden-hour outreach, family
    preparedness, disaster planning, Ambient context, Decision Forum, and
    Syntaxis packs.
- `tests/entitlement_marketplace.rs` proves:
  - free, paid, trial, gift, and marketplace states are explicit,
  - paid plans and paid capabilities require Stripe or custom-contract
    classification,
  - frontline family eligibility denies raw proof documents and requires
    deterministic metadata,
  - marketplace templates deny missing scope, plan gate, consent, audit, or
    disablement declarations.

## Boundaries

- No raw payment secrets, eligibility documents, or sensitive profile data are
  permitted in fixtures, metadata, or generated artifacts.
- No entitlement behavior claims EXOCHAIN enforcement or production trust
  verification.
- Gift, frontline, and marketplace monetization policy remain owner-controlled;
  this contract models safe state and denial behavior only.
