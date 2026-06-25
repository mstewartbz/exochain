#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    DagDbGraphContextPacketBuildRequest, DagDbGraphContextSelectionResponse,
    DagDbGraphContextSelectionStatus, DagDbOmittedContextRef, DagDbSelectedContextRef,
    MemoryCandidateKind, MemoryCandidateUse, MemoryGraphStyle, RiskClass, SafeMetadata,
    SafeMetadataDecision, ValidationStatus,
};
use exo_dag_db_domain::{
    context,
    graph::required_placement_steps,
    placement::{GraphOrganizer, MemoryPlacementController, MemoryPlacementInput},
};
use exo_dag_db_exchange::{
    kg_export::{
        KgExportBuildInput, KgExportScope, build_portable_export, parse_portable_export_json,
        validate_portable_export_for_persistence,
    },
    kg_import,
    kg_writeback::{
        self, KgWritebackExistingMemory, KgWritebackProposalRequest,
        build_writeback_dry_run_report, parse_agent_writeback_hint_json,
    },
};
use exo_dag_db_retrieval::{
    context_packet_output::{GRAPH_CONTEXT_PACKET_SCHEMA_VERSION, build_graph_context_packet},
    kg_retrieval,
};
use serde_json::json;

const CONTRACT_DOC: &str = include_str!(
    "../../../docs/dagdb/catalog-governed-memory/self-development/unified-memory-contract.md"
);

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

