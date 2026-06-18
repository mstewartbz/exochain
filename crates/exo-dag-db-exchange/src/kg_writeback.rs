//! Dry-run KG writeback proposal contracts.
//!
//! This module keeps agent writeback at the repository/test-level proposal
//! stage. It creates compact `MemoryCandidate` values from retrieval-backed
//! hints, binds them to context-packet preview evidence, and asks the existing
//! system-side placement controller for a proposal. It does not persist
//! writeback, expose a gateway route, export knowledge, or invalidate routes.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use exo_authority::Permission;
use exo_avc::AuthorityScope;
use exo_consent::ConsentDecision;
use exo_core::Timestamp;
use exo_dag_db_api::{
    CanonicalizationDecisionKind, MemoryCandidate, MemoryCandidateKind, MemoryCandidateUse,
    PlacementResult, RiskClass, ValidationStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::{
    kg_import::{KgImportError, hash_from_hex, stable_hash},
    kg_retrieval::{KG_CONTEXT_PACKET_PREVIEW_SCHEMA, KgContextPacketPreview},
    model::{DagDbAuthorizedScope, MemoryCandidateEmitter, OutputObserver, TaskAgentWritebackHint},
    placement::{MemoryPlacementController, MemoryPlacementInput, PlacementExistingMemory},
    scoring::{DomainError, DomainGateContext},
};

/// Schema for repository-level dry-run writeback proposal reports.
pub const KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA: &str = "dagdb_kg_writeback_dry_run_report_v1";
/// Summary schema for Markdown writeback proposal artifacts.
pub const KG_WRITEBACK_DRY_RUN_SUMMARY_SCHEMA: &str = "dagdb_kg_writeback_dry_run_summary_v1";
/// Route name used only in deterministic proposal hash material.
pub const KG_WRITEBACK_DRY_RUN_ROUTE_NAME: &str = "dagdb.kg_writeback.dry_run.v1";
/// Route name used for persisted writeback idempotency.
pub const KG_WRITEBACK_PERSISTED_ROUTE_NAME: &str = "dagdb.kg_writeback.persisted.v1";
/// Environment variable used by Postgres-gated writeback tests and helpers.
pub const KG_WRITEBACK_DATABASE_URL_ENV: &str = "EXO_DAGDB_TEST_DATABASE_URL";
/// Summary schema emitted by the repository-level persisted writeback adapter.
pub const KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA: &str = "dagdb_kg_persisted_writeback_summary_v1";
/// Default target directory for generated writeback dry-run artifacts.
pub const KG_WRITEBACK_DRY_RUN_TARGET_DIR: &str = "target/dagdb/kg_writeback_dry_run";
/// Default JSON report path for generated writeback dry-run artifacts.
pub const KG_WRITEBACK_DRY_RUN_JSON_PATH: &str = "target/dagdb/kg_writeback_dry_run/report.json";
/// Default Markdown summary path for generated writeback dry-run artifacts.
pub const KG_WRITEBACK_DRY_RUN_MD_PATH: &str = "target/dagdb/kg_writeback_dry_run/summary.md";

const RAW_BODY_KEYS: &[&str] = &[
    "body",
    "content",
    "document_body",
    "file_text",
    "full_output",
    "markdown",
    "model_output",
    "payload",
    "prompt_body",
    "private_payload",
    "raw_body",
    "raw_document_body",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_private_payload",
    "raw_prompt_body",
    "source_body",
    "source_excerpt",
    "text_body",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "begin private key",
    "private key-----",
    "authorization",
    "bearer ",
    "database_url",
    "db_url",
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

/// Agent-submitted writeback hint accepted by the dry-run proposal adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgAgentWritebackHint {
    pub source_request_id: String,
    pub parent_context_packet_id: String,
    pub route_hint_id: String,
    pub task_hash: String,
    pub answer_hash: Option<String>,
    pub output_hash: Option<String>,
    pub candidate_kind: MemoryCandidateKind,
    pub summary: String,
    #[serde(default)]
    pub citation_handles: Vec<String>,
    #[serde(default)]
    pub evidence_receipts: Vec<String>,
    pub risk_hint: RiskClass,
    #[serde(default)]
    pub allowed_future_uses: Vec<MemoryCandidateUse>,
    pub reason_to_remember: String,
    #[serde(default)]
    pub keyword_texts: Vec<String>,
    #[serde(default)]
    pub contradiction_refs: Vec<String>,
    #[serde(default)]
    pub supersession_refs: Vec<String>,
}

impl KgAgentWritebackHint {
    /// Return the normalized output hash, accepting either answer_hash or output_hash.
    pub fn normalized_output_hash(&self) -> Result<&str> {
        match (&self.answer_hash, &self.output_hash) {
            (Some(answer_hash), Some(output_hash)) if answer_hash != output_hash => {
                Err(KgWritebackError::InvalidHint {
                    reason: "answer_hash and output_hash must match when both are supplied"
                        .to_owned(),
                })
            }
            (Some(answer_hash), _) => Ok(answer_hash),
            (_, Some(output_hash)) => Ok(output_hash),
            (None, None) => Err(KgWritebackError::InvalidHint {
                reason: "answer_hash or output_hash is required".to_owned(),
            }),
        }
    }

    /// Validate deterministic and safety boundaries before candidate creation.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty("source_request_id", &self.source_request_id)?;
        hash_from_hex("parent_context_packet_id", &self.parent_context_packet_id)?;
        hash_from_hex("route_hint_id", &self.route_hint_id)?;
        hash_from_hex("task_hash", &self.task_hash)?;
        hash_from_hex("output_hash", self.normalized_output_hash()?)?;
        validate_non_empty("summary", &self.summary)?;
        validate_non_empty("reason_to_remember", &self.reason_to_remember)?;
        if self.allowed_future_uses.is_empty() {
            return Err(KgWritebackError::InvalidHint {
                reason: "allowed_future_uses must not be empty".to_owned(),
            });
        }
        if self.citation_handles.is_empty() && self.evidence_receipts.is_empty() {
            return Err(KgWritebackError::InvalidHint {
                reason: "citation_handles or evidence_receipts are required".to_owned(),
            });
        }
        validate_unique_non_empty("citation_handle", &self.citation_handles)?;
        validate_unique_non_empty("evidence_receipt", &self.evidence_receipts)?;
        validate_unique_non_empty("keyword_text", &self.keyword_texts)?;
        validate_unique_hash_refs("contradiction_ref", &self.contradiction_refs)?;
        validate_unique_hash_refs("supersession_ref", &self.supersession_refs)?;
        for receipt in &self.evidence_receipts {
            hash_from_hex("evidence_receipt", receipt)?;
        }
        Ok(())
    }
}

/// Request used by system code/tests to create a dry-run writeback proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgWritebackProposalRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub requesting_agent_did: String,
    pub context_packet: KgContextPacketPreview,
    pub hint: KgAgentWritebackHint,
    pub existing_memory: Vec<KgWritebackExistingMemory>,
}

/// Existing memory summary available to the dry-run placement overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackExistingMemory {
    pub memory_id: String,
    pub payload_hash: String,
    pub summary: String,
}

/// Deterministic dry-run writeback proposal report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackDryRunReport {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub requesting_agent_did: String,
    pub source_request_id: String,
    pub parent_context_packet_id: String,
    pub route_hint_id: String,
    pub task_hash: String,
    pub output_hash: String,
    pub proposal_id: String,
    pub candidate_id: String,
    pub dry_run_only: bool,
    pub postgres_writes: bool,
    pub raw_markdown_included: bool,
    pub preview_only: bool,
    pub proposed_memory_candidate: MemoryCandidate,
    pub evidence_binding: KgWritebackEvidenceBinding,
    pub placement_proposal: PlacementResult,
    pub placement_trace: Vec<String>,
    pub validation_proposal: KgWritebackValidationProposal,
    pub receipt_intent_proposal: KgWritebackReceiptIntentProposal,
    #[serde(default)]
    pub layered_writeback: Option<KgWritebackLayeredWriteback>,
    pub proposed_route_invalidations: Vec<KgWritebackRouteInvalidationProposal>,
    pub warnings: Vec<String>,
    pub proposal_summary: KgWritebackProposalSummary,
    pub acceptance: KgWritebackAcceptance,
}

/// Bound context-packet/citation evidence for the writeback proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackEvidenceBinding {
    pub parent_context_packet_id: String,
    pub route_hint_id: String,
    pub selected_memory_ids: Vec<String>,
    pub citation_handles: Vec<String>,
    pub evidence_receipts: Vec<String>,
    pub validation_report_ids: Vec<String>,
    pub missing_citation_handles: Vec<String>,
    pub status: String,
    pub binding_reasons: Vec<String>,
}

/// Dry-run validation proposal for the compact writeback candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackValidationProposal {
    pub validation_report_id: String,
    pub validation_status: String,
    pub decision: String,
    pub risk_class: RiskClass,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
}

/// Receipt intent proposal. No receipt is written in this phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackReceiptIntentProposal {
    pub receipt_intent_id: String,
    pub event_type: String,
    pub subject_id: String,
    pub status: String,
    pub dry_run_only: bool,
}

/// Advisory route invalidation proposal. No persisted route state is mutated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackRouteInvalidationProposal {
    pub route_hint_id: String,
    pub affected_memory_ids: Vec<String>,
    pub trigger_type: String,
    pub status: String,
    pub persisted_route_invalidation: bool,
    pub reason: String,
}

/// Optional layer-growth evidence for an accepted persisted writeback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackLayeredWriteback {
    pub target_layer_id: String,
    pub target_layer_path: String,
    pub target_layer_depth: u32,
    pub target_layer_kind: String,
    pub target_graph_style: String,
    pub target_layer_reason: String,
    pub parent_layer_id: Option<String>,
    pub parent_graph_node_id: Option<String>,
    pub created_child_layer_id: Option<String>,
    pub layer_membership_id: String,
    pub membership_role: String,
    pub local_node_rank: u32,
    pub layer_edge_id: Option<String>,
    pub layer_edge_kind: Option<String>,
    pub layer_fallback_used: bool,
}

/// Deterministic summary rollup for the writeback proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackProposalSummary {
    pub compact_candidate_created: bool,
    pub evidence_bound: bool,
    pub placement_proposed: bool,
    pub selected_memory_count: u32,
    pub citation_handle_count: u32,
    pub evidence_receipt_count: u32,
    pub route_invalidation_proposal_count: u32,
    pub needs_review: bool,
    pub persistence_ready: bool,
    pub next_required_phase: String,
}

/// Acceptance flags for this bounded phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackAcceptance {
    pub dry_run_only: bool,
    pub writeback_persistence_implemented: bool,
    pub export_persistence_implemented: bool,
    pub gateway_exposure_implemented: bool,
    pub graph_explorer_changes_implemented: bool,
    pub exo_dag_tables_mutated: bool,
    pub raw_markdown_returned: bool,
}

