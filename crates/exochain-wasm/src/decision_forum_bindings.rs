//! Decision Forum bindings: DecisionObject lifecycle, constitution, TNC enforcement,
//! contestation, accountability, workflow, emergency

use wasm_bindgen::prelude::*;
use crate::serde_bridge::*;

/// Create a new DecisionObject with full BCTS lifecycle
#[wasm_bindgen]
pub fn wasm_create_decision(title: &str, class_json: &str, constitution_hash_hex: &str) -> Result<JsValue, JsValue> {
    let class: decision_forum::decision_object::DecisionClass = from_json_str(class_json)?;
    let hash_bytes = hex::decode(constitution_hash_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = hash_bytes.try_into()
        .map_err(|_| JsValue::from_str("hash must be 32 bytes"))?;
    let hash = exo_core::Hash256::from_bytes(arr);

    let mut clock = exo_core::hlc::HybridClock::new();
    let decision = decision_forum::decision_object::DecisionObject::new(title, class, hash, &mut clock);
    to_js_value(&decision)
}

/// Transition a DecisionObject to a new BCTS state
#[wasm_bindgen]
pub fn wasm_transition_decision(decision_json: &str, to_state_json: &str, actor_did: &str) -> Result<JsValue, JsValue> {
    let mut decision: decision_forum::decision_object::DecisionObject = from_json_str(decision_json)?;
    let to_state: exo_core::bcts::BctsState = from_json_str(to_state_json)?;
    let actor = exo_core::Did::new(actor_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let mut clock = exo_core::hlc::HybridClock::new();

    decision.transition(to_state, &actor, &mut clock)
        .map_err(|e| JsValue::from_str(&format!("Transition error: {e}")))?;
    to_js_value(&decision)
}

/// Add a vote to a DecisionObject
#[wasm_bindgen]
pub fn wasm_add_vote(decision_json: &str, vote_json: &str) -> Result<JsValue, JsValue> {
    let mut decision: decision_forum::decision_object::DecisionObject = from_json_str(decision_json)?;
    let vote: decision_forum::decision_object::Vote = from_json_str(vote_json)?;

    decision.add_vote(vote)
        .map_err(|e| JsValue::from_str(&format!("Vote error: {e}")))?;
    to_js_value(&decision)
}

/// Add evidence to a DecisionObject
#[wasm_bindgen]
pub fn wasm_add_evidence(decision_json: &str, evidence_json: &str) -> Result<JsValue, JsValue> {
    let mut decision: decision_forum::decision_object::DecisionObject = from_json_str(decision_json)?;
    let evidence: decision_forum::decision_object::EvidenceItem = from_json_str(evidence_json)?;

    decision.add_evidence(evidence)
        .map_err(|e| JsValue::from_str(&format!("Evidence error: {e}")))?;
    to_js_value(&decision)
}

/// Check if a DecisionObject is in a terminal state
#[wasm_bindgen]
pub fn wasm_decision_is_terminal(decision_json: &str) -> Result<bool, JsValue> {
    let decision: decision_forum::decision_object::DecisionObject = from_json_str(decision_json)?;
    Ok(decision.is_terminal())
}

/// Compute the content hash of a DecisionObject (audit fingerprint)
#[wasm_bindgen]
pub fn wasm_decision_content_hash(decision_json: &str) -> Result<String, JsValue> {
    let decision: decision_forum::decision_object::DecisionObject = from_json_str(decision_json)?;
    let hash = decision.content_hash()
        .map_err(|e| JsValue::from_str(&format!("Hash error: {e}")))?;
    Ok(hex::encode(hash.as_bytes()))
}

/// File a challenge against a decision (contestation - GOV-008)
#[wasm_bindgen]
pub fn wasm_file_challenge(challenger_did: &str, decision_id: &str, ground_json: &str, evidence_hash_hex: &str) -> Result<JsValue, JsValue> {
    let challenger = exo_core::Did::new(challenger_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let id: uuid::Uuid = decision_id.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid UUID: {e}")))?;
    let ground: exo_governance::challenge::ChallengeGround = from_json_str(ground_json)?;
    let evidence_bytes = hex::decode(evidence_hash_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = evidence_bytes.try_into()
        .map_err(|_| JsValue::from_str("evidence hash must be 32 bytes"))?;
    let evidence_hash = exo_core::Hash256::from_bytes(arr);

    let mut clock = exo_core::hlc::HybridClock::new();
    let timestamp = clock.now();

    let challenge = decision_forum::contestation::file_challenge(id, &challenger, ground, evidence_hash, timestamp);
    to_js_value(&challenge)
}

/// Propose an accountability action (GOV-012)
#[wasm_bindgen]
pub fn wasm_propose_accountability(target_did: &str, proposer_did: &str, action_type_json: &str, reason: &str, evidence_hash_hex: &str) -> Result<JsValue, JsValue> {
    let target = exo_core::Did::new(target_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let proposer = exo_core::Did::new(proposer_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let action_type: decision_forum::accountability::AccountabilityActionType = from_json_str(action_type_json)?;
    let evidence_bytes = hex::decode(evidence_hash_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = evidence_bytes.try_into()
        .map_err(|_| JsValue::from_str("evidence hash must be 32 bytes"))?;
    let evidence_hash = exo_core::Hash256::from_bytes(arr);

    let mut clock = exo_core::hlc::HybridClock::new();
    let timestamp = clock.now();

    let action = decision_forum::accountability::propose(action_type, &target, &proposer, reason, evidence_hash, timestamp);
    to_js_value(&action)
}

/// Get workflow stages for a decision (Syntaxis integration)
#[wasm_bindgen]
pub fn wasm_workflow_stages() -> Result<JsValue, JsValue> {
    let stages = vec![
        "Draft", "Submitted", "IdentityResolved", "ConsentValidated",
        "Deliberated", "Verified", "Governed", "Approved",
        "Executed", "Recorded", "Closed",
    ];
    to_js_value(&stages)
}
