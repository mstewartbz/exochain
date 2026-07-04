# LiveSafe Human Safety Opportunity Model

## Status

Inception control document for the LiveSafe.ai human safety mesh.

This document reserves the product-opportunity doctrine that should guide the
first public expression of LiveSafe:

> Create your card. Invite your four. Protect your people.

This is a source-backed control document, not marketing copy. It exists to keep
Codex, human reviewers, product decisions, and future white-paper work aligned
around the smallest immediate safety loop before expanding into medical-jacket
custody, VitalLock vault depth, Ambient context, marketplace templates, or
EXOCHAIN-root-backed production claims.

## Source Basis

Current repo/source basis:

- `AGENTS.md`
- `README.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_TO_EXOCHAIN_INTEGRATION_MAP.md`
- `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md`
- `docs/context/LIVESAFE_ONBOARDING_AND_PACE_GROWTH_MODEL.md`
- `docs/context/LIVESAFE_EMERGENCY_PROFILE_MODEL.md`
- `docs/context/LIVESAFE_MEDICAL_JACKET_AND_CUSTODY_MODEL.md`
- `docs/context/LIVESAFE_QR_ACTIVATION_MODEL.md`
- `docs/context/LIVESAFE_VITALLOCK_VAULT_MODEL.md`
- `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`
- `src/onboarding_pace.rs`
- `src/emergency_profile.rs`
- `src/medical_jacket_custody.rs`
- `src/qr_pointer.rs`
- `src/vitallock_vault.rs`
- `src/entitlement_marketplace.rs`
- `src/consent_revocation_receipt.rs`
- `src/pace_readiness_grant.rs`, if present
- `docs/context/LIVESAFE_PACE_READINESS_GRANT_MODEL.md`, if present
- `docs/context/LIVESAFE_SAFETY_CIRCLE_INVITATION_LANGUAGE_MODEL.md`, if present

Current external/historical source basis:

- VitalLock source material: secure private emergency messaging, scheduled
  delivery, emergency contacts, PEBS alerts, delivery/read confirmation.
- LiveSafe marketspace research: fragmented emergency ID, caregiver, PHR,
  medical-vault, clinical-trial, responder, and family-safety markets.
- LiveSafe / EXOCHAIN context inventories: LiveSafe, VitalLock, ICE card,
  Ambient, and EXOCHAIN lineage map.
- Safety Circle / P.A.C.E. Readiness Grant conversation: reward readiness, not
  referral; recognize obligation, not traffic; complete the circle, then grant
  abundance.

## Ground Truth

LiveSafe is not merely an emergency card app and not merely a medical-record
vault. LiveSafe is a human safety mesh: a product surface where a person can
make themselves legible to trusted humans and appropriate responders when they
cannot speak, act, remember, or coordinate.

The first public loop must be:

1. Create a usable emergency card.
2. Invite the four P.A.C.E. humans.
3. Protect the people who would need to act.

The first loop must not require:

- full medical-jacket completion,
- genetic-data import,
- clinical-trial matching,
- EXOCHAIN production trust claims,
- formal responder agency adoption,
- legal/medical directive generation,
- raw sensitive data in fixtures, logs, anchors, or QR payloads.

## Product Doctrine

### Human safety before market abstraction

Do not rank opportunities only by revenue. Prioritize segments that combine:

- immediate human safety urgency,
- willingness to complete the card/P.A.C.E. loop,
- need for family or team continuity,
- potential to create accepted P.A.C.E. obligations,
- readiness to carry or save an emergency artifact,
- ability to validate without raw medical/genetic payloads or unsupported trust claims.

### Card before custody

The emergency card is the first artifact of care. It must work as a simple,
legible, revocable pointer before deeper vault or EXOCHAIN activation exists.

### P.A.C.E. before platform sprawl

P.A.C.E. is the first growth graph and the first trust graph. It is not merely
a contact list. A P.A.C.E. contact accepts a social-contract obligation for a
specific role.

### Protected node before generic user

The most important early unit is the protected node:

```text
Protected Node =
  active card owner
  or dependent profile
  or accepted P.A.C.E. contact
  or frontline-family member
  or team member with readiness state
```

### Readiness before monetization

Revenue matters, but the early product should be measured by whether it creates
readiness:

