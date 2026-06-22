//! PRD17B context-packet persistence and proof-binding contracts.
//!
//! This module is intentionally deterministic and fail-closed. It does not
//! claim production approval, mutate route invalidation state, or persist raw
//! context bodies.

use std::collections::BTreeSet;

use exo_core::Did;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Schema version for persisted/proof-bound PRD17B context packet records.
pub const CONTEXT_PACKET_RECORD_SCHEMA_VERSION: &str = "dagdb_prd17_context_packet_record_v1";
/// Schema version for PRD17B context packet persistence reports.
pub const CONTEXT_PACKET_PERSISTENCE_REPORT_SCHEMA_VERSION: &str =
    "dagdb_prd17_context_packet_persistence_report_v1";
/// Minimum citation coverage for default retrieval packets.
pub const MIN_DEFAULT_CITATION_COVERAGE_BP: u16 = 8_000;
/// Required validation coverage for default retrieval packets.
pub const REQUIRED_DEFAULT_VALIDATION_COVERAGE_BP: u16 = 10_000;

const MAX_BP: u16 = 10_000;
const RAW_FORBIDDEN_FRAGMENTS: &[&str] = &[
    "/Users/",
    "\\Users\\",
    "/home/",
    "~/",
    "DATABASE_URL",
    "PRIVATE KEY",
    "authorization",
    "bearer ",
    ".env",
    "postgres://",
    "postgresql://",
    "raw_body",
    "raw_markdown",
    "raw_private_payload",
    "raw_prompt_body",
    "source_excerpt",
];
const EXTERNAL_PRODUCTION_APPROVAL_REF_PREFIX: &str = "external-production-approval:";
const EXTERNAL_PACKET_QUALITY_REVIEW_REF_PREFIX: &str = "external-packet-quality-review:";
const EXTERNAL_FINALITY_REF_PREFIX: &str = "external-finality:";
/// Bound route purpose for external context-packet finality.
pub const CONTEXT_PACKET_FINALITY_PURPOSE: &str = "dagdb.context_packet";
const MIN_APPROVAL_TIMESTAMP: &str = "2026-01-01T00:00:00Z";
const MAX_APPROVAL_TIMESTAMP: &str = "2038-01-19T03:14:07Z";

/// Context quality emitted by the PRD17B default retrieval runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultContextQuality {
    /// Packet has current route/packet state and bounded cited memory refs.
    UsableContext,
    /// No memory refs were selected.
    EmptyContext,
    /// Packet token estimate exceeded the configured budget.
    OverBudget,
    /// Route or packet freshness was stale.
    StaleContext,
    /// Route was forbidden by policy.
    ForbiddenRoute,
    /// Gateway was unavailable.
    GatewayUnavailable,
    /// Raw-context fallback was requested or detected.
    RawFallback,
}

/// Freshness status carried by default retrieval packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketFreshnessStatus {
    /// Route and packet refs are current.
    Current,
    /// One or more memory refs are stale.
    StaleMemory,
    /// Catalog refs are stale.
    StaleCatalog,
    /// Validation refs are stale.
    StaleValidation,
    /// The upstream route has been invalidated by PRD17C-compatible state.
    RouteInvalidated,
    /// Freshness is unknown and therefore not default-accepted.
    Unknown,
}

/// Validation status for the persisted/proof-bound packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketValidationStatus {
    /// Packet validation passed.
    Passed,
    /// Packet validation failed.
    Failed,
    /// Packet validation is stale.
    Stale,
    /// Packet validation is missing.
    Missing,
    /// Operator review is still deferred.
    OperatorDeferred,
}

/// Persistence status for a context packet record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketPersistenceStatus {
    /// Packet was persisted in the default packet table.
    Persisted,
    /// Packet is proof-bound to source proof refs.
    ProofBound,
    /// Packet is preview-only and cannot be accepted as default runtime.
    PreviewOnly,
    /// Packet is dry-run-only and cannot be accepted as default runtime.
    DryRunOnly,
    /// Packet exists only under target artifacts and cannot be accepted.
    TargetArtifactOnly,
}

