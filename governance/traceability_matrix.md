# Traceability Matrix

| Spec Section | Requirement | Component / Crate | Test Location | Coverage Status |
|---|---|---|---|---|
| **9.1** | **Event Hashing** | `exo-core::event` | `exo-core/tests/hashing.rs` | 🔴 Planned |
| | BLAKE3(canonical_cbor) | `exo-core::crypto` | `tools/cross-impl-test` | 🔴 Planned |
| | Ed25519 Signing | `exo-core::crypto` | `exo-core/tests/signing.rs` | 🔴 Planned |
| **9.2** | **Hybrid Logical Clock** | `exo-core::hlc` | `exo-core/src/hlc.rs` (mod tests) | 🔴 Planned |
| | Causality Logic | `exo-dag` | `exo-dag/tests/causality.rs` | 🔴 Planned |
| **9.3** | **Policy Structure** | `exo-consent::policy` | `exo-consent/src/policy.rs` | 🔴 Planned |
| **9.4** | **Checkpoints** | `exo-dag::checkpoint` | `exo-dag/tests/checkpoint.rs` | 🔴 Planned |
| | Split Roots (MMR/SMT) | `exo-dag::proofs` | `exo-dag/tests/proofs.rs` | 🔴 Planned |
| **9.5** | **RiskAttestation** | `exo-identity::risk` | `exo-identity/tests/attestation.rs` | 🔴 Planned |
| **10.1** | **DID Derivation** | `exo-identity::did` | `exo-identity/src/did.rs` | 🔴 Planned |
| **12.0** | **Gatekeeper TEE** | `exo-gatekeeper` | `exo-gatekeeper/tests/mock.rs` | 🔴 Planned |
| **16.1** | **AC: Ledger Node** | `exo-dag` | `tests/integration/ledger_node.rs` | 🔴 Planned |
| **16.2** | **AC: Identity Svc** | `exo-identity` | `tests/integration/identity_svc.rs` | 🔴 Planned |
| **16.3** | **AC: Consent Svc** | `exo-consent` | `tests/integration/consent_svc.rs` | 🔴 Planned |
| **16.7** | **AC: Benchmarks** | Workspace `benches` | `benches/performance.rs` | 🔴 Planned |

## Detailed Acceptance Criteria Map (Section 16)

| AC ID | Description | Automated Test |
|---|---|---|
| 16.1.1 | `append` validates parent, sig, HLC | `tests/ledger.rs::test_append_validation` |
| 16.1.2 | `verify_integrity` recursive check | `tests/ledger.rs::proptest_integrity` |
| 16.2.1 | `create_identity` derivation | `tests/identity.rs::test_creation` |
| 16.2.2 | `rotate_key` proof validation | `tests/identity.rs::test_rotation` |
| 16.3.1 | `propose_bailment` event creation | `tests/consent.rs::test_bailment` |
| 16.7.1 | Append Latency < 5ms p99 | `benches/ledger_ops.rs` |
