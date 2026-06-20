//! PRD17B default retrieval route contract.
//!
//! This module defines the default-route readiness gate and default packet
//! decision wrapper without mutating PRD17C route-invalidation ownership.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::context_packet_persistence::{
    ContextPacketRecord, ContextPacketRequest, ContextPacketRouteBinding, DefaultContextQuality,
    PacketFreshnessStatus, build_context_packet_record,
};

/// Schema version for PRD17B default route records.
pub const DEFAULT_ROUTE_SCHEMA_VERSION: &str = "dagdb_prd17_default_route_v1";
/// Schema version for PRD17B default-route readiness reports.
pub const DEFAULT_ROUTE_READINESS_REPORT_SCHEMA_VERSION: &str =
    "dagdb_prd17_default_route_readiness_report_v1";
/// Schema version for default context packet decisions.
pub const DEFAULT_CONTEXT_PACKET_DECISION_SCHEMA_VERSION: &str =
    "dagdb_prd17_default_context_packet_decision_v1";

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

/// Route status for PRD17B default retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultRouteStatus {
    /// Route is active.
    Active,
    /// Route is forbidden by policy.
    Forbidden,
    /// Route is stale.
    Stale,
    /// Route was invalidated by PRD17C-compatible state.
    Invalidated,
    /// Route came from a preview-only surface.
    PreviewOnly,
    /// Route came from a dry-run-only surface.
    DryRunOnly,
    /// Route is explicit non-default.
    NonDefault,
}

/// Source of a route record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultRouteSource {
    /// Persisted route record.
    Persisted,
    /// Preview-only route artifact.
    Preview,
    /// Dry-run-only route artifact.
    DryRun,
    /// Ignored target artifact only.
    TargetArtifact,
}

/// Route freshness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteFreshnessStatus {
    /// Route refs are current.
    Current,
    /// Memory refs are stale.
    StaleMemory,
    /// Catalog refs are stale.
    StaleCatalog,
    /// Validation refs are stale.
    StaleValidation,
    /// Route has been invalidated by PRD17C-compatible state.
    RouteInvalidated,
    /// Freshness is unknown.
    Unknown,
}

impl From<RouteFreshnessStatus> for PacketFreshnessStatus {
    fn from(value: RouteFreshnessStatus) -> Self {
        match value {
            RouteFreshnessStatus::Current => PacketFreshnessStatus::Current,
            RouteFreshnessStatus::StaleMemory => PacketFreshnessStatus::StaleMemory,
            RouteFreshnessStatus::StaleCatalog => PacketFreshnessStatus::StaleCatalog,
            RouteFreshnessStatus::StaleValidation => PacketFreshnessStatus::StaleValidation,
            RouteFreshnessStatus::RouteInvalidated => PacketFreshnessStatus::RouteInvalidated,
            RouteFreshnessStatus::Unknown => PacketFreshnessStatus::Unknown,
        }
    }
}

/// Readiness status emitted by the default-route validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultRuntimeReadinessStatus {
    /// Default route and packet quality gates are accepted.
    Accepted,
    /// Route is structurally ready but operator/default gates are deferred.
    OperatorDeferred,
    /// Route is explicit non-default.
    NonDefault,
    /// Route is rejected by a fail-closed validation gate.
    Rejected,
}

/// Failure codes for default retrieval paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultRetrievalFailureCode {
    /// No failure.
    None,
    /// Required tenant id missing.
    MissingTenant,
    /// Required project id missing.
    MissingProject,
    /// Required namespace missing.
    MissingNamespace,
    /// Route id missing.
    MissingRoute,
    /// Route is preview-only.
    PreviewOnlyRoute,
    /// Route is dry-run-only.
    DryRunOnlyRoute,
    /// Route is stale.
    StaleRoute,
    /// Route is forbidden.
    ForbiddenRoute,
    /// Route was invalidated.
    RouteInvalidated,
    /// Policy proof is missing or not allowed.
    MissingPolicy,
    /// Freshness proof is missing.
    MissingFreshness,
    /// No eligible memory refs.
    NoEligibleMemoryRefs,
    /// Production/default approval is missing.
    MissingProductionApproval,
    /// Packet quality review is missing.
    MissingPacketQualityReview,
    /// Raw material was rejected.
    RawMaterialRejected,
    /// Packet persistence validation rejected the packet.
    PacketPersistenceRejected,
    /// Packet was over budget.
    OverBudgetPacket,
    /// Packet was empty.
    EmptyPacket,
    /// Citation coverage was too low.
    LowCitationCoverage,
    /// Gateway was unavailable.
    GatewayUnavailable,
    /// Database was unavailable.
    DatabaseUnavailable,
}

/// Selected memory ref carried by an active default route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultRouteMemoryRef {
    /// Selected memory id.
    pub memory_id: String,
    /// Latest receipt hash.
    pub latest_receipt_hash: String,
    /// Validation status for this ref.
    pub validation_status: String,
    /// Citation reference.
    pub citation_ref: String,
}