/// Route binding copied into a context-packet record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketRouteBinding {
    /// Route identifier.
    pub route_id: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Project id.
    pub project_id: String,
    /// Memory namespace / DB set.
    pub memory_namespace: String,
    /// Production/default route approval status.
    pub production_default_route_approval_status: String,
    /// Packet quality review status.
    pub packet_quality_review_status: String,
    /// Route freshness status.
    pub route_freshness_status: PacketFreshnessStatus,
}

/// Operator/finality evidence required to accept a deferred context packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketAcceptanceEvidence {
    /// Production/default-route approval ref.
    pub production_default_route_approval_ref: String,
    /// Packet-quality review ref.
    pub packet_quality_review_ref: String,
    /// Finality receipt or outbox ref.
    pub finality_ref: String,
    /// Tenant verified by the external finality receipt.
    pub tenant_id: String,
    /// Namespace verified by the external finality receipt.
    pub memory_namespace: String,
    /// Actor whose request was finalized.
    pub actor_id: String,
    /// Route verified by the external finality receipt.
    pub route_id: String,
    /// Packet verified by the external finality receipt.
    pub packet_id: String,
    /// Route purpose verified by the external finality receipt.
    pub route_purpose: String,
    /// Request id verified by the receipt.
    pub request_id: String,
    /// Canonical payload hash approved by the external authority.
    pub payload_hash: String,
    /// Payload hash carried by the external finality receipt.
    pub receipt_payload_hash: String,
    /// External production authority DID.
    pub authority_did: String,
    /// External production authority signature.
    pub authority_signature: String,
    /// External approval timestamp.
    pub approved_at: String,
}

/// Caller input for building a PRD17B packet record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketRequest {
    /// Packet identifier.
    pub packet_id: String,
    /// Hash of the normalized retrieval query.
    pub query_hash: String,
    /// Selected memory ids.
    pub selected_memory_ids: Vec<String>,
    /// Selected graph or layer edge ids.
    pub selected_edge_ids: Vec<String>,
    /// Token budget supplied by the caller.
    pub token_budget: u32,
    /// Deterministic token estimate for the selected packet.
    pub token_estimate: u32,
    /// Citation coverage in basis points.
    pub citation_coverage_bp: u16,
    /// Validation coverage in basis points.
    pub validation_coverage_bp: u16,
    /// Source proof refs binding the record.
    pub source_proof_refs: Vec<String>,
    /// Context quality.
    pub context_quality: DefaultContextQuality,
    /// Freshness status.
    pub freshness_status: PacketFreshnessStatus,
    /// Packet validation status.
    pub validation_status: PacketValidationStatus,
    /// Persistence status.
    pub persistence_status: PacketPersistenceStatus,
    /// Optional explicit fallback reason.
    pub fallback_reason: Option<String>,
    /// True if any raw body field was detected before persistence.
    pub raw_body_present: bool,
    /// Stable creation timestamp or HLC string supplied by caller.
    pub created_at: String,
}

/// Persisted or proof-bound PRD17B context packet record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketRecord {
    /// Schema version.
    pub schema_version: String,
    /// Packet identifier.
    pub packet_id: String,
    /// Route identifier.
    pub route_id: String,
    /// Query hash.
    pub query_hash: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Project id.
    pub project_id: String,
    /// Memory namespace / DB set.
    pub memory_namespace: String,
    /// Selected memory ids.
    pub selected_memory_ids: Vec<String>,
    /// Selected graph or layer edge ids.
    pub selected_edge_ids: Vec<String>,
    /// Token budget.
    pub token_budget: u32,
    /// Token estimate.
    pub token_estimate: u32,
    /// Context quality.
    pub context_quality: DefaultContextQuality,
    /// Citation coverage in basis points.
    pub citation_coverage_bp: u16,
    /// Validation coverage in basis points.
    pub validation_coverage_bp: u16,
    /// Freshness status.
    pub freshness_status: PacketFreshnessStatus,
    /// Packet validation status.
    pub validation_status: PacketValidationStatus,
    /// Source proof refs.
    pub source_proof_refs: Vec<String>,
    /// Explicit fallback reason when non-default.
    pub fallback_reason: Option<String>,
    /// Idempotency key for `route_id:query_hash:token_budget`.
    pub idempotency_key: String,
    /// Persistence status.
    pub persistence_status: PacketPersistenceStatus,
    /// Production/default route approval status.
    pub production_default_route_approval_status: String,
    /// Packet quality review status.
    pub packet_quality_review_status: String,
    /// Creation timestamp or HLC string supplied by caller.
    pub created_at: String,
}

