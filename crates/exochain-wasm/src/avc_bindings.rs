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
//! `#[cfg(test)]` block below pins this against drift by returning bytes from
//! this bridge, signing them externally, and verifying through the node's own
//! verification path ([`exo_core::crypto::verify`] over the same payload).

use exo_avc::{
    AutonomousVolitionCredential, AvcActionRequest, AvcValidationRequest,
    avc_action_signature_payload,
};
use exo_core::{PublicKey, Signature, Timestamp};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Core logic — returns `Result<_, String>` so it is fully testable on the
// native target. `wasm_bindgen::JsValue` only functions on wasm32, so the
// two wasm exports below are thin wrappers that map `String -> JsValue` at
// the boundary; all real logic + every rejection path lives here and is
// exercised by the native test block.
// ---------------------------------------------------------------------------

fn parse_subject_public_key_hex(value: &str) -> Result<Option<PublicKey>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let bytes = hex::decode(value).map_err(|e| format!("subject_public_key_hex: {e}"))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "subject_public_key_hex must be 32 bytes".to_string())?;
    if arr.iter().all(|byte| *byte == 0) {
        return Err("subject_public_key_hex must not be all-zero".to_string());
    }
    Ok(Some(PublicKey::from_bytes(arr)))
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

/// Core of [`wasm_avc_action_signing_payload`] — returns canonical CBOR bytes
/// for signing by external key management.
fn action_signing_payload_core(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
) -> Result<Vec<u8>, String> {
    let credential: AutonomousVolitionCredential =
        serde_json::from_str(credential_json).map_err(|e| format!("credential json: {e}"))?;
    let action: AvcActionRequest =
        serde_json::from_str(action_json).map_err(|e| format!("action json: {e}"))?;
    let now = parse_now(now_physical_ms, now_logical)?;
    avc_action_signature_payload(&credential, &action, &now)
        .map_err(|e| format!("avc action signature payload: {e}"))
}

