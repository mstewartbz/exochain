//! PRD17C continuation persistence contracts.
//!
//! Continuation records are persisted as compact refs and summaries. Expired,
//! cross-project, raw-body, and unsafe replay attempts fail closed.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::lifecycle_action::{LifecycleMemoryRef, ProductionLifecycleApproval};

/// Schema for PRD17C continuation records.
pub const PRD17_CONTINUATION_RECORD_SCHEMA: &str = "dagdb_prd17_continuation_record_v1";
/// Schema for PRD17C continuation persistence reports.
pub const PRD17_CONTINUATION_PERSISTENCE_REPORT_SCHEMA: &str =
    "dagdb_prd17_continuation_persistence_report_v1";

const RAW_BODY_KEYS: &[&str] = &[
    "body",
    "content",
    "file_text",
    "full_output",
    "markdown",
    "model_output",
    "payload",
    "prompt_body",
    "raw_body",
    "raw_markdown",
    "raw_model_output",
    "raw_payload",
    "raw_private_payload",
    "raw_prompt_body",
    "source_excerpt",
    "source_text",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "authorization",
    "database_url",
    ".env",
    "postgres://",
    "postgresql://",
    "password",
    "raw_body",
    "raw_markdown",
    "secret",
    "sk-proj-",
    "source_excerpt",
];

/// Later retrieval status for a continuation record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationRetrievalStatus {
    Pending,
    Retrieved,
    ExpiredRejected,
}

/// Persisted continuation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationRecord {
    pub schema_version: String,
    pub continuation_id: String,
    pub task_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub memory_namespace: String,
    pub summary_ref: String,
    pub memory_refs: Vec<LifecycleMemoryRef>,
    pub blocker_refs: Vec<String>,
    pub validation_refs: Vec<String>,
    pub expiry_epoch_seconds: u64,
    pub later_retrieval_status: ContinuationRetrievalStatus,
    pub production_lifecycle_approval: ProductionLifecycleApproval,
    pub created_at: String,
}

impl ContinuationRecord {
    /// Parse continuation JSON after rejecting raw/private material.
    pub fn parse_json(record_json: &str) -> Result<Self> {
        let raw: JsonValue = serde_json::from_str(record_json).map_err(|error| {
            ContinuationPersistenceError::Json {
                reason: error.to_string(),
            }
        })?;
        reject_forbidden_json(&raw, "$")?;
        let record: Self =
            serde_json::from_value(raw).map_err(|error| ContinuationPersistenceError::Json {
                reason: error.to_string(),
            })?;
        record.validate(0)?;
        Ok(record)
    }

    /// Validate continuation invariants against a current epoch timestamp.
    pub fn validate(&self, now_epoch_seconds: u64) -> Result<()> {
        if self.schema_version != PRD17_CONTINUATION_RECORD_SCHEMA {
            return Err(ContinuationPersistenceError::InvalidRecord {
                reason: "unsupported continuation schema_version".to_owned(),
            });
        }
        validate_non_empty("continuation_id", &self.continuation_id)?;
        validate_scope_field("task_id", &self.task_id)?;
        validate_scope_field("tenant_id", &self.tenant_id)?;
        validate_scope_field("project_id", &self.project_id)?;
        validate_scope_field("memory_namespace", &self.memory_namespace)?;
        validate_non_empty("summary_ref", &self.summary_ref)?;
        validate_non_empty("created_at", &self.created_at)?;
        validate_memory_refs_sorted_unique(
            "memory_refs",
            &self.memory_refs,
            &self.tenant_id,
            &self.project_id,
            &self.memory_namespace,
        )?;
        if self.memory_refs.is_empty() {
            return Err(ContinuationPersistenceError::InvalidRecord {
                reason: "memory_refs must not be empty".to_owned(),
            });
        }
        validate_sorted_unique_strings("blocker_refs", &self.blocker_refs)?;
        validate_sorted_unique_strings("validation_refs", &self.validation_refs)?;
        if self.blocker_refs.is_empty() {
            return Err(ContinuationPersistenceError::InvalidRecord {
                reason: "blocker_refs must not be empty".to_owned(),
            });
        }
        if self.validation_refs.is_empty() {
            return Err(ContinuationPersistenceError::InvalidRecord {
                reason: "validation_refs must not be empty".to_owned(),
            });
        }
        if now_epoch_seconds > 0 && self.expiry_epoch_seconds <= now_epoch_seconds {
            return Err(ContinuationPersistenceError::ExpiredContinuation {
                continuation_id: self.continuation_id.clone(),
            });
        }
        Ok(())
    }