/// Machine-readable validator report for PRD17B packet persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketPersistenceReport {
    /// Report schema version.
    pub schema_version: String,
    /// Packet identifier.
    pub packet_id: String,
    /// Route identifier.
    pub route_id: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Project id.
    pub project_id: String,
    /// Memory namespace / DB set.
    pub memory_namespace: String,
    /// Persistence status.
    pub persistence_status: PacketPersistenceStatus,
    /// Validation status.
    pub validation_status: PacketValidationStatus,
    /// Context quality.
    pub context_quality: DefaultContextQuality,
    /// Freshness status.
    pub freshness_status: PacketFreshnessStatus,
    /// Selected memory count.
    pub memory_ref_count: u32,
    /// Selected edge count.
    pub selected_edge_count: u32,
    /// Token budget.
    pub token_budget: u32,
    /// Token estimate.
    pub token_estimate: u32,
    /// Citation coverage in basis points.
    pub citation_coverage_bp: u16,
    /// Validation coverage in basis points.
    pub validation_coverage_bp: u16,
    /// Idempotency key.
    pub idempotency_key: String,
    /// True only when every default acceptance gate passed.
    pub accepted: bool,
    /// True when operator-only gates are still deferred.
    pub operator_deferred: bool,
    /// Production/default route approval status.
    pub production_default_route_approval_status: String,
    /// Packet quality review status.
    pub packet_quality_review_status: String,
    /// Explicit fallback reason when non-default.
    pub fallback_reason: Option<String>,
    /// Rejection or deferral reasons.
    pub rejection_reasons: Vec<String>,
    /// Explicit non-claims.
    pub non_claims: Vec<String>,
}

/// Errors raised by PRD17B packet validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContextPacketError {
    /// Required field was empty.
    #[error("missing_required_field: {field}")]
    MissingRequiredField {
        /// Field name.
        field: &'static str,
    },
    /// Duplicate id was supplied.
    #[error("duplicate_id: {field}")]
    DuplicateId {
        /// Field name.
        field: &'static str,
    },
    /// Raw or forbidden material was detected.
    #[error("raw_material_rejected: {field}")]
    RawMaterialRejected {
        /// Field name.
        field: &'static str,
    },
    /// Token estimate exceeded budget.
    #[error("over_budget_packet")]
    OverBudgetPacket,
    /// Token budget or estimate was invalid.
    #[error("invalid_token_budget")]
    InvalidTokenBudget,
    /// Packet contained no selected memory refs.
    #[error("empty_packet")]
    EmptyPacket,
    /// Citation coverage was too low.
    #[error("low_citation_coverage")]
    LowCitationCoverage,
    /// Validation coverage was incomplete.
    #[error("incomplete_validation_coverage")]
    IncompleteValidationCoverage,
    /// Packet freshness was not current.
    #[error("stale_packet")]
    StalePacket,
    /// Context quality is not default-usable.
    #[error("non_default_context_quality")]
    NonDefaultContextQuality,
    /// Packet validation status is not passed.
    #[error("packet_validation_not_passed")]
    PacketValidationNotPassed,
    /// Packet is preview, dry-run, or target-only.
    #[error("non_persistent_packet")]
    NonPersistentPacket,
    /// Idempotency key did not match canonical material.
    #[error("idempotency_key_mismatch")]
    IdempotencyKeyMismatch,
    /// Basis points exceeded 10,000.
    #[error("basis_points_out_of_range: {field}")]
    BasisPointsOutOfRange {
        /// Field name.
        field: &'static str,
    },
    /// External finality evidence did not bind to the context packet.
    #[error("external_finality_mismatch: {field}")]
    ExternalFinalityMismatch {
        /// Mismatched field.
        field: String,
    },
}

