# LiveSafe Medical Jacket And Custody Model

## Source Basis

- `context/canon/2026-05-24-phase-9-enterprise-onboarding-commercial-architecture.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `src/medical_jacket_custody.rs`
- `tests/medical_jacket_custody.rs`

## Product Rule

LiveSafe medical-jacket contracts must classify phenotypical records separately
from genotypical imports, require explicit consent scopes, keep raw sensitive
payloads out of fixtures and metadata, restrict emergency projection to
authorized phenotypical records, and keep precision-medicine trial matching
inactive until explicit opt-in and eligibility contracts pass.

## Domain Terms

- `Phenotypical`: medical-jacket record classes representing medical-record
  custody material.
- `GenotypicalImport`: externally sourced genetic-profile imports that require
  separate import consent and source references.
- `Custody`: consent state that permits custody evaluation for a record.
- `EmergencyProjection`: consent state that permits minimal emergency display
  for an authorized phenotypical record.
- `TrialMatching`: later capability that remains inactive until explicit opt-in,
  data-class validation, and eligibility-contract proof exist.

## Implemented Contract

The medical-jacket custody contract now enforces:

- synthetic record references and encrypted blob references for every record,
- no raw sensitive payloads in fixtures or metadata,
- explicit custody consent for every record under evaluation,
- custody receipt references for phenotypical records,
- separate import consent and external-source references for genotypical
  imports,
- emergency projection limited to phenotypical records with
  `EmergencyProjection` consent,
- trial matching denial unless activation, opt-in, data-class validation, and
  eligibility-contract proof all pass.

## Test Evidence

- `cargo test --test medical_jacket_custody`
- `npm run quality`

## Boundaries

- Classification: adjacent surface domain contract.
- Trust posture: inactive; no EXOCHAIN runtime enforcement, custody proof, or
  public trust claim.
- Data posture: synthetic references only; no raw medical, genetic, identity,
  contact, or location payloads.
- Disablement path: keep `src/medical_jacket_custody.rs` unwired from runtime
  routes or remove the `medical_jacket_custody` export from `src/lib.rs`.
