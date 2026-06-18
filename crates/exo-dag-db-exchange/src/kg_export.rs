//! Portable KG export report contracts.
//!
//! This module defines repository-level export artifacts and compact persistence
//! summaries only. It does not expose a gateway route, activate production
//! routes, write route invalidations, mutate graph explorer state, create
//! migrations, persist raw artifacts, or write `exo-dag` tables.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error as StdError,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::{
    kg_import::{KgImportError, stable_hash},
    scoring::hash_event_body,
};

/// Portable export schema emitted by the repository-level report adapter.
pub const KG_PORTABLE_EXPORT_SCHEMA: &str = "dagdb_kg_portable_export_v1";
/// Summary schema for generated Markdown export reports.
pub const KG_PORTABLE_EXPORT_SUMMARY_SCHEMA: &str = "dagdb_kg_portable_export_summary_v1";
/// Route name used only for deterministic export hash material.
pub const KG_EXPORT_REPORT_ROUTE_NAME: &str = "dagdb.kg_export.report.v1";
/// Route name used by the feature-gated repository export persistence adapter.
pub const KG_EXPORT_PERSISTED_ROUTE_NAME: &str = "dagdb.kg_export.persisted.v1";
/// Route name used by the feature-gated export finality/outbox adapter.
pub const KG_EXPORT_FINALITY_OUTBOX_ROUTE_NAME: &str = "dagdb.kg_export.finality_outbox.v1";
/// Schema for compact persisted export summaries stored in idempotency responses.
pub const KG_EXPORT_PERSISTED_SUMMARY_SCHEMA: &str = "dagdb_kg_export_persisted_summary_v1";
/// Schema for deterministic read-back verification of persisted export rows.
pub const KG_EXPORT_PERSISTENCE_VERIFICATION_SCHEMA: &str =
    "dagdb_kg_export_persistence_verification_v1";
/// Schema for compact export finality/outbox repository summaries.
pub const KG_EXPORT_FINALITY_OUTBOX_SUMMARY_SCHEMA: &str =
    "dagdb_kg_export_finality_outbox_summary_v1";
/// Environment variable used by Postgres-gated export tests and helpers.
pub const KG_EXPORT_DATABASE_URL_ENV: &str = "EXO_DAGDB_TEST_DATABASE_URL";
/// Default target directory for generated export report artifacts.
pub const KG_EXPORT_TARGET_DIR: &str = "target/dagdb/kg_export";
/// Default JSON report path for generated export report artifacts.
pub const KG_EXPORT_JSON_PATH: &str = "target/dagdb/kg_export/report.json";
/// Default Markdown summary path for generated export report artifacts.
pub const KG_EXPORT_MD_PATH: &str = "target/dagdb/kg_export/summary.md";

/// Source error carried by DB-backed export adapters without a reverse Postgres dependency.
pub type KgExportSourceError = Box<dyn StdError + Send + Sync + 'static>;

const FORBIDDEN_KEYS: &[&str] = &[
    "body",
    "content",
    "database_url",
    "db_url",
    "document_body",
    "file_path",
    "file_text",
    "gateway_secret",
    "markdown",
    "model_output",
    "payload",
    "private_key",
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
    "source_path",
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
    "database_url",
    "db_url",
    ".env",
    "bearer ",
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
    "raw_private_payload",
    "raw_markdown",
    "raw_markdown_body",
    "raw_model_output",
    "raw_payload",
    "raw_prompt_body",
    "source_excerpt",
];

/// Record map used for safe export sections.
pub type KgExportRecord = BTreeMap<String, JsonValue>;

/// Export scope for one tenant/namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportScope {
    pub tenant_id: String,
    pub namespace: String,
    #[serde(default)]
    pub included_memory_ids: Vec<String>,
    #[serde(default)]
    pub included_graph_styles: Vec<String>,
    #[serde(default)]
    pub included_writeback_idempotency_keys: Vec<String>,
    pub source_commit_or_repo_ref: Option<String>,
    pub include_preview_context: bool,
}

impl KgExportScope {
    /// Validate tenant/namespace scope and deterministic filters.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty("tenant_id", &self.tenant_id)?;
        validate_non_empty("namespace", &self.namespace)?;
        validate_unique("included_memory_ids", &self.included_memory_ids)?;
        validate_unique("included_graph_styles", &self.included_graph_styles)?;
        validate_unique(
            "included_writeback_idempotency_keys",
            &self.included_writeback_idempotency_keys,
        )?;
        if let Some(source_ref) = &self.source_commit_or_repo_ref {
            validate_non_empty("source_commit_or_repo_ref", source_ref)?;
            reject_forbidden_string("source_commit_or_repo_ref", source_ref)?;
        }
        Ok(())
    }
}

/// Deterministic HLC marker used for dry-run/report artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportHlc {
    pub physical_ms: u64,
    pub logical: u32,
}

/// Portable export artifact assembled from current-schema rows and previews.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgPortableExport {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub export_id: String,
    pub export_scope: KgExportScope,
    pub created_at_or_hlc: KgExportHlc,
    pub source_commit_or_repo_ref: Option<String>,
    pub memory_records: Vec<KgExportRecord>,
    pub catalog_entries: Vec<KgExportRecord>,
    pub graph_nodes: Vec<KgExportRecord>,
    pub graph_edges: Vec<KgExportRecord>,
    pub similarity_results: Vec<KgExportRecord>,
    pub canonicalization_decisions: Vec<KgExportRecord>,
    pub placement_traces: Vec<KgExportRecord>,
    pub validation_reports: Vec<KgExportRecord>,
    pub receipts: Vec<KgExportRecord>,
    pub subject_receipt_heads: Vec<KgExportRecord>,
    pub context_packet_previews: Vec<KgExportRecord>,
    pub context_packet_records: Vec<KgExportRecord>,
    pub route_receipts: Vec<KgExportRecord>,
    pub writeback_summaries: Vec<KgExportRecord>,
    pub idempotency_references: Vec<KgExportRecord>,
    pub citation_index: Vec<KgExportRecord>,
    pub provenance_index: Vec<KgExportRecord>,
    pub advisory_sections: Vec<KgExportRecord>,
    pub redaction_summary: KgExportRecord,
    pub omission_summary: KgExportRecord,
    pub diagnostics: KgExportDiagnostics,
    pub hashes: KgExportHashes,
    pub verification: KgExportVerification,
    pub acceptance: KgExportAcceptance,
}

/// Deterministic diagnostics derived from exported rows and advisory sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportDiagnostics {
    pub section_counts: BTreeMap<String, u32>,
    pub section_hashes: BTreeMap<String, String>,
    pub citation_diagnostics: KgExportCitationDiagnostics,
    pub provenance_diagnostics: KgExportProvenanceDiagnostics,
    pub redaction_omission_diagnostics: KgExportRedactionOmissionDiagnostics,
    pub advisory_deferred_diagnostics: KgExportAdvisoryDeferredDiagnostics,
    pub deterministic_ordering: bool,
    pub raw_material_exclusion_enforced: bool,
    pub tenant_namespace_scoped: bool,
    pub preview_only_context_count: u32,
}

/// Citation coverage diagnostics for the portable export report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportCitationDiagnostics {
    pub citation_handle_count: u32,
    pub memory_coverage_count: u32,
    pub validation_report_coverage_count: u32,
    pub receipt_coverage_count: u32,
    pub graph_edge_coverage_count: u32,
    pub partial_coverage_count: u32,
    pub missing_coverage_count: u32,
}

/// Provenance coverage diagnostics for exported rows and preview evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportProvenanceDiagnostics {
    pub memory_provenance_count: u32,
    pub validation_provenance_count: u32,
    pub receipt_provenance_count: u32,
    pub missing_latest_receipt_count: u32,
    pub preview_only_provenance_count: u32,
}

/// Redaction and omission diagnostics for raw-body/private-payload safety.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportRedactionOmissionDiagnostics {
    pub markdown_body_content_excluded: bool,
    pub private_payload_content_excluded: bool,
    pub model_output_content_excluded: bool,
    pub source_excerpts_excluded: bool,
    pub source_path_omission_count: u32,
    pub source_path_omission_reason: String,
    pub database_connection_values_excluded: bool,
    pub gateway_secrets_excluded: bool,
    pub private_keys_excluded: bool,
    pub local_absolute_paths_excluded: bool,
}

/// Advisory/deferred status diagnostics for non-authoritative export sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportAdvisoryDeferredDiagnostics {
    pub advisory_section_count: u32,
    pub deferred_section_count: u32,
    pub omitted_section_count: u32,
    pub route_invalidation_advisory_count: u32,
    pub export_persistence_deferred: bool,
    pub gateway_api_deferred: bool,
    pub graph_explorer_deferred: bool,
    pub production_route_activation_deferred: bool,
    pub route_invalidation_writes_deferred: bool,
    pub exo_dag_writes_deferred: bool,
}

/// Section and whole-export hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportHashes {
    pub section_hashes: BTreeMap<String, String>,
    pub export_id_material_hash: String,
    pub whole_export_hash: String,
}

/// Non-authoritative report verification flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportVerification {
    pub deterministic_ordering: bool,
    pub body_payload_exclusion_enforced: bool,
    pub tenant_namespace_scoped: bool,
    pub preview_context_marked_preview_only: bool,
    pub route_invalidation_writes_implemented: bool,
    pub export_persistence_implemented: bool,
    pub gateway_api_exposure_implemented: bool,
    pub graph_explorer_changes_implemented: bool,
    pub production_route_activation_implemented: bool,
    pub exo_dag_tables_mutated: bool,
}

/// Acceptance flags for this bounded export phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportAcceptance {
    pub report_only: bool,
    pub export_persistence_implemented: bool,
    pub gateway_api_exposure_implemented: bool,
    pub graph_explorer_changes_implemented: bool,
    pub production_route_activation_implemented: bool,
    pub route_invalidation_writes_implemented: bool,
    pub exo_dag_tables_mutated: bool,
}

