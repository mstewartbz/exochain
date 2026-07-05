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

#![allow(clippy::expect_used)]

//! RED-stage tests for the VCG-001a proof envelope + statement registry.
//!
//! Lane VCG-001a (see `GAP-REGISTRY.md` "VCG-001 - Production ZK Proof Backend
//! Absent", "Next red test" bullet: "Add proof-envelope tests binding
//! statement kind, backend id, version, public inputs, commitment roots,
//! verifier key or image id, and domain separator.").
//!
//! These tests target `exo_proofs::envelope`, which does not exist yet in
//! `src/`. **Expected red mode: COMPILE ERROR** — `error[E0433]: failed to
//! resolve: could not find \`envelope\` in the crate root` (or equivalent
//! "unresolved import" / "no \`envelope\` in the root"). This is the
//! documented red for RED stage VCG-001a: no production code has been
//! written, so the module cannot be named.
//!
//! Do NOT add `#[cfg(feature = "unaudited-pedagogical-proofs")]` gating here
//! for the module-existence problem — that would hide the compile red. The
//! envelope registry (statement kinds, `ProofEnvelope`, canonical CBOR
//! (de)serialization, and backend-id fail-closed behavior) is scoped to be
//! usable independent of the pedagogical-backend feature flag, mirroring how
//! `verifier::ProofType` / `verifier::SnarkBundle` etc. are always
//! constructible in `src/verifier.rs` even though the proving/verifying
//! entry points themselves refuse without the feature.

use exo_proofs::envelope::{
    BackendId, ProofEnvelope, ProofStatementKind, UNAUDITED_BLAKE3_STANDIN_BACKEND_ID,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sample_envelope() -> ProofEnvelope {
    ProofEnvelope {
        statement_kind: ProofStatementKind::GovernanceCompliance,
        backend_id: UNAUDITED_BLAKE3_STANDIN_BACKEND_ID,
        version: 1,
        public_inputs: vec![b"public-input-a".to_vec(), b"public-input-b".to_vec()],
        commitment_roots: vec![exo_core::types::Hash256::digest(b"commitment-root-1")],
        verifier_key_or_image_id: b"verifier-key-or-image-id-bytes".to_vec(),
        domain_separator: b"exo-proofs:envelope:v1:governance-compliance".to_vec(),
    }
}

fn cbor_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut encoded = Vec::new();
    ciborium::into_writer(value, &mut encoded).expect("canonical CBOR encode");
    encoded
}

// ---------------------------------------------------------------------------
// (a) CBOR round-trip of the envelope fields
// ---------------------------------------------------------------------------

#[test]
fn proof_envelope_round_trips_through_canonical_cbor() {
    let envelope = sample_envelope();

    let encoded = cbor_bytes(&envelope);
    let decoded: ProofEnvelope =
        ciborium::from_reader(encoded.as_slice()).expect("canonical CBOR decode");

    assert_eq!(decoded.statement_kind, envelope.statement_kind);
    assert_eq!(decoded.backend_id, envelope.backend_id);
    assert_eq!(decoded.version, envelope.version);
    assert_eq!(decoded.public_inputs, envelope.public_inputs);
    assert_eq!(decoded.commitment_roots, envelope.commitment_roots);
    assert_eq!(
        decoded.verifier_key_or_image_id,
        envelope.verifier_key_or_image_id
    );
    assert_eq!(decoded.domain_separator, envelope.domain_separator);
}

#[test]
fn proof_envelope_round_trip_covers_every_statement_kind() {
    // GAP-REGISTRY.md VCG-001 remediation track: "Define a versioned proof
    // statement registry covering governance compliance, DAG inclusion,
    // execution receipt, model inference, and compatibility-only pedagogical
    // proofs." The frozen work order additionally names these as the exact
    // five statement kinds in scope.
    let kinds = [
        ProofStatementKind::GovernanceCompliance,
        ProofStatementKind::DagInclusion,
        ProofStatementKind::ExecutionReceipt,
        ProofStatementKind::ModelInference,
        ProofStatementKind::PedagogicalCompatibility,
    ];

    for kind in kinds {
        let mut envelope = sample_envelope();
        envelope.statement_kind = kind;

        let encoded = cbor_bytes(&envelope);
        let decoded: ProofEnvelope =
            ciborium::from_reader(encoded.as_slice()).expect("canonical CBOR decode");

        assert_eq!(
            decoded.statement_kind, kind,
            "statement kind {kind:?} must round-trip through canonical CBOR"
        );
    }
}