    /// Deterministic continuation idempotency key.
    pub fn idempotency_key(&self) -> Result<String> {
        self.validate(0)?;
        let memory_material = self
            .memory_refs
            .iter()
            .map(|reference| reference.memory_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let memory_hash = sha256_hex(memory_material.as_bytes());
        Ok(format!(
            "{}:{}:{}:{}:{}:{}",
            self.tenant_id,
            self.project_id,
            self.memory_namespace,
            self.task_id,
            self.summary_ref,
            memory_hash
        ))
    }
}

/// Result of persisting a continuation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationPersistResult {
    pub continuation_id: String,
    pub idempotency_key: String,
    pub replayed: bool,
    pub later_retrieval_status: ContinuationRetrievalStatus,
}

/// In-memory continuation store for deterministic contract tests.
#[derive(Debug, Default)]
pub struct ContinuationStore {
    records_by_id: BTreeMap<String, ContinuationRecord>,
    idempotency_keys: BTreeMap<String, String>,
}

impl ContinuationStore {
    /// Persist a continuation record, replaying exact idempotent duplicates.
    pub fn persist_continuation(
        &mut self,
        record: ContinuationRecord,
        now_epoch_seconds: u64,
    ) -> Result<ContinuationPersistResult> {
        record.validate(now_epoch_seconds)?;
        let idempotency_key = record.idempotency_key()?;
        if let Some(existing_id) = self.idempotency_keys.get(&idempotency_key) {
            let Some(existing) = self.records_by_id.get(existing_id) else {
                return Err(ContinuationPersistenceError::InvalidRecord {
                    reason: "idempotency key points at missing continuation".to_owned(),
                });
            };
            if existing == &record {
                return Ok(ContinuationPersistResult {
                    continuation_id: existing_id.clone(),
                    idempotency_key,
                    replayed: true,
                    later_retrieval_status: existing.later_retrieval_status,
                });
            }
            return Err(ContinuationPersistenceError::DuplicateUnsafeReplay { idempotency_key });
        }
        if self.records_by_id.contains_key(&record.continuation_id) {
            return Err(ContinuationPersistenceError::DuplicateUnsafeReplay { idempotency_key });
        }
        let result = ContinuationPersistResult {
            continuation_id: record.continuation_id.clone(),
            idempotency_key: idempotency_key.clone(),
            replayed: false,
            later_retrieval_status: record.later_retrieval_status,
        };
        self.idempotency_keys
            .insert(idempotency_key, record.continuation_id.clone());
        self.records_by_id
            .insert(record.continuation_id.clone(), record);
        Ok(result)
    }

    /// Retrieve a scoped unexpired continuation and mark it consumed.
    pub fn retrieve_for_task(
        &mut self,
        task_id: &str,
        tenant_id: &str,
        project_id: &str,
        namespace: &str,
        now_epoch_seconds: u64,
    ) -> Result<ContinuationRecord> {
        validate_non_empty("task_id", task_id)?;
        validate_non_empty("tenant_id", tenant_id)?;
        validate_non_empty("project_id", project_id)?;
        validate_non_empty("memory_namespace", namespace)?;
        let continuation_id = self
            .records_by_id
            .iter()
            .find(|(_id, record)| {
                record.task_id == task_id
                    && record.tenant_id == tenant_id
                    && record.project_id == project_id
                    && record.memory_namespace == namespace
            })
            .map(|(id, _record)| id.clone())
            .ok_or_else(|| ContinuationPersistenceError::ContinuationNotFound {
                task_id: task_id.to_owned(),
            })?;
        let record = self
            .records_by_id
            .get_mut(&continuation_id)
            .ok_or_else(|| ContinuationPersistenceError::ContinuationNotFound {
                task_id: task_id.to_owned(),
            })?;
        record.validate(now_epoch_seconds)?;
        record.later_retrieval_status = ContinuationRetrievalStatus::Retrieved;
        Ok(record.clone())
    }

    /// Number of durable continuation records.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records_by_id.len()
    }
}

