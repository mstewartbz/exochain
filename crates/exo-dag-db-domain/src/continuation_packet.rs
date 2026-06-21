//! Rust-owned continuation packet compatibility contract.
//!
//! Existing `dagdb_continuation_packet_v1` artifacts are produced by
//! `tools/dagdb_continuation_packet.py`. Rust validates that Python v1 shape and
//! converts it into a compact canonical packet. This module does not enable
//! default memory, activate routes, call production runtime surfaces, mutate a
//! database, or satisfy final thesis/operator acceptance gates.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Continuation packet schema shared with existing Python compatibility tooling.
pub const DAGDB_CONTINUATION_PACKET_SCHEMA_VERSION: &str = "dagdb_continuation_packet_v1";
/// Continuation packet report schema emitted by existing Python tooling.
pub const DAGDB_CONTINUATION_PACKET_REPORT_SCHEMA_VERSION: &str =
    "dagdb_continuation_packet_report_v1";

use crate::tenant::{LOCAL_DEV_NAMESPACE, LOCAL_DEV_TENANT_ID};

const DEFAULT_TENANT_ID: &str = LOCAL_DEV_TENANT_ID;
const DEFAULT_NAMESPACE: &str = LOCAL_DEV_NAMESPACE;
const DEFAULT_DB_SET_VERSION: &str = "project_memory_v3";

const REQUIRED_BLOCKED_NON_CLAIMS: &[&str] = &[
    "default_memory_activation_blocked",
    "final_thesis_acceptance_blocked",
    "production_runtime_not_approved",
    "route_activation_not_approved",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "~/.",
    "begin private key",
    "private key-----",
    "authorization",
    "database_url",
    ".env",
    "mongodb://",
    "mysql://",
    "password",
    "postgres://",
    "postgresql://",
    "redis://",
    "secret",
    "sk-proj-",
    "sqlite://",
    "raw_body",
    "raw_document_body",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_private_payload",
    "raw_prompt_body",
    "source_excerpt",
];

const RAW_SOURCE_BODY_MARKERS: &[&str] = &[
    "agent transcript",
    "assistant:",
    "human:",
    "raw source body",
    "raw transcript",
    "user:",
    "<|im_start|>",
];

const BLOCKED_APPROVAL_SURFACES: &[&[&str]] = &[
    &["production", "runtime"],
    &["default", "memory"],
    &["route"],
    &["route", "activation"],
    &["m63"],
    &["final", "thesis"],
    &["final", "acceptance"],
];

const BLOCKED_APPROVAL_TERMS: &[&str] = &[
    "approved",
    "approve",
    "approval",
    "accepted",
    "accept",
    "acceptance",
    "pass",
    "passed",
    "activated",
    "activation",
    "active",
    "enabled",
    "enablement",
    "granted",
    "grant",
    "complete",
    "completed",
    "completion",
];

const BLOCKED_APPROVAL_NEGATIONS: &[&str] = &["blocked", "denied", "missing", "never", "no", "not"];

/// Compact governed-memory packet used by Rust after validating Python v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DagDbContinuationPacket {
    pub schema_version: String,
    pub packet_id: String,
    pub source_task_id: String,
    pub memory_refs: Vec<String>,
    pub relink_refs: Vec<String>,
    pub continuation_prompt: String,
    pub non_claims: Vec<String>,
}

impl DagDbContinuationPacket {
    /// Parse an existing Python `dagdb_continuation_packet_v1` JSON packet.
    pub fn parse_json(packet_json: &str) -> Result<Self> {
        let packet: PythonContinuationPacketV1 =
            serde_json::from_str(packet_json).map_err(json_error)?;
        packet.validate()?;
        let canonical = packet.into_canonical();
        canonical.validate()?;
        Ok(canonical)
    }

    /// Parse an existing Python `dagdb_continuation_packet_report_v1` JSON report.
    pub fn parse_report_json(report_json: &str) -> Result<Vec<Self>> {
        let report: PythonContinuationReportV1 =
            serde_json::from_str(report_json).map_err(json_error)?;
        report.validate()?;
        report
            .packets
            .into_iter()
            .map(|packet| {
                packet.validate()?;
                let canonical = packet.into_canonical();
                canonical.validate()?;
                Ok(canonical)
            })
            .collect()
    }

