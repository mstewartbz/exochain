//! Pure PRD05 layered backfill planning and compatibility contracts.
//!
//! This module is side-effect free. It maps existing flat graph rows into
//! layered graph rows, applies those rows to an in-memory snapshot for replay
//! proof, and records compatibility/non-claim evidence without mutating a
//! database or weakening flat graph behavior.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use exo_core::Hash256;
use exo_dag_db_api::MemoryGraphStyle;
use serde::{Deserialize, Serialize};

use crate::{
    layered_graph::{
        LayeredGraphInvariantError, LayeredGraphInvariantFailure, LayeredGraphLayer,
        LayeredGraphLayerEdge, LayeredGraphLayerEdgeKind, LayeredGraphLayerKind,
        LayeredGraphMembership, LayeredGraphMembershipRole, LayeredGraphNodeRef,
        LayeredGraphValidationStatus, validate_layered_graph_invariants,
    },
    scoring::hash_event_body,
};

/// Schema version emitted by PRD05 backfill plans.
pub const LAYERED_BACKFILL_PLAN_SCHEMA_VERSION: &str = "dagdb_layered_backfill_plan_v1";
/// Schema version emitted by PRD05 execution/replay reports.
pub const LAYERED_BACKFILL_EXECUTION_SCHEMA_VERSION: &str = "dagdb_layered_backfill_execution_v1";
/// Schema version emitted by PRD05 compatibility reports.
pub const LAYERED_BACKFILL_COMPATIBILITY_SCHEMA_VERSION: &str =
    "dagdb_layered_backfill_compatibility_v1";

const ROOT_LAYER_PATH: &str = "root";
const REPOSITORY_LAYER_PATH: &str = "root/repository";
const KNOWLEDGE_GRAPH_LAYER_PATH: &str = "root/knowledge-graph";
const BACKFILL_LAYER_ID_DOMAIN: &str = "exo.dagdb.layered_backfill.layer_id";
const BACKFILL_MEMBERSHIP_ID_DOMAIN: &str = "exo.dagdb.layered_backfill.membership_id";
const BACKFILL_LAYER_EDGE_ID_DOMAIN: &str = "exo.dagdb.layered_backfill.layer_edge_id";

/// Flat node source classifier used by migration planning.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayeredBackfillSourceKind {
    /// Repository or source-tree backed memory.
    Repository,
    /// KnowledgeGraph or semantic import memory.
    KnowledgeGraph,
    /// Ambiguous current memory remains visible in the root layer.
    #[default]
    Ambiguous,
}

impl LayeredBackfillSourceKind {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::KnowledgeGraph => "knowledge_graph",
            Self::Ambiguous => "ambiguous",
        }
    }
}

/// Existing flat graph node row needed by PRD05 planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredBackfillGraphNode {
    /// Existing graph node identifier.
    pub graph_node_id: Hash256,
    /// Existing memory object identifier backing the node.
    pub memory_id: Hash256,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Existing graph style.
    pub graph_style: MemoryGraphStyle,
    /// Existing node kind label.
    pub node_kind: String,
    /// Safe catalog path segments when known.
    pub catalog_path: Vec<String>,
    /// Source classifier supplied by import/writeback metadata.
    #[serde(default)]
    pub source_kind: LayeredBackfillSourceKind,
    /// Safe metadata only.
    pub metadata: serde_json::Value,
}

/// Existing flat graph edge row needed by PRD05 planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredBackfillGraphEdge {
    /// Existing graph edge identifier.
    pub graph_edge_id: Hash256,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Existing graph style.
    pub graph_style: MemoryGraphStyle,
    /// Existing source graph node.
    pub from_graph_node_id: Hash256,
    /// Existing target graph node.
    pub to_graph_node_id: Hash256,
    /// Existing edge kind label.
    pub edge_kind: String,
}

/// Snapshot of current flat and layered repository/test state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredBackfillInput {
    /// Tenant scope to backfill.
    pub tenant_id: String,
    /// Namespace scope to backfill.
    pub namespace: String,
    /// Root memory object used by the root layer.
    pub root_memory_id: Hash256,
    /// Existing flat graph nodes.
    pub flat_graph_nodes: Vec<LayeredBackfillGraphNode>,
    /// Existing flat graph edges.
    pub flat_graph_edges: Vec<LayeredBackfillGraphEdge>,
    /// Existing layers already present before PRD05 backfill.
    #[serde(default)]
    pub existing_layers: Vec<LayeredGraphLayer>,
    /// Existing memberships already present before PRD05 backfill.
    #[serde(default)]
    pub existing_memberships: Vec<LayeredGraphMembership>,
    /// Existing layer edges already present before PRD05 backfill.
    #[serde(default)]
    pub existing_layer_edges: Vec<LayeredGraphLayerEdge>,
}

/// Stable reason a flat node maps to a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayeredBackfillLayerReason {
    /// Visible root fallback for ambiguous or container membership.
    RootFallback,
    /// Repository source material maps to the repository child layer.
    RepositorySource,
    /// Knowledge graph material maps to the knowledge-graph child layer.
    KnowledgeGraphSource,
}

impl LayeredBackfillLayerReason {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RootFallback => "root_fallback",
            Self::RepositorySource => "repository_source",
            Self::KnowledgeGraphSource => "knowledge_graph_source",
        }
    }
}

/// Deterministic node-to-layer mapping emitted by the planner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredBackfillNodeMapping {
    /// Existing graph node identifier.
    pub graph_node_id: Hash256,
    /// Existing memory object identifier.
    pub memory_id: Hash256,
    /// Selected layer identifier.
    pub layer_id: Hash256,
    /// Selected layer path.
    pub layer_path: String,
    /// Selected layer kind.
    pub layer_kind: LayeredGraphLayerKind,
    /// Role for the node inside the selected layer.
    pub membership_role: LayeredGraphMembershipRole,
    /// Stable local rank inside the selected layer.
    pub local_node_rank: u32,
    /// Why the node was placed there.
    pub mapping_reason: LayeredBackfillLayerReason,
}

/// Rejected current-state record with stable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredBackfillRejectedRecord {
    /// Record kind such as `graph_node` or `graph_edge`.
    pub record_kind: String,
    /// Safe identifier for the rejected subject.
    pub record_id: String,
    /// Stable reason code.
    pub reason: String,
}

