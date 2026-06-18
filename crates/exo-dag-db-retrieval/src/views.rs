//! Rebuildable graph views for routing and context packet construction.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::Hash256;
use exo_dag_db_api::{GraphEdgeRef, GraphView, GraphViewType, MemoryGraphStyle};
use serde::Serialize;
use thiserror::Error;

use crate::{
    graph::MemoryGraphEdge,
    scoring::{DomainError, DomainResult, hash_event_body},
};

/// View generation errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphViewError {
    /// Dependency graph contains a cycle.
    #[error("dependency_cycle_detected")]
    DependencyCycle,
    /// An edge belongs to a different tenant or namespace than the view.
    #[error("edge_scope_mismatch")]
    EdgeScopeMismatch,
    /// An edge endpoint is outside the authorized node set for the view.
    #[error("edge_endpoint_outside_authorized_nodes")]
    EdgeEndpointOutsideAuthorizedNodes,
}

/// Source-of-truth records used to rebuild a graph view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphViewSource {
    pub tenant_id: String,
    pub namespace: String,
    pub graph_style: MemoryGraphStyle,
    pub source_root_id: Hash256,
    pub node_ids: Vec<Hash256>,
    pub edges: Vec<MemoryGraphEdge>,
    pub view_type: GraphViewType,
}

#[derive(Debug, Serialize)]
struct GraphViewIdMaterial<'a> {
    graph_style: MemoryGraphStyle,
    source_root_id: Hash256,
    included_node_ids: &'a [String],
    included_edge_ids: &'a [String],
    view_type: GraphViewType,
    topological_order: &'a [String],
    transitive_reduction_edges: &'a [GraphEdgeRef],
    omitted_edges: &'a [GraphEdgeRef],
    reason_edges_omitted: &'a [String],
}

/// Build a non-destructive graph view from source-of-truth graph records.
pub fn build_graph_view(source: &GraphViewSource) -> DomainResult<GraphView> {
    let mut node_ids = source.node_ids.clone();
    node_ids.sort();
    node_ids.dedup();

    let authorized_nodes = node_ids.iter().copied().collect::<BTreeSet<_>>();
    for edge in &source.edges {
        if edge.tenant_id != source.tenant_id || edge.namespace != source.namespace {
            return Err(graph_error(GraphViewError::EdgeScopeMismatch));
        }
        if !authorized_nodes.contains(&edge.from_memory_id)
            || !authorized_nodes.contains(&edge.to_memory_id)
        {
            return Err(graph_error(
                GraphViewError::EdgeEndpointOutsideAuthorizedNodes,
            ));
        }
    }

    let topological_hashes = topological_order(&node_ids, &source.edges).map_err(graph_error)?;
    let (transitive_reduction_edges, omitted_edges) =
        transitive_reduction(&source.edges).map_err(graph_error)?;
    let included_node_ids = node_ids.iter().map(ToString::to_string).collect::<Vec<_>>();
    let included_edge_ids = source
        .edges
        .iter()
        .map(|edge| edge.edge_id.to_string())
        .collect::<Vec<_>>();
    let topological_order = topological_hashes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let transitive_reduction_edges = edge_refs(&transitive_reduction_edges);
    let omitted_edges = edge_refs(&omitted_edges);
    let reason_edges_omitted = omitted_edges
        .iter()
        .map(|_| {
            "edge omitted from routing view because source-of-truth graph records contain an alternate dependency path".into()
        })
        .collect::<Vec<_>>();
    let view_id = hash_event_body(&GraphViewIdMaterial {
        graph_style: source.graph_style,
        source_root_id: source.source_root_id,
        included_node_ids: &included_node_ids,
        included_edge_ids: &included_edge_ids,
        view_type: source.view_type,
        topological_order: &topological_order,
        transitive_reduction_edges: &transitive_reduction_edges,
        omitted_edges: &omitted_edges,
        reason_edges_omitted: &reason_edges_omitted,
    })?;
    Ok(GraphView {
        view_id: view_id.to_string(),
        graph_style: source.graph_style,
        source_root_id: source.source_root_id.to_string(),
        included_node_ids,
        included_edge_ids,
        view_type: source.view_type,
        topological_order,
        transitive_reduction_edges,
        omitted_edges,
        reason_edges_omitted,
    })
}

