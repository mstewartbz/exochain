//! Deterministic memory graph primitives for EXOCHAIN DAG DB.

use exo_core::Hash256;
use exo_dag_db_api::{MemoryEdgeKind, MemoryGraphStyle, MemoryNodeKind};
use serde::Serialize;
use thiserror::Error;

use crate::scoring::{DomainResult, hash_event_body};

/// Memory graph node reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryGraphNode {
    pub memory_id: Hash256,
    pub node_kind: MemoryNodeKind,
    pub graph_style: MemoryGraphStyle,
}

/// Memory graph edge reference. Provenance edges are never deleted by view code.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryGraphEdge {
    pub edge_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub from_memory_id: Hash256,
    pub to_memory_id: Hash256,
    pub edge_kind: MemoryEdgeKind,
    pub graph_style: MemoryGraphStyle,
    pub provenance_receipt_id: Option<Hash256>,
}

#[derive(Debug, Serialize)]
struct EdgeHashMaterial<'a> {
    tenant_id: &'a str,
    namespace: &'a str,
    from_memory_id: Hash256,
    to_memory_id: Hash256,
    edge_kind: MemoryEdgeKind,
    graph_style: MemoryGraphStyle,
}

impl MemoryGraphEdge {
    /// Build a deterministic edge ID from tenant, namespace, endpoints, edge kind, and graph style.
    pub fn new(
        tenant_id: String,
        namespace: String,
        from_memory_id: Hash256,
        to_memory_id: Hash256,
        edge_kind: MemoryEdgeKind,
        graph_style: MemoryGraphStyle,
        provenance_receipt_id: Option<Hash256>,
    ) -> DomainResult<Self> {
        let edge_id = hash_event_body(&EdgeHashMaterial {
            tenant_id: &tenant_id,
            namespace: &namespace,
            from_memory_id,
            to_memory_id,
            edge_kind,
            graph_style,
        })?;
        Ok(Self {
            edge_id,
            tenant_id,
            namespace,
            from_memory_id,
            to_memory_id,
            edge_kind,
            graph_style,
            provenance_receipt_id,
        })
    }
}

/// Required placement trace steps in exact execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlacementTraceStep {
    SourceVerification,
    RiskClassification,
    IdentityAssignment,
    ExactDuplicateCheck,
    SimilarityOverlayCheck,
    CanonicalizationDecision,
    MetadataAttachment,
    SemanticCatalogGraphPlacement,
    ProvenanceReceiptDagPlacement,
    CanonicalMemoryGraphUpdate,
    DependencyDagUpdate,
    ContradictionSupersessionGraphUpdate,
    Validation,
    ReceiptWriteback,
    RoutingViewGraphRefresh,
    RouteInvalidation,
    QueryExposure,
}

impl PlacementTraceStep {
    /// Stable label used in placement traces and persistence.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceVerification => "Source verification",
            Self::RiskClassification => "Risk classification",
            Self::IdentityAssignment => "Identity assignment",
            Self::ExactDuplicateCheck => "Exact duplicate check",
            Self::SimilarityOverlayCheck => "Similarity overlay check",
            Self::CanonicalizationDecision => "Canonicalization decision",
            Self::MetadataAttachment => "Metadata attachment",
            Self::SemanticCatalogGraphPlacement => "Semantic Catalog Graph placement",
            Self::ProvenanceReceiptDagPlacement => "Provenance Receipt DAG placement",
            Self::CanonicalMemoryGraphUpdate => "Canonical Memory Graph update",
            Self::DependencyDagUpdate => "Dependency DAG update",
            Self::ContradictionSupersessionGraphUpdate => {
                "Contradiction / Supersession Graph update"
            }
            Self::Validation => "Validation",
            Self::ReceiptWriteback => "Receipt writeback",
            Self::RoutingViewGraphRefresh => "Routing View Graph refresh",
            Self::RouteInvalidation => "Route invalidation",
            Self::QueryExposure => "Query exposure",
        }
    }
}

/// Graph validation errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    /// Placement trace omitted or reordered an intake step.
    #[error("placement_order_mismatch: expected {expected} at {index}")]
    PlacementOrderMismatch {
        /// Expected step label.
        expected: &'static str,
        /// Mismatched step index.
        index: usize,
    },
}

