//! Graph query and route-invalidation helpers.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    CanonicalizationDecision, GraphView, GraphViewType, MemoryEdgeKind, MemoryGraphStyle,
    RouteInvalidationReceipt, RouteInvalidationStatus, RouteInvalidationTrigger, RouteStatus,
};

use crate::{
    graph::MemoryGraphEdge,
    scoring::DomainResult,
    views::{GraphViewSource, build_graph_view},
};

/// In-memory graph records used by pure query functions and tests.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphQueryState {
    pub tenant_id: String,
    pub namespace: String,
    pub canonicalization_decisions: Vec<CanonicalizationDecision>,
    pub edges: Vec<MemoryGraphEdge>,
    pub graph_views: Vec<GraphView>,
    pub receipt_ids: Vec<Hash256>,
}

/// Graph query service over source-of-truth graph records.
pub struct GraphQueryService<'a> {
    state: &'a GraphQueryState,
}

impl<'a> GraphQueryService<'a> {
    /// Create a query service over source-of-truth records.
    #[must_use]
    pub const fn new(state: &'a GraphQueryState) -> Self {
        Self { state }
    }

    /// Find the current canonical memory ID for an input memory ID.
    #[must_use]
    pub fn find_canonical(&self, memory_id: Hash256) -> Option<String> {
        let memory_hex = memory_id.to_string();
        self.state
            .canonicalization_decisions
            .iter()
            .find(|decision| decision.input_memory_id == memory_hex)
            .and_then(|decision| decision.canonical_memory_id.clone())
            .or(Some(memory_hex))
    }

    /// Find exact duplicates for a memory ID.
    #[must_use]
    pub fn find_duplicates(&self, memory_id: Hash256) -> Vec<String> {
        self.find_edges(memory_id, MemoryEdgeKind::DuplicateOf)
    }

    /// Find near duplicates for a memory ID.
    #[must_use]
    pub fn find_near_duplicates(&self, memory_id: Hash256) -> Vec<String> {
        self.find_edges(memory_id, MemoryEdgeKind::NearDuplicateOf)
    }

    /// Find related concepts for a memory ID.
    #[must_use]
    pub fn find_related_concepts(&self, memory_id: Hash256) -> Vec<String> {
        self.find_edges(memory_id, MemoryEdgeKind::RelatedTo)
    }

    /// Find contradictions for a memory ID.
    #[must_use]
    pub fn find_contradictions(&self, memory_id: Hash256) -> Vec<String> {
        self.find_edges(memory_id, MemoryEdgeKind::Contradicts)
    }

    /// Find supersessions for a memory ID.
    #[must_use]
    pub fn find_supersessions(&self, memory_id: Hash256) -> Vec<String> {
        self.find_edges(memory_id, MemoryEdgeKind::Supersedes)
    }