/// Default route activation/readiness record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultRouteRecord {
    /// Schema version.
    pub schema_version: String,
    /// Route id.
    pub route_id: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Project id.
    pub project_id: String,
    /// Memory namespace / DB set.
    pub memory_namespace: String,
    /// Route status.
    pub status: DefaultRouteStatus,
    /// Route source.
    pub route_source: DefaultRouteSource,
    /// Policy proof ref.
    pub policy_ref: String,
    /// Freshness proof ref.
    pub freshness_ref: String,
    /// True when policy allows this route.
    pub policy_allowed: bool,
    /// Freshness status.
    pub freshness_status: RouteFreshnessStatus,
    /// True when PRD17C-compatible invalidation state marks route invalid.
    pub invalidated: bool,
    /// Production/default route approval status.
    pub production_default_route_approval_status: String,
    /// Packet quality review status.
    pub packet_quality_review_status: String,
    /// Selected memory refs.
    pub selected_memory_refs: Vec<DefaultRouteMemoryRef>,
    /// Creation timestamp or HLC string.
    pub created_at: String,
    /// Update timestamp or HLC string.
    pub updated_at: String,
}

/// Operator/finality evidence required to accept a deferred default route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultRouteAcceptanceEvidence {
    /// Production/default-route approval ref.
    pub production_default_route_approval_ref: String,
    /// Packet-quality review ref.
    pub packet_quality_review_ref: String,
    /// Finality receipt or outbox ref.
    pub finality_ref: String,
}

/// Readiness report consumed by PRD17A scoring later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultRouteReadinessReport {
    /// Report schema version.
    pub schema_version: String,
    /// Route id.
    pub route_id: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Project id.
    pub project_id: String,
    /// Memory namespace / DB set.
    pub memory_namespace: String,
    /// Readiness status.
    pub readiness_status: DefaultRuntimeReadinessStatus,
    /// Route status.
    pub route_status: DefaultRouteStatus,
    /// Route source.
    pub route_source: DefaultRouteSource,
    /// Freshness status.
    pub freshness_status: RouteFreshnessStatus,
    /// Active route count.
    pub active_route_count: u32,
    /// Forbidden route count.
    pub forbidden_route_count: u32,
    /// Stale route count.
    pub stale_route_count: u32,
    /// Invalidated route count.
    pub invalidated_route_count: u32,
    /// Fallback count.
    pub fallback_count: u32,
    /// Selected memory ref count.
    pub selected_memory_ref_count: u32,
    /// Primary failure code.
    pub primary_failure_code: DefaultRetrievalFailureCode,
    /// Production/default route approval status.
    pub production_default_route_approval_status: String,
    /// Packet quality review status.
    pub packet_quality_review_status: String,
    /// Rejection or deferral reasons.
    pub rejection_reasons: Vec<String>,
    /// Explicit non-claims.
    pub non_claims: Vec<String>,
}

/// Result of attempting to build the default context packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultContextPacketDecision {
    /// Decision schema version.
    pub schema_version: String,
    /// Default runtime readiness status.
    pub readiness_status: DefaultRuntimeReadinessStatus,
    /// Context quality for this request.
    pub context_quality: DefaultContextQuality,
    /// Route readiness report.
    pub route_report: DefaultRouteReadinessReport,
    /// Packet record when accepted route/packet gates passed.
    pub packet_record: Option<ContextPacketRecord>,
    /// Explicit fallback reason when non-default.
    pub fallback_reason: Option<String>,
    /// Failure code.
    pub failure_code: DefaultRetrievalFailureCode,
}

/// Errors raised by PRD17B route validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DefaultRouteError {
    /// Required field was empty.
    #[error("missing_required_field: {field}")]
    MissingRequiredField {
        /// Field name.
        field: &'static str,
    },
    /// Raw or forbidden material was detected.
    #[error("raw_material_rejected: {field}")]
    RawMaterialRejected {
        /// Field name.
        field: &'static str,
    },
    /// Duplicate selected memory id.
    #[error("duplicate_selected_memory_id")]
    DuplicateSelectedMemoryId,
    /// A requested memory id is not bound by the accepted route.
    #[error("selected_memory_not_in_route: {memory_id}")]
    SelectedMemoryNotInRoute {
        /// Offending memory id.
        memory_id: String,
    },
    /// The request claims fresher status than the route binding allows.
    #[error("request_freshness_outranks_route")]
    RequestFreshnessOutranksRoute,
    /// Acceptance evidence was present but an existing readiness gate still rejected the route.
    #[error("default_route_acceptance_gate_rejected: {failure_code:?}")]
    AcceptanceGateRejected {
        /// Existing readiness failure.
        failure_code: DefaultRetrievalFailureCode,
    },
}

