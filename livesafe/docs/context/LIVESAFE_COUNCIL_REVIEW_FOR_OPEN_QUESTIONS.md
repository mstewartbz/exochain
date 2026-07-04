# LiveSafe Council Review For Open Questions

## Source Basis

- `AGENTS.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_CONTEXT_SEED.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md`
- `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`
- `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md`

## Ground Truth

LiveSafe automation is allowed to keep building from source-backed defaults and
repo truth unless a Bob-only escalation applies. The repo already defines the
primary escalation list in `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`, but
the automation also needs a durable control record that says which unresolved
questions can be answered by local evidence versus which questions must stop
and go to Bob.

This document is that boundary. It applies only to the adjacent surface in
`/Users/bobstewart/dev/livesafe`. It does not authorize EXOCHAIN core edits,
public trust claims, raw sensitive data handling, or commercial-policy guesses.

## Automation-Resolvable Questions

The automation may resolve an open question by repo truth and source-backed
defaults when all of the following are true:

- the answer can be derived from committed files, current deployment evidence,
  or the live health route without inventing facts,
- the slice can be implemented with synthetic fixtures only,
- the result does not require owner-only brand, legal, medical, pricing, or
  billing decisions, and
- the result keeps public trust posture inactive unless
  `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md` is satisfied.

Questions that default to automation resolution include:

- which documented file, test, route, contract, or deployment record is the
  current repo truth,
- whether a missing control document can be created from existing source basis,
- whether a product rule can be expressed as an adjacent Rust or TypeScript
  contract using synthetic fixtures,
- whether a route, policy, or display must fail closed because proof or adapter
  evidence is missing,
- whether runtime or documentation drift should be recorded from current repo
  evidence, and
- whether a proposed slice stays inside AGENTS, test-plan, and no-sensitive-data
  boundaries.

## Bob-Only Questions

The automation must stop and escalate when a question touches owner authority,
current legal text, live commercial policy, or raw sensitive data. The current
Bob-only questions are:

- `Public brand commitment among LiveSafe, ExoSafe, VitalLock, InCaseOfEmergencyCard, and Ambient`
- `Current legal, medical, privacy, or card-back terms text`
- `Live first-responder disclosure scope`
- `Frontline cohort eligibility proof policy`
- `Stripe product and price identifiers`
- `Gift subscription commercial rules`
- `Marketplace template monetization rules`
- `Use of raw imported medical, genetic, identity, contact, or eligibility data`
- `Any EXOCHAIN core modification`
- `Public release of proprietary architecture or transfer artifacts`

If a slice cannot be completed without answering one of those questions, the
automation must stop at the nearest clean validation point and report the
narrowed escalation rather than infer an answer.

## Default Resolution Rules

When an unresolved question is not Bob-only, the automation should use these
defaults:

1. Prefer repo truth over memory, summaries, or historical intent.
2. Prefer current source-backed defaults over invention.
3. Prefer inactive trust state over implied authority.
4. Prefer synthetic fixtures over imported real-world data.
5. Prefer fail-closed denial over permissive behavior when proof is missing.
6. Prefer documented deployment evidence over stale repo-era assumptions.

Applied defaults in current repo state:

- Trust-bearing output remains non-verified unless the activation gates pass.
- EXOCHAIN claims remain adjacent and inactive because no verified runtime
  adapter path is wired.
- Storage, onboarding, emergency, AI, and entitlement slices use synthetic
  fixtures and bounded contract coverage.
- Railway production health evidence may inform runtime drift, but not justify
  trust or adapter claims.

## Disablement And Rollback

- Path classification: adjacent surface documentation state.
- Runtime exposure added by this document: none.
- Disablement path: continue using source-backed defaults and fail-closed
  behavior; if a future change conflicts with this document, deny the slice and
  escalate when the question becomes Bob-only.
- Rollback path: revert this document and any linked slice-map or readiness
  updates if the recorded boundary no longer matches repo truth.
