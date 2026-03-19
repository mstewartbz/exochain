# EXOCHAIN v2.2

> **Trust Fabric for the Digital Economy**
>
> *Spec Revision 2.2 — Green-Field Implementation*

EXOCHAIN is a verifiable, privacy-preserving substrate enabling secure identity adjudication, data sovereignty, and deterministic finality.

## Repository Structure

*   **`crates/exo-core`**: Cryptographic primitives (BLAKE3, Ed25519), Canonical CBOR, and HLC.
*   **`crates/exo-dag`**: Directed Acyclic Graph engine, Checkpointing, and BFT consenus adapter.
*   **`crates/exo-identity`**: Decentralized Identity (DID), Key Management, and RiskAttestation.
*   **`crates/exo-consent`**: Bailment contracts, Policies, and Gatekeeper enforcement logic.
*   **`crates/exo-gatekeeper`**: TEE / Enclave interfaces and attestation verification.
*   **`governance/`**: Project governance, sub-agent charters, traceability matrices, and quality gates.

## Governance & Compliance

This repository is managed under strict **Judicial Build Governance**. All contributions must align with `EXOCHAIN_Specification_v2.2.pdf`.

*[Traceability Matrix](governance/traceability_matrix.md) | [Threat Model](governance/threat_matrix.md) | [Quality Gates](governance/quality_gates.md)*

## Getting Started

### Prerequisites

*   Rust 1.75+
*   Clang (for crypto extensions)

### Build & Test

```bash
cargo build
cargo test
```

### Contributing

See `governance/quality_gates.md` for strict PR requirements.
1.  **Safety**: No PII on ledger. No admins.
2.  **Quality**: 80% coverage required. No Clippy warnings.
3.  **Process**: All PRs must map to a Spec requirement.

## License

Apache-2.0 - See [LICENSE](LICENSE).
