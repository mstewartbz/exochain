# Phase 5 Exo Safe Card Runtime Import

## Source Basis

- Source: user-provided Phase 5 document pasted in the current Codex thread on
  2026-05-24.
- Claimed Phase 5 sources: Claude memory, prior Claude chats, Google Drive,
  Notion, and one live fetch of `github.com/exochain/exochain` main.
- Local verification performed in this Codex session against
  `/Users/bobstewart/dev/exochain/exochain`, whose origin is
  `https://github.com/exochain/exochain.git`.
- Remote verification performed with `git ls-remote`: remote `main` is
  `7a4137f74aa2996428c10b85d6e0adc7166df733`, matching the local checkout.
- Local README verification: current repo status reports 23 Rust crates, 312
  Rust source files, 201868 Rust LOC, 4,530 listed workspace tests, 22 CI gates,
  Apache-2.0 license, and no published GitHub release or crates.io package.
- Local command verification: `find crates -mindepth 1 -maxdepth 1 -type d`
  found 23 crate directories; `find . -name '*.rs'` found 315 Rust files across
  the repo; `git rev-list --count HEAD` returned 1385 commits.
- Local exact-string verification: `rg -i 'exo-safe|exosafe'` across README,
  crates, docs, governance, and terms returned no matches.
- Local crate verification: README maps `exo-identity` to DID, key management,
  Shamir secret sharing, and vault; `crates/exo-identity/src/lib.rs` exposes
  DID, DID verification, risk, Shamir, PACE, key management, registry, vault,
  and verification modules.
- Local test verification: `crates/exo-identity/tests/livesafe_integration.rs`
  names LiveSafe.ai to EXOCHAIN integration tests for PACE and Shamir APIs.
- Local terms verification: `TERMS-CONDITIONS-AND-PRIVACY.md` exists with 234
  lines, but targeted search did not find card, QR, first-responder, LiveSafe,
  InCase, driver-license, or activation language in that file.
- Classification: imported Phase 5 correction pack with partial local
  verification. Claims from Claude chats, Google Drive, Notion, exoforge,
  SENTIENTS, live websites, and card artwork/T&C remain retrieval targets.

## Fact vs Inference

- Fact: Phase 5 introduces a new correction that `exo-safe` should be treated
  as a core concept, not a separate consumer brand and not a literal crate.
- Fact: local verification found no literal `exo-safe` or `exosafe` string in
  the checked EXOCHAIN repo surfaces.
- Fact: local verification supports mapping the concept to `exo-identity` only
  as an inference: README and source modules show DID, key management, Shamir,
  PACE, and vault primitives.
- Fact: local verification found LiveSafe-specific PACE and Shamir integration
  tests in `crates/exo-identity/tests/livesafe_integration.rs`.
- Fact: Phase 5 reports a user correction that the physical emergency card is a
  website/product artifact on `livesafe.ai` and `incaseofemergencycard.com`,
  not a runtime repo artifact.
- Fact: Phase 5 reports the card as a personalized foldable item with
  personalized QR activation, first-responder retrieval, network activation,
  and Bob-authored card-back terms copy.
- Fact: local verification did not find card-specific wording in the EXOCHAIN
  root terms file, so `TERMS-CONDITIONS-AND-PRIVACY.md` remains only a
  candidate terms source until compared with the websites and exact card copy.
- Fact: Phase 5 reports current live repo metrics as 15 crates, 148 Rust files,
  about 31k LOC, 1,116 library tests, and 116 commits.
- Fact: local and remote verification for `exochain/exochain` main contradicts
  those metrics: README reports 23 crates, 312 Rust source files, 201868 LOC,
  4,530 listed workspace tests, and 22 CI gates; local git reports 1385 commits
  at the same remote `main` HEAD.
- Fact: Phase 5 introduces or emphasizes retrieval targets not fully represented
  in earlier records: `exochain/exoforge`, `exochain/SENTIENTS`, the live card
  websites, root `TERMS-CONDITIONS-AND-PRIVACY.md`, CyberMedica GTM, the
  GREENFIELD super-prompt, and FlexLaw.AI emergency/QR code.
- Fact: local search found several exoforge directories under
  `/Users/bobstewart/dev`, including `/Users/bobstewart/dev/exoforge`,
  `/Users/bobstewart/dev/exochain/exoforge`, and
  `/Users/bobstewart/dev/exochain/exochain/exoforge`.
- Inference: Phase 5 is the strongest imported source for distinguishing the
  physical card product from the ICE-PACE cryptographic/recovery mechanism.
- Inference: `exo-identity`, `exo-consent`, `exo-gatekeeper`,
  `exo-authority`, and `exo-gateway` are plausible runtime homes for the card
  retrieval path, but exact routes and token schemas are not verified.
- Inference: Ambient.li remains adjacent rather than integrated, because Phase
  5 still only ties it through emergency/PACE contact concepts.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| Phase 5 transfer pack | imported context pack | current Codex thread, pasted 2026-05-24 | LiveSafe, VitalLock, ICE, Ambient, EXOCHAIN, exo-safe, physical card | Adds corrections around exo-safe, card location, metrics drift, and AVC ambiguity | high for reported claims | preserve |