/// Validate and evaluate a default route for PRD17B readiness.
pub fn evaluate_default_route_readiness(
    route: &DefaultRouteRecord,
) -> Result<DefaultRouteReadinessReport, DefaultRouteError> {
    validate_default_route_record(route)?;
    let mut rejection_reasons = Vec::new();
    let mut status = DefaultRuntimeReadinessStatus::Accepted;
    let mut failure_code = DefaultRetrievalFailureCode::None;
    let mut fallback_count = 0u32;

    match route.route_source {
        DefaultRouteSource::Preview => {
            status = DefaultRuntimeReadinessStatus::NonDefault;
            failure_code = DefaultRetrievalFailureCode::PreviewOnlyRoute;
            fallback_count = 1;
            rejection_reasons.push("preview_only_route_not_default".to_owned());
        }
        DefaultRouteSource::DryRun | DefaultRouteSource::TargetArtifact => {
            status = DefaultRuntimeReadinessStatus::NonDefault;
            failure_code = DefaultRetrievalFailureCode::DryRunOnlyRoute;
            fallback_count = 1;
            rejection_reasons.push("dry_run_or_target_only_route_not_default".to_owned());
        }
        DefaultRouteSource::Persisted => {}
    }

    if status == DefaultRuntimeReadinessStatus::Accepted {
        match route.status {
            DefaultRouteStatus::Active => {}
            DefaultRouteStatus::Forbidden => {
                status = DefaultRuntimeReadinessStatus::Rejected;
                failure_code = DefaultRetrievalFailureCode::ForbiddenRoute;
                rejection_reasons.push("route_status_forbidden".to_owned());
            }
            DefaultRouteStatus::Stale => {
                status = DefaultRuntimeReadinessStatus::Rejected;
                failure_code = DefaultRetrievalFailureCode::StaleRoute;
                rejection_reasons.push("route_status_stale".to_owned());
            }
            DefaultRouteStatus::Invalidated => {
                status = DefaultRuntimeReadinessStatus::Rejected;
                failure_code = DefaultRetrievalFailureCode::RouteInvalidated;
                rejection_reasons.push("route_status_invalidated".to_owned());
            }
            DefaultRouteStatus::PreviewOnly => {
                status = DefaultRuntimeReadinessStatus::NonDefault;
                failure_code = DefaultRetrievalFailureCode::PreviewOnlyRoute;
                fallback_count = 1;
                rejection_reasons.push("route_status_preview_only".to_owned());
            }
            DefaultRouteStatus::DryRunOnly | DefaultRouteStatus::NonDefault => {
                status = DefaultRuntimeReadinessStatus::NonDefault;
                failure_code = DefaultRetrievalFailureCode::DryRunOnlyRoute;
                fallback_count = 1;
                rejection_reasons.push("route_status_non_default".to_owned());
            }
        }
    }

    if status == DefaultRuntimeReadinessStatus::Accepted && !route.policy_allowed {
        status = DefaultRuntimeReadinessStatus::Rejected;
        failure_code = DefaultRetrievalFailureCode::MissingPolicy;
        rejection_reasons.push("policy_not_allowed".to_owned());
    }
    if status == DefaultRuntimeReadinessStatus::Accepted
        && route.freshness_status != RouteFreshnessStatus::Current
    {
        status = DefaultRuntimeReadinessStatus::Rejected;
        failure_code = match route.freshness_status {
            RouteFreshnessStatus::RouteInvalidated => DefaultRetrievalFailureCode::RouteInvalidated,
            RouteFreshnessStatus::Unknown => DefaultRetrievalFailureCode::MissingFreshness,
            _ => DefaultRetrievalFailureCode::StaleRoute,
        };
        rejection_reasons.push("freshness_not_current".to_owned());
    }
    if status == DefaultRuntimeReadinessStatus::Accepted && route.invalidated {
        status = DefaultRuntimeReadinessStatus::Rejected;
        failure_code = DefaultRetrievalFailureCode::RouteInvalidated;
        rejection_reasons.push("route_invalidated".to_owned());
    }
    if status == DefaultRuntimeReadinessStatus::Accepted && route.selected_memory_refs.is_empty() {
        status = DefaultRuntimeReadinessStatus::Rejected;
        failure_code = DefaultRetrievalFailureCode::NoEligibleMemoryRefs;
        rejection_reasons.push("no_eligible_memory_refs".to_owned());
    }
    if status == DefaultRuntimeReadinessStatus::Accepted
        && route.production_default_route_approval_status != "accepted"
    {
        status = DefaultRuntimeReadinessStatus::OperatorDeferred;
        failure_code = DefaultRetrievalFailureCode::MissingProductionApproval;
        rejection_reasons.push("production_default_route_approval_missing".to_owned());
    }
    if matches!(
        status,
        DefaultRuntimeReadinessStatus::Accepted | DefaultRuntimeReadinessStatus::OperatorDeferred
    ) && route.packet_quality_review_status != "accepted"
    {
        status = DefaultRuntimeReadinessStatus::OperatorDeferred;
        if failure_code == DefaultRetrievalFailureCode::None {
            failure_code = DefaultRetrievalFailureCode::MissingPacketQualityReview;
        }
        rejection_reasons.push("packet_quality_review_operator_deferred".to_owned());
    }

    Ok(DefaultRouteReadinessReport {
        schema_version: DEFAULT_ROUTE_READINESS_REPORT_SCHEMA_VERSION.to_owned(),
        route_id: route.route_id.clone(),
        tenant_id: route.tenant_id.clone(),
        project_id: route.project_id.clone(),
        memory_namespace: route.memory_namespace.clone(),
        readiness_status: status,
        route_status: route.status,
        route_source: route.route_source,
        freshness_status: route.freshness_status,
        active_route_count: if route.status == DefaultRouteStatus::Active {
            1
        } else {
            0
        },
        forbidden_route_count: if route.status == DefaultRouteStatus::Forbidden {
            1
        } else {
            0
        },
        stale_route_count: if matches!(
            route.status,
            DefaultRouteStatus::Stale | DefaultRouteStatus::Invalidated
        ) || !matches!(
            route.freshness_status,
            RouteFreshnessStatus::Current | RouteFreshnessStatus::Unknown
        ) {
            1
        } else {
            0
        },
        invalidated_route_count: if route.invalidated
            || matches!(route.status, DefaultRouteStatus::Invalidated)
            || route.freshness_status == RouteFreshnessStatus::RouteInvalidated
        {
            1
        } else {
            0
        },
        fallback_count,
        selected_memory_ref_count: u32::try_from(route.selected_memory_refs.len())
            .unwrap_or(u32::MAX),
        primary_failure_code: failure_code,
        production_default_route_approval_status: route
            .production_default_route_approval_status
            .clone(),
        packet_quality_review_status: route.packet_quality_review_status.clone(),
        rejection_reasons,
        non_claims: vec![
            "preview_reports_do_not_accept_default_routes".to_owned(),
            "target_only_artifacts_are_not_proof".to_owned(),
            "production_default_route_approval_is_operator_owned".to_owned(),
            "packet_quality_review_is_prd17a_operator_owned".to_owned(),
        ],
    })
}

