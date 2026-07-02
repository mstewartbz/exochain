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

//! Versioned proof statement registry and proof envelope.
//!
//! Lane VCG-001a (see `GAP-REGISTRY.md` "VCG-001 - Production ZK Proof Backend
//! Absent", remediation track: "Define a versioned proof statement registry
//! covering governance compliance, DAG inclusion, execution receipt, model
//! inference, and compatibility-only pedagogical proofs.").
//!
//! [`ProofEnvelope`] binds together everything a verifier needs to know
//! *about* a proof before it ever looks at proof bytes: which kind of
//! statement is being proven ([`ProofStatementKind`]), which backend
//! produced it ([`BackendId`]), an envelope format version, the public
//! inputs, commitment roots, the verifier key or image id, and a domain
//! separator binding the proof to its intended context.
//!
//! ## Fail-closed backend registry
//!
//! [`BackendId`] is a closed set of *known* backends plus an
//! [`BackendId::Unknown`] catch-all for any numeric id that does not match a
//! known variant. [`ProofEnvelope::validate_backend`] refuses
//! `BackendId::Unknown` unconditionally — an envelope naming an
//! unrecognized or future backend id can never validate. This is the same
//! "never stub, fail loudly" doctrine that governs the rest of this crate
//! (see the crate-root docs and [`crate::guard_unaudited`]).
//!
//! ## Unaudited backend gating
//!
//! The only backend currently registered is
//! [`UNAUDITED_BLAKE3_STANDIN_BACKEND_ID`] — the same blake3 "stand-in"
//! cryptography described in the crate-root docs. Wrapping that backend id
//! in an envelope does not exempt it from the crate's unaudited-refusal
//! doctrine: [`ProofEnvelope::verify`] refuses with
//! [`crate::error::ProofError::UnauditedImplementation`] unless the opt-in
//! `unaudited-pedagogical-proofs` Cargo feature is enabled, mirroring the
//! [`crate::guard_unaudited`] pattern used by `snark`, `stark`, and `zkml`.
//!
//! ## Wire format
//!
//! Per the crate's canonical-CBOR-not-JSON convention (see
//! `src/verifier.rs`), [`ProofEnvelope`] is (de)serialized with
//! [`ciborium`]'s canonical CBOR encoding, never JSON.

use exo_core::types::Hash256;
use serde::{Deserialize, Serialize};

use crate::error::{ProofError, Result};

// ---------------------------------------------------------------------------
// ProofStatementKind
// ---------------------------------------------------------------------------

/// The kind of statement a [`ProofEnvelope`] attests to.
///
/// This is the versioned proof statement registry named in the VCG-001
/// remediation track. Adding a new kind is additive (append a variant);
/// removing or renumbering an existing kind is a breaking wire-format
/// change and must not be done silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatementKind {
    /// The statement attests to compliance with a governance rule or
    /// constitutional constraint.
    GovernanceCompliance,
    /// The statement attests that a value is included in a DAG at a given
    /// position/commitment.
    DagInclusion,
    /// The statement attests to the authenticity of an execution receipt
    /// (e.g. a computation actually ran and produced a given result).
    ExecutionReceipt,
    /// The statement attests to properties of a model inference (e.g. that
    /// a committed model produced a committed output from a committed
    /// input).
    ModelInference,
    /// The statement attests only to structural/shape compatibility of a
    /// pedagogical proof — not a production cryptographic claim. Used by
    /// the unaudited blake3 stand-in backend.
    PedagogicalCompatibility,
}

// ---------------------------------------------------------------------------
// BackendId
// ---------------------------------------------------------------------------

/// The numeric id space reserved for the crate's own unaudited pedagogical
/// stand-in backend (`BackendId::UnauditedBlake3Standin` encodes to this
/// value under `#[repr(u32)]`-style discriminant semantics via
/// [`BackendId::registry_id`]).
///
/// Exposed as a `u32` constant (rather than only the enum variant) so that
/// callers and tests can construct envelopes that name this backend without
/// reaching into the enum's internal discriminant.
pub const UNAUDITED_BLAKE3_STANDIN_BACKEND_ID: BackendId = BackendId::UnauditedBlake3Standin;

