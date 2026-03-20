//! Escalation bindings: anomaly detection, triage, Sybil adjudication, kanban

use wasm_bindgen::prelude::*;

use crate::serde_bridge::*;

/// Evaluate detection signals and produce threat assessment
#[wasm_bindgen]
pub fn wasm_evaluate_signals(signals_json: &str) -> Result<JsValue, JsValue> {
    let signals: Vec<exo_escalation::detector::DetectionSignal> = from_json_str(signals_json)?;
    let assessment = exo_escalation::detector::evaluate_signals(&signals);
    to_js_value(&assessment)
}

/// Escalate a detection signal to create a case
#[wasm_bindgen]
pub fn wasm_escalate(signal_json: &str, path_json: &str) -> Result<JsValue, JsValue> {
    let signal: exo_escalation::detector::DetectionSignal = from_json_str(signal_json)?;
    let path: exo_escalation::escalation::EscalationPath = from_json_str(path_json)?;
    let case = exo_escalation::escalation::escalate(&signal, &path);
    to_js_value(&case)
}

/// Record feedback on an escalation case (learning loop)
#[wasm_bindgen]
pub fn wasm_record_feedback(entries_json: &str, entry_json: &str) -> Result<JsValue, JsValue> {
    let existing_entries: Vec<exo_escalation::feedback::FeedbackEntry> =
        from_json_str(entries_json)?;
    let entry: exo_escalation::feedback::FeedbackEntry = from_json_str(entry_json)?;
    let mut log = exo_escalation::feedback::FeedbackLog::default();
    for e in existing_entries {
        exo_escalation::feedback::record_feedback(&mut log, e);
    }
    exo_escalation::feedback::record_feedback(&mut log, entry);
    // FeedbackLog doesn't derive Serialize, so serialize the entries directly
    to_js_value(&log.entries)
}

/// Apply learnings from feedback to generate policy recommendations
#[wasm_bindgen]
pub fn wasm_apply_learnings(feedbacks_json: &str) -> Result<JsValue, JsValue> {
    let feedbacks: Vec<exo_escalation::feedback::FeedbackEntry> = from_json_str(feedbacks_json)?;
    let recommendations = exo_escalation::feedback::apply_learnings(&feedbacks);
    to_js_value(&recommendations)
}

/// Check completeness of an escalation case
#[wasm_bindgen]
pub fn wasm_check_completeness(case_json: &str) -> Result<JsValue, JsValue> {
    let case: exo_escalation::escalation::EscalationCase = from_json_str(case_json)?;
    let result = exo_escalation::completeness::check_completeness(&case);
    to_js_value(&serde_json::json!({
        "complete": matches!(result, exo_escalation::completeness::CompletenessResult::Complete),
        "details": format!("{result:?}"),
    }))
}
