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

//! AVC bindings: subject-side action-signature production for
//! credential-bearing trust-receipt emission.
//!
//! An app holding a registered [`AutonomousVolitionCredential`] proves it
//! took an authorized action by signing the canonical action payload with
//! its subject key, then POSTing to the node's
//! `/api/v1/avc/receipts/emit`. The node re-verifies that signature, runs
//! [`exo_avc::validate_avc`], and — on `Allow` — signs and stores an
//! [`exo_avc::AvcTrustReceipt`].
//!
//! **Byte-parity is load-bearing.** The action signature the node accepts
//! is an Ed25519 signature over [`exo_avc::avc_action_signature_payload`]
//! (domain `exo.avc.action.v1`, canonical CBOR). This bridge **calls that
//! exact function** — it never reimplements the encoding — so the bytes a
//! JS caller signs are identical to the bytes the node verifies. The
//! `#[cfg(test)]` block below pins this against drift by signing through
//! this bridge and verifying through the node's own verification path
//! ([`exo_core::crypto::verify`] over the same payload).

use exo_avc::{
    AutonomousVolitionCredential, AvcActionRequest, AvcValidationRequest,
    avc_action_signature_payload,
};
use exo_core::{Signature, Timestamp, crypto};
use wasm_bindgen::prelude::*;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Core logic — returns `Result<_, String>` so it is fully testable on the
// native target. `wasm_bindgen::JsValue` only functions on wasm32, so the
// two wasm exports below are thin wrappers that map `String -> JsValue` at
// the boundary; all real logic + every rejection path lives here and is
// exercised by the native test block.
// ---------------------------------------------------------------------------

/// Parse a 32-byte Ed25519 secret from hex. The decoded bytes are held in
/// `Zeroizing` wrappers so they are wiped from memory on drop; the value is
/// never logged, returned, or echoed.
fn parse_subject_secret(label: &str, value: &str) -> Result<exo_core::SecretKey, String> {
    let bytes = Zeroizing::new(hex::decode(value).map_err(|e| format!("{label}: {e}"))?);
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("{label} must be 32 bytes"))?;
    if arr.iter().all(|byte| *byte == 0) {
        return Err(format!("{label} must not be all-zero"));
    }
    let arr = Zeroizing::new(arr);
    Ok(exo_core::SecretKey::from_bytes(*arr))
}

/// Derive the subject public key from the secret hex (only when the caller
/// asks to include it). Bytes are zeroized.
fn derive_subject_public_key(secret_hex: &str) -> Result<exo_core::PublicKey, String> {
    let bytes =
        Zeroizing::new(hex::decode(secret_hex).map_err(|e| format!("subject_secret_hex: {e}"))?);
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "subject_secret_hex must be 32 bytes".to_string())?;
    let arr = Zeroizing::new(arr);
    let keypair =
        crypto::KeyPair::from_secret_bytes(*arr).map_err(|e| format!("subject keypair: {e}"))?;
    Ok(*keypair.public_key())
}

/// Build a caller-supplied HLC timestamp; reject the zero sentinel (the
/// bridge never reads wall-clock — `now` must come from the caller).
fn parse_now(physical_ms: u64, logical: u32) -> Result<Timestamp, String> {
    if physical_ms == 0 {
        return Err(
            "validation_now timestamp must be a caller-supplied HLC (physical_ms != 0)".to_string(),
        );
    }
    Ok(Timestamp::new(physical_ms, logical))
}

/// Produce the subject action signature over the canonical action payload.
///
/// Calls [`avc_action_signature_payload`] directly — the same function the
/// node uses to reconstruct the bytes it verifies. This is the byte-parity
/// guarantee: do not replace this with a hand-rolled encoding.
fn sign_action_internal(
    credential: &AutonomousVolitionCredential,
    action: &AvcActionRequest,
    now: &Timestamp,
    secret: &exo_core::SecretKey,
) -> Result<Signature, String> {
    let payload = avc_action_signature_payload(credential, action, now)
        .map_err(|e| format!("avc action signature payload: {e}"))?;
    Ok(crypto::sign(&payload, secret))
}

/// Core of [`wasm_avc_sign_action`] — returns the signature as canonical serde
/// JSON (the shape `EmitReceiptRequest.subject_signature` deserializes).
fn sign_action_core(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
    subject_secret_hex: &str,
) -> Result<String, String> {
    let credential: AutonomousVolitionCredential =
        serde_json::from_str(credential_json).map_err(|e| format!("credential json: {e}"))?;
    let action: AvcActionRequest =
        serde_json::from_str(action_json).map_err(|e| format!("action json: {e}"))?;
    let now = parse_now(now_physical_ms, now_logical)?;
    let secret = parse_subject_secret("subject_secret_hex", subject_secret_hex)?;
    let signature = sign_action_internal(&credential, &action, &now, &secret)?;
    serde_json::to_string(&signature).map_err(|e| format!("signature json: {e}"))
}

