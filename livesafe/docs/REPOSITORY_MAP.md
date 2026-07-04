# Repository Map

## Source Basis

- `README.md`
- `docs/EXOCHAIN_APP_BOUNDARY.md`
- `docs/TEST_PLAN.md`
- `config/surface-intake.json`
- `config/exochain-production-trust.json`
- `.github/workflows/quality.yml`
- `railway.json`
- live health check on 2026-06-05: `https://livesafe-api-production.up.railway.app/api/health`
- Railway project `livesafe` in the `ARMORCLOUD` workspace with production
  app service `livesafe-api` and service `Postgres`.
- Railway project, environment, service, deployment, and instance ids are
  live Railway closeout evidence and must be read from `railway status --json`
  during bounded verification instead of being pinned in this control doc.
- EXOCHAIN production health/readiness and root-trust verification on
  2026-06-03:
  `https://exochain-production.up.railway.app/health`,
  `https://exochain-production.up.railway.app/ready`, and verifier commit
  `379a45e1d9ab092ecd446d095a7b524570530efd`
- read-only EXOCHAIN evidence:
  `/Users/bobstewart/dev/exochain/AGENTS.md`
- read-only EXOCHAIN evidence:
  `/Users/bobstewart/dev/exochain/Cargo.toml`
- read-only EXOCHAIN evidence:
  `/Users/bobstewart/dev/exochain/.github/workflows/ci.yml`

## Ground Truth

- Two repositories are currently mapped for this workspace:
  `github.com/bob-stewart/livesafe` and `github.com/exochain/exochain`.
- LiveSafe is the only repository this automation edits. EXOCHAIN remains
  read-only evidence until Bob requests core work.
- Current LiveSafe deployment evidence points to Railway project `livesafe`
  in the `ARMORCLOUD` workspace. The public health endpoint returned `200 OK`
  on 2026-06-05, with `server: railway-hikari`,
  `cache-control: no-store`, `x-railway-edge: railway/us-east4-eqdc4a`,
  and a fail-closed body including `status: ok`, `database: connected`, and
  `exochain_connected: false`.
- The public trust-status endpoint returned `200 OK` on 2026-06-05 with
  `cache-control: no-store` and a fail-closed body including
  `state: not-verified`, `machine_state: not_verified`, and
  `public_claims_allowed: false`.
- Railway CLI verification is currently available: `railway status --json`
  succeeded on 2026-06-05 and confirmed Railway project `livesafe`,
  production environment, repo-linked `livesafe-api` service, public domain
  `livesafe-api-production.up.railway.app`, and `Postgres` service.
  Project, environment, service, deployment, and instance ids are live Railway
  closeout evidence, not stable control values; closeout verification must
  read them live from Railway CLI.
- EXOCHAIN production evidence is verified for LiveSafe status reporting:
  production `/health` and `/ready` returned `ok`, AVC root-trust bundle
  `7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58`
  verified with EXOCHAIN commit
  `379a45e1d9ab092ecd446d095a7b524570530efd`, and the only current recorded
  sentinel observation is `QuorumHealth` below BFT minimum.
- No additional `bob-stewart` repositories are active dependencies for this
  workspace until they are retrieved, classified, and verified.

## Active Repository Intake Records

### `github.com/bob-stewart/livesafe`

| Field | Value |
| --- | --- |
| Repository URL | `github.com/bob-stewart/livesafe` |
| Local path | `/Users/bobstewart/dev/livesafe` |
| Owner | `bob-stewart` |
| Accountable maintainer | Bob Stewart |
| Deployment status | `prototype` |
| Path classification | adjacent surface and proprietary IP |
| EXOCHAIN primitive dependencies | evidence-only references through `/Users/bobstewart/dev/exochain`; verified EXOCHAIN production/root evidence through `config/exochain-production-trust.json`; no verified LiveSafe runtime adapter |
| EXOCHAIN core read/write capability | read: none; write: none |
| Public trust-claim permission | no; EXOCHAIN production evidence is verified, but trust remains inactive until a verified LiveSafe adapter and proof gates pass |
| Secret inventory and source | `config/surface-intake.json` lists no repo-tracked secrets; runtime secrets must stay outside version control |
| Local test command | `npm run quality` |
| CI gate | `.github/workflows/quality.yml` runs `npm ci`, `npm --prefix server ci`, and `npm run quality` on pushes to `main` and pull requests |
| Deployment evidence | `railway.json`; Railway project `livesafe`; service `livesafe-api`; public health endpoint `https://livesafe-api-production.up.railway.app/api/health`; project, environment, service, deployment, and instance ids are live Railway closeout evidence |
| Rollback or disablement path | `Keep `config/exochain-primitives.json` at `runtimeAdapterStatus: not-wired` so `server/utils/livesafe-exochain-adapter.js` denies EXOCHAIN transport calls and public trust status remains fail-closed.` |
| Artifact sources used to justify the mapping | `README.md`, `docs/EXOCHAIN_APP_BOUNDARY.md`, `config/surface-intake.json`, `config/exochain-production-trust.json`, `.github/workflows/quality.yml`, `railway.json`, live health check on 2026-06-05 |
| IP classification and public-release permission | private commercial venture; public release requires owner approval per `docs/IP_HANDLING.md` |

