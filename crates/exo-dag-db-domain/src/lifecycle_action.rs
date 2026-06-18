//! PRD17C lifecycle mutation contracts.
//!
//! This module validates mutation-backed writeback lifecycle records without
//! deleting source evidence or claiming production lifecycle approval.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Schema for PRD17C lifecycle action records.
pub const PRD17_LIFECYCLE_ACTION_SCHEMA: &str = "dagdb_prd17_lifecycle_action_v1";
/// Schema for PRD17C lifecycle mutation reports.
pub const PRD17_LIFECYCLE_MUTATION_REPORT_SCHEMA: &str = "dagdb_prd17_lifecycle_mutation_report_v1";

const RAW_BODY_KEYS: &[&str] = &[
    "body",
    "content",
    "document_body",
    "file_text",
    "full_output",
    "markdown",
    "model_output",
    "payload",
    "private_payload",
    "prompt_body",
    "raw_body",
    "raw_document_body",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_payload",
    "raw_private_payload",
    "raw_prompt_body",
    "source_body",
    "source_excerpt",
    "source_text",
    "text_body",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "authorization",
    "bearer ",
    "begin private key",
    "database_url",
    "db_url",
    ".env",
    "mongodb://",
    "mysql://",
    "password",
    "postgres://",
    "postgresql://",
    "private key-----",
    "raw_body",
    "raw_document_body",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_private_payload",
    "raw_prompt_body",
    "redis://",
    "secret",
    "sk-proj-",
    "sqlite://",
    "source_excerpt",
];

/// Lifecycle mutation action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleActionType {
    Writeback,
    Relink,
    Supersede,
    Recycle,
    Archive,
    Restore,
    RouteInvalidate,
}

impl LifecycleActionType {
    /// Stable wire value used in idempotency material.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Writeback => "writeback",
            Self::Relink => "relink",
            Self::Supersede => "supersede",
            Self::Recycle => "recycle",
            Self::Archive => "archive",
            Self::Restore => "restore",
            Self::RouteInvalidate => "route_invalidate",
        }
    }

    /// Expected inverse action for rollback references.
    #[must_use]
    pub const fn inverse(self) -> Self {
        match self {
            Self::Writeback => Self::Archive,
            Self::Relink => Self::Relink,
            Self::Supersede => Self::Restore,
            Self::Recycle => Self::Restore,
            Self::Archive => Self::Restore,
            Self::Restore => Self::Archive,
            Self::RouteInvalidate => Self::RouteInvalidate,
        }
    }
}

/// Lifecycle terminal state locked by PRD17C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleTerminalState {
    Accepted,
    HonestBlocked,
    OperatorDeferred,
    FailedValidation,
}

/// Production approval state. Missing production approval must stay deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionLifecycleApproval {
    Approved,
    OperatorDeferred,
}

/// Scoped memory reference used to reject cross-tenant/project lifecycle input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleMemoryRef {
    pub tenant_id: String,
    pub project_id: String,
    pub memory_namespace: String,
    pub memory_id: String,
}

impl LifecycleMemoryRef {
    fn validate_scope(&self, action: &LifecycleAction, field: &str) -> Result<()> {
        validate_non_empty(&format!("{field}.tenant_id"), &self.tenant_id)?;
        validate_non_empty(&format!("{field}.project_id"), &self.project_id)?;
        validate_non_empty(&format!("{field}.memory_namespace"), &self.memory_namespace)?;
        validate_non_empty(&format!("{field}.memory_id"), &self.memory_id)?;
        if self.tenant_id != action.tenant_id
            || self.project_id != action.project_id
            || self.memory_namespace != action.memory_namespace
        {
            return Err(LifecycleActionError::ScopeMismatch {
                field: field.to_owned(),
            });
        }
        Ok(())
    }
}

/// Evidence reference that preserves raw evidence by digest/ref, not by body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEvidenceRef {
    pub evidence_id: String,
    pub receipt_id: String,
    pub digest: String,
    pub summary_ref: String,
    pub preserved: bool,
}

impl LifecycleEvidenceRef {
    fn validate(&self, field: &str) -> Result<()> {
        validate_non_empty(&format!("{field}.evidence_id"), &self.evidence_id)?;
        validate_non_empty(&format!("{field}.receipt_id"), &self.receipt_id)?;
        validate_digest(&format!("{field}.digest"), &self.digest)?;
        validate_non_empty(&format!("{field}.summary_ref"), &self.summary_ref)?;
        Ok(())
    }
}

