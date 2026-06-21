#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use exo_core::Hash256;
use exo_dag_db_api::{
    MemoryCandidateKind, MemoryCandidateUse, RiskClass, SafeMetadata, SafeMetadataDecision,
};
use exo_dag_db_exchange::{
    hash::RequestHashMaterial,
    kg_retrieval::{self, KG_CONTEXT_PACKET_PREVIEW_SCHEMA, KG_RETRIEVAL_PREVIEW_ROUTE_NAME},
    kg_writeback::{
        KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA, KG_WRITEBACK_PERSISTED_ROUTE_NAME,
        KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA, KgWritebackDryRunReport, KgWritebackExistingMemory,
        KgWritebackPersistedSummary, KgWritebackProposalRequest, build_writeback_dry_run_report,
        parse_agent_writeback_hint_json,
    },
};
use serde_json::json;

fn h(byte: u8) -> String {
    Hash256::from_bytes([byte; 32]).to_string()
}

fn safe(text: &str) -> SafeMetadata {
    SafeMetadata {
        decision: SafeMetadataDecision::Allow,
        text: text.to_owned(),
        redaction_codes: Vec::new(),
        original_hash: h(0xee),
        truncated: false,
        byte_len: u32::try_from(text.len()).expect("fixture fits in u32"),
    }
}

fn context_preview() -> kg_retrieval::KgContextPacketPreview {
    let request = kg_retrieval::KgRetrievalRequest {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        task_hash: Some(h(0x01)),
        task_description: Some("unified memory persistence contract".to_owned()),
        token_budget: 512,
        requested_memory_ids: Vec::new(),
        catalog_path: Some(vec!["KnowledgeGraphs".to_owned(), "dag-db".to_owned()]),
        max_memory_refs: Some(1),
        layer_path: None,
        max_layer_depth: None,
        max_layers_selected: None,
        max_nodes_per_layer: None,
        max_layer_edges: None,
    };
    request.validate().expect("retrieval request validates");

    let memory_id = h(0x22);
    let route_hint_id = kg_retrieval::route_hint_id(&request, std::slice::from_ref(&memory_id))
        .expect("route hint");
    let context_packet_id = kg_retrieval::context_packet_preview_id(
        &request,
        &route_hint_id.to_string(),
        std::slice::from_ref(&memory_id),
    )
    .expect("context packet preview id");
    let citation_handle =
        kg_retrieval::citation_handle("tenant-a", "primary", &memory_id, Some("catalog-a"))
            .expect("citation handle");

    let mut validation_status_counts = BTreeMap::new();
    validation_status_counts.insert("pending".to_owned(), 1);

    let memory_ref = kg_retrieval::KgMemoryRef {
        memory_id: memory_id.clone(),
        catalog_id: Some("catalog-a".to_owned()),
        source_path: None,
        catalog_path: vec!["KnowledgeGraphs".to_owned(), "dag-db".to_owned()],
        layer_id: None,
        layer_path: None,
        layer_depth: None,
        layer_kind: None,
        layer_membership_role: None,
        layer_selection_reason: None,
        rollup_summary_ref: None,
        title: safe("Unified memory persistence contract"),
        summary: safe("Repository/test persistence DTO and idempotency chain"),
        latest_receipt_hash: h(0x23),
        memory_status: "pending".to_owned(),
        validation_status: "pending".to_owned(),
        risk_class: "R1".to_owned(),
        council_status: "not_required".to_owned(),
        dag_finality_status: "pending".to_owned(),
        graph_node_ids: vec![h(0x24)],
        validation_report_ids: vec![h(0x25)],
        citation_handle: citation_handle.clone(),
        token_estimate: 64,
        selection_reasons: vec!["within_token_budget".to_owned()],
    };
    kg_retrieval::KgContextPacketPreview {
        schema_version: KG_CONTEXT_PACKET_PREVIEW_SCHEMA.to_owned(),
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        context_packet_id: context_packet_id.to_string(),
        route_hint_id: route_hint_id.to_string(),
        memory_refs: vec![memory_ref.clone()],
        graph_edges: Vec::new(),
        selected_refs: vec![memory_ref],
        selected_layers: Vec::new(),
        selected_layer_edges: Vec::new(),
        selected_graph_edges: Vec::new(),
        rollup_summaries: Vec::new(),
        budget_report: kg_retrieval::KgLayerBudgetReport {
            max_layer_depth: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
            max_layers_selected: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
            max_nodes_per_layer: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
            max_memory_refs: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS,
            max_layer_edges: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
            selected_layer_count: 0,
            selected_layer_edge_count: 0,
            active_layer_edge_count: 0,
            excluded_demoted_layer_edge_count: 0,
            excluded_tombstoned_layer_edge_count: 0,
            selected_memory_ref_count: 1,
            selected_graph_edge_count: 0,
            depth_budget_truncated: false,
            layer_budget_truncated: false,
            node_budget_truncated: false,
            layer_edge_budget_truncated: false,
            token_budget_truncated: false,
            flat_fallback_used: true,
        },
        flat_fallback_used: true,
        citation_handles: vec![kg_retrieval::KgCitationHandle {
            handle: citation_handle,
            memory_id,
            catalog_id: Some("catalog-a".to_owned()),
            latest_receipt_hash: h(0x23),
            graph_node_ids: vec![h(0x24)],
            graph_edge_ids: Vec::new(),
            validation_report_ids: vec![h(0x25)],
        }],
        retrieval_diagnostics: kg_retrieval::KgRetrievalDiagnostics {
            selected_memory_count: 1,
            omitted_memory_count: 0,
            selected_graph_edge_count: 0,
            selected_layer_count: 0,
            selected_layer_edge_count: 0,
            active_layer_edge_count: 0,
            excluded_demoted_layer_edge_count: 0,
            excluded_tombstoned_layer_edge_count: 0,
            citation_handle_count: 1,
            warning_count: 1,
            token_budget: 512,
            token_estimate: 64,
            max_layer_depth: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
            max_layers_selected: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
            max_nodes_per_layer: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
            max_layer_edges: kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
            layer_path_filter_applied: false,
            max_memory_refs_applied: true,
            catalog_path_filter_applied: true,
            requested_memory_filter_applied: false,
            flat_fallback_used: true,
            depth_budget_truncated: false,
            layer_budget_truncated: false,
            node_budget_truncated: false,
            layer_edge_budget_truncated: false,
            deterministic_ordering: true,
            raw_markdown_returned: false,
            preview_only: true,
        },
        validation_summary: kg_retrieval::KgValidationSummary {
            selected_memory_count: 1,
            pending_count: 1,
            passed_count: 0,
            failed_count: 0,
            needs_council_count: 0,
            warning_count: 0,
            validation_status_counts,
            risk_class_counts: BTreeMap::new(),
            dag_finality_status_counts: BTreeMap::new(),
            council_status_counts: BTreeMap::new(),
        },
        graph_path_summary: kg_retrieval::KgGraphPathSummary {
            graph_edge_count: 0,
            graph_styles_seen: Vec::new(),
            edge_kinds_seen: Vec::new(),
            isolated_memory_count: 1,
            connected_memory_count: 0,
            missing_edge_warning_count: 0,
        },
        citation_diagnostics: Vec::new(),
        token_budget: 512,
        token_estimate: 64,
        omitted_memory_ids: Vec::new(),
        omitted_memory_refs: Vec::new(),
        warnings: vec!["preview_only_not_production_route".to_owned()],
        dry_run_or_preview_only: true,
    }
}

