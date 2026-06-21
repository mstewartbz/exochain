#![cfg(feature = "postgres")]
#![allow(clippy::expect_used)]

use std::{collections::BTreeMap, path::Path};

use exo_dag_db_api::MemoryGraphStyle;
use exo_dag_db_lab::{
    graph_explorer::{
        GRAPH_EXPLORER_DATABASE_URL_ENV, GraphExplorerError, LIVE_EXPORT_APPROVAL_ENV,
    },
    graph_explorer_postgres::{
        GraphExplorerPostgresExportRequest, write_approved_postgres_graph_explorer_artifacts,
    },
};

#[tokio::test]
async fn postgres_export_blocks_before_connect_without_approval() {
    let env = BTreeMap::from([(GRAPH_EXPLORER_DATABASE_URL_ENV.into(), "redacted".into())]);
    let request = GraphExplorerPostgresExportRequest {
        env: &env,
        tenant_id: Some("tenant-a"),
        namespace: Some("namespace-a"),
        active_graph_style: MemoryGraphStyle::CanonicalMemoryGraph,
        source_commit_or_run_id: Some("postgres-export-test"),
    };
    let error =
        write_approved_postgres_graph_explorer_artifacts(&request, Path::new("target/unused"))
            .await
            .expect_err("missing approval blocks before connect");
    assert_eq!(error, GraphExplorerError::LiveExportNotApproved);
}

#[tokio::test]
async fn postgres_export_blocks_before_connect_without_scope() {
    let env = BTreeMap::from([
        (GRAPH_EXPLORER_DATABASE_URL_ENV.into(), "redacted".into()),
        (LIVE_EXPORT_APPROVAL_ENV.into(), "true".into()),
    ]);
    let request = GraphExplorerPostgresExportRequest {
        env: &env,
        tenant_id: None,
        namespace: Some("namespace-a"),
        active_graph_style: MemoryGraphStyle::CanonicalMemoryGraph,
        source_commit_or_run_id: Some("postgres-export-test"),
    };
    let error =
        write_approved_postgres_graph_explorer_artifacts(&request, Path::new("target/unused"))
            .await
            .expect_err("missing tenant blocks before connect");
    assert_eq!(error, GraphExplorerError::LiveExportTenantIdMissing);
}
