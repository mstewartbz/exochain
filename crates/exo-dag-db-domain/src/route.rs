//! Pure route domain service for committed DAG DB memory.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    CouncilReviewStatus, DagFinalityStatus, MemoryStatus, RouteStatus, SubjectKind,
    ValidationStatus,
};

use crate::{
    council::ApprovalScope,
    model::{CouncilDecision, DagDbAuthorizedScope, ReceiptMemoryObject, RouteMemoryReceipt},
    scoring::{
        DomainError, DomainGateContext, DomainResult, RouteScoreComponents, compute_route_score,
        ensure_authority_and_consent, ensure_tenant_scope, ensure_token_budget, hash_error,
        memory_is_stale, recency_component_bp, require_durable_approval,
        risk_class_requires_approval, route_stale_at,
    },
};

/// Route request material after gateway scope verification.
#[derive(Debug, Clone)]
pub struct RouteDomainInput {
    pub tenant_id: String,
    pub namespace: String,
    pub requesting_agent_did: String,
    pub task_signature_hash: Hash256,
    pub approved_scope_hash: Hash256,
    pub token_budget: u32,
    pub credential_id: Option<Hash256>,
    pub validation_report_id: Option<Hash256>,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
}

/// Memory plus deterministic scoring features.
#[derive(Debug, Clone)]
pub struct RouteMemoryCandidate {
    pub memory: ReceiptMemoryObject,
    pub relevance_bp: u16,
    pub provenance_bp: u16,
    pub token_estimate: u32,
    pub memory_use_count_7d: u32,
    pub contradictory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedCandidate {
    memory_id: Hash256,
    risk_bp: u16,
    token_estimate: u32,
    route_score_bp: u16,
}

/// Create a route over eligible committed memory.
pub fn create_route(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
    input: RouteDomainInput,
    candidates: &[RouteMemoryCandidate],
    council_decision: Option<&CouncilDecision>,
    now: Timestamp,
) -> DomainResult<RouteMemoryReceipt> {
    ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
    ensure_authority_and_consent(scope, gate)?;

    let mut ranked = Vec::new();
    let mut rejected_memory_ids = Vec::new();
    let mut first_rejection = None;
    for candidate in candidates {
        match rank_candidate(&input, candidate, council_decision, now) {
            Ok(candidate) => ranked.push(candidate),
            Err(error) => {
                if first_rejection.is_none() {
                    first_rejection = Some(error);
                }
                rejected_memory_ids.push(candidate.memory.memory_id);
            }
        }
    }
    if ranked.is_empty() {
        return Err(first_rejection.unwrap_or(DomainError::NoEligibleMemory));
    }
    ranked.sort_by(compare_ranked_candidates);

    let mut selected_memory_ids = Vec::new();
    let mut selected_token_estimate = 0u32;
    let mut max_risk_bp = 0u16;
    let mut route_score_bp = 0u16;
    for candidate in &ranked {
        let next_estimate = selected_token_estimate
            .checked_add(candidate.token_estimate)
            .ok_or(DomainError::ArithmeticOverflow {
                operation: "route_token_estimate",
            })?;
        if ensure_token_budget(next_estimate, input.token_budget).is_err() {
            rejected_memory_ids.push(candidate.memory_id);
            continue;
        }
        selected_token_estimate = next_estimate;
        max_risk_bp = max_risk_bp.max(candidate.risk_bp);
        route_score_bp = route_score_bp.max(candidate.route_score_bp);
        selected_memory_ids.push(candidate.memory_id);
    }
    if selected_memory_ids.is_empty() {
        return Err(DomainError::TokenBudgetExceeded {
            token_estimate: selected_token_estimate,
            token_budget: input.token_budget,
        });
    }

    let route_id = crate::hash::RouteIdMaterial::new(
        input.tenant_id.clone(),
        input.namespace.clone(),
        input.requesting_agent_did.clone(),
        input.task_signature_hash,
        input.approved_scope_hash,
        selected_memory_ids.clone(),
        input.token_budget,
    )
    .hash()
    .map_err(hash_error)?;

    Ok(RouteMemoryReceipt {
        route_id,
        tenant_id: input.tenant_id,
        namespace: input.namespace,
        requesting_agent_did: input.requesting_agent_did,
        task_signature_hash: input.task_signature_hash,
        approved_scope_hash: input.approved_scope_hash,
        candidate_memory_ids: candidates
            .iter()
            .map(|candidate| candidate.memory.memory_id)
            .collect(),
        selected_memory_ids,
        rejected_memory_ids,
        route_score_bp,
        token_budget: input.token_budget,
        token_estimate: selected_token_estimate,
        risk_bp: max_risk_bp,
        status: RouteStatus::Active,
        validation_status: ValidationStatus::Passed,
        council_status: council_decision.map_or(CouncilReviewStatus::NotRequired, |_| {
            CouncilReviewStatus::Approved
        }),
        dag_finality_status: DagFinalityStatus::Pending,
        stale_at: route_stale_at(input.created_at)?,
        latest_receipt_hash: input.latest_receipt_hash,
        created_at: input.created_at,
        credential_id: input.credential_id,
        validation_report_id: input.validation_report_id,
        council_decision_id: council_decision.map(|decision| decision.decision_id),
    })
}

/// Validate memory eligibility for route and packet use.
pub fn ensure_memory_eligible(
    expected_tenant_id: &str,
    expected_namespace: &str,
    memory: &ReceiptMemoryObject,
) -> DomainResult<()> {
    if memory.tenant_id != expected_tenant_id || memory.namespace != expected_namespace {
        return Err(DomainError::TenantScopeMismatch {
            expected_tenant_id: expected_tenant_id.to_owned(),
            expected_namespace: expected_namespace.to_owned(),
            actual_tenant_id: memory.tenant_id.clone(),
            actual_namespace: memory.namespace.clone(),
        });
    }
    if memory.revoked_at.is_some() || memory.status == MemoryStatus::Revoked {
        return Err(DomainError::RevokedMemory {
            memory_id: memory.memory_id,
        });
    }
    if memory.superseded_by_memory_id.is_some() || memory.status == MemoryStatus::Superseded {
        return Err(DomainError::SupersededMemory {
            memory_id: memory.memory_id,
            superseded_by_memory_id: memory.superseded_by_memory_id,
        });
    }
    if memory.status != MemoryStatus::Routable {
        return Err(DomainError::NoEligibleMemory);
    }
    if !matches!(
        memory.validation_status,
        ValidationStatus::Passed | ValidationStatus::NotRequired
    ) {
        return Err(DomainError::ValidationFailed);
    }
    if memory.dag_finality_status != DagFinalityStatus::Committed {
        return Err(DomainError::NonCommittedFinality {
            subject_id: memory.memory_id,
        });
    }
    Ok(())
}

fn rank_candidate(
    input: &RouteDomainInput,
    candidate: &RouteMemoryCandidate,
    council_decision: Option<&CouncilDecision>,
    now: Timestamp,
) -> DomainResult<RankedCandidate> {
    ensure_memory_eligible(&input.tenant_id, &input.namespace, &candidate.memory)?;
    if candidate.contradictory {
        return Err(DomainError::ContradictoryValidation {
            subject_id: candidate.memory.memory_id,
        });
    }
    if memory_is_stale(candidate.memory.created_at, now)? {
        return Err(DomainError::NoEligibleMemory);
    }
    if risk_class_requires_approval(candidate.memory.risk_class) {
        let approval_scope = ApprovalScope {
            tenant_id: input.tenant_id.clone(),
            namespace: input.namespace.clone(),
            subject_kind: SubjectKind::Memory,
            subject_id: candidate.memory.memory_id,
            requested_action: "dagdb:route".into(),
            approved_scope_hash: input.approved_scope_hash,
            risk_class: candidate.memory.risk_class,
            council_decision_id: council_decision.map(|decision| decision.decision_id),
        };
        require_durable_approval(&approval_scope, council_decision, now)?;
    }
    ensure_token_budget(candidate.token_estimate, input.token_budget)?;
    let token_efficiency_bp = token_efficiency_bp(candidate.token_estimate, input.token_budget)?;
    let route_score = compute_route_score(RouteScoreComponents {
        relevance_bp: candidate.relevance_bp,
        validation_bp: 10_000,
        consent_authority_bp: 10_000,
        recency_bp: recency_component_bp(candidate.memory.created_at, now)?,
        provenance_bp: candidate.provenance_bp,
        risk_safety_bp: 10_000u16.saturating_sub(candidate.memory.risk_bp),
        token_efficiency_bp,
        diversity_bp: 10_000,
        memory_use_count_7d: candidate.memory_use_count_7d,
    })?;
    Ok(RankedCandidate {
        memory_id: candidate.memory.memory_id,
        risk_bp: candidate.memory.risk_bp,
        token_estimate: candidate.token_estimate,
        route_score_bp: route_score.route_score_bp,
    })
}

fn token_efficiency_bp(token_estimate: u32, token_budget: u32) -> DomainResult<u16> {
    ensure_token_budget(token_estimate, token_budget)?;
    let required = token_estimate
        .checked_add(256)
        .ok_or(DomainError::ArithmeticOverflow {
            operation: "token_efficiency_required",
        })?;
    let remaining = token_budget.saturating_sub(required);
    let score = remaining
        .checked_mul(10_000)
        .ok_or(DomainError::ArithmeticOverflow {
            operation: "token_efficiency_score",
        })?
        / token_budget.max(1);
    u16::try_from(score.min(10_000)).map_err(|_| DomainError::ArithmeticOverflow {
        operation: "token_efficiency_u16",
    })
}

fn compare_ranked_candidates(
    left: &RankedCandidate,
    right: &RankedCandidate,
) -> core::cmp::Ordering {
    right
        .route_score_bp
        .cmp(&left.route_score_bp)
        .then(left.risk_bp.cmp(&right.risk_bp))
        .then(left.token_estimate.cmp(&right.token_estimate))
        .then(left.memory_id.cmp(&right.memory_id))
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;
    use exo_dag_db_api::{
        ConsentPurpose, CouncilDecisionStatus, DecisionSource, MemoryNodeType, RiskClass,
        SafeMetadata, SafeMetadataDecision, SourceType,
    };

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn safe(text: &str) -> SafeMetadata {
        SafeMetadata {
            decision: SafeMetadataDecision::Allow,
            text: text.into(),
            redaction_codes: Vec::new(),
            original_hash: h(0xfe).to_string(),
            truncated: false,
            byte_len: u32::try_from(text.len()).expect("fixture fits"),
        }
    }

    fn scope(actions: &[&str]) -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:agent".into(),
            authority_scope_hash: h(0x90),
            consent_scope_hash: h(0x91),
            permitted_actions: actions.iter().map(|action| (*action).to_owned()).collect(),
            expires_at: ts(20_000),
        }
    }

    fn gate() -> DomainGateContext {
        DomainGateContext {
            action: "dagdb:route".into(),
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Read],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn memory(byte: u8, risk_bp: u16) -> ReceiptMemoryObject {
        ReceiptMemoryObject {
            memory_id: h(byte),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            node_type: MemoryNodeType::Source,
            source_type: SourceType::PublicWeb,
            source_hash: h(byte.wrapping_add(1)),
            payload_hash: h(byte.wrapping_add(2)),
            owner_did: "did:exo:owner".into(),
            controller_did: "did:exo:controller".into(),
            submitted_by_did: "did:exo:submitter".into(),
            consent_purpose: ConsentPurpose::Retrieval,
            title: safe("title"),
            summary: safe("summary"),
            keywords: Vec::new(),
            risk_class: crate::scoring::risk_class_for_bp(u32::from(risk_bp)).expect("risk class"),
            risk_bp,
            status: MemoryStatus::Routable,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            parent_memory_ids: Vec::new(),
            latest_receipt_hash: h(byte.wrapping_add(3)),
            created_at: ts(1_000),
            updated_at: ts(1_000),
            payload_uri_hash: None,
            access_policy_hash: None,
            declared_rights_hash: None,
            revoked_at: None,
            superseded_by_memory_id: None,
        }
    }

    fn input() -> RouteDomainInput {
        RouteDomainInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            requesting_agent_did: "did:exo:agent".into(),
            task_signature_hash: h(0x70),
            approved_scope_hash: h(0x90),
            token_budget: 2_000,
            credential_id: None,
            validation_report_id: None,
            latest_receipt_hash: h(0x71),
            created_at: ts(10_000),
        }
    }

    fn decision(subject_id: Hash256, risk_class: RiskClass) -> CouncilDecision {
        CouncilDecision {
            decision_id: h(0xd0),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            subject_kind: SubjectKind::Memory,
            subject_id,
            requested_action: "dagdb:route".into(),
            approved_scope_hash: h(0x90),
            risk_class,
            approver_did: "did:exo:approver".into(),
            decision_source: DecisionSource::Human,
            decision_status: CouncilDecisionStatus::Approved,
            reason_code: "approved".into(),
            created_at: ts(1_000),
            expires_at: ts(20_000),
            receipt_hash: h(0xd1),
            validation_report_id: None,
            route_id: None,
            context_packet_id: None,
            notes: None,
        }
    }

    fn candidate(memory: ReceiptMemoryObject, token_estimate: u32) -> RouteMemoryCandidate {
        RouteMemoryCandidate {
            memory,
            relevance_bp: 9_000,
            provenance_bp: 9_000,
            token_estimate,
            memory_use_count_7d: 0,
            contradictory: false,
        }
    }

    #[test]
    fn route_selects_only_committed_routable_memory() {
        let selected = RouteMemoryCandidate {
            memory: memory(0x10, 1_000),
            relevance_bp: 9_500,
            provenance_bp: 9_000,
            token_estimate: 500,
            memory_use_count_7d: 1,
            contradictory: false,
        };
        let mut pending = memory(0x20, 1_000);
        pending.dag_finality_status = DagFinalityStatus::Pending;
        let route = create_route(
            &scope(&["dagdb:route"]),
            &gate(),
            input(),
            &[
                selected,
                RouteMemoryCandidate {
                    memory: pending,
                    relevance_bp: 10_000,
                    provenance_bp: 10_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
            ],
            None,
            ts(10_000),
        )
        .expect("route succeeds");
        assert_eq!(route.selected_memory_ids, vec![h(0x10)]);
        assert_eq!(route.rejected_memory_ids, vec![h(0x20)]);
        assert_eq!(route.status, RouteStatus::Active);
    }

    #[test]
    fn route_filters_revoked_superseded_stale_and_contradictory_memory() {
        let mut revoked = memory(0x30, 1_000);
        revoked.status = MemoryStatus::Revoked;
        let mut superseded = memory(0x40, 1_000);
        superseded.superseded_by_memory_id = Some(h(0x41));
        let stale = ReceiptMemoryObject {
            created_at: ts(1),
            ..memory(0x50, 1_000)
        };
        let route = create_route(
            &scope(&["dagdb:route"]),
            &gate(),
            input(),
            &[
                RouteMemoryCandidate {
                    memory: revoked,
                    relevance_bp: 9_000,
                    provenance_bp: 9_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
                RouteMemoryCandidate {
                    memory: superseded,
                    relevance_bp: 9_000,
                    provenance_bp: 9_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
                RouteMemoryCandidate {
                    memory: stale,
                    relevance_bp: 9_000,
                    provenance_bp: 9_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
                RouteMemoryCandidate {
                    memory: memory(0x60, 1_000),
                    relevance_bp: 9_000,
                    provenance_bp: 9_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: true,
                },
                RouteMemoryCandidate {
                    memory: memory(0x70, 1_000),
                    relevance_bp: 8_000,
                    provenance_bp: 8_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
            ],
            None,
            ts(7_776_000_002),
        )
        .expect("one non-stale candidate remains");
        assert_eq!(route.selected_memory_ids, vec![h(0x70)]);
        assert_eq!(
            route.rejected_memory_ids,
            vec![h(0x30), h(0x40), h(0x50), h(0x60)]
        );
    }

    #[test]
    fn route_fails_for_missing_approval_and_token_budget() {
        let high_risk = RouteMemoryCandidate {
            memory: memory(0x70, 5_000),
            relevance_bp: 9_000,
            provenance_bp: 9_000,
            token_estimate: 300,
            memory_use_count_7d: 0,
            contradictory: false,
        };
        assert_eq!(
            create_route(
                &scope(&["dagdb:route"]),
                &gate(),
                input(),
                &[high_risk],
                None,
                ts(10_000),
            ),
            Err(DomainError::ApprovalRequired)
        );

        let over_budget = RouteMemoryCandidate {
            memory: memory(0x80, 1_000),
            relevance_bp: 9_000,
            provenance_bp: 9_000,
            token_estimate: 2_000,
            memory_use_count_7d: 0,
            contradictory: false,
        };
        assert_eq!(
            create_route(
                &scope(&["dagdb:route"]),
                &gate(),
                input(),
                &[over_budget],
                None,
                ts(10_000),
            ),
            Err(DomainError::TokenBudgetExceeded {
                token_estimate: 2_000,
                token_budget: 2_000,
            })
        );
    }

    #[test]
    fn route_accepts_matching_high_risk_approval_and_continues_past_over_budget_candidate() {
        let high_risk_memory = memory(0x70, 5_000);
        let approval = decision(high_risk_memory.memory_id, RiskClass::R3);
        let route = create_route(
            &scope(&["dagdb:route"]),
            &gate(),
            input(),
            &[
                RouteMemoryCandidate {
                    memory: high_risk_memory,
                    relevance_bp: 9_000,
                    provenance_bp: 9_000,
                    token_estimate: 300,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
                RouteMemoryCandidate {
                    memory: memory(0x90, 1_000),
                    relevance_bp: 10_000,
                    provenance_bp: 10_000,
                    token_estimate: 2_000,
                    memory_use_count_7d: 0,
                    contradictory: false,
                },
            ],
            Some(&approval),
            ts(10_000),
        )
        .expect("approved high risk routes");
        assert_eq!(route.selected_memory_ids, vec![h(0x70)]);
        assert_eq!(route.rejected_memory_ids, vec![h(0x90)]);
        assert_eq!(route.council_status, CouncilReviewStatus::Approved);
    }

    #[test]
    fn route_memory_eligibility_rejects_each_state_gate() {
        assert_eq!(
            create_route(
                &scope(&["dagdb:route"]),
                &gate(),
                input(),
                &[],
                None,
                ts(10_000),
            ),
            Err(DomainError::NoEligibleMemory)
        );

        let mut other_tenant = memory(0xa0, 1_000);
        other_tenant.tenant_id = "tenant-b".into();
        assert!(matches!(
            ensure_memory_eligible("tenant-a", "primary", &other_tenant),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut other_namespace = memory(0xa4, 1_000);
        other_namespace.namespace = "other".into();
        assert!(matches!(
            ensure_memory_eligible("tenant-a", "primary", &other_namespace),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut revoked_at_only = memory(0xa5, 1_000);
        revoked_at_only.revoked_at = Some(ts(2_000));
        assert_eq!(
            ensure_memory_eligible("tenant-a", "primary", &revoked_at_only),
            Err(DomainError::RevokedMemory { memory_id: h(0xa5) })
        );

        let mut superseded_status = memory(0xa6, 1_000);
        superseded_status.status = MemoryStatus::Superseded;
        assert_eq!(
            ensure_memory_eligible("tenant-a", "primary", &superseded_status),
            Err(DomainError::SupersededMemory {
                memory_id: h(0xa6),
                superseded_by_memory_id: None,
            })
        );

        let mut not_required_validation = memory(0xa7, 1_000);
        not_required_validation.validation_status = ValidationStatus::NotRequired;
        assert!(ensure_memory_eligible("tenant-a", "primary", &not_required_validation).is_ok());

        let mut approved_not_routable = memory(0xa1, 1_000);
        approved_not_routable.status = MemoryStatus::Approved;
        assert_eq!(
            ensure_memory_eligible("tenant-a", "primary", &approved_not_routable),
            Err(DomainError::NoEligibleMemory)
        );

        let mut failed_validation = memory(0xa2, 1_000);
        failed_validation.validation_status = ValidationStatus::Failed;
        assert_eq!(
            ensure_memory_eligible("tenant-a", "primary", &failed_validation),
            Err(DomainError::ValidationFailed)
        );

        let mut pending_finality = memory(0xa3, 1_000);
        pending_finality.dag_finality_status = DagFinalityStatus::Failed;
        assert_eq!(
            ensure_memory_eligible("tenant-a", "primary", &pending_finality),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0xa3),
            })
        );
    }

    #[test]
    fn route_ranker_failure_branches_are_deterministic() {
        assert_eq!(
            create_route(
                &scope(&["dagdb:route"]),
                &gate(),
                input(),
                &[candidate(
                    ReceiptMemoryObject {
                        status: MemoryStatus::Revoked,
                        ..memory(0xb0, 1_000)
                    },
                    300,
                )],
                None,
                ts(10_000),
            ),
            Err(DomainError::RevokedMemory { memory_id: h(0xb0) })
        );

        let route = create_route(
            &scope(&["dagdb:route"]),
            &gate(),
            input(),
            &[
                candidate(memory(0xb1, 1_000), 1_600),
                candidate(memory(0xb2, 1_000), 300),
            ],
            None,
            ts(10_000),
        )
        .expect("second candidate still fits after first is selected");
        assert_eq!(route.selected_memory_ids, vec![h(0xb2)]);
        assert_eq!(route.rejected_memory_ids, vec![h(0xb1)]);

        let mut bad_gate = gate();
        bad_gate.consent_decision = ConsentDecision::Denied {
            reason: "missing purpose".into(),
        };
        assert!(matches!(
            create_route(
                &scope(&["dagdb:route"]),
                &bad_gate,
                input(),
                &[candidate(memory(0xb3, 1_000), 300)],
                None,
                ts(10_000),
            ),
            Err(DomainError::ConsentDenied { .. })
        ));
    }

    #[test]
    fn route_tenant_and_authority_gates_fail_closed() {
        let mut bad_input = input();
        bad_input.namespace = "other".into();
        assert!(matches!(
            create_route(
                &scope(&["dagdb:route"]),
                &gate(),
                bad_input,
                &[],
                None,
                ts(10_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        assert!(matches!(
            create_route(&scope(&[]), &gate(), input(), &[], None, ts(10_000),),
            Err(DomainError::AuthorityDenied { .. })
        ));
    }
}