#[test]
fn proof_envelope_rejects_json_bytes() {
    // Mirrors verifier.rs's `verify_any_uses_canonical_cbor_not_json` /
    // `verify_any_rejects_json_*_bundle` convention: JSON is not the wire
    // format for envelope bytes, canonical CBOR is.
    let envelope = sample_envelope();
    let json_bytes = serde_json::to_vec(&envelope).expect("json encode for negative fixture");

    let result: Result<ProofEnvelope, _> = ciborium::from_reader(json_bytes.as_slice());
    assert!(
        result.is_err(),
        "JSON-encoded envelope bytes must not decode as canonical CBOR"
    );
}

// ---------------------------------------------------------------------------
// (b) unknown-backend-id fail-closed
// ---------------------------------------------------------------------------

/// Direct-construction path: an envelope built in-process (no
/// serialization involved) naming an unregistered backend id must fail
/// closed at `validate_backend()`. This is distinct from
/// `envelope_backend_registry_rejects_unregistered_numeric_id` below, which
/// exercises the *post-deserialization* path (a backend id that only
/// becomes "unknown" after a CBOR round-trip). The two paths are kept
/// separate because they exercise different code: this test never touches
/// `ciborium`, so it isolates `validate_backend()`'s own logic from the
/// (de)serialization layer.
#[test]
fn envelope_with_unknown_backend_id_fails_closed() {
    let mut envelope = sample_envelope();
    // BackendId::Unknown(_) (or equivalent future/unregistered id variant)
    // must never validate — future/unrecognized backend ids fail closed
    // rather than being silently accepted.
    envelope.backend_id = BackendId::Unknown(0xFFFF_FFFF);

    let result = envelope.validate_backend();
    assert!(
        result.is_err(),
        "an envelope naming an unknown/future backend id must fail closed, not validate"
    );
}

/// Post-deserialization path: a raw numeric backend id with no registry
/// entry must still fail closed *after* a full CBOR encode/decode
/// round-trip, not just when constructed directly in-process. This
/// complements `envelope_with_unknown_backend_id_fails_closed` above (the
/// direct-construction case) by proving the wire format itself cannot be
/// used to smuggle an unregistered backend id past validation — e.g. a
/// hand-crafted or future-version envelope on the wire that names a
/// backend id this build doesn't recognize.
#[test]
fn envelope_backend_registry_rejects_unregistered_numeric_id() {
    // Even if a caller round-trips an envelope through CBOR with a raw
    // numeric backend id that has no registry entry, validation must refuse
    // rather than silently treat it as any known backend.
    let mut envelope = sample_envelope();
    envelope.backend_id = BackendId::Unknown(1234);

    let encoded = cbor_bytes(&envelope);
    let decoded: ProofEnvelope =
        ciborium::from_reader(encoded.as_slice()).expect("canonical CBOR decode");

    assert!(
        decoded.validate_backend().is_err(),
        "unregistered numeric backend ids must fail closed after a CBOR round-trip"
    );
}

// ---------------------------------------------------------------------------
// (c) unaudited-backend-refused-unless-feature-enabled
//     (mirrors tests/refusal.rs pattern)
// ---------------------------------------------------------------------------

#[cfg(not(feature = "unaudited-pedagogical-proofs"))]
#[test]
fn envelope_wrapping_unaudited_backend_refuses_without_feature() {
    let envelope = sample_envelope(); // backend_id == UNAUDITED_BLAKE3_STANDIN_BACKEND_ID

    let result = envelope.verify(&[]);
    assert!(
        matches!(
            result,
            Err(exo_proofs::error::ProofError::UnauditedImplementation { .. })
        ),
        "an envelope wrapping the still-unaudited blake3 stand-in backend must refuse \
         verification unless 'unaudited-pedagogical-proofs' is enabled, got {result:?}"
    );
}