/// Return the exact deterministic intake order.
#[must_use]
pub const fn required_placement_steps() -> [PlacementTraceStep; 17] {
    [
        PlacementTraceStep::SourceVerification,
        PlacementTraceStep::RiskClassification,
        PlacementTraceStep::IdentityAssignment,
        PlacementTraceStep::ExactDuplicateCheck,
        PlacementTraceStep::SimilarityOverlayCheck,
        PlacementTraceStep::CanonicalizationDecision,
        PlacementTraceStep::MetadataAttachment,
        PlacementTraceStep::SemanticCatalogGraphPlacement,
        PlacementTraceStep::ProvenanceReceiptDagPlacement,
        PlacementTraceStep::CanonicalMemoryGraphUpdate,
        PlacementTraceStep::DependencyDagUpdate,
        PlacementTraceStep::ContradictionSupersessionGraphUpdate,
        PlacementTraceStep::Validation,
        PlacementTraceStep::ReceiptWriteback,
        PlacementTraceStep::RoutingViewGraphRefresh,
        PlacementTraceStep::RouteInvalidation,
        PlacementTraceStep::QueryExposure,
    ]
}

/// Validate that the placement trace preserves the exact intake order.
pub fn validate_placement_order(trace: &[PlacementTraceStep]) -> Result<(), GraphError> {
    let required = required_placement_steps();
    if trace.len() != required.len() {
        return Err(GraphError::PlacementOrderMismatch {
            expected: "end of placement trace",
            index: required.len(),
        });
    }
    for (index, expected) in required.iter().copied().enumerate() {
        if trace.get(index).copied() != Some(expected) {
            return Err(GraphError::PlacementOrderMismatch {
                expected: expected.label(),
                index,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_trace_enforces_exact_intake_order() {
        let trace = required_placement_steps();
        assert_eq!(validate_placement_order(&trace), Ok(()));
        assert_eq!(trace[0].label(), "Source verification");
        assert_eq!(trace[16].label(), "Query exposure");

        let mut wrong = trace;
        wrong.swap(0, 1);
        assert_eq!(
            validate_placement_order(&wrong),
            Err(GraphError::PlacementOrderMismatch {
                expected: "Source verification",
                index: 0,
            })
        );
    }

    #[test]
    fn placement_trace_rejects_extra_trailing_steps() {
        let mut extended = required_placement_steps().to_vec();
        extended.push(PlacementTraceStep::QueryExposure);
        assert_eq!(
            validate_placement_order(&extended),
            Err(GraphError::PlacementOrderMismatch {
                expected: "end of placement trace",
                index: 17,
            })
        );
    }

    #[test]
    fn graph_primitives_cover_labels_and_edge_identity() {
        let labels = required_placement_steps()
            .iter()
            .map(|step| step.label())
            .collect::<Vec<_>>();
        assert_eq!(labels.len(), 17);
        assert!(labels.contains(&"Risk classification"));
        assert!(labels.contains(&"Contradiction / Supersession Graph update"));
        assert!(labels.contains(&"Routing View Graph refresh"));

        let node = MemoryGraphNode {
            memory_id: Hash256::from_bytes([0x10; 32]),
            node_kind: MemoryNodeKind::Canonical,
            graph_style: MemoryGraphStyle::CanonicalMemoryGraph,
        };
        assert_eq!(node.node_kind, MemoryNodeKind::Canonical);

        let first = MemoryGraphEdge::new(
            "tenant-a".into(),
            "default".into(),
            Hash256::from_bytes([0x11; 32]),
            Hash256::from_bytes([0x12; 32]),
            MemoryEdgeKind::DependsOn,
            MemoryGraphStyle::DependencyDag,
            None,
        )
        .expect("edge");
        let second = MemoryGraphEdge::new(
            "tenant-a".into(),
            "default".into(),
            Hash256::from_bytes([0x11; 32]),
            Hash256::from_bytes([0x12; 32]),
            MemoryEdgeKind::DependsOn,
            MemoryGraphStyle::DependencyDag,
            None,
        )
        .expect("edge");
        assert_eq!(first.edge_id, second.edge_id);
    }
}