    /// Validate the canonical Rust packet without calling runtime or persistence.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != DAGDB_CONTINUATION_PACKET_SCHEMA_VERSION {
            return Err(DagDbContinuationPacketError::SchemaVersion {
                actual: self.schema_version.clone(),
            });
        }
        validate_non_empty("packet_id", &self.packet_id)?;
        validate_non_empty("source_task_id", &self.source_task_id)?;
        validate_non_empty("continuation_prompt", &self.continuation_prompt)?;
        validate_non_empty_list("memory_refs", &self.memory_refs)?;
        validate_non_empty_list("relink_refs", &self.relink_refs)?;
        validate_non_empty_list("non_claims", &self.non_claims)?;
        validate_sorted_unique("memory_refs", &self.memory_refs)?;
        validate_sorted_unique("relink_refs", &self.relink_refs)?;
        validate_sorted_unique("non_claims", &self.non_claims)?;
        validate_required_non_claims(&self.non_claims)?;

        reject_forbidden_string("packet_id", &self.packet_id)?;
        reject_forbidden_string("source_task_id", &self.source_task_id)?;
        reject_forbidden_string("continuation_prompt", &self.continuation_prompt)?;
        reject_raw_source_body("continuation_prompt", &self.continuation_prompt)?;
        reject_approval_overclaim("continuation_prompt", &self.continuation_prompt)?;
        reject_forbidden_list("memory_refs", &self.memory_refs)?;
        reject_forbidden_list("relink_refs", &self.relink_refs)?;
        reject_forbidden_list("non_claims", &self.non_claims)?;
        for (index, non_claim) in self.non_claims.iter().enumerate() {
            let field = format!("non_claims[{index}]");
            reject_raw_source_body(&field, non_claim)?;
            reject_approval_overclaim(&field, non_claim)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonContinuationPacketV1 {
    schema_version: String,
    tenant_id: String,
    namespace: String,
    db_set_version: String,
    task_id: String,
    context_packet_id: String,
    usage_event_id: String,
    stopped_at: String,
    next_steps: String,
    blockers: Vec<String>,
    changed_paths: Vec<String>,
    memory_ref_ids: Vec<String>,
    token_estimate: u32,
    boundary_warnings: Vec<String>,
}

impl PythonContinuationPacketV1 {
    fn validate(&self) -> Result<()> {
        if self.schema_version != DAGDB_CONTINUATION_PACKET_SCHEMA_VERSION {
            return Err(DagDbContinuationPacketError::SchemaVersion {
                actual: self.schema_version.clone(),
            });
        }
        validate_exact_value("tenant_id", &self.tenant_id, DEFAULT_TENANT_ID)?;
        validate_exact_value("namespace", &self.namespace, DEFAULT_NAMESPACE)?;
        validate_exact_value(
            "db_set_version",
            &self.db_set_version,
            DEFAULT_DB_SET_VERSION,
        )?;
        validate_non_empty("task_id", &self.task_id)?;
        validate_non_empty("context_packet_id", &self.context_packet_id)?;
        validate_non_empty("usage_event_id", &self.usage_event_id)?;
        validate_non_empty("stopped_at", &self.stopped_at)?;
        validate_non_empty("next_steps", &self.next_steps)?;
        validate_non_empty_list("blockers", &self.blockers)?;
        validate_non_empty_list("changed_paths", &self.changed_paths)?;
        validate_non_empty_list("memory_ref_ids", &self.memory_ref_ids)?;
        validate_non_empty_list("boundary_warnings", &self.boundary_warnings)?;
        validate_sorted_unique("memory_ref_ids", &self.memory_ref_ids)?;
        validate_repo_relative_paths(&self.changed_paths)?;

        reject_forbidden_python_packet_material(self)?;
        reject_approval_overclaim("stopped_at", &self.stopped_at)?;
        reject_approval_overclaim("next_steps", &self.next_steps)?;
        for (index, blocker) in self.blockers.iter().enumerate() {
            reject_approval_overclaim(&format!("blockers[{index}]"), blocker)?;
        }
        validate_sorted_unique("boundary_warnings", &self.boundary_warnings)?;
        validate_token_estimate(self)?;
        validate_relink_evidence(self)?;
        Ok(())
    }

    fn into_canonical(self) -> DagDbContinuationPacket {
        DagDbContinuationPacket {
            schema_version: self.schema_version,
            packet_id: format!("{}:{}", self.task_id, self.usage_event_id),
            source_task_id: self.task_id,
            memory_refs: self.memory_ref_ids,
            relink_refs: vec![
                "blocked_missing_live_relink:writeback_blocked_fallback_lineage".into(),
            ],
            continuation_prompt: compose_resume_task(
                &self.stopped_at,
                &self.next_steps,
                &self.blockers,
            ),
            non_claims: REQUIRED_BLOCKED_NON_CLAIMS
                .iter()
                .map(|claim| (*claim).to_owned())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonContinuationReportV1 {
    schema_version: String,
    tenant_id: String,
    namespace: String,
    db_set_version: String,
    packet_count: u32,
    valid_count: u32,
    invalid_count: u32,
    failure_codes: Vec<String>,
    invalid_packets: Vec<serde_json::Value>,
    boundary_warnings: Vec<String>,
    source_loop_report_path: String,
    source_promotion_report_path: String,
    packets: Vec<PythonContinuationPacketV1>,
    resume_round_trip: serde_json::Value,
}

impl PythonContinuationReportV1 {
    fn validate(&self) -> Result<()> {
        if self.schema_version != DAGDB_CONTINUATION_PACKET_REPORT_SCHEMA_VERSION {
            return Err(DagDbContinuationPacketError::ReportInvalid {
                reason: "report schema_version mismatch".to_owned(),
            });
        }
        validate_exact_value("tenant_id", &self.tenant_id, DEFAULT_TENANT_ID)?;
        validate_exact_value("namespace", &self.namespace, DEFAULT_NAMESPACE)?;
        validate_exact_value(
            "db_set_version",
            &self.db_set_version,
            DEFAULT_DB_SET_VERSION,
        )?;
        validate_non_empty("source_loop_report_path", &self.source_loop_report_path)?;
        reject_forbidden_string("source_loop_report_path", &self.source_loop_report_path)?;
        validate_non_empty(
            "source_promotion_report_path",
            &self.source_promotion_report_path,
        )?;
        reject_forbidden_string(
            "source_promotion_report_path",
            &self.source_promotion_report_path,
        )?;
        validate_non_empty_list("boundary_warnings", &self.boundary_warnings)?;
        validate_sorted_unique("boundary_warnings", &self.boundary_warnings)?;
        reject_forbidden_list("boundary_warnings", &self.boundary_warnings)?;

        if usize::try_from(self.packet_count).ok() != Some(self.packets.len()) {
            return Err(DagDbContinuationPacketError::ReportInvalid {
                reason: "packet_count mismatch".to_owned(),
            });
        }
        if self.valid_count != self.packet_count || self.invalid_count != 0 {
            return Err(DagDbContinuationPacketError::ReportInvalid {
                reason: "report contains invalid packets".to_owned(),
            });
        }
        if !self.failure_codes.is_empty() || !self.invalid_packets.is_empty() {
            return Err(DagDbContinuationPacketError::ReportInvalid {
                reason: "report contains failure material".to_owned(),
            });
        }
        if !self.resume_round_trip.is_object() {
            return Err(DagDbContinuationPacketError::ReportInvalid {
                reason: "resume_round_trip must be present".to_owned(),
            });
        }
        reject_forbidden_json("resume_round_trip", &self.resume_round_trip)?;
        Ok(())
    }
}

/// Errors raised by continuation packet validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DagDbContinuationPacketError {
    /// JSON could not be parsed into the strict Python v1 compatibility shape.
    #[error("dagdb_continuation_packet_json_failed: {reason}")]
    Json {
        /// Stable parse reason.
        reason: String,
    },
    /// Schema version does not match the Rust-owned compatibility contract.
    #[error("dagdb_continuation_packet_schema_version_mismatch: {actual}")]
    SchemaVersion {
        /// Supplied schema version.
        actual: String,
    },
    /// Required field was empty.
    #[error("dagdb_continuation_packet_empty_field: {field}")]
    EmptyField {
        /// Field name.
        field: String,
    },
    /// Required list was empty.
    #[error("dagdb_continuation_packet_empty_list: {field}")]
    EmptyList {
        /// Field name.
        field: String,
    },
    /// A deterministic list was not sorted or had duplicates.
    #[error("dagdb_continuation_packet_list_not_sorted_unique: {field}")]
    ListNotSortedUnique {
        /// Field name.
        field: String,
    },
    /// A field does not match the expected compatibility value.
    #[error("dagdb_continuation_packet_invalid_value: {field}: {reason}")]
    InvalidValue {
        /// Field name.
        field: String,
        /// Stable reason.
        reason: String,
    },
    /// Relink evidence is missing without an explicit blocked status.
    #[error("dagdb_continuation_packet_relink_evidence_missing")]
    RelinkEvidenceMissing,
    /// A required blocked non-claim is missing from the canonical packet.
    #[error("dagdb_continuation_packet_missing_blocked_non_claim: {claim}")]
    MissingBlockedNonClaim {
        /// Missing blocked non-claim.
        claim: String,
    },
    /// A field contains raw body, path, secret, or transcript material.
    #[error("dagdb_continuation_packet_forbidden_material: {field}: {reason}")]
    ForbiddenMaterial {
        /// Field name.
        field: String,
        /// Stable reason.
        reason: String,
    },
    /// A field claims blocked runtime/default-memory/route/final-thesis approval.
    #[error("dagdb_continuation_packet_approval_overclaim: {field}: {fragment}")]
    ApprovalOverclaim {
        /// Field name.
        field: String,
        /// Forbidden overclaim fragment.
        fragment: String,
    },
    /// Report-level validation failed.
    #[error("dagdb_continuation_packet_report_invalid: {reason}")]
    ReportInvalid {
        /// Stable report validation reason.
        reason: String,
    },
}

/// Result alias for continuation packet validation.
pub type Result<T> = std::result::Result<T, DagDbContinuationPacketError>;

fn json_error(error: serde_json::Error) -> DagDbContinuationPacketError {
    DagDbContinuationPacketError::Json {
        reason: error.to_string(),
    }
}

fn compose_resume_task(stopped_at: &str, next_steps: &str, blockers: &[String]) -> String {
    format!(
        "Resume DAG DB work. Stopped at: {stopped_at} Next steps: {next_steps} Blockers: {}",
        blockers.join("; ")
    )
}

fn validate_exact_value(field: &str, actual: &str, expected: &str) -> Result<()> {
    if actual != expected {
        return Err(DagDbContinuationPacketError::InvalidValue {
            field: field.to_owned(),
            reason: format!("expected {expected}"),
        });
    }
    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(DagDbContinuationPacketError::EmptyField {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn validate_non_empty_list(field: &str, values: &[String]) -> Result<()> {
    if values.is_empty() {
        return Err(DagDbContinuationPacketError::EmptyList {
            field: field.to_owned(),
        });
    }
    for value in values {
        validate_non_empty(field, value)?;
    }
    Ok(())
}

fn validate_sorted_unique(field: &str, values: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(DagDbContinuationPacketError::ListNotSortedUnique {
                field: field.to_owned(),
            });
        }
    }
    if values != seen.iter().copied().cloned().collect::<Vec<_>>() {
        return Err(DagDbContinuationPacketError::ListNotSortedUnique {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn validate_repo_relative_paths(paths: &[String]) -> Result<()> {
    for (index, path) in paths.iter().enumerate() {
        if path.starts_with('/') || path.starts_with('~') || path.contains('\\') {
            return Err(DagDbContinuationPacketError::InvalidValue {
                field: format!("changed_paths[{index}]"),
                reason: "changed path must be repo-relative".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_token_estimate(packet: &PythonContinuationPacketV1) -> Result<()> {
    let computed = token_estimate_from_text(&compact_token_material(packet));
    if packet.token_estimate != computed {
        return Err(DagDbContinuationPacketError::InvalidValue {
            field: "token_estimate".to_owned(),
            reason: "must match compact material estimate".to_owned(),
        });
    }
    Ok(())
}

fn validate_relink_evidence(packet: &PythonContinuationPacketV1) -> Result<()> {
    let has_writeback_blocked = packet
        .boundary_warnings
        .iter()
        .any(|warning| warning == "writeback_blocked");
    let blocker_text = packet.blockers.join(" ").to_ascii_lowercase();
    let has_blocked_lineage =
        blocker_text.contains("fallback lineage") || blocker_text.contains("relink");
    if has_writeback_blocked && has_blocked_lineage {
        return Ok(());
    }
    Err(DagDbContinuationPacketError::RelinkEvidenceMissing)
}

fn validate_required_non_claims(non_claims: &[String]) -> Result<()> {
    for claim in REQUIRED_BLOCKED_NON_CLAIMS {
        if !non_claims.iter().any(|actual| actual == claim) {
            return Err(DagDbContinuationPacketError::MissingBlockedNonClaim {
                claim: (*claim).to_owned(),
            });
        }
    }
    Ok(())
}

fn compact_token_material(packet: &PythonContinuationPacketV1) -> String {
    let blockers = packet.blockers.join(" ");
    let changed_paths = packet.changed_paths.join(" ");
    let parts = [
        packet.stopped_at.as_str(),
        packet.next_steps.as_str(),
        blockers.as_str(),
        changed_paths.as_str(),
    ];
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn token_estimate_from_text(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    u32::try_from(text.len().div_ceil(4)).unwrap_or(u32::MAX)
}

fn reject_forbidden_python_packet_material(packet: &PythonContinuationPacketV1) -> Result<()> {
    reject_forbidden_string("stopped_at", &packet.stopped_at)?;
    reject_raw_source_body("stopped_at", &packet.stopped_at)?;
    reject_forbidden_string("next_steps", &packet.next_steps)?;
    reject_raw_source_body("next_steps", &packet.next_steps)?;
    reject_forbidden_list("blockers", &packet.blockers)?;
    for (index, blocker) in packet.blockers.iter().enumerate() {
        reject_raw_source_body(&format!("blockers[{index}]"), blocker)?;
    }
    reject_forbidden_list("changed_paths", &packet.changed_paths)?;
    reject_forbidden_list("boundary_warnings", &packet.boundary_warnings)?;
    reject_forbidden_string("usage_event_id", &packet.usage_event_id)?;
    reject_raw_source_body("usage_event_id", &packet.usage_event_id)?;
    reject_forbidden_string("context_packet_id", &packet.context_packet_id)?;
    reject_raw_source_body("context_packet_id", &packet.context_packet_id)?;
    reject_forbidden_list("memory_ref_ids", &packet.memory_ref_ids)?;
    Ok(())
}

fn reject_forbidden_list(field: &str, values: &[String]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        reject_forbidden_string(&format!("{field}[{index}]"), value)?;
    }
    Ok(())
}

/// Recursively scan an arbitrary JSON value, applying `reject_forbidden_string`
/// to every object key and every string value (descending into nested
/// objects and arrays). Used to ensure free-form fields like
/// `resume_round_trip` cannot smuggle forbidden material in nested keys/values.
fn reject_forbidden_json(field: &str, value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::String(text) => reject_forbidden_string(field, text),
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                reject_forbidden_json(&format!("{field}[{index}]"), item)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                reject_forbidden_string(&format!("{field}.<key>"), key)?;
                reject_forbidden_json(&format!("{field}.{key}"), child)?;
            }
            Ok(())
        }
        // Numbers, booleans, and null carry no string material to scan.
        _ => Ok(()),
    }
}

fn reject_forbidden_string(field: &str, value: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| lowered.contains(**fragment))
    {
        return Err(DagDbContinuationPacketError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: format!("contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

fn reject_raw_source_body(field: &str, value: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if let Some(marker) = RAW_SOURCE_BODY_MARKERS
        .iter()
        .find(|marker| lowered.contains(**marker))
    {
        return Err(DagDbContinuationPacketError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: format!("contains raw source marker {marker}"),
        });
    }
    if value.len() > 4_000 || value.lines().count() > 12 {
        return Err(DagDbContinuationPacketError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: "looks like raw source body".to_owned(),
        });
    }
    Ok(())
}

fn reject_approval_overclaim(field: &str, value: &str) -> Result<()> {
    let tokens = normalized_tokens(value);
    let Some(surface) = BLOCKED_APPROVAL_SURFACES
        .iter()
        .find(|surface| contains_token_sequence(&tokens, surface))
    else {
        return Ok(());
    };

    for (index, token) in tokens.iter().enumerate() {
        if BLOCKED_APPROVAL_TERMS.contains(&token.as_str())
            && !approval_term_is_negated(&tokens, index)
        {
            return Err(DagDbContinuationPacketError::ApprovalOverclaim {
                field: field.to_owned(),
                fragment: format!("{} {}", surface.join(" "), token),
            });
        }
    }
    Ok(())
}

fn normalized_tokens(value: &str) -> Vec<String> {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn contains_token_sequence(tokens: &[String], sequence: &[&str]) -> bool {
    // pragma-allowlist-secret (NLP token-parsing, not a credential)
    tokens.windows(sequence.len()).any(|window| {
        window
            .iter()
            .map(String::as_str)
            .eq(sequence.iter().copied())
    })
}

fn approval_term_is_negated(tokens: &[String], term_index: usize) -> bool {
    // pragma-allowlist-secret (NLP token-parsing, not a credential)
    approval_term_has_prior_negation(tokens, term_index)
        || approval_term_has_following_blocker(tokens, term_index)
        || approval_term_has_following_negated_approval(tokens, term_index)
}

fn approval_term_has_prior_negation(tokens: &[String], term_index: usize) -> bool {
    // pragma-allowlist-secret (NLP token-parsing, not a credential)
    let window_start = term_index.saturating_sub(3);
    tokens[window_start..term_index]
        .iter()
        .any(|token| BLOCKED_APPROVAL_NEGATIONS.contains(&token.as_str()))
}

fn approval_term_has_following_blocker(tokens: &[String], term_index: usize) -> bool {
    // pragma-allowlist-secret (NLP token-parsing, not a credential)
    let Some(index) = next_meaningful_token_index(tokens, term_index + 1) else {
        return false;
    };
    matches!(
        tokens[index].as_str(),
        "blocked" | "denied" | "missing" | "never"
    )
}

fn approval_term_has_following_negated_approval(tokens: &[String], term_index: usize) -> bool {
    // pragma-allowlist-secret (NLP token-parsing, not a credential)
    let Some(negation_index) = next_meaningful_token_index(tokens, term_index + 1) else {
        return false;
    };
    if !matches!(tokens[negation_index].as_str(), "no" | "not" | "never") {
        return false;
    }

    let Some(approval_index) = next_meaningful_token_index(tokens, negation_index + 1) else {
        return false;
    };
    BLOCKED_APPROVAL_TERMS.contains(&tokens[approval_index].as_str())
}

fn next_meaningful_token_index(tokens: &[String], start: usize) -> Option<usize> {
    // pragma-allowlist-secret (NLP token-parsing, not a credential)
    tokens
        .iter()
        .enumerate()
        .skip(start)
        .take(4)
        .find(|(_, token)| !is_approval_connector(token))
        .map(|(index, _)| index)
}

fn is_approval_connector(token: &str) -> bool {
    matches!(
        token,
        "after"
            | "as"
            | "be"
            | "being"
            | "for"
            | "is"
            | "remain"
            | "remains"
            | "still"
            | "the"
            | "was"
    )
}

#[cfg(test)]
mod tests {
    use super::{approval_term_is_negated, normalized_tokens, token_estimate_from_text};

    #[test]
    fn token_estimate_handles_empty_and_non_empty_text() {
        assert_eq!(token_estimate_from_text(""), 0);
        assert_eq!(token_estimate_from_text("abcde"), 2);
    }

    #[test]
    fn approval_negation_helper_ignores_the_approval_term_itself() {
        let tokens = normalized_tokens("not approved"); // pragma-allowlist-secret (NLP token-parsing, not a credential)
        assert!(approval_term_is_negated(&tokens, 1));

        let tokens = normalized_tokens("approved"); // pragma-allowlist-secret (NLP token-parsing, not a credential)
        assert!(!approval_term_is_negated(&tokens, 0));

        let tokens = normalized_tokens("approved not blocked"); // pragma-allowlist-secret (NLP token-parsing, not a credential)
        assert!(!approval_term_is_negated(&tokens, 0));

        let tokens = normalized_tokens("approved without restrictions"); // pragma-allowlist-secret (NLP token-parsing, not a credential)
        assert!(!approval_term_is_negated(&tokens, 0));

        let tokens = normalized_tokens("approval remains blocked"); // pragma-allowlist-secret (NLP token-parsing fixture, not a credential)
        assert!(approval_term_is_negated(&tokens, 0));

        let tokens = normalized_tokens("acceptance not approved"); // pragma-allowlist-secret (NLP token-parsing fixture, not a credential)
        assert!(approval_term_is_negated(&tokens, 0));
    }
}