fn writeback_dry_run_report() -> KgWritebackDryRunReport {
    let preview = context_preview();
    let hint_json = json!({
        "source_request_id": "request-unified-memory-persistence",
        "parent_context_packet_id": preview.context_packet_id,
        "route_hint_id": preview.route_hint_id,
        "task_hash": h(0x31),
        "output_hash": h(0x32),
        "candidate_kind": "summary",
        "summary": "Unified memory persistence contract keeps writeback dry-run only.",
        "citation_handles": [preview.citation_handles[0].handle],
        "evidence_receipts": [h(0x23)],
        "risk_hint": "R1",
        "allowed_future_uses": ["routing"],
        "reason_to_remember": "Future workers need stable idempotency material."
    });
    let hint =
        parse_agent_writeback_hint_json(&hint_json.to_string()).expect("writeback hint parses");

    build_writeback_dry_run_report(KgWritebackProposalRequest {
        tenant_id: preview.tenant_id.clone(),
        namespace: preview.namespace.clone(),
        requesting_agent_did: "did:exo:agent".to_owned(),
        context_packet: preview,
        hint,
        existing_memory: vec![KgWritebackExistingMemory {
            memory_id: h(0x40),
            payload_hash: h(0x41),
            summary: "existing routed memory".to_owned(),
        }],
    })
    .expect("writeback dry-run report builds")
}

#[test]
fn unified_memory_persistence_schema_versions_are_pinned() {
    assert_eq!(
        KG_CONTEXT_PACKET_PREVIEW_SCHEMA,
        "dagdb_kg_context_packet_preview_v1"
    );
    assert_eq!(
        KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA,
        "dagdb_kg_writeback_dry_run_report_v1"
    );
    assert_eq!(
        KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA,
        "dagdb_kg_persisted_writeback_summary_v1"
    );
    assert_eq!(
        KG_WRITEBACK_PERSISTED_ROUTE_NAME,
        "dagdb.kg_writeback.persisted.v1"
    );
    assert_eq!(
        KG_RETRIEVAL_PREVIEW_ROUTE_NAME,
        "dagdb.kg_retrieval.preview.v1"
    );
}

