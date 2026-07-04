# LiveSafe Onboarding And P.A.C.E. Growth Model

## Source Basis

- Bob's 2026-05-24 and 2026-05-25 direction on enterprise onboarding,
  P.A.C.E. contacts, 0dentity scoring, 1:4 growth, medical jacket control,
  frontline eligibility, family/team plans, gift subscriptions, and marketplace
  capabilities.
- `docs/LIVESAFE_AUTOMATION_READINESS.md`.
- `docs/TEST_PLAN.md`.
- `src/onboarding_pace.rs`.
- `tests/onboarding_pace.rs`.

## Product Rule

The onboarding flow must lead a basic user from account creation to emergency
card setup, P.A.C.E. invitations, medical jacket progression, and entitlement
selection without exposing raw sensitive data.

P.A.C.E. contacts are a social-contract mechanism. They must be represented as
role-bearing references and acceptance states before they can be used for
notification eligibility, growth loops, 0dentity scoring inputs, or recovery
workflows.

## Domain Terms

| Term | Meaning |
| --- | --- |
| Primary | First P.A.C.E. role in the subscriber's trust set |
| Alternate | Alternate P.A.C.E. role |
| Contingent | Contingent P.A.C.E. role |
| Emergency | Emergency P.A.C.E. role |
| Accepted | Contact accepted the invitation and social-contract obligation |
| Revoked | Contact no longer participates |
| Replaced | Contact was superseded by another contact reference |
| Notification eligible | Accepted, obligation accepted, and not replaced |

## Implemented Contract

The Rust contract in `src/onboarding_pace.rs` enforces:

- no self-grant by using distinct subscriber and contact references,
- one Primary, Alternate, Contingent, and Emergency role,
- distinct active roles,
- accepted contacts must accept the obligation,
- pending, declined, revoked, and replaced contacts are not notification
  eligible,
- onboarding next action is derived from account, card, P.A.C.E., medical
  jacket, and entitlement state.

## Test Evidence

`tests/onboarding_pace.rs` proves:

- self-grant and missing or duplicate P.A.C.E. roles are denied,
- accepted P.A.C.E. sets advance onboarding to medical jacket completion,
- inactive contacts are not notification eligible,
- completed onboarding requires account, emergency card, accepted P.A.C.E.,
  medical jacket, and entitlement selection.

## Boundaries

The current contract does not process names, phone numbers, email addresses,
medical data, genetic data, eligibility documents, QR payloads, payment secrets,
or EXOCHAIN receipts. It operates on synthetic references and state enums only.

The current contract does not claim EXOCHAIN enforcement.