/// Core of [`wasm_avc_build_emit_request`] — returns the full POST body JSON.
fn build_emit_request_core(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
    subject_secret_hex: &str,
    include_public_key: bool,
) -> Result<String, String> {
    let credential: AutonomousVolitionCredential =
        serde_json::from_str(credential_json).map_err(|e| format!("credential json: {e}"))?;
    let action: AvcActionRequest =
        serde_json::from_str(action_json).map_err(|e| format!("action json: {e}"))?;
    let now = parse_now(now_physical_ms, now_logical)?;
    let secret = parse_subject_secret("subject_secret_hex", subject_secret_hex)?;

    let signature = sign_action_internal(&credential, &action, &now, &secret)?;

    let subject_public_key = if include_public_key {
        Some(derive_subject_public_key(subject_secret_hex)?)
    } else {
        None
    };

    // Inner validation request reuses the canonical exo-avc struct so its
    // shape never drifts from what the node deserializes.
    let validation = AvcValidationRequest {
        credential,
        action: Some(action),
        now,
    };

    // The outer 3-field wrapper mirrors the node's EmitReceiptRequest
    // (defined in exo-node, not importable here). Keys + optionality match.
    let mut body = serde_json::Map::new();
    body.insert(
        "validation".to_string(),
        serde_json::to_value(&validation).map_err(|e| format!("validation json: {e}"))?,
    );
    body.insert(
        "subject_signature".to_string(),
        serde_json::to_value(&signature).map_err(|e| format!("subject_signature json: {e}"))?,
    );
    if let Some(pk) = subject_public_key {
        body.insert(
            "subject_public_key".to_string(),
            serde_json::to_value(pk).map_err(|e| format!("subject_public_key json: {e}"))?,
        );
    }

    serde_json::to_string(&serde_json::Value::Object(body))
        .map_err(|e| format!("emit request json: {e}"))
}

// ---------------------------------------------------------------------------
// WASM boundary — thin wrappers. JsValue only constructed here.
// ---------------------------------------------------------------------------

/// Sign an AVC action with the subject key. Returns the signature as its
/// canonical serde-JSON form — exactly the shape the node's
/// `EmitReceiptRequest.subject_signature` field deserializes.
///
/// `credential_json` must be the credential **exactly as issued** (no
/// reshaping) — the node compares it against the registered credential
/// byte-for-byte.
#[wasm_bindgen]
pub fn wasm_avc_sign_action(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
    subject_secret_hex: &str,
) -> Result<String, JsValue> {
    sign_action_core(
        credential_json,
        action_json,
        now_physical_ms,
        now_logical,
        subject_secret_hex,
    )
    .map_err(|e| JsValue::from_str(&e))
}

