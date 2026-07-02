// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Integration tests verifying that exo-proofs *refuses* to execute when
//! the `unaudited-pedagogical-proofs` opt-in feature is OFF.
//!
//! These tests run in the default build (feature OFF). They assert that every
//! public proof entry point returns `Err(ProofError::UnauditedImplementation)`,
//! preventing accidental reliance on the unaudited skeleton in production.

#![cfg(not(feature = "unaudited-pedagogical-proofs"))]

use exo_proofs::error::ProofError;

#[test]
fn guard_unaudited_refuses_by_default() {
    // Direct guard check — this is the canonical refusal signal.
    let result = exo_proofs::guard_unaudited("test");
    assert!(matches!(
        result,
        Err(ProofError::UnauditedImplementation { .. })
    ));
}

#[test]
fn snark_verify_refuses_by_default() {
    use exo_proofs::snark::{Proof, VerifyingKey};
    // We can still construct types — the refusal is at the verify entry point.
    let vk = VerifyingKey {
        circuit_hash: exo_core::types::Hash256([0u8; 32]),
        num_public_inputs: 0,
    };
    let proof = Proof {
        a: [0u8; 32],
        b: [0u8; 32],
        c: [0u8; 32],
    };
    let result = exo_proofs::snark::verify(&vk, &proof, &[]);
    assert!(matches!(
        result,
        Err(ProofError::UnauditedImplementation { .. })
    ));
}

#[test]
fn zkml_daubert_admissibility_refuses_by_default() {
    use exo_core::types::Hash256;
    use exo_proofs::zkml::{DaubertAdmissibility, InferenceProof, ModelCommitment};

    let proof = InferenceProof {
        model_commitment: ModelCommitment::new(b"architecture", b"weights", 1),
        input_hash: Hash256::digest(b"context"),
        output_hash: Hash256::digest(b"output"),
        proof: Hash256::ZERO,
        verification_tag: Hash256::ZERO,
        prompt_hash: None,
        human_attestation: None,
        ai_delta: None,
        daubert_checklist: None,
    };

    let status = proof.daubert_admissibility_status();

    assert!(
        matches!(
            status,
            DaubertAdmissibility::Inadmissible { ref reason }
                if reason.contains("unaudited-pedagogical-proofs")
        ),
        "Daubert status must fail closed when unaudited proof APIs are disabled, got {status:?}"
    );
}

/// VCG-001a RED stage — see `GAP-REGISTRY.md` "VCG-001 - Production ZK Proof
/// Backend Absent", ratified decision D1 (RISC Zero is the selected
/// production backend family). This lane (VCG-001a) only introduces the
/// proof statement registry and envelope; it does NOT vendor a production
/// backend (that is lane VCG-001b, D1 risc0 vendoring — explicitly SCOPE OUT
/// here).
///
/// Standing red: no production `BackendId` variant exists yet in
/// `exo_proofs::envelope` (the module itself doesn't exist at RED stage for
/// VCG-001a — see `tests/envelope.rs`). Because a production backend variant
/// cannot yet be named, the test body is gated behind
/// `#[cfg(feature = "vcg-001b-production-backend")]` — a feature that is
/// deliberately NOT declared in `Cargo.toml` yet, so the gate always
/// evaluates false and the test body is compiled out. This keeps the crate
/// compiling (per the coordinator's requirement that the suite must build)
/// while the `#[ignore]` below documents the standing red: this test cannot
/// pass, and is not meant to, until VCG-001b lands both the `envelope`
/// module (VCG-001a) and a real production backend variant (VCG-001b).
///
/// Once VCG-001a lands `exo_proofs::envelope::BackendId` and VCG-001b adds a
/// production variant (e.g. `BackendId::RiscZero { .. }`), this test must be
/// rewritten to: (1) drop the `cfg` gate, (2) drop `#[ignore]`, (3) construct
/// a `ProofEnvelope` naming the production backend variant, and (4) assert
/// `envelope.verify()` succeeds WITHOUT the `unaudited-pedagogical-proofs`
/// feature enabled — proving production backends are exempt from the
/// pedagogical refusal gate because they carry their own audit evidence.
#[ignore = "red until VCG-001b lands a production backend"]
#[test]
// `vcg-001b-production-backend` is intentionally not declared in Cargo.toml
// (see the doc comment above) — it fences code that must not compile until
// VCG-001b lands. Mirrors the same `#[allow(unexpected_cfgs)]`-on-intentional-
// future-cfg convention already used in `crates/exo-gateway/src/dagdb.rs`.
#[allow(unexpected_cfgs)]
fn production_backend_variant_executes_without_unaudited_flag() {
    #[cfg(feature = "vcg-001b-production-backend")]
    {
        // This branch intentionally does not compile yet: `envelope` and its
        // production `BackendId` variant do not exist until VCG-001a and
        // VCG-001b land. It is fenced behind a feature that Cargo.toml never
        // declares, so it can never be selected, and the crate keeps
        // compiling in the meantime.
        use exo_proofs::envelope::{BackendId, ProofEnvelope, ProofStatementKind};

        let envelope = ProofEnvelope {
            statement_kind: ProofStatementKind::ExecutionReceipt,
            backend_id: BackendId::RiscZero {
                image_id: [0u8; 32],
            },
            version: 1,
            public_inputs: vec![],
            commitment_roots: vec![],
            verifier_key_or_image_id: vec![],
            domain_separator: b"exo-proofs:envelope:v1:execution-receipt".to_vec(),
        };

        // Production backends must verify WITHOUT the
        // unaudited-pedagogical-proofs feature enabled — this test binary is
        // compiled with that feature off (see the `#![cfg(not(feature =
        // "unaudited-pedagogical-proofs"))]` crate-level gate above).
        let result = envelope.verify();
        assert!(
            result.is_ok(),
            "a production backend variant (e.g. RISC Zero, ratified decision D1) must \
             verify without the unaudited-pedagogical-proofs feature enabled, got {result:?}"
        );
    }

    #[cfg(not(feature = "vcg-001b-production-backend"))]
    {
        panic!(
            "standing red (VCG-001a RED stage): no production BackendId variant exists yet. \
             This test is gated behind the non-existent 'vcg-001b-production-backend' feature \
             so the crate compiles; it must fail here until VCG-001b lands a real production \
             backend. See GAP-REGISTRY.md VCG-001 remediation track and ratified decision D1."
        );
    }
}
