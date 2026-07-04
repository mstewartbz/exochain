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
/// proof statement registry, envelope, and a minimal audit-status accessor
/// (`exo_proofs::envelope::{AuditStatus, default_registry}`); it does NOT
/// vendor a production backend or wire a real verifier (that is lane
/// VCG-001b, D1 risc0 vendoring — explicitly SCOPE OUT here).
///
/// Standing red: this test compiles and runs today (no feature gating, no
/// panic-body placeholder) against the *real* backend registry
/// (`exo_proofs::envelope::default_registry()`). It asserts two things that
/// are jointly required before this test can pass:
///
/// 1. The default registry contains at least one
///    [`exo_proofs::envelope::AuditStatus::ProductionReviewed`] backend
///    descriptor.
/// 2. That backend's [`exo_proofs::envelope::ProofEnvelope::verify`]
///    succeeds WITHOUT the `unaudited-pedagogical-proofs` feature enabled
///    (this test binary is compiled with that feature off — see the
///    `#![cfg(not(feature = "unaudited-pedagogical-proofs"))]` crate-level
///    gate above) — proving production backends are exempt from the
///    pedagogical refusal gate because they carry their own audit evidence
///    and a real wired verifier.
///
/// Today `default_registry()` returns two entries —
/// `BackendId::UnauditedBlake3Standin` marked `AuditStatus::Pedagogical` and
/// `BackendId::RiscZero` marked `AuditStatus::PendingExternalReview` — and
/// neither is `AuditStatus::ProductionReviewed`, so assertion (1) fails here
/// and this test is red. It is impossible to satisfy by declaring a feature
/// flag: there is no `cfg` gate left to exploit. The only way to turn this
/// green is to actually register a production-reviewed backend descriptor in
/// `default_registry()` (VCG-001c) whose `verify()` is backed by a real
/// wired, externally reviewed verifier.
#[ignore = "red until VCG-001b lands a production backend"]
#[test]
fn production_backend_variant_executes_without_unaudited_flag() {
    use exo_proofs::envelope::AuditStatus;

    let registry = exo_proofs::envelope::default_registry();

    let production_backend = registry
        .iter()
        .find(|descriptor| descriptor.audit_status == AuditStatus::ProductionReviewed)
        .unwrap_or_else(|| {
            panic!(
                "standing red (VCG-001a RED stage): default_registry() contains no \
                 AuditStatus::ProductionReviewed backend yet (got {registry:?}). This must \
                 fail here until VCG-001b actually registers a production-reviewed backend \
                 with a wired verifier. See GAP-REGISTRY.md VCG-001 remediation track and \
                 ratified decision D1."
            )
        });

    let envelope = exo_proofs::envelope::ProofEnvelope {
        statement_kind: exo_proofs::envelope::ProofStatementKind::ExecutionReceipt,
        backend_id: production_backend.backend_id,
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
        "a production-reviewed backend (e.g. RISC Zero, ratified decision D1) must \
         verify without the unaudited-pedagogical-proofs feature enabled, got {result:?}"
    );
}