/// Canonical approval hash the external context-packet authority must sign.
pub fn canonical_context_packet_approval_payload_hash(
    record: &ContextPacketRecord,
    actor_id: &str,
    request_id: &str,
    authority_did: &str,
    route_purpose: &str,
    approved_at: &str,
) -> Result<String, ContextPacketError> {
    sha256_hex_cbor(&ContextPacketApprovalMaterial {
        domain: "exo.dagdb.context_packet.external_finality.v1",
        schema_version: &record.schema_version,
        packet_id: &record.packet_id,
        route_id: &record.route_id,
        query_hash: &record.query_hash,
        request_id,
        idempotency_key: &record.idempotency_key,
        tenant_id: &record.tenant_id,
        project_id: &record.project_id,
        memory_namespace: &record.memory_namespace,
        selected_memory_ids: &record.selected_memory_ids,
        selected_edge_ids: &record.selected_edge_ids,
        token_budget: record.token_budget,
        token_estimate: record.token_estimate,
        context_quality: record.context_quality,
        citation_coverage_bp: record.citation_coverage_bp,
        validation_coverage_bp: record.validation_coverage_bp,
        freshness_status: record.freshness_status,
        validation_status: record.validation_status,
        source_proof_refs: &record.source_proof_refs,
        fallback_reason: record.fallback_reason.as_deref(),
        persistence_status: record.persistence_status,
        production_default_route_approval_status: &record.production_default_route_approval_status,
        packet_quality_review_status: &record.packet_quality_review_status,
        created_at: &record.created_at,
        actor_id,
        authority_did,
        route_purpose,
        approved_at,
    })
}

/// Build a context packet record with canonical idempotency.
pub fn build_context_packet_record(
    route: &ContextPacketRouteBinding,
    request: ContextPacketRequest,
) -> Result<ContextPacketRecord, ContextPacketError> {
    validate_binding(route)?;
    if request.raw_body_present {
        return Err(ContextPacketError::RawMaterialRejected {
            field: "raw_body_present",
        });
    }
    validate_required("packet_id", &request.packet_id)?;
    validate_required("query_hash", &request.query_hash)?;
    validate_required("created_at", &request.created_at)?;
    reject_forbidden("packet_id", &request.packet_id)?;
    reject_forbidden("query_hash", &request.query_hash)?;
    reject_forbidden("created_at", &request.created_at)?;
    validate_id_list("selected_memory_ids", &request.selected_memory_ids, true)?;
    validate_id_list("selected_edge_ids", &request.selected_edge_ids, false)?;
    validate_id_list("source_proof_refs", &request.source_proof_refs, true)?;

    let idempotency_key =
        canonical_idempotency_key(&route.route_id, &request.query_hash, request.token_budget);
    let record = ContextPacketRecord {
        schema_version: CONTEXT_PACKET_RECORD_SCHEMA_VERSION.to_owned(),
        packet_id: request.packet_id,
        route_id: route.route_id.clone(),
        query_hash: request.query_hash,
        tenant_id: route.tenant_id.clone(),
        project_id: route.project_id.clone(),
        memory_namespace: route.memory_namespace.clone(),
        selected_memory_ids: request.selected_memory_ids,
        selected_edge_ids: request.selected_edge_ids,
        token_budget: request.token_budget,
        token_estimate: request.token_estimate,
        context_quality: request.context_quality,
        citation_coverage_bp: request.citation_coverage_bp,
        validation_coverage_bp: request.validation_coverage_bp,
        freshness_status: request.freshness_status,
        validation_status: request.validation_status,
        source_proof_refs: request.source_proof_refs,
        fallback_reason: request.fallback_reason,
        idempotency_key,
        persistence_status: request.persistence_status,
        production_default_route_approval_status: route
            .production_default_route_approval_status
            .clone(),
        packet_quality_review_status: route.packet_quality_review_status.clone(),
        created_at: request.created_at,
    };
    validate_context_packet_record(&record)?;
    Ok(record)
}