/// Compact repository-level summary returned after export persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedSummary {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub export_id: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub replayed: bool,
    pub export_status: String,
    pub whole_export_hash: String,
    pub latest_receipt_hash: Option<String>,
    pub inserted_export_count: u32,
    pub inserted_challenge_count: u32,
    pub inserted_receipt_count: u32,
    pub inserted_subject_receipt_head_count: u32,
    pub inserted_idempotency_response_count: u32,
    pub persisted_route_invalidation_count: u32,
    pub persisted_dag_outbox_count: u32,
    pub persisted_raw_artifact_count: u32,
    pub persisted_exo_dag_write_count: u32,
    pub diagnostics: KgExportPersistedDiagnostics,
}

/// Repository-level read-back verification for persisted export rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistenceVerificationSummary {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub export_id: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub whole_export_hash: String,
    pub latest_receipt_hash: Option<String>,
    pub verified: bool,
    pub deterministic_readback: bool,
    pub export_row_verified: bool,
    pub challenge_rows_verified: bool,
    pub receipt_row_verified: bool,
    pub subject_head_verified: bool,
    pub idempotency_response_verified: bool,
    pub row_counts: KgExportPersistedRowCounts,
    pub challenge_hashes: BTreeMap<String, String>,
    pub challenge_coverage_complete: bool,
    pub persisted_summary_matches_idempotency_response: bool,
    pub route_invalidation_rows: u32,
    pub dagdb_dag_outbox_rows: u32,
    pub raw_artifact_rows: u32,
    pub exo_dag_rows: u32,
    pub preview_context_status: String,
    pub route_invalidation_status: String,
    pub warning_summaries: Vec<String>,
}

/// Compact request to queue a persisted export for future DAG finality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub export_id: String,
    pub requester_did: String,
    pub idempotency_key: Option<String>,
}

impl KgExportFinalityOutboxRequest {
    /// Validate compact request shape and forbidden-material boundaries.
    pub fn validate(&self) -> Result<()> {
        if self.tenant_id.trim().is_empty()
            || self.namespace.trim().is_empty()
            || self.export_id.trim().is_empty()
            || self.requester_did.trim().is_empty()
        {
            return Err(KgExportError::InvalidScope {
                reason: "export finality/outbox request fields must not be empty".to_owned(),
            });
        }
        if !self.requester_did.starts_with("did:") {
            return Err(KgExportError::InvalidScope {
                reason: "export finality/outbox requester_did must be a DID".to_owned(),
            });
        }
        reject_forbidden_string("tenant_id", &self.tenant_id)?;
        reject_forbidden_string("namespace", &self.namespace)?;
        reject_forbidden_string("export_id", &self.export_id)?;
        reject_forbidden_string("requester_did", &self.requester_did)?;
        if let Some(idempotency_key) = &self.idempotency_key {
            if idempotency_key.trim().is_empty() {
                return Err(KgExportError::InvalidScope {
                    reason: "export finality/outbox idempotency_key must not be empty".to_owned(),
                });
            }
            reject_forbidden_string("idempotency_key", idempotency_key)?;
        }
        Ok(())
    }
}

/// Repository-level summary returned after queuing export outbox metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxSummary {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub export_id: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub replayed: bool,
    pub outbox_id: String,
    pub dag_write_id: String,
    pub dag_payload_hash: String,
    pub export_status: String,
    pub whole_export_hash: String,
    pub latest_receipt_hash: String,
    pub inserted_dag_outbox_count: u32,
    pub inserted_idempotency_response_count: u32,
    pub persisted_dag_outbox_count: u32,
    pub persisted_route_invalidation_count: u32,
    pub persisted_raw_artifact_count: u32,
    pub persisted_exo_dag_write_count: u32,
    pub diagnostics: KgExportFinalityOutboxDiagnostics,
}

/// Deterministic diagnostics for the export finality/outbox adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxDiagnostics {
    pub evidence: KgExportFinalityOutboxEvidenceDiagnostics,
    pub challenge_proof: KgExportFinalityOutboxChallengeDiagnostics,
    pub receipt: KgExportFinalityOutboxReceiptDiagnostics,
    pub outbox: KgExportFinalityOutboxRowDiagnostics,
    pub idempotency_replay: KgExportPersistedIdempotencyDiagnostics,
    pub material_exclusion: KgExportFinalityOutboxMaterialExclusionDiagnostics,
    pub advisory_deferred: KgExportFinalityOutboxAdvisoryDiagnostics,
    pub warning_summaries: Vec<String>,
}

/// Evidence checks required before export outbox metadata is queued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxEvidenceDiagnostics {
    pub tenant_namespace_match: bool,
    pub export_id: String,
    pub requester_did: String,
    pub committed_export_evidence_checked: bool,
    pub committed_receipt_evidence_checked: bool,
    pub export_row_verified: bool,
    pub export_status: String,
    pub persisted_export_requester_did: String,
    pub whole_export_hash: String,
    pub latest_receipt_hash: String,
    pub outbox_eligible: bool,
    pub evidence_status: String,
    pub context_packet_evidence_status: String,
    pub preview_context_status: String,
    pub route_invalidation_status: String,
    pub evidence_warnings: Vec<String>,
}

/// Challenge/proof rows checked before finality/outbox queueing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxChallengeDiagnostics {
    pub expected_challenge_count: u32,
    pub challenge_count: u32,
    pub challenge_kinds: Vec<String>,
    pub challenge_hashes: BTreeMap<String, String>,
    pub challenge_statuses: BTreeMap<String, String>,
    pub challenge_coverage_complete: bool,
    pub whole_export_challenge_hash: String,
    pub citation_index_challenge_hash: String,
    pub provenance_index_challenge_hash: String,
    pub redaction_summary_challenge_hash: String,
    pub omission_summary_challenge_hash: String,
    pub proof_algorithm: String,
    pub verification_status: String,
    pub readback_verified: bool,
}

/// Receipt and subject-head checks required before outbox queueing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxReceiptDiagnostics {
    pub receipt_subject_kind: String,
    pub receipt_event_type: String,
    pub receipt_event_supported: bool,
    pub latest_receipt_hash: String,
    pub receipt_row_verified: bool,
    pub subject_head_verified: bool,
    pub latest_receipt_head_matches: bool,
    pub dag_receipt_hash_present: bool,
    pub compensation_receipt_hash_present: bool,
    pub receipt_body_raw_artifact_persisted: bool,
}

/// Inserted or replayed outbox row diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxRowDiagnostics {
    pub outbox_id: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub dag_write_id: String,
    pub dag_payload_hash: String,
    pub payload_material_class: String,
    pub dag_finality_status: String,
    pub dag_receipt_hash_present: bool,
    pub compensation_receipt_hash_present: bool,
    pub retry_attempt_count: u32,
    pub max_attempts: u32,
    pub next_attempt_status: String,
    pub inserted_dag_outbox_count: u32,
    pub persisted_dag_outbox_count: u32,
    pub direct_exo_dag_write: bool,
    pub exo_dag_table_mutated: bool,
    pub route_invalidation_written: bool,
    pub raw_artifact_persisted: bool,
}

/// Raw/private material exclusion diagnostics for finality/outbox queueing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxMaterialExclusionDiagnostics {
    pub json_markdown_artifact_absent: bool,
    pub markdown_body_absent: bool,
    pub private_payload_absent: bool,
    pub model_output_absent: bool,
    pub source_material_absent: bool,
    pub gateway_secret_absent: bool,
    pub database_connection_absent: bool,
    pub private_key_absent: bool,
    pub local_absolute_path_absent: bool,
    pub outbox_payload_is_hash_only: bool,
}

/// Deferred boundaries that remain outside export finality/outbox queueing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportFinalityOutboxAdvisoryDiagnostics {
    pub gateway_api_deferred: bool,
    pub graph_explorer_deferred: bool,
    pub production_route_activation_deferred: bool,
    pub route_invalidation_writes_deferred: bool,
    pub raw_artifact_storage_deferred: bool,
    pub broad_product_export_surface_deferred: bool,
    pub direct_exo_dag_writes_deferred: bool,
    pub exo_dag_table_mutation_deferred: bool,
}

/// Deterministic diagnostics for persisted export summaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedDiagnostics {
    pub row_counts: KgExportPersistedRowCounts,
    pub evidence: KgExportPersistedEvidenceDiagnostics,
    pub section_persistence: KgExportPersistedSectionDiagnostics,
    pub challenge_proof: KgExportPersistedChallengeDiagnostics,
    pub receipt: KgExportPersistedReceiptDiagnostics,
    pub idempotency_replay: KgExportPersistedIdempotencyDiagnostics,
    pub advisory_deferred: KgExportPersistedAdvisoryDiagnostics,
    pub warning_summaries: Vec<String>,
}

/// Row counts written by one persistence transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedRowCounts {
    pub export_rows: u32,
    pub challenge_rows: u32,
    pub receipt_rows: u32,
    pub subject_receipt_head_rows: u32,
    pub idempotency_response_rows: u32,
    pub route_invalidation_rows: u32,
    pub dagdb_dag_outbox_rows: u32,
    pub raw_artifact_rows: u32,
    pub exo_dag_rows: u32,
}

/// Evidence status for the export persistence request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedEvidenceDiagnostics {
    pub tenant_namespace_match: bool,
    pub memory_record_count: u32,
    pub receipt_record_count: u32,
    pub subject_receipt_head_count: u32,
    pub context_packet_record_count: u32,
    pub context_packet_preview_count: u32,
    pub route_receipt_count: u32,
    pub writeback_summary_count: u32,
    pub citation_handle_count: u32,
    pub provenance_record_count: u32,
    pub committed_memory_evidence_checked: bool,
    pub committed_receipt_evidence_checked: bool,
    pub evidence_status: String,
    pub preview_context_status: String,
    pub route_invalidation_status: String,
    pub evidence_warnings: Vec<String>,
}

