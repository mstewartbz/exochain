# LiveSafe Product Architecture

## Source Basis

- `docs/context/LIVESAFE_CONTEXT_SEED.md`
- `docs/EXOCHAIN_APP_BOUNDARY.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/TEST_PLAN.md`
- `docs/context/LIVESAFE_HUMAN_SAFETY_OPPORTUNITY_MODEL.md`
- `docs/whitepapers/LIVESAFE_CREATE_CARD_INVITE_FOUR_PROTECT_PEOPLE.md`
- `context/canon/2026-05-24-round-1-collective-context.md`
- `context/canon/2026-05-24-phase-9-enterprise-onboarding-commercial-architecture.md`
- `context/canon/2026-05-25-phase-14-genesis-development-trust.md`
- `context/canon/2026-05-25-phase-15-outward-trust-visual-language.md`
- `context/canon/2026-05-25-phase-17-storage-entitlement-offering.md`
- `src/lib.rs`
- `src/human_safety_opportunity.rs`
- `src/onboarding_pace.rs`
- `src/medical_jacket_custody.rs`
- `src/storage_entitlement.rs`
- `src/trust-signal.ts`
- `config/exochain-production-trust.json`
- `server/utils/exochain-production-trust-evidence.js`
- `server/utils/trust-status.js`
- `server/index.js`
- `railway.json`
- `fly.toml`

## Ground Truth

LiveSafe is a private EXOCHAIN-adjacent application surface in
`/Users/bobstewart/dev/livesafe`. It is not EXOCHAIN core. EXOCHAIN source
under `/Users/bobstewart/dev/exochain` remains read-only evidence unless Bob
explicitly authorizes a core change.

The current product direction is an enterprise-class personal safety mesh with
these adjacent product roles:

- LiveSafe as the onboarding and safety-network surface.
- VitalLock as the protected vault lineage.
- InCaseOfEmergencyCard as the printable and physical emergency-access artifact.
- Ambient as an adjacent context layer whose runtime role is not yet verified.
- EXOCHAIN as the future trust fabric target for commitments, policy
  references, access logs, and custody-style receipts after a verified adapter
  exists. Current EXOCHAIN production evidence verifies the AVC root-trust
  bundle, but that evidence does not activate LiveSafe public trust claims
  without LiveSafe adapter proof.

## Product Surfaces

The repo and current canon establish these primary LiveSafe surfaces:

- Onboarding and P.A.C.E. growth flow.
- Human-safety opportunity and first-loop readiness doctrine.
- Printable ICE card and card-packet generation.
- QR pointer and responder access boundary.
- Emergency profile editing and responder projection.
- Medical jacket custody and consent-scoped projection.
- Storage entitlements and provider-bound vault writes.
- VitalLock vault interaction rules across owner, delegated, and responder
  access.
- Commercial entitlements, trials, gifts, frontline classification, and
  marketplace templates.
- AI help, feedback, mandated-reporter, and gated agent-dispatch loops.
- Ambient context signal evaluation.
- Trust-state and genesis-status presentation.

These surfaces are represented as adjacent product rules and inactive trust
state, not as verified EXOCHAIN runtime enforcement.

## Domain Contracts

The current Rust contract layer exported by `src/lib.rs` covers:

- `src/human_safety_opportunity.rs` for the create-card, invite-your-four,
  protect-your-people loop, year-one segment priority, integer readiness
  metrics, and denial of raw sensitive payloads, unsupported responder
  adoption requirements, and unsupported EXOCHAIN/root-backed public claims.
- `src/onboarding_pace.rs` for account progression, P.A.C.E. role set,
  obligation acceptance, and notification eligibility.
- `src/medical_jacket_custody.rs` for phenotypical versus genotypical custody
  boundaries, emergency projection, and trial-matching denial.
- `src/storage_entitlement.rs` for included and paid storage levels, encrypted
  provider-write requirements, safe EXOCHAIN anchor fields, quota behavior, and
  Tier-0 emergency read behavior.
- `src/vitallock_vault.rs` for fail-closed VitalLock vault interaction
  vocabulary, storage and custody dependency checks, delegated and responder
  access gating, and blocked full-export posture.
- `src/ambient_signal.rs` for fail-closed Ambient context signal vocabulary,
  marketplace-template and consent gating, recipient-visible permit checks,
  and verified-claim denial while runtime trust remains inactive.
- `src/emergency_profile.rs` for allowed field names, release-bound emergency
  display, and fail-closed responder projection.
- `src/ice_card_packet.rs` for printable card packet composition and optional
  panel gating.
- `src/printable-card-render.ts` for synthetic PDF packet generation, cut/fold
  instruction rendering, trust-state display, and configuration-backed printed
  contact surfaces.
- `src/qr_pointer.rs` for synthetic token metadata, stale-target denial, and
  no raw-sensitive QR payloads.
- `src/qr_activation.rs` for fail-closed activation references, emergency-
  subset responder landing, and permit-only responder or network activation.
- `src/responder_access_display.rs` for fail-closed responder-facing status
  panels, emergency-subset-only display inventory, and explicit inactive or
  verified responder state tokens.