/// Summary returned by the feature-gated persisted writeback repository adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedSummary {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub proposal_id: String,
    pub candidate_id: String,
    pub idempotency_key: String,
    pub replayed: bool,
    pub inserted_memory_count: u32,
    pub inserted_catalog_count: u32,
    pub inserted_graph_node_count: u32,
    pub inserted_graph_edge_count: u32,
    #[serde(default)]
    pub inserted_layer_count: u32,
    #[serde(default)]
    pub inserted_layer_membership_count: u32,
    #[serde(default)]
    pub inserted_layer_edge_count: u32,
    #[serde(default)]
    pub inserted_memory_edge_count: u32,
    pub inserted_similarity_result_count: u32,
    pub inserted_validation_report_count: u32,
    pub inserted_placement_decision_count: u32,
    pub inserted_placement_trace_count: u32,
    pub inserted_receipt_count: u32,
    #[serde(default)]
    pub inserted_subject_receipt_head_count: u32,
    #[serde(default)]
    pub inserted_idempotency_response_count: u32,
    pub skipped_advisory_section_count: u32,
    pub persisted_route_invalidation_count: u32,
    pub persisted_export_record_count: u32,
    pub preview_evidence_only: bool,
    #[serde(default)]
    pub diagnostics: KgWritebackPersistedDiagnostics,
}

/// Deterministic diagnostics returned by the persisted writeback adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedDiagnostics {
    pub persisted_row_counts: KgWritebackPersistedRowCounts,
    pub advisory_deferred: KgWritebackAdvisoryDeferredDiagnostics,
    pub evidence: KgWritebackPersistedEvidenceDiagnostics,
    pub placement_governance: KgWritebackPersistedPlacementDiagnostics,
    #[serde(default)]
    pub layered_writeback: KgWritebackPersistedLayerDiagnostics,
    pub validation_risk_council: KgWritebackPersistedValidationDiagnostics,
    pub idempotency_replay: KgWritebackPersistedIdempotencyDiagnostics,
    pub warning_summaries: Vec<String>,
}

/// Persisted row-count rollup for review/debugging.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedRowCounts {
    pub memory_rows: u32,
    pub catalog_rows: u32,
    pub graph_node_rows: u32,
    pub graph_edge_rows: u32,
    #[serde(default)]
    pub layer_rows: u32,
    #[serde(default)]
    pub layer_membership_rows: u32,
    #[serde(default)]
    pub layer_edge_rows: u32,
    #[serde(default)]
    pub memory_edge_rows: u32,
    pub similarity_result_rows: u32,
    pub canonicalization_decision_rows: u32,
    pub placement_trace_rows: u32,
    pub validation_report_rows: u32,
    pub receipt_rows: u32,
    pub subject_receipt_head_rows: u32,
    pub idempotency_response_rows: u32,
    pub route_invalidation_rows: u32,
    pub export_record_rows: u32,
}

/// Advisory/deferred section rollup for unsupported first-slice write targets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackAdvisoryDeferredDiagnostics {
    pub route_invalidation_proposals: u32,
    pub governance_review_items: u32,
    pub export_records: u32,
    pub memory_candidate_queue_records: u32,
    pub skipped_section_count: u32,
    pub skipped_sections: Vec<KgWritebackSkippedSection>,
}

/// Deterministic explanation for a skipped/advisory writeback section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackSkippedSection {
    pub section: String,
    pub status: String,
    pub reason: String,
}

/// Evidence binding diagnostics for persisted writeback review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedEvidenceDiagnostics {
    pub parent_context_packet_id: String,
    pub route_hint_id: String,
    pub selected_memory_ids: Vec<String>,
    pub citation_handles: Vec<String>,
    pub validation_report_ids: Vec<String>,
    pub receipt_hashes: Vec<String>,
    pub task_hash: String,
    pub output_hash: String,
    pub tenant_namespace_match: bool,
    pub evidence_status: String,
    pub evidence_warnings: Vec<String>,
}

/// Placement/governance diagnostics for persisted writeback review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedPlacementDiagnostics {
    pub placement_decision_id: String,
    pub placement_decision_kind: String,
    pub placement_status: String,
    pub canonical_memory_id: Option<String>,
    pub matched_memory_ids: Vec<String>,
    pub required_edges_to_create_count: u32,
    pub graph_views_to_refresh: Vec<String>,
    pub validator_report: String,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
    pub route_invalidation_status: String,
}

/// Layer-growth diagnostics for persisted writeback review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedLayerDiagnostics {
    pub layered_writeback_status: String,
    pub target_layer_id: Option<String>,
    pub target_layer_path: Option<String>,
    pub target_layer_depth: Option<u32>,
    pub target_layer_reason: Option<String>,
    pub parent_layer_id: Option<String>,
    pub parent_graph_node_id: Option<String>,
    pub created_child_layer_id: Option<String>,
    pub layer_membership_id: Option<String>,
    pub layer_edge_id: Option<String>,
    pub receipt_hash: Option<String>,
}

/// Validation/risk/council diagnostics for persisted writeback review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedValidationDiagnostics {
    pub validation_report_id: String,
    pub validation_status: String,
    pub risk_class: String,
    pub risk_bp: u16,
    pub council_status: String,
    pub council_required: bool,
    pub decision: String,
    pub notes_status: String,
}

/// Idempotency and replay diagnostics for persisted writeback review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgWritebackPersistedIdempotencyDiagnostics {
    pub idempotency_key: String,
    pub replayed: bool,
    pub request_hash: String,
    pub duplicate_writeback_detected: bool,
    pub replay_reason: String,
}

/// Files written by the dry-run artifact helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgWritebackArtifactSet {
    pub output_json: PathBuf,
    pub output_md: PathBuf,
}

/// Errors raised by the dry-run writeback proposal layer.
#[derive(Debug, Error)]
pub enum KgWritebackError {
    /// Hint JSON failed to parse.
    #[error("kg_writeback_hint_json_invalid: {reason}")]
    InvalidJson {
        /// Stable parse reason.
        reason: String,
    },
    /// The writeback hint is invalid or unsafe.
    #[error("kg_writeback_hint_invalid: {reason}")]
    InvalidHint {
        /// Stable validation reason.
        reason: String,
    },
    /// Retrieval context evidence is missing, stale, invalid, or cross-scope.
    #[error("kg_writeback_evidence_invalid: {reason}")]
    InvalidEvidence {
        /// Stable evidence reason.
        reason: String,
    },
    /// Hash material could not be parsed or computed.
    #[error("kg_writeback_hash_failed: {reason}")]
    Hash {
        /// Stable hash reason.
        reason: String,
    },
    /// Compact memory candidate validation failed.
    #[error("kg_writeback_candidate_invalid")]
    Candidate {
        /// Source validation error.
        #[source]
        source: crate::model::MemoryCandidateValidationError,
    },
    /// System-side placement rejected the proposal.
    #[error("kg_writeback_placement_failed")]
    Placement {
        /// Source placement/domain error.
        #[source]
        source: DomainError,
    },
    /// Artifact JSON conversion failed.
    #[error("kg_writeback_json_failed: {reason}")]
    Json {
        /// Stable conversion reason.
        reason: String,
    },
    /// Artifact IO failed.
    #[error("kg_writeback_io_failed")]
    Io {
        /// Source IO error.
        #[source]
        source: io::Error,
    },
}

/// Result alias for writeback proposal work.
pub type Result<T> = std::result::Result<T, KgWritebackError>;

impl From<KgImportError> for KgWritebackError {
    fn from(error: KgImportError) -> Self {
        Self::Hash {
            reason: error.to_string(),
        }
    }
}

/// Parse an agent writeback hint JSON object and reject unsafe/raw payload fields.
pub fn parse_agent_writeback_hint_json(hint_json: &str) -> Result<KgAgentWritebackHint> {
    let raw: JsonValue =
        serde_json::from_str(hint_json).map_err(|error| KgWritebackError::InvalidJson {
            reason: error.to_string(),
        })?;
    reject_raw_body_keys(&raw, "writeback_hint")?;
    let hint: KgAgentWritebackHint =
        serde_json::from_value(raw).map_err(|error| KgWritebackError::InvalidJson {
            reason: error.to_string(),
        })?;
    hint.validate()?;
    Ok(hint)
}

impl KgWritebackDryRunReport {
    /// Parse and validate a dry-run writeback report before persisted adapter use.
    pub fn parse_json(report_json: &str) -> Result<Self> {
        let raw: JsonValue =
            serde_json::from_str(report_json).map_err(|error| KgWritebackError::InvalidJson {
                reason: error.to_string(),
            })?;
        reject_raw_body_keys(&raw, "writeback_report")?;
        let report: Self =
            serde_json::from_value(raw).map_err(|error| KgWritebackError::InvalidJson {
                reason: error.to_string(),
            })?;
        report.validate_for_persistence()?;
        Ok(report)
    }

    /// Deterministic idempotency key for persisted writeback replay.
    pub fn idempotency_key(&self) -> Result<String> {
        Ok(stable_hash(
            "exo.dagdb.kg_writeback.persisted.idempotency_key",
            &[
                &self.tenant_id,
                &self.namespace,
                &self.requesting_agent_did,
                &self.source_request_id,
                &self.parent_context_packet_id,
                &self.route_hint_id,
                &self.task_hash,
                &self.output_hash,
                &self.candidate_id,
                &self.proposal_id,
                &self.schema_version,
            ],
        )?
        .to_string())
    }

    /// Validate report invariants that do not require Postgres.
    pub fn validate_for_persistence(&self) -> Result<()> {
        if self.schema_version != KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA {
            return Err(KgWritebackError::InvalidHint {
                reason: format!("unsupported schema_version: {}", self.schema_version),
            });
        }
        validate_non_empty("tenant_id", &self.tenant_id)?;
        validate_non_empty("namespace", &self.namespace)?;
        validate_tenant_identity("tenant_id", &self.tenant_id)?;
        validate_tenant_identity("namespace", &self.namespace)?;
        validate_did("requesting_agent_did", &self.requesting_agent_did)?;
        validate_non_empty("source_request_id", &self.source_request_id)?;
        hash_from_hex("parent_context_packet_id", &self.parent_context_packet_id)?;
        hash_from_hex("route_hint_id", &self.route_hint_id)?;
        hash_from_hex("task_hash", &self.task_hash)?;
        hash_from_hex("output_hash", &self.output_hash)?;
        hash_from_hex("proposal_id", &self.proposal_id)?;
        hash_from_hex("candidate_id", &self.candidate_id)?;
        if !self.dry_run_only
            || self.postgres_writes
            || self.raw_markdown_included
            || !self.preview_only
        {
            return Err(KgWritebackError::InvalidHint {
                reason:
                    "writeback report must be dry-run, DB-free, raw-Markdown-free, and preview-only"
                        .to_owned(),
            });
        }
        if !self.acceptance.dry_run_only
            || self.acceptance.writeback_persistence_implemented
            || self.acceptance.export_persistence_implemented
            || self.acceptance.gateway_exposure_implemented
            || self.acceptance.graph_explorer_changes_implemented
            || self.acceptance.exo_dag_tables_mutated
            || self.acceptance.raw_markdown_returned
        {
            return Err(KgWritebackError::InvalidHint {
                reason: "writeback acceptance flags must remain dry-run only".to_owned(),
            });
        }
        if self.proposed_memory_candidate.source_request_id != self.source_request_id
            || self.proposed_memory_candidate.parent_context_packet_id
                != self.parent_context_packet_id
            || self.proposed_memory_candidate.full_output_hash != self.output_hash
        {
            return Err(KgWritebackError::InvalidHint {
                reason: "memory candidate does not match report hash material".to_owned(),
            });
        }
        OutputObserver::validate_compact_candidate(&self.proposed_memory_candidate)
            .map_err(|source| KgWritebackError::Candidate { source })?;
        self.evidence_binding.validate(self)?;
        self.validation_proposal.validate()?;
        self.receipt_intent_proposal.validate(self)?;
        if let Some(layered_writeback) = &self.layered_writeback {
            layered_writeback.validate()?;
        }
        validate_placement_proposal(self)?;
        self.validate_system_derived_consistency()?;
        Ok(())
    }