/// How portable export sections are represented by the persistence adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedSectionDiagnostics {
    pub persisted_row_sections: Vec<String>,
    pub hash_only_sections: Vec<String>,
    pub not_persisted_sections: Vec<String>,
    pub section_hash_count: u32,
    pub raw_artifact_persisted: bool,
}

/// Challenge/proof status for persisted export hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedChallengeDiagnostics {
    pub challenge_count: u32,
    pub challenge_kinds: Vec<String>,
    pub challenge_hashes: BTreeMap<String, String>,
    pub covered_hash_sections: Vec<String>,
    pub coverage_complete: bool,
    pub proof_algorithm: String,
    pub verification_status: String,
}

/// Receipt behavior for persisted export rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedReceiptDiagnostics {
    pub receipt_subject_kind: String,
    pub receipt_event_type: String,
    pub latest_receipt_hash: Option<String>,
    pub subject_head_written: bool,
    pub dag_finality_status: String,
    pub receipt_body_raw_artifact_persisted: bool,
    pub route_invalidation_receipt_written: bool,
}

/// Idempotency/replay status for persisted exports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedIdempotencyDiagnostics {
    pub idempotency_key: String,
    pub request_hash: String,
    pub replayed: bool,
    pub response_cached: bool,
    pub status_code: u16,
    pub replay_reason: String,
}

/// Advisory/deferred export persistence boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgExportPersistedAdvisoryDiagnostics {
    pub route_invalidation_advisory: bool,
    pub route_invalidation_status: String,
    pub gateway_api_deferred: bool,
    pub graph_explorer_deferred: bool,
    pub production_route_activation_deferred: bool,
    pub dagdb_dag_outbox_deferred: bool,
    pub exo_dag_writes_deferred: bool,
    pub raw_artifact_storage_deferred: bool,
    pub broad_product_export_surface_deferred: bool,
}

/// Inputs used by the read adapter to assemble the portable export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgExportBuildInput {
    pub scope: KgExportScope,
    pub memory_records: Vec<KgExportRecord>,
    pub catalog_entries: Vec<KgExportRecord>,
    pub graph_nodes: Vec<KgExportRecord>,
    pub graph_edges: Vec<KgExportRecord>,
    pub similarity_results: Vec<KgExportRecord>,
    pub canonicalization_decisions: Vec<KgExportRecord>,
    pub placement_traces: Vec<KgExportRecord>,
    pub validation_reports: Vec<KgExportRecord>,
    pub receipts: Vec<KgExportRecord>,
    pub subject_receipt_heads: Vec<KgExportRecord>,
    pub context_packet_previews: Vec<KgExportRecord>,
    pub context_packet_records: Vec<KgExportRecord>,
    pub route_receipts: Vec<KgExportRecord>,
    pub writeback_summaries: Vec<KgExportRecord>,
    pub idempotency_references: Vec<KgExportRecord>,
    pub citation_index: Vec<KgExportRecord>,
    pub provenance_index: Vec<KgExportRecord>,
}

/// Files written by the export artifact helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgExportArtifactSet {
    pub output_json: PathBuf,
    pub output_md: PathBuf,
}

