//! Deterministic layered graph contracts for graph-of-graphs memory.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::Hash256;
use exo_dag_db_api::MemoryGraphStyle;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema version emitted by layered graph invariant reports.
pub const LAYERED_GRAPH_INVARIANT_REPORT_SCHEMA_VERSION: &str = "layered_graph_invariant_report_v1";

/// Layer taxonomy for additive graph-of-graphs storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayeredGraphLayerKind {
    /// Root layer for a tenant/namespace memory graph.
    Root,
    /// Repository-scoped child layer.
    Repository,
    /// Knowledge-graph import child layer.
    KnowledgeGraph,
    /// Source-file or source-record child subgraph.
    SourceSubgraph,
    /// Task or request child subgraph.
    TaskSubgraph,
    /// Rollup or summary layer.
    Rollup,
    /// Route-oriented layer used by packet selection.
    Route,
}

impl LayeredGraphLayerKind {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Repository => "repository",
            Self::KnowledgeGraph => "knowledge_graph",
            Self::SourceSubgraph => "source_subgraph",
            Self::TaskSubgraph => "task_subgraph",
            Self::Rollup => "rollup",
            Self::Route => "route",
        }
    }
}

/// Layer membership role for graph nodes inside a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayeredGraphMembershipRole {
    /// Root node for the layer.
    Root,
    /// Container node that owns a child subgraph.
    Container,
    /// Ordinary member node.
    Member,
    /// Summary node for the layer.
    Summary,
    /// Route anchor node.
    RouteAnchor,
}

impl LayeredGraphMembershipRole {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Container => "container",
            Self::Member => "member",
            Self::Summary => "summary",
            Self::RouteAnchor => "route_anchor",
        }
    }
}

/// Typed edge between layered graph records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayeredGraphLayerEdgeKind {
    /// Parent layer contains a child subgraph.
    ContainsSubgraph,
    /// Traversal drills down from parent layer to child layer.
    DrillsDownTo,
    /// Traversal rolls up from child layer to parent layer.
    RollsUpTo,
    /// Cross-layer reference between peer or distant layers.
    CrossLayerRef,
    /// One layer summarizes another.
    SummarizesLayer,
}

impl LayeredGraphLayerEdgeKind {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ContainsSubgraph => "contains_subgraph",
            Self::DrillsDownTo => "drills_down_to",
            Self::RollsUpTo => "rolls_up_to",
            Self::CrossLayerRef => "cross_layer_ref",
            Self::SummarizesLayer => "summarizes_layer",
        }
    }
}

/// Minimal graph node reference required to validate layer memberships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredGraphNodeRef {
    /// Existing `dagdb_graph_nodes.graph_node_id`.
    pub graph_node_id: Hash256,
    /// Tenant scope copied from the existing graph node.
    pub tenant_id: String,
    /// Namespace scope copied from the existing graph node.
    pub namespace: String,
}

/// Additive layered graph row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredGraphLayer {
    /// Primary layer identifier.
    pub layer_id: Hash256,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Existing root memory object for this layer.
    pub root_memory_id: Hash256,
    /// Parent layer for child layers.
    pub parent_layer_id: Option<Hash256>,
    /// Existing graph node that owns this child layer.
    pub parent_graph_node_id: Option<Hash256>,
    /// Root is depth zero; child layers are positive depth.
    pub layer_depth: u32,
    /// Layer taxonomy.
    pub layer_kind: LayeredGraphLayerKind,
    /// Existing graph style hosted by the layer.
    pub graph_style: MemoryGraphStyle,
    /// Stable path unique within tenant and namespace.
    pub layer_path: String,
    /// Safe metadata only; raw payloads are not part of the contract.
    pub metadata: serde_json::Value,
    /// Created physical HLC component.
    pub created_at_physical_ms: u64,
    /// Created logical HLC component.
    pub created_at_logical: u32,
    /// Updated physical HLC component.
    pub updated_at_physical_ms: u64,
    /// Updated logical HLC component.
    pub updated_at_logical: u32,
}

