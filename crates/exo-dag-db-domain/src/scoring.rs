//! Deterministic domain gates and integer scoring for ExoChain DAG DB.

use exo_authority::{Permission, PermissionSet};
use exo_avc::AuthorityScope;
use exo_consent::ConsentDecision;
use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{CouncilReviewStatus, RiskClass, SafeMetadata};
use exo_identity::risk::RiskLevel;
use thiserror::Error;

use crate::{
    council::{ApprovalScope, CouncilError, require_approval_for_risk},
    metadata::MetadataError,
    model::{CouncilDecision, DagDbAuthorizedScope},
};

/// Basis-point maximum used across all DAG DB scoring formulas.
pub const MAX_BP: u16 = 10_000;
const AGENT_SAFETY_MIN_PASS_BP: u16 = 7_500;
const AGENT_SAFETY_COUNCIL_MIN_BP: u16 = 6_500;
const REQUIRED_COMPONENT_MIN_BP: u16 = 7_000;
const MAX_INCIDENT_PENALTY_BP: u16 = 2_000;
const ROUTE_STALE_AFTER_MS: u64 = 86_400_000;
const MEMORY_STALE_AFTER_MS: u64 = 7_776_000_000;
const TOKEN_BUDGET_RESERVE: u32 = 256;
const BENCHMARK_QUALITY_MIN_BP: u16 = 8_500;
const BENCHMARK_CITATION_MIN_BP: u16 = 9_500;
const BENCHMARK_UNSUPPORTED_MAX_BP: u16 = 500;

/// Result alias used by pure DAG DB domain services.
pub type DomainResult<T> = std::result::Result<T, DomainError>;

/// Fail-closed domain errors produced before persistence is mutated.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// User-provided tenant or namespace does not match authenticated scope.
    #[error("tenant_scope_mismatch")]
    TenantScopeMismatch {
        /// Tenant from the preverified scope.
        expected_tenant_id: String,
        /// Namespace from the preverified scope.
        expected_namespace: String,
        /// Tenant from the domain input.
        actual_tenant_id: String,
        /// Namespace from the domain input.
        actual_namespace: String,
    },
    /// Authenticated authority does not permit the requested action.
    #[error("authority_denied: {action}")]
    AuthorityDenied {
        /// Action that was denied.
        action: String,
    },
    /// Consent gate denied the requested action.
    #[error("consent_denied: {action}")]
    ConsentDenied {
        /// Action that was denied.
        action: String,
    },
    /// Metadata sanitizer rejected runtime text before persistence.
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    /// Duplicate active memory already exists.
    #[error("duplicate_active_memory")]
    DuplicateActiveMemory {
        /// Existing active memory ID.
        memory_id: Hash256,
    },
    /// R3-R5 action has no durable approval.
    #[error("approval_required")]
    ApprovalRequired,
    /// Durable approval denied the action.
    #[error("approval_denied")]
    ApprovalDenied,
    /// Durable approval escalated the action.
    #[error("council_escalation_required")]
    CouncilEscalationRequired,
    /// Durable approval scope did not match the action.
    #[error("approval_scope_mismatch")]
    ApprovalScopeMismatch,
    /// No eligible memory remains after domain filters.
    #[error("no_eligible_memory")]
    NoEligibleMemory,
    /// Subject finality is not committed.
    #[error("non_committed_finality")]
    NonCommittedFinality {
        /// Subject ID blocked by finality.
        subject_id: Hash256,
    },
    /// Revoked memory cannot be used.
    #[error("revoked_memory")]
    RevokedMemory {
        /// Revoked memory ID.
        memory_id: Hash256,
    },
    /// Superseded memory cannot be used directly.
    #[error("superseded_memory")]
    SupersededMemory {
        /// Superseded memory ID.
        memory_id: Hash256,
        /// Replacement memory ID when known.
        superseded_by_memory_id: Option<Hash256>,
    },
    /// Route is stale.
    #[error("stale_route")]
    StaleRoute,
    /// Contradictory validation blocks activation.
    #[error("contradictory_validation")]
    ContradictoryValidation {
        /// Subject whose validation contradicted prior reports.
        subject_id: Hash256,
    },
    /// Token estimate exceeds budget including reserve.
    #[error("token_budget_exceeded")]
    TokenBudgetExceeded {
        /// Deterministic token estimate.
        token_estimate: u32,
        /// Caller-provided token budget.
        token_budget: u32,
    },
    /// Integer arithmetic overflowed.
    #[error("arithmetic_overflow: {operation}")]
    ArithmeticOverflow {
        /// Operation name.
        operation: &'static str,
    },
    /// Basis-point score is outside `[0, 10000]`.
    #[error("invalid_score_component: {component}={value}")]
    InvalidScoreComponent {
        /// Component name.
        component: &'static str,
        /// Invalid value.
        value: u32,
    },
    /// Hash material could not be serialized.
    #[error("hash_material_failed: {reason}")]
    HashMaterial {
        /// Stable reason string.
        reason: String,
    },
    /// Domain subject failed validation.
    #[error("validation_failed")]
    ValidationFailed,
    /// Route must be active before packet or writeback use.
    #[error("route_not_active")]
    RouteNotActive,
    /// Context packet must be validated before writeback.
    #[error("context_packet_not_validated")]
    ContextPacketNotValidated,
    /// Credential signature material is invalid.
    #[error("invalid_signature")]
    InvalidSignature,
    /// Credential is expired.
    #[error("expired_credential")]
    ExpiredCredential,
}