    /// Return full provenance receipt IDs.
    #[must_use]
    pub fn get_full_provenance(&self, _memory_id: Hash256) -> Vec<String> {
        self.state
            .receipt_ids
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Return a clean routing view by root ID when already built.
    #[must_use]
    pub fn get_clean_routing_view(&self, root_id: Hash256) -> Option<&GraphView> {
        let root = root_id.to_string();
        self.state.graph_views.iter().find(|view| {
            view.source_root_id == root && view.graph_style == MemoryGraphStyle::RoutingViewGraph
        })
    }

    /// Return the topological order for a view.
    #[must_use]
    pub fn get_topological_order(&self, view_id: &str) -> Option<Vec<String>> {
        self.state
            .graph_views
            .iter()
            .find(|view| view.view_id == view_id)
            .map(|view| view.topological_order.clone())
    }

    /// Return transitive reduction edges for a view.
    #[must_use]
    pub fn get_transitive_reduction(&self, view_id: &str) -> Option<Vec<(String, String)>> {
        self.state
            .graph_views
            .iter()
            .find(|view| view.view_id == view_id)
            .map(|view| {
                view.transitive_reduction_edges
                    .iter()
                    .map(|edge| (edge.from_memory_id.clone(), edge.to_memory_id.clone()))
                    .collect()
            })
    }

    /// Explain placement for a memory ID.
    #[must_use]
    pub fn explain_placement(&self, memory_id: Hash256) -> String {
        let canonical = self
            .find_canonical(memory_id)
            .unwrap_or_else(|| memory_id.to_string());
        format!("memory {} resolves to canonical {}", memory_id, canonical)
    }

    /// Explain canonicalization for a memory ID.
    #[must_use]
    pub fn explain_canonicalization(&self, memory_id: Hash256) -> Option<String> {
        let memory = memory_id.to_string();
        self.state
            .canonicalization_decisions
            .iter()
            .find(|decision| decision.input_memory_id == memory)
            .map(|decision| format!("{:?}: {}", decision.decision_kind, decision.decision_reason))
    }

    fn find_edges(&self, memory_id: Hash256, edge_kind: MemoryEdgeKind) -> Vec<String> {
        let mut matches = self
            .state
            .edges
            .iter()
            .filter(|edge| {
                edge.tenant_id == self.state.tenant_id
                    && edge.namespace == self.state.namespace
                    && edge.from_memory_id == memory_id
                    && edge.edge_kind == edge_kind
            })
            .map(|edge| edge.to_memory_id.to_string())
            .collect::<Vec<_>>();
        matches.sort();
        matches
    }
}

/// Route planner graph style order locked by the plan.
#[must_use]
pub const fn graph_route_planner_order() -> [MemoryGraphStyle; 7] {
    [
        MemoryGraphStyle::SemanticCatalogGraph,
        MemoryGraphStyle::CanonicalMemoryGraph,
        MemoryGraphStyle::ProvenanceReceiptDag,
        MemoryGraphStyle::ContradictionSupersessionGraph,
        MemoryGraphStyle::RoutingViewGraph,
        MemoryGraphStyle::DependencyDag,
        MemoryGraphStyle::ContextPacketGraph,
    ]
}

/// Build a task-specific Context Packet Graph from selected evidence.
#[allow(clippy::too_many_arguments)]
pub fn build_context_packet_graph(
    _task_id: &str,
    _route_id: Hash256,
    tenant_id: String,
    namespace: String,
    source_root_id: Hash256,
    node_ids: Vec<Hash256>,
    edges: Vec<MemoryGraphEdge>,
) -> DomainResult<GraphView> {
    build_graph_view(&GraphViewSource {
        tenant_id,
        namespace,
        graph_style: MemoryGraphStyle::ContextPacketGraph,
        source_root_id,
        node_ids,
        edges,
        view_type: GraphViewType::ContextPacketView,
    })
}

/// Emit a governed route invalidation receipt payload without deleting the route.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn emit_route_invalidation_receipt(
    route_id: Hash256,
    affected_memory_ids: Vec<Hash256>,
    trigger_type: RouteInvalidationTrigger,
    triggering_receipt_id: Hash256,
    prior_route_status: RouteStatus,
    invalidation_reason: String,
    created_at: Timestamp,
    validator_id: Option<String>,
    validation_report_id: Option<Hash256>,
) -> RouteInvalidationReceipt {
    RouteInvalidationReceipt {
        route_id: route_id.to_string(),
        affected_memory_ids: affected_memory_ids
            .iter()
            .map(ToString::to_string)
            .collect(),
        trigger_type,
        triggering_receipt_id: triggering_receipt_id.to_string(),
        prior_route_status,
        new_route_status: invalidation_status_for(trigger_type),
        invalidation_reason,
        created_at: created_at.to_string(),
        validator_id,
        validation_report_id: validation_report_id.map(|id| id.to_string()),
        receipt_intent: "route_invalidated".into(),
        receipt_id: None,
    }
}

