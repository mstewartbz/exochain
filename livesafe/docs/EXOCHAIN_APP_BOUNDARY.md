# EXOCHAIN App Boundary

## Source Basis

- Current thread: Bob Stewart identified LiveSafe as a private commercial
  venture under `github.com/bob-stewart/livesafe`.
- Current thread: Bob Stewart identified LiveSafe as an EXOCHAIN-adjacent app
  surface for `github.com/exochain/exochain`.
- Local evidence: `/Users/bobstewart/dev/exochain/AGENTS.md` defines adjacent
  surface intake, no trust claim by proximity, and core regression rules.
- Local evidence: `/Users/bobstewart/dev/exochain/Cargo.toml` lists the
  EXOCHAIN Rust workspace crates.
- Local evidence: `/Users/bobstewart/dev/exochain/packages/exochain-sdk` and
  `/Users/bobstewart/dev/exochain/packages/exochain-wasm` exist as package
  surfaces.

## Classification

| Path | Classification | Editing Rule |
| --- | --- | --- |
| `/Users/bobstewart/dev/livesafe` | adjacent surface | owned private LiveSafe work |
| `/Users/bobstewart/dev/exochain` | EXOCHAIN core and adjacent evidence | read-only unless Bob explicitly asks |
| `context/inbox` | imported evidence | preserve source basis and classify before use |
| `context/canon` | normalized context record | source-backed records only |

## Proprietary IP Boundary

Bob classified the gathered LiveSafe, VitalLock, InCaseOfEmergencyCard, Ambient,
EXOCHAIN-adjacent, `exo-legacy`, AI help, feedback, agent, onboarding, P.A.C.E.,
medical-jacket, content-addressed storage, marketplace, entitlement, and
emergency-card architecture as project IP on 2026-05-24 and 2026-05-25.

Detailed architecture, transfer packages, implementation prompts, and
source-backed requirements remain private unless Bob approves an exact artifact
for public release. The executable local policy lives in `src/ip-boundary.ts`;
the written handling policy lives in `docs/IP_HANDLING.md`.

Public-domain constitutional and civic source text is handled separately from
project IP. "We the People" is a U.S. Constitution Preamble source phrase. The
"of the people, by the people, for the people" civic formula is from the
Gettysburg Address. LiveSafe may use those phrases only with correct source
provenance and without implying governmental authority or legal enforcement by
rhetoric alone.

## Genesis Development Trust

As of Bob's 2026-05-25 direction, LiveSafe may trust ExoForge for internal
development efforts because that is ExoForge's purpose: development planning,
implementation workflow, review routing, validation support, and bounded build
execution.

That internal trust does not authorize external trust signaling. LiveSafe must
not make public, customer-facing, legal, medical, EXOCHAIN runtime, root,
settlement, ratification, custody, consent, authority, or provenance claims
until the specific internal proof gate passes. The current genesis profile is a
scheduled 7-of-13 FROST keygen ceremony during the week of 2026-05-25. The
executable local rule lives in `src/genesis-trust.ts`; the written policy lives
in `docs/GENESIS_DEVELOPMENT_TRUST.md`.

Outward trust state must be visually symbolic and baked into the output. The
AVC badge, lock-style symbol, color treatment, glow treatment, status text, and
machine-readable state are mandatory on trust-bearing public and customer
surfaces. The visual-language rule lives in `src/trust-signal.ts`; the written
policy lives in `docs/TRUST_SIGNAL_VISUAL_LANGUAGE.md`.

## Initial Intake Record

| Field | Value |
| --- | --- |
| Owner | `bob-stewart` |
| Repository | `github.com/bob-stewart/livesafe` |
| Release status | prototype |
| EXOCHAIN trust claims allowed | no |
| Runtime adapter | not wired |
| EXOCHAIN core read access | none |
| EXOCHAIN core write access | none |
| Test command | `npm run quality` |
| Secrets inventory | none in repo |
| Disablement path | no runtime adapter exists in this scaffold |

Machine-readable intake lives in `config/surface-intake.json`.

## EXOCHAIN Primitive Evidence

The initial registry in `config/exochain-primitives.json` records evidence-only
dependencies against the local EXOCHAIN repo:

- `crates/exo-core`
- `crates/exo-identity`
- `crates/exo-consent`
- `crates/exo-authority`
- `crates/exo-gatekeeper`
- `crates/exo-dag`
- `crates/exo-proofs`
- `crates/exo-api`
- `crates/exo-gateway`
- `crates/exo-messaging`
- `crates/exo-avc`
- `crates/exo-economy`
- `crates/exochain-sdk`
- `packages/exochain-sdk`
- `packages/exochain-wasm`

These paths prove local evidence exists. They do not prove LiveSafe runtime
enforcement.

## Pending EXOCHAIN Primitive: `exo-legacy`

Bob provided an `exo-legacy` transfer package on 2026-05-24 for a proposed
EXOCHAIN core crate at `crates/exo-legacy`. Local read-only verification against
`/Users/bobstewart/dev/exochain/exochain` at commit `7a4137f7` shows that this
crate is not present yet.

LiveSafe must treat `exo-legacy` as a pending core dependency for legacy,
posthumous-representation, genetic bequest, memory-policy, lineage,
persistence, erasure, and legacy capability features. No LiveSafe route, UI, or
receipt may claim active `exo-legacy` verification until the crate exists,
passes EXOCHAIN gates, and exposes a verified adapter.

## Safety Mesh Context

Memory-derived context currently frames the product vocabulary this way:

- LiveSafe: safety network
- VitalLock: protected vault
- InCaseOfEmergencyCard: emergency access artifact
- Ambient: always-on context layer
- EXOCHAIN: consent, access, revocation, custody, commitment, policy-reference,
  access-log, and receipt fabric

Treat this as retrieval context until Bob provides the next source dump.

## Data Boundary

Raw sensitive personal, medical, genetic, safety, identity, trustee, location,
contact, PACE, emergency-access, and vault data stays off-chain and outside
content-addressed storage unless it is encrypted before provider write.

After a verified adapter exists, eligible EXOCHAIN records are limited to:

- content-addressed references
- commitments
- hashes
- policy references
- access logs
- custody receipts

## Runtime Claim Gate

LiveSafe may only claim EXOCHAIN runtime enforcement when all of these are true:

1. A runtime path invokes the relevant EXOCHAIN core API or verified adapter.
2. Tests prove denial when EXOCHAIN rejects the action.
3. Tests prove denial when EXOCHAIN times out or is unavailable.
4. LiveSafe cannot mint, cache, or simulate consent, authority, provenance,
   custody, revocation, governance, or receipt outcomes outside EXOCHAIN.
5. Health, debug, telemetry, and error responses do not disclose secrets,
   authority chains, private keys, bootstrap tokens, tenant data, or raw
   sensitive records.
6. For root-backed claims, the 7-of-13 FROST keygen ceremony has completed and
   the exact claim is backed by internal proof evidence.
