# Claims Remediation Record - 2026-05-09

## Path Classification

| Path | Classification | Rationale |
|---|---|---|
| `crates/exochain-wasm/src/gatekeeper_bindings.rs` | Core runtime adapter | Exposes Rust governance-monitor attestation verification across the WASM boundary. |
| `packages/exochain-wasm/test/bridge_verification.mjs` | Core runtime adapter | Verifies the generated WASM adapter contract. |
| `demo/services/audit-api/src/index.js` | Core runtime adapter | Ingests governance health records and now requires verified attestations before persistence. |
| `demo/services/audit-api/src/index.test.js` | Core runtime adapter | Proves governance-health ingestion fails closed before database writes. |
| `demo/package-lock.json` | Core runtime adapter | Restores reproducible dependency installation for the audit API adapter test workspace. |
| `.github/workflows/ci.yml` | EXOCHAIN core | CI gate contract for WASM export count and bridge validation. |
| `.github/workflows/release.yml` | EXOCHAIN core | Release supply-chain control plane. |
| `Cargo.toml`, `Cargo.lock`, `crates/exo-node/Cargo.toml`, `crates/exo-consensus/Cargo.toml` | EXOCHAIN core | Workspace dependency policy and direct dependency alignment. |
| `governance/threat_matrix.md`, `governance/traceability_matrix.md` | EXOCHAIN core | Canonical governance artifacts. |
| `README.md`, `SECURITY.md`, `VERSIONING.md`, `INTEGRATION.md`, `docs/grant/CODEX-CYBERSECURITY-GRANT-CLAIMS.md`, `docs/audit/DEPENDENCY-HYGIENE-2026-05-09.md` | EXOCHAIN core documentation | Public claim boundary and release/security evidence. |
| `command-base/README.md` | Adjacent surface | Removes constitutional-trust-by-proximity wording from CommandBase. |
| `tools/test_repo_truth.sh`, `tools/test_audit_policy_docs.sh`, `tools/test_dependency_hygiene.sh`, `tools/verify_live_node_claim.sh` | EXOCHAIN core | Source guards for claim drift and verification helpers. |

## Remediation Summary

- Replaced the stale "no release" claim with a precise distinction between
  unsigned pre-release git tags and absent GitHub/crates.io releases.
- Added a non-dry-run signed-tag release gate and aligned security/versioning
  docs to CycloneDX SBOM plus SLSA attestation outputs.
- Implemented T-14 signed-attestation verification across the WASM adapter and
  audit API ingestion path.
- Updated traceability and threat counts to 118 requirements and 16 threats,
  with MON-009 now implemented.
- Scoped invariant, coverage, and adjacent-surface language to tested evidence.
- Verified the live-node health endpoint with a dedicated leak check before
  allowing the claim into grant-facing material.
- Reduced `cargo deny check` duplicate dependency warnings from 26 to 24 and
  added a guard at the reduced count.

## Test Plan

```bash
bash tools/repo_truth.sh --json --skip-tests
bash tools/test_repo_truth.sh
bash tools/test_audit_policy_docs.sh
tools/test_dependency_hygiene.sh
tools/verify_live_node_claim.sh https://exochain-production.up.railway.app
cargo test -p exo-gatekeeper -p exochain-wasm
cargo check -p exo-node -p exo-gateway
wasm-pack build crates/exochain-wasm --target nodejs --out-dir ../../packages/exochain-wasm/wasm
node packages/exochain-wasm/test/bridge_verification.mjs
(cd demo && npm ci --no-audit --no-fund && npx vitest run services/audit-api/src/index.test.js)
```
