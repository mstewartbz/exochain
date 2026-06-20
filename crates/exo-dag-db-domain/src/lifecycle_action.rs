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
/// Evidence id prefix required before a lifecycle action can claim production approval.
pub const PRODUCTION_LIFECYCLE_APPROVAL_EVIDENCE_PREFIX: &str = "production-lifecycle-approval:";

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

/// Operator-owned approval/finality evidence for production lifecycle acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionLifecycleApprovalEvidence {
    pub evidence_ref: LifecycleEvidenceRef,
    pub tenant_id: String,
    pub memory_namespace: String,
    pub actor_id: String,
    pub route_id: String,
    pub request_id: String,
    pub payload_hash: String,
    pub authority_did: String,
    pub authority_signature: String,
    pub approved_at: String,
}

impl ProductionLifecycleApprovalEvidence {
    /// Validate the approval evidence independently of a lifecycle action.
    pub fn validate(&self) -> Result<()> {
        self.evidence_ref
            .validate("production_lifecycle_approval")?;
        if !self
            .evidence_ref
            .evidence_id
            .starts_with(PRODUCTION_LIFECYCLE_APPROVAL_EVIDENCE_PREFIX)
        {
            return Err(LifecycleActionError::ProductionApprovalMissing {
                action_id: self.evidence_ref.evidence_id.clone(),
            });
        }
        if !self.evidence_ref.preserved {
            return Err(LifecycleActionError::EvidenceWouldBeDeleted {
                action_id: self.evidence_ref.evidence_id.clone(),
            });
        }
        validate_scope_field("production_lifecycle_approval.tenant_id", &self.tenant_id)?;
        validate_scope_field(
            "production_lifecycle_approval.memory_namespace",
            &self.memory_namespace,
        )?;
        validate_non_empty("production_lifecycle_approval.actor_id", &self.actor_id)?;
        validate_non_empty("production_lifecycle_approval.route_id", &self.route_id)?;
        validate_non_empty("production_lifecycle_approval.request_id", &self.request_id)?;
        validate_digest(
            "production_lifecycle_approval.payload_hash",
            &self.payload_hash,
        )?;
        validate_non_empty(
            "production_lifecycle_approval.authority_did",
            &self.authority_did,
        )?;
        validate_signature(
            "production_lifecycle_approval.authority_signature",
            &self.authority_signature,
        )?;
        validate_non_empty(
            "production_lifecycle_approval.approved_at",
            &self.approved_at,
        )?;
        if self.evidence_ref.digest != self.payload_hash {
            return Err(LifecycleActionError::ProductionApprovalMismatch {
                field: "payload_hash".to_owned(),
            });
        }
        Ok(())
    }

    /// Validate approval evidence against the proposed lifecycle action it finalizes.
    pub fn validate_for_lifecycle_action(&self, action: &LifecycleAction) -> Result<()> {
        self.validate()?;
        self.require_equal("tenant_id", &self.tenant_id, &action.tenant_id)?;
        self.require_equal(
            "memory_namespace",
            &self.memory_namespace,
            &action.memory_namespace,
        )?;
        self.require_equal("actor_id", &self.actor_id, &action.actor_id)?;
        self.require_equal("route_id", &self.route_id, &action.policy_ref)?;
        self.require_equal("request_id", &self.request_id, &action.source_packet_id)?;
        Ok(())
    }

    /// Validate approval evidence against the proposed continuation record it finalizes.
    pub fn validate_for_continuation_record(
        &self,
        record: &crate::continuation_persistence::ContinuationRecord,
    ) -> Result<()> {
        self.validate()?;
        self.require_equal("tenant_id", &self.tenant_id, &record.tenant_id)?;
        self.require_equal(
            "memory_namespace",
            &self.memory_namespace,
            &record.memory_namespace,
        )?;
        self.require_equal("request_id", &self.request_id, &record.task_id)?;
        Ok(())
    }

