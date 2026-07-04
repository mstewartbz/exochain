# LiveSafe Context Seed

## Source Basis

- Local repo: `/Users/bobstewart/dev/livesafe`.
- Private repository: `github.com/bob-stewart/livesafe`.
- Local development rules: `AGENTS.md`.
- Canon records: `context/canon/*`.
- Boundary records: `docs/EXOCHAIN_APP_BOUNDARY.md`,
  `docs/LIVESAFE_AUTOMATION_READINESS.md`, and `docs/TEST_PLAN.md`.
- Current deployment evidence: Railway project `livesafe`
  in the `ARMORCLOUD` workspace, production environment, service
  `livesafe-api`, and service `Postgres`.
- Live Railway project, environment, service, deployment, and instance ids are
  closeout evidence and must be read from `railway status --json` during
  bounded verification instead of being pinned in this control doc.

## Ground Truth

LiveSafe is a private EXOCHAIN-adjacent commercial application surface. It is
not EXOCHAIN core. EXOCHAIN source under `/Users/bobstewart/dev/exochain` is
read-only evidence unless Bob explicitly directs core work.

The present product direction is an enterprise-class personal safety mesh:

- LiveSafe is the consumer onboarding and safety network.
- VitalLock is the protected vault lineage.
- InCaseOfEmergencyCard is the physical and printable emergency access
  artifact.
- Ambient is an adjacent context layer whose runtime role remains unproven.
- EXOCHAIN is the trust fabric target for consent, access, revocation, custody,
  commitments, access logs, and receipts after verified adapters exist.

## Operating Doctrine

Use this loop for LiveSafe work:

```text
Ground Truth -> Doctrine -> Domain -> Data -> Doors -> Documentation -> Deployment -> Drift
```

Current build posture:

- TDD first.
- Rust domain contracts for safety-critical product rules.
- Synthetic fixtures only.
- Raw medical, genetic, identity, location, contact, trustee, emergency, vault,
  payment, and eligibility data stays out of repo fixtures, logs, provider
  metadata, content-addressed storage, and EXOCHAIN anchors.
- Trust state remains inactive unless the exact runtime adapter and proof gate
  are verified.

## Deployment Baseline

- Working path: `/Users/bobstewart/dev/livesafe`.
- GitHub repository: `https://github.com/bob-stewart/livesafe`.
- Railway project: `livesafe`.
- Railway environment: `production`.
- Railway app service: `livesafe-api`.
- Railway database service: `Postgres`.
- Railway region for app and database: `us-east4-eqdc4a`.
- Public health endpoint:
  `https://livesafe-api-production.up.railway.app/api/health`.

## Current Product Priorities

1. Keep onboarding simple enough for immediate adoption.
2. Capture P.A.C.E. contacts as a social-contract growth engine.
3. Move users toward a controlled medical jacket without exposing raw records.
4. Support free basic use, family/team plans, gift subscriptions, trials,
   frontline eligibility, and paid capability gates.
5. Add storage levels, including encrypted content-addressed storage options.
6. Add AI help, feedback, mandated reporting, and gated agent dispatch.
7. Keep every trust-bearing surface visually and machine-readable as inactive
   until the proof gates pass.

## Completed Contract Slices

- Storage entitlement contract: `src/storage_entitlement.rs` and
  `tests/storage_entitlement.rs`.
- Onboarding and P.A.C.E. progression contract:
  `src/onboarding_pace.rs` and `tests/onboarding_pace.rs`.
- Medical jacket custody contract:
  `src/medical_jacket_custody.rs` and `tests/medical_jacket_custody.rs`.
- Emergency profile contract: `src/emergency_profile.rs` and
  `tests/emergency_profile.rs`.
- Responder access display contract: `src/responder_access_display.rs` and
  `tests/responder_access_display.rs`.

## Active Gate

```bash
npm run quality
```