/// Preverified authority and consent evidence for a domain call.
#[derive(Debug, Clone)]
pub struct DomainGateContext {
    pub action: String,
    pub authority_scope: AuthorityScope,
    pub consent_decision: ConsentDecision,
}

/// Agent memory safety scoring inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentSafetyComponents {
    pub identity_bp: u16,
    pub authority_bp: u16,
    pub consent_bp: u16,
    pub provenance_bp: u16,
    pub validation_bp: u16,
    pub recency_bp: u16,
    pub revocation_bp: u16,
    pub route_quality_bp: u16,
    pub incident_penalty_bp: u16,
}

/// Safety score decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSafetyDecision {
    Pass,
    NeedsCouncil,
    Block,
}

/// Computed agent memory safety score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentSafetyScoreResult {
    pub total_score_bp: u16,
    pub decision: AgentSafetyDecision,
}

/// Route score component inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteScoreComponents {
    pub relevance_bp: u16,
    pub validation_bp: u16,
    pub consent_authority_bp: u16,
    pub recency_bp: u16,
    pub provenance_bp: u16,
    pub risk_safety_bp: u16,
    pub token_efficiency_bp: u16,
    pub diversity_bp: u16,
    pub memory_use_count_7d: u32,
}

/// Computed route score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteScoreResult {
    pub route_score_bp: u16,
    pub overuse_penalty_bp: u16,
}

/// Benchmark quality gate input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkGateInput {
    pub quality_score_bp: u16,
    pub citation_accuracy_bp: u16,
    pub unsupported_claim_rate_bp: u16,
    pub gross_savings_micro_exo: u64,
    pub overhead_micro_exo: u64,
}

/// Benchmark quality gate output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkGateResult {
    pub gates_pass: bool,
    pub net_savings_micro_exo: u64,
    pub savings_claim_allowed: bool,
}

/// Enforce tenant and namespace match against preverified scope.
pub fn ensure_tenant_scope(
    scope: &DagDbAuthorizedScope,
    tenant_id: &str,
    namespace: &str,
) -> DomainResult<()> {
    if scope.tenant_id == tenant_id && scope.namespace == namespace {
        return Ok(());
    }
    Err(DomainError::TenantScopeMismatch {
        expected_tenant_id: scope.tenant_id.clone(),
        expected_namespace: scope.namespace.clone(),
        actual_tenant_id: tenant_id.to_owned(),
        actual_namespace: namespace.to_owned(),
    })
}

/// Enforce action authorization and consent decision.
pub fn ensure_authority_and_consent(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
) -> DomainResult<()> {
    if !scope
        .permitted_actions
        .iter()
        .any(|action| action == &gate.action)
    {
        return Err(DomainError::AuthorityDenied {
            action: gate.action.clone(),
        });
    }
    let required = required_permission_for_action(&gate.action);
    let permissions = PermissionSet::from_permissions(&gate.authority_scope.permissions);
    if !permissions.contains(&required) {
        return Err(DomainError::AuthorityDenied {
            action: gate.action.clone(),
        });
    }
    match &gate.consent_decision {
        ConsentDecision::Granted { .. } => Ok(()),
        ConsentDecision::Denied { .. } | ConsentDecision::Escalated { .. } => {
            Err(DomainError::ConsentDenied {
                action: gate.action.clone(),
            })
        }
    }
}

