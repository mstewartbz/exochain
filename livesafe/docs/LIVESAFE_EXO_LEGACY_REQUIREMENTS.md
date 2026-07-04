# LiveSafe Exo Legacy Requirements

## Source Basis

- Source: Bob Stewart current-thread transfer artifact on 2026-05-24 titled
  `exo-legacy - Claude Code Build Package`.
- Target core path named by the artifact:
  `/Users/bobstewart/dev/exochain/exochain/crates/exo-legacy`.
- Local read-only verification on 2026-05-24: `crates/exo-legacy` is not present
  in the local EXOCHAIN checkout at commit `7a4137f7`.
- Local read-only search found `legacy` terminology in existing EXOCHAIN crates,
  including `exo-economy`, but not the proposed `exo-legacy` crate.
- Classification: pending EXOCHAIN-core transfer artifact and LiveSafe
  dependency requirement. It is not a LiveSafe implementation claim.

## Proprietary IP Classification

The `exo-legacy` transfer package is proprietary project IP. LiveSafe may record
its requirements, dependency boundaries, and verification gates in this private
workspace, but the detailed transfer package, module design, invariants,
implementation prompt, and architecture rationale must not be moved to a public
repository, public issue tracker, public website, or third-party vendor surface
without explicit artifact-level approval from Bob.

## Boundary Decision

LiveSafe must treat `exo-legacy` as a pending core primitive until it exists in
EXOCHAIN, passes its own gates, and exposes a verified adapter or API surface.

This LiveSafe repo must not create substitute EXOCHAIN legacy receipts,
posthumous-representation authority, genetic bequest capabilities, erasure
receipts, or charter validation outcomes. LiveSafe may model adjacent product
requirements and inactive trust-state projections with synthetic fixtures.

## Proposed EXOCHAIN Core Scope

The transfer artifact describes a new `crates/exo-legacy` Rust crate with these
modules:

| Module | Required responsibility |
| --- | --- |
| `charter.rs` | Legacy charter Holon, status state machine, canonical CBOR, BLAKE3 charter hash. |
| `invariants.rs` | I1-I14 predicates and constitutional validation returning invariant violations. |
| `activation.rs` | Quorum and veto-window activation using Shamir and P.A.C.E. primitives. |
| `genetic.rs` | Inheritance bequest, age-18 unlock, steward-unveiling gate, third-party attribution filter. |
| `memory.rs` | Interaction-memory policy, per-interactant deletion, minor guardian rules. |
| `lineage.rs` | Content-addressed downstream-only DAG and rejection of upstream mutation. |
| `persistence.rs` | Sunset policy, mausoleum export descriptor, and recovery model. |
| `constitution_binding.rs` | Ratified-version binding and gated constitution replacement. |
| `erasure.rs` | Crypto-shred by key destruction and erasure receipt. |
| `capability.rs` | Capability tokens for emergency read, genetic unveiling, mausoleum export, and self-retirement. |
| `events.rs` | `legacy.*` attestation event set with hash-only payloads. |

## Core Constraints To Preserve

- No floating-point arithmetic.
- No nondeterministic collections or system clock in production paths.
- No raw PII, charter contents, genetic data, or interaction text on-chain.
- Store only BLAKE3 commitments, capabilities, public keys, and receipts.
- Use canonical CBOR and BLAKE3 for hashing.
- Reuse EXOCHAIN signing primitives.
- Store encrypted blobs off-chain by content id; anchor only content id and
  commitment.
- Treat deletion as crypto-shred through key destruction, not as storage-provider
  deletion guarantees.
- Emergency Tier-0 behavior must not consult quorum or payment state.

## LiveSafe Product Implications

The `exo-legacy` package matters to LiveSafe because LiveSafe already includes
legacy-adjacent product requirements:

- VitalLock lineage and digital legacy framing.
- Medical-jacket custody and bailed-control language.
- The historical ICE packet's transfer, authorization, and rights-assertion
  panels.
- Genetic-profile import as a later genotypical data class.
- P.A.C.E. contact obligations and social recovery.
- Marketplace templates for family readiness and disaster preparedness.
- Future precision-medicine and legacy-related capabilities that require strict
  consent, attribution, revocation, and erasure boundaries.

## LiveSafe Requirements Captured

- LiveSafe must distinguish emergency medical access from legacy/posthumous
  representation.
- LiveSafe must distinguish phenotypical medical records from genotypical data
  and inherited or bequeathed genetic interests.
- LiveSafe must not represent a legacy charter as active unless an EXOCHAIN
  `exo-legacy` adapter verifies it.
- LiveSafe must not expose genetic unveiling, mausoleum export, self-retirement,
  or posthumous-representation capabilities as active until the core primitive
  exists and activation gates pass.
- LiveSafe must model legacy-related product surfaces with inactive trust state,
  explicit consent, synthetic fixtures, and no raw sensitive payloads.
- LiveSafe must keep emergency Tier-0 access separate from payment, quorum, or
  commercial entitlement checks.
- LiveSafe must support future mapping from local product events to `legacy.*`
  attestations, but only through a verified adapter.

## Required Evidence Before Runtime Use

Before LiveSafe can rely on `exo-legacy`, evidence must include:

- `crates/exo-legacy` exists in the EXOCHAIN workspace.
- Workspace `Cargo.toml` includes the crate.
- `tools/repo_truth.sh` includes the crate in regenerated truth output.
- Core build, lib tests, clippy, and formatting gates pass.
- Per-module unit tests exist.
- I1-I14 invariant tests pass.
- Traceability matrix rows map each invariant and lifecycle transition to code
  and tests.
- Reconciliation report exists under EXOCHAIN audit docs.
- A stable adapter or API surface is documented for adjacent products.
- No core code stores raw PII, genetic data, charter contents, or interaction
  text on-chain.

## LiveSafe Acceptance Gates

Before LiveSafe surfaces any `exo-legacy`-dependent feature as active, tests must
prove:

1. Missing `exo-legacy` adapter yields inactive trust state.
2. Missing charter hash yields denial.
3. Failed invariant validation yields denial.
4. Legacy charter contents are never stored in LiveSafe receipt metadata.
5. Genetic data classes are never stored in LiveSafe receipt metadata.
6. Interaction text is never stored in LiveSafe receipt metadata.
7. Emergency Tier-0 access remains independent from payment and quorum state.
8. Erasure status is represented as key-destruction receipt state, not as a
   storage deletion guarantee.
9. Capability labels are inactive unless verified by an adapter response.
10. Product copy avoids posthumous, genetic, or erasure guarantees that are not
    backed by verified code and policy.

## Open Dependencies

- The Legacy Charter v0.3.1-master source artifact has not been imported into
  this repo.
- The I1-I14 invariant definitions are not present in this LiveSafe workspace.
- The proposed `exo-legacy` crate has not been built in the local EXOCHAIN
  checkout.
- The EXOCHAIN reconciliation table requested by the transfer package is not
  available in this thread yet.
- The stable adapter between LiveSafe and `exo-legacy` is not defined.
