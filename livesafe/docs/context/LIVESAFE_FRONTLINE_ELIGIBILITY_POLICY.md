# LiveSafe Frontline Eligibility Policy

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`
- `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`
- `context/canon/2026-05-24-phase-9-enterprise-onboarding-commercial-architecture.md`
- Bob Stewart direct instruction on 2026-05-26: Heroes includes any
  first-responder, LEOs, Fire & Rescue, Emergency Room personnel, powerline
  workers, FEMA, sheriffs, and NIMS personnel.
- `src/entitlement_marketplace.rs`
- `tests/entitlement_marketplace.rs`

## Ground Truth

LiveSafe currently models frontline eligibility as an adjacent-surface
commercial entitlement rule, not as a verified public-benefit program or
runtime identity proof system. The current repo truth is the entitlement
contract in `src/entitlement_marketplace.rs` and the denial/allow tests in
`tests/entitlement_marketplace.rs`.

The contract supports `EntitlementPlan::FrontlineBasicFamily` and restricts
eligibility to `FrontlineVerificationMethod::DeterministicMetadata`. The same
contract explicitly denies raw-document storage through
`stores_raw_proof_document`.

This policy does not activate EXOCHAIN authority, does not permit raw proof
documents, and does not define a live verification vendor, reviewer workflow,
or production adjudication process.

## Qualifying Cohorts

The current qualifying cohorts are now treated as the LiveSafe Heroes group.
They come from Bob's source direction in
`context/canon/2026-05-24-phase-9-enterprise-onboarding-commercial-architecture.md`
plus direct Bob clarification on 2026-05-26 and the executable enum in
`src/entitlement_marketplace.rs`:

| Canon cohort language | Contract code |
| --- | --- |
| firefighters | `FrontlineCohort::Firefighter` |
| EMTs | `FrontlineCohort::Emt` |
| law-enforcement officers | `FrontlineCohort::LawEnforcement` |
| sheriffs | `FrontlineCohort::Sheriff` |
| Emergency Room personnel | `FrontlineCohort::EmergencyRoomPersonnel` |
| hospital staff | `FrontlineCohort::HospitalStaff` |
| FEMA responders | `FrontlineCohort::FemaResponder` |
| front-line NIMS workers | `FrontlineCohort::NimsWorker` |
| powerline or utility emergency workers | `FrontlineCohort::PowerlineUtilityWorker` |
| active-duty military | `FrontlineCohort::ActiveDutyMilitary` |
| reserve military | `FrontlineCohort::ReserveMilitary` |
| tactical workers | `FrontlineCohort::TacticalWorker` |
| intelligence workers | `FrontlineCohort::IntelligenceWorker` |
| press operatives | `FrontlineCohort::PressOperative` |

No additional cohort classes are source-backed in this repo today. Expanding,
renaming, or removing Heroes cohorts requires owner review because it changes
the commercial policy surface.

## Deterministic Eligibility Metadata

The current frontline plan can be represented only with deterministic
metadata. In repo truth, that means:

- `frontline_eligibility.cohort` must be present.
- `frontline_eligibility.verification_method` must be
  `DeterministicMetadata`.
- `frontline_eligibility.evidence_ref` must contain a synthetic metadata
  reference.
- `stores_raw_proof_document` must remain `false`.

`tests/entitlement_marketplace.rs` proves both edges:

- missing cohort or metadata evidence is denied,
- `FrontlineBasicFamily` is allowed when `DeterministicMetadata` is present,
- raw-document storage is denied.

This keeps eligibility state deterministic for tests and synthetic fixtures
while preserving the repo boundary against sensitive identity or employment
records.

## Disallowed Proof Handling

LiveSafe currently supports no raw proof documents for frontline eligibility.
The following are out of bounds for this repo, fixtures, logs, and generated
artifacts:

- scans or photos of badges, IDs, pay stubs, or licenses,
- employment letters or agency rosters,
- uploads containing direct identity, contact, or location evidence,
- manual-review notes that reproduce raw proof contents,
- any fixture or metadata path that stores raw eligibility documents.

The current contract also treats `ManualReview` and `RawDocument` as
non-qualifying methods for the frontline family entitlement. Those enum cases
exist as boundary markers, not as approved implementation paths.

## Entitlement And Runtime Boundary

- Path classification: adjacent surface documentation and domain-contract
  mapping.
- Source-backed implementation basis:
  `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`,
  `src/entitlement_marketplace.rs`, and `tests/entitlement_marketplace.rs`.
- Runtime posture: inactive beyond deterministic contract state; there is no
  live verification route, vendor integration, or public trust claim.
- Billing posture: the frontline benefit is modeled as
  `FrontlineBasicFamily`, but exact Stripe configuration remains outside this
  document.
- Disablement path: deny any frontline entitlement request that lacks
  deterministic metadata, includes raw proof documents, or depends on an
  unwired manual-review or raw-document path.

## Bob-Only Escalation Boundary

`docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md` marks frontline cohort
eligibility proof policy as owner-only. That means this automation may model
safe deterministic metadata and no raw proof documents, but it must not invent
or activate:

- a live proof collection process,
- a human review policy,
- accepted production evidence types,
- retention periods for proof artifacts,
- fraud, appeals, or revocation handling beyond current contract denial,
- billing offsets or reimbursement policy tied to frontline eligibility.

Until Bob sets that live policy, the safe repo default is to keep frontline
eligibility at the deterministic contract layer only and fail closed outside
that boundary.