    /// Re-derive the governance-critical values the system owns and reject any
    /// caller-supplied report that does not equal them.
    ///
    /// `validate_for_persistence` above only checks the report against itself, so
    /// a forged report could set `validation_status=passed`, `decision=allow`, or
    /// `needs_review=false` and pass. The persistence path must never trust a
    /// caller-supplied proposal: it re-derives the candidate id from the hint
    /// material and re-derives the validation/review outcome from the system-owned
    /// signals (risk class + recorded placement decision), then requires equality.
    fn validate_system_derived_consistency(&self) -> Result<()> {
        // 1. Re-derive candidate_id from the report's own hint material + bound
        //    evidence. A forged candidate_id (and anything keyed off it) is caught.
        let system_candidate_id = writeback_candidate_id_parts(
            &self.tenant_id,
            &self.namespace,
            &self.requesting_agent_did,
            &self.source_request_id,
            &self.parent_context_packet_id,
            &self.route_hint_id,
            &self.task_hash,
            &self.output_hash,
            self.proposed_memory_candidate.candidate_kind,
            &self.proposed_memory_candidate.summary,
            &self.evidence_binding.selected_memory_ids,
            &self.evidence_binding.citation_handles,
            &self.evidence_binding.evidence_receipts,
        )?;
        if system_candidate_id != self.candidate_id {
            return Err(KgWritebackError::InvalidHint {
                reason: "candidate_id does not match system-derived hint material".to_owned(),
            });
        }

        // 2. Re-derive the validation/review outcome from the system-owned signals:
        //    the candidate's risk class and the recorded placement decision. These
        //    are exactly the inputs the dry-run generator uses, so an honest report
        //    reproduces them and a forged "passed/allow/no-review" report cannot.
        let system_review_reasons = system_writeback_review_reasons(
            self.proposed_memory_candidate.risk_hint,
            self.placement_proposal
                .canonicalization_decision
                .decision_kind,
            &self.validation_proposal.review_reasons,
        );
        let system_needs_review = !system_review_reasons.is_empty();
        if self.validation_proposal.needs_review != system_needs_review
            || self.proposal_summary.needs_review != system_needs_review
        {
            return Err(KgWritebackError::InvalidHint {
                reason: "needs_review does not match system-derived placement outcome".to_owned(),
            });
        }
        let system_validation_status = if system_needs_review {
            "needs_review"
        } else {
            "passed"
        };
        let system_decision = if system_needs_review {
            "needs_review"
        } else {
            "allow"
        };
        if self.validation_proposal.validation_status != system_validation_status
            || self.validation_proposal.decision != system_decision
        {
            return Err(KgWritebackError::InvalidHint {
                reason: "validation_status/decision does not match system-derived outcome"
                    .to_owned(),
            });
        }
        if self.validation_proposal.review_reasons != system_review_reasons {
            return Err(KgWritebackError::InvalidHint {
                reason: "review_reasons do not match system-derived outcome".to_owned(),
            });
        }

        // 3. Recompute the ids keyed off the re-derived outcome so the whole chain
        //    is system-owned (a forger cannot independently fix them up).
        let system_validation_report_id = stable_hash(
            "exo.dagdb.kg_writeback.dry_run.validation_report_id",
            &[
                &self.tenant_id,
                &self.namespace,
                &self.candidate_id,
                system_validation_status,
                &system_review_reasons.join("|"),
            ],
        )?
        .to_string();
        if self.validation_proposal.validation_report_id != system_validation_report_id {
            return Err(KgWritebackError::InvalidHint {
                reason: "validation_report_id does not match system-derived outcome".to_owned(),
            });
        }
        let system_receipt_intent_id = stable_hash(
            "exo.dagdb.kg_writeback.dry_run.receipt_intent_id",
            &[
                &self.tenant_id,
                &self.namespace,
                &self.candidate_id,
                "writeback_created",
            ],
        )?
        .to_string();
        if self.receipt_intent_proposal.receipt_intent_id != system_receipt_intent_id {
            return Err(KgWritebackError::InvalidHint {
                reason: "receipt_intent_id does not match system-derived material".to_owned(),
            });
        }
        let system_proposal_id = stable_hash(
            "exo.dagdb.kg_writeback.dry_run.proposal_id",
            &[
                &self.tenant_id,
                &self.namespace,
                &self.requesting_agent_did,
                &self.source_request_id,
                &self.parent_context_packet_id,
                &self.route_hint_id,
                &self.task_hash,
                &self.output_hash,
                &self.candidate_id,
                &system_validation_report_id,
            ],
        )?
        .to_string();
        if self.proposal_id != system_proposal_id {
            return Err(KgWritebackError::InvalidHint {
                reason: "proposal_id does not match system-derived material".to_owned(),
            });
        }
        Ok(())
    }
}

impl KgWritebackLayeredWriteback {
    fn validate(&self) -> Result<()> {
        hash_from_hex("target_layer_id", &self.target_layer_id)?;
        validate_relative_layer_path("target_layer_path", &self.target_layer_path)?;
        if self.target_layer_depth > crate::layered_placement::LAYER_PLACEMENT_MAX_DEPTH {
            return Err(KgWritebackError::InvalidHint {
                reason: "layered writeback exceeds max layer depth".to_owned(),
            });
        }
        validate_non_empty("target_layer_reason", &self.target_layer_reason)?;
        validate_choice(
            "target_layer_kind",
            &self.target_layer_kind,
            &[
                "root",
                "repository",
                "knowledge_graph",
                "source_subgraph",
                "task_subgraph",
                "rollup",
                "route",
            ],
        )?;
        validate_choice(
            "target_graph_style",
            &self.target_graph_style,
            &["canonical_memory_graph"],
        )?;
        hash_from_hex("layer_membership_id", &self.layer_membership_id)?;
        validate_choice(
            "membership_role",
            &self.membership_role,
            &["root", "container", "member", "summary", "route_anchor"],
        )?;
        if self.target_layer_depth == 0 {
            if self.target_layer_kind != "root" {
                return Err(KgWritebackError::InvalidHint {
                    reason: "root layered writeback must use root layer kind".to_owned(),
                });
            }
            if self.parent_layer_id.is_some()
                || self.parent_graph_node_id.is_some()
                || self.layer_edge_id.is_some()
                || self.layer_edge_kind.is_some()
                || self.created_child_layer_id.is_some()
            {
                return Err(KgWritebackError::InvalidHint {
                    reason: "root layered writeback cannot include child-layer bindings".to_owned(),
                });
            }
        } else {
            if self.target_layer_kind == "root" {
                return Err(KgWritebackError::InvalidHint {
                    reason: "child layered writeback cannot use root layer kind".to_owned(),
                });
            }
            let Some(parent_layer_id) = &self.parent_layer_id else {
                return Err(KgWritebackError::InvalidHint {
                    reason: "child layered writeback missing parent_layer_id".to_owned(),
                });
            };
            let Some(parent_graph_node_id) = &self.parent_graph_node_id else {
                return Err(KgWritebackError::InvalidHint {
                    reason: "child layered writeback missing parent_graph_node_id".to_owned(),
                });
            };
            hash_from_hex("parent_layer_id", parent_layer_id)?;
            hash_from_hex("parent_graph_node_id", parent_graph_node_id)?;
            let Some(created_child_layer_id) = &self.created_child_layer_id else {
                return Err(KgWritebackError::InvalidHint {
                    reason: "child layered writeback missing created_child_layer_id".to_owned(),
                });
            };
            if created_child_layer_id != &self.target_layer_id {
                return Err(KgWritebackError::InvalidHint {
                    reason: "created_child_layer_id must match target_layer_id".to_owned(),
                });
            }
            hash_from_hex("created_child_layer_id", created_child_layer_id)?;
            let Some(layer_edge_id) = &self.layer_edge_id else {
                return Err(KgWritebackError::InvalidHint {
                    reason: "child layered writeback missing layer_edge_id".to_owned(),
                });
            };
            let Some(layer_edge_kind) = &self.layer_edge_kind else {
                return Err(KgWritebackError::InvalidHint {
                    reason: "child layered writeback missing layer_edge_kind".to_owned(),
                });
            };
            hash_from_hex("layer_edge_id", layer_edge_id)?;
            validate_choice(
                "layer_edge_kind",
                layer_edge_kind,
                &[
                    "contains_subgraph",
                    "drills_down_to",
                    "rolls_up_to",
                    "cross_layer_ref",
                    "summarizes_layer",
                ],
            )?;
        }
        if self.layer_fallback_used && self.target_layer_path != "root" {
            return Err(KgWritebackError::InvalidHint {
                reason: "layer fallback can only target root".to_owned(),
            });
        }
        Ok(())
    }
}

impl KgWritebackEvidenceBinding {
    fn validate(&self, report: &KgWritebackDryRunReport) -> Result<()> {
        if self.parent_context_packet_id != report.parent_context_packet_id {
            return Err(KgWritebackError::InvalidEvidence {
                reason: "evidence parent_context_packet_id mismatch".to_owned(),
            });
        }
        if self.route_hint_id != report.route_hint_id {
            return Err(KgWritebackError::InvalidEvidence {
                reason: "evidence route_hint_id mismatch".to_owned(),
            });
        }
        validate_choice("evidence status", &self.status, &["bound"])?;
        validate_unique_hash_refs("selected_memory_id", &self.selected_memory_ids)?;
        validate_unique_non_empty("citation_handle", &self.citation_handles)?;
        validate_unique_hash_refs("evidence_receipt", &self.evidence_receipts)?;
        validate_unique_hash_refs("validation_report_id", &self.validation_report_ids)?;
        if !self.missing_citation_handles.is_empty() {
            return Err(KgWritebackError::InvalidEvidence {
                reason: "missing citation handles cannot be persisted".to_owned(),
            });
        }
        if self.selected_memory_ids.is_empty() {
            return Err(KgWritebackError::InvalidEvidence {
                reason: "writeback persistence requires selected memory evidence".to_owned(),
            });
        }
        Ok(())
    }
}

impl KgWritebackValidationProposal {
    fn validate(&self) -> Result<()> {
        hash_from_hex("validation_report_id", &self.validation_report_id)?;
        validate_choice(
            "validation_status",
            &self.validation_status,
            &["passed", "needs_review"],
        )?;
        validate_choice(
            "validation_decision",
            &self.decision,
            &["allow", "needs_review"],
        )?;
        if self.needs_review && self.review_reasons.is_empty() {
            return Err(KgWritebackError::InvalidHint {
                reason: "needs_review validation requires review_reasons".to_owned(),
            });
        }
        validate_unique_non_empty("review_reason", &self.review_reasons)
    }
}