/// Rollback reference required for every lifecycle mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleRollbackRef {
    pub rollback_id: String,
    pub action_id: String,
    pub inverse_action_type: LifecycleActionType,
    pub before_refs: Vec<LifecycleMemoryRef>,
    pub after_refs: Vec<LifecycleMemoryRef>,
    pub validation_ref: String,
    pub operator_required: bool,
}

impl LifecycleRollbackRef {
    fn validate(&self, action: &LifecycleAction) -> Result<()> {
        validate_non_empty("rollback_ref.rollback_id", &self.rollback_id)?;
        if self.action_id != action.action_id {
            return Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref.action_id must match action_id".to_owned(),
            });
        }
        if self.inverse_action_type != action.action_type.inverse() {
            return Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref.inverse_action_type mismatch".to_owned(),
            });
        }
        if self.validation_ref != action.validation_report_id {
            return Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref.validation_ref must match validation_report_id".to_owned(),
            });
        }
        validate_memory_refs_sorted_unique("rollback_ref.before_refs", &self.before_refs, action)?;
        validate_memory_refs_sorted_unique("rollback_ref.after_refs", &self.after_refs, action)?;
        if self.before_refs.is_empty() && self.after_refs.is_empty() {
            return Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref requires before_refs or after_refs".to_owned(),
            });
        }
        Ok(())
    }
}

/// Complete lifecycle action DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleAction {
    pub schema_version: String,
    pub action_id: String,
    pub action_type: LifecycleActionType,
    pub tenant_id: String,
    pub project_id: String,
    pub memory_namespace: String,
    pub actor_id: String,
    pub source_packet_id: String,
    pub source_receipt_id: String,
    pub parent_memory_ids: Vec<LifecycleMemoryRef>,
    pub target_memory_ids: Vec<LifecycleMemoryRef>,
    pub validation_report_id: String,
    pub policy_ref: String,
    pub rollback_ref: LifecycleRollbackRef,
    pub route_invalidation_event_ids: Vec<String>,
    pub evidence_refs: Vec<LifecycleEvidenceRef>,
    pub terminal_state: LifecycleTerminalState,
    pub production_lifecycle_approval: ProductionLifecycleApproval,
    pub created_at: String,
}

impl LifecycleAction {
    /// Parse a lifecycle action from JSON after rejecting raw/private material.
    pub fn parse_json(action_json: &str) -> Result<Self> {
        let raw: JsonValue =
            serde_json::from_str(action_json).map_err(|error| LifecycleActionError::Json {
                reason: error.to_string(),
            })?;
        reject_forbidden_json(&raw, "$")?;
        let action: Self =
            serde_json::from_value(raw).map_err(|error| LifecycleActionError::Json {
                reason: error.to_string(),
            })?;
        action.validate()?;
        Ok(action)
    }