/// Build the default context packet when the route is accepted.
pub fn build_default_context_packet(
    route: &DefaultRouteRecord,
    request: ContextPacketRequest,
) -> Result<DefaultContextPacketDecision, DefaultRouteError> {
    let route_report = evaluate_default_route_readiness(route)?;
    if route_report.readiness_status != DefaultRuntimeReadinessStatus::Accepted {
        return Ok(DefaultContextPacketDecision {
            schema_version: DEFAULT_CONTEXT_PACKET_DECISION_SCHEMA_VERSION.to_owned(),
            readiness_status: route_report.readiness_status,
            context_quality: DefaultContextQuality::StaleContext,
            fallback_reason: Some("default_route_not_accepted".to_owned()),
            failure_code: route_report.primary_failure_code,
            route_report,
            packet_record: None,
        });
    }

    let route_freshness: PacketFreshnessStatus = route.freshness_status.into();

    // Bind the packet to the route: every requested memory id must be one of the
    // route's selected memory refs (no smuggling arbitrary ids past the binding),
    // and the request must not claim a fresher status than the accepted route.
    let route_memory_ids: std::collections::BTreeSet<&str> = route
        .selected_memory_refs
        .iter()
        .map(|memory_ref| memory_ref.memory_id.as_str())
        .collect();
    for memory_id in &request.selected_memory_ids {
        if !route_memory_ids.contains(memory_id.as_str()) {
            return Err(DefaultRouteError::SelectedMemoryNotInRoute {
                memory_id: memory_id.clone(),
            });
        }
    }
    if request.freshness_status != route_freshness {
        return Err(DefaultRouteError::RequestFreshnessOutranksRoute);
    }

    let binding = ContextPacketRouteBinding {
        route_id: route.route_id.clone(),
        tenant_id: route.tenant_id.clone(),
        project_id: route.project_id.clone(),
        memory_namespace: route.memory_namespace.clone(),
        production_default_route_approval_status: route
            .production_default_route_approval_status
            .clone(),
        packet_quality_review_status: route.packet_quality_review_status.clone(),
        route_freshness_status: route_freshness,
    };
    match build_context_packet_record(&binding, request) {
        Ok(packet_record) => Ok(DefaultContextPacketDecision {
            schema_version: DEFAULT_CONTEXT_PACKET_DECISION_SCHEMA_VERSION.to_owned(),
            readiness_status: DefaultRuntimeReadinessStatus::Accepted,
            context_quality: DefaultContextQuality::UsableContext,
            route_report,
            packet_record: Some(packet_record),
            fallback_reason: None,
            failure_code: DefaultRetrievalFailureCode::None,
        }),
        Err(error) => Ok(DefaultContextPacketDecision {
            schema_version: DEFAULT_CONTEXT_PACKET_DECISION_SCHEMA_VERSION.to_owned(),
            readiness_status: DefaultRuntimeReadinessStatus::Rejected,
            context_quality: context_quality_for_packet_error(&error),
            route_report,
            packet_record: None,
            fallback_reason: Some(error.to_string()),
            failure_code: failure_code_for_packet_error(&error),
        }),
    }
}

/// Return an accepted default route only when validation, approval, and finality gates pass.
pub fn accept_default_route_record(
    route: &DefaultRouteRecord,
    evidence: &DefaultRouteAcceptanceEvidence,
    updated_at: String,
) -> Result<DefaultRouteRecord, DefaultRouteError> {
    validate_default_route_record(route)?;
    validate_acceptance_evidence(evidence)?;
    validate_required("updated_at", &updated_at)?;
    reject_forbidden("updated_at", &updated_at)?;

    let mut accepted = route.clone();
    accepted.production_default_route_approval_status = "accepted".to_owned();
    accepted.packet_quality_review_status = "accepted".to_owned();
    accepted.updated_at = updated_at;
    let report = evaluate_default_route_readiness(&accepted)?;
    if report.readiness_status != DefaultRuntimeReadinessStatus::Accepted {
        return Err(DefaultRouteError::AcceptanceGateRejected {
            failure_code: report.primary_failure_code,
        });
    }
    Ok(accepted)
}

/// Validate route record shape before readiness evaluation.
pub fn validate_default_route_record(route: &DefaultRouteRecord) -> Result<(), DefaultRouteError> {
    validate_required("schema_version", &route.schema_version)?;
    if route.schema_version != DEFAULT_ROUTE_SCHEMA_VERSION {
        return Err(DefaultRouteError::MissingRequiredField {
            field: "schema_version",
        });
    }
    for (field, value) in [
        ("route_id", route.route_id.as_str()),
        ("tenant_id", route.tenant_id.as_str()),
        ("project_id", route.project_id.as_str()),
        ("memory_namespace", route.memory_namespace.as_str()),
        ("policy_ref", route.policy_ref.as_str()),
        ("freshness_ref", route.freshness_ref.as_str()),
        (
            "production_default_route_approval_status",
            route.production_default_route_approval_status.as_str(),
        ),
        (
            "packet_quality_review_status",
            route.packet_quality_review_status.as_str(),
        ),
        ("created_at", route.created_at.as_str()),
        ("updated_at", route.updated_at.as_str()),
    ] {
        validate_required(field, value)?;
        reject_forbidden(field, value)?;
    }
    let mut seen = std::collections::BTreeSet::new();
    for selected in &route.selected_memory_refs {
        for (field, value) in [
            ("memory_id", selected.memory_id.as_str()),
            ("latest_receipt_hash", selected.latest_receipt_hash.as_str()),
            ("validation_status", selected.validation_status.as_str()),
            ("citation_ref", selected.citation_ref.as_str()),
        ] {
            validate_required(field, value)?;
            reject_forbidden(field, value)?;
        }
        if !seen.insert(&selected.memory_id) {
            return Err(DefaultRouteError::DuplicateSelectedMemoryId);
        }
    }
    Ok(())
}