/// Additive binding between an existing graph node and a layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredGraphMembership {
    /// Primary membership identifier.
    pub layer_membership_id: Hash256,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Existing layer identifier.
    pub layer_id: Hash256,
    /// Existing graph node identifier.
    pub graph_node_id: Hash256,
    /// Existing graph style hosted by the membership.
    pub graph_style: MemoryGraphStyle,
    /// Node role inside the layer.
    pub membership_role: LayeredGraphMembershipRole,
    /// Deterministic local rank inside the layer.
    pub local_node_rank: u32,
    /// Safe metadata only; raw payloads are not part of the contract.
    pub metadata: serde_json::Value,
    /// Created physical HLC component.
    pub created_at_physical_ms: u64,
    /// Created logical HLC component.
    pub created_at_logical: u32,
    /// Updated physical HLC component.
    pub updated_at_physical_ms: u64,
    /// Updated logical HLC component.
    pub updated_at_logical: u32,
}

/// Additive edge between two layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredGraphLayerEdge {
    /// Primary layer edge identifier.
    pub layer_edge_id: Hash256,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Existing graph style hosted by the layer edge.
    pub graph_style: MemoryGraphStyle,
    /// Source layer identifier.
    pub from_layer_id: Hash256,
    /// Target layer identifier.
    pub to_layer_id: Hash256,
    /// Layer edge taxonomy.
    pub edge_kind: LayeredGraphLayerEdgeKind,
    /// Optional EXOCHAIN receipt binding for the edge.
    pub receipt_hash: Option<Hash256>,
    /// Safe metadata only; raw payloads are not part of the contract.
    pub metadata: serde_json::Value,
    /// Created physical HLC component.
    pub created_at_physical_ms: u64,
    /// Created logical HLC component.
    pub created_at_logical: u32,
    /// Updated physical HLC component.
    pub updated_at_physical_ms: u64,
    /// Updated logical HLC component.
    pub updated_at_logical: u32,
}

/// Layered graph validation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayeredGraphValidationStatus {
    /// All invariants passed.
    Passed,
    /// One or more invariants failed.
    Failed,
}

/// Single invariant failure with a stable code and subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredGraphInvariantFailure {
    /// Stable machine-readable invariant code.
    pub invariant_code: String,
    /// Layer, membership, edge, or node identifier associated with the failure.
    pub subject_id: String,
}

/// Machine-readable report for PRD01 layered graph invariant checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredGraphInvariantReport {
    /// Report schema version.
    pub schema_version: String,
    /// Validation status.
    pub validation_status: LayeredGraphValidationStatus,
    /// Number of supplied graph node references checked.
    pub checked_graph_node_count: usize,
    /// Number of supplied layers checked.
    pub checked_layer_count: usize,
    /// Number of supplied memberships checked.
    pub checked_membership_count: usize,
    /// Number of supplied layer edges checked.
    pub checked_layer_edge_count: usize,
    /// Failed invariants; empty when validation passed.
    pub failed_invariants: Vec<LayeredGraphInvariantFailure>,
}

/// Errors raised by fail-closed layered invariant validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LayeredGraphInvariantError {
    /// One or more invariant failures were found.
    #[error("layered_graph_invariants_failed: {failed_count}")]
    Failed {
        /// Number of failed invariants.
        failed_count: usize,
        /// Stable invariant failures.
        failed_invariants: Vec<LayeredGraphInvariantFailure>,
    },
}