impl KgWritebackReceiptIntentProposal {
    fn validate(&self, report: &KgWritebackDryRunReport) -> Result<()> {
        hash_from_hex("receipt_intent_id", &self.receipt_intent_id)?;
        if self.subject_id != report.candidate_id {
            return Err(KgWritebackError::InvalidHint {
                reason: "receipt subject_id must match candidate_id".to_owned(),
            });
        }
        validate_choice(
            "receipt_event_type",
            &self.event_type,
            &["writeback_created"],
        )?;
        validate_choice("receipt_status", &self.status, &["proposed"])?;
        if !self.dry_run_only {
            return Err(KgWritebackError::InvalidHint {
                reason: "receipt intent must be dry-run only".to_owned(),
            });
        }
        Ok(())
    }
}

/// Build a deterministic dry-run writeback proposal report.
pub fn build_writeback_dry_run_report(
    request: KgWritebackProposalRequest,
) -> Result<KgWritebackDryRunReport> {
    validate_request_scope(&request)?;
    request.hint.validate()?;
    let evidence_binding = bind_context_evidence(&request)?;
    let output_hash = request.hint.normalized_output_hash()?.to_owned();
    let candidate_id = writeback_candidate_id(&request, &output_hash, &evidence_binding)?;
    let proposed_candidate = MemoryCandidateEmitter::observe_completed_task_output(
        request.hint.source_request_id.clone(),
        request.hint.parent_context_packet_id.clone(),
        output_hash.clone(),
        TaskAgentWritebackHint {
            candidate_kind: request.hint.candidate_kind,
            summary: request.hint.summary.clone(),
            evidence_receipts: request.hint.evidence_receipts.clone(),
            risk_hint: request.hint.risk_hint,
            allowed_future_uses: request.hint.allowed_future_uses.clone(),
            reason_to_remember: request.hint.reason_to_remember.clone(),
        },
    )
    .map_err(|source| KgWritebackError::Candidate { source })?;
    let placement_proposal =
        run_system_side_placement(&request, &candidate_id, &output_hash, &proposed_candidate)?;
    let review_reasons = writeback_review_reasons(&request, &placement_proposal);
    let needs_review = !review_reasons.is_empty();
    let validation_status = if needs_review {
        "needs_review"
    } else {
        "passed"
    };
    let validation_report_id = stable_hash(
        "exo.dagdb.kg_writeback.dry_run.validation_report_id",
        &[
            &request.tenant_id,
            &request.namespace,
            &candidate_id,
            validation_status,
            &review_reasons.join("|"),
        ],
    )?
    .to_string();
    let receipt_intent_id = stable_hash(
        "exo.dagdb.kg_writeback.dry_run.receipt_intent_id",
        &[
            &request.tenant_id,
            &request.namespace,
            &candidate_id,
            "writeback_created",
        ],
    )?
    .to_string();
    let proposal_id = stable_hash(
        "exo.dagdb.kg_writeback.dry_run.proposal_id",
        &[
            &request.tenant_id,
            &request.namespace,
            &request.requesting_agent_did,
            &request.hint.source_request_id,
            &request.hint.parent_context_packet_id,
            &request.hint.route_hint_id,
            &request.hint.task_hash,
            &output_hash,
            &candidate_id,
            &validation_report_id,
        ],
    )?
    .to_string();
    let proposed_route_invalidations =
        route_invalidation_proposals(&request, &evidence_binding, &review_reasons);
    let warnings = writeback_warnings(&request, needs_review);
    Ok(KgWritebackDryRunReport {
        schema_version: KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA.to_owned(),
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        requesting_agent_did: request.requesting_agent_did.clone(),
        source_request_id: request.hint.source_request_id.clone(),
        parent_context_packet_id: request.hint.parent_context_packet_id.clone(),
        route_hint_id: request.hint.route_hint_id.clone(),
        task_hash: request.hint.task_hash.clone(),
        output_hash,
        proposal_id,
        candidate_id: candidate_id.clone(),
        dry_run_only: true,
        postgres_writes: false,
        raw_markdown_included: false,
        preview_only: true,
        proposed_memory_candidate: proposed_candidate,
        evidence_binding,
        placement_proposal,
        placement_trace: placement_trace_labels(),
        validation_proposal: KgWritebackValidationProposal {
            validation_report_id,
            validation_status: validation_status.to_owned(),
            decision: if needs_review {
                "needs_review"
            } else {
                "allow"
            }
            .to_owned(),
            risk_class: request.hint.risk_hint,
            needs_review,
            review_reasons,
        },
        receipt_intent_proposal: KgWritebackReceiptIntentProposal {
            receipt_intent_id,
            event_type: "writeback_created".to_owned(),
            subject_id: candidate_id,
            status: "proposed".to_owned(),
            dry_run_only: true,
        },
        layered_writeback: None,
        proposed_route_invalidations,
        warnings: warnings.clone(),
        proposal_summary: KgWritebackProposalSummary {
            compact_candidate_created: true,
            evidence_bound: true,
            placement_proposed: true,
            selected_memory_count: usize_to_u32(
                "selected_memory_count",
                request.context_packet.memory_refs.len(),
            )?,
            citation_handle_count: usize_to_u32(
                "citation_handle_count",
                request.hint.citation_handles.len(),
            )?,
            evidence_receipt_count: usize_to_u32(
                "evidence_receipt_count",
                request.hint.evidence_receipts.len(),
            )?,
            route_invalidation_proposal_count: usize_to_u32(
                "route_invalidation_proposal_count",
                warnings
                    .iter()
                    .filter(|warning| warning.as_str() == "route_invalidation_advisory_only")
                    .count(),
            )?,
            needs_review,
            persistence_ready: false,
            next_required_phase: "writeback persistence contract review".to_owned(),
        },
        acceptance: KgWritebackAcceptance {
            dry_run_only: true,
            writeback_persistence_implemented: false,
            export_persistence_implemented: false,
            gateway_exposure_implemented: false,
            graph_explorer_changes_implemented: false,
            exo_dag_tables_mutated: false,
            raw_markdown_returned: false,
        },
    })
}

/// Write JSON and Markdown artifacts for a dry-run writeback report.
pub fn write_writeback_dry_run_artifacts(
    report: &KgWritebackDryRunReport,
    output_json: impl AsRef<Path>,
    output_md: impl AsRef<Path>,
) -> Result<KgWritebackArtifactSet> {
    let output_json = output_json.as_ref();
    let output_md = output_md.as_ref();
    if let Some(parent) = output_json.parent() {
        fs::create_dir_all(parent).map_err(|source| KgWritebackError::Io { source })?;
    }
    if let Some(parent) = output_md.parent() {
        fs::create_dir_all(parent).map_err(|source| KgWritebackError::Io { source })?;
    }
    let report_json =
        serde_json::to_string_pretty(report).map_err(|error| KgWritebackError::Json {
            reason: error.to_string(),
        })?;
    fs::write(output_json, format!("{report_json}\n"))
        .map_err(|source| KgWritebackError::Io { source })?;
    fs::write(output_md, writeback_markdown_summary(report))
        .map_err(|source| KgWritebackError::Io { source })?;
    Ok(KgWritebackArtifactSet {
        output_json: output_json.to_path_buf(),
        output_md: output_md.to_path_buf(),
    })
}

/// Write artifacts to the default target/dagdb path.
pub fn write_default_writeback_dry_run_artifacts(
    report: &KgWritebackDryRunReport,
) -> Result<KgWritebackArtifactSet> {
    write_writeback_dry_run_artifacts(
        report,
        workspace_relative_path(KG_WRITEBACK_DRY_RUN_JSON_PATH),
        workspace_relative_path(KG_WRITEBACK_DRY_RUN_MD_PATH),
    )
}

/// Build a compact deterministic Markdown summary for human review.
#[must_use]
pub fn writeback_markdown_summary(report: &KgWritebackDryRunReport) -> String {
    let mut lines = vec![
        "# DAG DB KG Writeback Dry-Run Proposal".to_owned(),
        String::new(),
        format!("- schema: `{KG_WRITEBACK_DRY_RUN_SUMMARY_SCHEMA}`"),
        format!("- report schema: `{}`", report.schema_version),
        format!("- tenant: `{}`", report.tenant_id),
        format!("- namespace: `{}`", report.namespace),
        format!("- proposal_id: `{}`", report.proposal_id),
        format!("- candidate_id: `{}`", report.candidate_id),
        format!(
            "- placement: `{:?}`",
            report
                .placement_proposal
                .canonicalization_decision
                .decision_kind
        ),
        format!(
            "- selected memories: `{}`",
            report.evidence_binding.selected_memory_ids.len()
        ),
        format!(
            "- citation handles: `{}`",
            report.evidence_binding.citation_handles.len()
        ),
        format!("- needs_review: `{}`", report.proposal_summary.needs_review),
        format!("- dry_run_only: `{}`", report.acceptance.dry_run_only),
        String::new(),
        "## Warnings".to_owned(),
    ];
    if report.warnings.is_empty() {
        lines.push("- none".to_owned());
    } else {
        for warning in &report.warnings {
            lines.push(format!("- `{warning}`"));
        }
    }
    lines.push(String::new());
    lines.push("## Review Reasons".to_owned());
    if report.validation_proposal.review_reasons.is_empty() {
        lines.push("- none".to_owned());
    } else {
        for reason in &report.validation_proposal.review_reasons {
            lines.push(format!("- `{reason}`"));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn validate_request_scope(request: &KgWritebackProposalRequest) -> Result<()> {
    validate_non_empty("tenant_id", &request.tenant_id)?;
    validate_non_empty("namespace", &request.namespace)?;
    validate_did("requesting_agent_did", &request.requesting_agent_did)?;
    if request.context_packet.schema_version != KG_CONTEXT_PACKET_PREVIEW_SCHEMA {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "context packet preview schema is unsupported".to_owned(),
        });
    }
    if !request.context_packet.dry_run_or_preview_only {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "context packet must be preview-only for this dry-run phase".to_owned(),
        });
    }
    if request.context_packet.tenant_id != request.tenant_id
        || request.context_packet.namespace != request.namespace
    {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "context packet tenant/namespace does not match request".to_owned(),
        });
    }
    if request.hint.parent_context_packet_id != request.context_packet.context_packet_id {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "parent_context_packet_id does not match context packet".to_owned(),
        });
    }
    if request.hint.route_hint_id != request.context_packet.route_hint_id {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "route_hint_id does not match context packet".to_owned(),
        });
    }
    for existing in &request.existing_memory {
        hash_from_hex("existing.memory_id", &existing.memory_id)?;
        hash_from_hex("existing.payload_hash", &existing.payload_hash)?;
        validate_non_empty("existing.summary", &existing.summary)?;
    }
    Ok(())
}