- `src/entitlement_marketplace.rs` for free, family, team, trial, gift,
  frontline, and marketplace-template state.
- `src/consent_revocation_receipt.rs` for fail-closed consent-proof inactivity,
  EXOCHAIN-only receipt provenance, and safe receipt-metadata boundaries.
- `src/ai_help_topics.rs` and `src/feedback_mandated_reporter.rs` for
  deterministic help-topic matching, feedback workflow, redaction, mandated
  reporting, and disabled-by-default dispatch.
- `src/exochain_adapter_activation.rs`, `src/trust_state_view.rs`, and
  `src/genesis_development_trust.rs` for fail-closed adapter activation,
  inactive trust-state display, and genesis-only internal development posture.
- `src/legacy_dependency.rs` for inactive `exo-legacy` dependency projection.

The TypeScript policy layer adds:

- `src/trust-signal.ts` for symbolic trust-state output and homologation rules.
- `src/exochain-root-trust-state.ts` for distinguishing read-only root evidence
  from LiveSafe adapter proof and public-claims eligibility.
- `src/exochain-boundary.ts` for adjacent-surface enforcement of the EXOCHAIN
  runtime boundary.
- `src/ip-boundary.ts` and `src/genesis-trust.ts` for disclosure and genesis
  trust policy evaluation.

## Runtime And Deployment Posture

The repo contains a legacy application stack alongside the newer contract
surface:

- `client/` contains a Vite frontend.
- `server/index.js` contains the Express API and `GET /api/health`.
- `responder/` contains a separate responder-facing frontend.
- `fly.toml` still records a prior Fly deployment shape for `livesafe-api`.

Current deployment control docs and live runtime evidence point to Railway
instead:

- `docs/context/LIVESAFE_CONTEXT_SEED.md` records Railway project `livesafe`,
  production environment, `livesafe-api` service, and `Postgres` in the
  `ARMORCLOUD` workspace.
- `railway.json` configures Dockerfile deploy and `/api/health`.
- The current production health endpoint is
  `https://livesafe-api-production.up.railway.app/api/health`, and the latest
  live Railway probe stayed healthy on 2026-06-05 with fail-closed metadata
  including `status: ok`, `database: connected`, and
  `exochain_connected: false`.
- The current production trust-status endpoint is
  `https://livesafe-api-production.up.railway.app/api/trust/status`, which
  returned `state: not-verified`, `machine_state: not_verified`, and
  `public_claims_allowed: false`.
- The repo trust-status contract now adds verified EXOCHAIN production evidence
  fields from `config/exochain-production-trust.json`, including root bundle id
  `7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58` and
  verifier commit `379a45e1d9ab092ecd446d095a7b524570530efd`, while keeping
  `public_claims_allowed: false` until the LiveSafe adapter is verified.
- Railway CLI verification is currently available: `railway status --json`
  succeeded on 2026-06-05 and confirmed project `livesafe`, production
  environment, repo-linked service `livesafe-api`, public domain
  `livesafe-api-production.up.railway.app`, and `Postgres` service. Project,
  service, deployment, and instance ids are release evidence, not stable
  architecture control values; closeout verification must read them live from
  Railway CLI.

This means the repo currently has deployment-shape drift: legacy Fly-oriented
artifacts still exist, while the current documented production target is
Railway.

## Current Boundaries

- Path classification: adjacent surface.
- Trust posture: EXOCHAIN production/root evidence is verified, but public
  EXOCHAIN-rooted claims stay inactive until a verified LiveSafe adapter and
  public-claims gate exist.
- Data posture: raw sensitive medical, genetic, identity, contact, location,
  trustee, P.A.C.E., emergency-access, payment, and eligibility data remain
  outside repo fixtures, logs, provider metadata, and EXOCHAIN anchors.
- EXOCHAIN write posture: no LiveSafe runtime path in this repo may mint or
  simulate consent, authority, provenance, custody, revocation, or governance
  outcomes.
- Disablement path: leave Rust contracts and TypeScript policy evaluators
  unwired from public runtime claims; keep trust-state output in inactive,
  genesis-pending, or internal-proof states until proof gates pass.

## Open Architecture Constraints

- No verified LiveSafe runtime adapter currently invokes EXOCHAIN for
  production authority decisions.
- No runtime route is yet wired to the adapter-activation contract.
- `exo-legacy` remains a pending EXOCHAIN-core dependency, not an available
  LiveSafe capability.
- First-responder disclosure scope remains owner-controlled and is not safe to
  activate from current repo evidence.
- Stripe product identifiers, gift/refund policy, marketplace monetization
  policy, and frontline proof policy remain Bob-only commercial decisions.
- The product and brand boundary among LiveSafe, ExoSafe, VitalLock,
  InCaseOfEmergencyCard, and Ambient is still not fully resolved by current
  source evidence.
- Deployment topology still contains historical Fly-era artifacts, but current
  production truth is Railway and must stay authoritative in user-facing docs
  and deploy controls.