    /// Validate lifecycle invariants independent of persistence.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != PRD17_LIFECYCLE_ACTION_SCHEMA {
            return Err(LifecycleActionError::InvalidAction {
                reason: "unsupported lifecycle action schema_version".to_owned(),
            });
        }
        validate_non_empty("action_id", &self.action_id)?;
        validate_scope_field("tenant_id", &self.tenant_id)?;
        validate_scope_field("project_id", &self.project_id)?;
        validate_scope_field("memory_namespace", &self.memory_namespace)?;
        validate_non_empty("actor_id", &self.actor_id)?;
        validate_non_empty("source_packet_id", &self.source_packet_id)?;
        validate_non_empty("source_receipt_id", &self.source_receipt_id)?;
        validate_non_empty("validation_report_id", &self.validation_report_id)?;
        validate_non_empty("policy_ref", &self.policy_ref)?;
        validate_non_empty("created_at", &self.created_at)?;

        validate_memory_refs_sorted_unique("parent_memory_ids", &self.parent_memory_ids, self)?;
        validate_memory_refs_sorted_unique("target_memory_ids", &self.target_memory_ids, self)?;
        if self.parent_memory_ids.is_empty() {
            return Err(LifecycleActionError::InvalidAction {
                reason: "parent_memory_ids must not be empty".to_owned(),
            });
        }
        if self.target_memory_ids.is_empty() {
            return Err(LifecycleActionError::InvalidAction {
                reason: "target_memory_ids must not be empty".to_owned(),
            });
        }

        validate_sorted_unique_strings(
            "route_invalidation_event_ids",
            &self.route_invalidation_event_ids,
        )?;
        if self.route_invalidation_event_ids.is_empty() {
            return Err(LifecycleActionError::RouteInvalidationGap {
                action_id: self.action_id.clone(),
            });
        }

        if self.evidence_refs.is_empty() {
            return Err(LifecycleActionError::InvalidAction {
                reason: "evidence_refs must not be empty".to_owned(),
            });
        }
        let mut evidence_ids = BTreeSet::new();
        for (index, evidence) in self.evidence_refs.iter().enumerate() {
            evidence.validate(&format!("evidence_refs[{index}]"))?;
            if !evidence_ids.insert(evidence.evidence_id.as_str()) {
                return Err(LifecycleActionError::InvalidAction {
                    reason: "evidence_refs must be unique".to_owned(),
                });
            }
        }
        if matches!(
            self.action_type,
            LifecycleActionType::Recycle | LifecycleActionType::Archive
        ) && self
            .evidence_refs
            .iter()
            .any(|evidence| !evidence.preserved)
        {
            return Err(LifecycleActionError::EvidenceWouldBeDeleted {
                action_id: self.action_id.clone(),
            });
        }

        self.rollback_ref.validate(self)?;

        // An `Accepted` terminal state asserts production approval, but this
        // local repository/test layer carries no operator-authority binding
        // (signature/session/receipt) over the action body — both approval
        // values are self-asserted untrusted JSON. Reject `Accepted` for both
        // so a deserialized `Approved` cannot mint a production-accepted action
        // without verified operator authority that this layer cannot provide.
        if self.terminal_state == LifecycleTerminalState::Accepted {
            return Err(LifecycleActionError::ProductionApprovalMissing {
                action_id: self.action_id.clone(),
            });
        }
        Ok(())
    }

    /// Deterministic PRD17C idempotency key.
    pub fn idempotency_key(&self) -> Result<String> {
        self.validate()?;
        let target_material = self
            .target_memory_ids
            .iter()
            .map(|reference| reference.memory_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let target_hash = sha256_hex(target_material.as_bytes());
        Ok(format!(
            "{}:{}:{}:{}:{}:{}",
            self.tenant_id,
            self.project_id,
            self.memory_namespace,
            self.action_type.as_str(),
            self.source_receipt_id,
            target_hash
        ))
    }
}

/// Result of applying a lifecycle action to a lifecycle ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleApplyResult {
    pub action_id: String,
    pub idempotency_key: String,
    pub replayed: bool,
    pub terminal_state: LifecycleTerminalState,
    pub route_invalidation_event_count: u32,
}

/// In-memory deterministic lifecycle ledger used by PRD17C contract tests.
#[derive(Debug, Default)]
pub struct LifecycleActionLedger {
    actions_by_id: BTreeMap<String, LifecycleAction>,
    idempotency_keys: BTreeMap<String, String>,
}

impl LifecycleActionLedger {
    /// Apply a lifecycle action, replaying exact idempotent duplicates.
    pub fn apply_lifecycle_action(
        &mut self,
        action: LifecycleAction,
    ) -> Result<LifecycleApplyResult> {
        action.validate()?;
        let idempotency_key = action.idempotency_key()?;
        if let Some(existing_action_id) = self.idempotency_keys.get(&idempotency_key) {
            let Some(existing) = self.actions_by_id.get(existing_action_id) else {
                return Err(LifecycleActionError::InvalidAction {
                    reason: "idempotency key points at missing action".to_owned(),
                });
            };
            if existing == &action {
                return Ok(LifecycleApplyResult {
                    action_id: existing_action_id.clone(),
                    idempotency_key,
                    replayed: true,
                    terminal_state: existing.terminal_state,
                    route_invalidation_event_count: to_u32(
                        existing.route_invalidation_event_ids.len(),
                    )?,
                });
            }
            return Err(LifecycleActionError::DuplicateUnsafeReplay { idempotency_key });
        }
        if self.actions_by_id.contains_key(&action.action_id) {
            return Err(LifecycleActionError::DuplicateUnsafeReplay { idempotency_key });
        }
        let route_invalidation_event_count = to_u32(action.route_invalidation_event_ids.len())?;
        let result = LifecycleApplyResult {
            action_id: action.action_id.clone(),
            idempotency_key: idempotency_key.clone(),
            replayed: false,
            terminal_state: action.terminal_state,
            route_invalidation_event_count,
        };
        self.idempotency_keys
            .insert(idempotency_key, action.action_id.clone());
        self.actions_by_id.insert(action.action_id.clone(), action);
        Ok(result)
    }

