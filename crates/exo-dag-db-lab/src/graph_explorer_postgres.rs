//! Approved read-only Postgres export for graph explorer artifacts.

#[cfg(not(feature = "postgres"))]
mod non_postgres {
    use std::{collections::BTreeMap, path::Path};

    use exo_dag_db_api::MemoryGraphStyle;

    use crate::{
        graph::{MemoryGraphEdge, MemoryGraphNode},
        graph_explorer::{
            GraphExplorerArtifactSet, GraphExplorerError, GraphExplorerSnapshot,
            LiveGraphExportRequest, validate_live_export_request,
        },
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GraphExplorerPostgresExportRequest<'a> {
        pub env: &'a BTreeMap<String, String>,
        pub tenant_id: Option<&'a str>,
        pub namespace: Option<&'a str>,
        pub active_graph_style: MemoryGraphStyle,
        pub source_commit_or_run_id: Option<&'a str>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GraphExplorerPostgresRows {
        pub nodes: Vec<MemoryGraphNode>,
        pub edges: Vec<MemoryGraphEdge>,
        pub edge_statuses: BTreeMap<String, crate::graph_explorer::GraphExplorerEdgeStatus>,
        pub source_graph_view_ids: Vec<String>,
    }

    pub async fn get_approved_postgres_graph_explorer_snapshot(
        request: &GraphExplorerPostgresExportRequest<'_>,
    ) -> Result<GraphExplorerSnapshot, GraphExplorerError> {
        validate_non_postgres_request(request)?;
        Err(GraphExplorerError::SchemaMismatch)
    }

    pub async fn write_approved_postgres_graph_explorer_artifacts(
        request: &GraphExplorerPostgresExportRequest<'_>,
        _target_dir: &Path,
    ) -> Result<GraphExplorerArtifactSet, GraphExplorerError> {
        validate_non_postgres_request(request)?;
        Err(GraphExplorerError::SchemaMismatch)
    }

    pub async fn read_scoped_postgres_graph(
        _database_url: &str,
        _tenant_id: &str,
        _namespace: &str,
    ) -> Result<GraphExplorerPostgresRows, GraphExplorerError> {
        Err(GraphExplorerError::SchemaMismatch)
    }

    pub fn apply_postgres_provenance(
        _snapshot: &mut GraphExplorerSnapshot,
    ) -> Result<(), GraphExplorerError> {
        Err(GraphExplorerError::SchemaMismatch)
    }

    #[must_use]
    pub fn postgres_source_table_names() -> Vec<String> {
        vec![
            "dagdb_graph_edges".into(),
            "dagdb_graph_edge_tombstones".into(),
            "dagdb_graph_nodes".into(),
            "dagdb_graph_views".into(),
        ]
    }

    #[must_use]
    pub fn postgres_source_columns() -> Vec<String> {
        vec![
            "dagdb_graph_edges.edge_kind".into(),
            "dagdb_graph_edges.from_memory_id".into(),
            "dagdb_graph_edges.graph_edge_id".into(),
            "dagdb_graph_edges.graph_style".into(),
            "dagdb_graph_edges.namespace".into(),
            "dagdb_graph_edges.receipt_hash".into(),
            "dagdb_graph_edges.tenant_id".into(),
            "dagdb_graph_edges.to_memory_id".into(),
            "dagdb_graph_edge_tombstones.namespace".into(),
            "dagdb_graph_edge_tombstones.prior_edge_id".into(),
            "dagdb_graph_edge_tombstones.tenant_id".into(),
            "dagdb_graph_nodes.graph_node_id".into(),
            "dagdb_graph_nodes.graph_style".into(),
            "dagdb_graph_nodes.memory_id".into(),
            "dagdb_graph_nodes.namespace".into(),
            "dagdb_graph_nodes.node_kind".into(),
            "dagdb_graph_nodes.tenant_id".into(),
            "dagdb_graph_views.namespace".into(),
            "dagdb_graph_views.tenant_id".into(),
            "dagdb_graph_views.view_id".into(),
        ]
    }