/// Return an accepted copy after binding approval/finality evidence into proof refs.
pub fn accept_context_packet_record(
    record: &ContextPacketRecord,
    evidence: &ContextPacketAcceptanceEvidence,
) -> Result<ContextPacketRecord, ContextPacketError> {
    validate_context_packet_record(record)?;
    validate_acceptance_evidence(record, evidence)?;
    let mut accepted = record.clone();
    push_unique_proof_ref(
        &mut accepted.source_proof_refs,
        "production_default_route_approval",
        &evidence.production_default_route_approval_ref,
    )?;
    push_unique_proof_ref(
        &mut accepted.source_proof_refs,
        "packet_quality_review",
        &evidence.packet_quality_review_ref,
    )?;
    push_unique_proof_ref(
        &mut accepted.source_proof_refs,
        "finality",
        &evidence.finality_ref,
    )?;
    accepted.production_default_route_approval_status = "accepted".to_owned();
    accepted.packet_quality_review_status = "accepted".to_owned();
    validate_context_packet_record(&accepted)?;
    Ok(accepted)
}

/// Validate a context packet record for default-runtime eligibility.
pub fn validate_context_packet_record(
    record: &ContextPacketRecord,
) -> Result<(), ContextPacketError> {
    validate_required("schema_version", &record.schema_version)?;
    if record.schema_version != CONTEXT_PACKET_RECORD_SCHEMA_VERSION {
        return Err(ContextPacketError::MissingRequiredField {
            field: "schema_version",
        });
    }
    validate_binding(&ContextPacketRouteBinding {
        route_id: record.route_id.clone(),
        tenant_id: record.tenant_id.clone(),
        project_id: record.project_id.clone(),
        memory_namespace: record.memory_namespace.clone(),
        production_default_route_approval_status: record
            .production_default_route_approval_status
            .clone(),
        packet_quality_review_status: record.packet_quality_review_status.clone(),
        route_freshness_status: record.freshness_status,
    })?;
    for (field, value) in [
        ("packet_id", record.packet_id.as_str()),
        ("query_hash", record.query_hash.as_str()),
        ("created_at", record.created_at.as_str()),
    ] {
        validate_required(field, value)?;
        reject_forbidden(field, value)?;
    }
    validate_id_list("selected_memory_ids", &record.selected_memory_ids, true)?;
    validate_id_list("selected_edge_ids", &record.selected_edge_ids, false)?;
    validate_id_list("source_proof_refs", &record.source_proof_refs, true)?;
    validate_bp("citation_coverage_bp", record.citation_coverage_bp)?;
    validate_bp("validation_coverage_bp", record.validation_coverage_bp)?;
    if record.token_budget == 0 || record.token_estimate == 0 {
        return Err(ContextPacketError::InvalidTokenBudget);
    }
    if record.token_estimate > record.token_budget {
        return Err(ContextPacketError::OverBudgetPacket);
    }
    if record.selected_memory_ids.is_empty() {
        return Err(ContextPacketError::EmptyPacket);
    }
    if record.citation_coverage_bp < MIN_DEFAULT_CITATION_COVERAGE_BP {
        return Err(ContextPacketError::LowCitationCoverage);
    }
    if record.validation_coverage_bp < REQUIRED_DEFAULT_VALIDATION_COVERAGE_BP {
        return Err(ContextPacketError::IncompleteValidationCoverage);
    }
    if record.freshness_status != PacketFreshnessStatus::Current {
        return Err(ContextPacketError::StalePacket);
    }
    if record.context_quality != DefaultContextQuality::UsableContext {
        return Err(ContextPacketError::NonDefaultContextQuality);
    }
    if record.validation_status != PacketValidationStatus::Passed {
        return Err(ContextPacketError::PacketValidationNotPassed);
    }
    if !matches!(
        record.persistence_status,
        PacketPersistenceStatus::Persisted | PacketPersistenceStatus::ProofBound
    ) {
        return Err(ContextPacketError::NonPersistentPacket);
    }
    if record.idempotency_key
        != canonical_idempotency_key(&record.route_id, &record.query_hash, record.token_budget)
    {
        return Err(ContextPacketError::IdempotencyKeyMismatch);
    }
    if let Some(reason) = &record.fallback_reason {
        reject_forbidden("fallback_reason", reason)?;
    }
    Ok(())
}

