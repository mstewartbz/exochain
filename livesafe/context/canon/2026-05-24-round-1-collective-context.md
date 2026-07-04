# Round 1 Collective Context Import

## Source Basis

- Source: user-provided Round 1 assistant output pasted in the current Codex
  thread on 2026-05-24.
- Related local transfer artifact checked later:
  `/Users/bobstewart/Downloads/livesafe_exochain_context_inventory.md`.
  That file has 656 lines and preserves a fuller A-P inventory with code,
  schema, governance, contradiction, retrieval, and search-term sections.
- Claimed external sources searched by the Round 1 assistant: personal memory,
  current chat, partial prior-chat access, file library, Google Drive, Notion,
  GitHub repositories, and connected documents.
- Sources not searched by that Round 1 assistant: public web, email, calendar,
  Dropbox, Slack, Teams, SharePoint, and global Figma search.
- Locally corroborated context available in this workspace family:
  `/Users/bobstewart/dev/livesafe`,
  `/Users/bobstewart/dev/demo/livesafe`,
  `/Users/bobstewart/dev/exochain`, and
  `/Users/bobstewart/dev/exochain/exochain`.
- Classification: imported evidence. Claims from Google Drive, remote GitHub
  repositories, and file-library artifacts remain retrieval targets until the
  cited artifacts are present in this repo or checked directly in the connected
  source.

## Fact vs Inference

- Fact: `/Users/bobstewart/dev/livesafe` is the private commercial LiveSafe
  workspace being prepared as an adjacent surface to `github.com/exochain/exochain`.
- Fact: this workspace currently treats EXOCHAIN as read-only evidence unless
  Bob explicitly asks for core edits.
- Fact: the local legacy checkout `/Users/bobstewart/dev/demo/livesafe` contains
  a LiveSafe implementation with subscriber health profile data, QR/NFC card
  flows, PACE trustee workflows, consent routes, audit receipts, and an
  EXOCHAIN GraphQL client.
- Fact: the local EXOCHAIN demo tree contains LiveSafe and VitalLock app and API
  surfaces under `/Users/bobstewart/dev/exochain/exochain/demo`.
- Fact: local EXOCHAIN records contain LiveSafe-related schema and migration
  evidence, including `livesafe_identities`, `scan_receipts`,
  `consent_anchors`, and `trustee_shard_status`.
- Fact: Round 1 claims that Google Drive contains ExoSafe, ExoChain operating
  architecture, VitalLock, AVC briefing, and Ambient transcript artifacts.
  Those documents are not yet stored in this repo.
- Fact: Round 1 claims that remote GitHub repositories exist for
  `bob-stewart/VitalLock`, `bob-stewart/ice-card`, `bob-stewart/ambientli`,
  `bob-stewart/ambient.li`, `bob-stewart/IceCardReact`, and
  `bob-stewart/ice-spring`. Those remote repositories need direct retrieval
  before their details can be treated as local truth.
- Fact: Round 1 found no direct evidence that Ambient.li is already integrated
  with LiveSafe, VitalLock, ICE card, or EXOCHAIN safety flows.
- Inference: the strongest current product lineage appears to be
  InCaseOfEmergencyCard.com and ICE card as physical emergency access,
  VitalLock as secure emergency messaging and digital legacy lineage,
  LiveSafe as health and emergency safety surface, and EXOCHAIN as the
  underlying consent, authority, receipt, and revocation substrate.
- Inference: ExoSafe may be a newer umbrella or consumer front-door name, while
  LiveSafe may be a healthcare-specific or prior naming layer. This remains
  unresolved because Round 1 reports both names in adjacent 2026 documents.
- Inference: Ambient.li is a candidate context or passive-presence layer only
  after direct artifacts establish that role and define consent boundaries.

## Artifact Inventory

### Locally Corroborated Artifacts