/// Map DAG DB action names to existing EXOCHAIN authority permissions.
#[must_use]
pub fn required_permission_for_action(action: &str) -> Permission {
    match action {
        "dagdb:intake" | "dagdb:writeback" => Permission::Write,
        "dagdb:validate" => Permission::Challenge,
        "dagdb:trust_check" | "dagdb:council_decision" => Permission::Govern,
        "dagdb:route"
        | "dagdb:context_packet"
        | "dagdb:receipt_lookup"
        | "dagdb:catalog_lookup"
        | "dagdb:route_lookup" => Permission::Read,
        _ => Permission::Read,
    }
}

/// Map risk basis points to the pinned DAG DB risk class.
pub fn risk_class_for_bp(risk_bp: u32) -> DomainResult<RiskClass> {
    let bp = clamp_to_bp(risk_bp)?;
    Ok(match bp {
        0..=999 => RiskClass::R0,
        1_000..=2_499 => RiskClass::R1,
        2_500..=4_999 => RiskClass::R2,
        5_000..=7_499 => RiskClass::R3,
        7_500..=8_999 => RiskClass::R4,
        9_000..=10_000 => RiskClass::R5,
        _ => {
            return Err(DomainError::InvalidScoreComponent {
                component: "risk_bp",
                value: risk_bp,
            });
        }
    })
}

/// Map DAG DB risk class to existing EXOCHAIN identity risk level.
#[must_use]
pub const fn risk_level_for_class(risk_class: RiskClass) -> RiskLevel {
    match risk_class {
        RiskClass::R0 => RiskLevel::Minimal,
        RiskClass::R1 => RiskLevel::Low,
        RiskClass::R2 => RiskLevel::Medium,
        RiskClass::R3 => RiskLevel::High,
        RiskClass::R4 => RiskLevel::Critical,
        RiskClass::R5 => RiskLevel::Unassessed,
    }
}

/// Return true when durable approval is required.
#[must_use]
pub const fn risk_class_requires_approval(risk_class: RiskClass) -> bool {
    matches!(risk_class, RiskClass::R3 | RiskClass::R4 | RiskClass::R5)
}

/// Enforce durable council approval for R3-R5 scope.
pub fn require_durable_approval(
    scope: &ApprovalScope,
    decision: Option<&CouncilDecision>,
    now: Timestamp,
) -> DomainResult<CouncilReviewStatus> {
    require_approval_for_risk(scope, decision, now).map_err(map_council_error)
}

