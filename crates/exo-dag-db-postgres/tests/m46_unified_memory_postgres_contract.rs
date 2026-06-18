#![cfg(feature = "postgres")]
#![allow(clippy::expect_used)]

use exo_core::Hash256;
use exo_dag_db_postgres::{
    KgExportScope, KgRetrievalRequest, build_persistent_graph_context_packet,
    build_persistent_graph_context_selection, persist_context_packet_receipt_to_db,
    persist_usage_event_to_db,
    postgres::{kg_export, kg_import, kg_retrieval, kg_writeback},
};

fn h(byte: u8) -> String {
    Hash256::from_bytes([byte; 32]).to_string()
}

fn retrieval_request() -> KgRetrievalRequest {
    KgRetrievalRequest {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        task_hash: Some(h(0x01)),
        task_description: Some("retrieve bounded M46 context".to_owned()),
        token_budget: 512,
        requested_memory_ids: Vec::new(),
        catalog_path: Some(vec!["KnowledgeGraphs".to_owned(), "dag-db".to_owned()]),
        max_memory_refs: Some(1),
        layer_path: None,
        max_layer_depth: None,
        max_layers_selected: None,
        max_nodes_per_layer: None,
        max_layer_edges: None,
    }
}

#[tokio::test]
async fn postgres_repository_adapters_are_public_and_fail_closed_without_live_db_url() {
    let request = retrieval_request();
    let export_scope = KgExportScope {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        included_memory_ids: Vec::new(),
        included_graph_styles: Vec::new(),
        included_writeback_idempotency_keys: Vec::new(),
        source_commit_or_repo_ref: Some("m46-contract-test".to_owned()),
        include_preview_context: true,
    };

    assert!(
        kg_import::persist_kg_import_report_from_database_url(None, "{}")
            .await
            .is_err()
    );
    assert!(
        kg_export::build_kg_portable_export_from_database_url(None, &export_scope, &[])
            .await
            .is_err()
    );
    assert!(
        kg_retrieval::retrieve_kg_context_packet_from_database_url(None, &request)
            .await
            .is_err()
    );
    assert!(
        kg_writeback::persist_kg_writeback_report_from_database_url(None, "{}")
            .await
            .is_err()
    );

    let _persistent_selection = build_persistent_graph_context_selection;
    let _persistent_packet = build_persistent_graph_context_packet;
    let _usage_write = persist_usage_event_to_db;
    let _packet_receipt_write = persist_context_packet_receipt_to_db;
    let _export_verify = kg_export::verify_persisted_kg_export;
    let _export_persist_json = kg_export::persist_kg_portable_export_json;
    let _import_from_env = kg_import::persist_kg_import_report_from_env;
    let _export_from_env = kg_export::build_kg_portable_export_from_env;
    let _retrieval_from_env = kg_retrieval::retrieve_kg_context_packet_from_env;
    let _writeback_from_env = kg_writeback::persist_kg_writeback_report_from_env;
}