| Artifact | Type | Location | Relevant concepts | Why it matters | Status |
| --- | --- | --- | --- | --- | --- |
| LiveSafe adjacent workspace | repo scaffold | `/Users/bobstewart/dev/livesafe` | LiveSafe, EXOCHAIN boundary | Private commercial workspace for normalized context and guarded implementation | active |
| EXOCHAIN app boundary | policy doc | `/Users/bobstewart/dev/livesafe/docs/EXOCHAIN_APP_BOUNDARY.md` | adjacent surface, sensitive data, runtime claim gate | Establishes no core edits and no EXOCHAIN runtime claims without a verified adapter | active |
| Boundary evaluator | TypeScript module | `/Users/bobstewart/dev/livesafe/src/exochain-boundary.ts` | consent, revocation, trust claims, sensitive data | Implements fail-closed classification for adjacent LiveSafe work | active |
| Legacy LiveSafe checkout | repo | `/Users/bobstewart/dev/demo/livesafe` | LiveSafe.ai, QR/NFC, PACE, health vault | Mature predecessor with real application and API code | retrieve selectively |
| Legacy LiveSafe README | doc | `/Users/bobstewart/dev/demo/livesafe/README.md` | patient-sovereign identity, EXOCHAIN DIDs, Bailment, receipts | Direct local evidence for the LiveSafe to EXOCHAIN target state | merge |
| Legacy LiveSafe schema | SQL schema | `/Users/bobstewart/dev/demo/livesafe/server/db/schema.sql` | subscribers, allergies, medications, contacts, trustees, scans, records, consent, audit | Concrete field-level data model for emergency profile and vault flows | compare |
| Legacy scan route | API code | `/Users/bobstewart/dev/demo/livesafe/server/routes/scan.js` | first responder, 4-hour access, PACE alerts, audit receipts | Core golden-hour emergency path and current fail-soft anchoring behavior | verify |
| Legacy EXOCHAIN client | API adapter | `/Users/bobstewart/dev/demo/livesafe/server/utils/exochain-client.js` | GraphQL anchoring, identity, consent, scans, audit | Shows how LiveSafe attempted to call EXOCHAIN gateway operations | verify |
| Legacy consent route | API code | `/Users/bobstewart/dev/demo/livesafe/server/routes/consent.js` | provider consent, access requests, revocation, receipts | Important source for role and revocation policy | compare |
| Legacy VSS helper | crypto utility | `/Users/bobstewart/dev/demo/livesafe/server/utils/vss.js` | Shamir, 3-of-4 PACE recovery | Concrete trustee recovery implementation evidence | verify |
| EXOCHAIN LiveSafe demo app | frontend code | `/Users/bobstewart/dev/exochain/exochain/demo/apps/livesafe` | ICE card, PACE, golden-hour, wellness | Local EXOCHAIN demo representation of LiveSafe workflows | compare |
| EXOCHAIN LiveSafe API | service code | `/Users/bobstewart/dev/exochain/exochain/demo/services/livesafe-api` | profile, emergency plans, ICE scans, PACE, receipts | Local API surface with tests for owner and responder access | compare |
| EXOCHAIN VitalLock demo app | frontend code | `/Users/bobstewart/dev/exochain/exochain/demo/apps/vitallock` | VitalLock, encrypted messages, PACE, death verification | Local evidence that VitalLock has an EXOCHAIN-adjacent demo surface | compare |
| EXOCHAIN VitalLock API | service code | `/Users/bobstewart/dev/exochain/exochain/demo/services/vitallock-api` | death verification, assets, family, PACE, keys | Concrete VitalLock workflow and security test surface | compare |
| LiveSafe demo SQL | database schema | `/Users/bobstewart/dev/exochain/exochain/demo/infra/postgres/init/005_livesafe.sql` | profiles, emergency plans, ICE cards, scans, PACE, wellness | EXOCHAIN demo database model for LiveSafe | merge |
| VitalLock demo SQL | database schema | `/Users/bobstewart/dev/exochain/exochain/demo/infra/postgres/init/004_vitallock.sql` | profiles, PACE, encrypted messages, assets, death verification | EXOCHAIN demo database model for VitalLock | merge |
| EXOCHAIN shared schema | database schema | `/Users/bobstewart/dev/exochain/exochain/demo/infra/postgres/init/001_schema.sql` | LiveSafe identities, scan receipts, consent anchors | Shared EXOCHAIN demo schema for identity and receipts | compare |
| LiveSafe remediation record | gap registry | `/Users/bobstewart/dev/exochain/GAP-REGISTRY.md` | LiveSafe, PACE, production resolver behavior | Records GAP-007 closure claim and remaining verification path | verify |
| EXOCHAIN operating README | doc | `/Users/bobstewart/dev/exochain/README.md` | CGR Kernel, AVC, AEGIS, SYBIL, HonorGood, primitives | Local EXOCHAIN primitive and product map | compare |
| EXOCHAIN adjacent rules | policy doc | `/Users/bobstewart/dev/exochain/AGENTS.md` | adjacent surface, no trust by proximity, adapter evidence | Governs how LiveSafe should depend on EXOCHAIN without overclaiming | active |

