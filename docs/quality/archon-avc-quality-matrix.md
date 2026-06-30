# Archon AVC Quality Matrix

Archon is a bounded workflow surface, not a constitutional proof authority.
QM-10 is the only slice that can enable a production cornerstone proof claim.
EXOCHAIN core is the source of AVC receipt truth.

## Path Classification

- Adjacent surface: `.archon` workflows, ExoForge workflow documentation, and command templates.
- Core runtime adapter: workflow evidence accepted by the ExoForge AVC gate.
- EXOCHAIN core: AVC emit/readback, finality, RFC3161 timestamp proof, and trust-anchor verification.
- Imported evidence: workflow run output, production smoke logs, GitHub issue comments, and callback payloads.

## Slice Matrix

| Slice ID | Subsystem | Trust Classification | Claim Enabled | Required Failing Tests | Green Implementation Evidence | Runtime/Smoke Evidence | Durable Artifact | Completion Gate | Open Risk | Next Slice |
|---|---|---|---|---|---|---|---|---|---|---|
| `QM-01` | CommandBase initiation | Adjacent surface | Archon recognizes CommandBase as initiation context only | Source guard rejects workflow text that treats CommandBase as proof authority | Guard test passes | No runtime evidence for this row | CommandBase matrix link | Initiation context is separated from proof authority | CommandBase implementation separate | `QM-02` |
| `QM-02` | CommandBase receipt blocking | Adjacent surface | Archon evidence cannot unblock CommandBase without AVC proof | Guard rejects docs that equate workflow success with receipt verification | Guard test passes | No runtime evidence for this row | Matrix row | Docs preserve blocking language | Runtime block lives in CommandBase | `QM-03` |
| `QM-03` | CommandBase to ExoForge dispatch | Core runtime adapter | Workflow evidence can be requested by ExoForge | Guard rejects missing ExoForge dispatch boundary | Guard test passes | No runtime evidence for this row | Matrix row | Workflow remains downstream from ExoForge request | Dispatch implementation separate | `QM-04` |
| `QM-04` | ExoForge battery definition | Adjacent surface | Archon proof step belongs to fixed battery id | Guard rejects arbitrary caller-controlled battery wording | Guard test passes | No runtime evidence for this row | Battery row link | Fixed battery is named | Battery registry implementation separate | `QM-05` |
| `QM-05` | Archon bounded workflow | Adjacent surface | Archon workflow is bounded and cannot self-approve | RED observed: stricter guard failed because `.archon/commands/exoforge-avc-proof-evidence.md` and `avc_proof_evidence` dependency were missing | GREEN: `bash tools/test_archon_avc_quality_matrix.sh` passes with `create_pr` gated on `avc_proof_evidence.output.verified == 'true'`, `self_approval: forbidden`, and EXOCHAIN readback as trusted authority | Workflow source evidence only | `.archon/workflows/exochain-self-improvement-cycle.yaml` and `.archon/commands/exoforge-avc-proof-evidence.md` | `create_pr` depends on AVC proof evidence, not direct validation output | Runtime workflow execution separate | `QM-06` |
| `QM-06` | ExoForge AVC emit | Core runtime adapter | Archon evidence can be included in ExoForge AVC action payload | Guard rejects claiming Archon emits AVC proof directly | Guard test passes | No runtime evidence for this row | Matrix row | Wording keeps emit responsibility in ExoForge/EXOCHAIN path | ExoForge emit implementation separate | `QM-07` |
| `QM-07` | EXOCHAIN readback validation | EXOCHAIN core | Archon evidence is subordinate to EXOCHAIN readback | Guard rejects finality or RFC3161 claims without EXOCHAIN proof packet | Guard test passes | No runtime evidence for this row | Proof packet shell | Readback fields named but not claimed complete | Live readback separate | `QM-08` |
| `QM-08` | CommandBase callback | Core runtime adapter | Archon evidence can appear in ExoForge callback payload | Guard rejects direct Archon to CommandBase verification claim | Guard test passes | No runtime evidence for this row | Matrix row | Callback remains ExoForge-signed | Callback implementation separate | `QM-09` |
| `QM-09` | Paperclip heartbeat harness | Adjacent surface | Paperclip run id may initiate bounded Archon work | Guard rejects continuous or unbounded heartbeat wording | Guard test passes | No runtime evidence for this row | Matrix row | Heartbeat remains finite and attributable | Paperclip e2e separate | `QM-10` |
| `QM-10` | Production cornerstone smoke | Imported evidence | Production cornerstone proof claim may be made for #713 closure scope | RED observed: guard failed while this branch lacked `docs/proof/avc-issue-713-production-closure-proof.md` and preserved a stale blocked packet | QM-10 completed for #713 closure scope: guard requires the closure proof packet, live readback counts, and issue closure truth | Production readback verifies 64/64 RFC3161 receipts; trust anchors split 18 signer_spki and 9 issuing_ca_spki among the 27 post-#718 receipts | `docs/proof/avc-cornerstone-production-proof.md` and `docs/proof/avc-issue-713-production-closure-proof.md` | Receipt hash, finality, RFC3161, trust anchor, and commit SHAs are recorded before the #713 closure claim | CommandBase/Paperclip production-loop run ids remain separate | Keep CommandBase/Paperclip e2e proof outside the #713 closure claim |

## Operating Rules

- Archon workflows must declare finite loop bounds and escalation.
- Archon output is untrusted workflow node output until validated.
- Archon may not create the proof claim; it supplies bounded workflow evidence.
- Production wording outside the `#713` closure scope stays blocked until the
  corresponding surface has its own AVC-linked proof.