/// Count summary used before and after fixture execution.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LayeredBackfillCounts {
    /// Flat graph node rows.
    pub flat_graph_node_count: u32,
    /// Flat graph edge rows.
    pub flat_graph_edge_count: u32,
    /// Layer rows.
    pub layer_count: u32,
    /// Layer membership rows.
    pub membership_count: u32,
    /// Layer edge rows.
    pub layer_edge_count: u32,
}

/// Machine-readable PRD05 plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredBackfillPlan {
    /// Schema version.
    pub schema_version: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Counts before proposed additions.
    pub before_counts: LayeredBackfillCounts,
    /// Expected counts after applying proposed additions once.
    pub after_counts: LayeredBackfillCounts,
    /// Deterministic node mappings.
    pub node_mappings: Vec<LayeredBackfillNodeMapping>,
    /// Layer rows to create or reuse.
    pub proposed_layers: Vec<LayeredGraphLayer>,
    /// Membership rows to create or reuse.
    pub proposed_memberships: Vec<LayeredGraphMembership>,
    /// Layer edge rows to create or reuse.
    pub proposed_layer_edges: Vec<LayeredGraphLayerEdge>,
    /// Rejected records.
    pub rejected_records: Vec<LayeredBackfillRejectedRecord>,
    /// True when at least one ambiguous record remains root-visible.
    pub flat_fallback_count: u32,
    /// Explicit non-claims preserved by PRD05.
    pub non_claims: Vec<String>,
}

/// Machine-readable PRD05 execution/replay report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredBackfillExecutionReport {
    /// Schema version.
    pub schema_version: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Namespace scope.
    pub namespace: String,
    /// Execution mode, currently fixture/repository-test.
    pub execution_mode: String,
    /// Execution status.
    pub execution_status: String,
    /// Counts before applying the plan.
    pub before_counts: LayeredBackfillCounts,
    /// Counts after first apply.
    pub after_counts: LayeredBackfillCounts,
    /// Counts after replaying the same plan.
    pub replay_counts: LayeredBackfillCounts,
    /// Rows added on first apply.
    pub inserted_layer_count: u32,
    /// Membership rows added on first apply.
    pub inserted_membership_count: u32,
    /// Layer-edge rows added on first apply.
    pub inserted_layer_edge_count: u32,
    /// Rows added on replay; all must be zero.
    pub replay_inserted_layer_count: u32,
    /// Membership rows added on replay; all must be zero.
    pub replay_inserted_membership_count: u32,
    /// Layer-edge rows added on replay; all must be zero.
    pub replay_inserted_layer_edge_count: u32,
    /// Whether replay is idempotent.
    pub idempotent_replay: bool,
    /// Rejected records carried from the plan.
    pub rejected_records: Vec<LayeredBackfillRejectedRecord>,
    /// Invariant failures after applying the plan.
    pub failed_invariants: Vec<LayeredGraphInvariantFailure>,
    /// Explicit non-claims preserved by PRD05.
    pub non_claims: Vec<String>,
}

/// Compatibility surface result consumed by PRD05 reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredCompatibilitySurface {
    /// Stable surface name.
    pub surface: String,
    /// Command or proof handle.
    pub proof_ref: String,
    /// Status such as `passed` or `blocked`.
    pub status: String,
}

/// Stale evidence item used to prevent overclaiming old proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredStaleEvidenceItem {
    /// Artifact or PRD family.
    pub artifact_ref: String,
    /// Current PRD05 treatment.
    pub status: String,
    /// Reason it must remain historical or be rerun.
    pub reason: String,
}

/// Machine-readable PRD05 compatibility/stale-evidence report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredBackfillCompatibilityReport {
    /// Schema version.
    pub schema_version: String,
    /// Overall status.
    pub compatibility_status: String,
    /// Surfaces validated after layered additions exist.
    pub validated_surfaces: Vec<LayeredCompatibilitySurface>,
    /// Old evidence treatment.
    pub stale_evidence: Vec<LayeredStaleEvidenceItem>,
    /// Claims rejected by this compatibility report.
    pub rejected_overclaims: Vec<String>,
    /// Explicit non-claims preserved by PRD05.
    pub non_claims: Vec<String>,
}

/// PRD05 backfill errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayeredBackfillError {
    /// Invalid tenant/namespace or scope mismatch.
    InvalidScope {
        /// Stable reason.
        reason: String,
    },
    /// Current state cannot anchor a root layer.
    MissingRoot {
        /// Stable reason.
        reason: String,
    },
    /// Unsafe path-like or raw material was found.
    UnsafeMaterial {
        /// Stable field.
        field: &'static str,
    },
    /// Hash material could not be serialized.
    HashMaterial {
        /// Stable reason.
        reason: String,
    },
    /// Layered invariant validation failed.
    Invariants {
        /// Failed invariants.
        failed_invariants: Vec<LayeredGraphInvariantFailure>,
    },
}

impl fmt::Display for LayeredBackfillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScope { reason } => write!(formatter, "invalid_scope:{reason}"),
            Self::MissingRoot { reason } => write!(formatter, "missing_root:{reason}"),
            Self::UnsafeMaterial { field } => write!(formatter, "unsafe_material:{field}"),
            Self::HashMaterial { reason } => write!(formatter, "hash_material:{reason}"),
            Self::Invariants { failed_invariants } => {
                write!(
                    formatter,
                    "layered_invariants_failed:{}",
                    failed_invariants.len()
                )
            }
        }
    }
}

impl Error for LayeredBackfillError {}