### Imported Retrieval Targets

| Artifact | Type | Claimed location | Relevant concepts | Why it matters | Next action |
| --- | --- | --- | --- | --- | --- |
| `livesafe_exochain_context_inventory.md` | transfer artifact | `/Users/bobstewart/Downloads/livesafe_exochain_context_inventory.md` | LiveSafe, VitalLock, ICE card, Ambient, EXOCHAIN | Fuller local copy of the Round 1-style all-brand inventory | preserve pointer |
| `bob-stewart/livesafe` at ref `02e98af35758e4439de2515f7d54d76ced951a4d` | remote repo | GitHub | LiveSafe, EXOCHAIN, PACE, QR/NFC | Canonical private repo state named in Round 1 | compare to local legacy checkout |
| `app_spec.txt` | product spec | `bob-stewart/livesafe/app_spec.txt` | LiveSafe roles, consent scopes, audit policy, APIs | Reported as the most complete LiveSafe spec | retrieve |
| `crates/exo-gateway/src/livesafe.rs` | Rust code | `bob-stewart/exochain` commit `be3dd...` | LiveSafe GraphQL, scan receipt, consent anchor, PACE shard status | Reported as strongest EXOCHAIN-side LiveSafe integration evidence | retrieve |
| `ULTRAPLAN-GAP-007-LIVESAFE.md` | remediation doc | `exochain/exochain/gap` | LiveSafe resolver behavior, mock removal, PACE tests | Needed to reconcile Round 1 remediation with local GAP registry | retrieve |
| `Exochain-audit-report-run2.html` | audit report | file library | LiveSafe gateway, notifications, consent, SYBIL, provenance | Reported security findings around LiveSafe and EXOCHAIN paths | retrieve |
| `AGENTS.md - EXOCHAIN USI-1 Draft` | policy draft | file library | AEON7, USI-1, consent, authority, human override | Reported constitutional constraints outside local AGENTS text | retrieve |
| `HONORGOOD_EXOCHAIN_PLAN_1.3.md` | plan/spec | file library | HonorGood, AVC, receipts, settlement, authority envelope | Reported source for receipt and authority envelope primitives | retrieve |
| `ExoChain_Operating_Architecture_v1.docx` | strategy doc | Google Drive | ExoClaw, ICE card, VitalLock, PACE, CGR Kernel | Older architecture and lineage source | retrieve |
| `ExoChain_Operating_Architecture_v2.docx` | strategy doc | Google Drive | ExoSafe, EXOCHAIN, ICE, PACE, ExoClaw, Decision.Forum | Reported newer ExoSafe consumer front-door source | retrieve |
| `ExoSafe_GTM_Playbook_v1.docx` | GTM playbook | Google Drive | ExoSafe, ICE card, vault, governed agent | Reported day-one product and language source | retrieve |
| `2026-03-23 - AVC Operator Briefing` | meeting transcript | Google Drive | LiveSafe, ExoClaw, first responders, InCaseOfEmergencyCard | Reported evidence that LiveSafe.ai remained active after ExoSafe docs | retrieve |
| VitalLock emergency messaging doc | marketing/product doc | Google Drive | VitalLock, private emergency messaging, PEBS alerts | Reported original VitalLock promise | retrieve |
| `bob-stewart/VitalLock` | remote repo | GitHub | VitalLock, ICE-PACE, digital legacy, ExoChain.ai | Reported broader VitalLock architecture | retrieve |
| `bob-stewart/ice-card` | remote repo | GitHub | ICE card, Hyperledger Fabric, key escrow | Historical ICE card implementation | retrieve |
| `bob-stewart/IceCardReact` | remote repo | GitHub | ICE card UI | Historical user experience source | retrieve |
| `bob-stewart/ice-spring` | remote repo | GitHub | ICE card backend | Possible alternate backend lineage | retrieve |
| `20250730 - 30 Min Meeting between Bob Stewart and Ben Lucyk - Transcript` | meeting transcript | Google Drive | Ambient.li, Ambiently, crypto, coaching, communications | Only reported direct Ambient concept source | retrieve |
| `bob-stewart/ambientli` | remote repo | GitHub | Ambient.li, Base44, Vite, React | Needed to test whether Ambient has real safety logic | retrieve |
| `bob-stewart/ambient.li` | remote repo | GitHub | Ambient.li, Base44, Vite, React | Needed to resolve canonical Ambient repo | retrieve |