/// Errors raised by the export report layer.
#[derive(Debug, Error)]
pub enum KgExportError {
    /// No database URL was supplied for persisted-row export mode.
    #[error("kg_export_database_url_missing: {env_var}")]
    MissingDatabaseUrl {
        /// Required env var.
        env_var: &'static str,
    },
    /// Export request shape is invalid or unsafe.
    #[error("kg_export_scope_invalid: {reason}")]
    InvalidScope {
        /// Stable validation reason.
        reason: String,
    },
    /// Export material contains forbidden content.
    #[error("kg_export_forbidden_material: {path}: {reason}")]
    ForbiddenMaterial {
        /// JSON path or field label.
        path: String,
        /// Stable reason.
        reason: String,
    },
    /// Existing persisted export, replay key, or evidence conflicts with the request.
    #[error("kg_export_conflict: {reason}")]
    Conflict {
        /// Stable conflict reason.
        reason: String,
    },
    /// Cached idempotency response cannot be replayed under the current summary schema.
    #[error("kg_export_incompatible_cached_response: {route_name}: {reason}")]
    IncompatibleCachedResponse {
        /// Idempotency route whose cached response failed compatibility checks.
        route_name: String,
        /// Stable incompatibility reason.
        reason: String,
    },
    /// The current schema cannot support the requested export persistence target.
    #[error("kg_export_unsupported_persistence_target: {target}")]
    UnsupportedPersistenceTarget {
        /// Unsupported target name.
        target: String,
    },
    /// Timestamp value cannot be stored.
    #[error("kg_export_timestamp_out_of_range")]
    TimestampOutOfRange,
    /// Count cannot fit response fields.
    #[error("kg_export_count_out_of_range")]
    CountOutOfRange,
    /// Shared hash validation failed.
    #[error(transparent)]
    ImportHash(#[from] KgImportError),
    /// Hashing failed.
    #[error("kg_export_hash_failed: {reason}")]
    Hash {
        /// Stable hash reason.
        reason: String,
    },
    /// File write failed.
    #[error("kg_export_io_failed")]
    Io {
        /// Source IO error.
        #[source]
        source: io::Error,
    },
    /// JSON conversion failed.
    #[error("kg_export_json_failed: {reason}")]
    Json {
        /// Stable conversion reason.
        reason: String,
    },
    /// Postgres foundation failed.
    #[error("kg_export_postgres_init_failed")]
    Init {
        /// Source Postgres foundation error.
        #[source]
        source: KgExportSourceError,
    },
    /// SQL operation failed.
    #[error("kg_export_postgres_failed")]
    Postgres {
        /// Source SQLx error.
        #[source]
        source: KgExportSourceError,
    },
}

/// Result alias for export reports.
pub type Result<T> = std::result::Result<T, KgExportError>;

/// Build a portable export artifact from already-sanitized section rows.
pub fn build_portable_export(input: KgExportBuildInput) -> Result<KgPortableExport> {
    input.scope.validate()?;
    let mut export = KgPortableExport {
        schema_version: KG_PORTABLE_EXPORT_SCHEMA.to_owned(),
        tenant_id: input.scope.tenant_id.clone(),
        namespace: input.scope.namespace.clone(),
        export_id: String::new(),
        export_scope: input.scope.clone(),
        created_at_or_hlc: KgExportHlc {
            physical_ms: 0,
            logical: 0,
        },
        source_commit_or_repo_ref: input.scope.source_commit_or_repo_ref.clone(),
        memory_records: sort_records(input.memory_records, &["memory_id"]),
        catalog_entries: sort_records(input.catalog_entries, &["catalog_path", "catalog_id"]),
        graph_nodes: sort_records(
            input.graph_nodes,
            &["graph_style", "memory_id", "node_kind", "graph_node_id"],
        ),
        graph_edges: sort_records(
            input.graph_edges,
            &[
                "graph_style",
                "from_memory_id",
                "to_memory_id",
                "edge_kind",
                "graph_edge_id",
            ],
        ),
        similarity_results: sort_records(
            input.similarity_results,
            &[
                "candidate_memory_id",
                "matched_memory_id",
                "similarity_type",
                "similarity_result_id",
            ],
        ),
        canonicalization_decisions: sort_records(
            input.canonicalization_decisions,
            &["input_memory_id", "decision_kind", "decision_id"],
        ),
        placement_traces: sort_records(
            input.placement_traces,
            &["input_memory_id", "placement_trace_id"],
        ),
        validation_reports: sort_records(
            input.validation_reports,
            &["subject_kind", "subject_id", "validation_report_id"],
        ),
        receipts: sort_records(
            input.receipts,
            &["subject_kind", "subject_id", "seq", "receipt_hash"],
        ),
        subject_receipt_heads: sort_records(
            input.subject_receipt_heads,
            &["subject_kind", "subject_id"],
        ),
        context_packet_previews: sort_records(
            input.context_packet_previews,
            &["context_packet_id"],
        ),
        context_packet_records: sort_records(input.context_packet_records, &["context_packet_id"]),
        route_receipts: sort_records(input.route_receipts, &["route_id"]),
        writeback_summaries: sort_records(
            input.writeback_summaries,
            &["proposal_id", "candidate_id", "idempotency_key"],
        ),
        idempotency_references: sort_records(
            input.idempotency_references,
            &["route_name", "idempotency_key"],
        ),
        citation_index: sort_records(input.citation_index, &["citation_handle", "memory_id"]),
        provenance_index: sort_records(input.provenance_index, &["subject_kind", "subject_id"]),
        advisory_sections: advisory_sections(),
        redaction_summary: redaction_summary(),
        omission_summary: omission_summary(),
        diagnostics: empty_diagnostics(),
        hashes: KgExportHashes {
            section_hashes: BTreeMap::new(),
            export_id_material_hash: String::new(),
            whole_export_hash: String::new(),
        },
        verification: KgExportVerification {
            deterministic_ordering: true,
            body_payload_exclusion_enforced: true,
            tenant_namespace_scoped: true,
            preview_context_marked_preview_only: true,
            route_invalidation_writes_implemented: false,
            export_persistence_implemented: false,
            gateway_api_exposure_implemented: false,
            graph_explorer_changes_implemented: false,
            production_route_activation_implemented: false,
            exo_dag_tables_mutated: false,
        },
        acceptance: KgExportAcceptance {
            report_only: true,
            export_persistence_implemented: false,
            gateway_api_exposure_implemented: false,
            graph_explorer_changes_implemented: false,
            production_route_activation_implemented: false,
            route_invalidation_writes_implemented: false,
            exo_dag_tables_mutated: false,
        },
    };
    let section_hashes = compute_section_hashes(&export)?;
    export.diagnostics = build_export_diagnostics(&export, &section_hashes);
    export.hashes = compute_hashes(&export, section_hashes)?;
    export.export_id = export.hashes.export_id_material_hash.clone();
    reject_forbidden_export_json(&serde_json::to_value(&export).map_err(json_error)?, "$")?;
    Ok(export)
}

/// Write JSON and Markdown artifacts for a portable export report.
pub fn write_kg_export_artifacts(
    export: &KgPortableExport,
    output_json: impl AsRef<Path>,
    output_md: impl AsRef<Path>,
) -> Result<KgExportArtifactSet> {
    let output_json = output_json.as_ref();
    let output_md = output_md.as_ref();
    if let Some(parent) = output_json.parent() {
        fs::create_dir_all(parent).map_err(|source| KgExportError::Io { source })?;
    }
    if let Some(parent) = output_md.parent() {
        fs::create_dir_all(parent).map_err(|source| KgExportError::Io { source })?;
    }
    reject_forbidden_export_json(&serde_json::to_value(export).map_err(json_error)?, "$")?;
    let export_json = serde_json::to_string_pretty(export).map_err(json_error)?;
    fs::write(output_json, format!("{export_json}\n"))
        .map_err(|source| KgExportError::Io { source })?;
    let summary = kg_export_markdown_summary(export);
    fs::write(output_md, summary).map_err(|source| KgExportError::Io { source })?;
    Ok(KgExportArtifactSet {
        output_json: output_json.to_path_buf(),
        output_md: output_md.to_path_buf(),
    })
}

/// Write artifacts to the default target/dagdb export path.
pub fn write_default_kg_export_artifacts(export: &KgPortableExport) -> Result<KgExportArtifactSet> {
    write_kg_export_artifacts(export, KG_EXPORT_JSON_PATH, KG_EXPORT_MD_PATH)
}

/// Parse and validate a portable export report for repository persistence.
pub fn parse_portable_export_json(export_json: &str) -> Result<KgPortableExport> {
    let export: KgPortableExport = serde_json::from_str(export_json).map_err(json_error)?;
    validate_portable_export_for_persistence(&export)?;
    Ok(export)
}

/// Validate the bounded export persistence contract before any Postgres writes.
pub fn validate_portable_export_for_persistence(export: &KgPortableExport) -> Result<()> {
    if export.schema_version != KG_PORTABLE_EXPORT_SCHEMA {
        return Err(KgExportError::InvalidScope {
            reason: "unsupported portable export schema version".to_owned(),
        });
    }
    export.export_scope.validate()?;
    if export.tenant_id != export.export_scope.tenant_id
        || export.namespace != export.export_scope.namespace
    {
        return Err(KgExportError::InvalidScope {
            reason: "export tenant/namespace must match export scope".to_owned(),
        });
    }
    if export.export_id != export.hashes.export_id_material_hash {
        return Err(KgExportError::Conflict {
            reason: "export_id does not match export_id_material_hash".to_owned(),
        });
    }
    validate_non_empty("export_id", &export.export_id)?;
    validate_non_empty("whole_export_hash", &export.hashes.whole_export_hash)?;
    if !export.verification.body_payload_exclusion_enforced
        || export.verification.route_invalidation_writes_implemented
        || export.verification.gateway_api_exposure_implemented
        || export.verification.graph_explorer_changes_implemented
        || export.verification.production_route_activation_implemented
        || export.verification.exo_dag_tables_mutated
    {
        return Err(KgExportError::UnsupportedPersistenceTarget {
            target: "portable export verification flags".to_owned(),
        });
    }
    if !export.acceptance.report_only
        || export.acceptance.route_invalidation_writes_implemented
        || export.acceptance.gateway_api_exposure_implemented
        || export.acceptance.graph_explorer_changes_implemented
        || export.acceptance.production_route_activation_implemented
        || export.acceptance.exo_dag_tables_mutated
    {
        return Err(KgExportError::UnsupportedPersistenceTarget {
            target: "portable export acceptance flags".to_owned(),
        });
    }
    reject_forbidden_export_json(&serde_json::to_value(export).map_err(json_error)?, "$")?;
    // Caller-supplied hashes are untrusted: recompute every section hash and
    // the whole-export hash from the submitted section content and fail
    // closed on any mismatch, so persistence never stores forged material as
    // `verified`.
    let recomputed_section_hashes = compute_section_hashes(export)?;
    if recomputed_section_hashes != export.hashes.section_hashes
        || recomputed_section_hashes != export.diagnostics.section_hashes
    {
        return Err(KgExportError::Conflict {
            reason: "submitted section hashes do not match recomputed section content".to_owned(),
        });
    }
    if section_counts(export) != export.diagnostics.section_counts {
        return Err(KgExportError::Conflict {
            reason: "submitted section counts do not match exported section content".to_owned(),
        });
    }
    let recomputed_hashes = compute_hashes(export, recomputed_section_hashes)?;
    if recomputed_hashes.export_id_material_hash != export.hashes.export_id_material_hash
        || recomputed_hashes.whole_export_hash != export.hashes.whole_export_hash
    {
        return Err(KgExportError::Conflict {
            reason: "submitted export hashes do not match recomputed export material".to_owned(),
        });
    }
    Ok(())
}

/// Build a compact deterministic Markdown summary for export review.
#[must_use]
pub fn kg_export_markdown_summary(export: &KgPortableExport) -> String {
    [
        "# DAG DB KG Portable Export Report".to_owned(),
        String::new(),
        format!("- schema: `{KG_PORTABLE_EXPORT_SUMMARY_SCHEMA}`"),
        format!("- export schema: `{}`", export.schema_version),
        format!("- tenant: `{}`", export.tenant_id),
        format!("- namespace: `{}`", export.namespace),
        format!("- export_id: `{}`", export.export_id),
        format!("- memory_records: `{}`", export.memory_records.len()),
        format!("- catalog_entries: `{}`", export.catalog_entries.len()),
        format!("- graph_nodes: `{}`", export.graph_nodes.len()),
        format!("- graph_edges: `{}`", export.graph_edges.len()),
        format!(
            "- validation_reports: `{}`",
            export.validation_reports.len()
        ),
        format!("- receipts: `{}`", export.receipts.len()),
        format!(
            "- context_packet_previews: `{}`",
            export.context_packet_previews.len()
        ),
        format!(
            "- writeback_summaries: `{}`",
            export.writeback_summaries.len()
        ),
        format!("- whole_export_hash: `{}`", export.hashes.whole_export_hash),
        String::new(),
        "## Diagnostics".to_owned(),
        format!(
            "- citation_handle_count: `{}`",
            export
                .diagnostics
                .citation_diagnostics
                .citation_handle_count
        ),
        format!(
            "- memory_provenance_count: `{}`",
            export
                .diagnostics
                .provenance_diagnostics
                .memory_provenance_count
        ),
        format!(
            "- preview_only_context_count: `{}`",
            export.diagnostics.preview_only_context_count
        ),
        format!(
            "- source_path_omission_count: `{}`",
            export
                .diagnostics
                .redaction_omission_diagnostics
                .source_path_omission_count
        ),
        String::new(),
        "## Verification".to_owned(),
        format!(
            "- body_payload_exclusion_enforced: `{}`",
            export.verification.body_payload_exclusion_enforced
        ),
        format!(
            "- export_persistence_implemented: `{}`",
            export.verification.export_persistence_implemented
        ),
        format!(
            "- route_invalidation_writes_implemented: `{}`",
            export.verification.route_invalidation_writes_implemented
        ),
        format!(
            "- exo_dag_tables_mutated: `{}`",
            export.verification.exo_dag_tables_mutated
        ),
        String::new(),
        "## Advisory Sections".to_owned(),
        advisory_lines(&export.advisory_sections).join("\n"),
        String::new(),
    ]
    .join("\n")
}

fn compute_section_hashes(export: &KgPortableExport) -> Result<BTreeMap<String, String>> {
    let mut section_hashes = BTreeMap::new();
    section_hashes.insert(
        "memory_records".to_owned(),
        hash_section("memory_records", &export.memory_records)?,
    );
    section_hashes.insert(
        "catalog_entries".to_owned(),
        hash_section("catalog_entries", &export.catalog_entries)?,
    );
    section_hashes.insert(
        "graph_nodes".to_owned(),
        hash_section("graph_nodes", &export.graph_nodes)?,
    );
    section_hashes.insert(
        "graph_edges".to_owned(),
        hash_section("graph_edges", &export.graph_edges)?,
    );
    section_hashes.insert(
        "similarity_results".to_owned(),
        hash_section("similarity_results", &export.similarity_results)?,
    );
    section_hashes.insert(
        "canonicalization_decisions".to_owned(),
        hash_section(
            "canonicalization_decisions",
            &export.canonicalization_decisions,
        )?,
    );
    section_hashes.insert(
        "placement_traces".to_owned(),
        hash_section("placement_traces", &export.placement_traces)?,
    );
    section_hashes.insert(
        "validation_reports".to_owned(),
        hash_section("validation_reports", &export.validation_reports)?,
    );
    section_hashes.insert(
        "receipts".to_owned(),
        hash_section("receipts", &export.receipts)?,
    );
    section_hashes.insert(
        "subject_receipt_heads".to_owned(),
        hash_section("subject_receipt_heads", &export.subject_receipt_heads)?,
    );
    section_hashes.insert(
        "context_packet_previews".to_owned(),
        hash_section("context_packet_previews", &export.context_packet_previews)?,
    );
    section_hashes.insert(
        "context_packet_records".to_owned(),
        hash_section("context_packet_records", &export.context_packet_records)?,
    );
    section_hashes.insert(
        "route_receipts".to_owned(),
        hash_section("route_receipts", &export.route_receipts)?,
    );
    section_hashes.insert(
        "writeback_summaries".to_owned(),
        hash_section("writeback_summaries", &export.writeback_summaries)?,
    );
    section_hashes.insert(
        "idempotency_references".to_owned(),
        hash_section("idempotency_references", &export.idempotency_references)?,
    );
    section_hashes.insert(
        "citation_index".to_owned(),
        hash_section("citation_index", &export.citation_index)?,
    );
    section_hashes.insert(
        "provenance_index".to_owned(),
        hash_section("provenance_index", &export.provenance_index)?,
    );
    section_hashes.insert(
        "advisory_sections".to_owned(),
        hash_section("advisory_sections", &export.advisory_sections)?,
    );
    section_hashes.insert(
        "redaction_summary".to_owned(),
        hash_section("redaction_summary", &export.redaction_summary)?,
    );
    section_hashes.insert(
        "omission_summary".to_owned(),
        hash_section("omission_summary", &export.omission_summary)?,
    );
    Ok(section_hashes)
}

fn compute_hashes(
    export: &KgPortableExport,
    section_hashes: BTreeMap<String, String>,
) -> Result<KgExportHashes> {
    let section_hash_material = section_hashes
        .iter()
        .map(|(section, hash)| format!("{section}:{hash}"))
        .collect::<Vec<_>>()
        .join("|");
    let source_ref = export
        .source_commit_or_repo_ref
        .as_deref()
        .unwrap_or("none");
    let export_id_material_hash = stable_hash(
        "exo.dagdb.kg_export.report.export_id",
        &[
            &export.tenant_id,
            &export.namespace,
            source_ref,
            &section_hash_material,
        ],
    )?
    .to_string();
    let whole_export_hash = hash_event_body(&WholeExportHashMaterial {
        schema_version: &export.schema_version,
        tenant_id: &export.tenant_id,
        namespace: &export.namespace,
        export_scope: &export.export_scope,
        section_hashes: &section_hashes,
        redaction_summary: &export.redaction_summary,
        omission_summary: &export.omission_summary,
        diagnostics: &export.diagnostics,
        verification: &export.verification,
    })
    .map_err(|error| KgExportError::Hash {
        reason: error.to_string(),
    })?
    .to_string();
    Ok(KgExportHashes {
        section_hashes,
        export_id_material_hash,
        whole_export_hash,
    })
}

#[derive(Serialize)]
struct WholeExportHashMaterial<'a> {
    schema_version: &'a str,
    tenant_id: &'a str,
    namespace: &'a str,
    export_scope: &'a KgExportScope,
    section_hashes: &'a BTreeMap<String, String>,
    redaction_summary: &'a KgExportRecord,
    omission_summary: &'a KgExportRecord,
    diagnostics: &'a KgExportDiagnostics,
    verification: &'a KgExportVerification,
}

