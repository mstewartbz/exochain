# ExoChain Demo

## Overview

The ExoChain demo is a proprietary adjacent prototype. It is not the canonical
EXOCHAIN Rust trust fabric and cannot claim constitutional enforcement by
proximity. It comprises:

- **7 Node.js microservices** handling identity, consent, governance, decision-making, auditing, and provenance
- **React web UI** with a 12-column configurable widget grid across 6 pages
- **Rust-to-WASM adapter** generated separately from the Apache-2.0 core WASM primitive
- **DAG DB gateway adapter** for authenticated, tenant-scoped persistence

The quarantined PostgreSQL configuration is a legacy fixture enabled only by
the `legacy-postgres-fixture` Compose profile. It is not a production writer.

## Licensing

Except for `packages/exochain-wasm`, this subtree is proprietary and
`UNLICENSED`; see [LICENSE](LICENSE). CrossChecked and LiveSafe require written
commercial terms, active EXOCHAIN bailment licensure, and EXOCHAIN usage
accounting. The Apache-2.0 WASM wrapper does not license adjacent products.

## Prerequisites

| Dependency | Version |
|---|---|
| Node.js | 20+ |
| Rust + wasm-pack | Latest stable |
| Docker | 24+ |
| Docker Compose | v2+ |

## Quick Start

Build the WASM engine and launch all services with Docker Compose:

```bash
npm run build:wasm
npm run dev
```

This runs `docker compose up` under the hood after the required `EXO_DEMO_DAGDB_*`
configuration is supplied. The legacy PostgreSQL fixture is not started unless
its profile is explicitly enabled with a separately supplied password.

## Local Development (without Docker)

For iterative development without Docker:

```bash
cd demo
npm install
bash scripts/dev.sh
```

The `dev.sh` script starts each service in the background with file watching enabled.

## Web UI

```bash
cd demo/web
npm install
npm run dev
```

The dev server starts at **http://localhost:5173**.

## Architecture

### Pages

The web UI is organized into 6 pages:

1. **Dashboard** -- Top-level overview of system health, recent governance decisions, and active workflows.
2. **System Explorer** -- Deep inspection of services, WASM module state, and runtime metrics.
3. **Board of Directors** -- Council management interface for reviewing and voting on proposals.
4. **Class Action** -- Batch governance actions across multiple entities or policies.
5. **Syntaxis Builder** -- Visual workflow editor for composing Syntaxis governance programs.
6. **AI + Backlog** -- AI-assisted suggestion pipeline and council-approved backlog management.

### Widget System

Every page uses a 12-column drag-and-drop widget grid:

- **Edit Mode** -- Toggle edit mode to rearrange, resize, add, or remove widgets.
- **Widget Catalog** -- Browse available widgets and drop them onto the grid.
- **localStorage Persistence** -- Layout configurations are saved per-user in the browser.
- **AI Help** -- Each widget includes a context-sensitive `?` menu providing inline documentation and usage guidance.

## Services

| Service | Port | Description |
|---|---|---|
| gateway-api | 3000 | API gateway and request router |
| identity-service | 3001 | Identity management and authentication |
| consent-service | 3002 | Consent collection and verification |
| governance-engine | 3003 | Core governance rule evaluation |
| decision-forge | 3004 | Decision aggregation and quorum logic |
| provenance-writer | 3006 | Immutable provenance record writer |
| audit-api | 3007 | Audit trail query and reporting |

Each service exposes a `/health` endpoint for liveness and readiness checks.

## Node.js & React Test Suite

Run all tests for the 7 Node.js services and React UI:

```bash
cd demo
npm run test              # run all 99 tests (services + React UI)
npm run test:watch        # watch mode
npm run test:coverage     # run with coverage report (enforced by CI Gate 10)
npm run test:services     # services only
npm run test:react        # React UI only
```