/// Build the persistence validator report for a packet record.
pub fn build_context_packet_persistence_report(
    record: &ContextPacketRecord,
) -> ContextPacketPersistenceReport {
    let validation_result = validate_context_packet_record(record);
    let mut rejection_reasons = validation_result
        .as_ref()
        .err()
        .map(|error| vec![error.to_string()])
        .unwrap_or_default();
    let operator_deferred = record.production_default_route_approval_status != "accepted"
        || record.packet_quality_review_status != "accepted";
    if record.production_default_route_approval_status != "accepted" {
        rejection_reasons.push("production_default_route_approval_missing".to_owned());
    }
    if record.packet_quality_review_status != "accepted" {
        rejection_reasons.push("packet_quality_review_operator_deferred".to_owned());
    }
    let accepted = validation_result.is_ok() && !operator_deferred;
    ContextPacketPersistenceReport {
        schema_version: CONTEXT_PACKET_PERSISTENCE_REPORT_SCHEMA_VERSION.to_owned(),
        packet_id: record.packet_id.clone(),
        route_id: record.route_id.clone(),
        tenant_id: record.tenant_id.clone(),
        project_id: record.project_id.clone(),
        memory_namespace: record.memory_namespace.clone(),
        persistence_status: record.persistence_status,
        validation_status: record.validation_status,
        context_quality: record.context_quality,
        freshness_status: record.freshness_status,
        memory_ref_count: u32::try_from(record.selected_memory_ids.len()).unwrap_or(u32::MAX),
        selected_edge_count: u32::try_from(record.selected_edge_ids.len()).unwrap_or(u32::MAX),
        token_budget: record.token_budget,
        token_estimate: record.token_estimate,
        citation_coverage_bp: record.citation_coverage_bp,
        validation_coverage_bp: record.validation_coverage_bp,
        idempotency_key: record.idempotency_key.clone(),
        accepted,
        operator_deferred,
        production_default_route_approval_status: record
            .production_default_route_approval_status
            .clone(),
        packet_quality_review_status: record.packet_quality_review_status.clone(),
        fallback_reason: record.fallback_reason.clone(),
        rejection_reasons,
        non_claims: vec![
            "target_only_artifacts_are_not_proof".to_owned(),
            "raw_context_fallback_is_not_default_runtime".to_owned(),
            "operator_review_required_for_final_packet_quality".to_owned(),
        ],
    }
}

/// Canonical idempotency key for packet persistence.
#[must_use]
pub fn canonical_idempotency_key(route_id: &str, query_hash: &str, token_budget: u32) -> String {
    format!("{route_id}:{query_hash}:{token_budget}")
}

fn validate_binding(route: &ContextPacketRouteBinding) -> Result<(), ContextPacketError> {
    for (field, value) in [
        ("route_id", route.route_id.as_str()),
        ("tenant_id", route.tenant_id.as_str()),
        ("project_id", route.project_id.as_str()),
        ("memory_namespace", route.memory_namespace.as_str()),
        (
            "production_default_route_approval_status",
            route.production_default_route_approval_status.as_str(),
        ),
        (
            "packet_quality_review_status",
            route.packet_quality_review_status.as_str(),
        ),
    ] {
        validate_required(field, value)?;
        reject_forbidden(field, value)?;
    }
    Ok(())
}