fn hash_section<T: Serialize>(section: &str, value: &T) -> Result<String> {
    hash_event_body(&(section, value))
        .map(|hash| hash.to_string())
        .map_err(|error| KgExportError::Hash {
            reason: error.to_string(),
        })
}

fn sort_records(mut records: Vec<KgExportRecord>, keys: &[&str]) -> Vec<KgExportRecord> {
    records.sort_by(|left, right| {
        keys.iter()
            .map(|key| record_key(left, key).cmp(&record_key(right, key)))
            .find(|ordering| *ordering != std::cmp::Ordering::Equal)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    records
}

fn record_key(record: &KgExportRecord, key: &str) -> String {
    record.get(key).map_or_else(String::new, value_key)
}

fn value_key(value: &JsonValue) -> String {
    match value {
        JsonValue::String(value) => value.clone(),
        JsonValue::Number(value) => value.to_string(),
        JsonValue::Bool(value) => value.to_string(),
        JsonValue::Array(values) => values.iter().map(value_key).collect::<Vec<_>>().join("/"),
        JsonValue::Null => String::new(),
        JsonValue::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn redaction_summary() -> KgExportRecord {
    let mut summary = KgExportRecord::new();
    summary.insert(
        "markdown_body_content_excluded".to_owned(),
        JsonValue::Bool(true),
    );
    summary.insert(
        "private_payload_content_excluded".to_owned(),
        JsonValue::Bool(true),
    );
    summary.insert(
        "model_output_content_excluded".to_owned(),
        JsonValue::Bool(true),
    );
    summary.insert("source_excerpts_excluded".to_owned(), JsonValue::Bool(true));
    summary.insert(
        "local_absolute_paths_excluded".to_owned(),
        JsonValue::Bool(true),
    );
    summary.insert(
        "database_connection_values_excluded".to_owned(),
        JsonValue::Bool(true),
    );
    summary.insert("gateway_secrets_excluded".to_owned(), JsonValue::Bool(true));
    summary.insert("private_keys_excluded".to_owned(), JsonValue::Bool(true));
    summary
}

fn omission_summary() -> KgExportRecord {
    let mut summary = KgExportRecord::new();
    summary.insert(
        "source_path_metadata".to_owned(),
        JsonValue::String("omitted_not_persisted_as_safe_metadata".to_owned()),
    );
    summary.insert(
        "export_persistence".to_owned(),
        JsonValue::String("deferred".to_owned()),
    );
    summary.insert(
        "route_invalidation_writes".to_owned(),
        JsonValue::String("deferred".to_owned()),
    );
    summary.insert(
        "gateway_api_exposure".to_owned(),
        JsonValue::String("deferred".to_owned()),
    );
    summary.insert(
        "graph_explorer_changes".to_owned(),
        JsonValue::String("deferred".to_owned()),
    );
    summary.insert(
        "exo_dag_writes".to_owned(),
        JsonValue::String("deferred".to_owned()),
    );
    summary
}

fn advisory_sections() -> Vec<KgExportRecord> {
    [
        (
            "route_invalidation_writes",
            "advisory_only",
            "no committed route invalidation writes are emitted by this report adapter",
        ),
        (
            "export_persistence",
            "deferred",
            "no export table or durable export record is written",
        ),
        (
            "gateway_api_exposure",
            "deferred",
            "no public API or gateway route is exposed",
        ),
        (
            "graph_explorer_changes",
            "deferred",
            "graph explorer state and display behavior are unchanged",
        ),
        (
            "production_route_activation",
            "deferred",
            "preview route material is not production route activation",
        ),
        (
            "exo_dag_writes",
            "deferred",
            "the adapter reads DAG DB rows only and does not mutate exo-dag tables",
        ),
        (
            "source_path_metadata",
            "omitted",
            "source paths are not persisted as approved safe retrieval metadata",
        ),
    ]
    .into_iter()
    .map(|(section, status, reason)| {
        let mut record = KgExportRecord::new();
        record.insert("section".to_owned(), JsonValue::String(section.to_owned()));
        record.insert("status".to_owned(), JsonValue::String(status.to_owned()));
        record.insert("reason".to_owned(), JsonValue::String(reason.to_owned()));
        record
    })
    .collect()
}

fn advisory_lines(advisory_sections: &[KgExportRecord]) -> Vec<String> {
    advisory_sections
        .iter()
        .map(|record| {
            let section = record
                .get("section")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown");
            let status = record
                .get("status")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown");
            format!("- `{section}`: `{status}`")
        })
        .collect()
}

fn empty_diagnostics() -> KgExportDiagnostics {
    KgExportDiagnostics {
        section_counts: BTreeMap::new(),
        section_hashes: BTreeMap::new(),
        citation_diagnostics: KgExportCitationDiagnostics {
            citation_handle_count: 0,
            memory_coverage_count: 0,
            validation_report_coverage_count: 0,
            receipt_coverage_count: 0,
            graph_edge_coverage_count: 0,
            partial_coverage_count: 0,
            missing_coverage_count: 0,
        },
        provenance_diagnostics: KgExportProvenanceDiagnostics {
            memory_provenance_count: 0,
            validation_provenance_count: 0,
            receipt_provenance_count: 0,
            missing_latest_receipt_count: 0,
            preview_only_provenance_count: 0,
        },
        redaction_omission_diagnostics: KgExportRedactionOmissionDiagnostics {
            markdown_body_content_excluded: false,
            private_payload_content_excluded: false,
            model_output_content_excluded: false,
            source_excerpts_excluded: false,
            source_path_omission_count: 0,
            source_path_omission_reason: String::new(),
            database_connection_values_excluded: false,
            gateway_secrets_excluded: false,
            private_keys_excluded: false,
            local_absolute_paths_excluded: false,
        },
        advisory_deferred_diagnostics: KgExportAdvisoryDeferredDiagnostics {
            advisory_section_count: 0,
            deferred_section_count: 0,
            omitted_section_count: 0,
            route_invalidation_advisory_count: 0,
            export_persistence_deferred: false,
            gateway_api_deferred: false,
            graph_explorer_deferred: false,
            production_route_activation_deferred: false,
            route_invalidation_writes_deferred: false,
            exo_dag_writes_deferred: false,
        },
        deterministic_ordering: false,
        raw_material_exclusion_enforced: false,
        tenant_namespace_scoped: false,
        preview_only_context_count: 0,
    }
}

fn build_export_diagnostics(
    export: &KgPortableExport,
    section_hashes: &BTreeMap<String, String>,
) -> KgExportDiagnostics {
    KgExportDiagnostics {
        section_counts: section_counts(export),
        section_hashes: section_hashes.clone(),
        citation_diagnostics: citation_diagnostics(export),
        provenance_diagnostics: provenance_diagnostics(export),
        redaction_omission_diagnostics: redaction_omission_diagnostics(export),
        advisory_deferred_diagnostics: advisory_deferred_diagnostics(export),
        deterministic_ordering: export.verification.deterministic_ordering,
        raw_material_exclusion_enforced: export.verification.body_payload_exclusion_enforced,
        tenant_namespace_scoped: export.verification.tenant_namespace_scoped,
        preview_only_context_count: count_records_where_bool(
            &export.context_packet_previews,
            "preview_only",
            true,
        ),
    }
}

fn section_counts(export: &KgPortableExport) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    counts.insert(
        "memory_records".to_owned(),
        safe_count(export.memory_records.len()),
    );
    counts.insert(
        "catalog_entries".to_owned(),
        safe_count(export.catalog_entries.len()),
    );
    counts.insert(
        "graph_nodes".to_owned(),
        safe_count(export.graph_nodes.len()),
    );
    counts.insert(
        "graph_edges".to_owned(),
        safe_count(export.graph_edges.len()),
    );
    counts.insert(
        "similarity_results".to_owned(),
        safe_count(export.similarity_results.len()),
    );
    counts.insert(
        "canonicalization_decisions".to_owned(),
        safe_count(export.canonicalization_decisions.len()),
    );
    counts.insert(
        "placement_traces".to_owned(),
        safe_count(export.placement_traces.len()),
    );
    counts.insert(
        "validation_reports".to_owned(),
        safe_count(export.validation_reports.len()),
    );
    counts.insert("receipts".to_owned(), safe_count(export.receipts.len()));
    counts.insert(
        "subject_receipt_heads".to_owned(),
        safe_count(export.subject_receipt_heads.len()),
    );
    counts.insert(
        "context_packet_previews".to_owned(),
        safe_count(export.context_packet_previews.len()),
    );
    counts.insert(
        "context_packet_records".to_owned(),
        safe_count(export.context_packet_records.len()),
    );
    counts.insert(
        "route_receipts".to_owned(),
        safe_count(export.route_receipts.len()),
    );
    counts.insert(
        "writeback_summaries".to_owned(),
        safe_count(export.writeback_summaries.len()),
    );
    counts.insert(
        "idempotency_references".to_owned(),
        safe_count(export.idempotency_references.len()),
    );
    counts.insert(
        "citation_index".to_owned(),
        safe_count(export.citation_index.len()),
    );
    counts.insert(
        "provenance_index".to_owned(),
        safe_count(export.provenance_index.len()),
    );
    counts.insert(
        "advisory_sections".to_owned(),
        safe_count(export.advisory_sections.len()),
    );
    counts.insert(
        "redaction_summary".to_owned(),
        safe_count(export.redaction_summary.len()),
    );
    counts.insert(
        "omission_summary".to_owned(),
        safe_count(export.omission_summary.len()),
    );
    counts
}

fn citation_diagnostics(export: &KgPortableExport) -> KgExportCitationDiagnostics {
    let mut memory_coverage_count = 0;
    let mut validation_report_coverage_count = 0;
    let mut receipt_coverage_count = 0;
    let mut graph_edge_coverage_count = 0;
    let mut partial_coverage_count = 0;
    let mut missing_coverage_count = 0;

    for citation in &export.citation_index {
        let has_memory = non_empty_string(citation, "memory_id");
        let has_validation = non_empty_array(citation, "validation_report_ids");
        let has_receipt = non_empty_string(citation, "latest_receipt_hash");
        let has_graph_edge = non_empty_array(citation, "graph_edge_ids");
        if has_memory {
            memory_coverage_count += 1;
        }
        if has_validation {
            validation_report_coverage_count += 1;
        }
        if has_receipt {
            receipt_coverage_count += 1;
        }
        if has_graph_edge {
            graph_edge_coverage_count += 1;
        }
        if has_memory && (!has_validation || !has_receipt || !has_graph_edge) {
            partial_coverage_count += 1;
        }
        if !has_memory {
            missing_coverage_count += 1;
        }
    }

    KgExportCitationDiagnostics {
        citation_handle_count: safe_count(export.citation_index.len()),
        memory_coverage_count,
        validation_report_coverage_count,
        receipt_coverage_count,
        graph_edge_coverage_count,
        partial_coverage_count,
        missing_coverage_count,
    }
}

fn provenance_diagnostics(export: &KgPortableExport) -> KgExportProvenanceDiagnostics {
    KgExportProvenanceDiagnostics {
        memory_provenance_count: count_records_where_string(
            &export.provenance_index,
            "subject_kind",
            "memory",
        ),
        validation_provenance_count: count_records_where_string(
            &export.provenance_index,
            "subject_kind",
            "validation_report",
        ),
        receipt_provenance_count: count_records_where_string(
            &export.provenance_index,
            "subject_kind",
            "receipt",
        ),
        missing_latest_receipt_count: count_records_missing_string(
            &export.memory_records,
            "latest_receipt_hash",
        ),
        preview_only_provenance_count: count_records_where_string(
            &export.citation_index,
            "citation_status",
            "preview_only",
        ) + count_records_where_bool(
            &export.context_packet_previews,
            "preview_only",
            true,
        ),
    }
}

fn redaction_omission_diagnostics(
    export: &KgPortableExport,
) -> KgExportRedactionOmissionDiagnostics {
    let source_path_omission_reason = export
        .omission_summary
        .get("source_path_metadata")
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown")
        .to_owned();
    KgExportRedactionOmissionDiagnostics {
        markdown_body_content_excluded: bool_field(
            &export.redaction_summary,
            "markdown_body_content_excluded",
        ),
        private_payload_content_excluded: bool_field(
            &export.redaction_summary,
            "private_payload_content_excluded",
        ),
        model_output_content_excluded: bool_field(
            &export.redaction_summary,
            "model_output_content_excluded",
        ),
        source_excerpts_excluded: bool_field(&export.redaction_summary, "source_excerpts_excluded"),
        source_path_omission_count: count_records_where_string(
            &export.advisory_sections,
            "section",
            "source_path_metadata",
        ),
        source_path_omission_reason,
        database_connection_values_excluded: bool_field(
            &export.redaction_summary,
            "database_connection_values_excluded",
        ),
        gateway_secrets_excluded: bool_field(&export.redaction_summary, "gateway_secrets_excluded"),
        private_keys_excluded: bool_field(&export.redaction_summary, "private_keys_excluded"),
        local_absolute_paths_excluded: bool_field(
            &export.redaction_summary,
            "local_absolute_paths_excluded",
        ),
    }
}

fn advisory_deferred_diagnostics(export: &KgPortableExport) -> KgExportAdvisoryDeferredDiagnostics {
    KgExportAdvisoryDeferredDiagnostics {
        advisory_section_count: safe_count(export.advisory_sections.len()),
        deferred_section_count: count_records_where_string(
            &export.advisory_sections,
            "status",
            "deferred",
        ),
        omitted_section_count: count_records_where_string(
            &export.advisory_sections,
            "status",
            "omitted",
        ),
        route_invalidation_advisory_count: count_records_where_string(
            &export.advisory_sections,
            "section",
            "route_invalidation_writes",
        ),
        export_persistence_deferred: !export.verification.export_persistence_implemented,
        gateway_api_deferred: !export.verification.gateway_api_exposure_implemented,
        graph_explorer_deferred: !export.verification.graph_explorer_changes_implemented,
        production_route_activation_deferred: !export
            .verification
            .production_route_activation_implemented,
        route_invalidation_writes_deferred: !export
            .verification
            .route_invalidation_writes_implemented,
        exo_dag_writes_deferred: !export.verification.exo_dag_tables_mutated,
    }
}

fn safe_count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

fn count_records_where_string(records: &[KgExportRecord], key: &str, expected: &str) -> u32 {
    safe_count(
        records
            .iter()
            .filter(|record| {
                record
                    .get(key)
                    .and_then(JsonValue::as_str)
                    .is_some_and(|value| value == expected)
            })
            .count(),
    )
}

fn count_records_where_bool(records: &[KgExportRecord], key: &str, expected: bool) -> u32 {
    safe_count(
        records
            .iter()
            .filter(|record| {
                record
                    .get(key)
                    .and_then(JsonValue::as_bool)
                    .is_some_and(|value| value == expected)
            })
            .count(),
    )
}

fn count_records_missing_string(records: &[KgExportRecord], key: &str) -> u32 {
    safe_count(
        records
            .iter()
            .filter(|record| !non_empty_string(record, key))
            .count(),
    )
}

fn non_empty_string(record: &KgExportRecord, key: &str) -> bool {
    record
        .get(key)
        .and_then(JsonValue::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn non_empty_array(record: &KgExportRecord, key: &str) -> bool {
    record
        .get(key)
        .and_then(JsonValue::as_array)
        .is_some_and(|values| !values.is_empty())
}

fn bool_field(record: &KgExportRecord, key: &str) -> bool {
    record
        .get(key)
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(KgExportError::InvalidScope {
            reason: format!("{field} must not be empty"),
        });
    }
    reject_forbidden_string(field, value)
}

fn validate_unique(field: &str, values: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_non_empty(field, value)?;
        if !seen.insert(value) {
            return Err(KgExportError::InvalidScope {
                reason: format!("duplicate {field}"),
            });
        }
    }
    Ok(())
}

/// Reject forbidden export keys and string fragments in a JSON value.
pub fn reject_forbidden_export_json(value: &JsonValue, path: &str) -> Result<()> {
    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                let lowered_key = key.to_ascii_lowercase();
                if FORBIDDEN_KEYS
                    .iter()
                    .any(|forbidden| lowered_key == *forbidden)
                {
                    return Err(KgExportError::ForbiddenMaterial {
                        path: format!("{path}.{key}"),
                        reason: "forbidden key".to_owned(),
                    });
                }
                reject_forbidden_export_json(child, format!("{path}.{key}").as_str())?;
            }
        }
        JsonValue::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                reject_forbidden_export_json(child, format!("{path}[{index}]").as_str())?;
            }
        }
        JsonValue::String(value) => reject_forbidden_string(path, value)?,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
    Ok(())
}

