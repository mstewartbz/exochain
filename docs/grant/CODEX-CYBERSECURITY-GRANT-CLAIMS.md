<!--
Copyright 2026 Exochain Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at:

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

SPDX-License-Identifier: Apache-2.0
-->

# Codex Cybersecurity Grant Claims

This file is the grant-facing claim boundary for the current repository state.
Public grant material should use these claims or weaker wording until a newer
repo-truth run and verification record supersede this document.

## Included Claims

- EXOCHAIN is a Rust constitutional trust-fabric workspace with 22 crates,
  300 Rust source files, and 178,725 tracked Rust LOC in the remediation branch.
- The traceability matrix tracks 118 requirements: 116 implemented, 0 partial,
  and 2 planned.
- The threat matrix tracks 16 threats: 16 implemented, 0 partial, and 0
  planned.
- Constitutional invariants are enforced in the tested gatekeeper and
  decision-forum adjudication paths.
- The WASM bridge is a core runtime adapter with 157 Rust `#[wasm_bindgen]`
  exports counted by CI Gate 22 and smoke-tested by the bridge verification
  harness.
- Governance-monitor health ingestion requires bearer authentication, a
  findings digest, and a Rust-verified Ed25519 attestation before persistence.
- Coverage is a scoped CI coverage gate. The default `tarpaulin.toml` coverage
  gate explicitly excludes runtime adapters, WASM bridge bindings, and proof
  modules.
- Supply-chain policy is enforced through `cargo audit`, `cargo deny check`,
  documented advisory exceptions in `deny.toml`, CycloneDX SBOM generation, and
  SLSA build-attestation workflow configuration.
- Formal non-dry-run releases require an existing signed `v<version>` tag before
  release artifacts or crates.io publication can proceed.
- Live node health for `https://exochain-production.up.railway.app/health` was
  verified on 2026-05-09 with `tools/verify_live_node_claim.sh`.

## Excluded Claims

- Do not claim a published GitHub Release or crates.io release. The repository
  currently has unsigned pre-release git tags (`v0.1.0-alpha`, `v0.1.0-beta`)
  and no formal `v0.1.0` release tag.
- Do not claim every release tag is currently signed. The workflow now enforces
  signed tags for non-dry-run formal releases.
- Do not claim the advisory set is empty. Use: "policy-enforced with documented
  advisory exceptions."
- Do not claim live-node production status for any URL other than the verified
  URL above unless `tools/verify_live_node_claim.sh <node-url>` passes for the
  exact URL cited in the proposal.
- Do not claim adjacent surfaces such as CommandBase are under EXOCHAIN
  constitutional authority unless the cited action has a tested core API or
  verified adapter path.
- Do not claim constitutional invariants apply to all governance paths without
  qualifying the tested gatekeeper and decision-forum adjudication paths.

## Verification Commands

```bash
bash tools/repo_truth.sh --json --skip-tests
bash tools/test_repo_truth.sh
bash tools/test_audit_policy_docs.sh
tools/test_dependency_hygiene.sh
cargo test -p exo-gatekeeper -p exochain-wasm
wasm-pack build crates/exochain-wasm --target nodejs --out-dir ../../packages/exochain-wasm/wasm
node packages/exochain-wasm/test/bridge_verification.mjs
(cd demo && npm ci --no-audit --no-fund && npx vitest run services/audit-api/src/index.test.js)
```