fn validate_acceptance_evidence(
    record: &ContextPacketRecord,
    evidence: &ContextPacketAcceptanceEvidence,
) -> Result<(), ContextPacketError> {
    validate_external_ref(
        "production_default_route_approval_ref",
        &evidence.production_default_route_approval_ref,
        EXTERNAL_PRODUCTION_APPROVAL_REF_PREFIX,
    )?;
    validate_external_ref(
        "packet_quality_review_ref",
        &evidence.packet_quality_review_ref,
        EXTERNAL_PACKET_QUALITY_REVIEW_REF_PREFIX,
    )?;
    validate_external_ref(
        "finality_ref",
        &evidence.finality_ref,
        EXTERNAL_FINALITY_REF_PREFIX,
    )?;
    for (field, value) in [
        ("tenant_id", evidence.tenant_id.as_str()),
        ("memory_namespace", evidence.memory_namespace.as_str()),
        ("actor_id", evidence.actor_id.as_str()),
        ("route_id", evidence.route_id.as_str()),
        ("packet_id", evidence.packet_id.as_str()),
        ("route_purpose", evidence.route_purpose.as_str()),
        ("request_id", evidence.request_id.as_str()),
        ("payload_hash", evidence.payload_hash.as_str()),
        (
            "receipt_payload_hash",
            evidence.receipt_payload_hash.as_str(),
        ),
        ("authority_did", evidence.authority_did.as_str()),
        ("authority_signature", evidence.authority_signature.as_str()),
        ("approved_at", evidence.approved_at.as_str()),
    ] {
        validate_required(field, value)?;
        reject_forbidden(field, value)?;
    }
    validate_did("actor_id", &evidence.actor_id)?;
    validate_did("authority_did", &evidence.authority_did)?;
    validate_digest("payload_hash", &evidence.payload_hash)?;
    validate_digest("receipt_payload_hash", &evidence.receipt_payload_hash)?;
    validate_signature("authority_signature", &evidence.authority_signature)?;
    validate_approval_timestamp("approved_at", &evidence.approved_at)?;
    require_external_match("tenant_id", &evidence.tenant_id, &record.tenant_id)?;
    require_external_match(
        "memory_namespace",
        &evidence.memory_namespace,
        &record.memory_namespace,
    )?;
    require_external_match("route_id", &evidence.route_id, &record.route_id)?;
    require_external_match("packet_id", &evidence.packet_id, &record.packet_id)?;
    require_external_match("request_id", &evidence.request_id, &record.idempotency_key)?;
    require_external_match(
        "route_purpose",
        &evidence.route_purpose,
        CONTEXT_PACKET_FINALITY_PURPOSE,
    )?;
    let expected_payload_hash = canonical_context_packet_approval_payload_hash(
        record,
        &evidence.actor_id,
        &evidence.request_id,
        &evidence.authority_did,
        &evidence.route_purpose,
        &evidence.approved_at,
    )?;
    require_external_match(
        "payload_hash",
        &evidence.payload_hash,
        &expected_payload_hash,
    )?;
    require_external_match(
        "receipt_payload_hash",
        &evidence.receipt_payload_hash,
        &expected_payload_hash,
    )?;
    if evidence.authority_did == evidence.actor_id {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: "authority_did".to_owned(),
        });
    }
    Ok(())
}

#[derive(Serialize)]
struct ContextPacketApprovalMaterial<'a> {
    domain: &'static str,
    schema_version: &'a str,
    packet_id: &'a str,
    route_id: &'a str,
    query_hash: &'a str,
    request_id: &'a str,
    idempotency_key: &'a str,
    tenant_id: &'a str,
    project_id: &'a str,
    memory_namespace: &'a str,
    selected_memory_ids: &'a [String],
    selected_edge_ids: &'a [String],
    token_budget: u32,
    token_estimate: u32,
    context_quality: DefaultContextQuality,
    citation_coverage_bp: u16,
    validation_coverage_bp: u16,
    freshness_status: PacketFreshnessStatus,
    validation_status: PacketValidationStatus,
    source_proof_refs: &'a [String],
    fallback_reason: Option<&'a str>,
    persistence_status: PacketPersistenceStatus,
    production_default_route_approval_status: &'a str,
    packet_quality_review_status: &'a str,
    created_at: &'a str,
    actor_id: &'a str,
    authority_did: &'a str,
    route_purpose: &'a str,
    approved_at: &'a str,
}