fn validate_placement_proposal(report: &KgWritebackDryRunReport) -> Result<()> {
    if report.placement_proposal.input_memory_id != report.candidate_id {
        return Err(KgWritebackError::InvalidHint {
            reason: "placement input_memory_id must match candidate_id".to_owned(),
        });
    }
    let decision = &report.placement_proposal.canonicalization_decision;
    if decision.input_memory_id != report.candidate_id {
        return Err(KgWritebackError::InvalidHint {
            reason: "canonicalization input_memory_id must match candidate_id".to_owned(),
        });
    }
    hash_from_hex("placement_decision_id", &decision.decision_id)?;
    if let Some(canonical_memory_id) = &decision.canonical_memory_id {
        hash_from_hex("canonical_memory_id", canonical_memory_id)?;
    }
    validate_unique_hash_refs("matched_memory_id", &decision.matched_memory_ids)?;
    for edge in &decision.required_edges_to_create {
        hash_from_hex("required_edge.from_memory_id", &edge.from_memory_id)?;
        hash_from_hex("required_edge.to_memory_id", &edge.to_memory_id)?;
    }
    for edge in &report.placement_proposal.edges_to_create {
        hash_from_hex("edge.from_memory_id", &edge.from_memory_id)?;
        hash_from_hex("edge.to_memory_id", &edge.to_memory_id)?;
    }
    if report.placement_trace != placement_trace_labels() {
        return Err(KgWritebackError::InvalidHint {
            reason: "placement trace does not match system-side order".to_owned(),
        });
    }
    Ok(())
}

fn validate_choice(field: &str, value: &str, allowed: &[&str]) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(KgWritebackError::InvalidHint {
            reason: format!("unsupported {field}: {value}"),
        })
    }
}

fn bind_context_evidence(
    request: &KgWritebackProposalRequest,
) -> Result<KgWritebackEvidenceBinding> {
    let mut citation_index = BTreeMap::new();
    for citation in &request.context_packet.citation_handles {
        citation_index.insert(citation.handle.clone(), citation);
    }
    let mut selected_memory_ids = BTreeSet::new();
    let mut validation_report_ids = BTreeSet::new();
    let mut graph_node_count = 0usize;
    let mut missing = Vec::new();
    for handle in &request.hint.citation_handles {
        match citation_index.get(handle) {
            Some(citation) => {
                selected_memory_ids.insert(citation.memory_id.clone());
                validation_report_ids.extend(citation.validation_report_ids.iter().cloned());
                graph_node_count = graph_node_count.saturating_add(citation.graph_node_ids.len());
            }
            None => missing.push(handle.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "citation handle was not present in parent context packet".to_owned(),
        });
    }
    if selected_memory_ids.is_empty() {
        return Err(KgWritebackError::InvalidEvidence {
            reason: "writeback must cite at least one selected memory".to_owned(),
        });
    }

    let mut citation_handles = sorted_unique(request.hint.citation_handles.clone());
    let evidence_receipts = sorted_unique(request.hint.evidence_receipts.clone());
    citation_handles.sort();
    let mut binding_reasons = vec![
        "parent_context_packet_id_match".to_owned(),
        "route_hint_id_match".to_owned(),
        "tenant_namespace_match".to_owned(),
        "citation_handles_resolved".to_owned(),
    ];
    if !evidence_receipts.is_empty() {
        binding_reasons.push("evidence_receipts_present".to_owned());
    }
    if graph_node_count > 0 {
        binding_reasons.push("citation_graph_nodes_present".to_owned());
    }
    binding_reasons.sort();
    Ok(KgWritebackEvidenceBinding {
        parent_context_packet_id: request.hint.parent_context_packet_id.clone(),
        route_hint_id: request.hint.route_hint_id.clone(),
        selected_memory_ids: selected_memory_ids.into_iter().collect(),
        citation_handles,
        evidence_receipts,
        validation_report_ids: validation_report_ids.into_iter().collect(),
        missing_citation_handles: missing,
        status: "bound".to_owned(),
        binding_reasons,
    })
}

fn writeback_candidate_id(
    request: &KgWritebackProposalRequest,
    output_hash: &str,
    evidence: &KgWritebackEvidenceBinding,
) -> Result<String> {
    writeback_candidate_id_parts(
        &request.tenant_id,
        &request.namespace,
        &request.requesting_agent_did,
        &request.hint.source_request_id,
        &request.hint.parent_context_packet_id,
        &request.hint.route_hint_id,
        &request.hint.task_hash,
        output_hash,
        request.hint.candidate_kind,
        &request.hint.summary,
        &evidence.selected_memory_ids,
        &evidence.citation_handles,
        &evidence.evidence_receipts,
    )
}

/// Deterministically derive the writeback candidate id from raw parts.
///
/// Shared by the dry-run generator and the persistence re-derivation so the two
/// paths can never drift; the persistence path re-runs this from the report's own
/// hint material and rejects any mismatch with the report's candidate_id.
#[allow(clippy::too_many_arguments)]
fn writeback_candidate_id_parts(
    tenant_id: &str,
    namespace: &str,
    requesting_agent_did: &str,
    source_request_id: &str,
    parent_context_packet_id: &str,
    route_hint_id: &str,
    task_hash: &str,
    output_hash: &str,
    candidate_kind: MemoryCandidateKind,
    summary: &str,
    selected_memory_ids: &[String],
    citation_handles: &[String],
    evidence_receipts: &[String],
) -> Result<String> {
    Ok(stable_hash(
        "exo.dagdb.kg_writeback.dry_run.candidate_id",
        &[
            tenant_id,
            namespace,
            requesting_agent_did,
            source_request_id,
            parent_context_packet_id,
            route_hint_id,
            task_hash,
            output_hash,
            &format!("{candidate_kind:?}"),
            summary,
            &selected_memory_ids.join(","),
            &citation_handles.join(","),
            &evidence_receipts.join(","),
        ],
    )?
    .to_string())
}

/// Re-derive the writeback review reasons from system-owned signals only.
///
/// The dry-run generator's [`writeback_review_reasons`] depends on the same risk
/// class and recorded placement decision plus the agent's contradiction/
/// supersession ref lists. Those ref lists are not carried in the persisted
/// report, but whenever they are non-empty the placement decision is forced to
/// `Contradiction`/`Supersession`, which already triggers
/// `placement_outcome_requires_review`. So the report's own claimed ref reasons
/// are accepted only when they are consistent with that placement outcome; the
/// system-required reasons (risk + placement) are always enforced. This rejects a
/// forged "no review needed" report while accepting an honest one.
fn system_writeback_review_reasons(
    risk_class: RiskClass,
    placement_decision_kind: CanonicalizationDecisionKind,
    reported_review_reasons: &[String],
) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    if matches!(risk_class, RiskClass::R3 | RiskClass::R4 | RiskClass::R5) {
        reasons.insert("high_risk_writeback_requires_council_path".to_owned());
    }
    let placement_requires_review = matches!(
        placement_decision_kind,
        CanonicalizationDecisionKind::Contradiction
            | CanonicalizationDecisionKind::Supersession
            | CanonicalizationDecisionKind::Replacement
            | CanonicalizationDecisionKind::RejectedNeedsReview
    );
    if placement_requires_review {
        reasons.insert("placement_outcome_requires_review".to_owned());
        // The ref-derived reasons co-occur only with a review-bearing placement.
        // Honor them when the report carries them so an honest report round-trips,
        // but never when the placement outcome would not itself require review.
        for ref_reason in [
            "contradiction_refs_require_review",
            "supersession_refs_require_review",
        ] {
            if reported_review_reasons
                .iter()
                .any(|reason| reason == ref_reason)
            {
                reasons.insert(ref_reason.to_owned());
            }
        }
    }
    reasons.into_iter().collect()
}

fn run_system_side_placement(
    request: &KgWritebackProposalRequest,
    candidate_id: &str,
    output_hash: &str,
    candidate: &MemoryCandidate,
) -> Result<PlacementResult> {
    let scope = DagDbAuthorizedScope {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        actor_did: request.requesting_agent_did.clone(),
        authority_scope_hash: stable_hash(
            "exo.dagdb.kg_writeback.dry_run.authority_scope_hash",
            &[
                &request.tenant_id,
                &request.namespace,
                &request.requesting_agent_did,
            ],
        )?,
        consent_scope_hash: stable_hash(
            "exo.dagdb.kg_writeback.dry_run.consent_scope_hash",
            &[
                &request.tenant_id,
                &request.namespace,
                &request.requesting_agent_did,
            ],
        )?,
        permitted_actions: vec!["dagdb:writeback".to_owned()],
        expires_at: Timestamp::new(10_000, 0),
    };
    let gate = DomainGateContext {
        action: "dagdb:writeback".to_owned(),
        authority_scope: AuthorityScope {
            permissions: vec![Permission::Write],
            tools: Vec::new(),
            data_classes: Vec::new(),
            counterparties: Vec::new(),
            jurisdictions: Vec::new(),
        },
        consent_decision: ConsentDecision::Granted { expires: None },
    };
    let existing_memory = request
        .existing_memory
        .iter()
        .map(|existing| {
            Ok(PlacementExistingMemory {
                memory_id: hash_from_hex("existing.memory_id", &existing.memory_id)?,
                payload_hash: hash_from_hex("existing.payload_hash", &existing.payload_hash)?,
                summary: existing.summary.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let requested_decision = requested_placement_decision(&request.hint);
    MemoryPlacementController::place_memory_candidate(
        &scope,
        &gate,
        candidate,
        MemoryPlacementInput {
            tenant_id: request.tenant_id.clone(),
            namespace: request.namespace.clone(),
            input_memory_id: hash_from_hex("candidate_id", candidate_id)?,
            payload_hash: hash_from_hex("output_hash", output_hash)?,
            summary: request.hint.summary.clone(),
            risk_class: request.hint.risk_hint,
            validator_status: ValidationStatus::Pending,
            existing_memory,
            requested_decision,
            receipt_intent: "writeback_dry_run_placement".to_owned(),
            now: Timestamp::new(1_000, 0),
        },
    )
    .map_err(|source| KgWritebackError::Placement { source })
}

fn requested_placement_decision(
    hint: &KgAgentWritebackHint,
) -> Option<CanonicalizationDecisionKind> {
    if !hint.contradiction_refs.is_empty() {
        return Some(CanonicalizationDecisionKind::Contradiction);
    }
    if !hint.supersession_refs.is_empty() {
        return Some(CanonicalizationDecisionKind::Supersession);
    }
    None
}

fn writeback_review_reasons(
    request: &KgWritebackProposalRequest,
    placement: &PlacementResult,
) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    if matches!(
        request.hint.risk_hint,
        RiskClass::R3 | RiskClass::R4 | RiskClass::R5
    ) {
        reasons.insert("high_risk_writeback_requires_council_path".to_owned());
    }
    if !request.hint.contradiction_refs.is_empty() {
        reasons.insert("contradiction_refs_require_review".to_owned());
    }
    if !request.hint.supersession_refs.is_empty() {
        reasons.insert("supersession_refs_require_review".to_owned());
    }
    if matches!(
        placement.canonicalization_decision.decision_kind,
        CanonicalizationDecisionKind::Contradiction
            | CanonicalizationDecisionKind::Supersession
            | CanonicalizationDecisionKind::Replacement
            | CanonicalizationDecisionKind::RejectedNeedsReview
    ) {
        reasons.insert("placement_outcome_requires_review".to_owned());
    }
    reasons.into_iter().collect()
}

fn route_invalidation_proposals(
    request: &KgWritebackProposalRequest,
    evidence: &KgWritebackEvidenceBinding,
    review_reasons: &[String],
) -> Vec<KgWritebackRouteInvalidationProposal> {
    if review_reasons.iter().all(|reason| {
        reason != "contradiction_refs_require_review"
            && reason != "supersession_refs_require_review"
            && reason != "placement_outcome_requires_review"
    }) {
        return Vec::new();
    }
    vec![KgWritebackRouteInvalidationProposal {
        route_hint_id: request.hint.route_hint_id.clone(),
        affected_memory_ids: evidence.selected_memory_ids.clone(),
        trigger_type: "writeback_review_boundary".to_owned(),
        status: "advisory_only".to_owned(),
        persisted_route_invalidation: false,
        reason: "persisted route invalidation is deferred until route state is active".to_owned(),
    }]
}

fn writeback_warnings(request: &KgWritebackProposalRequest, needs_review: bool) -> Vec<String> {
    let mut warnings = BTreeSet::new();
    warnings.insert("writeback_dry_run_only_not_persisted".to_owned());
    warnings.insert("preview_only_not_production_route".to_owned());
    warnings.insert("writeback_persistence_deferred".to_owned());
    if request
        .context_packet
        .memory_refs
        .iter()
        .any(|memory| memory.source_path.is_none())
    {
        warnings.insert("origin_path_not_persisted".to_owned());
    }
    if needs_review {
        warnings.insert("needs_review_before_persistence".to_owned());
        warnings.insert("route_invalidation_advisory_only".to_owned());
    }
    for warning in &request.context_packet.warnings {
        if warning == "unresolved_review_items_not_active_edges" {
            warnings.insert("unresolved_review_items_not_active_edges".to_owned());
        }
    }
    warnings.into_iter().collect()
}

fn placement_trace_labels() -> Vec<String> {
    crate::graph::required_placement_steps()
        .into_iter()
        .map(|step| format!("{step:?}").to_ascii_snake_case())
        .collect()
}

trait ToAsciiSnakeCase {
    fn to_ascii_snake_case(&self) -> String;
}

impl ToAsciiSnakeCase for str {
    fn to_ascii_snake_case(&self) -> String {
        let mut output = String::new();
        for (idx, ch) in self.chars().enumerate() {
            if idx > 0 && ch.is_ascii_uppercase() {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        }
        output
    }
}

fn validate_unique_hash_refs(field: &str, values: &[String]) -> Result<()> {
    validate_unique_non_empty(field, values)?;
    for value in values {
        hash_from_hex(field, value)?;
    }
    Ok(())
}

fn validate_unique_non_empty(field: &str, values: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_non_empty(field, value)?;
        if !seen.insert(value) {
            return Err(KgWritebackError::InvalidHint {
                reason: format!("duplicate {field}"),
            });
        }
    }
    Ok(())
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("{field} must not be empty"),
        });
    }
    reject_forbidden_string(field, value)
}

