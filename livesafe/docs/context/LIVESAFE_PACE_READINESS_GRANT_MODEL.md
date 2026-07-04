# LiveSafe P.A.C.E. Readiness Grant Model

## Status

Inception control document for the LiveSafe Safety Circle Completion Grant.

This document defines how LiveSafe may recognize a completed P.A.C.E. circle
without reducing a sacred emergency-contact social contract into a referral
bounty.

Primary doctrine:

> Reward readiness, not referral. Recognize obligation, not traffic. Complete
> the circle, then grant abundance.

Public-facing completion phrase:

> Complete your Safety Circle and receive 4 months of Plus.

## Source Basis

Repo/source basis:

- `AGENTS.md`
- `README.md`
- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_ONBOARDING_AND_PACE_GROWTH_MODEL.md`
- `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`
- `docs/context/LIVESAFE_HUMAN_SAFETY_OPPORTUNITY_MODEL.md`
- `src/onboarding_pace.rs`
- `src/entitlement_marketplace.rs`
- `tests/onboarding_pace.rs`
- `tests/entitlement_marketplace.rs`

Conversation/source basis:

- LiveSafe first-loop doctrine: “Create your card. Invite your four. Protect
  your people.”
- P.A.C.E. as a high-trust social contract, not a generic referral list.
- Circle completion incentive proposal: four accepted P.A.C.E. contacts can
  unlock four months of Plus.
- Incentive-design guidance: preserve autonomy, relatedness, consent, and the
  ability to decline or revoke without penalty.

## Ground Truth

P.A.C.E. is not merely a growth tactic.

P.A.C.E. means:

- Primary
- Alternate
- Contingent
- Emergency

Each P.A.C.E. role is an accepted human obligation. The product may recognize a
complete circle, but it must never make the invitee feel purchased, coerced, or
used as a lead.

The reward is granted to honor completion of a safety structure. It is not paid
per invite and not paid per signup.

## Relationship To Human Safety Opportunity

`docs/context/LIVESAFE_HUMAN_SAFETY_OPPORTUNITY_MODEL.md` is the parent
opportunity doctrine for the first public loop: create the emergency card,
invite the four P.A.C.E. humans, and protect the people who would need to act.

This P.A.C.E. Readiness Grant model is narrower. It does not supersede the
human-safety opportunity model; it implements one child incentive boundary for
recognizing a completed Safety Circle without turning the P.A.C.E. ask into
referral-bounty logic.

## Naming

Preferred product terms:

- Safety Circle
- Circle Strength
- Readiness Grant
- Safety Circle Completion Grant
- P.A.C.E. role acceptance
- P.A.C.E. obligation
- Protection circle

Avoid:

- referral bounty
- invite bounty
- viral reward
- lead reward
- get friends to sign up
- earn free months by recruiting
- gamification contest
- leaderboard
- streak
- score bragging

## Reward Rule

The canonical completion grant is:

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

The reward should not be granted for:

- invitations sent but not accepted,
- account registrations alone,
- duplicate contacts,
- self-grants,
- revoked, declined, replaced, or stale roles,
- fake or disposable contact channels,
- raw-contact-data-bearing metadata,
- unsupported responder or EXOCHAIN claims,
- coercive or penalty-based invite copy.

## Circle Strength Language

Circle strength may be shown as private readiness state:

| State | Meaning |
| --- | --- |
| `Not Started` | No P.A.C.E. roles accepted. |
| `Forming` | One or more roles accepted, but the circle is incomplete. |
| `Almost Complete` | Three roles accepted and notification-eligible. |
| `Complete` | All four roles accepted, obligation-accepted, distinct, and notification-eligible. |
| `Complete + Fresh` | Circle complete and recently reviewed. |
| `Blocked` | Circle contains self-grant, duplicate role/contact, raw-data metadata, unsupported claim, or coercive reward framing. |

Do not expose public leaderboards or comparative rankings.

## User Experience

### Subscriber view

```text
Your Safety Circle is forming.