## Open Conflicts

- LiveSafe versus ExoSafe: Round 1 reports ExoSafe as a newer consumer front
  door in 2026-03-22 documents while a 2026-03-23 meeting still discussed
  livesafe.ai as an onboarding or proof-of-concept surface.
- VitalLock scope: Round 1 reports both secure emergency messaging and a
  broader digital legacy architecture. Local EXOCHAIN demo code also presents
  VitalLock as encrypted messaging, assets, family, PACE, and death verification.
- ICE card boundary: ICE appears as a historical standalone site, a legacy
  implementation, an ExoSafe physical artifact, and a LiveSafe QR/NFC feature.
- Ambient evidence gap: Round 1 found Ambient.li and Ambiently references but
  no direct safety, caregiver, passive check-in, or EXOCHAIN integration
  artifact. Local EXOCHAIN "ambient status" UI should not be conflated with the
  Ambient.li product without direct evidence.
- EXOCHAIN strictness versus legacy adapter behavior: current LiveSafe boundary
  requires denial when protected operations lack verified EXOCHAIN authority,
  while the legacy LiveSafe EXOCHAIN client and scan route treat anchoring
  failures as non-fatal.
- Remote source drift: Round 1 mixes Google Drive documents, file-library
  artifacts, private GitHub repository state, and local-style path references.
  Each high-value artifact needs exact retrieval before being merged into the
  product architecture.
- SYBIL and AEGIS specificity: Round 1 found references but not a canonical
  retrieved spec for either concept in the LiveSafe safety architecture.

## Retrieval Backlog

1. Retrieve and compare `bob-stewart/livesafe/app_spec.txt` with the local
   legacy checkout and the current LiveSafe boundary.
2. Retrieve the ExoSafe v1/v2 architecture and GTM documents from Google Drive.
3. Retrieve the AVC Operator Briefing and extract only source-backed decisions.
4. Retrieve `bob-stewart/exochain/crates/exo-gateway/src/livesafe.rs` and
   compare it to local EXOCHAIN schema, gateway, and GAP registry state.
5. Retrieve the VitalLock Google Drive doc and `bob-stewart/VitalLock` README,
   then separate emergency messaging, digital legacy, vault, and PACE concepts.
6. Retrieve legacy ICE card repositories and map reusable concepts versus
   obsolete implementation details.
7. Retrieve Ambient.li repositories and transcript evidence before assigning it
   a product role in LiveSafe.
8. Retrieve EXOCHAIN USI-1, HonorGood, and audit report artifacts to refine
   consent, authority, receipt, and fail-closed constraints.

## Merge Criteria

- A claim can enter product architecture only when it has an artifact pointer,
  an owner/source, and a clear fact-versus-inference label.
- A claim about EXOCHAIN runtime behavior must cite the adapter, gateway,
  schema, and tests that prove denial on rejection or unavailable runtime.
- Health, identity, contact, trustee, location, and emergency profile data must
  remain off-chain except for commitments, hashes, policy references, access
  logs, custody receipts, or equivalent non-raw records.
- Brand decisions must preserve conflicts until Bob explicitly chooses the
  current naming hierarchy or the retrieved artifacts make one hierarchy clear.