/// Reject a tenant id / namespace that is not already in canonical, charset-safe
/// form (GAP-012 P1-E). Persisted rows carry this value verbatim and the by-hash
/// read predicates compare on it, so a non-canonical value must fail closed.
fn validate_tenant_identity(field: &str, value: &str) -> Result<()> {
    let normalized = crate::tenant::normalize_tenant_id(value).map_err(|error| {
        KgWritebackError::InvalidHint {
            reason: format!("{field}: {error}"),
        }
    })?;
    if normalized != value {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("{field} is not in canonical form"),
        });
    }
    Ok(())
}

fn validate_relative_layer_path(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if value.starts_with('/') || value.starts_with('~') || value.ends_with('/') {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("dangerous {field}"),
        });
    }
    if value.contains('\\') || value.contains("//") {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("dangerous {field}"),
        });
    }
    if value
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == ".." || part != part.trim())
    {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("dangerous {field}"),
        });
    }
    Ok(())
}

fn validate_did(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if !value.starts_with("did:") {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("{field} must be a DID"),
        });
    }
    Ok(())
}

fn reject_raw_body_keys(value: &JsonValue, location: &str) -> Result<()> {
    match value {
        JsonValue::Object(object) => {
            for (key, nested) in object {
                let normalized_key = key.to_ascii_lowercase();
                if RAW_BODY_KEYS.contains(&normalized_key.as_str()) {
                    return Err(KgWritebackError::InvalidHint {
                        reason: format!("raw body key {key} is not allowed at {location}"),
                    });
                }
                reject_raw_body_keys(nested, key)?;
            }
        }
        JsonValue::Array(items) => {
            for (idx, nested) in items.iter().enumerate() {
                reject_raw_body_keys(nested, &format!("{location}[{idx}]"))?;
            }
        }
        JsonValue::String(text) => reject_forbidden_string(location, text)?,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
    Ok(())
}

fn reject_forbidden_string(field: &str, value: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| lowered.contains(**fragment))
    {
        return Err(KgWritebackError::InvalidHint {
            reason: format!("{field} contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

fn usize_to_u32(field: &str, value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| KgWritebackError::InvalidHint {
        reason: format!("{field} does not fit in u32"),
    })
}

fn workspace_relative_path(relative_path: &str) -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root.join(relative_path)
}

#[cfg(test)]
mod tests {
    use exo_core::Hash256;
    use exo_dag_db_api::{SafeMetadata, SafeMetadataDecision};

    use super::*;
    use crate::kg_retrieval::{
        KgCitationDiagnostic, KgCitationHandle, KgGraphEdgeRef, KgGraphPathSummary, KgMemoryRef,
        KgOmittedMemoryRef, KgRetrievalDiagnostics, KgValidationSummary,
    };

    fn h(byte: u8) -> String {
        Hash256::from_bytes([byte; 32]).to_string()
    }

    fn safe(text: &str) -> SafeMetadata {
        SafeMetadata {
            decision: SafeMetadataDecision::Allow,
            text: text.to_owned(),
            redaction_codes: Vec::new(),
            original_hash: h(0xee),
            truncated: false,
            byte_len: u32::try_from(text.len()).expect("fixture length"),
        }
    }

    fn context_packet() -> KgContextPacketPreview {
        let memory_id = h(0x20);
        let catalog_id = h(0x21);
        let graph_node_id = h(0x22);
        let graph_edge_id = h(0x23);
        let validation_report_id = h(0x24);
        let latest_receipt_hash = h(0x25);
        let citation_handle =
            "dagdb://kg/tenant-a/primary/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned();
        let memory_ref = KgMemoryRef {
            memory_id: memory_id.clone(),
            catalog_id: Some(catalog_id.clone()),
            source_path: None,
            catalog_path: vec!["KnowledgeGraphs".to_owned(), "dag-db".to_owned()],
            layer_id: None,
            layer_path: None,
            layer_depth: None,
            layer_kind: None,
            layer_membership_role: None,
            layer_selection_reason: None,
            rollup_summary_ref: None,
            title: safe("DAG DB mission"),
            summary: safe("DAG DB improves agent context with graph governance"),
            latest_receipt_hash: latest_receipt_hash.clone(),
            memory_status: "pending".to_owned(),
            validation_status: "pending".to_owned(),
            risk_class: "R1".to_owned(),
            council_status: "not_required".to_owned(),
            dag_finality_status: "pending".to_owned(),
            graph_node_ids: vec![graph_node_id.clone()],
            validation_report_ids: vec![validation_report_id.clone()],
            citation_handle: citation_handle.clone(),
            token_estimate: 18,
            selection_reasons: vec!["selected_by_catalog_order".to_owned()],
        };
        let graph_edge = KgGraphEdgeRef {
            graph_edge_id: graph_edge_id.clone(),
            from_memory_id: memory_id.clone(),
            to_memory_id: h(0x26),
            edge_kind: "related_to".to_owned(),
            graph_style: "semantic_catalog_graph".to_owned(),
            receipt_hash: Some(latest_receipt_hash.clone()),
        };
        KgContextPacketPreview {
            schema_version: KG_CONTEXT_PACKET_PREVIEW_SCHEMA.to_owned(),
            tenant_id: "tenant-a".to_owned(),
            namespace: "primary".to_owned(),
            context_packet_id: h(0x30),
            route_hint_id: h(0x31),
            memory_refs: vec![memory_ref.clone()],
            graph_edges: vec![graph_edge.clone()],
            selected_refs: vec![memory_ref],
            selected_layers: Vec::new(),
            selected_layer_edges: Vec::new(),
            selected_graph_edges: vec![graph_edge],
            rollup_summaries: Vec::new(),
            budget_report: crate::kg_retrieval::KgLayerBudgetReport {
                max_layer_depth: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
                max_layers_selected: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
                max_nodes_per_layer: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
                max_memory_refs: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS,
                max_layer_edges: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
                selected_layer_count: 0,
                selected_layer_edge_count: 0,
                active_layer_edge_count: 0,
                excluded_demoted_layer_edge_count: 0,
                excluded_tombstoned_layer_edge_count: 0,
                selected_memory_ref_count: 1,
                selected_graph_edge_count: 1,
                depth_budget_truncated: false,
                layer_budget_truncated: false,
                node_budget_truncated: false,
                layer_edge_budget_truncated: false,
                token_budget_truncated: false,
                flat_fallback_used: true,
            },
            flat_fallback_used: true,
            citation_handles: vec![KgCitationHandle {
                handle: citation_handle.clone(),
                memory_id,
                catalog_id: Some(catalog_id),
                latest_receipt_hash: latest_receipt_hash.clone(),
                graph_node_ids: vec![graph_node_id.clone()],
                graph_edge_ids: vec![graph_edge_id.clone()],
                validation_report_ids: vec![validation_report_id.clone()],
            }],
            retrieval_diagnostics: KgRetrievalDiagnostics {
                selected_memory_count: 1,
                omitted_memory_count: 0,
                selected_graph_edge_count: 1,
                selected_layer_count: 0,
                selected_layer_edge_count: 0,
                active_layer_edge_count: 0,
                excluded_demoted_layer_edge_count: 0,
                excluded_tombstoned_layer_edge_count: 0,
                citation_handle_count: 1,
                warning_count: 2,
                token_budget: 128,
                token_estimate: 18,
                max_layer_depth: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
                max_layers_selected: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
                max_nodes_per_layer: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
                max_layer_edges: crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
                layer_path_filter_applied: false,
                max_memory_refs_applied: false,
                catalog_path_filter_applied: false,
                requested_memory_filter_applied: false,
                flat_fallback_used: true,
                depth_budget_truncated: false,
                layer_budget_truncated: false,
                node_budget_truncated: false,
                layer_edge_budget_truncated: false,
                deterministic_ordering: true,
                raw_markdown_returned: false,
                preview_only: true,
            },
            validation_summary: KgValidationSummary {
                selected_memory_count: 1,
                pending_count: 1,
                passed_count: 0,
                failed_count: 0,
                needs_council_count: 0,
                warning_count: 0,
                validation_status_counts: BTreeMap::new(),
                risk_class_counts: BTreeMap::new(),
                dag_finality_status_counts: BTreeMap::new(),
                council_status_counts: BTreeMap::new(),
            },
            graph_path_summary: KgGraphPathSummary {
                graph_edge_count: 1,
                graph_styles_seen: vec!["semantic_catalog_graph".to_owned()],
                edge_kinds_seen: vec!["related_to".to_owned()],
                isolated_memory_count: 0,
                connected_memory_count: 1,
                missing_edge_warning_count: 0,
            },
            citation_diagnostics: vec![KgCitationDiagnostic {
                citation_handle: citation_handle.clone(),
                memory_id: h(0x20),
                catalog_id: Some(h(0x21)),
                validation_report_id: Some(validation_report_id),
                latest_receipt_hash: Some(latest_receipt_hash),
                graph_node_ids: vec![graph_node_id],
                graph_edge_ids: vec![graph_edge_id],
                citation_status: "available".to_owned(),
                reason: "fixture".to_owned(),
            }],
            token_budget: 128,
            token_estimate: 18,
            omitted_memory_ids: Vec::new(),
            omitted_memory_refs: Vec::<KgOmittedMemoryRef>::new(),
            warnings: vec![
                "preview_only_not_production_route".to_owned(),
                "unresolved_review_items_not_active_edges".to_owned(),
            ],
            dry_run_or_preview_only: true,
        }
    }

    fn hint(context: &KgContextPacketPreview) -> KgAgentWritebackHint {
        KgAgentWritebackHint {
            source_request_id: "request-1".to_owned(),
            parent_context_packet_id: context.context_packet_id.clone(),
            route_hint_id: context.route_hint_id.clone(),
            task_hash: h(0x40),
            answer_hash: None,
            output_hash: Some(h(0x41)),
            candidate_kind: MemoryCandidateKind::Summary,
            summary: "Agents should remember the graph-governed writeback boundary".to_owned(),
            citation_handles: vec![context.citation_handles[0].handle.clone()],
            evidence_receipts: vec![h(0x25)],
            risk_hint: RiskClass::R1,
            allowed_future_uses: vec![MemoryCandidateUse::Routing, MemoryCandidateUse::Audit],
            reason_to_remember: "future tasks need the dry-run writeback contract".to_owned(),
            keyword_texts: vec!["writeback".to_owned(), "governance".to_owned()],
            contradiction_refs: Vec::new(),
            supersession_refs: Vec::new(),
        }
    }

    fn hint_json(context: &KgContextPacketPreview) -> JsonValue {
        serde_json::json!({
            "source_request_id": "request-1",
            "parent_context_packet_id": context.context_packet_id.clone(),
            "route_hint_id": context.route_hint_id.clone(),
            "task_hash": h(0x40),
            "output_hash": h(0x41),
            "candidate_kind": "summary",
            "summary": "safe compact memory",
            "citation_handles": [context.citation_handles[0].handle.clone()],
            "evidence_receipts": [h(0x25)],
            "risk_hint": "R1",
            "allowed_future_uses": ["routing"],
            "reason_to_remember": "routing needs it"
        })
    }

    fn request() -> KgWritebackProposalRequest {
        let context = context_packet();
        KgWritebackProposalRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "primary".to_owned(),
            requesting_agent_did: "did:exo:agent".to_owned(),
            hint: hint(&context),
            context_packet: context,
            existing_memory: Vec::new(),
        }
    }

    fn report() -> KgWritebackDryRunReport {
        build_writeback_dry_run_report(request()).expect("report")
    }

    fn layered_writeback() -> KgWritebackLayeredWriteback {
        KgWritebackLayeredWriteback {
            target_layer_id: h(0x80),
            target_layer_path: "root/repository/source-request-1".to_owned(),
            target_layer_depth: 2,
            target_layer_kind: "task_subgraph".to_owned(),
            target_graph_style: "canonical_memory_graph".to_owned(),
            target_layer_reason: "parent_child_source_parent_path_child".to_owned(),
            parent_layer_id: Some(h(0x81)),
            parent_graph_node_id: Some(h(0x82)),
            created_child_layer_id: Some(h(0x80)),
            layer_membership_id: h(0x83),
            membership_role: "member".to_owned(),
            local_node_rank: 0,
            layer_edge_id: Some(h(0x84)),
            layer_edge_kind: Some("contains_subgraph".to_owned()),
            layer_fallback_used: false,
        }
    }

    fn assert_invalid_report(report: KgWritebackDryRunReport) {
        assert!(report.validate_for_persistence().is_err());
        let json = serde_json::to_string(&report).expect("serialize report");
        assert!(KgWritebackDryRunReport::parse_json(&json).is_err());
    }

    #[test]
    fn writeback_proposal_report_is_deterministic_and_raw_free() {
        let first = build_writeback_dry_run_report(request()).expect("report");
        let second = build_writeback_dry_run_report(request()).expect("report");
        assert_eq!(first, second);
        assert_eq!(first.schema_version, KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA);
        assert!(first.dry_run_only);
        assert!(!first.postgres_writes);
        assert!(!first.raw_markdown_included);
        assert_eq!(first.evidence_binding.status, "bound");
        assert_eq!(first.evidence_binding.selected_memory_ids, vec![h(0x20)]);
        assert_eq!(
            first
                .placement_proposal
                .canonicalization_decision
                .decision_kind,
            CanonicalizationDecisionKind::NewCanonical
        );
        first.validate_for_persistence().expect("valid report");
        assert!(!first.idempotency_key().expect("idempotency").is_empty());
        let json = serde_json::to_string(&first).expect("serialize report");
        assert_eq!(
            KgWritebackDryRunReport::parse_json(&json).expect("parse report"),
            first
        );
        assert!(!json.contains("# private body"));
        assert!(!json.contains("private_payload"));
        assert!(json.contains("writeback_dry_run_only_not_persisted"));
    }

    #[test]
    fn writeback_hint_rejects_graph_allocation_and_raw_payload_fields() {
        let context = context_packet();
        let graph_allocation = serde_json::json!({
            "source_request_id": "request-1",
            "parent_context_packet_id": context.context_packet_id.clone(),
            "route_hint_id": context.route_hint_id.clone(),
            "task_hash": h(0x40),
            "output_hash": h(0x41),
            "candidate_kind": "summary",
            "summary": "safe compact memory",
            "citation_handles": [context.citation_handles[0].handle.clone()],
            "evidence_receipts": [h(0x25)],
            "risk_hint": "R1",
            "allowed_future_uses": ["routing"],
            "reason_to_remember": "routing needs it",
            "graph_node_id": h(0x99)
        });
        assert!(parse_agent_writeback_hint_json(&graph_allocation.to_string()).is_err());

        let mut raw_payload = hint_json(&context);
        raw_payload["raw_markdown"] = serde_json::json!("# private body");
        assert!(parse_agent_writeback_hint_json(&raw_payload.to_string()).is_err());

        let mut mixed_case_raw_payload = hint_json(&context);
        mixed_case_raw_payload["Raw_Private_Payload"] = serde_json::json!("forbidden");
        assert!(parse_agent_writeback_hint_json(&mixed_case_raw_payload.to_string()).is_err());

        let mut local_path_value = hint_json(&context);
        local_path_value["summary"] = serde_json::json!("see /Users/example/.env");
        assert!(parse_agent_writeback_hint_json(&local_path_value.to_string()).is_err());

        let mut secret_value = hint_json(&context);
        secret_value["summary"] = serde_json::json!("token sk-proj-example");
        assert!(parse_agent_writeback_hint_json(&secret_value.to_string()).is_err());

        let mut whitespace_value = hint(&context);
        whitespace_value.summary = " \n\t".to_owned();
        assert!(whitespace_value.validate().is_err());

        for fragment in FORBIDDEN_VALUE_FRAGMENTS {
            let mut forbidden_value = hint_json(&context);
            forbidden_value["summary"] =
                serde_json::json!(format!("unsafe {}", fragment.to_ascii_uppercase()));
            assert!(
                parse_agent_writeback_hint_json(&forbidden_value.to_string()).is_err(),
                "fragment {fragment}"
            );
        }
    }

    #[test]
    fn writeback_rejects_review_named_forbidden_values_and_raw_alias_keys() {
        let context = context_packet();

        for forbidden_value in [
            "~/dagdb",
            "/home/example/dagdb.md",
            r"C:\Users\example\dagdb.md",
            "DB_URL=redacted",
            "Bearer abc123",
            "source_excerpt leaked",
        ] {
            let mut unsafe_hint = hint_json(&context);
            unsafe_hint["summary"] = serde_json::json!(forbidden_value);
            assert!(
                parse_agent_writeback_hint_json(&unsafe_hint.to_string()).is_err(),
                "expected forbidden value to fail: {forbidden_value}"
            );
        }

        for raw_key in [
            "source_body",
            "document_body",
            "prompt_body",
            "model_output",
        ] {
            let mut unsafe_hint = hint_json(&context);
            unsafe_hint[raw_key] = serde_json::json!("leaked raw material");
            assert!(
                parse_agent_writeback_hint_json(&unsafe_hint.to_string()).is_err(),
                "expected raw key to fail: {raw_key}"
            );
        }
    }

    #[test]
    fn writeback_parse_errors_are_loud() {
        assert!(matches!(
            parse_agent_writeback_hint_json("{"),
            Err(KgWritebackError::InvalidJson { .. })
        ));
        assert!(matches!(
            KgWritebackDryRunReport::parse_json("{"),
            Err(KgWritebackError::InvalidJson { .. })
        ));

        let mut unsafe_report = serde_json::to_value(report()).expect("report json");
        unsafe_report["nested"] = serde_json::json!([{ "Raw_Private_Payload": "forbidden" }]);
        assert!(KgWritebackDryRunReport::parse_json(&unsafe_report.to_string()).is_err());
    }

    #[test]
    fn writeback_hint_validation_rejects_hash_and_scope_edges() {
        let context = context_packet();
        let mut candidate = hint(&context);
        candidate.answer_hash = Some(h(0x41));
        candidate.output_hash = Some(h(0x42));
        assert!(candidate.validate().is_err());

        candidate.output_hash = None;
        assert_eq!(
            candidate.normalized_output_hash().expect("answer hash"),
            h(0x41)
        );

        candidate.answer_hash = None;
        assert!(candidate.validate().is_err());

        let mut candidate = hint(&context);
        candidate.allowed_future_uses.clear();
        assert!(candidate.validate().is_err());

        let mut candidate = hint(&context);
        candidate.citation_handles.clear();
        candidate.evidence_receipts.clear();
        assert!(candidate.validate().is_err());

        let mut candidate = hint(&context);
        candidate
            .citation_handles
            .push(candidate.citation_handles[0].clone());
        assert!(candidate.validate().is_err());

        let mut candidate = hint(&context);
        candidate.evidence_receipts = vec!["not-a-hash".to_owned()];
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn writeback_proposal_fails_closed_for_missing_or_cross_scope_evidence() {
        let mut missing_citation = request();
        missing_citation.hint.citation_handles = vec!["dagdb://missing".to_owned()];
        assert!(matches!(
            build_writeback_dry_run_report(missing_citation),
            Err(KgWritebackError::InvalidEvidence { .. })
        ));

        let mut cross_scope = request();
        cross_scope.context_packet.namespace = "other".to_owned();
        assert!(matches!(
            build_writeback_dry_run_report(cross_scope),
            Err(KgWritebackError::InvalidEvidence { .. })
        ));

        let mut bad_schema = request();
        bad_schema.context_packet.schema_version = "wrong".to_owned();
        assert!(build_writeback_dry_run_report(bad_schema).is_err());

        let mut not_preview = request();
        not_preview.context_packet.dry_run_or_preview_only = false;
        assert!(build_writeback_dry_run_report(not_preview).is_err());

        let mut parent_mismatch = request();
        parent_mismatch.hint.parent_context_packet_id = h(0x99);
        assert!(build_writeback_dry_run_report(parent_mismatch).is_err());

        let mut route_mismatch = request();
        route_mismatch.hint.route_hint_id = h(0x98);
        assert!(build_writeback_dry_run_report(route_mismatch).is_err());

        let mut bad_existing_hash = request();
        bad_existing_hash.existing_memory = vec![KgWritebackExistingMemory {
            memory_id: "not-a-hash".to_owned(),
            payload_hash: h(0x41),
            summary: "older memory".to_owned(),
        }];
        assert!(build_writeback_dry_run_report(bad_existing_hash).is_err());

        let mut bad_existing_summary = request();
        bad_existing_summary.existing_memory = vec![KgWritebackExistingMemory {
            memory_id: h(0x55),
            payload_hash: h(0x41),
            summary: " ".to_owned(),
        }];
        assert!(build_writeback_dry_run_report(bad_existing_summary).is_err());
    }

    #[test]
    fn writeback_report_validation_rejects_dry_run_and_acceptance_drift() {
        let mut invalid = report();
        invalid.schema_version = "wrong".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.dry_run_only = false;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.postgres_writes = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.raw_markdown_included = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.preview_only = false;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.dry_run_only = false;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.writeback_persistence_implemented = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.export_persistence_implemented = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.gateway_exposure_implemented = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.graph_explorer_changes_implemented = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.exo_dag_tables_mutated = true;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.acceptance.raw_markdown_returned = true;
        assert_invalid_report(invalid);
    }

    #[test]
    fn writeback_report_validation_rejects_mismatched_sections() {
        let mut invalid = report();
        invalid.proposed_memory_candidate.source_request_id = "other".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.proposed_memory_candidate.parent_context_packet_id = h(0x99);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.proposed_memory_candidate.full_output_hash = h(0x98);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.proposed_memory_candidate.summary = String::new();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.evidence_binding.parent_context_packet_id = h(0x97);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.evidence_binding.route_hint_id = h(0x96);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.evidence_binding.status = "pending".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.evidence_binding.selected_memory_ids.clear();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.evidence_binding.missing_citation_handles = vec!["dagdb://missing".to_owned()];
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid
            .evidence_binding
            .citation_handles
            .push(invalid.evidence_binding.citation_handles[0].clone());
        assert_invalid_report(invalid);
    }

    #[test]
    fn writeback_report_validation_rejects_validation_receipt_and_placement_drift() {
        let mut invalid = report();
        invalid.validation_proposal.validation_status = "failed".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.validation_proposal.decision = "approved".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.validation_proposal.needs_review = true;
        invalid.validation_proposal.review_reasons.clear();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.validation_proposal.review_reasons = vec!["same".to_owned(), "same".to_owned()];
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.receipt_intent_proposal.subject_id = h(0x95);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.receipt_intent_proposal.event_type = "memory_approved".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.receipt_intent_proposal.status = "written".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.receipt_intent_proposal.dry_run_only = false;
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.placement_proposal.input_memory_id = h(0x94);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid
            .placement_proposal
            .canonicalization_decision
            .input_memory_id = h(0x93);
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.placement_trace.push("extra_step".to_owned());
        assert_invalid_report(invalid);
    }

    #[test]
    fn writeback_report_validation_accepts_and_rejects_layered_writeback_boundaries() {
        let mut valid = report();
        valid.layered_writeback = Some(layered_writeback());
        valid
            .validate_for_persistence()
            .expect("layered writeback report");
        let json = serde_json::to_string(&valid).expect("serialize layered report");
        assert_eq!(
            KgWritebackDryRunReport::parse_json(&json).expect("parse layered report"),
            valid
        );

        let mut invalid = valid.clone();
        invalid
            .layered_writeback
            .as_mut()
            .expect("layered")
            .parent_layer_id = None;
        assert_invalid_report(invalid);

        let mut invalid = valid.clone();
        invalid
            .layered_writeback
            .as_mut()
            .expect("layered")
            .target_layer_path = "/root/repository".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = valid.clone();
        invalid
            .layered_writeback
            .as_mut()
            .expect("layered")
            .target_layer_depth = crate::layered_placement::LAYER_PLACEMENT_MAX_DEPTH + 1;
        assert_invalid_report(invalid);

        let mut invalid = valid.clone();
        invalid
            .layered_writeback
            .as_mut()
            .expect("layered")
            .target_layer_kind = "root".to_owned();
        assert_invalid_report(invalid);

        let mut invalid = report();
        invalid.layered_writeback = Some(KgWritebackLayeredWriteback {
            target_layer_id: h(0x85),
            target_layer_path: "root".to_owned(),
            target_layer_depth: 0,
            target_layer_kind: "root".to_owned(),
            target_graph_style: "canonical_memory_graph".to_owned(),
            target_layer_reason: "ambiguous_source_visible_root_fallback".to_owned(),
            parent_layer_id: Some(h(0x81)),
            parent_graph_node_id: Some(h(0x82)),
            created_child_layer_id: Some(h(0x85)),
            layer_membership_id: h(0x86),
            membership_role: "root".to_owned(),
            local_node_rank: 0,
            layer_edge_id: Some(h(0x87)),
            layer_edge_kind: Some("contains_subgraph".to_owned()),
            layer_fallback_used: true,
        });
        assert_invalid_report(invalid);
    }

    #[test]
    fn writeback_duplicate_and_contradiction_boundaries_are_system_side() {
        let mut duplicate = request();
        duplicate.existing_memory = vec![KgWritebackExistingMemory {
            memory_id: h(0x55),
            payload_hash: h(0x41),
            summary: "older memory".to_owned(),
        }];
        let duplicate_report = build_writeback_dry_run_report(duplicate).expect("duplicate");
        duplicate_report
            .validate_for_persistence()
            .expect("duplicate report remains valid");
        assert_eq!(
            duplicate_report
                .placement_proposal
                .canonicalization_decision
                .decision_kind,
            CanonicalizationDecisionKind::ExactDuplicate
        );
        assert_eq!(
            duplicate_report.placement_proposal.edges_to_create[0].to_memory_id,
            h(0x55)
        );

        let mut contradiction = request();
        contradiction.existing_memory = vec![KgWritebackExistingMemory {
            memory_id: h(0x56),
            payload_hash: h(0x57),
            summary: "graph governed writeback boundary".to_owned(),
        }];
        contradiction.hint.contradiction_refs = vec![h(0x56)];
        let contradiction_report =
            build_writeback_dry_run_report(contradiction).expect("contradiction");
        contradiction_report
            .validate_for_persistence()
            .expect("contradiction report remains valid");
        assert_eq!(
            contradiction_report
                .placement_proposal
                .canonicalization_decision
                .decision_kind,
            CanonicalizationDecisionKind::Contradiction
        );
        assert!(contradiction_report.proposal_summary.needs_review);
        assert!(!contradiction_report.proposed_route_invalidations[0].persisted_route_invalidation);
        let contradiction_md = writeback_markdown_summary(&contradiction_report);
        assert!(contradiction_md.contains("contradiction_refs_require_review"));

        let mut supersession = request();
        supersession.existing_memory = vec![KgWritebackExistingMemory {
            memory_id: h(0x58),
            payload_hash: h(0x59),
            summary: "older graph governed writeback boundary".to_owned(),
        }];
        supersession.hint.supersession_refs = vec![h(0x58)];
        let supersession_report =
            build_writeback_dry_run_report(supersession).expect("supersession");
        assert_eq!(
            supersession_report
                .placement_proposal
                .canonicalization_decision
                .decision_kind,
            CanonicalizationDecisionKind::Supersession
        );
        assert!(supersession_report.proposal_summary.needs_review);
    }

    #[test]
    fn writeback_persistence_rejects_forged_validation_outcome() {
        // A system-consistent report passes the re-derivation.
        report()
            .validate_for_persistence()
            .expect("system-consistent report is accepted");

        // Forging validation_status to "passed" without changing needs_review is
        // caught by the self-consistency check; forging the full no-review tuple on
        // a review-bearing report is caught by the system re-derivation. Build a
        // review-bearing report, then forge it to claim no review is needed.
        let mut contradiction = request();
        contradiction.existing_memory = vec![KgWritebackExistingMemory {
            memory_id: h(0x56),
            payload_hash: h(0x57),
            summary: "graph governed writeback boundary".to_owned(),
        }];
        contradiction.hint.contradiction_refs = vec![h(0x56)];
        let review_report =
            build_writeback_dry_run_report(contradiction).expect("contradiction report");
        review_report
            .validate_for_persistence()
            .expect("honest review report is accepted");
        assert!(review_report.validation_proposal.needs_review);

        // Forge the entire validation outcome to "passed/allow/no-review". The
        // placement decision still records Contradiction, so the system re-derives
        // needs_review and rejects the forgery.
        let mut forged = review_report.clone();
        forged.validation_proposal.validation_status = "passed".to_owned();
        forged.validation_proposal.decision = "allow".to_owned();
        forged.validation_proposal.needs_review = false;
        forged.validation_proposal.review_reasons = Vec::new();
        forged.proposal_summary.needs_review = false;
        assert!(
            matches!(
                forged.validate_for_persistence(),
                Err(KgWritebackError::InvalidHint { .. })
            ),
            "forged no-review report on a review-bearing placement must be rejected"
        );
        let json = serde_json::to_string(&forged).expect("serialize forged report");
        assert!(KgWritebackDryRunReport::parse_json(&json).is_err());
    }

    #[test]
    fn writeback_persistence_rejects_mismatched_candidate_id() {
        // A candidate_id that does not match the system-derived hint material is
        // rejected even though the report is otherwise self-consistent in shape.
        let mut forged = report();
        forged.candidate_id = h(0x77);
        // Keep the placement input bound to the forged id so the earlier
        // self-consistency check passes and the system re-derivation is reached.
        forged.placement_proposal.input_memory_id = h(0x77);
        forged
            .placement_proposal
            .canonicalization_decision
            .input_memory_id = h(0x77);
        forged.receipt_intent_proposal.subject_id = h(0x77);
        assert!(
            matches!(
                forged.validate_for_persistence(),
                Err(KgWritebackError::InvalidHint { .. })
            ),
            "candidate_id not derivable from hint material must be rejected"
        );
    }

    #[test]
    fn writeback_default_artifacts_are_deterministic_review_outputs() {
        let report = build_writeback_dry_run_report(request()).expect("report");
        let artifacts = write_default_writeback_dry_run_artifacts(&report).expect("write");
        assert!(
            artifacts
                .output_json
                .ends_with(KG_WRITEBACK_DRY_RUN_JSON_PATH)
        );
        assert!(artifacts.output_md.ends_with(KG_WRITEBACK_DRY_RUN_MD_PATH));
        let json = fs::read_to_string(artifacts.output_json).expect("json artifact");
        let md = fs::read_to_string(artifacts.output_md).expect("md artifact");
        assert!(json.contains(KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA));
        assert!(md.contains(KG_WRITEBACK_DRY_RUN_SUMMARY_SCHEMA));
        assert!(!json.contains("# private body"));
        assert!(!md.contains("# private body"));
    }
}
