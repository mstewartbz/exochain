//! Pure layer placement contracts for additive layered graph routing.

use exo_core::Hash256;
use exo_dag_db_api::MemoryGraphStyle;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Visible root path used when placement cannot safely choose a child layer.
pub const LAYER_PLACEMENT_ROOT_PATH: &str = "root";
/// Deterministic repository layer path under the visible root.
pub const LAYER_PLACEMENT_REPOSITORY_PATH: &str = "root/repository";
/// Deterministic knowledge-graph layer path under the visible root.
pub const LAYER_PLACEMENT_KNOWLEDGE_GRAPH_PATH: &str = "root/knowledge-graph";
/// Maximum child layer depth accepted by PRD02 placement proof.
pub const LAYER_PLACEMENT_MAX_DEPTH: u32 = 3;

const LAYER_PLACEMENT_SCHEMA_VERSION: u16 = 1;
const CHILD_LAYER_ID_DOMAIN: &str = "exo.dagdb.layered_placement.child_layer_id";
const LAYER_MEMBERSHIP_ID_DOMAIN: &str = "exo.dagdb.layered_placement.layer_membership_id";
const LAYER_EDGE_ID_DOMAIN: &str = "exo.dagdb.layered_placement.layer_edge_id";

/// Source category used by pure layer placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerPlacementSourceKind {
    /// Repository-backed memory belongs in the repository child layer.
    Repository,
    /// Knowledge-graph import memory belongs in the knowledge-graph child layer.
    KnowledgeGraph,
    /// Memory with an explicit parent layer belongs under that parent path.
    ParentChild,
    /// Ambiguous memory remains visible at root rather than being hidden.
    Ambiguous,
}

impl LayerPlacementSourceKind {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::KnowledgeGraph => "knowledge_graph",
            Self::ParentChild => "parent_child",
            Self::Ambiguous => "ambiguous",
        }
    }
}

/// Pure request for selecting a target layered-graph destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerPlacementRequest {
    /// Tenant scope for the placement decision.
    pub tenant_id: String,
    /// Namespace scope for the placement decision.
    pub namespace: String,
    /// Existing graph style hosted by the selected layer.
    pub graph_style: MemoryGraphStyle,
    /// Caller-classified source category.
    pub source_kind: LayerPlacementSourceKind,
    /// Parent layer when selecting a child layer.
    pub parent_layer_id: Option<Hash256>,
    /// Parent layer path when selecting a child layer.
    pub parent_layer_path: Option<String>,
    /// Parent layer depth when selecting a child layer.
    pub parent_layer_depth: Option<u32>,
    /// Existing parent graph node that owns the child layer.
    pub parent_graph_node_id: Option<Hash256>,
    /// Child layer path segment for parent-child placement.
    pub child_layer_slug: Option<String>,
}

/// Selected layer fields returned by pure placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerPlacementSelection {
    /// Stable target layer path.
    pub target_layer_path: String,
    /// Root is depth zero; child layers are positive depth.
    pub target_layer_depth: u32,
    /// Stable machine-readable reason for the selected layer.
    pub target_layer_reason: String,
    /// True when ambiguous input fell back to the visible root.
    pub layer_fallback_used: bool,
    /// Parent layer to bind if a child layer is created.
    pub parent_layer_id: Option<Hash256>,
    /// Parent graph node to bind if a child layer is created.
    pub parent_graph_node_id: Option<Hash256>,
    /// Deterministic ID for the child layer that should exist or be created.
    pub created_child_layer_id: Option<Hash256>,
}