```text
Human Continuity Activation =
  active emergency card
  + emergency core profile completion
  + accepted P.A.C.E. obligations
  + review freshness
  + family/team readiness state
```

### Trust claims follow runtime truth

EXOCHAIN/root evidence, LiveSafe adapter verification, and public production
trust claims are separate gates. Root evidence does not automatically activate
LiveSafe public claims. Adapter proof and fail-closed runtime evidence remain
decisive.

## Fact / Inference Separation

### Facts

- The repo already models P.A.C.E. roles, contact acceptance states, obligation
  acceptance, and onboarding progression.
- The repo already models approved emergency-profile fields and denies raw
  payloads, direct contact data, location traces, and QR secrets in profile
  metadata.
- The repo already separates phenotypical medical records from genotypical
  imports and keeps trial matching inactive until explicit activation and
  opt-in conditions are met.
- The repo already treats QR/NFC data as pointer metadata and denies raw
  sensitive payloads.
- The repo already models VitalLock vault interaction as a fail-closed,
  metadata-safe, responder/delegate-gated surface.
- The repo already models Basic Free, Family Paid, Team Paid, Frontline Basic
  Family, paid capabilities, and marketplace template gates.
- The repo may now also model the P.A.C.E. Readiness Grant, which recognizes a
  completed Safety Circle without turning the ask into referral-bounty logic.

### Inferences

- The strongest year-one beachhead is not a generic emergency-card user; it is a
  safety loop across caregivers, high-risk individuals, frontline families, and
  small teams.
- The product-market signal is not first-year TAM; it is card completion,
  P.A.C.E. invitation, P.A.C.E. acceptance, dependent profile creation, and
  family/team readiness.
- A 1:4 spread coefficient should be treated as a design target, not a forecast.
- EXOCHAIN-root-backed public claims may become powerful later, but should not
  lead year-one adoption language.
- The Safety Circle Completion Grant can reinforce readiness if framed as
  gratitude for completed obligation, not as payment for registrations.

## Year-One Opportunity Prioritization

Codex and product reviewers should prioritize opportunities that satisfy all
of the following:

1. Supports the immediate loop:
   - create card,
   - invite four,
   - protect people.
2. Can be tested with synthetic fixtures and no raw sensitive data.
3. Does not require unsupported responder adoption.
4. Does not require EXOCHAIN public trust claims.
5. Does not require genetic import or trial matching.
6. Produces measurable readiness.
7. Can become a family, team, frontline, or marketplace path later.
8. Preserves the dignity and autonomy of P.A.C.E. invitees.

## Segment Hypotheses

### Highest-priority segments

1. Caregivers of medically complex dependents.
2. Adults with severe allergies, seizures, diabetes, implants, anticoagulants,
   transplant/immunosuppression, or other high-risk conditions.
3. Sandwich-generation caregivers and elder/dementia/wandering-risk households.
4. Frontline families: fire, EMS, ER, hospital, military, tactical, intelligence,
   press, powerline, FEMA/NIMS, and similar users.
5. Small teams with duty-of-care needs: camps, schools, churches, sports clubs,
   field teams, volunteer groups, tactical nonprofits.

### Lower-priority or later segments

- Generic wellness consumers without immediate safety urgency.
- Genetic-data owners without emergency/family need.
- Clinical-trial matching users before custody maturity.
- Enterprise buyers who require institutional responder integration before
  proving family/team readiness.
- Any segment that requires medical, legal, or EXOCHAIN enforcement claims
  before the product can truthfully support them.

## Metrics That Matter

Track these before building elaborate financial forecasts:

| Metric | Why it matters |
| --- | --- |
| `card_start_count` | Measures whether immediacy is compelling. |
| `card_created_count` | Measures completion of the first safety artifact. |
| `card_saved_or_printed_count` | Measures whether the artifact leaves the app. |
| `pace_invites_sent_count` | Measures trust-loop activation. |
| `pace_invites_accepted_count` | Measures real human obligation acceptance. |
| `accepted_contact_profile_created_count` | Measures P.A.C.E. spread. |
| `dependent_profiles_created_count` | Measures family/caregiver activation. |
| `core_profile_completed_count` | Measures useful emergency data completeness. |
| `review_fresh_count` | Measures data freshness and readiness. |
| `family_or_team_intent_count` | Measures monetizable continuity need. |
| `safety_circle_completed_count` | Measures full P.A.C.E. circle completion. |
| `readiness_grant_issued_count` | Measures completion recognition without referral-bounty framing. |

