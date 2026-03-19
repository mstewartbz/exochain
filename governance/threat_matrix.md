# Threat Model & Test Matrix

Based on Spec Section 13. Updated 2026-03-19 after council-driven implementation.

**Status key:** 🟢 Implemented (tests passing) | 🟡 Partial (core logic exists, gaps remain) | 🔴 Planned

## Threat Matrix

| ID | Threat | Mitigation (Code) | Crate(s) | Tests | Status |
|---|---|---|---|---|---|
| **T-01** | **Key Exfiltration** | `KeyStore` with `zeroize`-on-drop, key rotation, revocation, status tracking | `exo-identity/key_management.rs` | 15 unit | 🟢 Implemented |
| **T-02** | **Score Replay** | `verify_attestation()` validates attester key binding; `RiskAttestation` with subject DID, level, expiry | `exo-identity/risk.rs` | 8 unit | 🟢 Implemented |
| **T-03** | **BFT Liveness** | 2f+1 quorum via `quorum_size()`, `propose()`/`vote()`/`check_commit()`/`commit()` cycle, duplicate vote rejection, non-validator rejection | `exo-dag/consensus.rs` | 12 unit | 🟢 Implemented |
| **T-04** | **Sybil Attack** | 6 sub-threat taxonomy: `verify_independence()` checks signing keys, attestation chains, control metadata; `detect_coordination()` timing analysis (100ms threshold); independence-aware quorum counting | `exo-governance/crosscheck.rs`, `exo-governance/quorum.rs`, `exo-escalation/escalation.rs` | 11 + 12 + 10 unit | 🟢 Implemented |
| **T-05** | **Vault Breach** | XChaCha20 referenced in integration test only | `exo-identity/tests/` | 0 dedicated | 🟡 Partial — see [GH-14](#gh-14) |
| **T-06** | **Eclipse Attack** | `RateLimiter` per-peer rate limiting, `verify_message()` with nonce replay protection, message signature verification | `exo-api/p2p.rs` | 2 + 6 unit | 🟡 Partial — see [GH-15](#gh-15) |
| **T-07** | **Replay (Events)** | `HybridClock` monotonic `now()`, causal `update()` merge, `ClockDrift` rejection, injectable wall clock for testing | `exo-core/hlc.rs` | 15 unit | 🟢 Implemented |
| **T-08** | **Sig Forgery** | Real `ed25519_dalek::VerifyingKey::verify()` in authority chain verification; `SignerType` prefix binding (human 0x01 / AI 0x02) in signed payload | `exo-core/crypto.rs`, `exo-authority/chain.rs`, `exo-gatekeeper/mcp.rs` | 10 + 5 adversarial | 🟢 Implemented |
| **T-09** | **HLC Manipulation** | `MAX_DRIFT_MS` (5000ms) enforcement, `ClockDrift` error on forward skew, monotonic guarantee on backward skew | `exo-core/hlc.rs` | 15 unit | 🟢 Implemented |
| **T-10** | **DoS API** | `RateLimiter` with per-peer counting (100 req cap), reset capability | `exo-api/p2p.rs` | 2 unit | 🟢 Implemented |
| **T-11** | **Admin Bypass** | CGR Kernel immutability (constitution hash verified on every `adjudicate()`), `SeparationOfPowers` invariant, `NoSelfGrant` invariant, `KernelImmutability` invariant | `exo-gatekeeper/kernel.rs`, `exo-gatekeeper/invariants.rs` | 14 + 30 unit | 🟢 Implemented |
| **T-12** | **Holon Key Theft** | TEE attestation with `TeePlatform` enum (SGX, TrustZone, SEV, Simulated); platform policy, measurement hash, signature, age checks | `exo-gatekeeper/tee.rs` | 19 unit | 🟡 Partial — see [GH-16](#gh-16) |
| **T-13** | **Capability Esc.** | CGR Kernel `adjudicate()` with `NoSelfGrant` invariant; authority chain scope narrowing; AI delegation ceiling (`TNC-09`) | `exo-gatekeeper/kernel.rs`, `exo-authority/chain.rs`, `decision-forum/tnc_enforcer.rs` | 14 + 19 + 13 unit | 🟢 Implemented |

## Summary

| Status | Count | Threats |
|--------|-------|---------|
| 🟢 Implemented | 10 | T-01, T-02, T-03, T-04, T-07, T-08, T-09, T-10, T-11, T-13 |
| 🟡 Partial | 3 | T-05, T-06, T-12 |
| 🔴 Planned | 0 | — |

## Open Remediation Tickets

<a id="gh-14"></a>
### GH-14: T-05 Vault Breach — Client-Side Encryption Module

**Priority:** P1 | **Assigned:** Council Review
**Gap:** XChaCha20-Poly1305 encryption referenced in integration test but no runtime encryption/decryption module exists.
**Required:**
- [ ] `exo-identity/src/vault.rs` — `VaultEncryptor` with `encrypt()` / `decrypt()` using XChaCha20-Poly1305
- [ ] Key derivation from `SecretKey` via HKDF
- [ ] Authenticated encryption with associated data (AEAD) binding to DID
- [ ] Test: encrypt-then-decrypt round-trip
- [ ] Test: tampered ciphertext fails authentication
- [ ] Test: wrong key fails decryption
- [ ] Test: AEAD binding to wrong DID fails

<a id="gh-15"></a>
### GH-15: T-06 Eclipse Attack — ASN Diversity Enforcement

**Priority:** P1 | **Assigned:** Council Review
**Gap:** `RateLimiter` and `verify_message()` exist but no ASN (Autonomous System Number) diversity enforcement for peer discovery. An attacker controlling multiple peers in the same ASN could eclipse a node.
**Required:**
- [ ] `exo-api/src/p2p.rs` — `AsnPolicy` struct with `min_unique_asns` threshold
- [ ] Peer metadata to include ASN (or IP-to-ASN lookup)
- [ ] `discover_peers()` to enforce minimum ASN diversity before accepting peer set
- [ ] Periodic peer rotation to prevent long-term eclipse
- [ ] Test: reject peer set with all peers from single ASN
- [ ] Test: accept peer set meeting diversity threshold
- [ ] Test: peer rotation replaces stale peers

<a id="gh-16"></a>
### GH-16: T-12 Holon Key Theft — Production TEE Gate

**Priority:** P0 | **Assigned:** Council Review
**Gap:** `TeePlatform::Simulated` produces deterministic blake3-based signatures. No production-mode gate prevents simulated attestation from being accepted in production environments.
**Required:**
- [ ] `exo-gatekeeper/src/tee.rs` — `TeeEnvironment` enum (`Production`, `Testing`)
- [ ] `TeePolicy` to reject `Simulated` platform when environment is `Production`
- [ ] Compile-time feature flag: `#[cfg(not(feature = "allow-simulated-tee"))]` to strip `Simulated` from release builds
- [ ] Test: `Simulated` attestation rejected in `Production` environment
- [ ] Test: `Simulated` attestation accepted in `Testing` environment
- [ ] Test: real platform (SGX/TrustZone/SEV) accepted in both environments
- [ ] Audit: ensure no code path can set environment to `Testing` at runtime in release builds

## Security Policies

1. **Dependency Audit**: `cargo-deny` config (`deny.toml`) bans copyleft licenses, denies known vulnerabilities, bans OpenSSL (pure-Rust crypto only).
2. **Fuzzing**: Continuous fuzzing planned on `EventEnvelope` deserializers and signature verification inputs.
3. **No Unsafe**: `#![forbid(unsafe_code)]` enforced across all crates.
4. **Determinism**: `#[deny(clippy::float_arithmetic)]` workspace-wide; `BTreeMap` only (no `HashMap`); HLC for all temporal ordering.
5. **Post-Quantum**: `Signature` enum supports `Ed25519`, `PostQuantum`, and `Hybrid` variants for migration readiness.