/// Errors raised before any persistence layer can mutate state.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LayerPlacementError {
    /// Tenant and namespace must both be present.
    #[error("invalid_tenant_namespace")]
    InvalidTenantNamespace {
        /// Supplied tenant ID.
        tenant_id: String,
        /// Supplied namespace.
        namespace: String,
    },
    /// Parent-child placement is missing required parent context.
    #[error("missing_parent_child_field: {field}")]
    MissingParentChildField {
        /// Missing field name.
        field: &'static str,
    },
    /// Layer path input is not safe relative graph path material.
    #[error("invalid_layer_path: {field}")]
    InvalidLayerPath {
        /// Invalid field name.
        field: &'static str,
    },
    /// Child layer creation exceeded the bounded depth budget.
    #[error("layer_depth_exceeded: {target_layer_depth}>{max_layer_depth}")]
    LayerDepthExceeded {
        /// Requested target depth.
        target_layer_depth: u32,
        /// Maximum accepted depth.
        max_layer_depth: u32,
    },
    /// Child layer ID hash material could not be serialized.
    #[error("layer_placement_hash_material_failed: {reason}")]
    HashMaterial {
        /// Stable serialization reason.
        reason: String,
    },
}

/// Select target layer fields without writing to the database.
pub fn select_target_layer(
    request: &LayerPlacementRequest,
) -> Result<LayerPlacementSelection, LayerPlacementError> {
    validate_scope(&request.tenant_id, &request.namespace)?;
    match request.source_kind {
        LayerPlacementSourceKind::Repository => child_selection(
            request,
            LAYER_PLACEMENT_REPOSITORY_PATH,
            1,
            "repository_source_root_repository",
        ),
        LayerPlacementSourceKind::KnowledgeGraph => child_selection(
            request,
            LAYER_PLACEMENT_KNOWLEDGE_GRAPH_PATH,
            1,
            "knowledge_graph_source_root_knowledge_graph",
        ),
        LayerPlacementSourceKind::ParentChild => parent_child_selection(request),
        LayerPlacementSourceKind::Ambiguous => Ok(LayerPlacementSelection {
            target_layer_path: LAYER_PLACEMENT_ROOT_PATH.to_owned(),
            target_layer_depth: 0,
            target_layer_reason: "ambiguous_source_visible_root_fallback".to_owned(),
            layer_fallback_used: true,
            parent_layer_id: None,
            parent_graph_node_id: None,
            created_child_layer_id: None,
        }),
    }
}

fn parent_child_selection(
    request: &LayerPlacementRequest,
) -> Result<LayerPlacementSelection, LayerPlacementError> {
    let parent_path =
        required_relative_path(request.parent_layer_path.as_deref(), "parent_layer_path")?;
    let child_slug = required_slug(request.child_layer_slug.as_deref(), "child_layer_slug")?;
    let parent_depth =
        request
            .parent_layer_depth
            .ok_or(LayerPlacementError::MissingParentChildField {
                field: "parent_layer_depth",
            })?;
    let target_layer_depth =
        parent_depth
            .checked_add(1)
            .ok_or(LayerPlacementError::LayerDepthExceeded {
                target_layer_depth: u32::MAX,
                max_layer_depth: LAYER_PLACEMENT_MAX_DEPTH,
            })?;
    let target_layer_path = format!("{parent_path}/{child_slug}");
    child_selection(
        request,
        &target_layer_path,
        target_layer_depth,
        "parent_child_source_parent_path_child",
    )
}

fn child_selection(
    request: &LayerPlacementRequest,
    target_layer_path: &str,
    target_layer_depth: u32,
    target_layer_reason: &str,
) -> Result<LayerPlacementSelection, LayerPlacementError> {
    if target_layer_depth > LAYER_PLACEMENT_MAX_DEPTH {
        return Err(LayerPlacementError::LayerDepthExceeded {
            target_layer_depth,
            max_layer_depth: LAYER_PLACEMENT_MAX_DEPTH,
        });
    }
    let parent_layer_id =
        request
            .parent_layer_id
            .ok_or(LayerPlacementError::MissingParentChildField {
                field: "parent_layer_id",
            })?;
    let parent_graph_node_id =
        request
            .parent_graph_node_id
            .ok_or(LayerPlacementError::MissingParentChildField {
                field: "parent_graph_node_id",
            })?;
    validate_relative_path(target_layer_path, "target_layer_path")?;
    Ok(LayerPlacementSelection {
        target_layer_path: target_layer_path.to_owned(),
        target_layer_depth,
        target_layer_reason: target_layer_reason.to_owned(),
        layer_fallback_used: false,
        parent_layer_id: Some(parent_layer_id),
        parent_graph_node_id: Some(parent_graph_node_id),
        created_child_layer_id: Some(child_layer_id(
            request,
            target_layer_path,
            target_layer_depth,
        )?),
    })
}