## Readiness Formulas

Use integer arithmetic only.

```text
Card Creation Rate BPS =
  card_created_count * 10_000 / max(card_start_count, 1)

Card Carry-Forward Rate BPS =
  card_saved_or_printed_count * 10_000 / max(card_created_count, 1)

P.A.C.E. Invite Rate BPS =
  card_owners_with_one_or_more_invites * 10_000 / max(card_created_count, 1)

P.A.C.E. Acceptance Rate BPS =
  pace_invites_accepted_count * 10_000 / max(pace_invites_sent_count, 1)

P.A.C.E. Spread Coefficient Milli =
  average_invites_per_card_milli
  * acceptance_rate_bps
  * accepted_contact_profile_creation_rate_bps
  / 100_000_000

Safety Circle Completion Rate BPS =
  safety_circle_completed_count * 10_000 / max(card_created_count, 1)

Readiness Grant Issuance Rate BPS =
  readiness_grant_issued_count * 10_000 / max(safety_circle_completed_count, 1)

Human Continuity Activation Score =
  weighted integer score across:
  - card created,
  - saved or printed,
  - core emergency profile completed,
  - at least two P.A.C.E. accepted,
  - full Safety Circle completed,
  - review fresh,
  - dependent/family/team readiness present.
```

## Opportunity Scoring Doctrine

A year-one opportunity should score well on:

- urgency,
- onboarding willingness,
- P.A.C.E. spread potential,
- family/team need,
- willingness to pay.

It should be penalized for:

- compliance risk,
- implementation complexity,
- requiring raw sensitive data before card/P.A.C.E.,
- requiring unsupported responder claims,
- requiring EXOCHAIN/root-backed public trust claims,
- requiring genetic or trial matching before custody maturity.

## Safety Circle Completion Grant Boundary

The Safety Circle Completion Grant may support the opportunity model if it
follows this rule:

```text
Exactly four distinct accepted P.A.C.E. roles
+ each role obligation accepted
+ each role notification-eligible
+ each invitee able to decline or revoke
+ no duplicate/self-grant/contact collapse
+ no raw contact data in reward metadata
+ reward framed as readiness grant
= 4 months of Plus
```

Allowed phrase:

```text
Complete your Safety Circle and receive 4 months of Plus.
```

Disallowed phrase:

```text
Get four friends to sign up and get free premium.
```

The reward recognizes completed readiness. It does not buy the relationship.

## Build Boundaries

Do not use this opportunity model to justify:

- raw sensitive fixtures,
- QR payloads containing PHI/PII/contact/location data,
- full medical-jacket emergency disclosure,
- genetic import in the emergency loop,
- clinical-trial matching in MVP,
- public EXOCHAIN/root trust claims before activation gates,
- responder-agency acceptance claims,
- medical/legal directive validity claims,
- live SMS/email/push agent dispatch without disabled-by-default gates,
- referral-bounty framing for P.A.C.E. invitations,
- leaderboards or public social scoring for Safety Circle completion.

## Recommended Inception Slice

Add or reconcile the following bounded slice:

```text
LIVESAFE_HUMAN_SAFETY_OPPORTUNITY_MODEL
```

Allowed paths:

- `docs/context/LIVESAFE_HUMAN_SAFETY_OPPORTUNITY_MODEL.md`
- `docs/whitepapers/LIVESAFE_CREATE_CARD_INVITE_FOUR_PROTECT_PEOPLE.md`
- `src/human_safety_opportunity.rs`
- `tests/human_safety_opportunity.rs`
- `src/lib.rs`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md`

Validation:

- `cargo test --test human_safety_opportunity`
- `npm run quality`

## Disablement And Rollback

This document does not activate runtime behavior, public trust claims, billing,
EXOCHAIN writes, responder disclosure, medical-jacket disclosure, genetic import,
trial matching, reward issuance, or agent dispatch.

Rollback path:

- remove `src/human_safety_opportunity.rs`,
- remove `tests/human_safety_opportunity.rs`,
- remove `pub mod human_safety_opportunity;` from `src/lib.rs`,
- remove this control doc and white-paper frame,
- remove any slice-map/test-plan references.
