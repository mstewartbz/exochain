Work in /Users/bobstewart/dev/livesafe only. Treat /Users/bobstewart/dev/exochain and /Users/bobstewart/dev/exochain/exochain as read-only evidence; do not modify EXOCHAIN source.

Apply the LiveSafe P.A.C.E. Readiness Grant inception slice.

Slice name:
LIVESAFE_PACE_READINESS_GRANT_MODEL

Purpose:
Preserve and encode the Safety Circle Completion Grant doctrine:

- Create your card. Invite your four. Protect your people.
- Reward readiness, not referral.
- Recognize obligation, not traffic.
- Complete the circle, then grant abundance.
- Complete your Safety Circle and receive 4 months of Plus.

Allowed paths:
- docs/context/LIVESAFE_PACE_READINESS_GRANT_MODEL.md
- docs/context/LIVESAFE_SAFETY_CIRCLE_INVITATION_LANGUAGE_MODEL.md
- docs/collateral/LIVESAFE_SAFETY_CIRCLE_EXPLAINER_FOR_MAX_AND_ROBERT.md
- docs/collateral/LIVESAFE_SAFETY_CIRCLE_ONE_PAGER.md
- docs/collateral/LIVESAFE_INVITATION_COPY_PACK.md
- docs/collateral/LIVESAFE_FAQ_AND_OBJECTIONS.md
- docs/whitepapers/LIVESAFE_SAFETY_CIRCLE_COMPLETION_GRANT_FRAME.md
- docs/codex_prompts/APPLY_LIVESAFE_PACE_READINESS_GRANT_SLICE.md
- src/pace_readiness_grant.rs
- tests/pace_readiness_grant.rs
- src/lib.rs
- docs/TEST_PLAN.md
- docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md

Disallowed behavior:
- Do not activate live SMS/email/push dispatch.
- Do not activate Stripe or real billing.
- Do not create real price ids.
- Do not expand responder disclosure.
- Do not disclose medical-jacket or vault payloads.
- Do not import genetic data.
- Do not activate trial matching.
- Do not claim EXOCHAIN/root-backed production authority.
- Do not place raw medical, genetic, identity, guardian, trustee, location,
  contact, QR, vault, eligibility, payment, or privileged content into fixtures,
  logs, receipts, anchors, screenshots, status routes, or docs.

Implementation:
1. Add the control/collateral/white-paper docs.
2. Add src/pace_readiness_grant.rs.
3. Export it from src/lib.rs.
4. Add tests/pace_readiness_grant.rs.
5. Update docs/TEST_PLAN.md with:
   cargo test --test pace_readiness_grant
6. Update docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md with this completed
   slice only if tests pass.
7. Run:
   cargo test --test pace_readiness_grant
   npm run quality

Acceptance criteria:
- Grant is allowed only for exactly four distinct accepted P.A.C.E. roles.
- Each accepted role must have obligation_accepted = true.
- Each accepted role must have verified notification eligibility.
- Each invitee must be able to decline without penalty and revoke after acceptance.
- Self-grant, duplicate roles, duplicate contacts, revoked/replaced/declined states,
  raw contact metadata, unsupported responder claims, unsupported EXOCHAIN claims,
  and referral-bounty framing all deny the grant.
- Allowed grant duration is exactly 4 months.
- Output distinguishes Safety Circle strength from public gamification.
- No raw sensitive data appears anywhere.

End with:
completed work,
path classification,
files changed,
tests run,
validation result,
deployment result,
next item,
rollback or disablement path,
and any narrowed escalation.
