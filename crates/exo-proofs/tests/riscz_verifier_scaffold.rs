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

//! RED-stage tests for lane VCG-001b: an honest, fail-closed RISC Zero
//! verifier-integration *scaffold*.
//!
//! See `GAP-REGISTRY.md` "VCG-001 - Production ZK Proof Backend Absent",
//! ratified decision D1 (2026-07-02): RISC Zero is the selected production
//! backend family, Groth16 wrapping for receipt compression, proving is
//! server-side only, and the verifier "stays small, in-workspace, and
//! pinned, and carries the external audit budget."
//!
//! ## What this lane is honest about
//!
//! Ratified D1 says the production verifier *carries the external audit
//! budget* — i.e. marking a backend [`exo_proofs::envelope::AuditStatus::ProductionReviewed`]
//! is the claim that it has passed external cryptographic review. That
//! review has **not** happened yet. This lane therefore:
//!
//! 1. Registers a genuine `BackendId::RiscZero` variant (not `Unknown`).
//! 2. Registers it in `default_registry()` under a **third**, new
//!    `AuditStatus` variant — `PendingExternalReview` — documented as
//!    "integration wired but NOT yet cryptographically reviewed; not a
//!    production trust claim." It is explicitly **not**
//!    `AuditStatus::ProductionReviewed`.
//! 3. Wires a verifier *seam* for `BackendId::RiscZero` in
//!    `ProofEnvelope::verify()` that still **fails closed** — mirroring the
//!    `UnauditedBlake3Standin` arm's fail-closed posture, but for a
//!    distinct, documented reason (pending external review, not missing
//!    feature opt-in).
//!
//! The standing red in `tests/refusal.rs`
//! (`production_backend_variant_executes_without_unaudited_flag`) MUST stay
//! red/`#[ignore]`d — nothing in this lane registers a
//! `AuditStatus::ProductionReviewed` backend, and nothing here un-ignores
//! or otherwise touches that test.
//!
//! ## Expected red mode
//!
//! Today (commit `3dc7672a`) `exo_proofs::envelope` has exactly two
//! `BackendId` variants (`UnauditedBlake3Standin`, `Unknown(u32)`) and two
//! `AuditStatus` variants (`Pedagogical`, `ProductionReviewed`). This file
//! names `BackendId::RiscZero` and `AuditStatus::PendingExternalReview`,
//! neither of which exist yet — **expected red mode is a COMPILE ERROR**
//! (`error[E0599]: no variant named \`RiscZero\` found for enum \`BackendId\``
//! or equivalent "no variant/associated item" errors), mirroring how
//! `tests/envelope.rs` documented an expected compile-error red for
//! VCG-001a. This is the documented red for RED stage VCG-001b: no
//! production code has been written yet, so the variants cannot be named.

use exo_proofs::envelope::{AuditStatus, BackendId, ProofEnvelope, ProofStatementKind};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// (a) default_registry() carries a RiscZero / PendingExternalReview entry
// ---------------------------------------------------------------------------

/// RED (a): `default_registry()` must contain a `BackendId::RiscZero`
/// descriptor whose `audit_status` is `AuditStatus::PendingExternalReview`.
///
/// Fails today: neither `BackendId::RiscZero` nor
/// `AuditStatus::PendingExternalReview` exist, so this does not compile.
/// Once both variants exist, it fails at runtime until `default_registry()`
/// actually registers the entry.
#[test]
fn default_registry_contains_riscz_pending_external_review_backend() {
    let registry = exo_proofs::envelope::default_registry();

    let riscz_entry = registry
        .iter()
        .find(|descriptor| matches!(descriptor.backend_id, BackendId::RiscZero))
        .unwrap_or_else(|| {
            panic!(
                "expected default_registry() to contain a BackendId::RiscZero descriptor \
                 (VCG-001b honest scaffold), got {registry:?}"
            )
        });

    assert_eq!(
        riscz_entry.audit_status,
        AuditStatus::PendingExternalReview,
        "the RiscZero descriptor must be AuditStatus::PendingExternalReview — integration \
         wired but NOT yet cryptographically reviewed; it must NOT be marked \
         AuditStatus::ProductionReviewed until external review actually happens, got {riscz_entry:?}"
    );
}

// ---------------------------------------------------------------------------
// (b) RiscZero verify() fails closed with a pending-review-specific error
// ---------------------------------------------------------------------------