Primary: Accepted
Alternate: Accepted
Contingent: Waiting
Emergency: Waiting

When all four trusted people accept their P.A.C.E. roles, your Safety Circle is
complete. We’ll add 4 months of Plus as a readiness grant.
```

### Completion view

```text
Your Safety Circle is complete.

Four trusted humans accepted their P.A.C.E. roles. Your people are more ready
to act if you cannot speak for yourself.

We’ve added 4 months of Plus as a readiness grant.
```

### Invitee transparency

```text
When Bob’s full P.A.C.E. circle is complete, LiveSafe may grant Bob a readiness
credit. Your decision should be based only on whether you are willing to serve
in this role. You can decline or revoke later.
```

## Design Principles

### Autonomy

The invitee must be able to accept, decline, request a change, or later revoke
without penalty.

### Relatedness

The invitation should frame the ask as human trust and care:

> Bob is asking whether you are willing to be one of four trusted people who may
> be notified if Bob’s LiveSafe emergency card is scanned or if Bob cannot speak
> for himself.

### Competence

The role should be understandable. The invitee should know:

- what role they were asked to accept,
- what they may receive,
- what they will not receive,
- how to update availability,
- how to revoke.

### Non-extraction

The invitee is not a lead. The invitee is a newly protected node and may create
their own free LiveSafe Basic profile.

## Honor Good Receipt Model

Future receipt classes may include:

- `PaceRoleAcceptedReceipt`
- `SafetyCircleCompletedReceipt`
- `ReadinessGrantIssuedReceipt`
- `SafetyCircleReviewedReceipt`
- `PaceRoleRevokedReceipt`

These receipts should remain private or subject-controlled unless explicitly
shared. They must not contain raw contact values, medical data, genetic data,
location traces, QR tokens, vault contents, or sensitive identity payloads.

## Data Boundary

Reward/grant metadata may contain:

- subscriber reference,
- contact reference,
- P.A.C.E. role,
- acceptance state,
- obligation accepted state,
- notification eligibility state,
- grant-month count,
- policy reference,
- audit reference,
- synthetic receipt reference.

Reward/grant metadata must not contain:

- raw names,
- phone numbers,
- email addresses,
- location traces,
- emergency medical fields,
- medical jacket payloads,
- genetic payloads,
- QR/NFC secrets,
- identity documents,
- payment data,
- unsupported EXOCHAIN trust claims.

## Commercial Boundary

A 4-month grant is a retention and readiness mechanism, not a discounting
identity. It should be attached to Safety Circle completion, not to traffic or
registration volume.

Allowed:

```text
Complete your Safety Circle and receive 4 months of Plus.
```

Disallowed:

```text
Get four friends to sign up and get free premium.
```

## Implementation Slice

Slice name:

```text
LIVESAFE_PACE_READINESS_GRANT_MODEL
```

Allowed paths:

- `docs/context/LIVESAFE_PACE_READINESS_GRANT_MODEL.md`
- `docs/context/LIVESAFE_SAFETY_CIRCLE_INVITATION_LANGUAGE_MODEL.md`
- `docs/collateral/*`
- `docs/whitepapers/LIVESAFE_SAFETY_CIRCLE_COMPLETION_GRANT_FRAME.md`
- `docs/codex_prompts/APPLY_LIVESAFE_PACE_READINESS_GRANT_SLICE.md`
- `src/pace_readiness_grant.rs`
- `tests/pace_readiness_grant.rs`
- `src/lib.rs`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md`

Validation:

- `cargo test --test pace_readiness_grant`
- `npm run quality`

## Disablement And Rollback

This document does not activate runtime reward issuance, Stripe, dispatch,
public claims, or EXOCHAIN writes.

Rollback path:

- remove `src/pace_readiness_grant.rs`,
- remove `tests/pace_readiness_grant.rs`,
- remove `pub mod pace_readiness_grant;` from `src/lib.rs`,
- remove linked control docs/collateral,
- remove slice-map and test-plan references.