/// Build an invariant report without failing on invalid records.
#[must_use]
pub fn build_layered_graph_invariant_report(
    graph_nodes: &[LayeredGraphNodeRef],
    layers: &[LayeredGraphLayer],
    memberships: &[LayeredGraphMembership],
    layer_edges: &[LayeredGraphLayerEdge],
) -> LayeredGraphInvariantReport {
    let mut failed_invariants = Vec::new();
    let graph_node_index = graph_nodes
        .iter()
        .map(|node| (node.graph_node_id, node))
        .collect::<BTreeMap<_, _>>();
    let layer_index = layers
        .iter()
        .map(|layer| (layer.layer_id, layer))
        .collect::<BTreeMap<_, _>>();
    let membership_pairs = memberships
        .iter()
        .map(|membership| (membership.layer_id, membership.graph_node_id))
        .collect::<BTreeSet<_>>();

    validate_layer_records(layers, &layer_index, &mut failed_invariants);
    validate_membership_records(
        memberships,
        &layer_index,
        &graph_node_index,
        &mut failed_invariants,
    );
    validate_parent_node_bindings(layers, &membership_pairs, &mut failed_invariants);
    validate_layer_edge_records(layer_edges, &layer_index, &mut failed_invariants);

    let validation_status = if failed_invariants.is_empty() {
        LayeredGraphValidationStatus::Passed
    } else {
        LayeredGraphValidationStatus::Failed
    };
    LayeredGraphInvariantReport {
        schema_version: LAYERED_GRAPH_INVARIANT_REPORT_SCHEMA_VERSION.to_owned(),
        validation_status,
        checked_graph_node_count: graph_nodes.len(),
        checked_layer_count: layers.len(),
        checked_membership_count: memberships.len(),
        checked_layer_edge_count: layer_edges.len(),
        failed_invariants,
    }
}

/// Validate layered graph invariants and fail closed on any issue.
pub fn validate_layered_graph_invariants(
    graph_nodes: &[LayeredGraphNodeRef],
    layers: &[LayeredGraphLayer],
    memberships: &[LayeredGraphMembership],
    layer_edges: &[LayeredGraphLayerEdge],
) -> Result<LayeredGraphInvariantReport, LayeredGraphInvariantError> {
    let report =
        build_layered_graph_invariant_report(graph_nodes, layers, memberships, layer_edges);
    if report.failed_invariants.is_empty() {
        Ok(report)
    } else {
        Err(LayeredGraphInvariantError::Failed {
            failed_count: report.failed_invariants.len(),
            failed_invariants: report.failed_invariants,
        })
    }
}

fn validate_layer_records(
    layers: &[LayeredGraphLayer],
    layer_index: &BTreeMap<Hash256, &LayeredGraphLayer>,
    failed_invariants: &mut Vec<LayeredGraphInvariantFailure>,
) {
    let mut scoped_paths = BTreeSet::new();
    for layer in layers {
        let subject = layer.layer_id.to_string();
        if layer.layer_path.is_empty() {
            push_failure(
                failed_invariants,
                "layered_empty_layer_path",
                subject.clone(),
            );
        }
        let scoped_path = (
            layer.tenant_id.clone(),
            layer.namespace.clone(),
            layer.layer_path.clone(),
        );
        if !scoped_paths.insert(scoped_path) {
            push_failure(
                failed_invariants,
                "layered_duplicate_layer_path",
                subject.clone(),
            );
        }
        if layer.layer_depth == 0 {
            if layer.parent_layer_id.is_some() || layer.parent_graph_node_id.is_some() {
                push_failure(failed_invariants, "layered_root_has_parent", subject);
            }
            continue;
        }
        let Some(parent_layer_id) = layer.parent_layer_id else {
            push_failure(
                failed_invariants,
                "layered_child_missing_parent_layer",
                subject.clone(),
            );
            continue;
        };
        if layer.parent_graph_node_id.is_none() {
            push_failure(
                failed_invariants,
                "layered_child_missing_parent_node",
                subject.clone(),
            );
        }
        let Some(parent_layer) = layer_index.get(&parent_layer_id) else {
            push_failure(
                failed_invariants,
                "layered_orphan_child_layer",
                subject.clone(),
            );
            continue;
        };
        if layer.tenant_id != parent_layer.tenant_id {
            push_failure(
                failed_invariants,
                "layered_tenant_mismatch",
                subject.clone(),
            );
        }
        if layer.namespace != parent_layer.namespace {
            push_failure(
                failed_invariants,
                "layered_namespace_mismatch",
                subject.clone(),
            );
        }
        if layer.layer_depth != parent_layer.layer_depth.saturating_add(1) {
            push_failure(failed_invariants, "layered_depth_mismatch", subject);
        }
    }
}

