# Sub-Agent Charters

## A) SPEC_GUARDIAN (Legislative)
*   **Mission**: Ensure implementation strictly follows EXOCHAIN v2.2 Specification and maintain traceability.
*   **Inputs**: `EXOCHAIN_Specification_v2.2.pdf`, `EXOCHAIN-FABRIC-PLATFORM.md`.
*   **Outputs**: `traceability_matrix.md`, ADR reviews, "Non-negotiable invariants checklist".
*   **Definition of Done**: Traceability matrix maps every Spec section to code/tests; no unmapped requirements.

## B) ARCHITECTURE_AGENT (Legislative)
*   **Mission**: efficient, modular crate design aligned with Spec phases.
*   **Inputs**: Spec Phases, Rust Ecosystem Best Practices.
*   **Outputs**: Repo layout, `Cargo.toml` workspace, crate boundaries (`exo-core`, `exo-dag`, etc.).
*   **Definition of Done**: Use of `workspace` pattern, circular dependency check pass, clear API boundaries.

## C) CRYPTO_CANONICAL_AGENT (Judicial)
*   **Mission**: Implement deterministic cryptographic substrate (CBOR, BLAKE3, Ed25519) with cross-platform compatibility.
*   **Inputs**: Spec Section 9.1 (Event Hashing), Section 9.5 (RiskAttestation).
*   **Outputs**: `exo-core` crate, `cross-impl-test` harness, test vectors.
*   **Definition of Done**: Cross-implementation hash tests pass between Rust and JS Reference.

## D) CONSENSUS_DAG_AGENT (Executive)
*   **Mission**: Implement DAG storage, appending, and Hybrid Logical Clocks.
*   **Inputs**: Spec Section 9.2 (HLC), Section 9.4 (Checkpoints).
*   **Outputs**: `exo-dag` crate, `DAGStore` trait, `append()` logic.
*   **Definition of Done**: `verify_integrity()` passes on complex DAG topologies (proptest).

## E) PROOFS_INDEXER_AGENT (Executive/Judicial)
*   **Mission**: Implement verifiable query structures (MMR, SMT) and proofs.
*   **Inputs**: Spec Section 9.4 (Split Roots).
*   **Outputs**: MMR/SMT implementations, `EventInclusionProof` struct.
*   **Definition of Done**: Proof generation and verification roundtrip tests pass.

## F) IDENTITY_CONSENT_AGENT (Executive)
*   **Mission**: Logic for Identity, Consent, and Bailment fabrics.
*   **Inputs**: Spec Section 10 (Identity), Section 11 (Functional Reqs).
*   **Outputs**: `exo-identity`, `exo-consent`, `DidDocument`, `Policy` structs.
*   **Definition of Done**: Functional tests for Lifecycle (Create -> Rotate -> Revoke).

## G) GATEKEEPER_TEE_AGENT (Executive/Judicial)
*   **Mission**: Enforce TRUSTED boundaries for vault access.
*   **Inputs**: Spec Section 12 (Gatekeeper Trust).
*   **Outputs**: `exo-gatekeeper` interfaces, Mock TEE for dev.
*   **Definition of Done**: Policy enforcement tests (Consent -> AccessLogged), TEE Attestation mock flow.

## H) SECURITY_THREATS_AGENT (Judicial)
*   **Mission**: Maintain Threat Model and ensure coverage.
*   **Inputs**: Spec Section 13 (Threat Model).
*   **Outputs**: `threat_matrix.md`, fuzzing targets, `cargo-audit` config.
*   **Definition of Done**: Every threat in Section 13 has > 1 corresponding test case.

## I) QA_TDD_AGENT (Judicial)
*   **Mission**: Enforce Testing Pyramid and Acceptance Criteria.
*   **Inputs**: Spec Section 16 (Acceptance Criteria).
*   **Outputs**: Test harnesses, Integration tests, Fuzz targets.
*   **Definition of Done**: Section 16 Acceptance Criteria are automated and passing.

## J) DEVOPS_RELEASE_AGENT (Judicial)
*   **Mission**: CI/CD Pipelines and Release Quality Gates.
*   **Inputs**: Quality Gate Policies.
*   **Outputs**: `.github/workflows/ci.yml`, Release scripts.
*   **Definition of Done**: CI pipeline enforces coverage, formatting, and audit checks.

## K) DOCS_OSS_GOVERNANCE_AGENT (Legislative)
*   **Mission**: Open Source Community Governance and Documentation.
*   **Inputs**: OSS Best Practices, Spec.
*   **Outputs**: `README.md`, `LICENSE`, `CONTRIBUTING.md`, `GOVERNANCE.md`.
*   **Definition of Done**: Documentation is complete, accessible, and inclusive.