Tests use [Vitest 3](https://vitest.dev/) with a workspace configuration covering all 8 projects.

**Coverage thresholds** (enforced in CI):
| Project | Lines | Functions | Branches | Statements |
|---------|-------|-----------|----------|------------|
| Services (×7) | 80% | 80% | 70% | 80% |
| React UI | 70% | 70% | 60% | 70% |

Test results:
```
Test Files  8 passed (8)
     Tests  99 passed (99)
```

## WASM Test Suite

Run the full WASM binding test suite:

```bash
npm run test:wasm
```

This executes **25 tests** covering all **9 binding modules**:

- Constitution loader and validator
- Governance state machine transitions
- Decision quorum calculations
- Consent verification logic
- Provenance hashing
- Audit event serialization
- Policy distribution encoding
- Syntaxis workflow compilation
- Kernel invariant checks

## ExoForge Integration

The demo platform integrates with [ExoForge](https://github.com/exochain/exoforge) for autonomous self-improvement:

### Feedback Loop

Every widget includes an AI help menu (`?` button). User interactions generate structured feedback that enters the ExoForge self-improvement cycle:

```bash
# Submit feedback from the UI
POST /api/feedback
{
  "widget": "bcts-machine",
  "page": "dashboard",
  "type": "suggestion",
  "message": "Add real-time state transition animation",
  "context": { "current_state": "Deliberated" }
}

# View backlog
GET /api/backlog

# Council vote on item
POST /api/backlog/vote
{ "id": "FB-xxx", "vote": "approve", "panel": "Architecture" }

# Update item status (ExoForge callback)
POST /api/backlog/status
{ "id": "FB-xxx", "status": "implementing", "exoforge_run_id": "run-123" }
```

### GitHub Issues

Issues on [exochain/exochain](https://github.com/exochain/exochain/issues) labeled `exoforge:triage` are automatically ingested into the same pipeline via the `exoforge-triage.yml` GitHub Action.

See [docs/guides/ARCHON-INTEGRATION.md](../docs/guides/ARCHON-INTEGRATION.md) for full ExoForge documentation.

## Environment Variables

| Variable | Description | Default |
|---|---|---|
| `EXO_DEMO_DAGDB_GATEWAY_URL` | EXOCHAIN DAG DB gateway origin for demo persistence | Required |
| `EXO_DEMO_DAGDB_AUTH_TOKEN` | Bearer token for the DAG DB gateway | Required |
| `EXO_DEMO_DAGDB_TENANT_ID` | Tenant scope bound into every demo DAG DB request | Required |
| `EXO_DEMO_DAGDB_NAMESPACE` | Namespace scope bound into every demo DAG DB request | Required |
| `EXO_DEMO_DAGDB_OWNER_DID` | Owner DID recorded on demo DAG DB intake records | Required |
| `EXO_DEMO_DAGDB_CONTROLLER_DID` | Controller DID recorded on demo DAG DB intake records | Required |
| `EXO_DEMO_DAGDB_SUBMITTED_BY_DID` | Submitter DID recorded on demo DAG DB intake records | Required |
| `EXO_DEMO_DAGDB_WRITE_SIGNATURE` | Write-signature header forwarded to the DAG DB gateway | Required |
| `PORT` | Service listen port | Varies per service (see table above) |
| `NODE_ENV` | Runtime environment | `development` |
| `CROSSCHECKED_API_TOKENS` | CrossChecked API bearer principal map as JSON: `{"token":{"actor_did":"did:exo:...","role":"steward"}}`. Missing or malformed values make `/api` routes fail closed with `401`. | None |
| `LIVESAFE_API_TOKENS` | LiveSafe API bearer principal map as JSON: `{"token":{"actor_did":"did:exo:...","role":"owner"}}`. Roles are `owner`, `trustee`, `responder`, or `admin`; missing or malformed values make `/api` routes fail closed with `401`. | None |

Each service reads its own `PORT` from the environment. In Docker Compose, these are pre-configured in the compose file.