/// Compute the agent memory safety score exactly as specified.
pub fn compute_agent_memory_safety_score(
    components: AgentSafetyComponents,
) -> DomainResult<AgentSafetyScoreResult> {
    validate_bp("identity_bp", components.identity_bp)?;
    validate_bp("authority_bp", components.authority_bp)?;
    validate_bp("consent_bp", components.consent_bp)?;
    validate_bp("provenance_bp", components.provenance_bp)?;
    validate_bp("validation_bp", components.validation_bp)?;
    validate_bp("recency_bp", components.recency_bp)?;
    validate_bp("revocation_bp", components.revocation_bp)?;
    validate_bp("route_quality_bp", components.route_quality_bp)?;
    validate_bp("incident_penalty_bp", components.incident_penalty_bp)?;

    let weighted_sum = checked_weight("identity_bp", components.identity_bp, 1_400)?
        .checked_add(checked_weight(
            "authority_bp",
            components.authority_bp,
            1_400,
        )?)
        .and_then(|sum| {
            sum.checked_add(checked_weight("consent_bp", components.consent_bp, 1_400).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("provenance_bp", components.provenance_bp, 1_200).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("validation_bp", components.validation_bp, 1_400).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("recency_bp", components.recency_bp, 900).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("revocation_bp", components.revocation_bp, 1_300).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(
                checked_weight("route_quality_bp", components.route_quality_bp, 1_000).ok()?,
            )
        })
        .ok_or(DomainError::ArithmeticOverflow {
            operation: "agent_safety_weighted_sum",
        })?;
    let weighted_sum_bp = weighted_sum / u32::from(MAX_BP);
    let penalty = u32::from(components.incident_penalty_bp.min(MAX_INCIDENT_PENALTY_BP));
    let total = weighted_sum_bp.saturating_sub(penalty);
    let total_score_bp = u16::try_from(total).map_err(|_| DomainError::ArithmeticOverflow {
        operation: "agent_safety_total_score",
    })?;
    let decision = if components.revocation_bp < MAX_BP
        || components.identity_bp < REQUIRED_COMPONENT_MIN_BP
        || components.authority_bp < REQUIRED_COMPONENT_MIN_BP
        || components.consent_bp < REQUIRED_COMPONENT_MIN_BP
        || total_score_bp < AGENT_SAFETY_COUNCIL_MIN_BP
    {
        AgentSafetyDecision::Block
    } else if total_score_bp < AGENT_SAFETY_MIN_PASS_BP {
        AgentSafetyDecision::NeedsCouncil
    } else {
        AgentSafetyDecision::Pass
    };
    Ok(AgentSafetyScoreResult {
        total_score_bp,
        decision,
    })
}

/// Compute the route score using deterministic integer basis-point arithmetic.
pub fn compute_route_score(components: RouteScoreComponents) -> DomainResult<RouteScoreResult> {
    validate_bp("relevance_bp", components.relevance_bp)?;
    validate_bp("validation_bp", components.validation_bp)?;
    validate_bp("consent_authority_bp", components.consent_authority_bp)?;
    validate_bp("recency_bp", components.recency_bp)?;
    validate_bp("provenance_bp", components.provenance_bp)?;
    validate_bp("risk_safety_bp", components.risk_safety_bp)?;
    validate_bp("token_efficiency_bp", components.token_efficiency_bp)?;
    validate_bp("diversity_bp", components.diversity_bp)?;

    let weighted_sum = checked_weight("relevance_bp", components.relevance_bp, 2_500)?
        .checked_add(checked_weight(
            "validation_bp",
            components.validation_bp,
            2_000,
        )?)
        .and_then(|sum| {
            sum.checked_add(
                checked_weight(
                    "consent_authority_bp",
                    components.consent_authority_bp,
                    1_500,
                )
                .ok()?,
            )
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("recency_bp", components.recency_bp, 1_000).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("provenance_bp", components.provenance_bp, 1_000).ok()?)
        })
        .and_then(|sum| {
            sum.checked_add(
                checked_weight("risk_safety_bp", components.risk_safety_bp, 1_000).ok()?,
            )
        })
        .and_then(|sum| {
            sum.checked_add(
                checked_weight("token_efficiency_bp", components.token_efficiency_bp, 700).ok()?,
            )
        })
        .and_then(|sum| {
            sum.checked_add(checked_weight("diversity_bp", components.diversity_bp, 300).ok()?)
        })
        .ok_or(DomainError::ArithmeticOverflow {
            operation: "route_weighted_sum",
        })?;
    let raw_score = weighted_sum / u32::from(MAX_BP);
    let overuse_penalty_bp = overuse_penalty_bp(components.memory_use_count_7d)?;
    let route_score = raw_score.saturating_sub(u32::from(overuse_penalty_bp));
    let route_score_bp =
        u16::try_from(route_score).map_err(|_| DomainError::ArithmeticOverflow {
            operation: "route_score",
        })?;
    Ok(RouteScoreResult {
        route_score_bp,
        overuse_penalty_bp,
    })
}

/// Compute route overuse penalty.
pub fn overuse_penalty_bp(memory_use_count_7d: u32) -> DomainResult<u16> {
    let extra_uses = memory_use_count_7d.saturating_sub(20);
    let penalty = extra_uses
        .checked_mul(50)
        .ok_or(DomainError::ArithmeticOverflow {
            operation: "overuse_penalty",
        })?
        .min(1_500);
    u16::try_from(penalty).map_err(|_| DomainError::ArithmeticOverflow {
        operation: "overuse_penalty_u16",
    })
}