fn validate_membership_records(
    memberships: &[LayeredGraphMembership],
    layer_index: &BTreeMap<Hash256, &LayeredGraphLayer>,
    graph_node_index: &BTreeMap<Hash256, &LayeredGraphNodeRef>,
    failed_invariants: &mut Vec<LayeredGraphInvariantFailure>,
) {
    for membership in memberships {
        let subject = membership.layer_membership_id.to_string();
        let Some(layer) = layer_index.get(&membership.layer_id) else {
            push_failure(
                failed_invariants,
                "layered_membership_missing_layer",
                subject.clone(),
            );
            continue;
        };
        let Some(graph_node) = graph_node_index.get(&membership.graph_node_id) else {
            push_failure(
                failed_invariants,
                "layered_membership_missing_graph_node",
                subject,
            );
            continue;
        };
        if membership.tenant_id != layer.tenant_id || membership.tenant_id != graph_node.tenant_id {
            push_failure(
                failed_invariants,
                "layered_tenant_mismatch",
                subject.clone(),
            );
        }
        if membership.namespace != layer.namespace || membership.namespace != graph_node.namespace {
            push_failure(failed_invariants, "layered_namespace_mismatch", subject);
        }
    }
}

fn validate_parent_node_bindings(
    layers: &[LayeredGraphLayer],
    membership_pairs: &BTreeSet<(Hash256, Hash256)>,
    failed_invariants: &mut Vec<LayeredGraphInvariantFailure>,
) {
    for layer in layers {
        let (Some(parent_layer_id), Some(parent_graph_node_id)) =
            (layer.parent_layer_id, layer.parent_graph_node_id)
        else {
            continue;
        };
        if !membership_pairs.contains(&(parent_layer_id, parent_graph_node_id)) {
            push_failure(
                failed_invariants,
                "layered_parent_node_not_in_parent_layer",
                layer.layer_id.to_string(),
            );
        }
    }
}

fn validate_layer_edge_records(
    layer_edges: &[LayeredGraphLayerEdge],
    layer_index: &BTreeMap<Hash256, &LayeredGraphLayer>,
    failed_invariants: &mut Vec<LayeredGraphInvariantFailure>,
) {
    for layer_edge in layer_edges {
        let subject = layer_edge.layer_edge_id.to_string();
        let Some(from_layer) = layer_index.get(&layer_edge.from_layer_id) else {
            push_failure(
                failed_invariants,
                "layered_dangling_layer_edge",
                subject.clone(),
            );
            continue;
        };
        let Some(to_layer) = layer_index.get(&layer_edge.to_layer_id) else {
            push_failure(
                failed_invariants,
                "layered_dangling_layer_edge",
                subject.clone(),
            );
            continue;
        };
        if layer_edge.tenant_id != from_layer.tenant_id
            || layer_edge.tenant_id != to_layer.tenant_id
        {
            push_failure(
                failed_invariants,
                "layered_tenant_mismatch",
                subject.clone(),
            );
        }
        if layer_edge.namespace != from_layer.namespace
            || layer_edge.namespace != to_layer.namespace
        {
            push_failure(failed_invariants, "layered_namespace_mismatch", subject);
        }
    }
}