/// Identifies which proof backend produced (and must verify) a
/// [`ProofEnvelope`].
///
/// This is a closed registry: [`BackendId::Unknown`] is the only variant
/// that accepts arbitrary numeric ids, and it is the *only* variant that
/// [`ProofEnvelope::validate_backend`] ever refuses. Every other variant is
/// a backend this crate knows about by construction. Registering a new
/// backend means adding a new named variant here — not widening what
/// `Unknown` accepts.
///
/// Ratified decision D1 (2026-07-02, see `GAP-REGISTRY.md` VCG-001) selects
/// RISC Zero as the production backend family. No production backend
/// variant is added in this lane (VCG-001a) — that is lane VCG-001b, scoped
/// out here. The only concrete backend registered today is the crate's
/// existing unaudited blake3 stand-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendId {
    /// The crate's existing unaudited blake3 "stand-in" cryptography
    /// (`circuit.rs` / `snark.rs` / `stark.rs` / `zkml.rs`). Gated behind
    /// the `unaudited-pedagogical-proofs` feature at verification time.
    UnauditedBlake3Standin,
    /// An unrecognized or future backend id. Always fails closed in
    /// [`ProofEnvelope::validate_backend`] — this crate refuses to treat an
    /// id it does not recognize as valid, regardless of the numeric value.
    Unknown(u32),
}

impl BackendId {
    /// Returns `true` for the crate's only currently-registered *known*
    /// backend variant (i.e. not [`BackendId::Unknown`]).
    #[must_use]
    pub const fn is_registered(&self) -> bool {
        !matches!(self, BackendId::Unknown(_))
    }
}

// ---------------------------------------------------------------------------
// AuditStatus / backend descriptor registry
// ---------------------------------------------------------------------------

/// The audit/review status carried by each entry in [`default_registry`].
///
/// This is a minimal accessor, not a verification mechanism: it exists so
/// tests (and future callers) can ask "does a production-reviewed backend
/// exist yet?" without hardcoding backend ids. Today every registered
/// backend is [`AuditStatus::Pedagogical`] — no [`AuditStatus::ProductionReviewed`]
/// entry exists until VCG-001b lands a real production backend with a wired
/// verifier (see `tests/refusal.rs`'s standing red).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditStatus {
    /// Structural/pedagogical stand-in only — not a production trust claim.
    /// Gated behind the `unaudited-pedagogical-proofs` feature at
    /// verification time (see [`crate::guard_unaudited`]).
    Pedagogical,
    /// A production backend that has undergone cryptographic review and
    /// carries its own audit evidence. Exempt from the pedagogical
    /// unaudited-refusal gate. No backend holds this status yet — it is
    /// introduced here only so the registry has somewhere to record one
    /// once VCG-001b lands.
    ProductionReviewed,
}

/// A single entry in [`default_registry`]: a known [`BackendId`] paired with
/// its [`AuditStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendDescriptor {
    /// The backend this descriptor describes.
    pub backend_id: BackendId,
    /// Whether this backend is pedagogical or has been production-reviewed.
    pub audit_status: AuditStatus,
}

/// Returns the crate's default backend registry as descriptors.
///
/// This is the minimal audit-status accessor named in the VCG-001a
/// hardening pass: it lets callers (and tests) inspect what backends are
/// registered and whether any of them are [`AuditStatus::ProductionReviewed`]
/// without reaching into [`ProofEnvelope::verify`] internals. Today this
/// returns exactly one entry — the unaudited blake3 stand-in, marked
/// [`AuditStatus::Pedagogical`] — because no production backend has been
/// registered yet (VCG-001b, ratified decision D1, out of scope for
/// VCG-001a).
#[must_use]
pub fn default_registry() -> Vec<BackendDescriptor> {
    vec![BackendDescriptor {
        backend_id: BackendId::UnauditedBlake3Standin,
        audit_status: AuditStatus::Pedagogical,
    }]
}

// ---------------------------------------------------------------------------
// ProofEnvelope
// ---------------------------------------------------------------------------

/// A versioned envelope binding everything a verifier needs to know about a
/// proof, independent of the proof bytes themselves.
///
/// Field order mirrors the VCG-001 "Next red test" bullet: "statement kind,
/// backend id, version, public inputs, commitment roots, verifier key or
/// image id, and domain separator."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofEnvelope {
    /// Which kind of statement this envelope attests to.
    pub statement_kind: ProofStatementKind,
    /// Which backend produced (and must verify) the wrapped proof.
    pub backend_id: BackendId,
    /// Envelope format version. Independent of the crate/package version;
    /// bump when the envelope's own field shape changes.
    pub version: u32,
    /// Public inputs to the statement, as opaque byte strings. Semantics
    /// are defined per [`ProofStatementKind`] / backend.
    pub public_inputs: Vec<Vec<u8>>,
    /// Commitment roots (e.g. DAG roots, state roots) the statement is
    /// anchored to.
    pub commitment_roots: Vec<Hash256>,
    /// The verifier key (SNARK/STARK) or image id (zkVM-style backends)
    /// needed to verify the wrapped proof, as opaque bytes.
    pub verifier_key_or_image_id: Vec<u8>,
    /// Domain separator binding this envelope to its intended context, so
    /// a valid proof for one domain cannot be replayed as valid for
    /// another.
    pub domain_separator: Vec<u8>,
}