/// RED (b): a `ProofEnvelope` naming `BackendId::RiscZero` must fail closed
/// at `verify()` with an error whose message mentions the backend is
/// pending external review / not yet audited. This must be true
/// regardless of the `unaudited-pedagogical-proofs` feature flag — the
/// RiscZero seam's refusal reason is "no external cryptographic review has
/// landed yet," which is orthogonal to the pedagogical-backend opt-in gate.
///
/// Fails today: `BackendId::RiscZero` does not exist, so this does not
/// compile. Once it exists, it fails at runtime until `ProofEnvelope::verify`
/// has a match arm for it.
#[test]
fn riscz_backend_verify_fails_closed_pending_external_review() {
    let envelope = riscz_envelope();

    let result = envelope.verify(&[]);
    assert!(
        result.is_err(),
        "BackendId::RiscZero must fail closed at verify() until external review lands, \
         got {result:?}"
    );

    let message = result.unwrap_err().to_string();
    let mentions_pending_review = message.to_lowercase().contains("pending")
        || message.to_lowercase().contains("external review")
        || message.to_lowercase().contains("not yet")
        || message.to_lowercase().contains("not audited")
        || message.to_lowercase().contains("unaudited");
    assert!(
        mentions_pending_review,
        "RiscZero verify() error must mention pending external review / not yet audited, \
         got message: {message:?}"
    );
}

/// RED (b), feature-flag independence: the RiscZero pending-review refusal
/// must NOT be gated by (or removable via) the
/// `unaudited-pedagogical-proofs` feature — that feature only concerns the
/// crate's own pedagogical blake3 stand-in, not the RiscZero integration
/// seam. Enabling it must not turn RiscZero's refusal into `Ok(true)`.
#[cfg(feature = "unaudited-pedagogical-proofs")]
#[test]
fn riscz_backend_verify_still_fails_closed_with_pedagogical_feature_enabled() {
    let envelope = riscz_envelope();

    let result = envelope.verify(&[]);
    assert!(
        result.is_err(),
        "enabling 'unaudited-pedagogical-proofs' must not make BackendId::RiscZero verify() \
         succeed — the RiscZero seam's refusal is about pending external review, not the \
         pedagogical opt-in gate, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// (c) anti-overclaim regression lock (must remain true before AND after green)
// ---------------------------------------------------------------------------

/// This is a REGRESSION LOCK, not a red test: it already passes today
/// (`default_registry()` has zero `ProductionReviewed` entries because it
/// has exactly one `Pedagogical` entry) and it MUST keep passing after this
/// lane goes green. It protects the honest posture required by ratified
/// decision D1: nothing in VCG-001b is permitted to claim a backend has
/// passed external cryptographic review it has not actually passed. If a
/// future change makes this test fail, that change is a false soundness
/// claim and must be reverted, not the test.
#[test]
fn anti_overclaim_default_registry_has_zero_production_reviewed_backends() {
    let registry = exo_proofs::envelope::default_registry();

    let production_reviewed_count = registry
        .iter()
        .filter(|descriptor| descriptor.audit_status == AuditStatus::ProductionReviewed)
        .count();

    assert_eq!(
        production_reviewed_count, 0,
        "default_registry() must contain ZERO AuditStatus::ProductionReviewed backends — \
         no backend has passed external cryptographic review yet (ratified decision D1). \
         Registering one here would be a false soundness claim. Got: {registry:?}"
    );
}

/// Companion regression lock: the standing red in `tests/refusal.rs`,
/// `production_backend_variant_executes_without_unaudited_flag`, asserts
/// that a `ProductionReviewed` backend's `verify()` succeeds without the
/// unaudited feature. Since (per the lock above) no such backend is ever
/// registered by this lane, that assertion's precondition
/// (`registry.iter().find(...ProductionReviewed...)`) can never be
/// satisfied by anything this lane adds — i.e. this lane cannot
/// accidentally turn that standing red green. This test documents that
/// invariant directly against the RiscZero entry specifically: it must be
/// `PendingExternalReview`, never `ProductionReviewed`.
#[test]
fn riscz_backend_is_never_marked_production_reviewed() {
    let registry = exo_proofs::envelope::default_registry();

    for descriptor in registry
        .iter()
        .filter(|descriptor| matches!(descriptor.backend_id, BackendId::RiscZero))
    {
        assert_ne!(
            descriptor.audit_status,
            AuditStatus::ProductionReviewed,
            "BackendId::RiscZero must never be registered as AuditStatus::ProductionReviewed \
             until external cryptographic review actually lands, got {descriptor:?}"
        );
    }
}