/// With the opt-in feature enabled, *constructing/wrapping* the unaudited
/// pedagogical backend in an envelope is allowed — but `verify()` still
/// fails closed, because no verifier is wired for any backend at this
/// stage (see the `ProofEnvelope::verify` doc comment). This test must NOT
/// claim execution/verification succeeds: it asserts on the specific
/// no-verifier-wired error, not on `Ok`.
#[cfg(feature = "unaudited-pedagogical-proofs")]
#[test]
fn envelope_wrapping_unaudited_backend_construction_allowed_but_verify_fails_closed() {
    let envelope = sample_envelope(); // backend_id == UNAUDITED_BLAKE3_STANDIN_BACKEND_ID

    // With the opt-in feature enabled, the unaudited pedagogical backend no
    // longer hard-refuses at the `guard_unaudited` gate — but verification
    // itself is not implemented in this lane (VCG-001a). `verify()` must
    // still fail closed with a typed "no verifier wired" error, never
    // `Ok(true)`.
    let result = envelope.verify(&[]);
    assert!(
        !matches!(
            result,
            Err(exo_proofs::error::ProofError::UnauditedImplementation { .. })
        ),
        "with 'unaudited-pedagogical-proofs' enabled, the unaudited backend must not \
         hard-refuse at the guard_unaudited gate with UnauditedImplementation, got {result:?}"
    );
    assert!(
        matches!(
            result,
            Err(exo_proofs::error::ProofError::VerificationFailed(_))
        ),
        "verify() must still fail closed with VerificationFailed (no verifier wired yet, \
         arrives with VCG-001b) rather than reporting success, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// (d) real negative fixtures for the envelope wire format
// ---------------------------------------------------------------------------

/// Truncating a valid canonical-CBOR encoding at any prefix length shorter
/// than the full encoding must fail to deserialize — never silently decode
/// a partial/corrupted envelope as if it were valid.
#[test]
fn truncated_cbor_bytes_fail_to_deserialize() {
    let envelope = sample_envelope();
    let full = cbor_bytes(&envelope);
    assert!(
        full.len() > 16,
        "sanity: sample envelope encoding should be long enough to exercise several cut points"
    );

    // Cut at several byte lengths: very early (mid-header), mid-way through
    // the map, and one byte short of the full encoding.
    let cut_points = [1usize, 4, full.len() / 4, full.len() / 2, full.len() - 1];

    for cut in cut_points {
        let truncated = &full[..cut];
        let result: Result<ProofEnvelope, _> = ciborium::from_reader(truncated);
        assert!(
            result.is_err(),
            "truncated CBOR bytes (cut at {cut} of {} total) must fail to deserialize, got Ok",
            full.len()
        );
    }
}

/// Constructs canonical CBOR bytes naming a `statement_kind` value that is
/// not one of the five registered [`ProofStatementKind`] variants (by
/// splicing an out-of-registry variant-name string into an otherwise valid
/// encoding, since this crate's enums serialize as their variant name, not
/// a raw numeric discriminant — see the module's canonical-CBOR
/// convention). Deserialization must fail closed rather than silently
/// accept, coerce, or default an unrecognized statement kind.
#[test]
fn unknown_statement_kind_code_fails_closed() {
    let envelope = sample_envelope();
    let valid = cbor_bytes(&envelope);

    // `statement_kind`'s value in the sample envelope encodes as the
    // 20-byte CBOR text string `"GovernanceCompliance"` (0x74 prefix +
    // 20 length bytes). Replace it with a same-shape but unregistered
    // variant name so the map structure otherwise stays byte-for-byte
    // valid CBOR — only the statement-kind discriminant itself is bogus.
    let needle = {
        let mut bytes = Vec::new();
        bytes.push(0x74u8); // CBOR text string, length 20
        bytes.extend_from_slice(b"GovernanceCompliance");
        bytes
    };
    let replacement = {
        let mut bytes = Vec::new();
        bytes.push(0x74u8); // CBOR text string, length 20 (same length, valid CBOR)
        bytes.extend_from_slice(b"NotARealStatementKnd"); // 20 bytes, unregistered name
        bytes
    };
    assert_eq!(
        needle.len(),
        replacement.len(),
        "sanity: splice must preserve overall byte layout so only the discriminant changes"
    );

    let position = valid
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .expect("sample envelope encoding must contain the statement_kind value bytes");

    let mut corrupted = valid.clone();
    corrupted[position..position + replacement.len()].copy_from_slice(&replacement);

    let result: Result<ProofEnvelope, _> = ciborium::from_reader(corrupted.as_slice());
    assert!(
        result.is_err(),
        "an out-of-registry statement-kind discriminant must fail closed at deserialization, \
         got Ok({result:?})"
    );
}

/// Bytes that are not CBOR at all (not merely a truncated/corrupted valid
/// encoding) must fail to deserialize as a [`ProofEnvelope`].
#[test]
fn garbage_bytes_fail_to_deserialize() {
    let garbage_fixtures: [&[u8]; 4] = [
        &[],
        &[0xFF, 0xFF, 0xFF, 0xFF],
        b"not cbor at all, just ascii text padding to be non-trivially long",
        &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09],
    ];

    for garbage in garbage_fixtures {
        let result: Result<ProofEnvelope, _> = ciborium::from_reader(garbage);
        assert!(
            result.is_err(),
            "garbage bytes {garbage:?} must fail to deserialize as ProofEnvelope, got Ok"
        );
    }
}