| `exochain/exochain` current main | repo | `/Users/bobstewart/dev/exochain/exochain`, remote `https://github.com/exochain/exochain.git`, HEAD `7a4137f74aa2996428c10b85d6e0adc7166df733` | EXOCHAIN, exo-identity, gateway, consent, authority, AVC, HonorGood | Local/remote verified runtime evidence and metric source | high | compare |
| EXOCHAIN README repo status | doc | `/Users/bobstewart/dev/exochain/exochain/README.md` lines 27-60 | crate count, source count, tests, gates, no floating point | Corrects Phase 5 metric claims with local-current README numbers | high | use current |
| EXOCHAIN README crate table | doc | `/Users/bobstewart/dev/exochain/exochain/README.md` lines 150-173 | exo-identity, exo-consent, exo-gatekeeper, exo-avc, exo-economy | Supports the runtime primitive map and AVC/HonorGood locations | high | use |
| `exo-identity` module map | Rust code | `/Users/bobstewart/dev/exochain/exochain/crates/exo-identity/src/lib.rs` lines 17-38 | DID, risk, Shamir, PACE, key management, vault | Best local evidence for the exo-safe concept mapping | high | inspect deeper |
| LiveSafe identity tests | Rust tests | `/Users/bobstewart/dev/exochain/exochain/crates/exo-identity/tests/livesafe_integration.rs` lines 17-60 | LiveSafe, PACE, Shamir | Direct local LiveSafe plus EXOCHAIN integration evidence | high | compare |
| Root terms file | policy doc | `/Users/bobstewart/dev/exochain/exochain/TERMS-CONDITIONS-AND-PRIVACY.md` | general Exochain Foundation terms | Candidate card terms source from Phase 5, but local search did not confirm card-specific wording | medium | verify against card sites |
| `exoforge` local directories | repo/workspaces | `/Users/bobstewart/dev/exoforge`, `/Users/bobstewart/dev/exochain/exoforge`, `/Users/bobstewart/dev/exochain/exochain/exoforge` | ExoForge, LiveSafe, PACE, 0dentity | Phase 5 says exoforge hosts the LiveSafe/PACE/0dentity layer | medium | retrieve |
| `exochain/SENTIENTS` | repo | imported Phase 5 pointer | AEGIS, SYBIL, sentient charters, DIDs | Reported governance/identity artifact not opened locally in this pass | medium | retrieve |
| `livesafe.ai` | website | imported Phase 5 pointer | physical card, activation, terms | Reported product surface for card UI and QR activation | medium | retrieve |
| `incaseofemergencycard.com` | website | imported Phase 5 pointer | physical card, first-responder retrieval, terms | Reported dedicated physical card surface | medium | retrieve |
| CyberMedica GTM | Drive doc | imported Phase 5 pointer `1UJVlgDF9uVtujVLgMtqhVGqzQlEU5O_BSe2ELmUfxug` | PHI/PII vault, PACE, 0dentity | Candidate source for medical/vital-record schema | medium | retrieve |
| GREENFIELD super-prompt | chat/design prompt | imported Phase 5 pointer `c1cd4b59` | VitalLock, ICE-PACE, InCaseOfEmergency, invariants | Reported design bundle for legacy brands | medium | retrieve |
| FlexLaw.AI codebase | code | imported Phase 5 pointer `c097a530` | emergency notify, QR, biometric | Possible reusable pattern but not LiveSafe-labeled | low | verify before use |

## Open Conflicts

- Repo metrics conflict: Phase 5 reports 15 crates, 148 Rust files, about 31k
  LOC, 1,116 library tests, and 116 commits; current remote main and local
  README report 23 crates, 312 Rust source files, 201868 LOC, 4,530 listed
  workspace tests, and local git reports 1385 commits.
- Phase 5 says live main was fetched on 2026-05-24, but remote `main` currently
  matches local HEAD and local evidence contradicts the Phase 5 metric table.
- Phase 5 says CR-001 AEGIS/SYBIL is ratified, while the locally verified
  README still says CR-001 is draft and pending council ratification. This
  needs resolution against `governance/resolutions/INDEX.md` and the current
  governance files.
- Phase 5 treats `TERMS-CONDITIONS-AND-PRIVACY.md` as a candidate card-back
  terms source, but local search shows general foundation/software terms, not
  the card-specific copy described by the user.
- `exo-safe` is a user-stated concept with no literal crate or local string
  match. The `exo-identity` mapping is strong but still an inference until Bob
  names that mapping or a source artifact does.
- The physical card and its activation flow are asserted as product intent and
  website content, but the websites were not retrieved in this pass.
- Ambient.li remains weakly integrated: Phase 5 reports a `usePaceContacts`
  hook and emergency contacts, but no direct EXOCHAIN or LiveSafe runtime link.
- AVC remains overloaded between Autonomous Volition Credential and Apex
  Velocity Catalysts; use the full phrase in architecture records.
- AEGIS definition and invariant count are reported as drifted across sources;
  keep the current source and date with every use.

## Retrieval Impact

- Promote `crates/exo-identity/src/`, `crates/exo-gateway/src/`,
  `crates/exo-api/src/`, and EXOCHAIN migrations to immediate retrieval targets
  for vault, PACE, card/QR, and first-responder route verification.
- Retrieve and compare the full root terms file against `livesafe.ai` and
  `incaseofemergencycard.com` terms before treating it as card-back copy.
- Retrieve the most relevant local exoforge directory first:
  `/Users/bobstewart/dev/exochain/exoforge` or `/Users/bobstewart/dev/exoforge`,
  then compare with any remote `exochain/exoforge` source.
- Search for exact terms: `exo-safe`, `QR activation`, `first-responder retrieval`,
  `network activation`, `TERMS-CONDITIONS-AND-PRIVACY`, `usePaceContacts`,
  `PACEConfigured`, `RecoveryRequest`, `AbortRecovery`, and `KeyRotated`.
- Keep Phase 5 as a correction pack: useful for product boundaries and retrieval
  priority, but not authoritative for repo metrics where local-current evidence
  disagrees.