#[test]
fn unified_memory_writeback_dry_run_report_is_persistence_ready_without_db() {
    let first = writeback_dry_run_report();
    let second = writeback_dry_run_report();
    assert_eq!(first, second);
    assert_eq!(first.schema_version, KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA);
    assert!(first.dry_run_only);
    assert!(!first.postgres_writes);
    assert!(first.preview_only);
    assert!(!first.raw_markdown_included);
    assert!(first.acceptance.dry_run_only);
    assert!(!first.acceptance.writeback_persistence_implemented);
    assert!(!first.acceptance.exo_dag_tables_mutated);
    assert!(!first.proposal_summary.persistence_ready);

    first
        .validate_for_persistence()
        .expect("dry-run report validates");

    let idempotency_key = first.idempotency_key().expect("idempotency key");
    assert_eq!(
        idempotency_key,
        first.idempotency_key().expect("idempotency key again")
    );
    assert!(!idempotency_key.is_empty());

    let report_json = serde_json::to_string(&first).expect("serialize dry-run report");
    let reparsed = KgWritebackDryRunReport::parse_json(&report_json).expect("parse dry-run report");
    assert_eq!(reparsed, first);
    assert!(!report_json.contains("# private"));
    assert!(!report_json.contains("private_payload"));
}

#[test]
fn unified_memory_writeback_request_hash_material_is_stable_for_persisted_route() {
    let report = writeback_dry_run_report();
    let report_json = serde_json::to_string(&report).expect("serialize dry-run report");

    let first_hash = RequestHashMaterial {
        route_name: KG_WRITEBACK_PERSISTED_ROUTE_NAME.to_owned(),
        tenant_id: report.tenant_id.clone(),
        namespace: report.namespace.clone(),
        canonical_redacted_request_body: report_json.as_bytes().to_vec(),
    }
    .hash()
    .expect("request hash");

    let second_hash = RequestHashMaterial {
        route_name: KG_WRITEBACK_PERSISTED_ROUTE_NAME.to_owned(),
        tenant_id: report.tenant_id.clone(),
        namespace: report.namespace.clone(),
        canonical_redacted_request_body: report_json.as_bytes().to_vec(),
    }
    .hash()
    .expect("request hash again");

    assert_eq!(first_hash, second_hash);

    let different_route_hash = RequestHashMaterial {
        route_name: KG_RETRIEVAL_PREVIEW_ROUTE_NAME.to_owned(),
        tenant_id: report.tenant_id.clone(),
        namespace: report.namespace.clone(),
        canonical_redacted_request_body: report_json.as_bytes().to_vec(),
    }
    .hash()
    .expect("preview route hash");

    assert_ne!(first_hash, different_route_hash);
}

#[test]
fn unified_memory_persisted_summary_schema_is_pinned() {
    assert_eq!(
        KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA,
        "dagdb_kg_persisted_writeback_summary_v1"
    );

    let incomplete = json!({
        "schema_version": KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA,
        "tenant_id": "tenant-a",
        "namespace": "primary"
    });
    assert!(serde_json::from_value::<KgWritebackPersistedSummary>(incomplete).is_err());
}

#[test]
fn unified_memory_writeback_hint_stays_compact_and_evidence_bound() {
    let preview = context_preview();
    let hint_json = json!({
        "source_request_id": "request-unified-memory-persistence",
        "parent_context_packet_id": preview.context_packet_id,
        "route_hint_id": preview.route_hint_id,
        "task_hash": h(0x31),
        "output_hash": h(0x32),
        "candidate_kind": "summary",
        "summary": "compact writeback hint",
        "citation_handles": [preview.citation_handles[0].handle],
        "evidence_receipts": [h(0x23)],
        "risk_hint": "R1",
        "allowed_future_uses": ["routing"],
        "reason_to_remember": "routing needs it",
        "graph_node_id": h(0x99)
    });
    assert!(parse_agent_writeback_hint_json(&hint_json.to_string()).is_err());

    let report = writeback_dry_run_report();
    assert_eq!(
        report.evidence_binding.parent_context_packet_id,
        report.parent_context_packet_id
    );
    assert_eq!(report.evidence_binding.status, "bound");
    assert_eq!(
        report.proposed_memory_candidate.candidate_kind,
        MemoryCandidateKind::Summary
    );
    assert_eq!(
        report.proposed_memory_candidate.allowed_future_uses,
        vec![MemoryCandidateUse::Routing]
    );
    assert_eq!(report.validation_proposal.risk_class, RiskClass::R1);
}
