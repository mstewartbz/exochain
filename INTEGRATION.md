# EXOCHAIN WASM Integration Contract

## Overview

The EXOCHAIN WASM bridge compiles the Rust constitutional trust fabric into a
WebAssembly package for JavaScript consumers. The current public bridge is
source-counted by CI Gate 22 at **157 Rust `#[wasm_bindgen]` exports** and
smoke-tested by the Node bridge verification harness before the aggregator gate
can pass.

The bridge is a core runtime adapter. Consumers may present EXOCHAIN trust
claims only when they call the relevant WASM or core API path and tests prove
fail-closed behavior when the adapter rejects, times out, or is unavailable.

## Source And Artifacts

- Rust source: `crates/exochain-wasm/src/`
- Generated Node package: `packages/exochain-wasm/wasm/`
- Bridge verification harness: `packages/exochain-wasm/test/bridge_verification.mjs`
- CI gates: `.github/workflows/ci.yml` Gates 20, 21, and 22

## Verification

```bash
cargo test -p exochain-wasm
wasm-pack build crates/exochain-wasm --target nodejs --out-dir ../../packages/exochain-wasm/wasm
node packages/exochain-wasm/test/bridge_verification.mjs
```

## Adapter Boundary

WASM consumers must not mint, cache, or simulate consent, authority,
provenance, governance outcomes, settlement authority, or constitutional
invariant results outside the Rust adapter. Adjacent surfaces such as
CommandBase and ExoForge remain adjacent unless the runtime path invokes the
tested adapter and the surface has its own fail-closed tests.

## Governance Monitoring Attestation

Continuous governance monitoring uses the Rust governance-monitor verifier
through the WASM bridge:

- `wasm_governance_findings_digest(findings_json)` computes the canonical
  findings digest.
- `wasm_verify_governance_attestation(signer_did, findings_json,
  signature_json, signer_public_key_hex)` verifies that the signed envelope
  matches the submitted findings before ingestion.

The audit API rejects missing, mismatched, or invalid attestations before any
database write. This completes the T-14 adapter path and aligns the threat
matrix with the implementation.