fn push_failure(
    failed_invariants: &mut Vec<LayeredGraphInvariantFailure>,
    invariant_code: &str,
    subject_id: String,
) {
    failed_invariants.push(LayeredGraphInvariantFailure {
        invariant_code: invariant_code.to_owned(),
        subject_id,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layered_graph_schema_exports_stable_labels() {
        assert_eq!(
            LayeredGraphLayerKind::KnowledgeGraph.as_str(),
            "knowledge_graph"
        );
        assert_eq!(LayeredGraphLayerKind::Root.as_str(), "root");
        assert_eq!(LayeredGraphLayerKind::Repository.as_str(), "repository");
        assert_eq!(
            LayeredGraphLayerKind::SourceSubgraph.as_str(),
            "source_subgraph"
        );
        assert_eq!(
            LayeredGraphLayerKind::TaskSubgraph.as_str(),
            "task_subgraph"
        );
        assert_eq!(LayeredGraphLayerKind::Rollup.as_str(), "rollup");
        assert_eq!(LayeredGraphLayerKind::Route.as_str(), "route");
        assert_eq!(LayeredGraphMembershipRole::Root.as_str(), "root");
        assert_eq!(LayeredGraphMembershipRole::Container.as_str(), "container");
        assert_eq!(LayeredGraphMembershipRole::Member.as_str(), "member");
        assert_eq!(LayeredGraphMembershipRole::Summary.as_str(), "summary");
        assert_eq!(
            LayeredGraphMembershipRole::RouteAnchor.as_str(),
            "route_anchor"
        );
        assert_eq!(
            LayeredGraphLayerEdgeKind::ContainsSubgraph.as_str(),
            "contains_subgraph"
        );
        assert_eq!(
            LayeredGraphLayerEdgeKind::DrillsDownTo.as_str(),
            "drills_down_to"
        );
        assert_eq!(LayeredGraphLayerEdgeKind::RollsUpTo.as_str(), "rolls_up_to");
        assert_eq!(
            LayeredGraphLayerEdgeKind::CrossLayerRef.as_str(),
            "cross_layer_ref"
        );
        assert_eq!(
            LayeredGraphLayerEdgeKind::SummarizesLayer.as_str(),
            "summarizes_layer"
        );
        assert_eq!(
            LAYERED_GRAPH_INVARIANT_REPORT_SCHEMA_VERSION,
            "layered_graph_invariant_report_v1"
        );
    }

    #[test]
    fn layered_graph_invariants_accept_valid_root_and_child() {
        let graph_nodes = fixture_graph_nodes();
        let layers = fixture_layers();
        let memberships = fixture_memberships();
        let layer_edges = fixture_layer_edges();

        let report =
            validate_layered_graph_invariants(&graph_nodes, &layers, &memberships, &layer_edges)
                .expect("valid fixture passes layered graph invariants");

        assert_eq!(
            report.validation_status,
            LayeredGraphValidationStatus::Passed
        );
        assert_eq!(report.checked_layer_count, 2);
        assert!(report.failed_invariants.is_empty());
    }

    #[test]
    fn layered_graph_invariants_reject_duplicate_paths() {
        let graph_nodes = fixture_graph_nodes();
        let mut layers = fixture_layers();
        layers[1].layer_path = layers[0].layer_path.clone();
        let error = validate_layered_graph_invariants(
            &graph_nodes,
            &layers,
            &fixture_memberships(),
            &fixture_layer_edges(),
        )
        .expect_err("duplicate scoped paths fail");

        assert_error_has_code(error, "layered_duplicate_layer_path");
    }

    #[test]
    fn layered_graph_invariants_reject_invalid_layer_records() {
        assert_layer_failure(
            |layers| layers[0].layer_path.clear(),
            "layered_empty_layer_path",
        );
        assert_layer_failure(
            |layers| layers[0].parent_layer_id = Some(h(0x12)),
            "layered_root_has_parent",
        );
        assert_layer_failure(
            |layers| layers[1].parent_layer_id = None,
            "layered_child_missing_parent_layer",
        );
        assert_layer_failure(
            |layers| layers[1].parent_graph_node_id = None,
            "layered_child_missing_parent_node",
        );
        assert_layer_failure(
            |layers| layers[1].parent_layer_id = Some(h(0x99)),
            "layered_orphan_child_layer",
        );
        assert_layer_failure(
            |layers| layers[1].tenant_id = "tenant-b".to_owned(),
            "layered_tenant_mismatch",
        );
        assert_layer_failure(
            |layers| layers[1].namespace = "other".to_owned(),
            "layered_namespace_mismatch",
        );
        assert_layer_failure(|layers| layers[1].layer_depth = 2, "layered_depth_mismatch");
    }

    #[test]
    fn layered_graph_invariants_reject_invalid_membership_records() {
        assert_membership_failure(
            |memberships| memberships[1].layer_id = h(0x99),
            "layered_membership_missing_layer",
        );
        assert_membership_failure(
            |memberships| memberships[1].graph_node_id = h(0x99),
            "layered_membership_missing_graph_node",
        );
        assert_membership_failure(
            |memberships| memberships[1].tenant_id = "tenant-b".to_owned(),
            "layered_tenant_mismatch",
        );
        assert_membership_failure(
            |memberships| memberships[1].namespace = "other".to_owned(),
            "layered_namespace_mismatch",
        );
    }

    #[test]
    fn layered_graph_invariants_reject_dangling_layer_edges() {
        let graph_nodes = fixture_graph_nodes();
        let mut layer_edges = fixture_layer_edges();
        layer_edges[0].to_layer_id = h(0x99);
        let error = validate_layered_graph_invariants(
            &graph_nodes,
            &fixture_layers(),
            &fixture_memberships(),
            &layer_edges,
        )
        .expect_err("dangling layer edge fails");

        assert_error_has_code(error, "layered_dangling_layer_edge");
    }

    #[test]
    fn layered_graph_invariants_reject_invalid_layer_edge_records() {
        assert_layer_edge_failure(
            |layer_edges| layer_edges[0].from_layer_id = h(0x99),
            "layered_dangling_layer_edge",
        );
        assert_layer_edge_failure(
            |layer_edges| layer_edges[0].tenant_id = "tenant-b".to_owned(),
            "layered_tenant_mismatch",
        );
        assert_layer_edge_failure(
            |layer_edges| layer_edges[0].namespace = "other".to_owned(),
            "layered_namespace_mismatch",
        );
    }

    #[test]
    fn layered_graph_invariants_reject_parent_node_outside_parent_layer() {
        let graph_nodes = fixture_graph_nodes();
        let memberships = vec![fixture_memberships()[1].clone()];
        let error = validate_layered_graph_invariants(
            &graph_nodes,
            &fixture_layers(),
            &memberships,
            &fixture_layer_edges(),
        )
        .expect_err("parent node must be member of parent layer");

        assert_error_has_code(error, "layered_parent_node_not_in_parent_layer");
    }

    fn assert_layer_failure(mutate: impl FnOnce(&mut Vec<LayeredGraphLayer>), expected_code: &str) {
        let graph_nodes = fixture_graph_nodes();
        let mut layers = fixture_layers();
        mutate(&mut layers);
        let error = validate_layered_graph_invariants(
            &graph_nodes,
            &layers,
            &fixture_memberships(),
            &fixture_layer_edges(),
        )
        .expect_err("invalid layer record fails");

        assert_error_has_code(error, expected_code);
    }

    fn assert_membership_failure(
        mutate: impl FnOnce(&mut Vec<LayeredGraphMembership>),
        expected_code: &str,
    ) {
        let graph_nodes = fixture_graph_nodes();
        let mut memberships = fixture_memberships();
        mutate(&mut memberships);
        let error = validate_layered_graph_invariants(
            &graph_nodes,
            &fixture_layers(),
            &memberships,
            &fixture_layer_edges(),
        )
        .expect_err("invalid membership record fails");

        assert_error_has_code(error, expected_code);
    }

    fn assert_layer_edge_failure(
        mutate: impl FnOnce(&mut Vec<LayeredGraphLayerEdge>),
        expected_code: &str,
    ) {
        let graph_nodes = fixture_graph_nodes();
        let mut layer_edges = fixture_layer_edges();
        mutate(&mut layer_edges);
        let error = validate_layered_graph_invariants(
            &graph_nodes,
            &fixture_layers(),
            &fixture_memberships(),
            &layer_edges,
        )
        .expect_err("invalid layer edge record fails");

        assert_error_has_code(error, expected_code);
    }

    fn assert_error_has_code(error: LayeredGraphInvariantError, code: &str) {
        let LayeredGraphInvariantError::Failed {
            failed_invariants, ..
        } = error;
        assert!(
            failed_invariants
                .iter()
                .any(|failure| failure.invariant_code == code),
            "missing {code} in {failed_invariants:?}"
        );
    }

    fn fixture_graph_nodes() -> Vec<LayeredGraphNodeRef> {
        vec![
            LayeredGraphNodeRef {
                graph_node_id: h(0x21),
                tenant_id: "tenant-a".to_owned(),
                namespace: "default".to_owned(),
            },
            LayeredGraphNodeRef {
                graph_node_id: h(0x22),
                tenant_id: "tenant-a".to_owned(),
                namespace: "default".to_owned(),
            },
        ]
    }

    fn fixture_layers() -> Vec<LayeredGraphLayer> {
        vec![
            LayeredGraphLayer {
                layer_id: h(0x11),
                tenant_id: "tenant-a".to_owned(),
                namespace: "default".to_owned(),
                root_memory_id: h(0x31),
                parent_layer_id: None,
                parent_graph_node_id: None,
                layer_depth: 0,
                layer_kind: LayeredGraphLayerKind::Root,
                graph_style: MemoryGraphStyle::SemanticCatalogGraph,
                layer_path: "root".to_owned(),
                metadata: serde_json::json!({}),
                created_at_physical_ms: 1,
                created_at_logical: 0,
                updated_at_physical_ms: 1,
                updated_at_logical: 1,
            },
            LayeredGraphLayer {
                layer_id: h(0x12),
                tenant_id: "tenant-a".to_owned(),
                namespace: "default".to_owned(),
                root_memory_id: h(0x32),
                parent_layer_id: Some(h(0x11)),
                parent_graph_node_id: Some(h(0x21)),
                layer_depth: 1,
                layer_kind: LayeredGraphLayerKind::Repository,
                graph_style: MemoryGraphStyle::DependencyDag,
                layer_path: "root/repository".to_owned(),
                metadata: serde_json::json!({}),
                created_at_physical_ms: 2,
                created_at_logical: 0,
                updated_at_physical_ms: 2,
                updated_at_logical: 1,
            },
        ]
    }

    fn fixture_memberships() -> Vec<LayeredGraphMembership> {
        vec![
            LayeredGraphMembership {
                layer_membership_id: h(0x41),
                tenant_id: "tenant-a".to_owned(),
                namespace: "default".to_owned(),
                layer_id: h(0x11),
                graph_node_id: h(0x21),
                graph_style: MemoryGraphStyle::SemanticCatalogGraph,
                membership_role: LayeredGraphMembershipRole::Root,
                local_node_rank: 0,
                metadata: serde_json::json!({}),
                created_at_physical_ms: 1,
                created_at_logical: 0,
                updated_at_physical_ms: 1,
                updated_at_logical: 1,
            },
            LayeredGraphMembership {
                layer_membership_id: h(0x42),
                tenant_id: "tenant-a".to_owned(),
                namespace: "default".to_owned(),
                layer_id: h(0x12),
                graph_node_id: h(0x22),
                graph_style: MemoryGraphStyle::DependencyDag,
                membership_role: LayeredGraphMembershipRole::Member,
                local_node_rank: 0,
                metadata: serde_json::json!({}),
                created_at_physical_ms: 2,
                created_at_logical: 0,
                updated_at_physical_ms: 2,
                updated_at_logical: 1,
            },
        ]
    }

    fn fixture_layer_edges() -> Vec<LayeredGraphLayerEdge> {
        vec![LayeredGraphLayerEdge {
            layer_edge_id: h(0x51),
            tenant_id: "tenant-a".to_owned(),
            namespace: "default".to_owned(),
            graph_style: MemoryGraphStyle::DependencyDag,
            from_layer_id: h(0x11),
            to_layer_id: h(0x12),
            edge_kind: LayeredGraphLayerEdgeKind::ContainsSubgraph,
            receipt_hash: None,
            metadata: serde_json::json!({}),
            created_at_physical_ms: 3,
            created_at_logical: 0,
            updated_at_physical_ms: 3,
            updated_at_logical: 1,
        }]
    }

    const fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }
}