fn graph_error(error: GraphViewError) -> DomainError {
    DomainError::HashMaterial {
        reason: error.to_string(),
    }
}

/// Regenerate a stale graph view from source truth; return the prior view when fresh.
pub fn regenerate_graph_view_if_stale(
    stale: bool,
    existing: &GraphView,
    source: &GraphViewSource,
) -> DomainResult<GraphView> {
    if stale {
        build_graph_view(source)
    } else {
        Ok(existing.clone())
    }
}

/// Return dependency-safe topological order without mutating source edges.
pub fn topological_order(
    node_ids: &[Hash256],
    edges: &[MemoryGraphEdge],
) -> Result<Vec<Hash256>, GraphViewError> {
    let nodes = node_set(node_ids, edges);
    let mut indegree = nodes
        .iter()
        .map(|node| (*node, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing: BTreeMap<Hash256, Vec<Hash256>> = BTreeMap::new();
    for edge in edges {
        outgoing
            .entry(edge.from_memory_id)
            .or_default()
            .push(edge.to_memory_id);
        *indegree.entry(edge.to_memory_id).or_insert(0) += 1;
        indegree.entry(edge.from_memory_id).or_insert(0);
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(*node))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::new();
    while let Some(node) = ready.pop_first() {
        ordered.push(node);
        if let Some(children) = outgoing.get(&node) {
            for child in children {
                let Some(degree) = indegree.get_mut(child) else {
                    continue;
                };
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    ready.insert(*child);
                }
            }
        }
    }
    if ordered.len() != indegree.len() {
        return Err(GraphViewError::DependencyCycle);
    }
    Ok(ordered)
}

/// Compute transitive reduction as a derived view without mutating source edges.
pub fn transitive_reduction(
    edges: &[MemoryGraphEdge],
) -> Result<(Vec<MemoryGraphEdge>, Vec<MemoryGraphEdge>), GraphViewError> {
    topological_order(&[], edges)?;
    let mut kept = Vec::new();
    let mut omitted = Vec::new();
    for edge in edges {
        if path_exists_without_edge(edge.from_memory_id, edge.to_memory_id, edge.edge_id, edges) {
            omitted.push(edge.clone());
        } else {
            kept.push(edge.clone());
        }
    }
    Ok((kept, omitted))
}

fn node_set(node_ids: &[Hash256], edges: &[MemoryGraphEdge]) -> BTreeSet<Hash256> {
    let mut nodes = node_ids.iter().copied().collect::<BTreeSet<_>>();
    for edge in edges {
        nodes.insert(edge.from_memory_id);
        nodes.insert(edge.to_memory_id);
    }
    nodes
}

fn path_exists_without_edge(
    from: Hash256,
    to: Hash256,
    excluded_edge_id: Hash256,
    edges: &[MemoryGraphEdge],
) -> bool {
    let mut stack = vec![from];
    let mut seen = BTreeSet::new();
    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        for edge in edges {
            if edge.edge_id == excluded_edge_id || edge.from_memory_id != current {
                continue;
            }
            if edge.to_memory_id == to {
                return true;
            }
            stack.push(edge.to_memory_id);
        }
    }
    false
}

