# Phase 3 Context Inventory File

## Source Basis

- Source file checked on 2026-05-24:
  `/Users/bobstewart/Downloads/livesafe_exochain_context_inventory.md`.
- File size: 656 lines.
- File title: `LiveSafe / VitalLock / ICE Card / Ambient / EXOCHAIN Context
  Inventory`.
- File description: monolithic markdown transfer artifact from a synthesized
  context inventory.
- Section coverage: A through P, including search scope, direct references,
  integration evidence, artifact inventory, code inventory, code excerpts,
  product architecture clues, schema clues, governance constraints, memory
  records, contradictions, open questions, synthesis, retrieval targets, search
  terms, and transfer compression.
- Local classification: fuller transfer artifact for the same high-signal
  evidence family captured in Round 1. It should be retained as a source
  pointer and compared against retrieved artifacts, not copied wholesale.
- Citation caveat: the file contains external assistant citation markers such
  as `filecite` handles. Those handles are useful evidence that the source
  assistant had referenced artifacts, but they are not directly resolvable from
  this repo by themselves.

## Fact vs Inference

- Fact: the downloaded file contains the strongest all-brand evidence set seen
  so far in the imported phases.
- Fact: the file substantially duplicates the pasted Round 1 content, but with
  a complete monolithic source artifact and more explicit code, schema, and
  governance sections.
- Fact: the file reports strong references for LiveSafe.ai, ExoSafe,
  VitalLock, InCaseOfEmergencyCard.com, ICE card, EXOCHAIN, AEGIS, AVC, CGR
  Kernel, HonorGood, consent, revocation, audit receipts, and authority-chain
  constraints.
- Fact: the file reports weaker evidence for Ambient.li: repository existence
  and a meeting/transcript framing, but no direct safety, passive check-in,
  caregiver, or LiveSafe/EXOCHAIN integration evidence.
- Fact: the file reports that no single artifact directly unifies LiveSafe.ai,
  VitalLock.com, InCaseOfEmergencyCard.com, Ambient.li, and EXOCHAIN.
- Fact: the file includes code excerpts for LiveSafe responder QR/DID parsing,
  LiveSafe scan audit and EXOCHAIN anchoring, EXOCHAIN-side LiveSafe GraphQL
  types, and the LiveSafe Node API EXOCHAIN client bridge.
- Fact: the file includes field-level schema clues for legacy LiveSafe
  operational tables and EXOCHAIN LiveSafe structures.
- Fact: the file identifies the same major open conflicts already captured in
  Round 1: LiveSafe versus ExoSafe, VitalLock product versus module, ICE card
  brand versus feature, Ambient.li naming and role, and legacy fail-soft
  adapter behavior versus strict EXOCHAIN authority expectations.
- Inference: this downloaded file should become the preferred pointer for the
  Round 1-style comprehensive inventory because it is complete and locally
  addressable.
- Inference: the file increases priority for retrieving the exact referenced
  GitHub, Google Drive, and file-library artifacts, because it names many
  concrete documents and code paths.
- Inference: the file does not remove the need to verify remote and connected
  artifacts directly before merging their claims into product architecture.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| Phase 3 context inventory file | imported transfer artifact | `/Users/bobstewart/Downloads/livesafe_exochain_context_inventory.md` | LiveSafe, VitalLock, ICE card, Ambient, EXOCHAIN | Complete local copy of the all-brand context inventory with sections A-P | high | preserve pointer |
