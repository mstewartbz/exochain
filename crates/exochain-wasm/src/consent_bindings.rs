//! Consent bindings: bailment lifecycle, consent enforcement

use wasm_bindgen::prelude::*;

use crate::serde_bridge::*;

/// Propose a new bailment (consent-conditioned data sharing)
#[wasm_bindgen]
pub fn wasm_propose_bailment(
    bailor_did: &str,
    bailee_did: &str,
    terms: &[u8],
    bailment_type_json: &str,
) -> Result<JsValue, JsValue> {
    let bailor = exo_core::Did::new(bailor_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let bailee = exo_core::Did::new(bailee_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let bailment_type: exo_consent::BailmentType = from_json_str(bailment_type_json)?;
    let bailment = exo_consent::bailment::propose(&bailor, &bailee, terms, bailment_type);
    to_js_value(&bailment)
}

/// Check if a bailment is currently active
#[wasm_bindgen]
pub fn wasm_bailment_is_active(bailment_json: &str) -> Result<bool, JsValue> {
    let bailment: exo_consent::Bailment = from_json_str(bailment_json)?;
    let mut clock = exo_core::hlc::HybridClock::new();
    let now = clock.now();
    Ok(exo_consent::bailment::is_active(&bailment, &now))
}
