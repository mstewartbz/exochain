//! Governance bindings: quorum, clearance, conflict, challenge, audit

use wasm_bindgen::prelude::*;
use crate::serde_bridge::*;

/// Compute quorum result from approvals and policy
#[wasm_bindgen]
pub fn wasm_compute_quorum(approvals_json: &str, policy_json: &str) -> Result<JsValue, JsValue> {
    let approvals: Vec<exo_governance::quorum::Approval> = from_json_str(approvals_json)?;
    let policy: exo_governance::quorum::QuorumPolicy = from_json_str(policy_json)?;
    let result = exo_governance::quorum::compute_quorum(&approvals, &policy);
    // QuorumResult doesn't derive Serialize, so format it manually
    let json = match result {
        exo_governance::quorum::QuorumResult::Met { independent_count, total_count } =>
            serde_json::json!({"status": "Met", "independent_count": independent_count, "total_count": total_count}),
        exo_governance::quorum::QuorumResult::NotMet { reason } =>
            serde_json::json!({"status": "NotMet", "reason": reason}),
        exo_governance::quorum::QuorumResult::Contested { challenge } =>
            serde_json::json!({"status": "Contested", "challenge": challenge}),
    };
    to_js_value(&json)
}

/// Check clearance level for an actor on an action
#[wasm_bindgen]
pub fn wasm_check_clearance(actor_did: &str, action: &str, policy_json: &str) -> Result<JsValue, JsValue> {
    let actor = exo_core::Did::new(actor_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let policy: exo_governance::clearance::ClearancePolicy = from_json_str(policy_json)?;
    // ClearanceRegistry doesn't derive Deserialize, so we build a default one
    // with the actor set to Governor level for checking purposes
    let mut registry = exo_governance::clearance::ClearanceRegistry::default();
    registry.set_level(actor.clone(), exo_governance::clearance::ClearanceLevel::Governor);
    let decision = exo_governance::clearance::check_clearance(&actor, action, &policy, &registry);
    // ClearanceDecision doesn't derive Serialize, format manually
    let json = match decision {
        exo_governance::clearance::ClearanceDecision::Granted =>
            serde_json::json!({"status": "Granted"}),
        exo_governance::clearance::ClearanceDecision::Denied { missing_level } =>
            serde_json::json!({"status": "Denied", "missing_level": format!("{missing_level}")}),
        exo_governance::clearance::ClearanceDecision::InsufficientIndependence { details } =>
            serde_json::json!({"status": "InsufficientIndependence", "details": details}),
    };
    to_js_value(&json)
}

/// Check for conflicts of interest
#[wasm_bindgen]
pub fn wasm_check_conflicts(actor_did: &str, action_json: &str, declarations_json: &str) -> Result<JsValue, JsValue> {
    let actor = exo_core::Did::new(actor_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let action: exo_governance::conflict::ActionRequest = from_json_str(action_json)?;
    let declarations: Vec<exo_governance::conflict::ConflictDeclaration> = from_json_str(declarations_json)?;
    let conflicts = exo_governance::conflict::check_conflicts(&actor, &action, &declarations);
    let must_recuse = exo_governance::conflict::must_recuse(&conflicts);
    to_js_value(&serde_json::json!({
        "conflicts": conflicts,
        "must_recuse": must_recuse,
    }))
}

/// Append to a hash-chained audit log
#[wasm_bindgen]
pub fn wasm_audit_append(actor_did: &str, action: &str, result: &str, evidence_hash_hex: &str) -> Result<JsValue, JsValue> {
    let actor = exo_core::Did::new(actor_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let evidence_bytes = hex::decode(evidence_hash_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = evidence_bytes.try_into()
        .map_err(|_| JsValue::from_str("evidence hash must be 32 bytes"))?;

    let mut log = exo_governance::audit::AuditLog::new();
    let entry = exo_governance::audit::create_entry(&log, actor, action.to_string(), result.to_string(), arr);
    exo_governance::audit::append(&mut log, entry)
        .map_err(|e| JsValue::from_str(&format!("Audit error: {e}")))?;
    // AuditLog doesn't derive Serialize, return summary
    to_js_value(&serde_json::json!({
        "entries": log.len(),
        "head_hash": hex::encode(log.head_hash()),
    }))
}

/// Verify the integrity of an audit log's hash chain
#[wasm_bindgen]
pub fn wasm_audit_verify(entries_json: &str) -> Result<JsValue, JsValue> {
    let entries: Vec<exo_governance::audit::AuditEntry> = from_json_str(entries_json)?;
    let mut log = exo_governance::audit::AuditLog::new();
    // Rebuild the log from entries
    for entry in entries {
        if let Err(e) = exo_governance::audit::append(&mut log, entry) {
            return to_js_value(&serde_json::json!({"valid": false, "error": format!("{e}")}));
        }
    }
    match exo_governance::audit::verify_chain(&log) {
        Ok(()) => to_js_value(&serde_json::json!({"valid": true})),
        Err(e) => to_js_value(&serde_json::json!({"valid": false, "error": format!("{e}")})),
    }
}

/// File a governance challenge
#[wasm_bindgen]
pub fn wasm_file_governance_challenge(
    challenger_did: &str,
    target_hash_hex: &str,
    ground_json: &str,
    evidence: &[u8],
) -> Result<JsValue, JsValue> {
    let challenger = exo_core::Did::new(challenger_did)
        .map_err(|e| JsValue::from_str(&format!("DID error: {e}")))?;
    let target_bytes = hex::decode(target_hash_hex)
        .map_err(|e| JsValue::from_str(&format!("hex: {e}")))?;
    let arr: [u8; 32] = target_bytes.try_into()
        .map_err(|_| JsValue::from_str("target must be 32 bytes"))?;
    let ground: exo_governance::challenge::ChallengeGround = from_json_str(ground_json)?;
    let challenge = exo_governance::challenge::file_challenge(&challenger, &arr, ground, evidence);
    to_js_value(&challenge)
}
