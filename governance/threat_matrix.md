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

# Threat Model & Test Matrix

Based on Spec Section 13. Updated 2026-06-18 after DAG DB opt-in adapter gate review.

**Status key:** 🟢 Implemented (tests passing) | 🟡 Partial (core logic exists, gaps remain) | 🔴 Planned

## Threat Matrix

| ID | Threat | Mitigation (Code) | Crate(s) | Tests | Status |
|---|---|---|---|---|---|
| **T-01** | **Key Exfiltration** | `KeyStore` with `zeroize`-on-drop, key rotation, revocation, status tracking | `exo-identity/key_management.rs` | 15 unit | 🟢 Implemented |
| **T-02** | **Score Replay** | `verify_attestation()` validates attester key binding; `RiskAttestation` with subject DID, level, expiry | `exo-identity/risk.rs` | 8 unit | 🟢 Implemented |
| **T-03** | **BFT Liveness** | 2f+1 quorum via `quorum_size()`, `propose()`/`vote()`/`check_commit()`/`commit()` cycle, duplicate vote rejection, non-validator rejection | `exo-dag/consensus.rs` | 12 unit | 🟢 Implemented |
| **T-04** | **Sybil Attack** | 6 sub-threat taxonomy: `verify_independence()` checks signing keys, attestation chains, control metadata; `detect_coordination()` timing analysis (100ms threshold); independence-aware quorum counting | `exo-governance/crosscheck.rs`, `exo-governance/quorum.rs`, `exo-escalation/escalation.rs` | 11 + 12 + 10 unit | 🟢 Implemented |
| **T-05** | **Vault Breach** | `VaultEncryptor` with XChaCha20-Poly1305 AEAD, HKDF-SHA256 key derivation, DID-bound associated data, zeroize-on-drop | `exo-identity/vault.rs` | 9 unit | 🟢 Implemented |
| **T-06** | **Eclipse Attack** | `RateLimiter` + `AsnPolicy` with min ASN diversity, `select_diverse_peers()` round-robin, `rotate_peers()` stale eviction, unknown ASN grouped as single bucket | `exo-api/p2p.rs` | 31 unit | 🟢 Implemented |
| **T-07** | **Replay (Events)** | `HybridClock` monotonic `now()`, causal `update()` merge, `ClockDrift` rejection, injectable wall clock for testing | `exo-core/hlc.rs` | 15 unit | 🟢 Implemented |
| **T-08** | **Sig Forgery** | Real `ed25519_dalek::VerifyingKey::verify()` in authority chain verification; `SignerType` prefix binding (human 0x01 / AI 0x02) in signed payload | `exo-core/crypto.rs`, `exo-authority/chain.rs`, `exo-gatekeeper/mcp.rs` | 10 + 5 adversarial | 🟢 Implemented |
| **T-09** | **HLC Manipulation** | `MAX_DRIFT_MS` (5000ms) enforcement, `ClockDrift` error on forward skew, monotonic guarantee on backward skew | `exo-core/hlc.rs` | 15 unit | 🟢 Implemented |
| **T-10** | **DoS API** | `RateLimiter` with per-peer counting (100 req cap), reset capability | `exo-api/p2p.rs` | 2 unit | 🟢 Implemented |
| **T-11** | **Admin Bypass** | CGR Kernel immutability (constitution hash verified on every `adjudicate()`), `SeparationOfPowers` invariant, `NoSelfGrant` invariant, `KernelImmutability` invariant | `exo-gatekeeper/kernel.rs`, `exo-gatekeeper/invariants.rs` | 14 + 30 unit | 🟢 Implemented |
| **T-12** | **Holon Key Theft** | TEE attestation with `TeeEnvironment` production gate; `Simulated` rejected in Production; `#[cfg(not(feature = "allow-simulated-tee"))]` compile-time strip; secure-by-default | `exo-gatekeeper/tee.rs` | 28 unit | 🟢 Implemented |
| **T-13** | **Capability Esc.** | CGR Kernel `adjudicate()` with `NoSelfGrant` invariant; authority chain scope narrowing; AI delegation ceiling (`TNC-09`) | `exo-gatekeeper/kernel.rs`, `exo-authority/chain.rs`, `decision-forum/tnc_enforcer.rs` | 14 + 19 + 13 unit | 🟢 Implemented |
| **T-14** | **Governance Monitor Poisoning** | Adversarial manipulation of continuous-governance monitoring output to trigger false-positive self-improvement cycles, potentially introducing malicious changes under cover of remediation. Sub-threats: (a) Unsigned findings injection — mitigated by requiring a BLAKE3/Ed25519 signed attestation envelope on all `POST /governance/health` payloads before storage (`exo-gatekeeper::governance_monitor::verify_attestation` exposed through `wasm_verify_governance_attestation`); (b) Automated AI->AI bypass — mitigated by human-DID approval gate (SignerType 0x01 required) via `ApprovalGate::approve()` before self-improvement cycle may begin implementation; (c) Circuit-breaker flooding — mitigated by `GovernanceCircuitBreaker` auto-pause when >3 Critical findings recorded within 24h; (d) Credential compromise — mitigated by `GOVERNANCE_API_TOKEN` bearer auth on all health endpoints + read-only ExoForge credential scoping per `exo-identity` key rotation policy. Detection signals: unexpected spike in Critical findings, governance score regression >=2 grades in one scan, self-improvement cycle triggered without matching approval record, audit chain break on health snapshot sequence. | `exo-gatekeeper/governance_monitor.rs`, `crates/exochain-wasm/src/gatekeeper_bindings.rs`, `demo/services/audit-api/src/index.js`, `demo/infra/postgres/init/003_governance_health.sql` | Rust attestation verification tests; WASM bridge valid/invalid attestation tests; audit API fail-closed tests for missing, mismatched, and rejected attestations | 🟢 Implemented |
| **T-15** | **Unratified Provenance Settlement** | A downstream surface or agent could attempt to convert recognition-only upstream provenance into economic settlement without accepted terms, ratification, materiality evidence, or authority. Mitigations: `LegacyReceipt` state machine rejects direct `Proposed -> Ratified`; Archon/Paperclip fixtures remain `Proposed` with `VoluntaryRecognitionOnly`; automated settlement requires accepted terms, valid authority envelope, active contribution node, active ruleset, sufficient legal effect, non-disputed materiality, and checked per-basis allocation. Opaque beneficiary references prevent sensitive payment or estate data from being placed on-ledger. | `exo-economy::legacy`, `exo-economy::honorgood`, `exo-economy::settlement` | module unit tests for state transitions, fixtures, automated settlement rejection, zero-launch settlement | 🟢 Implemented |
| **T-16** | **Adjacent Surface Settlement Claim** | CommandBase or ExoForge could imply EXOCHAIN settlement authority by proximity or create local settlement state during proposal/UI workflows. Mitigations: both surfaces have intake records; adapters fail closed when EXOCHAIN API configuration is absent; adapter responses state local settlement authority is false; CommandBase proxies to EXOCHAIN economy routes; ExoForge proposals stay unratified and submissions go to `/api/v1/economy/*`; TypeScript SDK and WASM bridge expose core routes/anchors without payment execution. | `command-base/app/services/honorgood-economy.js`, `command-base/app/routes/honorgood-economy.js`, `exoforge/lib/honorgood.js`, `exoforge/bin/exoforge-honorgood.js`, `packages/exochain-sdk/src/client.ts`, `crates/exochain-wasm/src/economy_bindings.rs` | adapter tests, SDK tests, WASM anchor tests | 🟢 Implemented |
| **T-17** | **DAG DB Governed Memory Writeback Bypass** | An opt-in DAG DB caller could try to persist or retrieve graph memory without tenant-scoped consent, actor provenance, or constitutional gate checks. Mitigations in this PR: `/api/v1/dag-db/*` remains default-off unless `exo-gateway/production-db` and a Postgres pool are configured; served writeback fails closed without DB authority; import/export fail closed with `403 consent_denied` until distinct import/export consent exists; `DagDbGatekeeperService` enforces active bailment/consent, Ed25519 write signatures over canonical payload hashes, and the constructible invariant subset (`ConsentRequired`, `SeparationOfPowers`, `NoSelfGrant`, `HumanOverride`, `KernelImmutability`, `QuorumLegitimate`) before writeback persistence. DORMANT PRD-D5 persistence methods are gated at the method boundary but are not served routes yet. | `crates/exo-gatekeeper/src/dagdb_gate.rs`, `crates/exo-gateway/src/dagdb.rs`, `crates/exo-dag-db-postgres/src/postgres/*` | `dagdb_gate` consent/signature/invariant tests; `dagdb_routes_integration_contract`; `dagdb_cross_tenant_denies_every_get_and_post_route`; `writeback_authorizes_against_real_db_consent_and_identity_state` | 🟡 Partial |