/// Reject forbidden strings in generated export content.
pub fn reject_forbidden_string(path: &str, value: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| lowered.contains(**fragment))
    {
        return Err(KgExportError::ForbiddenMaterial {
            path: path.to_owned(),
            reason: format!("contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

fn json_error(error: serde_json::Error) -> KgExportError {
    KgExportError::Json {
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn base_scope() -> KgExportScope {
        KgExportScope {
            tenant_id: "tenant-test".to_owned(),
            namespace: "dag-db".to_owned(),
            included_memory_ids: Vec::new(),
            included_graph_styles: Vec::new(),
            included_writeback_idempotency_keys: Vec::new(),
            source_commit_or_repo_ref: Some("test-ref".to_owned()),
            include_preview_context: true,
        }
    }

    fn export_record(entries: Vec<(&str, JsonValue)>) -> KgExportRecord {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    fn rich_export() -> KgPortableExport {
        let mut scope = base_scope();
        scope.source_commit_or_repo_ref = None;
        build_portable_export(KgExportBuildInput {
            scope,
            memory_records: vec![
                export_record(vec![
                    ("memory_id", json!("memory-b")),
                    ("latest_receipt_hash", json!("receipt-b")),
                ]),
                export_record(vec![("memory_id", json!("memory-a"))]),
            ],
            catalog_entries: vec![export_record(vec![
                ("catalog_path", json!(["docs", "dagdb"])),
                ("catalog_id", json!("catalog-a")),
            ])],
            graph_nodes: vec![export_record(vec![
                ("graph_style", json!("semantic_catalog_graph")),
                ("memory_id", json!("memory-a")),
                ("node_kind", json!("raw")),
                ("graph_node_id", json!("node-a")),
            ])],
            graph_edges: vec![export_record(vec![
                ("graph_style", json!("semantic_catalog_graph")),
                ("from_memory_id", json!("memory-a")),
                ("to_memory_id", json!("memory-b")),
                ("edge_kind", json!("related_to")),
                ("graph_edge_id", json!("edge-a")),
            ])],
            similarity_results: vec![export_record(vec![
                ("candidate_memory_id", json!("memory-a")),
                ("matched_memory_id", json!("memory-b")),
                ("similarity_type", json!("semantic")),
                ("similarity_result_id", json!("similarity-a")),
            ])],
            canonicalization_decisions: vec![export_record(vec![
                ("input_memory_id", json!("memory-a")),
                ("decision_kind", json!("new_canonical")),
                ("decision_id", json!("decision-a")),
            ])],
            placement_traces: vec![export_record(vec![
                ("input_memory_id", json!("memory-a")),
                ("placement_trace_id", json!("trace-a")),
            ])],
            validation_reports: vec![export_record(vec![
                ("subject_kind", json!("memory")),
                ("subject_id", json!("memory-a")),
                ("validation_report_id", json!("validation-a")),
            ])],
            receipts: vec![export_record(vec![
                ("subject_kind", json!("memory")),
                ("subject_id", json!("memory-a")),
                ("seq", json!(1)),
                ("receipt_hash", json!("receipt-a")),
            ])],
            subject_receipt_heads: vec![export_record(vec![
                ("subject_kind", json!("memory")),
                ("subject_id", json!("memory-a")),
            ])],
            context_packet_previews: vec![
                export_record(vec![
                    ("context_packet_id", json!("packet-a")),
                    ("preview_only", json!(true)),
                ]),
                export_record(vec![
                    ("context_packet_id", json!("packet-b")),
                    ("preview_only", json!(false)),
                ]),
            ],
            context_packet_records: vec![export_record(vec![(
                "context_packet_id",
                json!("packet-a"),
            )])],
            route_receipts: vec![export_record(vec![("route_id", json!("route-a"))])],
            writeback_summaries: vec![export_record(vec![
                ("proposal_id", json!("proposal-a")),
                ("candidate_id", json!("candidate-a")),
                ("idempotency_key", json!("idem-a")),
            ])],
            idempotency_references: vec![export_record(vec![
                ("route_name", json!("dagdb.kg_export.persisted.v1")),
                ("idempotency_key", json!("idem-a")),
            ])],
            citation_index: vec![
                export_record(vec![
                    ("citation_handle", json!("cite-a")),
                    ("memory_id", json!("memory-a")),
                    ("validation_report_ids", json!(["validation-a"])),
                    ("latest_receipt_hash", json!("receipt-a")),
                    ("graph_edge_ids", json!(["edge-a"])),
                    ("citation_status", json!("preview_only")),
                ]),
                export_record(vec![
                    ("citation_handle", json!("cite-b")),
                    ("memory_id", json!("memory-b")),
                    ("validation_report_ids", json!([])),
                ]),
                export_record(vec![("citation_handle", json!("cite-c"))]),
            ],
            provenance_index: vec![
                export_record(vec![
                    ("subject_kind", json!("memory")),
                    ("subject_id", json!("memory-a")),
                ]),
                export_record(vec![("subject_kind", json!("validation_report"))]),
                export_record(vec![("subject_kind", json!("receipt"))]),
            ],
        })
        .expect("build rich export")
    }

    #[test]
    fn portable_export_hashes_are_deterministic() {
        let mut memory = KgExportRecord::new();
        memory.insert("memory_id".to_owned(), json!("bb"));
        let first = build_portable_export(KgExportBuildInput {
            scope: base_scope(),
            memory_records: vec![memory.clone()],
            catalog_entries: Vec::new(),
            graph_nodes: Vec::new(),
            graph_edges: Vec::new(),
            similarity_results: Vec::new(),
            canonicalization_decisions: Vec::new(),
            placement_traces: Vec::new(),
            validation_reports: Vec::new(),
            receipts: Vec::new(),
            subject_receipt_heads: Vec::new(),
            context_packet_previews: Vec::new(),
            context_packet_records: Vec::new(),
            route_receipts: Vec::new(),
            writeback_summaries: Vec::new(),
            idempotency_references: Vec::new(),
            citation_index: Vec::new(),
            provenance_index: Vec::new(),
        })
        .expect("build export");
        let second = build_portable_export(KgExportBuildInput {
            scope: base_scope(),
            memory_records: vec![memory],
            catalog_entries: Vec::new(),
            graph_nodes: Vec::new(),
            graph_edges: Vec::new(),
            similarity_results: Vec::new(),
            canonicalization_decisions: Vec::new(),
            placement_traces: Vec::new(),
            validation_reports: Vec::new(),
            receipts: Vec::new(),
            subject_receipt_heads: Vec::new(),
            context_packet_previews: Vec::new(),
            context_packet_records: Vec::new(),
            route_receipts: Vec::new(),
            writeback_summaries: Vec::new(),
            idempotency_references: Vec::new(),
            citation_index: Vec::new(),
            provenance_index: Vec::new(),
        })
        .expect("build export again");
        assert_eq!(first, second);
        assert_eq!(first.schema_version, KG_PORTABLE_EXPORT_SCHEMA);
        assert!(!first.hashes.whole_export_hash.is_empty());
        assert!(!first.verification.export_persistence_implemented);
    }

    #[test]
    fn portable_export_rich_report_covers_summary_artifacts_and_parse() {
        let export = rich_export();
        assert_eq!(export.source_commit_or_repo_ref, None);
        assert_eq!(
            export
                .diagnostics
                .citation_diagnostics
                .memory_coverage_count,
            2
        );
        assert_eq!(
            export
                .diagnostics
                .citation_diagnostics
                .partial_coverage_count,
            1
        );
        assert_eq!(
            export
                .diagnostics
                .citation_diagnostics
                .missing_coverage_count,
            1
        );
        assert_eq!(
            export
                .diagnostics
                .provenance_diagnostics
                .receipt_provenance_count,
            1
        );
        assert_eq!(export.diagnostics.preview_only_context_count, 1);

        let summary = kg_export_markdown_summary(&export);
        assert!(summary.contains("DAG DB KG Portable Export Report"));
        assert!(summary.contains("route_invalidation_writes"));

        let export_json = serde_json::to_string_pretty(&export).expect("serialize export");
        assert_eq!(
            parse_portable_export_json(&export_json).expect("parse export"),
            export
        );

        let output_root = std::path::Path::new("target/dagdb/kg_export_coverage");
        let artifact_set = write_kg_export_artifacts(
            &export,
            output_root.join("report.json"),
            output_root.join("summary.md"),
        )
        .expect("write artifacts");
        assert!(artifact_set.output_json.exists());
        assert!(artifact_set.output_md.exists());

        let default_set =
            write_default_kg_export_artifacts(&export).expect("write default artifacts");
        assert!(default_set.output_json.ends_with(KG_EXPORT_JSON_PATH));
        assert!(default_set.output_md.ends_with(KG_EXPORT_MD_PATH));
    }

    #[test]
    fn export_persistence_validation_rejects_conflicts_and_unsupported_flags() {
        let export = rich_export();

        let mut bad_schema = export.clone();
        bad_schema.schema_version = "other".to_owned();
        assert!(matches!(
            validate_portable_export_for_persistence(&bad_schema),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut bad_scope = export.clone();
        bad_scope.namespace = "other".to_owned();
        assert!(matches!(
            validate_portable_export_for_persistence(&bad_scope),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut bad_scope_tenant = export.clone();
        bad_scope_tenant.tenant_id = "other".to_owned();
        assert!(matches!(
            validate_portable_export_for_persistence(&bad_scope_tenant),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut bad_export_id = export.clone();
        bad_export_id.export_id = "wrong".to_owned();
        assert!(matches!(
            validate_portable_export_for_persistence(&bad_export_id),
            Err(KgExportError::Conflict { .. })
        ));

        let mut empty_hash = export.clone();
        empty_hash.hashes.whole_export_hash.clear();
        assert!(matches!(
            validate_portable_export_for_persistence(&empty_hash),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut unsafe_json = export.clone();
        unsafe_json.memory_records[0].insert("Raw_Body".to_owned(), json!("unsafe"));
        assert!(matches!(
            validate_portable_export_for_persistence(&unsafe_json),
            Err(KgExportError::ForbiddenMaterial { .. })
        ));

        let verification_mutators: Vec<fn(&mut KgPortableExport)> = vec![
            |item| item.verification.body_payload_exclusion_enforced = false,
            |item| item.verification.route_invalidation_writes_implemented = true,
            |item| item.verification.gateway_api_exposure_implemented = true,
            |item| item.verification.graph_explorer_changes_implemented = true,
            |item| item.verification.production_route_activation_implemented = true,
            |item| item.verification.exo_dag_tables_mutated = true,
        ];
        for mutate in verification_mutators {
            let mut candidate = export.clone();
            mutate(&mut candidate);
            assert!(matches!(
                validate_portable_export_for_persistence(&candidate),
                Err(KgExportError::UnsupportedPersistenceTarget { .. })
            ));
        }

        let acceptance_mutators: Vec<fn(&mut KgPortableExport)> = vec![
            |item| item.acceptance.report_only = false,
            |item| item.acceptance.route_invalidation_writes_implemented = true,
            |item| item.acceptance.gateway_api_exposure_implemented = true,
            |item| item.acceptance.graph_explorer_changes_implemented = true,
            |item| item.acceptance.production_route_activation_implemented = true,
            |item| item.acceptance.exo_dag_tables_mutated = true,
        ];
        for mutate in acceptance_mutators {
            let mut candidate = export.clone();
            mutate(&mut candidate);
            assert!(matches!(
                validate_portable_export_for_persistence(&candidate),
                Err(KgExportError::UnsupportedPersistenceTarget { .. })
            ));
        }

        assert!(matches!(
            parse_portable_export_json("{not-json"),
            Err(KgExportError::Json { .. })
        ));
    }

    #[test]
    fn export_persistence_validation_recomputes_hashes_from_section_content() {
        let export = rich_export();
        assert!(validate_portable_export_for_persistence(&export).is_ok());

        let mut forged_sections = export.clone();
        forged_sections.memory_records.clear();
        forged_sections.receipts.clear();
        assert!(matches!(
            validate_portable_export_for_persistence(&forged_sections),
            Err(KgExportError::Conflict { .. })
        ));

        let mut forged_counts = export.clone();
        forged_counts
            .diagnostics
            .section_counts
            .insert("memory_records".to_owned(), 99);
        assert!(matches!(
            validate_portable_export_for_persistence(&forged_counts),
            Err(KgExportError::Conflict { .. })
        ));

        let mut forged_section_hash = export.clone();
        forged_section_hash
            .hashes
            .section_hashes
            .insert("memory_records".to_owned(), "11".repeat(32));
        assert!(matches!(
            validate_portable_export_for_persistence(&forged_section_hash),
            Err(KgExportError::Conflict { .. })
        ));

        let mut forged_whole_hash = export.clone();
        forged_whole_hash.hashes.whole_export_hash = "22".repeat(32);
        assert!(matches!(
            validate_portable_export_for_persistence(&forged_whole_hash),
            Err(KgExportError::Conflict { .. })
        ));
    }

    #[test]
    fn export_scope_and_finality_request_validation_cover_fail_closed_edges() {
        let mut none_source_ref = base_scope();
        none_source_ref.source_commit_or_repo_ref = None;
        assert!(none_source_ref.validate().is_ok());

        let mut duplicate_filter = base_scope();
        duplicate_filter.included_memory_ids = vec!["memory-a".to_owned(), "memory-a".to_owned()];
        assert!(matches!(
            duplicate_filter.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut duplicate_graph_style = base_scope();
        duplicate_graph_style.included_graph_styles =
            vec!["semantic".to_owned(), "semantic".to_owned()];
        assert!(matches!(
            duplicate_graph_style.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut duplicate_writeback_key = base_scope();
        duplicate_writeback_key.included_writeback_idempotency_keys =
            vec!["idem-a".to_owned(), "idem-a".to_owned()];
        assert!(matches!(
            duplicate_writeback_key.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut empty_source_ref = base_scope();
        empty_source_ref.source_commit_or_repo_ref = Some(" \t\n ".to_owned());
        assert!(matches!(
            empty_source_ref.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut forbidden_source_ref = base_scope();
        forbidden_source_ref.source_commit_or_repo_ref =
            Some("DATABASE_URL=postgres://x".to_owned());
        assert!(matches!(
            forbidden_source_ref.validate(),
            Err(KgExportError::ForbiddenMaterial { .. })
        ));

        let valid_request = KgExportFinalityOutboxRequest {
            tenant_id: "tenant-test".to_owned(),
            namespace: "dag-db".to_owned(),
            export_id: "export-a".to_owned(),
            requester_did: "did:exo:exporter".to_owned(),
            idempotency_key: None,
        };
        assert!(valid_request.validate().is_ok());

        for mutate in [
            |item: &mut KgExportFinalityOutboxRequest| item.tenant_id = " ".to_owned(),
            |item: &mut KgExportFinalityOutboxRequest| item.namespace = " ".to_owned(),
            |item: &mut KgExportFinalityOutboxRequest| item.export_id = " ".to_owned(),
            |item: &mut KgExportFinalityOutboxRequest| item.requester_did = " ".to_owned(),
        ] {
            let mut request = valid_request.clone();
            mutate(&mut request);
            assert!(matches!(
                request.validate(),
                Err(KgExportError::InvalidScope { .. })
            ));
        }

        let mut bad_did = valid_request.clone();
        bad_did.requester_did = "exo:exporter".to_owned();
        assert!(matches!(
            bad_did.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut empty_key = valid_request.clone();
        empty_key.idempotency_key = Some(" \t\n ".to_owned());
        assert!(matches!(
            empty_key.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let mut forbidden_key = valid_request.clone();
        forbidden_key.idempotency_key = Some("postgres://exo:secret@localhost/db".to_owned());
        assert!(matches!(
            forbidden_key.validate(),
            Err(KgExportError::ForbiddenMaterial { .. })
        ));
    }

    #[test]
    fn export_private_helper_branch_vectors_cover_record_shapes() {
        let values = [
            json!("text"),
            json!(7),
            json!(true),
            json!(["a", "b"]),
            JsonValue::Null,
            json!({"nested": "value"}),
        ];
        for value in values {
            assert!(!value_key(&value).contains("forbidden"));
        }

        let unknown_lines = advisory_lines(&[KgExportRecord::new()]);
        assert_eq!(unknown_lines, vec!["- `unknown`: `unknown`"]);

        assert!(reject_forbidden_export_json(&json!([null, true, 1, "safe"]), "$").is_ok());
        assert_eq!(safe_count(usize::MAX), u32::MAX);
        assert_eq!(
            count_records_where_bool(&[KgExportRecord::new()], "missing", true),
            0
        );
    }

    #[test]
    fn export_artifact_writer_covers_parentless_paths_and_forbidden_payloads() {
        let export = rich_export();
        let json_path = std::path::Path::new("kg_export_parentless_report.json");
        let md_path = std::path::Path::new("kg_export_parentless_summary.md");

        let artifact_set =
            write_kg_export_artifacts(&export, json_path, md_path).expect("write parentless paths");
        assert_eq!(artifact_set.output_json, json_path);
        assert_eq!(artifact_set.output_md, md_path);
        let _ = std::fs::remove_file(json_path);
        let _ = std::fs::remove_file(md_path);

        let mut unsafe_export = export;
        unsafe_export.memory_records[0].insert("raw_prompt_body".to_owned(), json!("unsafe"));
        assert!(matches!(
            write_kg_export_artifacts(&unsafe_export, json_path, md_path),
            Err(KgExportError::ForbiddenMaterial { .. })
        ));
    }

    #[test]
    fn export_rejects_forbidden_material() {
        let unsafe_value = json!({
            "safe": "ok",
            "nested": {
                "database_url": "postgres://secret"
            }
        });
        assert!(reject_forbidden_export_json(&unsafe_value, "$").is_err());
        assert!(reject_forbidden_string("path", "/Users/example/private").is_err());
    }

    #[test]
    fn export_rejects_case_variant_forbidden_material() {
        let unsafe_key = json!({
            "safe": "ok",
            "nested": {
                "Raw_Markdown_Body": "forbidden"
            }
        });
        assert!(reject_forbidden_export_json(&unsafe_key, "$").is_err());

        for forbidden_value in [
            "/Users/example/private",
            "PostgreSQL://exo:secret@localhost/dagdb",
            "BEGIN PRIVATE KEY",
            "Authorization: Bearer token",
            "sk-proj-example",
            "source_excerpt leaked",
        ] {
            assert!(
                reject_forbidden_string("fixture", forbidden_value).is_err(),
                "expected forbidden value to fail: {forbidden_value}"
            );
        }
    }

    #[test]
    fn export_rejects_review_named_dynamic_raw_material_keys() {
        for key in [
            "payload",
            "private_payload",
            "Body",
            "body",
            "content",
            "source_body",
            "document_body",
            "prompt_body",
            "model_output",
            "raw_payload",
            "source_path",
            "text_body",
            "file_text",
            "file_path",
            "markdown",
        ] {
            let mut export = rich_export();
            export.memory_records[0].insert(key.to_owned(), json!("unsafe raw material"));

            assert!(
                matches!(
                    validate_portable_export_for_persistence(&export),
                    Err(KgExportError::ForbiddenMaterial { .. })
                ),
                "expected export to reject raw material key: {key}"
            );
        }
    }

    #[test]
    fn export_rejects_plain_bearer_marker_in_dynamic_record() {
        let mut export = rich_export();
        export.memory_records[0].insert("safe_note".to_owned(), json!("Bearer abc123"));

        assert!(matches!(
            validate_portable_export_for_persistence(&export),
            Err(KgExportError::ForbiddenMaterial { .. })
        ));
    }

    #[test]
    fn export_rejects_whitespace_only_required_fields() {
        let mut scope = base_scope();
        scope.tenant_id = " \t\n ".to_owned();
        assert!(matches!(
            scope.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));

        let request = KgExportFinalityOutboxRequest {
            tenant_id: "tenant-test".to_owned(),
            namespace: "dag-db".to_owned(),
            export_id: "export-1".to_owned(),
            requester_did: "did:exo:exporter".to_owned(),
            idempotency_key: Some("   ".to_owned()),
        };
        assert!(matches!(
            request.validate(),
            Err(KgExportError::InvalidScope { .. })
        ));
    }
}