impl ProofEnvelope {
    /// Validates that [`Self::backend_id`] names a known, registered
    /// backend.
    ///
    /// Fails closed: any [`BackendId::Unknown`] value — including ids that
    /// happen to coincide with a future backend not yet registered here —
    /// is refused. This must be called (directly, or transitively via
    /// [`Self::verify`]) before any proof bytes wrapped by this envelope
    /// are trusted.
    pub fn validate_backend(&self) -> Result<()> {
        if self.backend_id.is_registered() {
            Ok(())
        } else {
            Err(ProofError::InvalidProofFormat(format!(
                "proof envelope names unknown/unregistered backend id: {:?}",
                self.backend_id
            )))
        }
    }

    /// Verifies the envelope's named backend is both registered and, if
    /// unaudited, explicitly opted into.
    ///
    /// This lane (VCG-001a) does not implement per-backend proof
    /// verification logic: no production backend exists yet (VCG-001b,
    /// ratified decision D1, out of scope here), and this method does not
    /// wire up or bypass the crate's existing unaudited blake3 stand-in
    /// verification in `snark.rs` / `stark.rs` / `zkml.rs` /
    /// `verifier::verify_any`. Fail-closed: **every** backend currently
    /// registered returns a typed error here — there is no verifier wired
    /// for any backend at this stage. This is a deliberate success-shaped
    /// surface trap avoidance: `verify()` must never report `Ok(true)`
    /// unless it actually verified something.
    ///
    /// Behavior:
    /// - Unknown/unregistered backend id → `Err(ProofError::InvalidProofFormat)`
    ///   (fail-closed registry, checked first via [`Self::validate_backend`]).
    /// - [`UNAUDITED_BLAKE3_STANDIN_BACKEND_ID`] → first refuses with
    ///   `Err(ProofError::UnauditedImplementation)` unless the
    ///   `unaudited-pedagogical-proofs` feature is enabled (mirroring
    ///   [`crate::guard_unaudited`]); if that guard passes, still refuses
    ///   with `Err(ProofError::VerificationFailed)` because no verifier is
    ///   wired for this backend yet — construction/wrapping of an envelope
    ///   for this backend is feature-gated as above, but *verification* is
    ///   not implemented at all in VCG-001a. Real verification arrives with
    ///   VCG-001b.
    pub fn verify(&self) -> Result<bool> {
        self.validate_backend()?;

        match self.backend_id {
            BackendId::UnauditedBlake3Standin => {
                crate::guard_unaudited("envelope::ProofEnvelope::verify")?;
                // Even once opted into the unaudited pedagogical stand-in,
                // no verifier is wired for it yet at this stage: this lane
                // (VCG-001a) only establishes the envelope/registry shape.
                // Returning `Ok(true)` here would be a success-shaped
                // surface that verifies nothing. Fail closed instead.
                Err(ProofError::VerificationFailed(format!(
                    "no verifier is wired for backend {:?} yet; \
                     ProofEnvelope::verify is a fail-closed stub until \
                     VCG-001b lands real per-backend verification",
                    self.backend_id
                )))
            }
            BackendId::Unknown(_) => unreachable!(
                "validate_backend() above must have already refused an unregistered backend id"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod canonical_encoding_contract_tests {
    #[test]
    fn envelope_module_uses_canonical_cbor_not_json() {
        // Mirrors verifier.rs's `verify_any_uses_canonical_cbor_not_json`
        // source-grep guard: the envelope module itself must never reach
        // for JSON as a wire format for proof-adjacent data.
        let source = include_str!("envelope.rs");
        let production = source
            .split("// ---------------------------------------------------------------------------\n// Tests")
            .next()
            .expect("production section exists");

        assert!(
            !production.contains("serde_json"),
            "envelope module must not use serde_json anywhere in its production code path"
        );
    }

    #[test]
    fn backend_id_unknown_variant_is_the_only_unregistered_case() {
        use super::BackendId;

        assert!(BackendId::UnauditedBlake3Standin.is_registered());
        assert!(!BackendId::Unknown(0).is_registered());
        assert!(!BackendId::Unknown(u32::MAX).is_registered());
    }
}