fn retrieval_request() -> kg_retrieval::KgRetrievalRequest {
    kg_retrieval::KgRetrievalRequest {
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

fn context_preview() -> kg_retrieval::KgContextPacketPreview {
    let request = retrieval_request();
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
    let mut risk_class_counts = BTreeMap::new();
    risk_class_counts.insert("R1".to_owned(), 1);
    let mut dag_finality_status_counts = BTreeMap::new();
    dag_finality_status_counts.insert("pending".to_owned(), 1);
    let mut council_status_counts = BTreeMap::new();
    council_status_counts.insert("not_required".to_owned(), 1);
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
        title: safe("M46 contract"),
        summary: safe("Rust-first unified memory contract proof"),
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
        schema_version: kg_retrieval::KG_CONTEXT_PACKET_PREVIEW_SCHEMA.to_owned(),
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
            handle: citation_handle.clone(),
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
            risk_class_counts,
            dag_finality_status_counts,
            council_status_counts,
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

#[test]
fn m46_contract_doc_records_continuation_gap_and_blocked_claims() {
    assert!(CONTRACT_DOC.contains("M46 Rust-First Unified Memory Contract"));
    assert!(CONTRACT_DOC.contains("DagDbContinuationPacket"));
    assert!(CONTRACT_DOC.contains("Python remains compatibility/evidence tooling"));
    assert!(CONTRACT_DOC.contains("M46 does not approve live DB mutation"));
    assert!(CONTRACT_DOC.contains("M56 acceptance"));
}

#[test]
fn import_export_and_retrieval_contract_surfaces_are_public() {
    let import_json = json!({
        "schema_version": kg_import::KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
        "source_candidates_schema_version": kg_import::KG_IMPORT_CANDIDATES_SCHEMA,
        "graph_root": "KnowledgeGraphs/dag-db",
        "tenant_id": "tenant-a",
        "namespace": "primary",
        "actor_did": "did:exo:agent",
        "batch_id": h(0x10),
        "dry_run_only": true,
        "postgres_writes": false,
        "raw_markdown_included": false,
        "proposed_memory_records": [],
        "proposed_catalog_entries": [],
        "proposed_graph_nodes": [],
        "proposed_graph_edges": [],
        "proposed_required_edges": [],
        "proposed_placement_decisions": [],
        "proposed_receipt_intents": [],
        "proposed_validation_reports": []
    });
    let import_report = kg_import::KgImportDryRunReport::parse_json(&import_json.to_string())
        .expect("dry-run import report parses");
    import_report.validate().expect("dry-run import validates");
    assert!(
        !import_report
            .idempotency_key()
            .expect("idempotency key")
            .is_empty()
    );

    let export_scope = KgExportScope {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        included_memory_ids: Vec::new(),
        included_graph_styles: Vec::new(),
        included_writeback_idempotency_keys: Vec::new(),
        source_commit_or_repo_ref: Some("m46-contract-test".to_owned()),
        include_preview_context: true,
    };
    let export = build_portable_export(KgExportBuildInput {
        scope: export_scope,
        memory_records: Vec::new(),
        catalog_entries: Vec::new(),
        graph_nodes: Vec::new(),
        graph_edges: Vec::new(),
        similarity_results: Vec::new(),
        canonicalization_decisions: Vec::new(),
        placement_traces: Vec::new(),
        validation_reports: Vec::new(),
        receipts: Vec::new(),
        subject_receipt_heads: Vec::new(),
        context_packet_previews: Vec::new(),
        context_packet_records: Vec::new(),
        route_receipts: Vec::new(),
        writeback_summaries: Vec::new(),
        idempotency_references: Vec::new(),
        citation_index: Vec::new(),
        provenance_index: Vec::new(),
    })
    .expect("portable export builds");
    validate_portable_export_for_persistence(&export).expect("portable export validates");
    let reparsed = parse_portable_export_json(&serde_json::to_string(&export).expect("json"))
        .expect("portable export parses");
    assert_eq!(reparsed.export_id, export.export_id);

    let request = retrieval_request();
    request.validate().expect("retrieval request validates");
    let selected_memory_ids = vec![h(0x22)];
    let route_hint =
        kg_retrieval::route_hint_id(&request, &selected_memory_ids).expect("route hint id");
    let preview_id = kg_retrieval::context_packet_preview_id(
        &request,
        &route_hint.to_string(),
        &selected_memory_ids,
    )
    .expect("preview id");
    let citation =
        kg_retrieval::citation_handle("tenant-a", "primary", &selected_memory_ids[0], None)
            .expect("citation handle");
    assert!(!preview_id.to_string().is_empty());
    assert!(citation.starts_with("dagdb://kg/tenant-a/primary/"));
}

#[test]
fn writeback_and_placement_contract_surfaces_remain_dry_run_only() {
    let preview = context_preview();
    let hint_json = json!({
        "source_request_id": "request-m46",
        "parent_context_packet_id": preview.context_packet_id,
        "route_hint_id": preview.route_hint_id,
        "task_hash": h(0x31),
        "output_hash": h(0x32),
        "candidate_kind": "summary",
        "summary": "M46 proof keeps writeback dry-run only.",
        "citation_handles": [preview.citation_handles[0].handle],
        "evidence_receipts": [],
        "risk_hint": "R1",
        "allowed_future_uses": ["routing"],
        "reason_to_remember": "Future workers need the M46 contract boundary."
    });
    let hint =
        parse_agent_writeback_hint_json(&hint_json.to_string()).expect("writeback hint parses");
    assert_eq!(hint.candidate_kind, MemoryCandidateKind::Summary);
    assert_eq!(hint.allowed_future_uses, vec![MemoryCandidateUse::Routing]);

    let report = build_writeback_dry_run_report(KgWritebackProposalRequest {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        requesting_agent_did: "did:exo:agent".to_owned(),
        context_packet: preview,
        hint,
        existing_memory: vec![KgWritebackExistingMemory {
            memory_id: h(0x40),
            payload_hash: h(0x41),
            summary: "existing routed memory".to_owned(),
        }],
    })
    .expect("writeback dry-run report builds");
    assert_eq!(
        report.schema_version,
        kg_writeback::KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA
    );
    assert!(report.dry_run_only);
    assert!(!report.postgres_writes);
    assert!(!report.acceptance.exo_dag_tables_mutated);

    let placement = GraphOrganizer::organize(MemoryPlacementInput {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        input_memory_id: Hash256::from_bytes([0x50; 32]),
        payload_hash: Hash256::from_bytes([0x51; 32]),
        summary: "M46 placement and recycling proof".to_owned(),
        risk_class: RiskClass::R1,
        validator_status: ValidationStatus::Passed,
        existing_memory: Vec::new(),
        requested_decision: None,
        receipt_intent: "m46_placement_contract".to_owned(),
        now: Timestamp::new(1_000, 0),
    })
    .expect("placement organizes");
    assert!(
        placement
            .graph_views_to_refresh
            .contains(&MemoryGraphStyle::SemanticCatalogGraph)
    );
    assert!(
        MemoryPlacementController::validate_placement_order(&required_placement_steps()).is_ok()
    );
}

#[test]
fn context_packet_generation_contract_surface_builds_metadata_only_packet() {
    let memory_id = h(0x60);
    let selection = DagDbGraphContextSelectionResponse {
        tenant_id: "tenant-a".to_owned(),
        namespace: "primary".to_owned(),
        request_id: "request-m46-packet".to_owned(),
        task_hash: h(0x61),
        selection_status: DagDbGraphContextSelectionStatus::Selected,
        selected_memory_refs: vec![DagDbSelectedContextRef {
            memory_id,
            catalog_id: Some("catalog-m46".to_owned()),
            title: safe("M46 packet"),
            summary: safe("metadata-only context packet"),
            catalog_path: vec!["KnowledgeGraphs".to_owned(), "dag-db".to_owned()],
            document_type: "contract".to_owned(),
            selection_reason: "m46_contract_surface".to_owned(),
            token_estimate: 64,
            validation_status: ValidationStatus::Passed,
            citation_ref: "citation:m46".to_owned(),
            boundary_flags: vec!["repository_test_only".to_owned()],
        }],
        selected_graph_edges: Vec::new(),
        omitted_memory_refs: Vec::<DagDbOmittedContextRef>::new(),
        selection_trace: Vec::new(),
        selected_token_estimate: 64,
        token_budget: 512,
        boundary_warnings: vec!["production_runtime_not_approved".to_owned()],
    };
    let packet = build_graph_context_packet(&DagDbGraphContextPacketBuildRequest {
        tenant_id: selection.tenant_id.clone(),
        namespace: selection.namespace.clone(),
        request_id: selection.request_id.clone(),
        task: "Build M46 contract packet".to_owned(),
        task_hash: selection.task_hash.clone(),
        audit_id: "audit-m46-contract".to_owned(),
        token_budget: selection.token_budget,
        max_memory_refs: None,
        selection,
        import_tracking_status: None,
    })
    .expect("graph context packet builds");

    assert_eq!(packet.schema_version, GRAPH_CONTEXT_PACKET_SCHEMA_VERSION);
    assert!(packet.boundaries.repository_test_level_only);
    assert_eq!(packet.boundaries.production_runtime, "blocked");
    assert!(
        packet
            .citation_refs
            .iter()
            .all(|citation| citation.citation_status == "metadata_only_no_locator")
    );

    let _lower_level_context_builder = context::build_context_packet;
    let _lower_level_context_input = core::mem::size_of::<context::ContextPacketDomainInput>();
}
