# Phase 3 VitalLock EXOCHAIN Memory Import

## Source Basis

- Source: user-provided Phase 3 document pasted in the current Codex thread on
  2026-05-24.
- Claimed source set: internal memory, prior conversation
  `95180778-49a0-4187-bec5-8daf0de8fdea` dated 2026-04-10, and a
  user-provided artifact inventory.
- Claimed unavailable sources for Phase 3: private repositories, Notion,
  Google Drive, code artifacts, local project files, design tools, uploaded
  files, connected documents, and email/calendar.
- Local memory corroboration in this Codex session: `MEMORY.md` names
  `95180778-49a0-4187-bec5-8daf0de8fdea` as a high-signal retrieval handle for
  VitalLock plus EXOCHAIN integration intent, with Rust/WASM, trustee/PACE,
  keystore, and key-sharding relevance.
- Classification: imported memory evidence. The underlying April 10 transcript
  is still a retrieval target and has not been loaded into this repo.

## Fact vs Inference

- Fact: Phase 3 reports one relevant prior chat:
  `95180778-49a0-4187-bec5-8daf0de8fdea`, dated 2026-04-10.
- Fact: Phase 3 reports direct references for VitalLock, EXOCHAIN, and PACE
  contacts in that prior chat.
- Fact: Phase 3 reports a VitalLock-to-EXOCHAIN mapping, keystore strengthening
  with 0dentity scoring and key sharding, Shamir secret sharing, trustee roles,
  a 1:4 user-to-PACE growth model, constitutional governance, and a Rust/WASM
  preference.
- Fact: Phase 3 reports no accessible code, schemas, repositories, designs,
  diagrams, database tables, or config artifacts.
- Fact: Phase 3 reports no direct references in its accessible sources for
  LiveSafe.ai, ExoSafe, InCaseOfEmergencyCard.com, ICE card, QR/NFC emergency
  access, golden-hour emergency access, Ambient.li, AEGIS, SYBIL, CGR Kernel,
  AVC, consent fabric, authority chain, provenance, revocation, trust receipts,
  HonorGood, personal safety mesh, vital records vault, emergency health data
  vault, or protected emergency data access.
- Fact: local memory corroborates that the same conversation id is important
  for VitalLock plus EXOCHAIN, but does not provide the full transcript.
- Fact: local Round 1 evidence already identifies LiveSafe, VitalLock, and
  EXOCHAIN demo code and local repo artifacts, so Phase 3's "no code available"
  statement is scoped to that assistant's access.
- Inference: Phase 3 strengthens the VitalLock plus EXOCHAIN lineage more than
  the LiveSafe, ICE card, or Ambient lineage.
- Inference: the 0dentity scoring plus Shamir key-sharding detail may be an
  early keystore-hardening requirement that should be compared against the
  local VitalLock demo crypto and key material controls.
- Inference: EXOFORGE and LYNK Protocol should be searched as adjacent terms,
  but Phase 3 does not provide enough detail to assign them a LiveSafe role.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| Phase 3 document | imported memory summary | current Codex thread, pasted 2026-05-24 | VitalLock, EXOCHAIN, PACE, 0dentity, Shamir, Rust/WASM | Adds a focused prior-chat claim set centered on VitalLock plus EXOCHAIN | high for reported claims | preserve |
| April 10 prior conversation | chat transcript | convo id `95180778-49a0-4187-bec5-8daf0de8fdea` | VitalLock, EXOCHAIN, EXOFORGE, 0dentity scoring, key sharding, PACE, Shamir, constitutional governance | Reported as the only direct internal record for this phase and a high-signal retrieval target in local memory | medium until retrieved | retrieve |
| Local memory registry entry | memory index | `/Users/bobstewart/.codex/memories/MEMORY.md` | LiveSafe, VitalLock, Ambient, EXOCHAIN, conversation id, Rust/WASM, key sharding, PACE contacts | Corroborates that the conversation id and concepts were previously surfaced in cross-assistant context gathering | high for index existence | use as retrieval pointer |
| Local VitalLock demo app | code | `/Users/bobstewart/dev/exochain/exochain/demo/apps/vitallock` | VitalLock, PACE, encrypted messages, key controls | Local artifact to compare against the imported Rust/WASM and key-sharding claims | high for local existence | compare |
| Local VitalLock demo API | code | `/Users/bobstewart/dev/exochain/exochain/demo/services/vitallock-api` | VitalLock, PACE, death verification, family, keys | Local service to compare against prior-chat trustee and keystore design intent | high for local existence | compare |
| Local VitalLock SQL | database schema | `/Users/bobstewart/dev/exochain/exochain/demo/infra/postgres/init/004_vitallock.sql` | profiles, PACE, encrypted messages, assets, family, death verification | Local schema to compare against prior-chat product mapping | high for local existence | compare |
| Local VitalLock crypto module | code | `/Users/bobstewart/dev/exochain/exochain/demo/apps/vitallock/src/lib/crypto.ts` | passphrase keys, Shamir, WASM bridge | Candidate implementation evidence for the Phase 3 key-sharding claim | high for local existence | verify |
| EXOFORGE term | search term | Phase 3 imported text | EXOCHAIN adjacent builder/runtime concept | Appears in Phase 3 architecture clues but lacks artifact detail | low | search |
| LYNK Protocol term | search term | Phase 3 imported text | possible adjacent protocol | Appears only in search-term list and has no direct explanation here | low | search |

## Open Conflicts

- Phase 3 says only VitalLock, EXOCHAIN, and PACE had direct references in its
  accessible context. Round 1 and local repo inspection already found stronger
  LiveSafe and VitalLock code evidence. Treat this as an access-scope gap.
- Phase 3 labels VitalLock plus EXOCHAIN as canonical in its own status field,
  but this repo should keep it as imported memory evidence until the April 10
  transcript is retrieved.
- Phase 3 says no contradictions or duplicates were identified. Across all
  imported phases, there are still unresolved naming and product-boundary
  conflicts around LiveSafe versus ExoSafe, VitalLock as product versus module,
  ICE card as brand versus feature, and Ambient.li as standalone brand versus
  context layer.
- Phase 3 reports no governance constraints beyond high-level constitutional
  governance and trustee roles. Current LiveSafe boundary rules still require
  explicit consent, raw-data minimization, adapter evidence, and denial on
  missing verified authority.
- Phase 3 has no implementation artifacts, while the local EXOCHAIN demo tree
  has VitalLock implementation artifacts. The transcript should be compared
  against those local files before making architecture decisions.

## Retrieval Impact

- Promote the April 10 transcript
  `95180778-49a0-4187-bec5-8daf0de8fdea` to a high-priority retrieval target.
- Add targeted searches for `0dentity scoring`, `keystore`, `key sharding`,
  `Shamir secret sharing`, `EXOFORGE`, and `LYNK Protocol`.
- Compare the prior-chat keystore design with the local VitalLock crypto module
  and security guards before reusing any design claim.
- Keep the Phase 3 VitalLock integration evidence separate from LiveSafe,
  ICE card, and Ambient decisions until those concepts have their own retrieved
  source artifacts.
