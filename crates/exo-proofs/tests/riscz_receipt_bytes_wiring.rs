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

//! Objective O-1.2 increment (A), lane VCG-001c2: thread `receipt_bytes`
//! through the public [`exo_proofs::envelope::ProofEnvelope::verify`] entry
//! point down to the RISC Zero verifier seam, so the future audited verifier
//! has the actual serialized receipt bytes to decode and check. Before this
//! increment, nothing passed receipt bytes at all — only the image id and the
//! journal-binding digest reached the seam.
//!
//! This file only asserts the PUBLIC path. Because `verify()` hardcodes
//! [`exo_proofs::envelope::FailClosedRiscZeroVerifier`] and exposes no
//! injection point, the seam-threading proof (that the exact bytes reach the
//! verifier) lives as in-module unit tests next to the private `verify_riscz`
//! method in `src/envelope.rs`. Here we only need to confirm the public path
//! still refuses, regardless of the receipt bytes it is handed — no
//! receipt-bytes value may turn the fail-closed RiscZero arm into `Ok`.

use exo_proofs::envelope::{BackendId, ProofEnvelope, ProofStatementKind};

fn riscz_envelope() -> ProofEnvelope {
    ProofEnvelope {
        statement_kind: ProofStatementKind::ExecutionReceipt,
        backend_id: BackendId::RiscZero,
        version: 1,
        public_inputs: vec![],
        commitment_roots: vec![],
        verifier_key_or_image_id: b"riscz-image-id-placeholder".to_vec(),
        domain_separator: b"exo-proofs:envelope:v1:execution-receipt".to_vec(),
    }
}

/// The public `verify()` path must refuse a RiscZero-backend envelope when
/// handed EMPTY receipt bytes. Threading `receipt_bytes` through the seam must
/// not open any new success path: an empty receipt is still refused
/// fail-closed, because no external cryptographic review of the risc0 verify
/// path has landed yet (ratified decision D1).
#[test]
fn riscz_verify_refuses_with_empty_receipt_bytes() {
    let envelope = riscz_envelope();

    let result = envelope.verify(&[]);
    assert!(
        result.is_err(),
        "BackendId::RiscZero verify(&[]) must fail closed regardless of receipt bytes, \
         got {result:?}"
    );
}

/// The public `verify()` path must ALSO refuse a RiscZero-backend envelope
/// when handed plausible-looking, non-empty receipt bytes. The point of this
/// increment is only to *thread the bytes to the seam*, not to add a verifier:
/// the fail-closed default ignores them and still refuses.
#[test]
fn riscz_verify_refuses_with_plausible_receipt_bytes() {
    let envelope = riscz_envelope();

    let result = envelope.verify(b"plausible-receipt");
    assert!(
        result.is_err(),
        "BackendId::RiscZero verify(b\"plausible-receipt\") must fail closed regardless of \
         receipt bytes, got {result:?}"
    );
}