    /// Number of committed non-replay lifecycle actions.
    #[must_use]
    pub fn committed_action_count(&self) -> usize {
        self.actions_by_id.len()
    }
}

/// Errors raised by PRD17C lifecycle contracts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LifecycleActionError {
    #[error("dagdb_prd17_lifecycle_json_invalid: {reason}")]
    Json { reason: String },
    #[error("dagdb_prd17_lifecycle_invalid: {reason}")]
    InvalidAction { reason: String },
    #[error("dagdb_prd17_lifecycle_empty_field: {field}")]
    EmptyField { field: String },
    #[error("dagdb_prd17_lifecycle_list_not_sorted_unique: {field}")]
    ListNotSortedUnique { field: String },
    #[error("dagdb_prd17_lifecycle_scope_mismatch: {field}")]
    ScopeMismatch { field: String },
    #[error("dagdb_prd17_lifecycle_forbidden_material: {field}: {reason}")]
    ForbiddenMaterial { field: String, reason: String },
    #[error("dagdb_prd17_lifecycle_evidence_would_be_deleted: {action_id}")]
    EvidenceWouldBeDeleted { action_id: String },
    #[error("dagdb_prd17_lifecycle_route_invalidation_gap: {action_id}")]
    RouteInvalidationGap { action_id: String },
    #[error("dagdb_prd17_lifecycle_duplicate_unsafe_replay: {idempotency_key}")]
    DuplicateUnsafeReplay { idempotency_key: String },
    #[error("dagdb_prd17_lifecycle_production_approval_missing: {action_id}")]
    ProductionApprovalMissing { action_id: String },
    #[error("dagdb_prd17_lifecycle_count_out_of_range")]
    CountOutOfRange,
}

/// Result alias for lifecycle action validation.
pub type Result<T> = std::result::Result<T, LifecycleActionError>;

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(LifecycleActionError::EmptyField {
            field: field.to_owned(),
        });
    }
    reject_forbidden_string(field, value)
}

/// Scope fields feed the colon-joined idempotency key, so a ':' inside them
/// would make distinct scopes collide on the same key (cross-scope replay
/// denial). They must stay colon-free.
fn validate_scope_field(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if value.contains(':') {
        return Err(LifecycleActionError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: "scope fields must not contain ':' (idempotency key delimiter)".to_owned(),
        });
    }
    Ok(())
}

fn validate_digest(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(LifecycleActionError::InvalidAction {
            reason: format!("{field} must be a 64-char hex digest"),
        });
    }
    Ok(())
}

fn validate_memory_refs_sorted_unique(
    field: &str,
    refs: &[LifecycleMemoryRef],
    action: &LifecycleAction,
) -> Result<()> {
    for (index, reference) in refs.iter().enumerate() {
        reference.validate_scope(action, &format!("{field}[{index}]"))?;
    }
    let memory_ids = refs
        .iter()
        .map(|reference| reference.memory_id.clone())
        .collect::<Vec<_>>();
    validate_sorted_unique_strings(field, &memory_ids)
}

fn validate_sorted_unique_strings(field: &str, values: &[String]) -> Result<()> {
    for value in values {
        validate_non_empty(field, value)?;
    }
    let sorted = values.iter().cloned().collect::<BTreeSet<_>>();
    if sorted.len() != values.len() || values != sorted.into_iter().collect::<Vec<_>>() {
        return Err(LifecycleActionError::ListNotSortedUnique {
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
                    return Err(LifecycleActionError::ForbiddenMaterial {
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
        return Err(LifecycleActionError::ForbiddenMaterial {
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

fn to_u32(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| LifecycleActionError::CountOutOfRange)
}
