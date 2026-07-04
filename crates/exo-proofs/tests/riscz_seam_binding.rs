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

#![allow(clippy::expect_used, clippy::unwrap_used)]

//! RED-stage tests for lane VCG-001c / objective O-1.1 (2026-07-04 ratification
//! slate).
//!
//! The RISC Zero verifier seam must BIND the envelope's statement context —
//! `statement_kind`, `commitment_roots`, `domain_separator` (and the rest of
//! the envelope) — into the value a receipt's journal is checked against, so a
//! receipt proven for one (statement, roots, domain) context can never be
//! replayed as valid under another. Before O-1.1 the seam
//! (`RiscZeroReceiptVerifier::verify_receipt`) received only the image id and
//! the raw public inputs; `domain_separator`, `commitment_roots`, and
//! `statement_kind` were dropped, so even a perfectly sound receipt verifier
//! plugged in there would certify an UNBOUND statement.
//!
//! `ProofEnvelope::binding_digest()` is the canonical journal-binding digest
//! the audited verifier checks the receipt journal against. These tests assert
//! it actually varies with each of the three previously-dropped fields (plus
//! `public_inputs`) and is otherwise deterministic.
//!
//! Expected red mode today: `binding_digest` does not exist yet, so this file
//! is a COMPILE ERROR (`no method named binding_digest`), mirroring the
//! documented-compile-red convention of `tests/envelope.rs` (VCG-001a) and
//! `tests/riscz_verifier_scaffold.rs` (VCG-001b).

use exo_core::types::Hash256;
use exo_proofs::envelope::{BackendId, ProofEnvelope, ProofStatementKind};

fn base_riscz_envelope() -> ProofEnvelope {
    ProofEnvelope {
        statement_kind: ProofStatementKind::ExecutionReceipt,
        backend_id: BackendId::RiscZero,
        version: 1,
        public_inputs: vec![b"public-input-0".to_vec()],
        commitment_roots: vec![Hash256([7u8; 32])],
        verifier_key_or_image_id: b"riscz-image-id".to_vec(),
        domain_separator: b"exo-proofs:envelope:v1:execution-receipt".to_vec(),
    }
}

#[test]
fn binding_digest_is_deterministic() {
    let a = base_riscz_envelope();
    let b = base_riscz_envelope();
    assert_eq!(
        a.binding_digest().expect("digest a"),
        b.binding_digest().expect("digest b"),
        "identical envelopes must produce identical binding digests"
    );
}

#[test]
fn binding_digest_binds_domain_separator() {
    let a = base_riscz_envelope();
    let mut b = base_riscz_envelope();
    b.domain_separator = b"exo-proofs:envelope:v1:a-different-domain".to_vec();
    assert_ne!(
        a.binding_digest().expect("digest a"),
        b.binding_digest().expect("digest b"),
        "binding digest MUST change when domain_separator changes — otherwise a proof \
         for one domain can be replayed as valid for another (the O-1.1 gap)"
    );
}

#[test]
fn binding_digest_binds_commitment_roots() {
    let a = base_riscz_envelope();
    let mut b = base_riscz_envelope();
    b.commitment_roots = vec![Hash256([9u8; 32])];
    assert_ne!(
        a.binding_digest().expect("digest a"),
        b.binding_digest().expect("digest b"),
        "binding digest MUST change when commitment_roots change — otherwise a proof \
         anchored to one root can be replayed against another"
    );
}

#[test]
fn binding_digest_binds_statement_kind() {
    let a = base_riscz_envelope();
    let mut b = base_riscz_envelope();
    b.statement_kind = ProofStatementKind::GovernanceCompliance;
    assert_ne!(
        a.binding_digest().expect("digest a"),
        b.binding_digest().expect("digest b"),
        "binding digest MUST change when statement_kind changes — otherwise a receipt \
         proving one kind of statement can be presented as proving another"
    );
}

#[test]
fn binding_digest_binds_public_inputs() {
    let a = base_riscz_envelope();
    let mut b = base_riscz_envelope();
    b.public_inputs = vec![b"a-different-public-input".to_vec()];
    assert_ne!(
        a.binding_digest().expect("digest a"),
        b.binding_digest().expect("digest b"),
        "binding digest MUST change when public_inputs change"
    );
}

#[test]
fn binding_digest_binds_image_id() {
    let a = base_riscz_envelope();
    let mut b = base_riscz_envelope();
    b.verifier_key_or_image_id = b"a-different-image-id".to_vec();
    assert_ne!(
        a.binding_digest().expect("digest a"),
        b.binding_digest().expect("digest b"),
        "binding digest MUST change when the verifier key / image id changes"
    );
}
