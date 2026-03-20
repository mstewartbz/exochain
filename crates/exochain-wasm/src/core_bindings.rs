//! Core bindings: crypto, hashing, BCTS state machine, events, HLC

use wasm_bindgen::prelude::*;
use crate::serde_bridge::*;

// ── Hashing ──────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_hash_bytes(data: &[u8]) -> String {
    let hash = exo_core::hash::canonical_hash(data);
    hex::encode(hash.as_bytes())
}

#[wasm_bindgen]
pub fn wasm_hash_structured(json: &str) -> Result<String, JsValue> {
    let val: serde_json::Value = from_json_str(json)?;
    let hash = exo_core::hash::hash_structured(&val)
        .map_err(|e| JsValue::from_str(&format!("Hash error: {e}")))?;
    Ok(hex::encode(hash.as_bytes()))
}

// ── Merkle Trees ─────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_merkle_root(leaves_json: &str) -> Result<String, JsValue> {
    let hex_leaves: Vec<String> = from_json_str(leaves_json)?;
    let leaves: Vec<exo_core::Hash256> = hex_leaves
        .iter()
        .map(|h| {
            let bytes = hex::decode(h).map_err(|e| format!("hex: {e}"))?;
            let arr: [u8; 32] = bytes.try_into().map_err(|_| "not 32 bytes")?;
            Ok(exo_core::Hash256::from_bytes(arr))
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| JsValue::from_str(&e))?;
    let root = exo_core::hash::merkle_root(&leaves);
    Ok(hex::encode(root.as_bytes()))
}

// ── Crypto ───────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_generate_keypair() -> Result<JsValue, JsValue> {
    let kp = exo_core::crypto::KeyPair::generate();
    let result = serde_json::json!({
        "public_key": hex::encode(kp.public_key().as_bytes()),
        "secret_key": hex::encode(kp.secret_key().as_bytes()),
    });
    to_js_value(&result)
}

#[wasm_bindgen]
pub fn wasm_sign(message: &[u8], secret_hex: &str) -> Result<String, JsValue> {
    let secret_bytes = hex::decode(secret_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = secret_bytes.try_into()
        .map_err(|_| JsValue::from_str("secret key must be 32 bytes"))?;
    let secret = exo_core::SecretKey::from_bytes(arr);
    let sig = exo_core::crypto::sign(message, &secret);
    let sig_json = serde_json::to_string(&sig)
        .map_err(|e| JsValue::from_str(&format!("serialize: {e}")))?;
    Ok(sig_json)
}

#[wasm_bindgen]
pub fn wasm_verify(message: &[u8], signature_json: &str, public_hex: &str) -> Result<bool, JsValue> {
    let sig: exo_core::Signature = from_json_str(signature_json)?;
    let pub_bytes = hex::decode(public_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = pub_bytes.try_into()
        .map_err(|_| JsValue::from_str("public key must be 32 bytes"))?;
    let pubkey = exo_core::PublicKey::from_bytes(arr);
    Ok(exo_core::crypto::verify(message, &sig, &pubkey))
}

// ── BCTS State Machine ──────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_bcts_valid_transitions(state_json: &str) -> Result<JsValue, JsValue> {
    let state: exo_core::bcts::BctsState = from_json_str(state_json)?;
    let transitions = state.valid_transitions();
    to_js_value(&transitions)
}

#[wasm_bindgen]
pub fn wasm_bcts_is_terminal(state_json: &str) -> Result<bool, JsValue> {
    let state: exo_core::bcts::BctsState = from_json_str(state_json)?;
    Ok(matches!(state, exo_core::bcts::BctsState::Closed | exo_core::bcts::BctsState::Denied))
}

// ── Events ───────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_create_signed_event(
    event_type_json: &str,
    payload: &[u8],
    source_did: &str,
    secret_hex: &str,
) -> Result<JsValue, JsValue> {
    let event_type: exo_core::events::EventType = from_json_str(event_type_json)?;
    let did = exo_core::Did::new(source_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let secret_bytes = hex::decode(secret_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = secret_bytes.try_into()
        .map_err(|_| JsValue::from_str("secret key must be 32 bytes"))?;
    let secret = exo_core::SecretKey::from_bytes(arr);

    let mut clock = exo_core::hlc::HybridClock::new();
    let ts = clock.now();
    let corr = exo_core::CorrelationId::new();

    let event = exo_core::events::create_signed_event(corr, ts, event_type, payload.to_vec(), did, &secret);
    to_js_value(&event)
}