    fn validate_non_postgres_request(
        request: &GraphExplorerPostgresExportRequest<'_>,
    ) -> Result<(), GraphExplorerError> {
        validate_live_export_request(&LiveGraphExportRequest {
            env: request.env,
            tenant_id: request.tenant_id,
            namespace: request.namespace,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::graph_explorer::{GRAPH_EXPLORER_DATABASE_URL_ENV, LIVE_EXPORT_APPROVAL_ENV};

        #[tokio::test]
        async fn non_postgres_stub_validates_before_schema_error() {
            let env = BTreeMap::new();
            let request = GraphExplorerPostgresExportRequest {
                env: &env,
                tenant_id: Some("tenant-a"),
                namespace: Some("namespace-a"),
                active_graph_style: MemoryGraphStyle::DependencyDag,
                source_commit_or_run_id: None,
            };
            let error = get_approved_postgres_graph_explorer_snapshot(&request)
                .await
                .expect_err("approval is checked before stub fallback");
            assert_eq!(error, GraphExplorerError::LiveExportNotApproved);
        }

        #[tokio::test]
        async fn non_postgres_stub_fails_closed_after_preconditions() {
            let env = BTreeMap::from([
                (LIVE_EXPORT_APPROVAL_ENV.into(), "true".into()),
                (GRAPH_EXPLORER_DATABASE_URL_ENV.into(), "redacted".into()),
            ]);
            let request = GraphExplorerPostgresExportRequest {
                env: &env,
                tenant_id: Some("tenant-a"),
                namespace: Some("namespace-a"),
                active_graph_style: MemoryGraphStyle::DependencyDag,
                source_commit_or_run_id: Some("run-a"),
            };
            let error = get_approved_postgres_graph_explorer_snapshot(&request)
                .await
                .expect_err("non-postgres build cannot export live graph data");
            assert_eq!(error, GraphExplorerError::SchemaMismatch);
            assert_eq!(
                write_approved_postgres_graph_explorer_artifacts(
                    &request,
                    Path::new("target/dagdb/graph_explorer")
                )
                .await
                .expect_err("non-postgres build cannot write live export artifacts"),
                GraphExplorerError::SchemaMismatch
            );
            assert_eq!(
                read_scoped_postgres_graph("redacted", "tenant-a", "namespace-a")
                    .await
                    .expect_err("non-postgres build cannot read postgres rows"),
                GraphExplorerError::SchemaMismatch
            );
        }

        #[test]
        fn non_postgres_stub_exposes_schema_inventory_metadata() {
            assert_eq!(
                postgres_source_table_names(),
                vec![
                    "dagdb_graph_edges".to_string(),
                    "dagdb_graph_edge_tombstones".to_string(),
                    "dagdb_graph_nodes".to_string(),
                    "dagdb_graph_views".to_string(),
                ]
            );
            assert!(
                postgres_source_columns()
                    .iter()
                    .any(|column| column == "dagdb_graph_nodes.tenant_id")
            );
        }
    }
}

#[cfg(not(feature = "postgres"))]
pub use non_postgres::*;

#[cfg(feature = "postgres")]
mod postgres {

    use std::{
        collections::{BTreeMap, BTreeSet},
        path::Path,
    };

    use exo_core::Hash256;
    use exo_dag_db_api::{MemoryEdgeKind, MemoryGraphStyle, MemoryNodeKind};
    use serde::Serialize;
    use sqlx::{Connection, PgConnection, Row};

    use crate::{
        graph::{MemoryGraphEdge, MemoryGraphNode},
        graph_explorer::{
            GRAPH_EXPLORER_DATABASE_URL_ENV, GRAPH_EXPLORER_MAX_EDGE_ROWS_READ,
            GRAPH_EXPLORER_MAX_GRAPH_VIEW_ROWS_READ, GRAPH_EXPLORER_MAX_NODE_ROWS_READ,
            GraphExplorerArtifactSet, GraphExplorerEdgeStatus, GraphExplorerError,
            GraphExplorerExportInput, GraphExplorerSnapshot, GraphSourceTruthLevel,
            LiveGraphExportRequest, get_graph_explorer_snapshot, inspector_details_for_snapshot,
            validate_graph_explorer_source_rows, validate_live_export_request,
            write_graph_explorer_artifacts,
        },
        scoring::hash_event_body,
    };

    pub const GRAPH_EXPLORER_NODE_QUERY: &str = "\
SELECT graph_node_id, tenant_id, namespace, memory_id, graph_style, node_kind
FROM dagdb_graph_nodes
WHERE tenant_id = $1 AND namespace = $2
ORDER BY graph_style, memory_id, node_kind, graph_node_id
LIMIT $3";

    pub const GRAPH_EXPLORER_EDGE_QUERY: &str = "\
SELECT edge.graph_edge_id,
       edge.tenant_id,
       edge.namespace,
       edge.graph_style,
       edge.from_memory_id,
       edge.to_memory_id,
       edge.edge_kind,
       edge.receipt_hash,
       EXISTS (
         SELECT 1
         FROM dagdb_graph_edge_tombstones tombstone
         WHERE tombstone.tenant_id = edge.tenant_id
           AND tombstone.namespace = edge.namespace
           AND tombstone.prior_edge_id = edge.graph_edge_id
       ) AS is_tombstoned
FROM dagdb_graph_edges edge
WHERE edge.tenant_id = $1 AND edge.namespace = $2
ORDER BY edge.graph_style, edge.from_memory_id, edge.to_memory_id, edge.edge_kind, edge.graph_edge_id
LIMIT $3";

    pub const GRAPH_EXPLORER_VIEW_QUERY: &str = "\
SELECT view_id
FROM dagdb_graph_views
WHERE tenant_id = $1 AND namespace = $2
ORDER BY graph_style, view_id
LIMIT $3";

    const BEGIN_READ_ONLY_SQL: &str = "BEGIN READ ONLY";
    const SET_STATEMENT_TIMEOUT_SQL: &str = "SET LOCAL statement_timeout = '5000ms'";
    const SET_LOCK_TIMEOUT_SQL: &str = "SET LOCAL lock_timeout = '5000ms'";
    const COMMIT_SQL: &str = "COMMIT";
    const ROLLBACK_SQL: &str = "ROLLBACK";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GraphExplorerPostgresExportRequest<'a> {
        pub env: &'a BTreeMap<String, String>,
        pub tenant_id: Option<&'a str>,
        pub namespace: Option<&'a str>,
        pub active_graph_style: MemoryGraphStyle,
        pub source_commit_or_run_id: Option<&'a str>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GraphExplorerPostgresRows {
        pub nodes: Vec<MemoryGraphNode>,
        pub edges: Vec<MemoryGraphEdge>,
        pub edge_statuses: BTreeMap<String, GraphExplorerEdgeStatus>,
        pub source_graph_view_ids: Vec<String>,
    }

    pub async fn get_approved_postgres_graph_explorer_snapshot(
        request: &GraphExplorerPostgresExportRequest<'_>,
    ) -> Result<GraphExplorerSnapshot, GraphExplorerError> {
        let live_request = LiveGraphExportRequest {
            env: request.env,
            tenant_id: request.tenant_id,
            namespace: request.namespace,
        };
        validate_live_export_request(&live_request)?;
        let database_url = request
            .env
            .get(GRAPH_EXPLORER_DATABASE_URL_ENV)
            .ok_or(GraphExplorerError::LiveExportDatabaseUrlMissing)?;
        let tenant_id = request
            .tenant_id
            .ok_or(GraphExplorerError::LiveExportTenantIdMissing)?;
        let namespace = request
            .namespace
            .ok_or(GraphExplorerError::LiveExportNamespaceMissing)?;
        let rows = read_scoped_postgres_graph(database_url, tenant_id, namespace).await?;
        let source_truth_level = if rows.nodes.is_empty() {
            GraphSourceTruthLevel::ApprovedLiveExportEmptyScope
        } else {
            GraphSourceTruthLevel::ActualStoredDag
        };
        let source_receipt_ids = rows
            .edges
            .iter()
            .filter_map(|edge| {
                edge.provenance_receipt_id
                    .map(|receipt| receipt.to_string())
            })
            .collect::<Vec<_>>();
        let edge_statuses = rows.edge_statuses.clone();
        let input = GraphExplorerExportInput {
            tenant_id: Some(tenant_id.into()),
            namespace: Some(namespace.into()),
            active_graph_style: request.active_graph_style,
            source_truth_level,
            source_commit_or_run_id: request.source_commit_or_run_id.map(str::to_owned),
            nodes: rows.nodes,
            edges: rows.edges,
            source_graph_view_ids: rows.source_graph_view_ids,
            source_receipt_ids,
        };
        let mut snapshot =
            get_graph_explorer_snapshot(&input).map_err(|_| GraphExplorerError::SchemaMismatch)?;
        apply_postgres_edge_statuses(&mut snapshot, &edge_statuses);
        apply_postgres_provenance(&mut snapshot)?;
        Ok(snapshot)
    }

    pub async fn write_approved_postgres_graph_explorer_artifacts(
        request: &GraphExplorerPostgresExportRequest<'_>,
        target_dir: &Path,
    ) -> Result<GraphExplorerArtifactSet, GraphExplorerError> {
        let snapshot = get_approved_postgres_graph_explorer_snapshot(request).await?;
        let inspector = inspector_details_for_snapshot(&snapshot);
        write_graph_explorer_artifacts(&snapshot, &inspector, target_dir)
    }

    pub async fn read_scoped_postgres_graph(
        database_url: &str,
        tenant_id: &str,
        namespace: &str,
    ) -> Result<GraphExplorerPostgresRows, GraphExplorerError> {
        let mut connection = PgConnection::connect(database_url)
            .await
            .map_err(|_| GraphExplorerError::DatabaseConnectionFailed)?;

        sqlx::raw_sql(BEGIN_READ_ONLY_SQL)
            .execute(&mut connection)
            .await
            .map_err(schema_error)?;
        let rows =
            match read_scoped_postgres_graph_in_transaction(&mut connection, tenant_id, namespace)
                .await
            {
                Ok(rows) => rows,
                Err(error) => {
                    let _ = sqlx::raw_sql(ROLLBACK_SQL).execute(&mut connection).await;
                    return Err(error);
                }
            };
        sqlx::raw_sql(COMMIT_SQL)
            .execute(&mut connection)
            .await
            .map_err(schema_error)?;
        Ok(rows)
    }

    async fn read_scoped_postgres_graph_in_transaction(
        connection: &mut PgConnection,
        tenant_id: &str,
        namespace: &str,
    ) -> Result<GraphExplorerPostgresRows, GraphExplorerError> {
        sqlx::raw_sql(SET_STATEMENT_TIMEOUT_SQL)
            .execute(&mut *connection)
            .await
            .map_err(schema_error)?;
        sqlx::raw_sql(SET_LOCK_TIMEOUT_SQL)
            .execute(&mut *connection)
            .await
            .map_err(schema_error)?;

        let node_rows = sqlx::query(GRAPH_EXPLORER_NODE_QUERY)
            .bind(tenant_id)
            .bind(namespace)
            .bind(i64::from(GRAPH_EXPLORER_MAX_NODE_ROWS_READ))
            .fetch_all(&mut *connection)
            .await
            .map_err(schema_error)?;
        let edge_rows = sqlx::query(GRAPH_EXPLORER_EDGE_QUERY)
            .bind(tenant_id)
            .bind(namespace)
            .bind(i64::from(GRAPH_EXPLORER_MAX_EDGE_ROWS_READ))
            .fetch_all(&mut *connection)
            .await
            .map_err(schema_error)?;
        let view_rows = sqlx::query(GRAPH_EXPLORER_VIEW_QUERY)
            .bind(tenant_id)
            .bind(namespace)
            .bind(i64::from(GRAPH_EXPLORER_MAX_GRAPH_VIEW_ROWS_READ))
            .fetch_all(&mut *connection)
            .await
            .map_err(schema_error)?;

        let mut nodes = Vec::new();
        let mut source_nodes = Vec::new();
        for row in node_rows {
            assert_scope(&row, tenant_id, namespace, "node")?;
            let memory_id = hash_from_row(&row, "memory_id")?;
            let graph_style = parse_graph_style(row.try_get("graph_style").map_err(schema_error)?)?;
            let node_kind = parse_node_kind(row.try_get("node_kind").map_err(schema_error)?)?;
            nodes.push(MemoryGraphNode {
                memory_id,
                node_kind,
                graph_style,
            });
            source_nodes.push(crate::graph_explorer::GraphExplorerSourceNode {
                node_id: memory_id.to_string(),
                label: memory_id.to_string(),
                node_kind,
                graph_style,
                catalog_path: Vec::new(),
                status: crate::graph_explorer::GraphExplorerNodeStatus::Active,
                risk_class: None,
                owner_id: None,
                receipt_ids: Vec::new(),
                source_hash: Some(memory_id.to_string()),
                content_hash: Some(memory_id.to_string()),
                metadata_summary: Vec::new(),
            });
        }

        let exported_node_ids = nodes
            .iter()
            .map(|node| node.memory_id)
            .collect::<BTreeSet<_>>();
        let mut edges = Vec::new();
        let mut edge_statuses = BTreeMap::new();
        let mut source_edges = Vec::new();
        for row in edge_rows {
            assert_scope(&row, tenant_id, namespace, "edge")?;
            let edge_id = hash_from_row(&row, "graph_edge_id")?;
            let graph_style = parse_graph_style(row.try_get("graph_style").map_err(schema_error)?)?;
            let from_memory_id = hash_from_row(&row, "from_memory_id")?;
            let to_memory_id = hash_from_row(&row, "to_memory_id")?;
            if !exported_node_ids.contains(&from_memory_id)
                || !exported_node_ids.contains(&to_memory_id)
            {
                continue;
            }
            let edge_kind = parse_edge_kind(row.try_get("edge_kind").map_err(schema_error)?)?;
            let receipt_hash = optional_hash_from_row(&row, "receipt_hash")?;
            let is_tombstoned = row.try_get("is_tombstoned").map_err(schema_error)?;
            let edge_status = if is_tombstoned {
                GraphExplorerEdgeStatus::Tombstoned
            } else {
                GraphExplorerEdgeStatus::Active
            };
            edges.push(MemoryGraphEdge {
                edge_id,
                tenant_id: tenant_id.into(),
                namespace: namespace.into(),
                from_memory_id,
                to_memory_id,
                edge_kind,
                graph_style,
                provenance_receipt_id: receipt_hash,
            });
            edge_statuses.insert(edge_id.to_string(), edge_status);
            source_edges.push(crate::graph_explorer::GraphExplorerSourceEdge {
                edge_id: edge_id.to_string(),
                source_node_id: from_memory_id.to_string(),
                target_node_id: to_memory_id.to_string(),
                edge_kind,
                graph_style,
                receipt_id: receipt_hash.map(|receipt| receipt.to_string()),
                status: edge_status,
                confidence_bp: None,
            });
        }
        validate_graph_explorer_source_rows(&source_nodes, &source_edges)?;

        let mut source_graph_view_ids = Vec::new();
        for row in view_rows {
            source_graph_view_ids.push(hash_from_row(&row, "view_id")?.to_string());
        }
        source_graph_view_ids.sort();
        source_graph_view_ids.dedup();

        Ok(GraphExplorerPostgresRows {
            nodes,
            edges,
            edge_statuses,
            source_graph_view_ids,
        })
    }

    fn apply_postgres_edge_statuses(
        snapshot: &mut GraphExplorerSnapshot,
        edge_statuses: &BTreeMap<String, GraphExplorerEdgeStatus>,
    ) {
        for edge in &mut snapshot.edges {
            if let Some(status) = edge_statuses.get(&edge.edge_id) {
                edge.status = *status;
            }
        }
    }

    pub fn apply_postgres_provenance(
        snapshot: &mut GraphExplorerSnapshot,
    ) -> Result<(), GraphExplorerError> {
        snapshot.source_table_names = postgres_source_table_names();
        snapshot.schema_inventory_hash = Some(
            hash_event_body(&PostgresSchemaInventoryMaterial {
                source_table_names: &snapshot.source_table_names,
                source_columns: &postgres_source_columns(),
            })
            .map_err(|_| GraphExplorerError::SchemaMismatch)?
            .to_string(),
        );
        snapshot.source_column_set_hash = Some(
            hash_event_body(&PostgresColumnSetMaterial {
                source_columns: &postgres_source_columns(),
            })
            .map_err(|_| GraphExplorerError::SchemaMismatch)?
            .to_string(),
        );
        snapshot.artifact_hash = None;
        snapshot.artifact_hash = Some(
            hash_event_body(snapshot)
                .map_err(|_| GraphExplorerError::SchemaMismatch)?
                .to_string(),
        );
        Ok(())
    }

    #[must_use]
    pub fn postgres_source_table_names() -> Vec<String> {
        vec![
            "dagdb_graph_edges".into(),
            "dagdb_graph_edge_tombstones".into(),
            "dagdb_graph_nodes".into(),
            "dagdb_graph_views".into(),
        ]
    }

    #[must_use]
    pub fn postgres_source_columns() -> Vec<String> {
        vec![
            "dagdb_graph_edges.edge_kind".into(),
            "dagdb_graph_edges.from_memory_id".into(),
            "dagdb_graph_edges.graph_edge_id".into(),
            "dagdb_graph_edges.graph_style".into(),
            "dagdb_graph_edges.namespace".into(),
            "dagdb_graph_edges.receipt_hash".into(),
            "dagdb_graph_edges.tenant_id".into(),
            "dagdb_graph_edges.to_memory_id".into(),
            "dagdb_graph_edge_tombstones.namespace".into(),
            "dagdb_graph_edge_tombstones.prior_edge_id".into(),
            "dagdb_graph_edge_tombstones.tenant_id".into(),
            "dagdb_graph_nodes.graph_node_id".into(),
            "dagdb_graph_nodes.graph_style".into(),
            "dagdb_graph_nodes.memory_id".into(),
            "dagdb_graph_nodes.namespace".into(),
            "dagdb_graph_nodes.node_kind".into(),
            "dagdb_graph_nodes.tenant_id".into(),
            "dagdb_graph_views.namespace".into(),
            "dagdb_graph_views.tenant_id".into(),
            "dagdb_graph_views.view_id".into(),
        ]
    }

    fn assert_scope(
        row: &sqlx::postgres::PgRow,
        tenant_id: &str,
        namespace: &str,
        row_kind: &'static str,
    ) -> Result<(), GraphExplorerError> {
        let row_tenant_id: String = row.try_get("tenant_id").map_err(schema_error)?;
        let row_namespace: String = row.try_get("namespace").map_err(schema_error)?;
        if row_tenant_id != tenant_id || row_namespace != namespace {
            return Err(GraphExplorerError::InvalidSourceRow {
                reason: format!("cross_scope_{row_kind}"),
            });
        }
        Ok(())
    }

    fn hash_from_row(
        row: &sqlx::postgres::PgRow,
        field_name: &'static str,
    ) -> Result<Hash256, GraphExplorerError> {
        let value: Vec<u8> = row.try_get(field_name).map_err(schema_error)?;
        hash_from_vec(value, field_name)
    }

    fn optional_hash_from_row(
        row: &sqlx::postgres::PgRow,
        field_name: &'static str,
    ) -> Result<Option<Hash256>, GraphExplorerError> {
        let value: Option<Vec<u8>> = row.try_get(field_name).map_err(schema_error)?;
        value
            .map(|bytes| hash_from_vec(bytes, field_name))
            .transpose()
    }

    fn hash_from_vec(
        value: Vec<u8>,
        field_name: &'static str,
    ) -> Result<Hash256, GraphExplorerError> {
        let bytes: [u8; 32] =
            value
                .try_into()
                .map_err(|_| GraphExplorerError::InvalidSourceRow {
                    reason: format!("invalid_hash_length_{field_name}"),
                })?;
        Ok(Hash256::from_bytes(bytes))
    }

    fn parse_graph_style(value: &str) -> Result<MemoryGraphStyle, GraphExplorerError> {
        match value {
            "provenance_receipt_dag" => Ok(MemoryGraphStyle::ProvenanceReceiptDag),
            "canonical_memory_graph" => Ok(MemoryGraphStyle::CanonicalMemoryGraph),
            "semantic_catalog_graph" => Ok(MemoryGraphStyle::SemanticCatalogGraph),
            "similarity_overlay_graph" => Ok(MemoryGraphStyle::SimilarityOverlayGraph),
            "dependency_dag" => Ok(MemoryGraphStyle::DependencyDag),
            "routing_view_graph" => Ok(MemoryGraphStyle::RoutingViewGraph),
            "contradiction_supersession_graph" => {
                Ok(MemoryGraphStyle::ContradictionSupersessionGraph)
            }
            "context_packet_graph" => Ok(MemoryGraphStyle::ContextPacketGraph),
            _ => Err(GraphExplorerError::InvalidSourceRow {
                reason: "invalid_graph_style".into(),
            }),
        }
    }

    fn parse_node_kind(value: &str) -> Result<MemoryNodeKind, GraphExplorerError> {
        match value {
            "raw" => Ok(MemoryNodeKind::Raw),
            "chunk" => Ok(MemoryNodeKind::Chunk),
            "summary" => Ok(MemoryNodeKind::Summary),
            "concept" => Ok(MemoryNodeKind::Concept),
            "canonical" => Ok(MemoryNodeKind::Canonical),
            "duplicate_reference" => Ok(MemoryNodeKind::DuplicateReference),
            "related" => Ok(MemoryNodeKind::Related),
            "replacement" => Ok(MemoryNodeKind::Replacement),
            "contradiction" => Ok(MemoryNodeKind::Contradiction),
            "supersession" => Ok(MemoryNodeKind::Supersession),
            "alternate_summary" => Ok(MemoryNodeKind::AlternateSummary),
            "decision" => Ok(MemoryNodeKind::Decision),
            "route" => Ok(MemoryNodeKind::Route),
            "validation_report" => Ok(MemoryNodeKind::ValidationReport),
            "savings_report" => Ok(MemoryNodeKind::SavingsReport),
            _ => Err(GraphExplorerError::InvalidSourceRow {
                reason: "invalid_node_kind".into(),
            }),
        }
    }

    fn parse_edge_kind(value: &str) -> Result<MemoryEdgeKind, GraphExplorerError> {
        match value {
            "derived_from" => Ok(MemoryEdgeKind::DerivedFrom),
            "summarizes" => Ok(MemoryEdgeKind::Summarizes),
            "supports" => Ok(MemoryEdgeKind::Supports),
            "contradicts" => Ok(MemoryEdgeKind::Contradicts),
            "supersedes" => Ok(MemoryEdgeKind::Supersedes),
            "replaces" => Ok(MemoryEdgeKind::Replaces),
            "duplicate_of" => Ok(MemoryEdgeKind::DuplicateOf),
            "near_duplicate_of" => Ok(MemoryEdgeKind::NearDuplicateOf),
            "related_to" => Ok(MemoryEdgeKind::RelatedTo),
            "alternative_summary_of" => Ok(MemoryEdgeKind::AlternativeSummaryOf),
            "depends_on" => Ok(MemoryEdgeKind::DependsOn),
            "part_of" => Ok(MemoryEdgeKind::PartOf),
            "owned_by" => Ok(MemoryEdgeKind::OwnedBy),
            "access_granted_by" => Ok(MemoryEdgeKind::AccessGrantedBy),
            "verified_by" => Ok(MemoryEdgeKind::VerifiedBy),
            "used_by_route" => Ok(MemoryEdgeKind::UsedByRoute),
            "included_in_context_packet" => Ok(MemoryEdgeKind::IncludedInContextPacket),
            "revoked_by" => Ok(MemoryEdgeKind::RevokedBy),
            _ => Err(GraphExplorerError::InvalidSourceRow {
                reason: "invalid_edge_kind".into(),
            }),
        }
    }

    fn schema_error(_source: sqlx::Error) -> GraphExplorerError {
        GraphExplorerError::SchemaMismatch
    }

    #[derive(Debug, Serialize)]
    struct PostgresSchemaInventoryMaterial<'a> {
        source_table_names: &'a [String],
        source_columns: &'a [String],
    }

    #[derive(Debug, Serialize)]
    struct PostgresColumnSetMaterial<'a> {
        source_columns: &'a [String],
    }
}

#[cfg(feature = "postgres")]
pub use postgres::*;