fn validate_external_ref(
    field: &'static str,
    value: &str,
    prefix: &str,
) -> Result<(), ContextPacketError> {
    validate_required(field, value)?;
    reject_forbidden(field, value)?;
    if !value.starts_with(prefix) {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn require_external_match(
    field: &str,
    actual: &str,
    expected: &str,
) -> Result<(), ContextPacketError> {
    if actual != expected {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn validate_did(field: &str, value: &str) -> Result<(), ContextPacketError> {
    if Did::new(value).is_err() {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn validate_digest(field: &str, value: &str) -> Result<(), ContextPacketError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn validate_signature(field: &str, value: &str) -> Result<(), ContextPacketError> {
    if value.len() != 128 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    if value
        .as_bytes()
        .first()
        .is_some_and(|first| value.as_bytes().iter().all(|byte| byte == first))
    {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn validate_approval_timestamp(field: &str, value: &str) -> Result<(), ContextPacketError> {
    if value.len() != 20
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
        || value.as_bytes().get(10) != Some(&b'T')
        || value.as_bytes().get(13) != Some(&b':')
        || value.as_bytes().get(16) != Some(&b':')
        || value.as_bytes().get(19) != Some(&b'Z')
        || !value.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
        || !(MIN_APPROVAL_TIMESTAMP..=MAX_APPROVAL_TIMESTAMP).contains(&value)
    {
        return Err(ContextPacketError::ExternalFinalityMismatch {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn sha256_hex_cbor<T: Serialize>(value: &T) -> Result<String, ContextPacketError> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes).map_err(|_| {
        ContextPacketError::ExternalFinalityMismatch {
            field: "payload_hash".to_owned(),
        }
    })?;
    let digest = Sha256::digest(bytes);
    Ok(digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}

fn push_unique_proof_ref(
    refs: &mut Vec<String>,
    kind: &'static str,
    value: &str,
) -> Result<(), ContextPacketError> {
    let proof_ref = format!("{kind}:{value}");
    reject_forbidden("source_proof_refs", &proof_ref)?;
    if refs.contains(&proof_ref) {
        return Err(ContextPacketError::DuplicateId {
            field: "source_proof_refs",
        });
    }
    refs.push(proof_ref);
    Ok(())
}

fn validate_required(field: &'static str, value: &str) -> Result<(), ContextPacketError> {
    if value.trim().is_empty() {
        return Err(ContextPacketError::MissingRequiredField { field });
    }
    Ok(())
}

fn validate_bp(field: &'static str, value: u16) -> Result<(), ContextPacketError> {
    if value > MAX_BP {
        return Err(ContextPacketError::BasisPointsOutOfRange { field });
    }
    Ok(())
}

fn validate_id_list(
    field: &'static str,
    values: &[String],
    require_non_empty: bool,
) -> Result<(), ContextPacketError> {
    if require_non_empty && values.is_empty() {
        return Err(ContextPacketError::MissingRequiredField { field });
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_required(field, value)?;
        reject_forbidden(field, value)?;
        if !seen.insert(value) {
            return Err(ContextPacketError::DuplicateId { field });
        }
    }
    Ok(())
}

fn reject_forbidden(field: &'static str, value: &str) -> Result<(), ContextPacketError> {
    let lower = value.to_ascii_lowercase();
    for fragment in RAW_FORBIDDEN_FRAGMENTS {
        if lower.contains(&fragment.to_ascii_lowercase()) {
            return Err(ContextPacketError::RawMaterialRejected { field });
        }
    }
    Ok(())
}