fn validate_scope(tenant_id: &str, namespace: &str) -> Result<(), LayerPlacementError> {
    if tenant_id.trim().is_empty() || namespace.trim().is_empty() {
        return Err(LayerPlacementError::InvalidTenantNamespace {
            tenant_id: tenant_id.to_owned(),
            namespace: namespace.to_owned(),
        });
    }
    Ok(())
}

fn required_relative_path<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, LayerPlacementError> {
    let value = required_raw(value, field)?;
    validate_relative_path(value, field)?;
    Ok(value)
}

fn required_slug<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, LayerPlacementError> {
    let value = required_raw(value, field)?;
    validate_slug(value, field)?;
    Ok(value)
}

fn required_raw<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, LayerPlacementError> {
    let value = value.ok_or(LayerPlacementError::MissingParentChildField { field })?;
    if value.is_empty() || value.trim().is_empty() {
        return Err(LayerPlacementError::MissingParentChildField { field });
    }
    if value != value.trim() {
        return Err(LayerPlacementError::InvalidLayerPath { field });
    }
    Ok(value)
}

fn validate_relative_path(value: &str, field: &'static str) -> Result<(), LayerPlacementError> {
    if value.starts_with('/') || value.starts_with('~') || value.ends_with('/') {
        return Err(LayerPlacementError::InvalidLayerPath { field });
    }
    if value.contains('\\') || value.contains("//") {
        return Err(LayerPlacementError::InvalidLayerPath { field });
    }
    for part in value.split('/') {
        validate_slug(part, field)?;
    }
    Ok(())
}

fn validate_slug(value: &str, field: &'static str) -> Result<(), LayerPlacementError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.starts_with('~')
        || value != value.trim()
    {
        return Err(LayerPlacementError::InvalidLayerPath { field });
    }
    Ok(())
}

fn child_layer_id(
    request: &LayerPlacementRequest,
    target_layer_path: &str,
    target_layer_depth: u32,
) -> Result<Hash256, LayerPlacementError> {
    deterministic_layer_id(
        &request.tenant_id,
        &request.namespace,
        request.graph_style,
        target_layer_path,
        target_layer_depth,
    )
}

/// Deterministic scoped layer identity bound to tenant, namespace, style, path, and depth.
pub fn deterministic_layer_id(
    tenant_id: &str,
    namespace: &str,
    graph_style: MemoryGraphStyle,
    layer_path: &str,
    layer_depth: u32,
) -> Result<Hash256, LayerPlacementError> {
    digest_id_material(&(
        CHILD_LAYER_ID_DOMAIN,
        LAYER_PLACEMENT_SCHEMA_VERSION,
        tenant_id,
        namespace,
        graph_style,
        layer_path,
        layer_depth,
    ))
}

/// Deterministic scoped layer-membership identity bound to its layer/node pair.
pub fn deterministic_layer_membership_id(
    tenant_id: &str,
    namespace: &str,
    layer_id: Hash256,
    graph_node_id: Hash256,
) -> Result<Hash256, LayerPlacementError> {
    digest_id_material(&(
        LAYER_MEMBERSHIP_ID_DOMAIN,
        LAYER_PLACEMENT_SCHEMA_VERSION,
        tenant_id,
        namespace,
        layer_id,
        graph_node_id,
    ))
}

/// Deterministic scoped layer-edge identity bound to its unique edge tuple.
pub fn deterministic_layer_edge_id(
    tenant_id: &str,
    namespace: &str,
    graph_style: MemoryGraphStyle,
    from_layer_id: Hash256,
    to_layer_id: Hash256,
    edge_kind: &str,
) -> Result<Hash256, LayerPlacementError> {
    digest_id_material(&(
        LAYER_EDGE_ID_DOMAIN,
        LAYER_PLACEMENT_SCHEMA_VERSION,
        tenant_id,
        namespace,
        graph_style,
        from_layer_id,
        to_layer_id,
        edge_kind,
    ))
}

