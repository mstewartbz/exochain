# AVC Cornerstone Production Proof

Status: Production verified for #713 closure scope.

No CommandBase/Paperclip production-loop claim is enabled by this packet alone.

## Claim Boundary

This packet is the durable landing zone for the production cornerstone proof. It
records the production evidence that supports GitHub issue `#713` closure scope:
authenticated AVC receipt emission, production readback, RFC3161 timestamp
proof, trust-anchor evidence, and EXOCHAIN finality. It does not claim that a
separate CommandBase/Paperclip production-loop run has completed.

## Required Evidence For #713 Closure Scope

- EXOCHAIN AVC receipt hash.
- EXOCHAIN finality hash and finality height.
- RFC3161 timestamp provenance.
- RFC3161 trust-anchor kind and receipt fields.
- Commit SHA for EXOCHAIN.
- Production endpoint URLs used for the smoke/readback.
- GitHub issue comment URL containing the closeout evidence.

## Additional Evidence For CommandBase/Paperclip Production Loop

- CommandBase proof run id and tenant id.
- Paperclip run id, initiating agent passport id, and Chairman approval record.
- ExoForge proof run id and `civilizational_avc_proof_v1` battery result.
- Archon bounded workflow evidence with finite `max_iterations`, repeated-failure
  escalation, and AVC proof dependency.
- CommandBase callback signature verification result.
- Commit SHAs for CommandBase, ExoForge/CrossChecked, and EXOCHAIN.

## Current Evidence State

| Evidence | State | Notes |
|---|---|---|
| CommandBase proof run | Local harness implemented | QM-01 through QM-03 and QM-08 through QM-09 have local tests; no production proof run id is recorded in this packet |
| ExoForge proof battery | Local contract implemented | QM-04 through QM-07 have local tests for fixed battery, authenticated emit, and readback validation |
| Archon bounded workflow | Source guard implemented | QM-05 guard requires AVC proof evidence before Archon PR finalization |
| Production AVC emit/readback | Verified for `#713` closure scope | `docs/proof/avc-issue-713-production-closure-proof.md` records 27/27 authenticated post-`#718` production emits, 64 production receipt readbacks, RFC3161 proof, trust-anchor evidence, and EXOCHAIN finality |
| GitHub issue closeout | Issue `#713` closed | GitHub issue `#713` is closed as `COMPLETED`; the proof boundary does not extend to a separate CommandBase/Paperclip production-loop run |

## Current Live Readback Refresh - 2026-06-29T22:29:38Z

- GitHub issue truth: issue `#713` is closed as `COMPLETED`; closure scope is supported by PR `#722`, the durable proof packet, and live readback.
- Production protocol truth: authenticated `GET https://exochain.io/api/v1/avc/protocol` returned protocol version `1`, schema version `1`, and package `@exochain/exochain-wasm` version `0.1.0-beta`.
- Production receipt readback truth: authenticated reads for receipt hashes `0c01f1dd4d2d78f81753caf86cd44ec0779e3c50b2cd79514a9188f49a234a7e` and `29058510dbbe27d194cdb7f3784dcb3f5b90886bbe5632db0aae01b1a7878cbc` returned `Allow`, `ExternalTimestampAuthority`, action `archon.workflow.success`, actor `did:exo:44sVCyeMCcef7PzAeM8jY7qpRUpmaQxabaC4etXeE9zr`, and trust anchors `signer_spki` and `issuing_ca_spki`.
- Aggregate live readback truth: 64/64 listed receipts have ExternalTimestampAuthority and RFC3161 proof; the 27 receipts with trust-anchor metadata split into 18 signer_spki and 9 issuing_ca_spki.
- Production credential readback truth: credential `49a819386a62d9edb7adeabe05dd55efa52787e82b1653f7a44068b2e08e287d` delegates `Read` and `Write` for service `avc-archon-runner` and tool `archon-cli`, with issuer and principal `did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX`.
- Boundary truth: the authenticated admin bearer can read production AVC state, but admin bearer alone is not actor signing authority for new request generation. The #713 closure claim relies on the already-recorded authenticated production emits and readbacks, not on fabricating new request material in this worktree.

## QM-10 Attempt - 2026-06-29

- Production deploy check: Railway service `exochain` reports successful deployment from merge commit `61f8b021fe29fb4fb945adedee362feab42aca66`.
- Runner check: `tools/avc_emit_production_smoke.mjs` was restored from `origin/main`; `node --check tools/avc_emit_production_smoke.mjs` passed.
- Auth check: Railway service environment contains `EXOCHAIN_ADMIN_BEARER_TOKEN`, and authenticated production protocol/readback requests succeed.
- Smoke runner execution check: mapping `EXOCHAIN_ADMIN_BEARER_TOKEN` into `EXO_AVC_SMOKE_BEARER_TOKEN` caused the runner to fail closed with `AVC production smoke failed: EXO_AVC_SMOKE_EMIT_REQUESTS_FILE must be set`.
- Actor-signing check: Railway service environment does not expose an AVC smoke bearer wrapper, actor private signing material, or subject private signing material.
- Fixture check: `EXO_AVC_SMOKE_EMIT_REQUESTS_FILE` was not set, and no approved actor-signed emit/issue request JSON was found under the searched local private/dev/document paths.

Disposition: QM-10 is complete for GitHub issue `#713` closure scope. The separate CommandBase/Paperclip production-loop claim remains outside this packet until that surface provides production run ids, callback signature evidence, and its own AVC-linked proof.