| Direct references section | inventory section | lines 28-94 in the source file | LiveSafe, ExoSafe, VitalLock, ICE, Ambient, EXOCHAIN, AEGIS, AVC, CGR Kernel | Most compact source for direct concept references and confidence levels | high | compare |
| Integration evidence section | inventory section | lines 96-113 in the source file | LiveSafe plus EXOCHAIN, VitalLock plus EXOCHAIN, ICE lineage, ExoSafe | Summarizes pairwise integration evidence and status | high | merge after retrieval |
| Artifact inventory section | inventory section | lines 116-151 in the source file | repos, docs, specs, schemas, APIs, audit reports | Names top retrieval targets with claimed source locations | high | retrieve |
| Code inventory section | inventory section | lines 153-216 in the source file | LiveSafe, EXOCHAIN gateway, ice-card, VitalLock, Ambient repos | Consolidates implementation pointers across repositories | high | compare |
| Code excerpts section | inventory section | lines 218-406 in the source file | QR scan, audit receipts, GraphQL gateway, EXOCHAIN client | Shows the most important implementation behavior without copying repos | high | verify against source |
| Product architecture section | inventory section | lines 408-427 in the source file | consumer safety front door, emergency card, vault, first responder, PACE, AI agent, consent ledger | Good product-role extraction for future PRD work | high | merge selectively |
| Schema clues section | inventory section | lines 429-478 in the source file | LiveSafe database tables, EXOCHAIN structures, VitalLock conceptual models | Field-level model map for migration and comparison | high | compare |
| Governance constraints section | inventory section | lines 481-501 in the source file | consent, emergency access, PHI/PII minimization, revocation, human override, fail-closed behavior | Captures safety constraints and current implementation concerns | high | merge |
| Prior decisions section | inventory section | lines 503-517 in the source file | InCaseOfEmergencyCard, LiveSafe, ExoSafe, VitalLock, Ambient, EXOCHAIN governance | Lists historical decisions with dates and source pointers | medium-high | retrieve sources |
| Contradictions section | inventory section | lines 519-532 in the source file | naming conflicts, product boundaries, adapter strictness | Preserves unresolved conflicts instead of smoothing them away | high | keep open |
| Retrieval targets section | inventory section | lines 573-601 in the source file | Drive docs, GitHub repos, audit reports, specs, Ambient repos | Prioritized next retrieval list | high | use |
| Search terms section | inventory section | lines 603-645 in the source file | brand, schema, receipt, authority, governance terms | Useful exact-match search set | high | use |

## Open Conflicts

- This file duplicates much of Round 1. It should strengthen Round 1's source
  basis, not create a competing architecture record.
- The file's citation markers are not direct local files. Exact artifacts still
  need retrieval from GitHub, Google Drive, file library, or local repos before
  claims are considered verified-current.
- The file reports `bob-stewart/livesafe` at ref `02e98af...`, while the local
  predecessor checkout exists at `/Users/bobstewart/dev/demo/livesafe`. These
  should be compared before treating either as canonical.
- The file reports `bob-stewart/exochain` and `exochain/exochain` evidence. The
  current repo boundary still keeps EXOCHAIN core read-only unless Bob
  explicitly asks for core edits.
- The file reports strong ExoSafe strategic evidence, but the current workspace
  is named LiveSafe. Naming remains unresolved.
- The file reports strong LiveSafe and EXOCHAIN integration evidence while also
  identifying fail-soft legacy anchoring behavior. Any commercial product path
  must classify emergency degraded mode versus strict denial before claiming
  governed runtime behavior.
- Ambient remains the weakest part of the architecture in this file. The file
  records repo shells and a meeting concept, not a safety-context integration.

## Retrieval Impact

- Treat `/Users/bobstewart/Downloads/livesafe_exochain_context_inventory.md` as
  the local transfer artifact for the comprehensive inventory.
- Use the line ranges in this record to retrieve specific sections quickly.
- Keep Round 1 as the normalized summary and use this file as the fuller source
  artifact behind it.
- Prioritize exact retrieval of `app_spec.txt`, `server/routes/scan.js`,
  `server/utils/exochain-client.js`, `crates/exo-gateway/src/livesafe.rs`,
  `ULTRAPLAN-GAP-007-LIVESAFE.md`, ExoSafe v1/v2 Drive docs, the AVC Operator
  Briefing, the VitalLock README/doc, and the legacy ICE card repo files.
