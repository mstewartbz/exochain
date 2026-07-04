# Phase 11 Exo Legacy Build Package

## Source Basis

- Source: Bob Stewart current-thread transfer artifact on 2026-05-24 titled
  `exo-legacy - Claude Code Build Package`.
- The artifact instructs Claude Code to work in
  `/Users/bobstewart/dev/exochain/exochain/` and scaffold a new
  `crates/exo-legacy` Rust crate.
- This Codex thread is operating in `/Users/bobstewart/dev/livesafe`, an
  EXOCHAIN-adjacent product repo.
- Local read-only verification: `/Users/bobstewart/dev/exochain/exochain` is on
  branch `main` at short commit `7a4137f7`, and `crates/exo-legacy` is not
  present.
- Local read-only search found existing `legacy` terminology in EXOCHAIN source,
  including `exo-economy`, but no proposed `exo-legacy` crate.

## Fact vs Inference

- Fact: the transfer artifact proposes a new EXOCHAIN core crate named
  `exo-legacy`.
- Fact: the proposed crate implements posthumous-representation governance as
  EXOCHAIN-native types.
- Fact: the proposed crate modules include charter, invariants, activation,
  genetic, memory, lineage, persistence, constitution binding, erasure,
  capability, and events.
- Fact: the artifact requires a Phase 0 reconciliation before coding: run
  `tools/repo_truth.sh`, verify counts, run build/test/clippy/format gates,
  report licensing, and summarize selected existing EXOCHAIN modules.
- Fact: the artifact requires no EXOCHAIN `main` commit and expects branch-based
  review through AEGIS/CGR and AI-IRB process.
- Fact: the artifact says this chat should receive the reconciliation table,
  per-module API summaries, and gate results after Claude Code runs it.
- Fact: local read-only verification shows the crate does not currently exist
  in the checked local EXOCHAIN workspace.
- Inference: LiveSafe should track this as a pending EXOCHAIN-core dependency
  for legacy, genetic, posthumous-representation, erasure, and mausoleum export
  features.
- Inference: LiveSafe should not implement substitute core authority or receipt
  semantics for `exo-legacy`; it should model inactive adjacent requirements
  until verified core evidence exists.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| `exo-legacy` build package | current-chat transfer artifact | Codex thread, 2026-05-24 | Legacy Charter, posthumous representation, genetics, erasure, lineage, capabilities | Defines the intended EXOCHAIN core crate and review process | high | preserve |
| Proposed `crates/exo-legacy` | pending core crate | `/Users/bobstewart/dev/exochain/exochain/crates/exo-legacy` | I1-I14, charter validation, legacy events | Core dependency required before LiveSafe can rely on legacy primitives | medium until built | verify later |
| LiveSafe exo-legacy requirements | requirements doc | `/Users/bobstewart/dev/livesafe/docs/LIVESAFE_EXO_LEGACY_REQUIREMENTS.md` | adjacent boundary, inactive trust state, dependency evidence | Converts the transfer package into LiveSafe-safe requirements | high | use |
| Existing EXOCHAIN checkout | read-only evidence repo | `/Users/bobstewart/dev/exochain/exochain`, commit `7a4137f7` | EXOCHAIN core | Verified that the proposed crate is not present locally | high | compare after Claude Code run |

## Requirements Captured

- Track `exo-legacy` as a pending core primitive, not as current LiveSafe runtime
  evidence.
- Keep LiveSafe emergency access separate from legacy and posthumous
  representation.
- Keep LiveSafe genetic and medical data classes separate.
- Do not store raw charter contents, genetic data, interaction memory, PII, or
  emergency data in receipts or on-chain payloads.
- Treat crypto-shred as key-destruction evidence, not as a storage-provider
  deletion guarantee.
- Represent legacy-related capabilities as inactive unless verified by a real
  EXOCHAIN adapter.
- Preserve emergency Tier-0 access independence from quorum and payment checks.

## Product Architecture Impact

- VitalLock and LiveSafe legacy-oriented features need a distinct dependency
  boundary from emergency-card and medical-jacket features.
- Marketplace templates touching memorialization, inheritance, genetics,
  posthumous representation, family stewardship, or memory policies must remain
  gated until `exo-legacy` evidence exists.
- The historical transfer-on-death and rights-assertion card panels should not
  be treated as current legal product behavior without a current policy source
  and verified implementation.
- The LiveSafe implementation map should add a legacy-charter dependency slice
  and an adapter-readiness gate.

## Open Conflicts

- The Legacy Charter v0.3.1-master source text is referenced but not included in
  this repo.
- I1-I14 invariant definitions are referenced but not available locally.
- The proposed `exo-legacy` crate has not been built in this environment.
- The EXOCHAIN reconciliation table and gate results requested by the transfer
  package have not been returned to this thread.
- The eventual LiveSafe adapter shape for `exo-legacy` is undefined.