    fn require_equal(&self, field: &str, actual: &str, expected: &str) -> Result<()> {
        if actual != expected {
            return Err(LifecycleActionError::ProductionApprovalMismatch {
                field: field.to_owned(),
            });
        }
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
        if action.claims_production_approval() {
            return Err(LifecycleActionError::ProductionApprovalMissing {
                action_id: action.action_id.clone(),
            });
        }
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

        if self.claims_production_approval()
            && (self.terminal_state != LifecycleTerminalState::Accepted
                || self.production_lifecycle_approval != ProductionLifecycleApproval::Approved
                || !self.has_production_approval_evidence())
        {
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

    /// Return a production-accepted copy only when approval/finality evidence is bound.
    pub fn approved_with_evidence(
        &self,
        approval: &ProductionLifecycleApprovalEvidence,
    ) -> Result<Self> {
        self.validate()?;
        if self.terminal_state != LifecycleTerminalState::OperatorDeferred
            || self.production_lifecycle_approval != ProductionLifecycleApproval::OperatorDeferred
        {
            return Err(LifecycleActionError::InvalidAction {
                reason: "production approval can only finalize operator_deferred actions"
                    .to_owned(),
            });
        }
        approval.validate_for_lifecycle_action(self)?;
        let mut accepted = self.clone();
        if accepted
            .evidence_refs
            .iter()
            .any(|evidence| evidence.evidence_id == approval.evidence_ref.evidence_id)
        {
            return Err(LifecycleActionError::InvalidAction {
                reason: "production approval evidence must be new evidence".to_owned(),
            });
        }
        accepted.evidence_refs.push(approval.evidence_ref.clone());
        accepted.terminal_state = LifecycleTerminalState::Accepted;
        accepted.production_lifecycle_approval = ProductionLifecycleApproval::Approved;
        accepted.validate()?;
        Ok(accepted)
    }

    fn claims_production_approval(&self) -> bool {
        self.terminal_state == LifecycleTerminalState::Accepted
            || self.production_lifecycle_approval == ProductionLifecycleApproval::Approved
    }

    fn has_production_approval_evidence(&self) -> bool {
        self.evidence_refs.iter().any(|evidence| {
            evidence.preserved
                && evidence
                    .evidence_id
                    .starts_with(PRODUCTION_LIFECYCLE_APPROVAL_EVIDENCE_PREFIX)
        })
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
        if action.claims_production_approval() {
            return Err(LifecycleActionError::ProductionApprovalMissing {
                action_id: action.action_id.clone(),
            });
        }
        self.apply_lifecycle_action_internal(action)
    }

    fn apply_lifecycle_action_internal(
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

    /// Apply a lifecycle action after binding production approval/finality evidence.
    pub fn apply_approved_lifecycle_action(
        &mut self,
        action: LifecycleAction,
        approval: &ProductionLifecycleApprovalEvidence,
    ) -> Result<LifecycleApplyResult> {
        let accepted = action.approved_with_evidence(approval)?;
        self.apply_lifecycle_action_internal(accepted)
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
    #[error("dagdb_prd17_lifecycle_production_approval_mismatch: {field}")]
    ProductionApprovalMismatch { field: String },
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

fn validate_signature(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if value.len() != 128 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(LifecycleActionError::InvalidAction {
            reason: format!("{field} must be a 128-char hex Ed25519 signature"),
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

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "dag_db-local";
    const PROJECT: &str = "dag_db";
    const NAMESPACE: &str = "project_memory_v3";

    fn digest(byte: &str) -> String {
        byte.repeat(64)
    }

    fn memory_ref(memory_id: &str) -> LifecycleMemoryRef {
        LifecycleMemoryRef {
            tenant_id: TENANT.to_owned(),
            project_id: PROJECT.to_owned(),
            memory_namespace: NAMESPACE.to_owned(),
            memory_id: memory_id.to_owned(),
        }
    }

    fn evidence_ref(evidence_id: &str) -> LifecycleEvidenceRef {
        LifecycleEvidenceRef {
            evidence_id: evidence_id.to_owned(),
            receipt_id: format!("receipt-{evidence_id}"),
            digest: digest("a"),
            summary_ref: format!("summary-{evidence_id}"),
            preserved: true,
        }
    }

    fn rollback_ref(
        action_id: &str,
        action_type: LifecycleActionType,
        validation_report_id: &str,
    ) -> LifecycleRollbackRef {
        LifecycleRollbackRef {
            rollback_id: format!("rollback-{action_id}"),
            action_id: action_id.to_owned(),
            inverse_action_type: action_type.inverse(),
            before_refs: vec![memory_ref("memory-parent-a")],
            after_refs: vec![memory_ref("memory-target-a")],
            validation_ref: validation_report_id.to_owned(),
            operator_required: true,
        }
    }

    fn lifecycle_action(action_id: &str, action_type: LifecycleActionType) -> LifecycleAction {
        let validation_report_id = format!("validation-{action_id}");
        LifecycleAction {
            schema_version: PRD17_LIFECYCLE_ACTION_SCHEMA.to_owned(),
            action_id: action_id.to_owned(),
            action_type,
            tenant_id: TENANT.to_owned(),
            project_id: PROJECT.to_owned(),
            memory_namespace: NAMESPACE.to_owned(),
            actor_id: "did:agent:codex-prd17c".to_owned(),
            source_packet_id: "packet-prd17c-unit-001".to_owned(),
            source_receipt_id: "receipt-prd17c-unit-001".to_owned(),
            parent_memory_ids: vec![memory_ref("memory-parent-a")],
            target_memory_ids: vec![memory_ref("memory-target-a")],
            validation_report_id: validation_report_id.clone(),
            policy_ref: "policy-prd17c-local-mutation".to_owned(),
            rollback_ref: rollback_ref(action_id, action_type, &validation_report_id),
            route_invalidation_event_ids: vec!["route-event-prd17c-unit-001".to_owned()],
            evidence_refs: vec![evidence_ref("evidence-prd17c-unit-001")],
            terminal_state: LifecycleTerminalState::OperatorDeferred,
            production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
            created_at: "2026-06-07T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn lifecycle_action_type_wire_values_and_inverses_cover_all_variants() {
        let cases = [
            (
                LifecycleActionType::Writeback,
                "writeback",
                LifecycleActionType::Archive,
            ),
            (
                LifecycleActionType::Relink,
                "relink",
                LifecycleActionType::Relink,
            ),
            (
                LifecycleActionType::Supersede,
                "supersede",
                LifecycleActionType::Restore,
            ),
            (
                LifecycleActionType::Recycle,
                "recycle",
                LifecycleActionType::Restore,
            ),
            (
                LifecycleActionType::Archive,
                "archive",
                LifecycleActionType::Restore,
            ),
            (
                LifecycleActionType::Restore,
                "restore",
                LifecycleActionType::Archive,
            ),
            (
                LifecycleActionType::RouteInvalidate,
                "route_invalidate",
                LifecycleActionType::RouteInvalidate,
            ),
        ];

        for (action_type, wire_value, inverse) in cases {
            assert_eq!(action_type.as_str(), wire_value);
            assert_eq!(action_type.inverse(), inverse);
        }
    }

    #[test]
    fn lifecycle_action_parse_json_covers_success_json_errors_and_forbidden_material() {
        let action = lifecycle_action("lifecycle-unit-parse-001", LifecycleActionType::Writeback);
        let encoded = serde_json::to_string(&action).expect("serialize lifecycle action");
        assert_eq!(
            LifecycleAction::parse_json(&encoded).expect("parse valid lifecycle action"),
            action
        );

        assert!(matches!(
            LifecycleAction::parse_json("{"),
            Err(LifecycleActionError::Json { .. })
        ));

        let mut missing_field = serde_json::to_value(lifecycle_action(
            "lifecycle-unit-parse-002",
            LifecycleActionType::Writeback,
        ))
        .expect("lifecycle action json value");
        missing_field
            .as_object_mut()
            .expect("object")
            .remove("rollback_ref");
        assert!(matches!(
            LifecycleAction::parse_json(&missing_field.to_string()),
            Err(LifecycleActionError::Json { .. })
        ));

        let mut forbidden_key = serde_json::to_value(lifecycle_action(
            "lifecycle-unit-parse-003",
            LifecycleActionType::Writeback,
        ))
        .expect("lifecycle action json value");
        forbidden_key.as_object_mut().expect("object").insert(
            "raw_markdown".to_owned(),
            JsonValue::String("raw".to_owned()),
        );
        assert!(matches!(
            LifecycleAction::parse_json(&forbidden_key.to_string()),
            Err(LifecycleActionError::ForbiddenMaterial { .. })
        ));

        let mut forbidden_value = serde_json::to_value(lifecycle_action(
            "lifecycle-unit-parse-004",
            LifecycleActionType::Writeback,
        ))
        .expect("lifecycle action json value");
        forbidden_value.as_object_mut().expect("object").insert(
            "policy_ref".to_owned(),
            JsonValue::String("postgres://local-only".to_owned()),
        );
        assert!(matches!(
            LifecycleAction::parse_json(&forbidden_value.to_string()),
            Err(LifecycleActionError::ForbiddenMaterial { .. })
        ));
    }

    #[test]
    fn lifecycle_action_validation_rejects_schema_collections_digest_and_ordering() {
        let mut wrong_schema = lifecycle_action(
            "lifecycle-unit-validate-001",
            LifecycleActionType::Writeback,
        );
        wrong_schema.schema_version = "dagdb_prd17_lifecycle_action_v0".to_owned();
        assert_eq!(
            wrong_schema.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "unsupported lifecycle action schema_version".to_owned(),
            })
        );

        let mut missing_parent = lifecycle_action(
            "lifecycle-unit-validate-002",
            LifecycleActionType::Writeback,
        );
        missing_parent.parent_memory_ids.clear();
        assert_eq!(
            missing_parent.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "parent_memory_ids must not be empty".to_owned(),
            })
        );

        let mut missing_target = lifecycle_action(
            "lifecycle-unit-validate-003",
            LifecycleActionType::Writeback,
        );
        missing_target.target_memory_ids.clear();
        assert_eq!(
            missing_target.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "target_memory_ids must not be empty".to_owned(),
            })
        );

        let mut missing_evidence = lifecycle_action(
            "lifecycle-unit-validate-004",
            LifecycleActionType::Writeback,
        );
        missing_evidence.evidence_refs.clear();
        assert_eq!(
            missing_evidence.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "evidence_refs must not be empty".to_owned(),
            })
        );

        let mut duplicate_evidence = lifecycle_action(
            "lifecycle-unit-validate-005",
            LifecycleActionType::Writeback,
        );
        duplicate_evidence
            .evidence_refs
            .push(evidence_ref("evidence-prd17c-unit-001"));
        assert_eq!(
            duplicate_evidence.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "evidence_refs must be unique".to_owned(),
            })
        );

        let mut invalid_digest = lifecycle_action(
            "lifecycle-unit-validate-006",
            LifecycleActionType::Writeback,
        );
        invalid_digest.evidence_refs[0].digest = "not-a-digest".to_owned();
        assert!(matches!(
            invalid_digest.validate(),
            Err(LifecycleActionError::InvalidAction { reason })
                if reason == "evidence_refs[0].digest must be a 64-char hex digest"
        ));

        let mut unsorted_route_events = lifecycle_action(
            "lifecycle-unit-validate-007",
            LifecycleActionType::Writeback,
        );
        unsorted_route_events.route_invalidation_event_ids = vec![
            "route-event-prd17c-unit-b".to_owned(),
            "route-event-prd17c-unit-a".to_owned(),
        ];
        assert_eq!(
            unsorted_route_events.validate(),
            Err(LifecycleActionError::ListNotSortedUnique {
                field: "route_invalidation_event_ids".to_owned(),
            })
        );
    }

    #[test]
    fn lifecycle_action_rollback_ref_validation_rejects_mismatches_and_empty_refs() {
        let mut action_id_mismatch = lifecycle_action(
            "lifecycle-unit-rollback-001",
            LifecycleActionType::Writeback,
        );
        action_id_mismatch.rollback_ref.action_id = "other-action".to_owned();
        assert_eq!(
            action_id_mismatch.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref.action_id must match action_id".to_owned(),
            })
        );

        let mut inverse_mismatch = lifecycle_action(
            "lifecycle-unit-rollback-002",
            LifecycleActionType::Writeback,
        );
        inverse_mismatch.rollback_ref.inverse_action_type = LifecycleActionType::Restore;
        assert_eq!(
            inverse_mismatch.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref.inverse_action_type mismatch".to_owned(),
            })
        );

        let mut validation_ref_mismatch = lifecycle_action(
            "lifecycle-unit-rollback-003",
            LifecycleActionType::Writeback,
        );
        validation_ref_mismatch.rollback_ref.validation_ref = "other-validation".to_owned();
        assert_eq!(
            validation_ref_mismatch.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref.validation_ref must match validation_report_id".to_owned(),
            })
        );

        let mut empty_rollback_refs = lifecycle_action(
            "lifecycle-unit-rollback-004",
            LifecycleActionType::Writeback,
        );
        empty_rollback_refs.rollback_ref.before_refs.clear();
        empty_rollback_refs.rollback_ref.after_refs.clear();
        assert_eq!(
            empty_rollback_refs.validate(),
            Err(LifecycleActionError::InvalidAction {
                reason: "rollback_ref requires before_refs or after_refs".to_owned(),
            })
        );
    }

    #[test]
    fn lifecycle_action_ledger_replays_and_rejects_duplicate_or_missing_entries() {
        let action = lifecycle_action("lifecycle-unit-ledger-001", LifecycleActionType::Writeback);
        let mut ledger = LifecycleActionLedger::default();
        let first = ledger
            .apply_lifecycle_action(action.clone())
            .expect("first lifecycle action");
        assert!(!first.replayed);
        assert_eq!(ledger.committed_action_count(), 1);

        let replay = ledger
            .apply_lifecycle_action(action.clone())
            .expect("exact lifecycle replay");
        assert!(replay.replayed);
        assert_eq!(ledger.committed_action_count(), 1);

        let mut unsafe_replay = action.clone();
        unsafe_replay.action_id = "lifecycle-unit-ledger-002".to_owned();
        unsafe_replay.rollback_ref.action_id = unsafe_replay.action_id.clone();
        unsafe_replay.rollback_ref.rollback_id = "rollback-lifecycle-unit-ledger-002".to_owned();
        assert!(matches!(
            ledger.apply_lifecycle_action(unsafe_replay),
            Err(LifecycleActionError::DuplicateUnsafeReplay { .. })
        ));

        let mut duplicate_action_id = action.clone();
        duplicate_action_id.source_receipt_id = "receipt-prd17c-unit-duplicate".to_owned();
        assert!(matches!(
            ledger.apply_lifecycle_action(duplicate_action_id),
            Err(LifecycleActionError::DuplicateUnsafeReplay { .. })
        ));

        let missing = lifecycle_action(
            "lifecycle-unit-ledger-missing",
            LifecycleActionType::Writeback,
        );
        let missing_key = missing
            .idempotency_key()
            .expect("missing-entry action idempotency key");
        let mut inconsistent_ledger = LifecycleActionLedger::default();
        inconsistent_ledger
            .idempotency_keys
            .insert(missing_key, "missing-action".to_owned());
        assert_eq!(
            inconsistent_ledger.apply_lifecycle_action(missing),
            Err(LifecycleActionError::InvalidAction {
                reason: "idempotency key points at missing action".to_owned(),
            })
        );
    }
}