/// Errors raised by PRD17C continuation persistence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContinuationPersistenceError {
    #[error("dagdb_prd17_continuation_json_invalid: {reason}")]
    Json { reason: String },
    #[error("dagdb_prd17_continuation_invalid: {reason}")]
    InvalidRecord { reason: String },
    #[error("dagdb_prd17_continuation_empty_field: {field}")]
    EmptyField { field: String },
    #[error("dagdb_prd17_continuation_list_not_sorted_unique: {field}")]
    ListNotSortedUnique { field: String },
    #[error("dagdb_prd17_continuation_scope_mismatch: {field}")]
    ScopeMismatch { field: String },
    #[error("dagdb_prd17_continuation_forbidden_material: {field}: {reason}")]
    ForbiddenMaterial { field: String, reason: String },
    #[error("dagdb_prd17_continuation_expired: {continuation_id}")]
    ExpiredContinuation { continuation_id: String },
    #[error("dagdb_prd17_continuation_not_found: {task_id}")]
    ContinuationNotFound { task_id: String },
    #[error("dagdb_prd17_continuation_duplicate_unsafe_replay: {idempotency_key}")]
    DuplicateUnsafeReplay { idempotency_key: String },
}

/// Result alias for continuation persistence.
pub type Result<T> = std::result::Result<T, ContinuationPersistenceError>;

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ContinuationPersistenceError::EmptyField {
            field: field.to_owned(),
        });
    }
    reject_forbidden_string(field, value)
}

/// Scope fields (including `task_id`) feed the colon-joined idempotency key,
/// so a ':' inside them would make distinct scopes collide on the same key
/// (cross-scope replay denial). They must stay colon-free.
fn validate_scope_field(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if value.contains(':') {
        return Err(ContinuationPersistenceError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: "scope fields must not contain ':' (idempotency key delimiter)".to_owned(),
        });
    }
    Ok(())
}

fn validate_memory_refs_sorted_unique(
    field: &str,
    refs: &[LifecycleMemoryRef],
    tenant_id: &str,
    project_id: &str,
    namespace: &str,
) -> Result<()> {
    let mut memory_ids = Vec::new();
    for (index, reference) in refs.iter().enumerate() {
        validate_non_empty(&format!("{field}[{index}].tenant_id"), &reference.tenant_id)?;
        validate_non_empty(
            &format!("{field}[{index}].project_id"),
            &reference.project_id,
        )?;
        validate_non_empty(
            &format!("{field}[{index}].memory_namespace"),
            &reference.memory_namespace,
        )?;
        validate_non_empty(&format!("{field}[{index}].memory_id"), &reference.memory_id)?;
        if reference.tenant_id != tenant_id
            || reference.project_id != project_id
            || reference.memory_namespace != namespace
        {
            return Err(ContinuationPersistenceError::ScopeMismatch {
                field: format!("{field}[{index}]"),
            });
        }
        memory_ids.push(reference.memory_id.clone());
    }
    validate_sorted_unique_strings(field, &memory_ids)
}

fn validate_sorted_unique_strings(field: &str, values: &[String]) -> Result<()> {
    for value in values {
        validate_non_empty(field, value)?;
    }
    let sorted = values.iter().cloned().collect::<BTreeSet<_>>();
    if sorted.len() != values.len() || values != sorted.into_iter().collect::<Vec<_>>() {
        return Err(ContinuationPersistenceError::ListNotSortedUnique {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn reject_forbidden_json(value: &JsonValue, field: &str) -> Result<()> {
    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                let lowered = key.to_ascii_lowercase();
                if RAW_BODY_KEYS.iter().any(|raw_key| lowered == *raw_key) {
                    return Err(ContinuationPersistenceError::ForbiddenMaterial {
                        field: format!("{field}.{key}"),
                        reason: "raw body field is not allowed".to_owned(),
                    });
                }
                reject_forbidden_json(child, &format!("{field}.{key}"))?;
            }
        }
        JsonValue::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                reject_forbidden_json(child, &format!("{field}[{index}]"))?;
            }
        }
        JsonValue::String(text) => reject_forbidden_string(field, text)?,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
    Ok(())
}

fn reject_forbidden_string(field: &str, value: &str) -> Result<()> {
    let normalized = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| normalized.contains(**fragment))
    {
        return Err(ContinuationPersistenceError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: format!("contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