## Summary

| Status | Count | Threats |
|--------|-------|---------|
| 🟢 Implemented | 16 | T-01, T-02, T-03, T-04, T-05, T-06, T-07, T-08, T-09, T-10, T-11, T-12, T-13, T-14, T-15, T-16 |
| 🟡 Partial | 1 | T-17 |
| 🔴 Planned | 0 | — |

## Resolved Remediation Tickets

All three remediation tickets have been resolved and closed:

- **#11 (T-05):** Resolved in commit `0371a4b` — VaultEncryptor with XChaCha20-Poly1305, HKDF-SHA256, AEAD binding to DID, zeroize-on-drop. 9 tests.
- **#12 (T-06):** Resolved in commit `0371a4b` — AsnPolicy with diversity enforcement, round-robin selection, stale peer rotation. 9 tests.
- **#13 (T-12):** Resolved in commit `0371a4b` — TeeEnvironment production gate, compile-time feature flag, secure-by-default. 9 tests.

## Security Policies

1. **Dependency Audit**: `cargo-deny` config (`deny.toml`) bans copyleft licenses, denies known vulnerabilities, bans OpenSSL (pure-Rust crypto only).
2. **Fuzzing**: Continuous fuzzing planned on `EventEnvelope` deserializers and signature verification inputs.
3. **No Unsafe**: `#![forbid(unsafe_code)]` enforced across all crates.
4. **Determinism**: `#[deny(clippy::float_arithmetic)]` workspace-wide; `BTreeMap` only (no `HashMap`); HLC for all temporal ordering.
5. **Post-Quantum**: `Signature` enum supports `Ed25519`, `PostQuantum`, and `Hybrid` variants for migration readiness.