/// Build a deterministic additive plan that maps flat graph rows into layers.
pub fn plan_layered_backfill(
    input: &LayeredBackfillInput,
) -> Result<LayeredBackfillPlan, LayeredBackfillError> {
    validate_input(input)?;
    let before_counts = counts_for(
        input.flat_graph_nodes.len(),
        input.flat_graph_edges.len(),
        input.existing_layers.len(),
        input.existing_memberships.len(),
        input.existing_layer_edges.len(),
    )?;
    let mut layers_by_path = input
        .existing_layers
        .iter()
        .map(|layer| {
            (
                (
                    layer.tenant_id.clone(),
                    layer.namespace.clone(),
                    layer.layer_path.clone(),
                ),
                layer.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut memberships_by_pair = input
        .existing_memberships
        .iter()
        .map(|membership| {
            (
                membership.tenant_id.clone(),
                membership.namespace.clone(),
                membership.layer_id,
                membership.graph_node_id,
            )
        })
        .collect::<BTreeSet<_>>();
    let mut layer_edges_by_tuple = input
        .existing_layer_edges
        .iter()
        .map(|edge| {
            (
                edge.tenant_id.clone(),
                edge.namespace.clone(),
                edge.graph_style,
                edge.from_layer_id,
                edge.to_layer_id,
                edge.edge_kind,
            )
        })
        .collect::<BTreeSet<_>>();

    let mut proposed_layers = Vec::new();
    let mut proposed_memberships = Vec::new();
    let mut proposed_layer_edges = Vec::new();
    let mut node_mappings = Vec::new();
    let mut rejected_records = Vec::new();

    let mut nodes = input.flat_graph_nodes.clone();
    nodes.sort_by_key(|node| {
        (
            node.source_kind,
            node.catalog_path.clone(),
            node.graph_node_id,
        )
    });
    let root_anchor = nodes
        .first()
        .ok_or_else(|| LayeredBackfillError::MissingRoot {
            reason: "flat_graph_nodes_empty".to_owned(),
        })?
        .graph_node_id;

    let root_layer = ensure_layer(
        input,
        &mut layers_by_path,
        &mut proposed_layers,
        LayerSpec {
            layer_path: ROOT_LAYER_PATH,
            layer_depth: 0,
            layer_kind: LayeredGraphLayerKind::Root,
            graph_style: MemoryGraphStyle::CanonicalMemoryGraph,
            root_memory_id: input.root_memory_id,
            parent_layer_id: None,
            parent_graph_node_id: None,
        },
    )?;

    let mut child_anchor_by_path = BTreeMap::<String, Hash256>::new();
    for node in &nodes {
        let (layer_path, layer_kind, reason) = selected_layer_for_node(node);
        if layer_path == ROOT_LAYER_PATH {
            continue;
        }
        child_anchor_by_path
            .entry(layer_path.to_owned())
            .or_insert(node.graph_node_id);
        ensure_layer(
            input,
            &mut layers_by_path,
            &mut proposed_layers,
            LayerSpec {
                layer_path,
                layer_depth: 1,
                layer_kind,
                graph_style: node.graph_style,
                root_memory_id: node.memory_id,
                parent_layer_id: Some(root_layer.layer_id),
                parent_graph_node_id: Some(node.graph_node_id),
            },
        )?;
        let child_layer = layers_by_path
            .get(&scoped_layer_key(input, layer_path))
            .ok_or_else(|| LayeredBackfillError::MissingRoot {
                reason: "child_layer_not_indexed".to_owned(),
            })?
            .clone();
        ensure_layer_edge(
            input,
            &mut layer_edges_by_tuple,
            &mut proposed_layer_edges,
            LayerEdgeSpec {
                from_layer_id: root_layer.layer_id,
                to_layer_id: child_layer.layer_id,
                edge_kind: LayeredGraphLayerEdgeKind::ContainsSubgraph,
                graph_style: node.graph_style,
            },
        )?;
        let mapping = LayeredBackfillNodeMapping {
            graph_node_id: node.graph_node_id,
            memory_id: node.memory_id,
            layer_id: child_layer.layer_id,
            layer_path: child_layer.layer_path.clone(),
            layer_kind: child_layer.layer_kind,
            membership_role: LayeredGraphMembershipRole::Member,
            local_node_rank: rank_for_layer(&node.graph_node_id, &nodes, layer_path)?,
            mapping_reason: reason,
        };
        node_mappings.push(mapping.clone());
        ensure_membership(
            input,
            &mut memberships_by_pair,
            &mut proposed_memberships,
            MembershipSpec {
                layer_id: mapping.layer_id,
                graph_node_id: node.graph_node_id,
                graph_style: node.graph_style,
                membership_role: LayeredGraphMembershipRole::Member,
                local_node_rank: mapping.local_node_rank,
                mapping_reason: mapping.mapping_reason,
            },
        )?;
    }

    for node in &nodes {
        let (layer_path, layer_kind, reason) = selected_layer_for_node(node);
        if layer_path != ROOT_LAYER_PATH
            && child_anchor_by_path.get(layer_path) != Some(&node.graph_node_id)
        {
            continue;
        }
        let membership_role = if layer_path == ROOT_LAYER_PATH {
            if node.graph_node_id == root_anchor {
                LayeredGraphMembershipRole::Root
            } else {
                LayeredGraphMembershipRole::Member
            }
        } else if node.graph_node_id == root_anchor {
            LayeredGraphMembershipRole::Root
        } else {
            LayeredGraphMembershipRole::Container
        };
        let mapping = LayeredBackfillNodeMapping {
            graph_node_id: node.graph_node_id,
            memory_id: node.memory_id,
            layer_id: root_layer.layer_id,
            layer_path: ROOT_LAYER_PATH.to_owned(),
            layer_kind: LayeredGraphLayerKind::Root,
            membership_role,
            local_node_rank: rank_for_layer(&node.graph_node_id, &nodes, ROOT_LAYER_PATH)?,
            mapping_reason: if layer_kind == LayeredGraphLayerKind::Root {
                reason
            } else {
                LayeredBackfillLayerReason::RootFallback
            },
        };
        node_mappings.push(mapping.clone());
        ensure_membership(
            input,
            &mut memberships_by_pair,
            &mut proposed_memberships,
            MembershipSpec {
                layer_id: root_layer.layer_id,
                graph_node_id: node.graph_node_id,
                graph_style: node.graph_style,
                membership_role,
                local_node_rank: mapping.local_node_rank,
                mapping_reason: mapping.mapping_reason,
            },
        )?;
    }

    validate_flat_edges(input, &nodes, &mut rejected_records);
    node_mappings.sort_by_key(|mapping| {
        (
            mapping.layer_path.clone(),
            mapping.local_node_rank,
            mapping.graph_node_id,
        )
    });
    proposed_layers.sort_by_key(|layer| (layer.layer_depth, layer.layer_path.clone()));
    proposed_memberships.sort_by_key(|membership| {
        (
            membership.layer_id,
            membership.local_node_rank,
            membership.graph_node_id,
        )
    });
    proposed_layer_edges.sort_by_key(|edge| (edge.from_layer_id, edge.to_layer_id, edge.edge_kind));

    let after_counts = counts_for(
        input.flat_graph_nodes.len(),
        input.flat_graph_edges.len(),
        input.existing_layers.len() + proposed_layers.len(),
        input.existing_memberships.len() + proposed_memberships.len(),
        input.existing_layer_edges.len() + proposed_layer_edges.len(),
    )?;
    Ok(LayeredBackfillPlan {
        schema_version: LAYERED_BACKFILL_PLAN_SCHEMA_VERSION.to_owned(),
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        before_counts,
        after_counts,
        node_mappings,
        proposed_layers,
        proposed_memberships,
        proposed_layer_edges,
        rejected_records,
        flat_fallback_count: u32_from_usize(
            input
                .flat_graph_nodes
                .iter()
                .filter(|node| selected_layer_for_node(node).0 == ROOT_LAYER_PATH)
                .count(),
        )?,
        non_claims: layered_backfill_non_claims(),
    })
}

/// Apply a plan to an in-memory snapshot and replay it to prove idempotency.
pub fn execute_layered_backfill_fixture(
    input: &LayeredBackfillInput,
    plan: &LayeredBackfillPlan,
) -> Result<LayeredBackfillExecutionReport, LayeredBackfillError> {
    if plan.tenant_id != input.tenant_id || plan.namespace != input.namespace {
        return Err(LayeredBackfillError::InvalidScope {
            reason: "plan_scope_mismatch".to_owned(),
        });
    }
    let before_counts = plan.before_counts.clone();
    let mut layers = input.existing_layers.clone();
    let mut memberships = input.existing_memberships.clone();
    let mut layer_edges = input.existing_layer_edges.clone();
    let first = apply_plan_rows(&mut layers, &mut memberships, &mut layer_edges, plan)?;
    let graph_nodes = input
        .flat_graph_nodes
        .iter()
        .map(|node| LayeredGraphNodeRef {
            graph_node_id: node.graph_node_id,
            tenant_id: node.tenant_id.clone(),
            namespace: node.namespace.clone(),
        })
        .collect::<Vec<_>>();
    let invariant_report =
        validate_layered_graph_invariants(&graph_nodes, &layers, &memberships, &layer_edges)
            .map_err(|error| match error {
                LayeredGraphInvariantError::Failed {
                    failed_invariants, ..
                } => LayeredBackfillError::Invariants { failed_invariants },
            })?;
    if invariant_report.validation_status != LayeredGraphValidationStatus::Passed {
        return Err(LayeredBackfillError::Invariants {
            failed_invariants: invariant_report.failed_invariants,
        });
    }
    let after_counts = counts_for(
        input.flat_graph_nodes.len(),
        input.flat_graph_edges.len(),
        layers.len(),
        memberships.len(),
        layer_edges.len(),
    )?;
    let replay = apply_plan_rows(&mut layers, &mut memberships, &mut layer_edges, plan)?;
    let replay_counts = counts_for(
        input.flat_graph_nodes.len(),
        input.flat_graph_edges.len(),
        layers.len(),
        memberships.len(),
        layer_edges.len(),
    )?;
    Ok(LayeredBackfillExecutionReport {
        schema_version: LAYERED_BACKFILL_EXECUTION_SCHEMA_VERSION.to_owned(),
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        execution_mode: "fixture".to_owned(),
        execution_status: "passed".to_owned(),
        before_counts,
        after_counts,
        replay_counts,
        inserted_layer_count: first.layers,
        inserted_membership_count: first.memberships,
        inserted_layer_edge_count: first.layer_edges,
        replay_inserted_layer_count: replay.layers,
        replay_inserted_membership_count: replay.memberships,
        replay_inserted_layer_edge_count: replay.layer_edges,
        idempotent_replay: replay.layers == 0 && replay.memberships == 0 && replay.layer_edges == 0,
        rejected_records: plan.rejected_records.clone(),
        failed_invariants: Vec::new(),
        non_claims: layered_backfill_non_claims(),
    })
}

/// Build a compatibility report from explicit checked surfaces.
pub fn build_layered_backfill_compatibility_report(
    validated_surfaces: Vec<LayeredCompatibilitySurface>,
    stale_evidence: Vec<LayeredStaleEvidenceItem>,
) -> LayeredBackfillCompatibilityReport {
    let compatibility_status = if validated_surfaces
        .iter()
        .all(|surface| surface.status == "passed")
        && !validated_surfaces.is_empty()
    {
        "passed"
    } else {
        "blocked"
    };
    LayeredBackfillCompatibilityReport {
        schema_version: LAYERED_BACKFILL_COMPATIBILITY_SCHEMA_VERSION.to_owned(),
        compatibility_status: compatibility_status.to_owned(),
        validated_surfaces,
        stale_evidence,
        rejected_overclaims: vec![
            "layered_final_acceptance_without_prd06_prd08".to_owned(),
            "production_migration_without_operator_approval".to_owned(),
            "old_flat_evidence_as_layered_evidence".to_owned(),
        ],
        non_claims: layered_backfill_non_claims(),
    }
}

fn validate_input(input: &LayeredBackfillInput) -> Result<(), LayeredBackfillError> {
    validate_scope(&input.tenant_id, &input.namespace)?;
    for node in &input.flat_graph_nodes {
        validate_scope(&node.tenant_id, &node.namespace)?;
        if node.tenant_id != input.tenant_id || node.namespace != input.namespace {
            return Err(LayeredBackfillError::InvalidScope {
                reason: "graph_node_scope_mismatch".to_owned(),
            });
        }
        reject_forbidden_json("graph_node.metadata", &node.metadata)?;
        for segment in &node.catalog_path {
            validate_slug(segment, "catalog_path")?;
        }
    }
    for edge in &input.flat_graph_edges {
        validate_scope(&edge.tenant_id, &edge.namespace)?;
        if edge.tenant_id != input.tenant_id || edge.namespace != input.namespace {
            return Err(LayeredBackfillError::InvalidScope {
                reason: "graph_edge_scope_mismatch".to_owned(),
            });
        }
    }
    for layer in &input.existing_layers {
        validate_scope(&layer.tenant_id, &layer.namespace)?;
        if layer.tenant_id != input.tenant_id || layer.namespace != input.namespace {
            return Err(LayeredBackfillError::InvalidScope {
                reason: "existing_layer_scope_mismatch".to_owned(),
            });
        }
    }
    for membership in &input.existing_memberships {
        validate_scope(&membership.tenant_id, &membership.namespace)?;
        if membership.tenant_id != input.tenant_id || membership.namespace != input.namespace {
            return Err(LayeredBackfillError::InvalidScope {
                reason: "existing_membership_scope_mismatch".to_owned(),
            });
        }
    }
    for edge in &input.existing_layer_edges {
        validate_scope(&edge.tenant_id, &edge.namespace)?;
        if edge.tenant_id != input.tenant_id || edge.namespace != input.namespace {
            return Err(LayeredBackfillError::InvalidScope {
                reason: "existing_layer_edge_scope_mismatch".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_scope(tenant_id: &str, namespace: &str) -> Result<(), LayeredBackfillError> {
    if tenant_id.trim().is_empty() || namespace.trim().is_empty() {
        return Err(LayeredBackfillError::InvalidScope {
            reason: "empty_tenant_or_namespace".to_owned(),
        });
    }
    Ok(())
}

fn selected_layer_for_node(
    node: &LayeredBackfillGraphNode,
) -> (
    &'static str,
    LayeredGraphLayerKind,
    LayeredBackfillLayerReason,
) {
    match node.source_kind {
        LayeredBackfillSourceKind::Repository => (
            REPOSITORY_LAYER_PATH,
            LayeredGraphLayerKind::Repository,
            LayeredBackfillLayerReason::RepositorySource,
        ),
        LayeredBackfillSourceKind::KnowledgeGraph => (
            KNOWLEDGE_GRAPH_LAYER_PATH,
            LayeredGraphLayerKind::KnowledgeGraph,
            LayeredBackfillLayerReason::KnowledgeGraphSource,
        ),
        LayeredBackfillSourceKind::Ambiguous => {
            if node
                .catalog_path
                .iter()
                .any(|part| part == "KnowledgeGraphs" || part == "knowledge-graph")
            {
                (
                    KNOWLEDGE_GRAPH_LAYER_PATH,
                    LayeredGraphLayerKind::KnowledgeGraph,
                    LayeredBackfillLayerReason::KnowledgeGraphSource,
                )
            } else if node
                .catalog_path
                .iter()
                .any(|part| part == "crates" || part == "tools" || part == "docs")
            {
                (
                    REPOSITORY_LAYER_PATH,
                    LayeredGraphLayerKind::Repository,
                    LayeredBackfillLayerReason::RepositorySource,
                )
            } else {
                (
                    ROOT_LAYER_PATH,
                    LayeredGraphLayerKind::Root,
                    LayeredBackfillLayerReason::RootFallback,
                )
            }
        }
    }
}

struct LayerSpec<'a> {
    layer_path: &'a str,
    layer_depth: u32,
    layer_kind: LayeredGraphLayerKind,
    graph_style: MemoryGraphStyle,
    root_memory_id: Hash256,
    parent_layer_id: Option<Hash256>,
    parent_graph_node_id: Option<Hash256>,
}

fn scoped_layer_key(input: &LayeredBackfillInput, layer_path: &str) -> (String, String, String) {
    (
        input.tenant_id.clone(),
        input.namespace.clone(),
        layer_path.to_owned(),
    )
}

fn ensure_layer(
    input: &LayeredBackfillInput,
    layers_by_path: &mut BTreeMap<(String, String, String), LayeredGraphLayer>,
    proposed_layers: &mut Vec<LayeredGraphLayer>,
    spec: LayerSpec<'_>,
) -> Result<LayeredGraphLayer, LayeredBackfillError> {
    validate_layer_path(spec.layer_path)?;
    if let Some(layer) = layers_by_path.get(&scoped_layer_key(input, spec.layer_path)) {
        return Ok(layer.clone());
    }
    let layer_id = stable_id(
        BACKFILL_LAYER_ID_DOMAIN,
        &[
            &input.tenant_id,
            &input.namespace,
            spec.layer_kind.as_str(),
            spec.layer_path,
        ],
    )?;
    let layer = LayeredGraphLayer {
        layer_id,
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        root_memory_id: spec.root_memory_id,
        parent_layer_id: spec.parent_layer_id,
        parent_graph_node_id: spec.parent_graph_node_id,
        layer_depth: spec.layer_depth,
        layer_kind: spec.layer_kind,
        graph_style: spec.graph_style,
        layer_path: spec.layer_path.to_owned(),
        metadata: serde_json::json!({
            "source": "layered_backfill_prd05",
            "backfill_status": "planned"
        }),
        created_at_physical_ms: 0,
        created_at_logical: 0,
        updated_at_physical_ms: 0,
        updated_at_logical: 0,
    };
    layers_by_path.insert(scoped_layer_key(input, spec.layer_path), layer.clone());
    proposed_layers.push(layer.clone());
    Ok(layer)
}

struct MembershipSpec {
    layer_id: Hash256,
    graph_node_id: Hash256,
    graph_style: MemoryGraphStyle,
    membership_role: LayeredGraphMembershipRole,
    local_node_rank: u32,
    mapping_reason: LayeredBackfillLayerReason,
}

fn ensure_membership(
    input: &LayeredBackfillInput,
    memberships_by_pair: &mut BTreeSet<(String, String, Hash256, Hash256)>,
    proposed_memberships: &mut Vec<LayeredGraphMembership>,
    spec: MembershipSpec,
) -> Result<(), LayeredBackfillError> {
    if !memberships_by_pair.insert((
        input.tenant_id.clone(),
        input.namespace.clone(),
        spec.layer_id,
        spec.graph_node_id,
    )) {
        return Ok(());
    }
    proposed_memberships.push(LayeredGraphMembership {
        layer_membership_id: stable_id(
            BACKFILL_MEMBERSHIP_ID_DOMAIN,
            &[
                &input.tenant_id,
                &input.namespace,
                &spec.layer_id.to_string(),
                &spec.graph_node_id.to_string(),
            ],
        )?,
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        layer_id: spec.layer_id,
        graph_node_id: spec.graph_node_id,
        graph_style: spec.graph_style,
        membership_role: spec.membership_role,
        local_node_rank: spec.local_node_rank,
        metadata: serde_json::json!({
            "source": "layered_backfill_prd05",
            "mapping_reason": spec.mapping_reason.as_str()
        }),
        created_at_physical_ms: 0,
        created_at_logical: 0,
        updated_at_physical_ms: 0,
        updated_at_logical: 0,
    });
    Ok(())
}

struct LayerEdgeSpec {
    from_layer_id: Hash256,
    to_layer_id: Hash256,
    edge_kind: LayeredGraphLayerEdgeKind,
    graph_style: MemoryGraphStyle,
}

fn ensure_layer_edge(
    input: &LayeredBackfillInput,
    layer_edges_by_tuple: &mut BTreeSet<(
        String,
        String,
        MemoryGraphStyle,
        Hash256,
        Hash256,
        LayeredGraphLayerEdgeKind,
    )>,
    proposed_layer_edges: &mut Vec<LayeredGraphLayerEdge>,
    spec: LayerEdgeSpec,
) -> Result<(), LayeredBackfillError> {
    if !layer_edges_by_tuple.insert((
        input.tenant_id.clone(),
        input.namespace.clone(),
        spec.graph_style,
        spec.from_layer_id,
        spec.to_layer_id,
        spec.edge_kind,
    )) {
        return Ok(());
    }
    proposed_layer_edges.push(LayeredGraphLayerEdge {
        layer_edge_id: stable_id(
            BACKFILL_LAYER_EDGE_ID_DOMAIN,
            &[
                &input.tenant_id,
                &input.namespace,
                spec.graph_style.stable_label(),
                &spec.from_layer_id.to_string(),
                &spec.to_layer_id.to_string(),
                spec.edge_kind.as_str(),
            ],
        )?,
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        graph_style: spec.graph_style,
        from_layer_id: spec.from_layer_id,
        to_layer_id: spec.to_layer_id,
        edge_kind: spec.edge_kind,
        receipt_hash: None,
        metadata: serde_json::json!({
            "source": "layered_backfill_prd05",
            "hygiene_state": "active"
        }),
        created_at_physical_ms: 0,
        created_at_logical: 0,
        updated_at_physical_ms: 0,
        updated_at_logical: 0,
    });
    Ok(())
}

#[derive(Default)]
struct ApplyCounts {
    layers: u32,
    memberships: u32,
    layer_edges: u32,
}

fn apply_plan_rows(
    layers: &mut Vec<LayeredGraphLayer>,
    memberships: &mut Vec<LayeredGraphMembership>,
    layer_edges: &mut Vec<LayeredGraphLayerEdge>,
    plan: &LayeredBackfillPlan,
) -> Result<ApplyCounts, LayeredBackfillError> {
    let mut counts = ApplyCounts::default();
    let mut layer_paths = layers
        .iter()
        .map(|layer| {
            (
                layer.tenant_id.clone(),
                layer.namespace.clone(),
                layer.layer_path.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    for layer in &plan.proposed_layers {
        if layer_paths.insert((
            layer.tenant_id.clone(),
            layer.namespace.clone(),
            layer.layer_path.clone(),
        )) {
            layers.push(layer.clone());
            counts.layers = counts.layers.saturating_add(1);
        }
    }
    let mut membership_pairs = memberships
        .iter()
        .map(|membership| {
            (
                membership.tenant_id.clone(),
                membership.namespace.clone(),
                membership.layer_id,
                membership.graph_node_id,
            )
        })
        .collect::<BTreeSet<_>>();
    for membership in &plan.proposed_memberships {
        if membership_pairs.insert((
            membership.tenant_id.clone(),
            membership.namespace.clone(),
            membership.layer_id,
            membership.graph_node_id,
        )) {
            memberships.push(membership.clone());
            counts.memberships = counts.memberships.saturating_add(1);
        }
    }
    let mut edge_tuples = layer_edges
        .iter()
        .map(|edge| {
            (
                edge.tenant_id.clone(),
                edge.namespace.clone(),
                edge.graph_style,
                edge.from_layer_id,
                edge.to_layer_id,
                edge.edge_kind,
            )
        })
        .collect::<BTreeSet<_>>();
    for edge in &plan.proposed_layer_edges {
        if edge_tuples.insert((
            edge.tenant_id.clone(),
            edge.namespace.clone(),
            edge.graph_style,
            edge.from_layer_id,
            edge.to_layer_id,
            edge.edge_kind,
        )) {
            layer_edges.push(edge.clone());
            counts.layer_edges = counts.layer_edges.saturating_add(1);
        }
    }
    Ok(counts)
}

fn validate_flat_edges(
    input: &LayeredBackfillInput,
    nodes: &[LayeredBackfillGraphNode],
    rejected_records: &mut Vec<LayeredBackfillRejectedRecord>,
) {
    let node_ids = nodes
        .iter()
        .map(|node| node.graph_node_id)
        .collect::<BTreeSet<_>>();
    for edge in &input.flat_graph_edges {
        if !node_ids.contains(&edge.from_graph_node_id)
            || !node_ids.contains(&edge.to_graph_node_id)
        {
            rejected_records.push(LayeredBackfillRejectedRecord {
                record_kind: "graph_edge".to_owned(),
                record_id: edge.graph_edge_id.to_string(),
                reason: "missing_graph_node".to_owned(),
            });
        }
    }
}

fn rank_for_layer(
    graph_node_id: &Hash256,
    nodes: &[LayeredBackfillGraphNode],
    layer_path: &str,
) -> Result<u32, LayeredBackfillError> {
    let mut ids = nodes
        .iter()
        .filter(|node| {
            let (candidate_path, _, _) = selected_layer_for_node(node);
            candidate_path == layer_path || layer_path == ROOT_LAYER_PATH
        })
        .map(|node| node.graph_node_id)
        .collect::<Vec<_>>();
    ids.sort();
    let rank = ids
        .iter()
        .position(|id| id == graph_node_id)
        .unwrap_or(ids.len());
    u32_from_usize(rank)
}

fn counts_for(
    flat_graph_node_count: usize,
    flat_graph_edge_count: usize,
    layer_count: usize,
    membership_count: usize,
    layer_edge_count: usize,
) -> Result<LayeredBackfillCounts, LayeredBackfillError> {
    Ok(LayeredBackfillCounts {
        flat_graph_node_count: u32_from_usize(flat_graph_node_count)?,
        flat_graph_edge_count: u32_from_usize(flat_graph_edge_count)?,
        layer_count: u32_from_usize(layer_count)?,
        membership_count: u32_from_usize(membership_count)?,
        layer_edge_count: u32_from_usize(layer_edge_count)?,
    })
}

fn u32_from_usize(value: usize) -> Result<u32, LayeredBackfillError> {
    u32::try_from(value).map_err(|_| LayeredBackfillError::InvalidScope {
        reason: "count_out_of_range".to_owned(),
    })
}

fn stable_id(domain: &str, parts: &[&str]) -> Result<Hash256, LayeredBackfillError> {
    hash_event_body(&(domain, parts)).map_err(|error| LayeredBackfillError::HashMaterial {
        reason: error.to_string(),
    })
}

fn validate_layer_path(path: &str) -> Result<(), LayeredBackfillError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('~')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains("//")
    {
        return Err(LayeredBackfillError::UnsafeMaterial {
            field: "layer_path",
        });
    }
    for part in path.split('/') {
        validate_slug(part, "layer_path")?;
    }
    Ok(())
}

fn validate_slug(value: &str, field: &'static str) -> Result<(), LayeredBackfillError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.starts_with('~')
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
        || value != value.trim()
    {
        return Err(LayeredBackfillError::UnsafeMaterial { field });
    }
    Ok(())
}

fn reject_forbidden_json(
    field: &'static str,
    value: &serde_json::Value,
) -> Result<(), LayeredBackfillError> {
    let rendered = value.to_string().to_ascii_lowercase();
    for forbidden in [
        "raw_private_payload",
        "raw_model_output",
        "raw_markdown",
        "raw_body",
        "source_excerpt",
        "full_output",
        "postgres://",
        "postgresql://",
        "file://",
        "/users/",
        "/volumes/",
    ] {
        if rendered.contains(forbidden) {
            return Err(LayeredBackfillError::UnsafeMaterial { field });
        }
    }
    Ok(())
}

fn layered_backfill_non_claims() -> Vec<String> {
    vec![
        "repository_test_scope_only".to_owned(),
        "production_migration_not_approved".to_owned(),
        "flat_graph_data_not_dropped".to_owned(),
        "layered_final_acceptance_not_claimed".to_owned(),
        "operator_evidence_not_fabricated".to_owned(),
    ]
}

trait MemoryGraphStyleLabel {
    fn stable_label(self) -> &'static str;
}

impl MemoryGraphStyleLabel for MemoryGraphStyle {
    fn stable_label(self) -> &'static str {
        match self {
            MemoryGraphStyle::ProvenanceReceiptDag => "provenance_receipt_dag",
            MemoryGraphStyle::CanonicalMemoryGraph => "canonical_memory_graph",
            MemoryGraphStyle::SemanticCatalogGraph => "semantic_catalog_graph",
            MemoryGraphStyle::SimilarityOverlayGraph => "similarity_overlay_graph",
            MemoryGraphStyle::DependencyDag => "dependency_dag",
            MemoryGraphStyle::RoutingViewGraph => "routing_view_graph",
            MemoryGraphStyle::ContradictionSupersessionGraph => "contradiction_supersession_graph",
            MemoryGraphStyle::ContextPacketGraph => "context_packet_graph",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layered_backfill_plans_root_and_child_layers_deterministically() {
        let input = fixture_input();
        let first = plan_layered_backfill(&input).expect("plan");
        let second = plan_layered_backfill(&input).expect("plan again");

        assert_eq!(first, second);
        assert_eq!(first.schema_version, LAYERED_BACKFILL_PLAN_SCHEMA_VERSION);
        assert_eq!(first.proposed_layers.len(), 3);
        assert_eq!(first.proposed_memberships.len(), 6);
        assert_eq!(first.proposed_layer_edges.len(), 2);
        assert_eq!(first.after_counts.layer_count, 3);
        assert_eq!(first.after_counts.membership_count, 6);
        assert!(
            first
                .proposed_memberships
                .iter()
                .any(|membership| membership.membership_role == LayeredGraphMembershipRole::Root)
        );
        assert_eq!(first.after_counts.layer_edge_count, 2);
        assert_eq!(first.flat_fallback_count, 1);
        assert!(
            first
                .proposed_layer_edges
                .iter()
                .all(|edge| edge.metadata["hygiene_state"] == "active")
        );
        assert!(
            first
                .node_mappings
                .iter()
                .any(|mapping| mapping.layer_path == REPOSITORY_LAYER_PATH)
        );
        assert!(
            first
                .node_mappings
                .iter()
                .any(|mapping| mapping.layer_path == KNOWLEDGE_GRAPH_LAYER_PATH)
        );
    }

    #[test]
    fn layered_backfill_fixture_execution_is_idempotent_and_preserves_flat_counts() {
        let input = fixture_input();
        let plan = plan_layered_backfill(&input).expect("plan");
        let report = execute_layered_backfill_fixture(&input, &plan).expect("execute");

        assert_eq!(
            report.schema_version,
            LAYERED_BACKFILL_EXECUTION_SCHEMA_VERSION
        );
        assert_eq!(report.execution_status, "passed");
        assert!(report.idempotent_replay);
        assert_eq!(report.before_counts.flat_graph_node_count, 4);
        assert_eq!(report.after_counts.flat_graph_node_count, 4);
        assert_eq!(report.replay_counts, report.after_counts);
        assert_eq!(report.inserted_layer_count, 3);
        assert_eq!(report.replay_inserted_layer_count, 0);
        assert!(report.failed_invariants.is_empty());
    }

    #[test]
    fn layered_backfill_reuses_existing_rows_on_replay_plan() {
        let input = fixture_input();
        let plan = plan_layered_backfill(&input).expect("plan");
        let replay_input = LayeredBackfillInput {
            existing_layers: plan.proposed_layers.clone(),
            existing_memberships: plan.proposed_memberships.clone(),
            existing_layer_edges: plan.proposed_layer_edges.clone(),
            ..input
        };
        let replay_plan = plan_layered_backfill(&replay_input).expect("replay plan");

        assert!(replay_plan.proposed_layers.is_empty());
        assert!(replay_plan.proposed_memberships.is_empty());
        assert!(replay_plan.proposed_layer_edges.is_empty());
        assert_eq!(replay_plan.before_counts.layer_count, 3);
        assert_eq!(replay_plan.after_counts.layer_count, 3);
    }

    #[test]
    fn layered_backfill_fails_closed_on_scope_mismatch_and_unsafe_metadata() {
        let mut input = fixture_input();
        input.flat_graph_nodes[0].tenant_id = "other".to_owned();
        assert!(matches!(
            plan_layered_backfill(&input),
            Err(LayeredBackfillError::InvalidScope { .. })
        ));

        let mut input = fixture_input();
        input.flat_graph_nodes[0].metadata =
            serde_json::json!({"source_excerpt": "do not persist"});
        assert!(matches!(
            plan_layered_backfill(&input),
            Err(LayeredBackfillError::UnsafeMaterial { .. })
        ));
    }

    #[test]
    fn layered_backfill_rejects_foreign_scope_existing_rows() {
        let plan = plan_layered_backfill(&fixture_input()).expect("plan");

        let mut foreign_layer_input = fixture_input();
        let mut foreign_layer = plan.proposed_layers[0].clone();
        foreign_layer.tenant_id = "tenant-other".to_owned();
        foreign_layer_input.existing_layers = vec![foreign_layer];
        assert!(matches!(
            plan_layered_backfill(&foreign_layer_input),
            Err(LayeredBackfillError::InvalidScope { reason })
                if reason == "existing_layer_scope_mismatch"
        ));

        let mut foreign_membership_input = fixture_input();
        let mut foreign_membership = plan.proposed_memberships[0].clone();
        foreign_membership.namespace = "other-namespace".to_owned();
        foreign_membership_input.existing_memberships = vec![foreign_membership];
        assert!(matches!(
            plan_layered_backfill(&foreign_membership_input),
            Err(LayeredBackfillError::InvalidScope { reason })
                if reason == "existing_membership_scope_mismatch"
        ));

        let mut foreign_edge_input = fixture_input();
        let mut foreign_edge = plan.proposed_layer_edges[0].clone();
        foreign_edge.tenant_id = "tenant-other".to_owned();
        foreign_edge_input.existing_layer_edges = vec![foreign_edge];
        assert!(matches!(
            plan_layered_backfill(&foreign_edge_input),
            Err(LayeredBackfillError::InvalidScope { reason })
                if reason == "existing_layer_edge_scope_mismatch"
        ));
    }

    #[test]
    fn layered_backfill_reports_missing_edge_nodes_without_mutating_flat_edges() {
        let mut input = fixture_input();
        input.flat_graph_edges[0].to_graph_node_id = h(0xfe);
        let plan = plan_layered_backfill(&input).expect("plan with rejected edge");

        assert_eq!(plan.before_counts.flat_graph_edge_count, 2);
        assert_eq!(plan.after_counts.flat_graph_edge_count, 2);
        assert_eq!(plan.rejected_records.len(), 1);
        assert_eq!(plan.rejected_records[0].reason, "missing_graph_node");
    }

    #[test]
    fn layered_compatibility_report_rejects_stale_evidence_overclaims() {
        let report = build_layered_backfill_compatibility_report(
            vec![
                surface(
                    "flat_retrieval",
                    "cargo test -p exochain-dag-db-retrieval kg_retrieval",
                    "passed",
                ),
                surface(
                    "context_packets",
                    "cargo test -p exochain-dag-db-retrieval context_packet_output",
                    "passed",
                ),
            ],
            vec![LayeredStaleEvidenceItem {
                artifact_ref: "agent-brain/prd06-live-semantic-benchmark".to_owned(),
                status: "historical_requires_layered_rerun".to_owned(),
                reason: "flat evidence cannot prove layered superiority".to_owned(),
            }],
        );

        assert_eq!(
            report.schema_version,
            LAYERED_BACKFILL_COMPATIBILITY_SCHEMA_VERSION
        );
        assert_eq!(report.compatibility_status, "passed");
        assert!(
            report
                .rejected_overclaims
                .contains(&"old_flat_evidence_as_layered_evidence".to_owned())
        );
    }

    fn surface(surface: &str, proof_ref: &str, status: &str) -> LayeredCompatibilitySurface {
        LayeredCompatibilitySurface {
            surface: surface.to_owned(),
            proof_ref: proof_ref.to_owned(),
            status: status.to_owned(),
        }
    }

    fn fixture_input() -> LayeredBackfillInput {
        let tenant_id = "dag_db-local".to_owned();
        let namespace = "dag_db".to_owned();
        LayeredBackfillInput {
            tenant_id: tenant_id.clone(),
            namespace: namespace.clone(),
            root_memory_id: h(0x01),
            flat_graph_nodes: vec![
                node(
                    0x10,
                    0x20,
                    &tenant_id,
                    &namespace,
                    MemoryGraphStyle::CanonicalMemoryGraph,
                    vec!["docs", "dagdb"],
                    LayeredBackfillSourceKind::Repository,
                ),
                node(
                    0x11,
                    0x21,
                    &tenant_id,
                    &namespace,
                    MemoryGraphStyle::SemanticCatalogGraph,
                    vec!["KnowledgeGraphs", "dag-db"],
                    LayeredBackfillSourceKind::KnowledgeGraph,
                ),
                node(
                    0x12,
                    0x22,
                    &tenant_id,
                    &namespace,
                    MemoryGraphStyle::CanonicalMemoryGraph,
                    vec!["misc"],
                    LayeredBackfillSourceKind::Ambiguous,
                ),
                node(
                    0x13,
                    0x23,
                    &tenant_id,
                    &namespace,
                    MemoryGraphStyle::CanonicalMemoryGraph,
                    vec!["crates", "exo-dag-db-retrieval"],
                    LayeredBackfillSourceKind::Ambiguous,
                ),
            ],
            flat_graph_edges: vec![
                edge(0x30, 0x10, 0x11, &tenant_id, &namespace),
                edge(0x31, 0x11, 0x12, &tenant_id, &namespace),
            ],
            existing_layers: Vec::new(),
            existing_memberships: Vec::new(),
            existing_layer_edges: Vec::new(),
        }
    }

    fn node(
        graph_node_byte: u8,
        memory_byte: u8,
        tenant_id: &str,
        namespace: &str,
        graph_style: MemoryGraphStyle,
        catalog_path: Vec<&str>,
        source_kind: LayeredBackfillSourceKind,
    ) -> LayeredBackfillGraphNode {
        LayeredBackfillGraphNode {
            graph_node_id: h(graph_node_byte),
            memory_id: h(memory_byte),
            tenant_id: tenant_id.to_owned(),
            namespace: namespace.to_owned(),
            graph_style,
            node_kind: "canonical".to_owned(),
            catalog_path: catalog_path.into_iter().map(str::to_owned).collect(),
            source_kind,
            metadata: serde_json::json!({"safe": true}),
        }
    }

    fn edge(
        edge_byte: u8,
        from_node_byte: u8,
        to_node_byte: u8,
        tenant_id: &str,
        namespace: &str,
    ) -> LayeredBackfillGraphEdge {
        LayeredBackfillGraphEdge {
            graph_edge_id: h(edge_byte),
            tenant_id: tenant_id.to_owned(),
            namespace: namespace.to_owned(),
            graph_style: MemoryGraphStyle::CanonicalMemoryGraph,
            from_graph_node_id: h(from_node_byte),
            to_graph_node_id: h(to_node_byte),
            edge_kind: "related_to".to_owned(),
        }
    }

    const fn h(byte: u8) -> Hash256 {
        Hash256([byte; 32])
    }
}
