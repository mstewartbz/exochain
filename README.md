# EXOCHAIN v2.2

> **Trust Fabric for the Digital Economy**
>
> *Spec Revision 2.2 — Green-Field Implementation*

EXOCHAIN is a verifiable, privacy-preserving substrate enabling secure identity adjudication, data sovereignty, and deterministic finality. Built in Rust with absolute determinism, constitutional governance, and post-quantum readiness.

**29,587 lines of Rust | 14 crates | 136 source files | 1,116 tests | 0 failures**

## The Five Axioms

1. **Identity Adjudication** — Decentralized identity with cryptographic proof, not administrative fiat.
2. **Data Sovereignty** — Data belongs to its subject; consent is explicit, revocable, and auditable.
3. **Deterministic Finality** — Every operation is reproducible and verifiable; no floating-point arithmetic permitted.
4. **Constitutional Governance** — All system behavior is governed by ratified resolutions (CR-001 AEGIS/SYBIL framework).
5. **Authentic Plurality** — Sybil-resistant participation ensuring one-entity-one-voice integrity.

## Repository Structure

### Core Crates

| Crate | LOC | Description |
|-------|-----|-------------|
| `exo-core` | 4,360 | Cryptographic primitives (BLAKE3, Ed25519, post-quantum), Canonical CBOR, HLC |
| `exo-governance` | 4,308 | Constitutional governance engine, AEGIS framework, council process |
| `exo-gatekeeper` | 3,193 | TEE/Enclave interfaces, attestation verification, kernel invariants, holon, MCP |
| `exo-dag` | 2,883 | Directed Acyclic Graph engine, BFT consensus adapter, checkpointing, HLC |
| `exo-gateway` | 2,135 | External gateway interfaces and API routing |
| `exo-identity` | 2,034 | Decentralized Identity (DID), key management, Shamir secret sharing, vault |
| `exo-proofs` | 1,916 | SNARK, STARK, ZKML proof systems, verifier infrastructure |
| `exo-authority` | 1,438 | Authority delegation, role-based access, attestation chains |
| `exo-legal` | 1,032 | Legal compliance, audit admissibility, provenance tracking |
| `exo-consent` | 899 | Bailment contracts, consent policies, gatekeeper enforcement |
| `exo-escalation` | 824 | Escalation workflows, dispute resolution, threshold triggers |
| `exo-tenant` | 482 | Multi-tenancy isolation and tenant lifecycle management |
| `exo-api` | 283 | Public API surface and type exports |

### Applications

| Crate | LOC | Description |
|-------|-----|-------------|
| `decision-forum` | 3,800 | Full governance application: 15 modules, 131 tests, council-driven decision making |

### Governance & Infrastructure

* **`governance/`** — Council resolutions, sub-agent charters, traceability matrices, quality gates
* **`docs/`** — Architecture, guides, council panel reports, proofs, reference documentation
* **`.github/workflows/ci.yml`** — CI pipeline with 8 quality gates

## Governance & Compliance

This repository is managed under strict **Judicial Build Governance**. All contributions must align with `EXOCHAIN_Specification_v2.2.pdf`. CR-001 (AEGIS/SYBIL/Authentic Plurality) is **RATIFIED** and fully implemented.

### Key Governance Artifacts

* [Traceability Matrix](governance/traceability_matrix.md) — 75/75 requirements implemented
* [Threat Model](governance/threat_matrix.md) — 13/13 threats mitigated
* [Quality Gates](governance/quality_gates.md) — 8 CI-enforced gates
* [Sub-Agent Charters](governance/sub_agents.md) — 11 agents, all missions complete
* [Council Resolutions](governance/resolutions/INDEX.md) — CR-001 RATIFIED
* [Refactor Plan](governance/EXOCHAIN-REFACTOR-PLAN.md) — All 3 phases complete

### Documentation

* [Architecture](docs/architecture/ARCHITECTURE.md)
* [Threat Model](docs/architecture/THREAT-MODEL.md)
* [Getting Started Guide](docs/guides/GETTING-STARTED.md)
* [Crate Reference](docs/reference/CRATE-REFERENCE.md)
* [Constitutional Proofs](docs/proofs/CONSTITUTIONAL-PROOFS.md)
* [Council Panel Reports](docs/council/) — 5-panel assessment
* [Optimized Spec](docs/council/OPTIMIZED-SPEC.md)
* [Decision Forum Documentation](docs/decision-forum/SYSTEM-DOCUMENTATION.md)
* [Decision Forum User Manual](docs/decision-forum/USER-MANUAL.md)
* [ASI Report](docs/ASI-REPORT-FEATURE.md)

## Syntaxis Builder & Self-Development Kernel

EXOCHAIN operates through the **Syntaxis Builder** pipeline: a council-driven process where a 5-panel council (Governance, Legal, Architecture, Security, Operations) assesses all changes before implementation. This enables EXOCHAIN to function as a **self-developing system** — a system that develops systems, including itself.

The **decision-forum** application (15 modules, 131 tests) provides the runtime governance layer for council-driven decision making.

## Getting Started

### Prerequisites

* **Rust 1.85+** (`rustup update stable`)
* Clang (for crypto extensions)

### Build & Test

```bash
# Build all crates
cargo build --workspace --all-targets

# Run all library tests
cargo test --workspace --lib

# Run full test suite (includes doc tests and integration tests)
cargo test --workspace --all-features

# Lint (strict — no warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings

# Dependency audit
cargo deny check

# Format check
cargo fmt --all -- --check
```

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full judicial build workflow.

1. **Safety**: No PII on ledger. No admins. No floating-point arithmetic.
2. **Quality**: 90% coverage required. No Clippy warnings. All quality gates must pass.
3. **Process**: All PRs must map to a Spec requirement. See [AGENTS.md](AGENTS.md) for AI-assisted development.

## Post-Quantum Cryptography

EXOCHAIN uses a `Signature` enum supporting three modes:

* **Ed25519** — Current default for signing and verification
* **PostQuantum** — Forward-looking post-quantum signature scheme
* **Hybrid** — Combined Ed25519 + post-quantum for transition period

Float arithmetic is denied workspace-wide via `#[deny(clippy::float_arithmetic)]`.

## License

Apache-2.0 — See [LICENSE](LICENSE).

---
*EXOCHAIN Foundation — Judicial Build Governance*