fn validate_acceptance_evidence(
    evidence: &DefaultRouteAcceptanceEvidence,
) -> Result<(), DefaultRouteError> {
    for (field, value) in [
        (
            "production_default_route_approval_ref",
            evidence.production_default_route_approval_ref.as_str(),
        ),
        (
            "packet_quality_review_ref",
            evidence.packet_quality_review_ref.as_str(),
        ),
        ("finality_ref", evidence.finality_ref.as_str()),
    ] {
        validate_required(field, value)?;
        reject_forbidden(field, value)?;
    }
    Ok(())
}

fn context_quality_for_packet_error(
    error: &crate::context_packet_persistence::ContextPacketError,
) -> DefaultContextQuality {
    use crate::context_packet_persistence::ContextPacketError;
    match error {
        ContextPacketError::OverBudgetPacket | ContextPacketError::InvalidTokenBudget => {
            DefaultContextQuality::OverBudget
        }
        ContextPacketError::EmptyPacket => DefaultContextQuality::EmptyContext,
        ContextPacketError::StalePacket => DefaultContextQuality::StaleContext,
        ContextPacketError::RawMaterialRejected { .. } => DefaultContextQuality::RawFallback,
        _ => DefaultContextQuality::ForbiddenRoute,
    }
}

fn failure_code_for_packet_error(
    error: &crate::context_packet_persistence::ContextPacketError,
) -> DefaultRetrievalFailureCode {
    use crate::context_packet_persistence::ContextPacketError;
    match error {
        ContextPacketError::OverBudgetPacket | ContextPacketError::InvalidTokenBudget => {
            DefaultRetrievalFailureCode::OverBudgetPacket
        }
        ContextPacketError::EmptyPacket => DefaultRetrievalFailureCode::EmptyPacket,
        ContextPacketError::LowCitationCoverage => DefaultRetrievalFailureCode::LowCitationCoverage,
        ContextPacketError::RawMaterialRejected { .. } => {
            DefaultRetrievalFailureCode::RawMaterialRejected
        }
        _ => DefaultRetrievalFailureCode::PacketPersistenceRejected,
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), DefaultRouteError> {
    if value.trim().is_empty() {
        return Err(DefaultRouteError::MissingRequiredField { field });
    }
    Ok(())
}

fn reject_forbidden(field: &'static str, value: &str) -> Result<(), DefaultRouteError> {
    let lower = value.to_ascii_lowercase();
    for fragment in RAW_FORBIDDEN_FRAGMENTS {
        if lower.contains(&fragment.to_ascii_lowercase()) {
            return Err(DefaultRouteError::RawMaterialRejected { field });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_packet_persistence::{
        ContextPacketError, PacketPersistenceStatus, PacketValidationStatus,
    };

    fn memory_ref(memory_id: &str) -> DefaultRouteMemoryRef {
        DefaultRouteMemoryRef {
            memory_id: memory_id.to_owned(),
            latest_receipt_hash: format!("receipt-{memory_id}"),
            validation_status: "accepted".to_owned(),
            citation_ref: format!("citation-{memory_id}"),
        }
    }

    fn accepted_route() -> DefaultRouteRecord {
        DefaultRouteRecord {
            schema_version: DEFAULT_ROUTE_SCHEMA_VERSION.to_owned(),
            route_id: "route-default".to_owned(),
            tenant_id: "tenant-alpha".to_owned(),
            project_id: "project-dagdb".to_owned(),
            memory_namespace: "namespace-main".to_owned(),
            status: DefaultRouteStatus::Active,
            route_source: DefaultRouteSource::Persisted,
            policy_ref: "policy-proof-1".to_owned(),
            freshness_ref: "freshness-proof-1".to_owned(),
            policy_allowed: true,
            freshness_status: RouteFreshnessStatus::Current,
            invalidated: false,
            production_default_route_approval_status: "accepted".to_owned(),
            packet_quality_review_status: "accepted".to_owned(),
            selected_memory_refs: vec![memory_ref("memory-1")],
            created_at: "hlc-created-1".to_owned(),
            updated_at: "hlc-updated-1".to_owned(),
        }
    }

    fn accepted_request() -> ContextPacketRequest {
        ContextPacketRequest {
            packet_id: "packet-1".to_owned(),
            query_hash: "query-hash-1".to_owned(),
            selected_memory_ids: vec!["memory-1".to_owned()],
            selected_edge_ids: vec!["edge-1".to_owned()],
            token_budget: 1_000,
            token_estimate: 250,
            citation_coverage_bp: 10_000,
            validation_coverage_bp: 10_000,
            source_proof_refs: vec!["source-proof-1".to_owned()],
            context_quality: DefaultContextQuality::UsableContext,
            freshness_status: PacketFreshnessStatus::Current,
            validation_status: PacketValidationStatus::Passed,
            persistence_status: PacketPersistenceStatus::ProofBound,
            fallback_reason: None,
            raw_body_present: false,
            created_at: "hlc-packet-1".to_owned(),
        }
    }

    fn readiness_report(route: &DefaultRouteRecord) -> DefaultRouteReadinessReport {
        evaluate_default_route_readiness(route).expect("route shape should validate")
    }

    fn has_reason(report: &DefaultRouteReadinessReport, reason: &str) -> bool {
        report
            .rejection_reasons
            .iter()
            .any(|actual| actual == reason)
    }

    fn rejected_packet_decision(request: ContextPacketRequest) -> DefaultContextPacketDecision {
        let decision = build_default_context_packet(&accepted_route(), request)
            .expect("accepted route should convert packet validation into a decision");
        assert_eq!(
            decision.readiness_status,
            DefaultRuntimeReadinessStatus::Rejected
        );
        assert!(decision.packet_record.is_none());
        decision
    }

    fn assert_packet_rejection(
        request: ContextPacketRequest,
        context_quality: DefaultContextQuality,
        failure_code: DefaultRetrievalFailureCode,
        fallback_fragment: &str,
    ) {
        let decision = rejected_packet_decision(request);
        assert_eq!(decision.context_quality, context_quality);
        assert_eq!(decision.failure_code, failure_code);
        assert!(
            decision
                .fallback_reason
                .as_deref()
                .is_some_and(|reason| reason.contains(fallback_fragment)),
            "fallback reason should mention {fallback_fragment:?}: {:?}",
            decision.fallback_reason
        );
    }

    #[test]
    fn route_freshness_status_maps_to_packet_freshness_status() {
        let cases = [
            (
                RouteFreshnessStatus::Current,
                PacketFreshnessStatus::Current,
            ),
            (
                RouteFreshnessStatus::StaleMemory,
                PacketFreshnessStatus::StaleMemory,
            ),
            (
                RouteFreshnessStatus::StaleCatalog,
                PacketFreshnessStatus::StaleCatalog,
            ),
            (
                RouteFreshnessStatus::StaleValidation,
                PacketFreshnessStatus::StaleValidation,
            ),
            (
                RouteFreshnessStatus::RouteInvalidated,
                PacketFreshnessStatus::RouteInvalidated,
            ),
            (
                RouteFreshnessStatus::Unknown,
                PacketFreshnessStatus::Unknown,
            ),
        ];

        for (route_freshness, packet_freshness) in cases {
            assert_eq!(
                PacketFreshnessStatus::from(route_freshness),
                packet_freshness
            );
        }
    }

    #[test]
    fn accepted_route_builds_usable_packet_decision() {
        let route = accepted_route();
        let decision = build_default_context_packet(&route, accepted_request())
            .expect("accepted route and packet should build");

        assert_eq!(
            decision.readiness_status,
            DefaultRuntimeReadinessStatus::Accepted
        );
        assert_eq!(
            decision.context_quality,
            DefaultContextQuality::UsableContext
        );
        assert_eq!(decision.failure_code, DefaultRetrievalFailureCode::None);
        assert!(decision.fallback_reason.is_none());

        let record = decision.packet_record.expect("packet record should exist");
        assert_eq!(record.route_id, route.route_id);
        assert_eq!(record.tenant_id, route.tenant_id);
        assert_eq!(record.project_id, route.project_id);
        assert_eq!(record.memory_namespace, route.memory_namespace);
        assert_eq!(record.selected_memory_ids, vec!["memory-1".to_owned()]);
        assert_eq!(record.freshness_status, PacketFreshnessStatus::Current);
    }

    #[test]
    fn non_persisted_route_sources_are_non_default() {
        let cases = [
            (
                DefaultRouteSource::Preview,
                DefaultRetrievalFailureCode::PreviewOnlyRoute,
                "preview_only_route_not_default",
            ),
            (
                DefaultRouteSource::DryRun,
                DefaultRetrievalFailureCode::DryRunOnlyRoute,
                "dry_run_or_target_only_route_not_default",
            ),
            (
                DefaultRouteSource::TargetArtifact,
                DefaultRetrievalFailureCode::DryRunOnlyRoute,
                "dry_run_or_target_only_route_not_default",
            ),
        ];

        for (route_source, failure_code, reason) in cases {
            let mut route = accepted_route();
            route.route_source = route_source;

            let report = readiness_report(&route);
            assert_eq!(
                report.readiness_status,
                DefaultRuntimeReadinessStatus::NonDefault
            );
            assert_eq!(report.route_source, route_source);
            assert_eq!(report.primary_failure_code, failure_code);
            assert_eq!(report.fallback_count, 1);
            assert!(has_reason(&report, reason));
        }
    }

    #[test]
    fn route_status_rejections_and_non_default_statuses_are_reported() {
        let cases = [
            (
                DefaultRouteStatus::Forbidden,
                DefaultRuntimeReadinessStatus::Rejected,
                DefaultRetrievalFailureCode::ForbiddenRoute,
                "route_status_forbidden",
                0,
            ),
            (
                DefaultRouteStatus::Stale,
                DefaultRuntimeReadinessStatus::Rejected,
                DefaultRetrievalFailureCode::StaleRoute,
                "route_status_stale",
                0,
            ),
            (
                DefaultRouteStatus::Invalidated,
                DefaultRuntimeReadinessStatus::Rejected,
                DefaultRetrievalFailureCode::RouteInvalidated,
                "route_status_invalidated",
                0,
            ),
            (
                DefaultRouteStatus::PreviewOnly,
                DefaultRuntimeReadinessStatus::NonDefault,
                DefaultRetrievalFailureCode::PreviewOnlyRoute,
                "route_status_preview_only",
                1,
            ),
            (
                DefaultRouteStatus::DryRunOnly,
                DefaultRuntimeReadinessStatus::NonDefault,
                DefaultRetrievalFailureCode::DryRunOnlyRoute,
                "route_status_non_default",
                1,
            ),
            (
                DefaultRouteStatus::NonDefault,
                DefaultRuntimeReadinessStatus::NonDefault,
                DefaultRetrievalFailureCode::DryRunOnlyRoute,
                "route_status_non_default",
                1,
            ),
        ];

        for (route_status, readiness_status, failure_code, reason, fallback_count) in cases {
            let mut route = accepted_route();
            route.status = route_status;

            let report = readiness_report(&route);
            assert_eq!(report.readiness_status, readiness_status);
            assert_eq!(report.route_status, route_status);
            assert_eq!(report.primary_failure_code, failure_code);
            assert_eq!(report.fallback_count, fallback_count);
            assert!(has_reason(&report, reason));
        }
    }

    #[test]
    fn freshness_policy_invalidation_and_empty_memory_gates_reject() {
        let freshness_cases = [
            (
                RouteFreshnessStatus::StaleMemory,
                DefaultRetrievalFailureCode::StaleRoute,
                1,
                0,
            ),
            (
                RouteFreshnessStatus::StaleCatalog,
                DefaultRetrievalFailureCode::StaleRoute,
                1,
                0,
            ),
            (
                RouteFreshnessStatus::StaleValidation,
                DefaultRetrievalFailureCode::StaleRoute,
                1,
                0,
            ),
            (
                RouteFreshnessStatus::RouteInvalidated,
                DefaultRetrievalFailureCode::RouteInvalidated,
                1,
                1,
            ),
            (
                RouteFreshnessStatus::Unknown,
                DefaultRetrievalFailureCode::MissingFreshness,
                0,
                0,
            ),
        ];

        for (freshness_status, failure_code, stale_count, invalidated_count) in freshness_cases {
            let mut route = accepted_route();
            route.freshness_status = freshness_status;

            let report = readiness_report(&route);
            assert_eq!(
                report.readiness_status,
                DefaultRuntimeReadinessStatus::Rejected
            );
            assert_eq!(report.primary_failure_code, failure_code);
            assert_eq!(report.stale_route_count, stale_count);
            assert_eq!(report.invalidated_route_count, invalidated_count);
            assert!(has_reason(&report, "freshness_not_current"));
        }

        let mut missing_policy = accepted_route();
        missing_policy.policy_allowed = false;
        let report = readiness_report(&missing_policy);
        assert_eq!(
            report.readiness_status,
            DefaultRuntimeReadinessStatus::Rejected
        );
        assert_eq!(
            report.primary_failure_code,
            DefaultRetrievalFailureCode::MissingPolicy
        );
        assert!(has_reason(&report, "policy_not_allowed"));

        let mut invalidated = accepted_route();
        invalidated.invalidated = true;
        let report = readiness_report(&invalidated);
        assert_eq!(
            report.primary_failure_code,
            DefaultRetrievalFailureCode::RouteInvalidated
        );
        assert_eq!(report.invalidated_route_count, 1);
        assert!(has_reason(&report, "route_invalidated"));

        let mut empty_refs = accepted_route();
        empty_refs.selected_memory_refs.clear();
        let report = readiness_report(&empty_refs);
        assert_eq!(
            report.primary_failure_code,
            DefaultRetrievalFailureCode::NoEligibleMemoryRefs
        );
        assert_eq!(report.selected_memory_ref_count, 0);
        assert!(has_reason(&report, "no_eligible_memory_refs"));
    }

    #[test]
    fn operator_review_gates_defer_without_accepting_default_runtime() {
        let mut missing_packet_review = accepted_route();
        missing_packet_review.packet_quality_review_status = "pending".to_owned();
        let report = readiness_report(&missing_packet_review);
        assert_eq!(
            report.readiness_status,
            DefaultRuntimeReadinessStatus::OperatorDeferred
        );
        assert_eq!(
            report.primary_failure_code,
            DefaultRetrievalFailureCode::MissingPacketQualityReview
        );
        assert!(has_reason(
            &report,
            "packet_quality_review_operator_deferred"
        ));

        let mut missing_both_operator_reviews = accepted_route();
        missing_both_operator_reviews.production_default_route_approval_status =
            "pending".to_owned();
        missing_both_operator_reviews.packet_quality_review_status = "pending".to_owned();
        let report = readiness_report(&missing_both_operator_reviews);
        assert_eq!(
            report.readiness_status,
            DefaultRuntimeReadinessStatus::OperatorDeferred
        );
        assert_eq!(
            report.primary_failure_code,
            DefaultRetrievalFailureCode::MissingProductionApproval
        );
        assert!(has_reason(
            &report,
            "production_default_route_approval_missing"
        ));
        assert!(has_reason(
            &report,
            "packet_quality_review_operator_deferred"
        ));
    }

    #[test]
    fn non_accepted_route_builds_fallback_decision_without_packet() {
        let mut route = accepted_route();
        route.route_source = DefaultRouteSource::Preview;

        let decision = build_default_context_packet(&route, accepted_request())
            .expect("non-default route should produce fallback decision");
        assert_eq!(
            decision.readiness_status,
            DefaultRuntimeReadinessStatus::NonDefault
        );
        assert_eq!(
            decision.context_quality,
            DefaultContextQuality::StaleContext
        );
        assert_eq!(
            decision.failure_code,
            DefaultRetrievalFailureCode::PreviewOnlyRoute
        );
        assert_eq!(
            decision.fallback_reason.as_deref(),
            Some("default_route_not_accepted")
        );
        assert!(decision.packet_record.is_none());
    }

    #[test]
    fn accepted_route_rejects_unbound_memory_and_freshness_mismatch() {
        let route = accepted_route();

        let mut unbound_memory = accepted_request();
        unbound_memory.selected_memory_ids = vec!["memory-other".to_owned()];
        let error =
            build_default_context_packet(&route, unbound_memory).expect_err("memory must bind");
        assert_eq!(
            error,
            DefaultRouteError::SelectedMemoryNotInRoute {
                memory_id: "memory-other".to_owned()
            }
        );

        let mut stale_claim = accepted_request();
        stale_claim.freshness_status = PacketFreshnessStatus::Unknown;
        let error =
            build_default_context_packet(&route, stale_claim).expect_err("freshness must bind");
        assert_eq!(error, DefaultRouteError::RequestFreshnessOutranksRoute);
    }

    #[test]
    fn packet_validation_errors_map_to_default_packet_decisions() {
        let mut over_budget = accepted_request();
        over_budget.token_estimate = over_budget.token_budget + 1;
        assert_packet_rejection(
            over_budget,
            DefaultContextQuality::OverBudget,
            DefaultRetrievalFailureCode::OverBudgetPacket,
            "over_budget_packet",
        );

        let mut invalid_budget = accepted_request();
        invalid_budget.token_budget = 0;
        assert_packet_rejection(
            invalid_budget,
            DefaultContextQuality::OverBudget,
            DefaultRetrievalFailureCode::OverBudgetPacket,
            "invalid_token_budget",
        );

        let mut low_citation = accepted_request();
        low_citation.citation_coverage_bp = 7_999;
        assert_packet_rejection(
            low_citation,
            DefaultContextQuality::ForbiddenRoute,
            DefaultRetrievalFailureCode::LowCitationCoverage,
            "low_citation_coverage",
        );

        let mut raw_packet = accepted_request();
        raw_packet.raw_body_present = true;
        assert_packet_rejection(
            raw_packet,
            DefaultContextQuality::RawFallback,
            DefaultRetrievalFailureCode::RawMaterialRejected,
            "raw_material_rejected",
        );

        let mut duplicate_proof = accepted_request();
        duplicate_proof
            .source_proof_refs
            .push("source-proof-1".to_owned());
        assert_packet_rejection(
            duplicate_proof,
            DefaultContextQuality::ForbiddenRoute,
            DefaultRetrievalFailureCode::PacketPersistenceRejected,
            "duplicate_id",
        );
    }

    #[test]
    fn unreachable_packet_error_mappings_are_still_deterministic() {
        assert_eq!(
            context_quality_for_packet_error(&ContextPacketError::EmptyPacket),
            DefaultContextQuality::EmptyContext
        );
        assert_eq!(
            failure_code_for_packet_error(&ContextPacketError::EmptyPacket),
            DefaultRetrievalFailureCode::EmptyPacket
        );
        assert_eq!(
            context_quality_for_packet_error(&ContextPacketError::StalePacket),
            DefaultContextQuality::StaleContext
        );
        assert_eq!(
            failure_code_for_packet_error(&ContextPacketError::StalePacket),
            DefaultRetrievalFailureCode::PacketPersistenceRejected
        );
    }

    #[test]
    fn route_validation_rejects_schema_duplicates_forbidden_and_missing_fields() {
        let mut wrong_schema = accepted_route();
        wrong_schema.schema_version = "dagdb_prd17_default_route_v2".to_owned();
        assert_eq!(
            validate_default_route_record(&wrong_schema),
            Err(DefaultRouteError::MissingRequiredField {
                field: "schema_version"
            })
        );

        let mut duplicate_memory = accepted_route();
        duplicate_memory
            .selected_memory_refs
            .push(memory_ref("memory-1"));
        assert_eq!(
            validate_default_route_record(&duplicate_memory),
            Err(DefaultRouteError::DuplicateSelectedMemoryId)
        );

        let mut forbidden_route_field = accepted_route();
        forbidden_route_field.policy_ref = "postgres://local-secret".to_owned();
        assert_eq!(
            validate_default_route_record(&forbidden_route_field),
            Err(DefaultRouteError::RawMaterialRejected {
                field: "policy_ref"
            })
        );

        let mut forbidden_memory_ref = accepted_route();
        forbidden_memory_ref.selected_memory_refs[0].citation_ref =
            "source_excerpt: raw".to_owned();
        assert_eq!(
            validate_default_route_record(&forbidden_memory_ref),
            Err(DefaultRouteError::RawMaterialRejected {
                field: "citation_ref"
            })
        );

        let mut missing_route_id = accepted_route();
        missing_route_id.route_id = " ".to_owned();
        assert_eq!(
            validate_default_route_record(&missing_route_id),
            Err(DefaultRouteError::MissingRequiredField { field: "route_id" })
        );
    }
}
