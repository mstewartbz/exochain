# Threat Model & Test Matrix

Based on Spec Section 13.

| ID | Threat | Mitigation (Code) | Test Strategy | Status |
|---|---|---|---|---|
| **T-01** | **Key Exfiltration** | HSM Support (Trait), Ed25519 storage limits | Unit: KeyStore defines secure traits. Ops: Policy. | 🔴 Planned |
| **T-02** | **Score Replay** | Nonce + Audience Binding in `RiskAttestation` | Unit: `verify_attestation` checks nonce/audience. | 🔴 Planned |
| **T-03** | **BFT Liveness** | 2f+1 Quorum checks in `exo-dag` | Integration: Simulate network partition. | 🔴 Planned |
| **T-04** | **Sybil Attack** | DID Derivation cost + Risk Scoring | Unit: DID uniqueness checks. | 🔴 Planned |
| **T-05** | **Vault Breach** | Client-Side Encryption (XChaCha20) | Crypto: Test vectors for decryption. | 🔴 Planned |
| **T-06** | **Eclipse Attack** | Peer auth + min ASNs (P2P layer) | Network: `libp2p` config validation. | 🔴 Planned |
| **T-07** | **Replay (Events)** | `HLC` + Event ID Uniqueness | Proptest: Attempt replay of events. | 🔴 Planned |
| **T-08** | **Sig Forgery** | Ed25519 (Dalek) | Fuzzing: Signature verification inputs. | 🔴 Planned |
| **T-09** | **HLC Manipulation** | HLC `skew` checks (Spec 9.2) | Unit: `HLC::new_event` strict ordering. | 🔴 Planned |
| **T-10** | **DoS API** | Rate Limiting (API layer) | Load Test: Send 1000 req/s. | 🔴 Planned |
| **T-11** | **Admin Bypass** | Gatekeeper TEE requirement | Design Review: No "root" keys in code. | 🔴 Planned |
| **T-12** | **Holon Key Theft** | TEE-only key generation | (Hardware dependency) - Mock TEE checks. | 🔴 Planned |
| **T-13** | **Capability Esc.** | CGR Kernel Invariants (INV-002) | Unit: `ActionVerifier` rejects self-grant. | 🔴 Planned |

## Security Policies
1. **Dependency Audit**: `cargo-deny` config to ban permissive licenses and high CVSS.
2. **Fuzzing**: continuous fuzzing on `EventEnvelope` deserializers.
3. **No Unsafe**: `#![forbid(unsafe_code)]` in `exo-core` (unless strictly audited FFI).