fn invalidation_status_for(trigger_type: RouteInvalidationTrigger) -> RouteInvalidationStatus {
    match trigger_type {
        RouteInvalidationTrigger::Superseded | RouteInvalidationTrigger::Replaced => {
            RouteInvalidationStatus::Superseded
        }
        RouteInvalidationTrigger::Revoked
        | RouteInvalidationTrigger::Contradicted
        | RouteInvalidationTrigger::PermissionChanged
        | RouteInvalidationTrigger::RiskChanged => RouteInvalidationStatus::NeedsReview,
    }
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::{
        CanonicalizationDecisionKind, GraphEdgeRef, RiskClass, SimilarityType, ValidationStatus,
    };

    use super::*;
    use crate::canonicalization::{CanonicalizationRequest, decide_canonicalization};

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn edge(from: u8, to: u8, kind: MemoryEdgeKind) -> MemoryGraphEdge {
        MemoryGraphEdge::new(
            "tenant-a".into(),
            "primary".into(),
            h(from),
            h(to),
            kind,
            MemoryGraphStyle::CanonicalMemoryGraph,
            Some(h(0xee)),
        )
        .expect("edge")
    }

    fn decision(kind: CanonicalizationDecisionKind) -> CanonicalizationDecision {
        decide_canonicalization(CanonicalizationRequest {
            input_memory_id: h(1),
            risk_class: RiskClass::R1,
            validator_status: ValidationStatus::Passed,
            similarity_results: vec![exo_dag_db_api::SimilarityResult {
                candidate_memory_id: h(2).to_string(),
                similarity_type: SimilarityType::NearDuplicate,
                similarity_bp: 9_000,
                matched_fields: vec!["summary".into()],
                reason: "fixture".into(),
            }],
            requested_decision: Some(kind),
        })
        .expect("decision")
    }

    fn scoped_state() -> GraphQueryState {
        GraphQueryState {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            ..GraphQueryState::default()
        }
    }

    #[test]
    fn graph_query_explainers() {
        let state = GraphQueryState {
            canonicalization_decisions: vec![decision(CanonicalizationDecisionKind::NearDuplicate)],
            edges: vec![
                edge(1, 2, MemoryEdgeKind::NearDuplicateOf),
                edge(1, 3, MemoryEdgeKind::RelatedTo),
                edge(1, 4, MemoryEdgeKind::Contradicts),
                edge(1, 5, MemoryEdgeKind::Supersedes),
            ],
            receipt_ids: vec![h(0xaa)],
            ..scoped_state()
        };
        let query = GraphQueryService::new(&state);
        assert_eq!(query.find_canonical(h(1)), Some(h(2).to_string()));
        assert_eq!(query.find_near_duplicates(h(1)), vec![h(2).to_string()]);
        assert_eq!(query.find_related_concepts(h(1)), vec![h(3).to_string()]);
        assert_eq!(query.find_contradictions(h(1)), vec![h(4).to_string()]);
        assert_eq!(query.find_supersessions(h(1)), vec![h(5).to_string()]);
        assert!(query.explain_placement(h(1)).contains("canonical"));
        assert!(
            query
                .explain_canonicalization(h(1))
                .expect("explanation")
                .contains("NearDuplicate")
        );
        assert_eq!(query.get_full_provenance(h(1)), vec![h(0xaa).to_string()]);
    }

    #[test]
    fn graph_query_views_duplicates_and_missing_paths_are_deterministic() {
        let routing_view = GraphView {
            view_id: "view-1".into(),
            graph_style: MemoryGraphStyle::RoutingViewGraph,
            source_root_id: h(1).to_string(),
            included_node_ids: vec![h(1).to_string(), h(2).to_string()],
            included_edge_ids: vec![h(0xee).to_string()],
            view_type: GraphViewType::RoutingView,
            topological_order: vec![h(1).to_string(), h(2).to_string()],
            transitive_reduction_edges: vec![GraphEdgeRef {
                from_memory_id: h(1).to_string(),
                to_memory_id: h(2).to_string(),
                edge_kind: MemoryEdgeKind::DependsOn,
            }],
            omitted_edges: Vec::new(),
            reason_edges_omitted: Vec::new(),
        };
        let wrong_style_same_root = GraphView {
            view_id: "view-2".into(),
            graph_style: MemoryGraphStyle::CanonicalMemoryGraph,
            source_root_id: h(9).to_string(),
            included_node_ids: vec![h(9).to_string()],
            included_edge_ids: Vec::new(),
            view_type: GraphViewType::CanonicalView,
            topological_order: vec![h(9).to_string()],
            transitive_reduction_edges: Vec::new(),
            omitted_edges: Vec::new(),
            reason_edges_omitted: Vec::new(),
        };
        let state = GraphQueryState {
            edges: vec![
                edge(1, 3, MemoryEdgeKind::DuplicateOf),
                edge(1, 2, MemoryEdgeKind::DuplicateOf),
                edge(1, 4, MemoryEdgeKind::RelatedTo),
                edge(9, 8, MemoryEdgeKind::DuplicateOf),
            ],
            graph_views: vec![routing_view, wrong_style_same_root],
            ..scoped_state()
        };
        let query = GraphQueryService::new(&state);
        assert_eq!(query.find_canonical(h(9)), Some(h(9).to_string()));
        assert_eq!(
            query.find_duplicates(h(1)),
            vec![h(2).to_string(), h(3).to_string()]
        );
        assert!(query.get_clean_routing_view(h(1)).is_some());
        assert!(query.get_clean_routing_view(h(9)).is_none());
        assert_eq!(
            query.get_topological_order("view-1"),
            Some(vec![h(1).to_string(), h(2).to_string()])
        );
        assert_eq!(
            query.get_transitive_reduction("view-1"),
            Some(vec![(h(1).to_string(), h(2).to_string())])
        );
        assert!(query.get_topological_order("missing").is_none());
        assert!(query.explain_canonicalization(h(9)).is_none());
    }

    #[test]
    fn graph_aware_routing_order_vectors() {
        assert_eq!(
            graph_route_planner_order(),
            [
                MemoryGraphStyle::SemanticCatalogGraph,
                MemoryGraphStyle::CanonicalMemoryGraph,
                MemoryGraphStyle::ProvenanceReceiptDag,
                MemoryGraphStyle::ContradictionSupersessionGraph,
                MemoryGraphStyle::RoutingViewGraph,
                MemoryGraphStyle::DependencyDag,
                MemoryGraphStyle::ContextPacketGraph,
            ]
        );
    }

    #[test]
    fn context_packet_graph_evidence_vectors() {
        let view = build_context_packet_graph(
            "task-1",
            h(9),
            "tenant-a".into(),
            "primary".into(),
            h(1),
            vec![h(1), h(2)],
            vec![edge(1, 2, MemoryEdgeKind::IncludedInContextPacket)],
        )
        .expect("context packet graph");
        assert_eq!(view.graph_style, MemoryGraphStyle::ContextPacketGraph);
        assert_eq!(view.view_type, GraphViewType::ContextPacketView);
        assert_eq!(
            view.included_node_ids,
            vec![h(1).to_string(), h(2).to_string()]
        );
    }

    #[test]
    fn graph_query_excludes_edges_outside_the_state_scope() {
        let mut foreign_tenant_edge = edge(1, 6, MemoryEdgeKind::RelatedTo);
        foreign_tenant_edge.tenant_id = "tenant-b".into();
        let mut foreign_namespace_edge = edge(1, 7, MemoryEdgeKind::RelatedTo);
        foreign_namespace_edge.namespace = "other".into();
        let state = GraphQueryState {
            edges: vec![
                edge(1, 3, MemoryEdgeKind::RelatedTo),
                foreign_tenant_edge,
                foreign_namespace_edge,
            ],
            ..scoped_state()
        };
        let query = GraphQueryService::new(&state);
        assert_eq!(query.find_related_concepts(h(1)), vec![h(3).to_string()]);
    }

    #[test]
    fn context_packet_graph_rejects_cross_scope_evidence() {
        let mut foreign_edge = edge(1, 2, MemoryEdgeKind::IncludedInContextPacket);
        foreign_edge.tenant_id = "tenant-b".into();
        assert!(
            build_context_packet_graph(
                "task-1",
                h(9),
                "tenant-a".into(),
                "primary".into(),
                h(1),
                vec![h(1), h(2)],
                vec![foreign_edge],
            )
            .is_err()
        );

        assert!(
            build_context_packet_graph(
                "task-1",
                h(9),
                "tenant-a".into(),
                "primary".into(),
                h(1),
                vec![h(1)],
                vec![edge(1, 2, MemoryEdgeKind::IncludedInContextPacket)],
            )
            .is_err()
        );
    }

    #[test]
    fn graph_route_invalidation_vectors() {
        let receipt = emit_route_invalidation_receipt(
            h(0x10),
            vec![h(0x20), h(0x21)],
            RouteInvalidationTrigger::RiskChanged,
            h(0x30),
            RouteStatus::Active,
            "risk changed".into(),
            ts(3_000),
            Some("did:exo:validator".into()),
            Some(h(0x40)),
        );
        assert_eq!(
            receipt.new_route_status,
            RouteInvalidationStatus::NeedsReview
        );
        assert_eq!(receipt.affected_memory_ids.len(), 2);
        assert_eq!(receipt.validation_report_id, Some(h(0x40).to_string()));
    }

    #[test]
    fn route_invalidation_emits_governed_receipt() {
        let receipt = emit_route_invalidation_receipt(
            h(0x10),
            vec![h(0x20)],
            RouteInvalidationTrigger::Superseded,
            h(0x30),
            RouteStatus::Active,
            "memory superseded".into(),
            ts(3_000),
            None,
            None,
        );
        assert_eq!(receipt.receipt_intent, "route_invalidated");
        assert_eq!(receipt.triggering_receipt_id, h(0x30).to_string());
        assert_eq!(
            receipt.new_route_status,
            RouteInvalidationStatus::Superseded
        );
    }

    #[test]
    fn route_invalidation_preserves_audit_history() {
        let receipt = emit_route_invalidation_receipt(
            h(0x10),
            vec![h(0x20)],
            RouteInvalidationTrigger::Contradicted,
            h(0x30),
            RouteStatus::Active,
            "contradiction surfaced".into(),
            ts(3_000),
            None,
            Some(h(0x41)),
        );
        assert_eq!(receipt.prior_route_status, RouteStatus::Active);
        assert_eq!(
            receipt.new_route_status,
            RouteInvalidationStatus::NeedsReview
        );
        assert_eq!(receipt.route_id, h(0x10).to_string());
        assert!(receipt.receipt_id.is_none());
    }

    #[test]
    fn route_invalidation_status_vectors_cover_every_trigger() {
        for (trigger, expected) in [
            (
                RouteInvalidationTrigger::Revoked,
                RouteInvalidationStatus::NeedsReview,
            ),
            (
                RouteInvalidationTrigger::Superseded,
                RouteInvalidationStatus::Superseded,
            ),
            (
                RouteInvalidationTrigger::Contradicted,
                RouteInvalidationStatus::NeedsReview,
            ),
            (
                RouteInvalidationTrigger::Replaced,
                RouteInvalidationStatus::Superseded,
            ),
            (
                RouteInvalidationTrigger::PermissionChanged,
                RouteInvalidationStatus::NeedsReview,
            ),
            (
                RouteInvalidationTrigger::RiskChanged,
                RouteInvalidationStatus::NeedsReview,
            ),
        ] {
            assert_eq!(invalidation_status_for(trigger), expected);
        }
    }
}
