//! Legal bindings: evidence chain of custody, fiduciary duty, eDiscovery

use wasm_bindgen::prelude::*;
use crate::serde_bridge::*;

/// Create a new piece of evidence with chain of custody
#[wasm_bindgen]
pub fn wasm_create_evidence(content: &[u8], type_tag: &str, creator_did: &str) -> Result<JsValue, JsValue> {
    let creator = exo_core::Did::new(creator_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let mut clock = exo_core::hlc::HybridClock::new();
    let timestamp = clock.now();
    let evidence = exo_legal::evidence::create_evidence(content, &creator, type_tag, timestamp)
        .map_err(|e| JsValue::from_str(&format!("Evidence error: {e}")))?;
    to_js_value(&evidence)
}

/// Verify the chain of custody for a piece of evidence
#[wasm_bindgen]
pub fn wasm_verify_chain_of_custody(evidence_json: &str) -> Result<JsValue, JsValue> {
    let evidence: exo_legal::evidence::Evidence = from_json_str(evidence_json)?;
    match exo_legal::evidence::verify_chain_of_custody(&evidence) {
        Ok(()) => to_js_value(&serde_json::json!({"valid": true})),
        Err(e) => to_js_value(&serde_json::json!({"valid": false, "error": format!("{e}")})),
    }
}

/// Check fiduciary duty compliance
#[wasm_bindgen]
pub fn wasm_check_fiduciary_duty(duty_json: &str, actions_json: &str) -> Result<JsValue, JsValue> {
    let duty: exo_legal::fiduciary::FiduciaryDuty = from_json_str(duty_json)?;
    let actions: Vec<exo_legal::fiduciary::AuditEntry> = from_json_str(actions_json)?;
    let result = exo_legal::fiduciary::check_duty_compliance(&duty, &actions);
    to_js_value(&result)
}

/// Search evidence corpus (eDiscovery)
#[wasm_bindgen]
pub fn wasm_ediscovery_search(request_json: &str, corpus_json: &str) -> Result<JsValue, JsValue> {
    let request: exo_legal::ediscovery::DiscoveryRequest = from_json_str(request_json)?;
    let corpus: Vec<exo_legal::evidence::Evidence> = from_json_str(corpus_json)?;
    let response = exo_legal::ediscovery::search(&request, &corpus);
    to_js_value(&response)
}
