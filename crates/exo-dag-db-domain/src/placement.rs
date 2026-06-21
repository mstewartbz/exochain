//! System-side memory placement controller and graph organizer.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    CanonicalizationDecisionKind, GraphEdgeRef, MemoryCandidate, MemoryGraphStyle, PlacementResult,
    RiskClass, SimilarityResult, ValidationStatus,
};

use crate::{
    canonicalization::{CanonicalizationRequest, decide_canonicalization},
    graph::{PlacementTraceStep, required_placement_steps, validate_placement_order},
    model::{DagDbAuthorizedScope, OutputObserver},
    scoring::{
        DomainError, DomainGateContext, DomainResult, ensure_authority_and_consent,
        ensure_tenant_scope,
    },
    similarity::{SimilarityInput, compare_all_similarity},
};

/// Existing graph memory summary used for duplicate and similarity checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementExistingMemory {
    pub memory_id: Hash256,
    pub payload_hash: Hash256,
    pub summary: String,
}

/// Placement input after gateway/domain scope verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPlacementInput {
    pub tenant_id: String,
    pub namespace: String,
    pub input_memory_id: Hash256,
    pub payload_hash: Hash256,
    pub summary: String,
    pub risk_class: RiskClass,
    pub validator_status: ValidationStatus,
    pub existing_memory: Vec<PlacementExistingMemory>,
    pub requested_decision: Option<CanonicalizationDecisionKind>,
    pub receipt_intent: String,
    pub now: Timestamp,
}

/// System-level placement controller. Task agents never call graph allocation directly.
pub struct MemoryPlacementController;

/// Deterministic graph organization worker.
pub struct GraphOrganizer;

impl MemoryPlacementController {
    /// Run the system-side placement flow for a compact candidate.
    pub fn place_memory_candidate(
        scope: &DagDbAuthorizedScope,
        gate: &DomainGateContext,
        candidate: &MemoryCandidate,
        input: MemoryPlacementInput,
    ) -> DomainResult<PlacementResult> {
        OutputObserver::validate_compact_candidate(candidate)
            .map_err(|_| DomainError::ValidationFailed)?;
        ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
        ensure_authority_and_consent(scope, gate)?;
        GraphOrganizer::organize(input)
    }

    /// Validate the deterministic placement order.
    pub fn validate_placement_order(trace: &[PlacementTraceStep]) -> DomainResult<()> {
        validate_placement_order(trace).map_err(|error| DomainError::HashMaterial {
            reason: error.to_string(),
        })
    }
}

impl GraphOrganizer {
    /// Perform duplicate detection, similarity, and canonicalization in the exact order.
    pub fn organize(input: MemoryPlacementInput) -> DomainResult<PlacementResult> {
        let trace = required_placement_steps();
        MemoryPlacementController::validate_placement_order(&trace)?;

        let similarity_results = similarity_results(&input);
        let canonicalization_decision = decide_canonicalization(CanonicalizationRequest {
            input_memory_id: input.input_memory_id,
            risk_class: input.risk_class,
            validator_status: input.validator_status,
            similarity_results: similarity_results.clone(),
            requested_decision: input.requested_decision,
        })?;
        let decision_kind = canonicalization_decision.decision_kind;

        let edges_to_create = canonicalization_decision.required_edges_to_create.clone();
        let graph_views_to_refresh = graph_views_to_refresh(&edges_to_create);
        Ok(PlacementResult {
            input_memory_id: input.input_memory_id.to_string(),
            canonicalization_decision,
            similarity_results,
            proposed_canonical_node: proposed_canonical_node(input.input_memory_id, decision_kind),
            edges_to_create,
            catalog_updates: vec!["semantic_catalog_graph".into()],
            graph_views_to_refresh,
            route_invalidations: Vec::new(),
            validator_report: "placement_validated".into(),
            receipt_id: None,
            receipt_intent: Some(input.receipt_intent),
        })
    }
}

fn similarity_results(input: &MemoryPlacementInput) -> Vec<SimilarityResult> {
    let comparisons: Vec<SimilarityInput> = input
        .existing_memory
        .iter()
        .map(|existing| SimilarityInput {
            candidate_memory_id: input.input_memory_id,
            candidate_payload_hash: input.payload_hash,
            candidate_summary: input.summary.clone(),
            matched_memory_id: existing.memory_id,
            matched_payload_hash: existing.payload_hash,
            matched_summary: existing.summary.clone(),
        })
        .collect();
    compare_all_similarity(&comparisons)
}

fn graph_views_to_refresh(edges_to_create: &[GraphEdgeRef]) -> Vec<MemoryGraphStyle> {
    let mut views = vec![
        MemoryGraphStyle::SemanticCatalogGraph,
        MemoryGraphStyle::ProvenanceReceiptDag,
        MemoryGraphStyle::CanonicalMemoryGraph,
        MemoryGraphStyle::RoutingViewGraph,
        MemoryGraphStyle::ContextPacketGraph,
    ];
    if edges_to_create.iter().any(|edge| {
        matches!(
            edge.edge_kind,
            exo_dag_db_api::MemoryEdgeKind::Contradicts
                | exo_dag_db_api::MemoryEdgeKind::Supersedes
                | exo_dag_db_api::MemoryEdgeKind::Replaces
        )
    }) {
        views.push(MemoryGraphStyle::ContradictionSupersessionGraph);
    }
    views.sort();
    views.dedup();
    views
}