fn edge_refs(edges: &[MemoryGraphEdge]) -> Vec<GraphEdgeRef> {
    edges
        .iter()
        .map(|edge| GraphEdgeRef {
            from_memory_id: edge.from_memory_id.to_string(),
            to_memory_id: edge.to_memory_id.to_string(),
            edge_kind: edge.edge_kind,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::MemoryEdgeKind;

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn edge(from: u8, to: u8) -> MemoryGraphEdge {
        MemoryGraphEdge::new(
            "tenant-a".into(),
            "primary".into(),
            h(from),
            h(to),
            MemoryEdgeKind::DependsOn,
            MemoryGraphStyle::DependencyDag,
            Some(h(0xf0)),
        )
        .expect("edge hashes")
    }

    #[test]
    fn dependency_dag_rejects_cycles() {
        let edges = vec![edge(1, 2), edge(2, 1)];
        assert_eq!(
            topological_order(&[], &edges),
            Err(GraphViewError::DependencyCycle)
        );
    }

    #[test]
    fn topological_order_vectors() {
        let order = topological_order(&[h(1), h(2), h(3)], &[edge(1, 2), edge(2, 3)])
            .expect("acyclic graph");
        assert_eq!(order, vec![h(1), h(2), h(3)]);
    }

    #[test]
    fn topological_sort_preserves_provenance_edges() {
        let edges = vec![edge(1, 2), edge(2, 3)];
        let before = edges.clone();
        let _ = topological_order(&[], &edges).expect("order");
        assert_eq!(edges, before);
    }

    #[test]
    fn transitive_reduction_preserves_provenance_edges() {
        let edges = vec![edge(1, 2), edge(2, 3), edge(1, 3)];
        let before = edges.clone();
        let (kept, omitted) = transitive_reduction(&edges).expect("reduction");
        assert_eq!(edges, before);
        assert_eq!(kept.len(), 2);
        assert_eq!(omitted.len(), 1);
        assert_eq!(omitted[0].from_memory_id, h(1));
        assert_eq!(omitted[0].to_memory_id, h(3));
    }

    fn source(node_ids: Vec<Hash256>, edges: Vec<MemoryGraphEdge>) -> GraphViewSource {
        GraphViewSource {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            graph_style: MemoryGraphStyle::RoutingViewGraph,
            source_root_id: h(1),
            node_ids,
            edges,
            view_type: GraphViewType::RoutingView,
        }
    }

    #[test]
    fn graph_view_rebuilds_from_source_truth() {
        let source = source(
            vec![h(1), h(2), h(3)],
            vec![edge(1, 2), edge(2, 3), edge(1, 3)],
        );
        let view = build_graph_view(&source).expect("view");
        assert_eq!(view.graph_style, MemoryGraphStyle::RoutingViewGraph);
        assert_eq!(view.transitive_reduction_edges.len(), 2);
        assert_eq!(view.omitted_edges.len(), 1);
    }

    #[test]
    fn routing_view_regenerates_when_stale() {
        let source = source(vec![h(1), h(2)], vec![edge(1, 2)]);
        let existing = build_graph_view(&source).expect("view");
        let regenerated =
            regenerate_graph_view_if_stale(true, &existing, &source).expect("regenerated");
        assert_eq!(regenerated, existing);
        let fresh = regenerate_graph_view_if_stale(false, &existing, &source).expect("fresh");
        assert_eq!(fresh, existing);
    }

    #[test]
    fn routing_view_excludes_unnecessary_raw_payloads() {
        let source = source(
            vec![h(1), h(2), h(3)],
            vec![edge(1, 2), edge(2, 3), edge(1, 3)],
        );
        let view = build_graph_view(&source).expect("view");
        let encoded = serde_json::to_string(&view).expect("serialize");
        assert!(!encoded.contains("raw_payload"));
        assert!(view.reason_edges_omitted[0].contains("source-of-truth"));
    }

    #[test]
    fn graph_view_rejects_edges_from_other_tenants_or_namespaces() {
        let mut foreign_tenant = source(vec![h(1), h(2)], vec![edge(1, 2)]);
        foreign_tenant.edges[0].tenant_id = "tenant-b".into();
        assert!(build_graph_view(&foreign_tenant).is_err());

        let mut foreign_namespace = source(vec![h(1), h(2)], vec![edge(1, 2)]);
        foreign_namespace.edges[0].namespace = "other".into();
        assert!(build_graph_view(&foreign_namespace).is_err());
    }

    #[test]
    fn graph_view_rejects_edge_endpoints_outside_authorized_nodes() {
        let unauthorized_target = source(vec![h(1), h(2)], vec![edge(1, 2), edge(2, 3)]);
        assert!(build_graph_view(&unauthorized_target).is_err());

        let unauthorized_origin = source(vec![h(2), h(3)], vec![edge(1, 2), edge(2, 3)]);
        assert!(build_graph_view(&unauthorized_origin).is_err());
    }
}
