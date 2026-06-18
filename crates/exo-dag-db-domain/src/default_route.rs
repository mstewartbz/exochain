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