### `github.com/exochain/exochain`

| Field | Value |
| --- | --- |
| Repository URL | `github.com/exochain/exochain` |
| Local path | `/Users/bobstewart/dev/exochain` |
| Owner | `exochain` |
| Accountable maintainer | Exochain Foundation maintainers |
| Deployment status | local read-only evidence for LiveSafe; not a LiveSafe deployment target |
| Path classification | read-only dependency evidence |
| EXOCHAIN primitive dependencies | canonical workspace crates listed in `/Users/bobstewart/dev/exochain/Cargo.toml`; LiveSafe currently references these as evidence only |
| EXOCHAIN core read/write capability | LiveSafe automation: read-only evidence; write: prohibited without Bob-only escalation |
| Public trust-claim permission | not from this repository mapping; LiveSafe may not inherit constitutional claims by proximity |
| Secret inventory and source | not inspected as a secret source for LiveSafe; do not import EXOCHAIN runtime credentials into the LiveSafe repo |
| Local test command | `cargo test --workspace` |
| CI gate | `/Users/bobstewart/dev/exochain/.github/workflows/ci.yml` runs release builds, workspace tests, coverage, clippy, format, audit, deny, docs, and repo hygiene |
| Deployment evidence | repository remote `https://github.com/exochain/exochain.git`; local checkout at commit `d47f58d3` |
| Rollback or disablement path | do not wire LiveSafe directly into EXOCHAIN core runtime paths; keep EXOCHAIN as read-only evidence until a verified LiveSafe adapter is implemented and tested fail-closed |
| Artifact sources used to justify the mapping | `docs/EXOCHAIN_APP_BOUNDARY.md`, `/Users/bobstewart/dev/exochain/AGENTS.md`, `/Users/bobstewart/dev/exochain/Cargo.toml`, `/Users/bobstewart/dev/exochain/.github/workflows/ci.yml` |
| IP classification and public-release permission | separate EXOCHAIN Foundation repository; LiveSafe automation may read evidence but may not reclassify or export proprietary/internal artifacts by default |

## Candidate Retrieval Repositories

These repositories were reported by imported context inventories. They are not
active dependencies for this workspace until retrieved, classified, and
verified.

| Repository | Reported role | Source record | Current action |
| --- | --- | --- | --- |
| `github.com/bob-stewart/livesafe-app` | possible LiveSafe app variant | Phase 7 | verify |
| `github.com/bob-stewart/livesafe-test` | possible LiveSafe test/prototype | Phase 7 | verify |
| `github.com/bob-stewart/livesafe-test-2` | possible LiveSafe test/prototype | Phase 7 | verify |
| `github.com/bob-stewart/VitalLock` | VitalLock lineage and ICE-PACE architecture | Round 1 / Phase 7 | retrieve |
| `github.com/bob-stewart/ice-card` | legacy ICE card implementation | Round 1 / Phase 7 | retrieve |
| `github.com/bob-stewart/IceCardReact` | historical ICE card UI | Round 1 / Phase 7 | retrieve |
| `github.com/bob-stewart/ice-spring` | possible ICE card backend | Round 1 / Phase 7 | retrieve |
| `github.com/bob-stewart/ambient.li` | Ambient.li app surface | Round 1 / Phase 7 | retrieve |
| `github.com/bob-stewart/ambientli` | Ambient.li app surface | Round 1 / Phase 7 | retrieve |
| `github.com/bob-stewart/exochain` | personal EXOCHAIN fork or surface | Round 1 / Phase 7 | compare |
| `github.com/bob-stewart/exochain-1` | reported EXOCHAIN variant | Phase 7 | verify |
| `github.com/bob-stewart/exochain-platform` | reported EXOCHAIN platform variant | Phase 7 | verify |
| `github.com/bob-stewart/exochain-v2` | reported EXOCHAIN variant | Phase 7 | verify |
| `github.com/bob-stewart/exochain-crosschecked` | reported EXOCHAIN crosscheck variant | Phase 7 | compare |
| `github.com/exochain/exoforge` | reported ExoForge runtime/build surface | Phase 5 | retrieve |
| `github.com/exochain/SENTIENTS` | reported AEGIS/SYBIL governance surface | Phase 5 | retrieve |

## Pending EXOCHAIN Core Crates

These crates were named by source artifacts but are not current LiveSafe runtime
dependencies.

| Crate | Proposed path | Source record | Current status | LiveSafe action |
| --- | --- | --- | --- | --- |
| `exo-legacy` | `/Users/bobstewart/dev/exochain/exochain/crates/exo-legacy` | Phase 11 transfer package | not present in local EXOCHAIN checkout at `7a4137f7` | track as pending dependency |

## Repository Intake Record Requirements

Each added repository must record:

- repository URL
- local path
- owner
- accountable maintainer
- deployment status
- path classification
- EXOCHAIN primitive dependencies
- EXOCHAIN core read/write capability
- public trust-claim permission
- secret inventory and source
- local test command
- CI gate
- rollback or disablement path
- artifact sources used to justify the mapping
- IP classification and public-release permission