/// Core of [`wasm_avc_build_emit_request_from_signature`] — returns the full
/// POST body JSON after the caller signs the canonical payload outside WASM.
fn build_emit_request_from_signature_core(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
    subject_signature_json: &str,
    subject_public_key_hex: &str,
) -> Result<String, String> {
    let credential: AutonomousVolitionCredential =
        serde_json::from_str(credential_json).map_err(|e| format!("credential json: {e}"))?;
    let action: AvcActionRequest =
        serde_json::from_str(action_json).map_err(|e| format!("action json: {e}"))?;
    let now = parse_now(now_physical_ms, now_logical)?;
    let signature: Signature =
        serde_json::from_str(subject_signature_json).map_err(|e| format!("signature json: {e}"))?;
    if signature.is_empty() {
        return Err("subject_signature_json must not be empty".to_string());
    }

    // Reconstruct the canonical payload so request building fails before
    // transport if the credential/action/timestamp tuple cannot be signed.
    avc_action_signature_payload(&credential, &action, &now)
        .map_err(|e| format!("avc action signature payload: {e}"))?;
    let subject_public_key = parse_subject_public_key_hex(subject_public_key_hex)?;

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

/// Legacy raw subject-secret signing entry point.
///
/// This fails closed because public WASM cannot be the subject-key custody
/// boundary for AVC receipts. Use [`wasm_avc_action_signing_payload`], sign the
/// returned canonical CBOR bytes with external key management, then call
/// [`wasm_avc_build_emit_request_from_signature`].
#[wasm_bindgen]
pub fn wasm_avc_sign_action(
    _credential_json: &str,
    _action_json: &str,
    _now_physical_ms: u64,
    _now_logical: u32,
    _raw_subject_key_material: &str,
) -> Result<String, JsValue> {
    Err(JsValue::from_str(
        "raw AVC subject-key signing is disabled at the WASM boundary; use wasm_avc_action_signing_payload, sign externally, then call wasm_avc_build_emit_request_from_signature",
    ))
}

/// Return the canonical `exo.avc.action.v1` CBOR payload bytes for an AVC
/// subject action. The caller signs these bytes outside WASM.
#[wasm_bindgen]
pub fn wasm_avc_action_signing_payload(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
) -> Result<Vec<u8>, JsValue> {
    action_signing_payload_core(credential_json, action_json, now_physical_ms, now_logical)
        .map_err(|e| JsValue::from_str(&e))
}

/// Legacy raw subject-secret request builder.
///
/// This fails closed for the same reason as [`wasm_avc_sign_action`].
#[wasm_bindgen]
pub fn wasm_avc_build_emit_request(
    _credential_json: &str,
    _action_json: &str,
    _now_physical_ms: u64,
    _now_logical: u32,
    _raw_subject_key_material: &str,
    _include_public_key: bool,
) -> Result<String, JsValue> {
    Err(JsValue::from_str(
        "raw AVC subject-key emit request building is disabled at the WASM boundary; use wasm_avc_action_signing_payload, sign externally, then call wasm_avc_build_emit_request_from_signature",
    ))
}

/// Build the full `POST /api/v1/avc/receipts/emit` request body from an
/// externally produced signature:
/// `{ validation: { credential, action, now }, subject_signature,
/// subject_public_key? }`. The app POSTs this JSON verbatim.
///
/// `subject_public_key_hex` is optional; pass an empty string to omit it. For
/// registered credentials this should be omitted so the node resolves the actor
/// key from its registry.
#[wasm_bindgen]
pub fn wasm_avc_build_emit_request_from_signature(
    credential_json: &str,
    action_json: &str,
    now_physical_ms: u64,
    now_logical: u32,
    subject_signature_json: &str,
    subject_public_key_hex: &str,
) -> Result<String, JsValue> {
    build_emit_request_from_signature_core(
        credential_json,
        action_json,
        now_physical_ms,
        now_logical,
        subject_signature_json,
        subject_public_key_hex,
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

    fn signature_json_for_payload(payload: &[u8], subject_sk: &exo_core::SecretKey) -> String {
        serde_json::to_string(&crypto::sign(payload, subject_sk)).expect("signature json")
    }

    /// THE byte-parity proof. The bridge returns exactly the canonical payload
    /// reconstructed by the node's `avc_action_signature_payload`.
    #[test]
    fn bridge_action_payload_matches_node_verification_path() {
        let (_subject_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("byte-parity-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);
        let now = Timestamp::new(1_500_000, 0);

        let node_payload =
            avc_action_signature_payload(&credential, &action, &now).expect("node payload");
        let bridge_payload = action_signing_payload_core(
            &serde_json::to_string(&credential).unwrap(),
            &serde_json::to_string(&action).unwrap(),
            now.physical_ms,
            now.logical,
        )
        .expect("bridge payload");
        assert_eq!(
            bridge_payload, node_payload,
            "bridge payload bytes must match the node's canonical action payload"
        );
    }

    /// The payload entry point round-trips through JSON and can be signed by an
    /// external signer for node acceptance.
    #[test]
    fn avc_action_payload_entrypoint_supports_external_signature() {
        let (subject_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("entrypoint-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);

        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let payload = action_signing_payload_core(&credential_json, &action_json, 2_000_000, 0)
            .expect("action payload");
        let signature: Signature =
            serde_json::from_str(&signature_json_for_payload(&payload, &subject_sk))
                .expect("sig parse");

        assert!(
            crypto::verify(&payload, &signature, &subject_pk),
            "externally signed wasm_avc_action_signing_payload output must verify"
        );
    }

    /// `wasm_avc_build_emit_request_from_signature` emits the wrapper the node expects.
    #[test]
    fn emit_request_has_expected_shape() {
        let (subject_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("emit-shape-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);

        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let payload = action_signing_payload_core(&credential_json, &action_json, 3_000_000, 0)
            .expect("action payload");
        let signature_json = signature_json_for_payload(&payload, &subject_sk);

        let body = build_emit_request_from_signature_core(
            &credential_json,
            &action_json,
            3_000_000,
            0,
            &signature_json,
            "",
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
        assert_eq!(v["subject_signature"].to_string(), signature_json);

        let body_with_key = build_emit_request_from_signature_core(
            &credential_json,
            &action_json,
            3_000_000,
            0,
            &signature_json,
            &hex::encode(subject_pk.as_bytes()),
        )
        .expect("build emit request with key");
        let with_key: serde_json::Value = serde_json::from_str(&body_with_key).unwrap();
        assert!(
            with_key.get("subject_public_key").is_some(),
            "includes subject_public_key when explicitly supplied"
        );
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
        let payload =
            action_signing_payload_core(&credential_json, &action_json, 1_700_000, 5).unwrap();
        let sig_json = signature_json_for_payload(&payload, &subject_sk);

        let vector = serde_json::json!({
            "_comment": "Byte-parity vector for wasm_avc_action_signing_payload. Inputs are fixed; \
                         Ed25519 is deterministic so expected_signature is exact. \
                         Rust and bridge_verification.mjs assert the binding reproduces \
                         the node's avc_action_signature_payload bytes and an external \
                         signature verifies against those bytes.",
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

    /// Consume the checked-in vector: the binding must reproduce the node's
    /// canonical action payload, and an external signature over those bytes must
    /// match the checked-in signature and verify.
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

        let credential: AutonomousVolitionCredential =
            serde_json::from_str(credential_json).unwrap();
        let action: AvcActionRequest = serde_json::from_str(action_json).unwrap();
        let now = Timestamp::new(now_ms, now_logical);
        let bridge_payload =
            action_signing_payload_core(credential_json, action_json, now_ms, now_logical)
                .expect("payload");
        let node_payload = avc_action_signature_payload(&credential, &action, &now).unwrap();
        assert_eq!(
            bridge_payload, node_payload,
            "binding must reproduce the node's checked-in action payload exactly"
        );

        let sk_bytes: [u8; 32] = hex::decode(secret_hex).unwrap().try_into().unwrap();
        let subject_sk = exo_core::SecretKey::from_bytes(sk_bytes);
        let produced = signature_json_for_payload(&bridge_payload, &subject_sk);
        assert_eq!(
            produced, expected_sig_json,
            "external signature over bridge payload must reproduce the checked-in expected signature"
        );

        let signature: Signature = serde_json::from_str(expected_sig_json).unwrap();
        let pk_bytes: [u8; 32] = hex::decode(subject_pk_hex).unwrap().try_into().unwrap();
        let subject_pk = exo_core::PublicKey::from_bytes(pk_bytes);
        assert!(
            crypto::verify(&node_payload, &signature, &subject_pk),
            "checked-in vector signature must verify against the node payload"
        );
    }

    /// Zero/empty inputs are rejected loudly, never silently request-built.
    #[test]
    fn rejects_zero_timestamp_and_bad_signature_or_public_key() {
        let (_pk, subject_sk) = crypto::generate_keypair();
        let subject_did = did("reject-subject");
        let credential = test_credential(&subject_did, &subject_sk);
        let action = test_action(&subject_did);
        let credential_json = serde_json::to_string(&credential).unwrap();
        let action_json = serde_json::to_string(&action).unwrap();
        let payload =
            action_signing_payload_core(&credential_json, &action_json, 1_000, 0).expect("payload");
        let signature_json = signature_json_for_payload(&payload, &subject_sk);

        assert!(
            action_signing_payload_core(&credential_json, &action_json, 0, 0).is_err(),
            "zero timestamp must be rejected"
        );
        assert!(
            build_emit_request_from_signature_core(
                &credential_json,
                &action_json,
                1_000,
                0,
                r#""Empty""#,
                "",
            )
            .is_err(),
            "bad signature JSON must be rejected"
        );
        assert!(
            build_emit_request_from_signature_core(
                &credential_json,
                &action_json,
                1_000,
                0,
                &signature_json,
                "00",
            )
            .is_err(),
            "short subject public key must be rejected"
        );
    }
}