/// Build the full `POST /api/v1/avc/receipts/emit` request body:
/// `{ validation: { credential, action, now }, subject_signature,
/// subject_public_key? }`. The app POSTs this JSON verbatim.
///
/// When `include_public_key` is false (recommended for a registered
/// credential — the node resolves the subject key from the registry), the
/// `subject_public_key` field is omitted.
#[wasm_bindgen]
pub fn wasm_avc_build_emit_request(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
    subject_secret_hex: &str,
    include_public_key: bool,
) -> Result<String, JsValue> {
    build_emit_request_core(
        credential_json,
        action_json,
        now_physical_ms,
        now_logical,
        subject_secret_hex,
        include_public_key,
    )
    .map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use exo_authority::permission::Permission;
    use exo_avc::{
        AuthorityScope, AutonomyLevel, AvcConstraints, AvcSubjectKind, DataClass, DelegatedIntent,
        issue_avc,
    };
    use exo_core::{Did, Hash256, Timestamp, crypto};

    use super::*;

    fn did(label: &str) -> Did {
        Did::new(&format!("did:exo:{label}")).expect("did")
    }

    /// A registered-shaped credential whose subject is `subject_did`,
    /// self-issued for the test (issuer == subject is fine — action signing
    /// is over credential_id + action + now, independent of the issuer sig).
    fn test_credential(
        subject_did: &Did,
        issuer_secret: &exo_core::SecretKey,
    ) -> AutonomousVolitionCredential {
        let draft = exo_avc::AvcDraft {
            schema_version: exo_avc::AVC_SCHEMA_VERSION,
            issuer_did: subject_did.clone(),
            principal_did: subject_did.clone(),
            subject_did: subject_did.clone(),
            holder_did: None,
            subject_kind: AvcSubjectKind::Service {
                service_id: "test-svc".into(),
            },
            created_at: Timestamp::new(1_000_000, 0),
            expires_at: Some(Timestamp::new(9_000_000, 0)),
            delegated_intent: DelegatedIntent {
                intent_id: Hash256::from_bytes([7u8; 32]),
                purpose: "byte-parity test".into(),
                allowed_objectives: vec!["test".into()],
                prohibited_objectives: vec![],
                autonomy_level: AutonomyLevel::ExecuteWithinBounds,
                delegation_allowed: true,
            },
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Read, Permission::Write],
                tools: vec!["test-tool".into()],
                data_classes: vec![DataClass::Public, DataClass::Internal],
                counterparties: vec![],
                jurisdictions: vec!["US".into()],
            },
            constraints: AvcConstraints::permissive(),
            authority_chain: None,
            consent_refs: vec![],
            policy_refs: vec![],
            parent_avc_id: None,
        };
        issue_avc(draft, |bytes| crypto::sign(bytes, issuer_secret)).expect("issue credential")
    }

    fn test_action(actor_did: &Did) -> AvcActionRequest {
        AvcActionRequest {
            action_id: Hash256::from_bytes([3u8; 32]),
            actor_did: actor_did.clone(),
            requested_permission: Permission::Write,
            tool: Some("test-tool".into()),
            target_did: None,
            data_class: Some(DataClass::Internal),
            estimated_budget_minor_units: None,
            estimated_risk_bp: None,
            human_approval: None,
            requires_human_approval: false,
            action_name: Some("test-action".into()),
        }
    }

    /// THE byte-parity proof. The bridge signs an action; we independently
    /// reconstruct the canonical payload via `avc_action_signature_payload`
    /// (the node's function) and verify the bridge's signature against the
    /// subject public key. If the bridge ever reimplemented the encoding,
    /// the bytes would diverge and `crypto::verify` would fail.
    #[test]
    fn bridge_action_signature_is_accepted_by_node_verification_path() {
        let (subject_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("byte-parity-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);
        let now = Timestamp::new(1_500_000, 0);

        // Sign through the bridge's internal path.
        let signature =
            sign_action_internal(&credential, &action, &now, &subject_sk).expect("bridge signs");

        // Reconstruct the canonical payload the NODE verifies and check the
        // bridge's signature against the subject key — exactly what
        // exo-node's verify_subject_action_signature does.
        let node_payload =
            avc_action_signature_payload(&credential, &action, &now).expect("node payload");
        assert!(
            crypto::verify(&node_payload, &signature, &subject_pk),
            "bridge signature must verify against the node's canonical action payload"
        );
    }

    /// The public `wasm_avc_sign_action` entry point round-trips through JSON and
    /// produces a signature accepted by the node verification path.
    #[test]
    fn avc_sign_action_entrypoint_matches_node_payload() {
        let (subject_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("entrypoint-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);
        let now = Timestamp::new(2_000_000, 0);

        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let secret_hex = hex::encode(subject_sk.as_bytes());

        let sig_json = sign_action_core(&credential_json, &action_json, 2_000_000, 0, &secret_hex)
            .expect("sign_action_core");
        let signature: Signature = serde_json::from_str(&sig_json).expect("sig parse");

        let node_payload =
            avc_action_signature_payload(&credential, &action, &now).expect("node payload");
        assert!(
            crypto::verify(&node_payload, &signature, &subject_pk),
            "wasm_avc_sign_action output must verify against the node payload"
        );
    }

    /// `wasm_avc_build_emit_request` emits the 3-field wrapper the node expects.
    #[test]
    fn emit_request_has_expected_shape() {
        let (_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("emit-shape-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);

        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let secret_hex = hex::encode(subject_sk.as_bytes());

        let body = build_emit_request_core(
            &credential_json,
            &action_json,
            3_000_000,
            0,
            &secret_hex,
            false,
        )
        .expect("build emit request");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v.get("validation").is_some(), "has validation");
        assert!(
            v.get("subject_signature").is_some(),
            "has subject_signature"
        );
        assert!(
            v.get("subject_public_key").is_none(),
            "omits subject_public_key when not requested"
        );
        assert!(
            v["validation"].get("credential").is_some(),
            "validation.credential"
        );
        assert!(v["validation"].get("action").is_some(), "validation.action");
        assert!(v["validation"].get("now").is_some(), "validation.now");
    }

    /// Deterministic vector emitter. Ed25519 is deterministic (RFC 8032),
    /// and `issue_avc` + `avc_action_signature_payload` are canonical, so a
    /// fixed subject key + fixed credential + fixed action + fixed `now`
    /// yields a fixed signature. Run with `--ignored --nocapture` to print
    /// the checked-in vector at `test/avc_action_vector.json`.
    #[test]
    #[ignore]
    fn emit_deterministic_vector() {
        let subject_sk = exo_core::SecretKey::from_bytes([0x11u8; 32]);
        let issuer_sk = exo_core::SecretKey::from_bytes([0x22u8; 32]);
        let subject_kp = crypto::KeyPair::from_secret_bytes([0x11u8; 32]).unwrap();
        let subject_pk = *subject_kp.public_key();
        let subject_did = did("vector-subject");
        let credential = test_credential(&subject_did, &issuer_sk);
        let action = test_action(&subject_did);

        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let secret_hex = hex::encode([0x11u8; 32]);
        let sig_json =
            sign_action_core(&credential_json, &action_json, 1_700_000, 5, &secret_hex).unwrap();

        let vector = serde_json::json!({
            "_comment": "Byte-parity vector for wasm_avc_sign_action. Inputs are fixed; \
                         Ed25519 is deterministic so expected_signature is exact. \
                         Both the Rust test and bridge_verification.mjs assert the \
                         binding reproduces expected_signature AND it verifies against \
                         the node's avc_action_signature_payload.",
            "credential_json": credential_json,
            "action_json": action_json,
            "now_physical_ms": 1_700_000u64,
            "now_logical": 5u32,
            "subject_secret_hex": secret_hex,
            "subject_public_key_hex": hex::encode(subject_pk.as_bytes()),
            "expected_signature_json": sig_json,
        });
        println!(
            "AVC_VECTOR_JSON={}",
            serde_json::to_string_pretty(&vector).unwrap()
        );
        let _ = subject_sk;
    }

    /// Consume the checked-in vector: the binding must reproduce
    /// `expected_signature_json` exactly, AND that signature must verify
    /// against the node's canonical action payload. This is the literal
    /// "checked-in test vector proves the binding produces a node-accepted
    /// signature" guarantee. `bridge_verification.mjs` asserts the same
    /// vector against the compiled wasm artifact in CI.
    #[test]
    fn checked_in_vector_reproduces_and_verifies() {
        let raw = include_str!("../test/avc_action_vector.json");
        let v: serde_json::Value = serde_json::from_str(raw).expect("vector json");

        let credential_json = v["credential_json"].as_str().unwrap();
        let action_json = v["action_json"].as_str().unwrap();
        let now_ms = v["now_physical_ms"].as_u64().unwrap();
        let now_logical = v["now_logical"].as_u64().unwrap() as u32;
        let secret_hex = v["subject_secret_hex"].as_str().unwrap();
        let expected_sig_json = v["expected_signature_json"].as_str().unwrap();
        let subject_pk_hex = v["subject_public_key_hex"].as_str().unwrap();

        // 1. Exact reproduction (determinism).
        let produced = sign_action_core(
            credential_json,
            action_json,
            now_ms,
            now_logical,
            secret_hex,
        )
        .expect("sign");
        assert_eq!(
            produced, expected_sig_json,
            "binding must reproduce the checked-in expected signature exactly"
        );

        // 2. Node acceptance: verify against avc_action_signature_payload.
        let credential: AutonomousVolitionCredential =
            serde_json::from_str(credential_json).unwrap();
        let action: AvcActionRequest = serde_json::from_str(action_json).unwrap();
        let now = Timestamp::new(now_ms, now_logical);
        let signature: Signature = serde_json::from_str(expected_sig_json).unwrap();
        let pk_bytes: [u8; 32] = hex::decode(subject_pk_hex).unwrap().try_into().unwrap();
        let subject_pk = exo_core::PublicKey::from_bytes(pk_bytes);
        let node_payload = avc_action_signature_payload(&credential, &action, &now).unwrap();
        assert!(
            crypto::verify(&node_payload, &signature, &subject_pk),
            "checked-in vector signature must verify against the node payload"
        );
    }

    /// Zero/empty inputs are rejected loudly, never silently signed.
    #[test]
    fn rejects_zero_timestamp_and_bad_secret() {
        let (_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("reject-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);
        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let secret_hex = hex::encode(subject_sk.as_bytes());

        assert!(
            sign_action_core(&credential_json, &action_json, 0, 0, &secret_hex).is_err(),
            "zero timestamp must be rejected"
        );
        assert!(
            sign_action_core(&credential_json, &action_json, 1_000, 0, "00").is_err(),
            "short secret must be rejected"
        );
        assert!(
            sign_action_core(&credential_json, &action_json, 1_000, 0, &"0".repeat(64)).is_err(),
            "all-zero secret must be rejected"
        );
    }
}
