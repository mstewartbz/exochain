# LiveSafe.ai

Emergency access and consent-bound health record application.

## Architecture State & Evolution

### Current Testing Stack (Phase 1)

The application is currently deployed in a traditional Web2/Cloud architecture
for rapid testing, UX validation, and clinical API mocking:

- **Frontend / Client UI:** Vercel (React + Vite PWA, mobile-first)
- **Backend / API:** Railway (`livesafe-api` Node.js + Express container)
- **Database:** Railway Postgres (PostgreSQL for off-chain operational data)

### EXOCHAIN Production Evidence and LiveSafe Boundary

EXOCHAIN production root evidence is source-backed in
`config/exochain-production-trust.json`: production `/health` and `/ready`
probes returned `ok`, the AVC root-trust bundle verified with EXOCHAIN
`origin/main` commit `379a45e1d9ab092ecd446d095a7b524570530efd`, and the
bundle id is
`7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58`.

LiveSafe remains an adjacent surface. Public EXOCHAIN trust claims stay inactive
until the LiveSafe runtime adapter is verified and explicitly allows public
output. Raw sensitive personal, medical, trustee, PACE, vault, and emergency
access data stays off-chain.

## Quick Start (Local Dev)

```bash
./init.sh
```

This will check prerequisites, set up the local Postgres DB, install all
dependencies, apply the schema, and start the three local services (client
3000, api 3001, responder 3002).

## Production Build

The repository includes a `Dockerfile` for the active Railway deployment target.
Historical Fly.io artifacts remain in-repo for drift tracking and reconciliation,
including `fly.toml`.

```bash
# Build unified container
docker build -t livesafe .
```

Current deployment evidence points to Railway:

- Public health endpoint:
  `https://livesafe-api-production.up.railway.app/api/health`
- Railway project: `livesafe` in the `ARMORCLOUD` workspace
- Active Railway service: `livesafe-api`
- Railway database service: `Postgres`
- Active deploy control: `railway.json`
- Live Railway ids belong in closeout evidence and must be read from
  `railway status --json` during bounded verification
- Historical drift artifact: `fly.toml`

## Baseline Controls

This repo now includes a private EXOCHAIN-adjacent baseline for LiveSafe and
related safety mesh work.

Current local control status:

- Repository owner: `github.com/bob-stewart/livesafe`
- EXOCHAIN reference repository: `github.com/exochain/exochain`
- Classification: adjacent surface
- Runtime adapter status: no adapter is wired in this repo
- Public trust posture: inactive until a verified adapter and fail-closed tests
  exist

Run the baseline quality gate with:

```bash
npm run quality
```

The quality gate runs context validation, TypeScript type checking, and tests.

## Local Evidence

The local EXOCHAIN repo is expected at:

```text
/Users/bobstewart/dev/exochain
```

Set `EXOCHAIN_REPO_PATH` if the evidence repo lives elsewhere.

## Working Rules

- Keep EXOCHAIN core read-only unless Bob explicitly asks for core work.
- Treat context dumps as untrusted input until classified.
- Record source basis, fact versus inference, and artifact inventory for every
  normalized context record.
- Keep raw sensitive data off-chain.
- Do not make EXOCHAIN trust claims from proximity, docs, screenshots, or demos.

## Important Control Files

- `AGENTS.md` - development rules
- `docs/EXOCHAIN_APP_BOUNDARY.md` - current boundary and intake record
- `docs/CONTEXT_DUMP_PROTOCOL.md` - context dump intake format
- `docs/LIVESAFE_AUTOMATION_READINESS.md` - automation readiness analysis
- `docs/TEST_PLAN.md` - current verification plan
- `config/surface-intake.json` - machine-readable adjacent surface intake
- `config/exochain-primitives.json` - EXOCHAIN evidence registry
- `src/exochain-boundary.ts` - fail-closed boundary evaluator