/// Enforce token budget plus reserve.
pub fn ensure_token_budget(token_estimate: u32, token_budget: u32) -> DomainResult<()> {
    let required = token_estimate.checked_add(TOKEN_BUDGET_RESERVE).ok_or(
        DomainError::ArithmeticOverflow {
            operation: "token_budget_reserve",
        },
    )?;
    if required > token_budget {
        return Err(DomainError::TokenBudgetExceeded {
            token_estimate,
            token_budget,
        });
    }
    Ok(())
}

/// Return true when `created_at` has reached the 24-hour stale threshold.
pub fn route_is_stale(created_at: Timestamp, now: Timestamp) -> DomainResult<bool> {
    let stale_at = add_ms(created_at, ROUTE_STALE_AFTER_MS, "route_stale_at")?;
    Ok(stale_at <= now)
}

/// Return route stale timestamp.
pub fn route_stale_at(created_at: Timestamp) -> DomainResult<Timestamp> {
    add_ms(created_at, ROUTE_STALE_AFTER_MS, "route_stale_at")
}

/// Return true when memory age has reached the 90-day stale threshold.
pub fn memory_is_stale(created_at: Timestamp, now: Timestamp) -> DomainResult<bool> {
    let stale_at = add_ms(created_at, MEMORY_STALE_AFTER_MS, "memory_stale_at")?;
    Ok(stale_at <= now)
}

/// Score recency as 0 for stale source memory and 10000 otherwise.
pub fn recency_component_bp(created_at: Timestamp, now: Timestamp) -> DomainResult<u16> {
    if memory_is_stale(created_at, now)? {
        Ok(0)
    } else {
        Ok(MAX_BP)
    }
}

/// Evaluate benchmark quality and savings gates.
pub fn evaluate_benchmark_gates(input: BenchmarkGateInput) -> DomainResult<BenchmarkGateResult> {
    validate_bp("quality_score_bp", input.quality_score_bp)?;
    validate_bp("citation_accuracy_bp", input.citation_accuracy_bp)?;
    validate_bp("unsupported_claim_rate_bp", input.unsupported_claim_rate_bp)?;
    let net_savings_micro_exo = input
        .gross_savings_micro_exo
        .saturating_sub(input.overhead_micro_exo);
    let gates_pass = input.quality_score_bp >= BENCHMARK_QUALITY_MIN_BP
        && input.citation_accuracy_bp >= BENCHMARK_CITATION_MIN_BP
        && input.unsupported_claim_rate_bp <= BENCHMARK_UNSUPPORTED_MAX_BP;
    Ok(BenchmarkGateResult {
        gates_pass,
        net_savings_micro_exo,
        savings_claim_allowed: gates_pass && net_savings_micro_exo > 0,
    })
}