fn proposed_canonical_node(
    input_memory_id: Hash256,
    decision_kind: CanonicalizationDecisionKind,
) -> Option<String> {
    matches!(decision_kind, CanonicalizationDecisionKind::NewCanonical)
        .then(|| input_memory_id.to_string())
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;
    use exo_dag_db_api::{MemoryCandidateKind, MemoryCandidateUse, SimilarityType};

    use super::*;
    use crate::model::TaskAgentWritebackHint;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn scope() -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:agent".into(),
            authority_scope_hash: h(0x90),
            consent_scope_hash: h(0x91),
            permitted_actions: vec!["dagdb:intake".into()],
            expires_at: ts(10_000),
        }
    }

    fn gate() -> DomainGateContext {
        DomainGateContext {
            action: "dagdb:intake".into(),
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Write],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn candidate() -> MemoryCandidate {
        OutputObserver::observe_completed_task_output(
            "request-1".into(),
            h(0x10).to_string(),
            h(0x11).to_string(),
            TaskAgentWritebackHint {
                candidate_kind: MemoryCandidateKind::Summary,
                summary: "canonical graph routing summary".into(),
                evidence_receipts: vec![h(0x12).to_string()],
                risk_hint: RiskClass::R1,
                allowed_future_uses: vec![MemoryCandidateUse::Routing],
                reason_to_remember: "route planning needs this memory".into(),
            },
        )
        .expect("candidate")
    }

    fn input(existing: Vec<PlacementExistingMemory>) -> MemoryPlacementInput {
        MemoryPlacementInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            input_memory_id: h(0x20),
            payload_hash: h(0x21),
            summary: "canonical graph routing summary".into(),
            risk_class: RiskClass::R1,
            validator_status: ValidationStatus::Passed,
            existing_memory: existing,
            requested_decision: None,
            receipt_intent: "placement_completed".into(),
            now: ts(1_000),
        }
    }

    #[test]
    fn placement_controller_owns_graph_allocation() {
        let result = MemoryPlacementController::place_memory_candidate(
            &scope(),
            &gate(),
            &candidate(),
            input(vec![PlacementExistingMemory {
                memory_id: h(0x30),
                payload_hash: h(0x22),
                summary: "canonical graph routing summary".into(),
            }]),
        )
        .expect("placement succeeds");
        assert_eq!(
            result.canonicalization_decision.decision_kind,
            CanonicalizationDecisionKind::NearDuplicate
        );
        assert_eq!(
            result.similarity_results[0].similarity_type,
            SimilarityType::NearDuplicate
        );
        assert!(result.proposed_canonical_node.is_none());
        assert!(
            result
                .graph_views_to_refresh
                .contains(&MemoryGraphStyle::RoutingViewGraph)
        );
    }

    #[test]
    fn placement_controller_runs_without_task_agent() {
        let result = GraphOrganizer::organize(input(Vec::new())).expect("system side placement");
        assert_eq!(
            result.canonicalization_decision.decision_kind,
            CanonicalizationDecisionKind::NewCanonical
        );
        assert_eq!(result.proposed_canonical_node, Some(h(0x20).to_string()));
    }

    #[test]
    fn placement_reuses_scope_and_consent_gates() {
        let mut wrong_scope = scope();
        wrong_scope.namespace = "other".into();
        assert!(matches!(
            MemoryPlacementController::place_memory_candidate(
                &wrong_scope,
                &gate(),
                &candidate(),
                input(Vec::new())
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));
    }

    #[test]
    fn placement_failures_and_contradiction_refresh_are_explicit() {
        let mut invalid_candidate = candidate();
        invalid_candidate.candidate_type = "GraphAllocation".into();
        assert!(matches!(
            MemoryPlacementController::place_memory_candidate(
                &scope(),
                &gate(),
                &invalid_candidate,
                input(Vec::new())
            ),
            Err(DomainError::ValidationFailed)
        ));

        let mut denied_gate = gate();
        denied_gate.consent_decision = ConsentDecision::Denied {
            reason: "no consent".into(),
        };
        assert!(matches!(
            MemoryPlacementController::place_memory_candidate(
                &scope(),
                &denied_gate,
                &candidate(),
                input(Vec::new())
            ),
            Err(DomainError::ConsentDenied { .. })
        ));

        let mut contradiction = input(vec![PlacementExistingMemory {
            memory_id: h(0x30),
            payload_hash: h(0x22),
            summary: "canonical graph routing summary".into(),
        }]);
        contradiction.requested_decision = Some(CanonicalizationDecisionKind::Contradiction);
        let result = GraphOrganizer::organize(contradiction).expect("contradiction placement");
        assert!(
            result
                .graph_views_to_refresh
                .contains(&MemoryGraphStyle::ContradictionSupersessionGraph)
        );
        assert!(result.proposed_canonical_node.is_none());
    }

    #[test]
    fn placement_supersession_is_proposed_without_live_route_mutation() {
        let mut supersession = input(vec![PlacementExistingMemory {
            memory_id: h(0x30),
            payload_hash: h(0x22),
            summary: "canonical graph routing summary".into(),
        }]);
        supersession.requested_decision = Some(CanonicalizationDecisionKind::Supersession);

        let result = GraphOrganizer::organize(supersession).expect("supersession placement");

        assert_eq!(
            result.canonicalization_decision.decision_kind,
            CanonicalizationDecisionKind::Supersession
        );
        assert!(result.route_invalidations.is_empty());
        assert!(
            result
                .graph_views_to_refresh
                .contains(&MemoryGraphStyle::ContradictionSupersessionGraph)
        );
        assert!(result.proposed_canonical_node.is_none());
    }
}
