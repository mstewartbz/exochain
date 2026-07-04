# Phase 9 Enterprise Onboarding And Commercial Architecture

## Source Basis

- Source: Bob Stewart current-thread product direction on 2026-05-24.
- Scope: enterprise-class LiveSafe onboarding, medical-jacket ownership,
  bailed custody, PACE social contract, marketplace capability expansion,
  freemium-to-paid revenue model, gift subscriptions, and free frontline
  eligibility.
- Classification: user-stated product doctrine and requirements input.
- Sensitive material handling: this record contains no raw medical, genetic,
  contact, identity, payment, or emergency data.

## Fact vs Inference

- Fact: Bob states that LiveSafe must remain enterprise-class, not a small
  emergency-card utility.
- Fact: Bob frames the onboarding mechanism as the foundation that leads users
  to build a full medical jacket as part of their profile.
- Fact: Bob states the medical jacket should help users take back legal control
  of medical records through bailed control and custody.
- Fact: Bob distinguishes phenotypical records, meaning medical records, from
  genotypical profile data that may be imported later from other services.
- Fact: Bob states that the medical and genetic profile direction should support
  later sourcing of precision-medicine clinical-trial opportunities.
- Fact: Bob identifies P.A.C.E. contact selection as an intrinsic social
  contract that strengthens 0dentity scoring.
- Fact: Bob frames P.A.C.E. as a 1:4 viral spread coefficient with low churn
  once human obligations are accepted.
- Fact: Bob plans significant marketplace templates that incent users to add
  rule-driven capabilities using Syntaxis, Decision Forum, and related systems.
- Fact: Bob identifies examples ranging from simple golden-hour outreach to
  complex multi-family disaster and emergency-preparedness PACE plans.
- Fact: Bob states that Ambient.li shows additional capability patterns to
  consider.
- Fact: Bob identifies the simplified onboarding wizard and
  invitation-acceptance/onboarding as the most valuable product mechanism.
- Fact: Bob states that basic accounts are free.
- Fact: Bob states that family plans, team plans, and advanced capabilities are
  paid or trial-gated as they are released.
- Fact: Bob identifies immediate revenue through monthly recurring revenue using
  Stripe, freemium conversion, free trials, paid feature gates, and gift
  subscriptions.
- Fact: Bob states that basic family plans should be free for firefighters,
  EMTs, law-enforcement officers, front-line NIMS workers, hospital staff,
  active-duty and reserve military, tactical workers, intelligence workers, and
  press operatives.
- Inference: LiveSafe needs an entitlement model before product automation can
  safely implement paid plans, free trials, frontline eligibility, gift
  subscriptions, and marketplace add-ons.
- Inference: LiveSafe needs separate consent, custody, provenance, and data
  class boundaries for phenotypical medical records and genotypical profile
  imports.
- Inference: the onboarding wizard must be treated as a core product engine
  with acceptance-state tracking, not merely a form.
- Inference: marketplace templates should be modeled as governed capability
  packs with explicit rules, scopes, plan gates, and audit behavior.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| Phase 9 product doctrine | current-chat source | Codex thread, 2026-05-24 | enterprise onboarding, medical jacket, PACE, 0dentity, marketplace, Stripe, free frontline plans | Establishes LiveSafe's enterprise-class product and revenue architecture | high | preserve |
| Existing Phase 8 card record | canon record | `/Users/bobstewart/dev/livesafe/context/canon/2026-05-24-phase-8-ice-card-images.md` | physical ICE card, QR, legal/medical panels, foldable packet | Physical onboarding artifact that feeds the enterprise onboarding path | high | connect |
| Automation readiness doc | readiness doc | `/Users/bobstewart/dev/livesafe/docs/LIVESAFE_AUTOMATION_READINESS.md` | bounded automation, control records, slice map | Must incorporate enterprise-class and commercial-readiness constraints | high | update |

## Requirements Captured

- The product must be designed as an enterprise-class personal safety mesh and
  medical-record custody platform, not only an emergency card generator.
- Onboarding must progressively lead users from basic safety setup into a full
  medical jacket.
- The medical jacket must support user-controlled custody of phenotypical
  medical records.
- The architecture must leave room for genotypical profile imports from external
  services, with separate consent and data-class controls.
- Precision-medicine clinical-trial opportunity matching must be treated as a
  later capability that depends on explicit consent, data classification,
  matching rules, and opt-in state.
- P.A.C.E. contact setup must be a required or strongly guided onboarding
  pathway because it drives safety value, 0dentity scoring, and viral network
  growth.
- P.A.C.E. invitations must track invite, acceptance, obligation, role,
  replacement, revocation, and churn-prevention states.
- The onboarding wizard must minimize friction for the primary user and for
  invitees accepting P.A.C.E. roles.
- Marketplace templates must support simple golden-hour outreach, family
  readiness, team readiness, disaster planning, and more complex governed
  Decision Forum or Syntaxis rule packs.
- Basic individual accounts must be free.
- Family plans, team plans, advanced capabilities, marketplace templates, and
  feature drips must be gateable through entitlements.
- Stripe must be the billing rail for monthly recurring revenue, trials,
  upgrades, gifts, family plans, team plans, and add-ons.
- Gift subscriptions must be a first-class commercial flow.
- Frontline and service-worker eligibility must support free basic family plans
  for the qualifying cohorts named by Bob.
- Free frontline eligibility must be verifiable without requiring raw sensitive
  documents to be stored in the repo or exposed in logs.

## Product Architecture Impact

- LiveSafe automation readiness must include commercial infrastructure, not only
  safety contracts.
- The first implementation map should include onboarding wizard contracts,
  P.A.C.E. invitation contracts, entitlement contracts, and marketplace template
  contracts near the top of the slice list.
- Medical-jacket custody should be separated from emergency-card display:
  emergency access projects a minimal authorized view, while the full profile
  remains under user-controlled custody.
- Phenotypical and genotypical data must be separately classified in any data
  model, consent model, custody receipt, export, or matching feature.
- Marketplace templates should compile into explicit rule sets and plan gates
  rather than becoming ad hoc feature flags.
- Eligibility programs for frontline cohorts are a commercial policy surface and
  require deterministic entitlement tests before launch.

## Open Conflicts

- The exact legal structure for bailed control and custody remains unstated in
  local source files.
- The exact phenotypical medical-record schema and genotypical import schema are
  not yet located.
- The precision-medicine clinical-trial matching workflow is strategic product
  direction, not an implemented contract in this repo.
- The exact Stripe product catalog, trial policy, gift subscription terms,
  family plan limits, team plan limits, and advanced capability pricing are not
  specified.
- The exact eligibility proof process for free frontline/basic family plans is
  not specified.
- The exact relationship among marketplace templates, Syntaxis, Decision Forum,
  Ambient.li, and EXOCHAIN runtime verification remains open.