fn digest_id_material<T: serde::Serialize>(material: &T) -> Result<Hash256, LayerPlacementError> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(material, &mut buf).map_err(|error| {
        LayerPlacementError::HashMaterial {
            reason: error.to_string(),
        }
    })?;
    Ok(Hash256::digest(&buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layered_placement_repository_source_targets_root_repository() {
        let selection = select_target_layer(&request(LayerPlacementSourceKind::Repository))
            .expect("repository placement succeeds");

        assert_eq!(selection.target_layer_path, LAYER_PLACEMENT_REPOSITORY_PATH);
        assert_eq!(selection.target_layer_depth, 1);
        assert_eq!(
            selection.target_layer_reason,
            "repository_source_root_repository"
        );
        assert!(!selection.layer_fallback_used);
        assert_eq!(selection.parent_layer_id, Some(h(0x11)));
        assert_eq!(selection.parent_graph_node_id, Some(h(0x21)));
        assert!(selection.created_child_layer_id.is_some());
    }

    #[test]
    fn layered_placement_kg_source_targets_root_knowledge_graph() {
        let selection = select_target_layer(&request(LayerPlacementSourceKind::KnowledgeGraph))
            .expect("knowledge graph placement succeeds");

        assert_eq!(
            selection.target_layer_path,
            LAYER_PLACEMENT_KNOWLEDGE_GRAPH_PATH
        );
        assert_eq!(selection.target_layer_depth, 1);
        assert_eq!(
            selection.target_layer_reason,
            "knowledge_graph_source_root_knowledge_graph"
        );
        assert!(!selection.layer_fallback_used);
        assert!(selection.created_child_layer_id.is_some());
    }

    #[test]
    fn layered_placement_parent_child_source_targets_parent_path_child() {
        let mut request = request(LayerPlacementSourceKind::ParentChild);
        request.parent_layer_path = Some("root/repository".to_owned());
        request.parent_layer_depth = Some(1);
        request.child_layer_slug = Some("source-file".to_owned());

        let selection = select_target_layer(&request).expect("parent child placement succeeds");

        assert_eq!(selection.target_layer_path, "root/repository/source-file");
        assert_eq!(selection.target_layer_depth, 2);
        assert_eq!(
            selection.target_layer_reason,
            "parent_child_source_parent_path_child"
        );
        assert_eq!(selection.parent_layer_id, Some(h(0x11)));
        assert_eq!(selection.parent_graph_node_id, Some(h(0x21)));
        assert!(selection.created_child_layer_id.is_some());
    }

    #[test]
    fn layered_placement_ambiguous_source_uses_visible_root_fallback() {
        let selection = select_target_layer(&request(LayerPlacementSourceKind::Ambiguous))
            .expect("ambiguous fallback succeeds");

        assert_eq!(selection.target_layer_path, LAYER_PLACEMENT_ROOT_PATH);
        assert_eq!(selection.target_layer_depth, 0);
        assert_eq!(
            selection.target_layer_reason,
            "ambiguous_source_visible_root_fallback"
        );
        assert!(selection.layer_fallback_used);
        assert_eq!(selection.parent_layer_id, None);
        assert_eq!(selection.parent_graph_node_id, None);
        assert_eq!(selection.created_child_layer_id, None);
    }

    #[test]
    fn layered_placement_invalid_tenant_or_namespace_errors() {
        let mut request = request(LayerPlacementSourceKind::Repository);
        request.tenant_id = " ".to_owned();

        assert!(matches!(
            select_target_layer(&request),
            Err(LayerPlacementError::InvalidTenantNamespace { .. })
        ));

        request.tenant_id = "tenant-a".to_owned();
        request.namespace.clear();

        assert!(matches!(
            select_target_layer(&request),
            Err(LayerPlacementError::InvalidTenantNamespace { .. })
        ));
    }

    #[test]
    fn layered_placement_child_layer_ids_are_deterministic() {
        let request = request(LayerPlacementSourceKind::Repository);

        let first = select_target_layer(&request).expect("first placement succeeds");
        let second = select_target_layer(&request).expect("second placement succeeds");

        assert_eq!(first.created_child_layer_id, second.created_child_layer_id);
    }

    #[test]
    fn layered_placement_child_claim_requires_parent_bindings() {
        let mut request = request(LayerPlacementSourceKind::Repository);
        request.parent_layer_id = None;

        assert!(matches!(
            select_target_layer(&request),
            Err(LayerPlacementError::MissingParentChildField { field })
                if field == "parent_layer_id"
        ));

        request.parent_layer_id = Some(h(0x11));
        request.parent_graph_node_id = None;

        assert!(matches!(
            select_target_layer(&request),
            Err(LayerPlacementError::MissingParentChildField { field })
                if field == "parent_graph_node_id"
        ));
    }

    #[test]
    fn layered_placement_parent_child_rejects_over_depth() {
        let mut request = request(LayerPlacementSourceKind::ParentChild);
        request.parent_layer_path = Some("root/repository/depth-two".to_owned());
        request.parent_layer_depth = Some(LAYER_PLACEMENT_MAX_DEPTH);
        request.child_layer_slug = Some("too-deep".to_owned());

        assert!(matches!(
            select_target_layer(&request),
            Err(LayerPlacementError::LayerDepthExceeded {
                target_layer_depth,
                max_layer_depth
            }) if target_layer_depth == LAYER_PLACEMENT_MAX_DEPTH + 1
                && max_layer_depth == LAYER_PLACEMENT_MAX_DEPTH
        ));

        request.parent_layer_depth = Some(u32::MAX);
        assert!(matches!(
            select_target_layer(&request),
            Err(LayerPlacementError::LayerDepthExceeded {
                target_layer_depth: u32::MAX,
                max_layer_depth: LAYER_PLACEMENT_MAX_DEPTH
            })
        ));
    }

    #[test]
    fn layered_placement_rejects_unsafe_paths_without_trimming() {
        for parent_path in [
            "/Users/example/root",
            "root/repository/",
            "root//repository",
            "root/../repository",
            " root/repository",
            "root\\repository",
        ] {
            let mut request = request(LayerPlacementSourceKind::ParentChild);
            request.parent_layer_path = Some(parent_path.to_owned());
            request.parent_layer_depth = Some(1);
            request.child_layer_slug = Some("source-file".to_owned());

            assert!(
                matches!(
                    select_target_layer(&request),
                    Err(LayerPlacementError::InvalidLayerPath {
                        field: "parent_layer_path"
                    })
                ),
                "expected invalid parent path rejection for {parent_path}"
            );
        }

        for child_slug in [
            "../source",
            "source/file",
            " source",
            "~source",
            "source\\file",
        ] {
            let mut request = request(LayerPlacementSourceKind::ParentChild);
            request.parent_layer_path = Some("root/repository".to_owned());
            request.parent_layer_depth = Some(1);
            request.child_layer_slug = Some(child_slug.to_owned());

            assert!(
                matches!(
                    select_target_layer(&request),
                    Err(LayerPlacementError::InvalidLayerPath {
                        field: "child_layer_slug"
                    })
                ),
                "expected invalid child slug rejection for {child_slug}"
            );
        }
    }

    fn request(source_kind: LayerPlacementSourceKind) -> LayerPlacementRequest {
        LayerPlacementRequest {
            tenant_id: "tenant-a".to_owned(),
            namespace: "default".to_owned(),
            graph_style: MemoryGraphStyle::DependencyDag,
            source_kind,
            parent_layer_id: Some(h(0x11)),
            parent_layer_path: None,
            parent_layer_depth: None,
            parent_graph_node_id: Some(h(0x21)),
            child_layer_slug: None,
        }
    }

    const fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }
}