/// Hash a serializable event body using canonical EXOCHAIN CBOR.
pub fn hash_event_body<T: serde::Serialize>(value: &T) -> DomainResult<Hash256> {
    exo_core::hash::hash_structured(value).map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

/// Convert a hash-material error to the domain error shape.
pub fn hash_error(error: crate::error::DagDbError) -> DomainError {
    DomainError::HashMaterial {
        reason: error.to_string(),
    }
}

/// Return a redacted text reference for response summaries.
#[must_use]
pub fn safe_excerpt(metadata: &SafeMetadata) -> String {
    metadata.text.clone()
}

fn map_council_error(error: CouncilError) -> DomainError {
    match error {
        CouncilError::ApprovalRequired => DomainError::ApprovalRequired,
        CouncilError::ApprovalDenied => DomainError::ApprovalDenied,
        CouncilError::CouncilEscalationRequired => DomainError::CouncilEscalationRequired,
        CouncilError::ApprovalScopeMismatch => DomainError::ApprovalScopeMismatch,
        CouncilError::InvalidRequestShape(_)
        | CouncilError::Metadata(_)
        | CouncilError::Hash(_) => DomainError::ApprovalRequired,
    }
}

fn clamp_to_bp(value: u32) -> DomainResult<u16> {
    u16::try_from(value.min(u32::from(MAX_BP))).map_err(|_| DomainError::ArithmeticOverflow {
        operation: "basis_points_u16",
    })
}

fn validate_bp(component: &'static str, value: u16) -> DomainResult<()> {
    if value <= MAX_BP {
        Ok(())
    } else {
        Err(DomainError::InvalidScoreComponent {
            component,
            value: u32::from(value),
        })
    }
}

fn checked_weight(component: &'static str, value: u16, weight: u32) -> DomainResult<u32> {
    validate_bp(component, value)?;
    u32::from(value)
        .checked_mul(weight)
        .ok_or(DomainError::ArithmeticOverflow {
            operation: component,
        })
}

fn add_ms(
    timestamp: Timestamp,
    amount_ms: u64,
    operation: &'static str,
) -> DomainResult<Timestamp> {
    let physical_ms = timestamp
        .physical_ms
        .checked_add(amount_ms)
        .ok_or(DomainError::ArithmeticOverflow { operation })?;
    Ok(Timestamp::new(physical_ms, timestamp.logical))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_scope(permissions: &[Permission]) -> AuthorityScope {
        AuthorityScope {
            permissions: permissions.to_vec(),
            tools: Vec::new(),
            data_classes: Vec::new(),
            counterparties: Vec::new(),
            jurisdictions: Vec::new(),
        }
    }

    fn authorized_scope(actions: &[&str]) -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:agent".into(),
            authority_scope_hash: Hash256::from_bytes([1; 32]),
            consent_scope_hash: Hash256::from_bytes([2; 32]),
            permitted_actions: actions.iter().map(|action| (*action).to_owned()).collect(),
            expires_at: Timestamp::new(10_000, 0),
        }
    }

    #[test]
    fn integer_scoring_vectors() {
        assert_eq!(risk_class_for_bp(0), Ok(RiskClass::R0));
        assert_eq!(risk_class_for_bp(999), Ok(RiskClass::R0));
        assert_eq!(risk_class_for_bp(1_000), Ok(RiskClass::R1));
        assert_eq!(risk_class_for_bp(2_500), Ok(RiskClass::R2));
        assert_eq!(risk_class_for_bp(5_000), Ok(RiskClass::R3));
        assert_eq!(risk_class_for_bp(7_500), Ok(RiskClass::R4));
        assert_eq!(risk_class_for_bp(9_000), Ok(RiskClass::R5));
        assert_eq!(risk_level_for_class(RiskClass::R4), RiskLevel::Critical);
        assert_eq!(risk_class_for_bp(10_001), Ok(RiskClass::R5));

        let pass = compute_agent_memory_safety_score(AgentSafetyComponents {
            identity_bp: 8_000,
            authority_bp: 8_000,
            consent_bp: 8_000,
            provenance_bp: 8_000,
            validation_bp: 8_000,
            recency_bp: 8_000,
            revocation_bp: 10_000,
            route_quality_bp: 8_000,
            incident_penalty_bp: 0,
        })
        .expect("pass score computes");
        assert_eq!(pass.total_score_bp, 8_260);
        assert_eq!(pass.decision, AgentSafetyDecision::Pass);

        let needs_council = compute_agent_memory_safety_score(AgentSafetyComponents {
            identity_bp: 7_000,
            authority_bp: 7_000,
            consent_bp: 7_000,
            provenance_bp: 7_000,
            validation_bp: 7_000,
            recency_bp: 7_000,
            revocation_bp: 10_000,
            route_quality_bp: 7_000,
            incident_penalty_bp: 0,
        })
        .expect("council score computes");
        assert_eq!(needs_council.total_score_bp, 7_390);
        assert_eq!(needs_council.decision, AgentSafetyDecision::NeedsCouncil);

        let blocked = compute_agent_memory_safety_score(AgentSafetyComponents {
            revocation_bp: 9_999,
            ..AgentSafetyComponents {
                identity_bp: 8_000,
                authority_bp: 8_000,
                consent_bp: 8_000,
                provenance_bp: 8_000,
                validation_bp: 8_000,
                recency_bp: 8_000,
                revocation_bp: 10_000,
                route_quality_bp: 8_000,
                incident_penalty_bp: 0,
            }
        })
        .expect("revocation block computes");
        assert_eq!(blocked.decision, AgentSafetyDecision::Block);

        let route = compute_route_score(RouteScoreComponents {
            relevance_bp: 9_000,
            validation_bp: 9_000,
            consent_authority_bp: 9_000,
            recency_bp: 8_000,
            provenance_bp: 7_000,
            risk_safety_bp: 8_000,
            token_efficiency_bp: 10_000,
            diversity_bp: 5_000,
            memory_use_count_7d: 25,
        })
        .expect("route score computes");
        assert_eq!(route.overuse_penalty_bp, 250);
        assert_eq!(route.route_score_bp, 8_300);

        assert_eq!(
            ensure_token_budget(u32::MAX, 1),
            Err(DomainError::ArithmeticOverflow {
                operation: "token_budget_reserve",
            })
        );
    }

    #[test]
    fn domain_gate_consumes_authority_scope_and_consent_decision() {
        let scope = authorized_scope(&["dagdb:intake"]);
        let gate = DomainGateContext {
            action: "dagdb:intake".into(),
            authority_scope: authority_scope(&[Permission::Write]),
            consent_decision: ConsentDecision::Granted { expires: None },
        };
        assert_eq!(ensure_tenant_scope(&scope, "tenant-a", "primary"), Ok(()));
        assert_eq!(ensure_authority_and_consent(&scope, &gate), Ok(()));

        let denied_authority = DomainGateContext {
            authority_scope: authority_scope(&[Permission::Read]),
            ..gate.clone()
        };
        assert!(matches!(
            ensure_authority_and_consent(&scope, &denied_authority),
            Err(DomainError::AuthorityDenied { .. })
        ));

        let denied_consent = DomainGateContext {
            consent_decision: ConsentDecision::Denied {
                reason: "missing bailment".into(),
            },
            ..gate
        };
        assert!(matches!(
            ensure_authority_and_consent(&scope, &denied_consent),
            Err(DomainError::ConsentDenied { .. })
        ));
    }

    #[test]
    fn benchmark_savings_gates_block_lower_quality_claims() {
        let pass = evaluate_benchmark_gates(BenchmarkGateInput {
            quality_score_bp: 8_500,
            citation_accuracy_bp: 9_500,
            unsupported_claim_rate_bp: 500,
            gross_savings_micro_exo: 1_000,
            overhead_micro_exo: 250,
        })
        .expect("benchmark pass computes");
        assert_eq!(pass.net_savings_micro_exo, 750);
        assert!(pass.savings_claim_allowed);

        let low_quality = evaluate_benchmark_gates(BenchmarkGateInput {
            quality_score_bp: 8_499,
            citation_accuracy_bp: 9_500,
            unsupported_claim_rate_bp: 500,
            gross_savings_micro_exo: 1_000,
            overhead_micro_exo: 250,
        })
        .expect("benchmark fail computes");
        assert!(!low_quality.gates_pass);
        assert!(!low_quality.savings_claim_allowed);

        let no_savings = evaluate_benchmark_gates(BenchmarkGateInput {
            quality_score_bp: 8_500,
            citation_accuracy_bp: 9_500,
            unsupported_claim_rate_bp: 500,
            gross_savings_micro_exo: 100,
            overhead_micro_exo: 250,
        })
        .expect("benchmark zero savings computes");
        assert_eq!(no_savings.net_savings_micro_exo, 0);
        assert!(!no_savings.savings_claim_allowed);
    }

    #[test]
    fn integer_scoring_edge_vectors_cover_fail_closed_branches() {
        assert!(matches!(
            ensure_tenant_scope(&authorized_scope(&["dagdb:route"]), "tenant-b", "primary"),
            Err(DomainError::TenantScopeMismatch { .. })
        ));
        assert!(matches!(
            ensure_authority_and_consent(
                &authorized_scope(&[]),
                &DomainGateContext {
                    action: "dagdb:route".into(),
                    authority_scope: authority_scope(&[Permission::Read]),
                    consent_decision: ConsentDecision::Granted { expires: None },
                },
            ),
            Err(DomainError::AuthorityDenied { .. })
        ));
        assert!(matches!(
            ensure_authority_and_consent(
                &authorized_scope(&["dagdb:route"]),
                &DomainGateContext {
                    action: "dagdb:route".into(),
                    authority_scope: authority_scope(&[Permission::Read]),
                    consent_decision: ConsentDecision::Escalated {
                        to: exo_core::Did::new("did:exo:council").expect("fixture DID is valid"),
                    },
                },
            ),
            Err(DomainError::ConsentDenied { .. })
        ));
        assert_eq!(required_permission_for_action("unknown"), Permission::Read);

        let invalid_component = compute_route_score(RouteScoreComponents {
            relevance_bp: 10_001,
            validation_bp: 0,
            consent_authority_bp: 0,
            recency_bp: 0,
            provenance_bp: 0,
            risk_safety_bp: 0,
            token_efficiency_bp: 0,
            diversity_bp: 0,
            memory_use_count_7d: 0,
        });
        assert_eq!(
            invalid_component,
            Err(DomainError::InvalidScoreComponent {
                component: "relevance_bp",
                value: 10_001,
            })
        );

        let low_total = compute_agent_memory_safety_score(AgentSafetyComponents {
            identity_bp: 7_000,
            authority_bp: 7_000,
            consent_bp: 7_000,
            provenance_bp: 1_000,
            validation_bp: 1_000,
            recency_bp: 1_000,
            revocation_bp: 10_000,
            route_quality_bp: 1_000,
            incident_penalty_bp: 0,
        })
        .expect("low score computes");
        assert_eq!(low_total.decision, AgentSafetyDecision::Block);

        let low_required_component = compute_agent_memory_safety_score(AgentSafetyComponents {
            identity_bp: 6_999,
            authority_bp: 8_000,
            consent_bp: 8_000,
            provenance_bp: 8_000,
            validation_bp: 8_000,
            recency_bp: 8_000,
            revocation_bp: 10_000,
            route_quality_bp: 8_000,
            incident_penalty_bp: 0,
        })
        .expect("low component score computes");
        assert_eq!(low_required_component.decision, AgentSafetyDecision::Block);

        assert_eq!(overuse_penalty_bp(20), Ok(0));
        assert_eq!(overuse_penalty_bp(21), Ok(50));
        assert_eq!(overuse_penalty_bp(51), Ok(1_500));
        assert_eq!(
            overuse_penalty_bp(u32::MAX),
            Err(DomainError::ArithmeticOverflow {
                operation: "overuse_penalty",
            })
        );
        assert_eq!(ensure_token_budget(500, 1_000), Ok(()));
        assert_eq!(
            ensure_token_budget(800, 1_000),
            Err(DomainError::TokenBudgetExceeded {
                token_estimate: 800,
                token_budget: 1_000,
            })
        );

        assert_eq!(
            route_is_stale(Timestamp::new(1_000, 0), Timestamp::new(2_000, 0)),
            Ok(false)
        );
        assert_eq!(
            route_is_stale(Timestamp::new(1_000, 0), Timestamp::new(86_401_000, 0)),
            Ok(true)
        );
        assert!(matches!(
            route_stale_at(Timestamp::new(u64::MAX, 0)),
            Err(DomainError::ArithmeticOverflow { .. })
        ));
        assert_eq!(
            recency_component_bp(Timestamp::new(1_000, 0), Timestamp::new(2_000, 0)),
            Ok(MAX_BP)
        );
        assert_eq!(
            recency_component_bp(Timestamp::new(1_000, 0), Timestamp::new(7_776_001_000, 0)),
            Ok(0)
        );

        let high_unsupported = evaluate_benchmark_gates(BenchmarkGateInput {
            quality_score_bp: 10_000,
            citation_accuracy_bp: 10_000,
            unsupported_claim_rate_bp: 501,
            gross_savings_micro_exo: 1_000,
            overhead_micro_exo: 0,
        })
        .expect("benchmark computes");
        assert!(!high_unsupported.gates_pass);
        assert_eq!(
            safe_excerpt(&SafeMetadata {
                decision: exo_dag_db_api::SafeMetadataDecision::Allow,
                text: "safe".into(),
                redaction_codes: Vec::new(),
                original_hash: Hash256::from_bytes([9; 32]).to_string(),
                truncated: false,
                byte_len: 4,
            }),
            "safe"
        );
        assert!(hash_event_body(&("event", 1u16)).is_ok());
    }
}
