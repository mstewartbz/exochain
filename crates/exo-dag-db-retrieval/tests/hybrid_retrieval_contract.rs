#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use exo_dag_db_api::{
    DagDbGraphContextSelectionResponse, DagDbGraphContextSelectionStatus, DagDbSelectedContextRef,
    DagDbSelectedGraphEdgeRef, MemoryEdgeKind, MemoryGraphStyle, SafeMetadata,
    SafeMetadataDecision, ValidationStatus,
};
use exo_dag_db_retrieval::{
    hybrid_retrieval::{
        HYBRID_RETRIEVAL_CONTRACT_SCHEMA, HybridRetrievalError, HybridRetrievalGraphEdgeRef,
        HybridRetrievalOmittedRef, HybridRetrievalPreview, HybridRetrievalRequest,
        build_hybrid_retrieval_preview,
    },
    kg_retrieval::{
        KG_CONTEXT_PACKET_PREVIEW_SCHEMA, KgContextPacketPreview, KgGraphEdgeRef,
        KgGraphPathSummary, KgMemoryRef, KgRetrievalDiagnostics, KgValidationSummary,
    },
};

const FORBIDDEN_FRAGMENTS: &[&str] = &[
    "/Users/",
    "DATABASE_URL",
    "PRIVATE KEY",
    ".env",
    "raw_markdown",
    "raw_body",
    "source_path",
    "postgres://",
    "file://",
];

fn safe(text: &str) -> SafeMetadata {
    SafeMetadata {
        decision: SafeMetadataDecision::Allow,
        text: text.into(),
        redaction_codes: Vec::new(),
        original_hash: "fixture-original-hash".into(),
        truncated: false,
        byte_len: u32::try_from(text.len()).expect("fixture fits"),
    }
}

fn mem_id(label: &str) -> String {
    format!("memory-{label}")
}

fn kg_memory(label: &str, token_estimate: u32) -> KgMemoryRef {
    KgMemoryRef {
        memory_id: mem_id(label),
        catalog_id: Some(format!("kg-catalog-{label}")),
        source_path: None,
        catalog_path: vec!["04_Plans".into(), "Next Steps".into()],
        layer_id: None,
        layer_path: None,
        layer_depth: None,
        layer_kind: None,
        layer_membership_role: None,
        layer_selection_reason: None,
        rollup_summary_ref: None,
        title: safe(&format!("KG title {label}")),
        summary: safe(&format!("KG summary {label}")),
        latest_receipt_hash: "receipt-hash".into(),
        memory_status: "active".into(),
        validation_status: "passed".into(),
        risk_class: "R1".into(),
        council_status: "not_required".into(),
        dag_finality_status: "pending".into(),
        graph_node_ids: vec![],
        validation_report_ids: vec!["validation-report".into()],
        citation_handle: format!("kg-citation-{label}"),
        token_estimate,
        selection_reasons: vec!["kg_selected".into()],
    }
}

fn graph_memory(label: &str, token_estimate: u32) -> DagDbSelectedContextRef {
    DagDbSelectedContextRef {
        memory_id: mem_id(label),
        catalog_id: Some(format!("graph-catalog-{label}")),
        title: safe(&format!("Graph title {label}")),
        summary: safe(&format!("Graph summary {label}")),
        catalog_path: vec!["04_Plans".into(), "Next Steps".into()],
        document_type: "plan".into(),
        selection_reason: "graph_selected".into(),
        token_estimate,
        validation_status: ValidationStatus::Passed,
        citation_ref: format!("graph-citation-{label}"),
        boundary_flags: vec!["repository_test_only".into()],
    }
}

fn base_kg_preview(
    memory_refs: Vec<KgMemoryRef>,
    graph_edges: Vec<KgGraphEdgeRef>,
    token_budget: u32,
) -> KgContextPacketPreview {
    let selected_refs = memory_refs.clone();
    let selected_graph_edges = graph_edges.clone();
    let selected_memory_ref_count = u32::try_from(selected_refs.len()).expect("fixture count");
    let selected_graph_edge_count =
        u32::try_from(selected_graph_edges.len()).expect("fixture count");
    KgContextPacketPreview {
        schema_version: KG_CONTEXT_PACKET_PREVIEW_SCHEMA.to_owned(),
        tenant_id: "tenant-a".into(),
        namespace: "primary".into(),
        context_packet_id: "ctx-packet-1".into(),
        route_hint_id: "route-hint-1".into(),
        memory_refs,
        graph_edges,
        selected_refs,
        selected_layers: Vec::new(),
        selected_layer_edges: Vec::new(),
        selected_graph_edges,
        rollup_summaries: Vec::new(),
        budget_report: exo_dag_db_retrieval::kg_retrieval::KgLayerBudgetReport {
            max_layer_depth:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
            max_layers_selected:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
            max_nodes_per_layer:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
            max_memory_refs:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS,
            max_layer_edges:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
            selected_layer_count: 0,
            selected_layer_edge_count: 0,
            active_layer_edge_count: 0,
            excluded_demoted_layer_edge_count: 0,
            excluded_tombstoned_layer_edge_count: 0,
            selected_memory_ref_count,
            selected_graph_edge_count,
            depth_budget_truncated: false,
            layer_budget_truncated: false,
            node_budget_truncated: false,
            layer_edge_budget_truncated: false,
            token_budget_truncated: false,
            flat_fallback_used: true,
        },
        flat_fallback_used: true,
        citation_handles: Vec::new(),
        retrieval_diagnostics: KgRetrievalDiagnostics {
            selected_memory_count: 0,
            omitted_memory_count: 0,
            selected_graph_edge_count: 0,
            selected_layer_count: 0,
            selected_layer_edge_count: 0,
            active_layer_edge_count: 0,
            excluded_demoted_layer_edge_count: 0,
            excluded_tombstoned_layer_edge_count: 0,
            citation_handle_count: 0,
            warning_count: 0,
            token_budget,
            token_estimate: 0,
            max_layer_depth:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
            max_layers_selected:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED,
            max_nodes_per_layer:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER,
            max_layer_edges:
                exo_dag_db_retrieval::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
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
        validation_summary: KgValidationSummary {
            selected_memory_count: 0,
            pending_count: 0,
            passed_count: 0,
            failed_count: 0,
            needs_council_count: 0,
            warning_count: 0,
            validation_status_counts: BTreeMap::new(),
            risk_class_counts: BTreeMap::new(),
            dag_finality_status_counts: BTreeMap::new(),
            council_status_counts: BTreeMap::new(),
        },
        graph_path_summary: KgGraphPathSummary {
            graph_edge_count: 0,
            graph_styles_seen: Vec::new(),
            edge_kinds_seen: Vec::new(),
            isolated_memory_count: 0,
            connected_memory_count: 0,
            missing_edge_warning_count: 0,
        },
        citation_diagnostics: Vec::new(),
        token_budget,
        token_estimate: 0,
        omitted_memory_ids: Vec::new(),
        omitted_memory_refs: Vec::new(),
        warnings: Vec::new(),
        dry_run_or_preview_only: true,
    }
}

fn base_graph_selection(
    selected_memory_refs: Vec<DagDbSelectedContextRef>,
    selected_graph_edges: Vec<DagDbSelectedGraphEdgeRef>,
    token_budget: u32,
) -> DagDbGraphContextSelectionResponse {
    DagDbGraphContextSelectionResponse {
        tenant_id: "tenant-a".into(),
        namespace: "primary".into(),
        request_id: "req-1".into(),
        task_hash: "task-hash-1".into(),
        selection_status: DagDbGraphContextSelectionStatus::Selected,
        selected_memory_refs,
        selected_graph_edges,
        omitted_memory_refs: Vec::new(),
        selection_trace: Vec::new(),
        selected_token_estimate: 0,
        token_budget,
        boundary_warnings: Vec::new(),
    }
}

fn base_request(token_budget: u32, max_memory_refs: u32) -> HybridRetrievalRequest {
    HybridRetrievalRequest {
        tenant_id: "tenant-a".into(),
        namespace: "primary".into(),
        request_id: "req-1".into(),
        task_hash: "task-hash-1".into(),
        token_budget,
        max_memory_refs,
    }
}

fn valid_hybrid_preview() -> HybridRetrievalPreview {
    let kg_preview = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    let graph_selection = base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    build_hybrid_retrieval_preview(&base_request(1_000, 4), &kg_preview, &graph_selection)
        .expect("valid hybrid preview")
}

fn assert_invalid_request(
    result: Result<HybridRetrievalPreview, HybridRetrievalError>,
    fragment: &str,
) {
    let error = result.expect_err("request must fail");
    assert!(
        matches!(error, HybridRetrievalError::InvalidRequest { .. }),
        "expected InvalidRequest, got {error:?}"
    );
    assert!(
        error.to_string().contains(fragment),
        "expected fragment {fragment} in {error}"
    );
}

fn assert_invalid_evidence(
    result: Result<HybridRetrievalPreview, HybridRetrievalError>,
    fragment: &str,
) {
    let error = result.expect_err("evidence must fail");
    assert!(
        matches!(error, HybridRetrievalError::InvalidEvidence { .. }),
        "expected InvalidEvidence, got {error:?}"
    );
    assert!(
        error.to_string().contains(fragment),
        "expected fragment {fragment} in {error}"
    );
}

fn assert_preview_invalid(preview: &HybridRetrievalPreview, fragment: &str) {
    let error = preview.validate().expect_err("preview must fail");
    assert!(
        error
            .to_string()
            .to_ascii_lowercase()
            .contains(&fragment.to_ascii_lowercase()),
        "expected fragment {fragment} in {error}"
    );
}

#[test]
fn hybrid_retrieval_contract_selects_overlap_and_is_deterministic() {
    let kg_preview = base_kg_preview(
        vec![kg_memory("plan", 120), kg_memory("extra-kg", 100)],
        vec![KgGraphEdgeRef {
            graph_edge_id: "edge-1".into(),
            from_memory_id: mem_id("plan"),
            to_memory_id: mem_id("extra-kg"),
            edge_kind: "depends_on".into(),
            graph_style: "dependency_dag".into(),
            receipt_hash: None,
        }],
        1_000,
    );
    let graph_selection = base_graph_selection(
        vec![graph_memory("plan", 120), graph_memory("extra-graph", 100)],
        vec![DagDbSelectedGraphEdgeRef {
            graph_edge_id: "edge-2".into(),
            from_memory_id: mem_id("plan"),
            to_memory_id: mem_id("extra-graph"),
            edge_kind: MemoryEdgeKind::DependsOn,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "graph_selected".into(),
        }],
        1_000,
    );

    let request = base_request(1_000, 4);
    let first = build_hybrid_retrieval_preview(&request, &kg_preview, &graph_selection)
        .expect("hybrid preview");
    let second = build_hybrid_retrieval_preview(&request, &kg_preview, &graph_selection)
        .expect("hybrid preview again");

    assert_eq!(first, second, "hybrid preview must be deterministic");
    assert_eq!(first.schema_version, HYBRID_RETRIEVAL_CONTRACT_SCHEMA);
    assert_eq!(first.selected_memory_refs.len(), 1);
    assert_eq!(first.selected_memory_refs[0].memory_id, mem_id("plan"));
    assert!(
        first
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.reason == "kg_preview_missing"),
        "graph-only ref should be omitted"
    );
    assert!(
        first
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.reason == "graph_selection_missing"),
        "kg-only ref should be omitted"
    );
    assert!(first.acceptance.preview_only);
    assert!(!first.diagnostics.provider_calls_made);
}

#[test]
fn hybrid_retrieval_contract_truncates_by_token_budget_and_max_refs() {
    let kg_preview = base_kg_preview(
        vec![kg_memory("one", 400), kg_memory("two", 400)],
        Vec::new(),
        450,
    );
    let graph_selection = base_graph_selection(
        vec![graph_memory("one", 400), graph_memory("two", 400)],
        Vec::new(),
        450,
    );

    let token_preview =
        build_hybrid_retrieval_preview(&base_request(450, 4), &kg_preview, &graph_selection)
            .expect("token budget preview");
    assert!(
        token_preview
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.reason == "token_budget_exceeded")
    );

    let kg_preview_max = base_kg_preview(
        vec![kg_memory("one", 400), kg_memory("two", 400)],
        Vec::new(),
        1_000,
    );
    let graph_selection_max = base_graph_selection(
        vec![graph_memory("one", 400), graph_memory("two", 400)],
        Vec::new(),
        1_000,
    );

    let max_refs_preview = build_hybrid_retrieval_preview(
        &base_request(1_000, 1),
        &kg_preview_max,
        &graph_selection_max,
    )
    .expect("max refs preview");
    assert_eq!(max_refs_preview.selected_memory_refs.len(), 1);
    assert!(
        max_refs_preview
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.reason == "max_memory_refs_exceeded")
    );
}

#[test]
fn hybrid_retrieval_contract_preview_json_has_no_forbidden_material() {
    let kg_preview = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    let graph_selection = base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    let preview =
        build_hybrid_retrieval_preview(&base_request(1_000, 4), &kg_preview, &graph_selection)
            .expect("preview");
    let json = preview.to_canonical_json().expect("canonical json");
    for fragment in FORBIDDEN_FRAGMENTS {
        assert!(
            !json.contains(fragment),
            "forbidden fragment {fragment} leaked into preview json"
        );
    }
}

#[test]
fn hybrid_retrieval_contract_rejects_scope_mismatch() {
    let kg_preview = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    let mut graph_selection =
        base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    graph_selection.tenant_id = "other-tenant".into();
    let error =
        build_hybrid_retrieval_preview(&base_request(1_000, 4), &kg_preview, &graph_selection)
            .expect_err("tenant mismatch must fail");
    assert!(error.to_string().contains("tenant/namespace"));
}

#[test]
fn hybrid_retrieval_contract_rejects_invalid_request_fields() {
    let kg_preview = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    let graph_selection = base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);

    let mut empty_tenant = base_request(1_000, 4);
    empty_tenant.tenant_id = "   ".into();
    assert_invalid_request(
        build_hybrid_retrieval_preview(&empty_tenant, &kg_preview, &graph_selection),
        "tenant_id",
    );

    let mut zero_budget = base_request(1_000, 4);
    zero_budget.token_budget = 0;
    assert_invalid_request(
        build_hybrid_retrieval_preview(&zero_budget, &kg_preview, &graph_selection),
        "token_budget",
    );

    let mut zero_max_refs = base_request(1_000, 4);
    zero_max_refs.max_memory_refs = 0;
    assert_invalid_request(
        build_hybrid_retrieval_preview(&zero_max_refs, &kg_preview, &graph_selection),
        "max_memory_refs",
    );

    let mut forbidden_request = base_request(1_000, 4);
    forbidden_request.request_id = "leaked postgres:// request".into();
    assert_invalid_request(
        build_hybrid_retrieval_preview(&forbidden_request, &kg_preview, &graph_selection),
        "forbidden fragment",
    );
}

#[test]
fn hybrid_retrieval_contract_rejects_invalid_input_evidence() {
    let graph_selection = base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    let request = base_request(1_000, 4);

    let mut unsupported_schema = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    unsupported_schema.schema_version = "unsupported".into();
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &unsupported_schema, &graph_selection),
        "schema",
    );

    let mut not_preview_only = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    not_preview_only.dry_run_or_preview_only = false;
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &not_preview_only, &graph_selection),
        "preview-only",
    );

    let mut kg_diag_not_preview = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    kg_diag_not_preview.retrieval_diagnostics.preview_only = false;
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &kg_diag_not_preview, &graph_selection),
        "preview-only",
    );

    let mut raw_material = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    raw_material.retrieval_diagnostics.raw_markdown_returned = true;
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &raw_material, &graph_selection),
        "raw material",
    );

    let mut kg_tenant_mismatch = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    kg_tenant_mismatch.tenant_id = "other-tenant".into();
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &kg_tenant_mismatch, &graph_selection),
        "tenant/namespace",
    );

    let mut graph_namespace_mismatch =
        base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    graph_namespace_mismatch.namespace = "secondary".into();
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000),
            &graph_namespace_mismatch,
        ),
        "tenant/namespace",
    );

    let mut kg_namespace_mismatch =
        base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    kg_namespace_mismatch.namespace = "secondary".into();
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &kg_namespace_mismatch, &graph_selection),
        "tenant/namespace",
    );

    let mut graph_task_mismatch =
        base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    graph_task_mismatch.task_hash = "other-task".into();
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000),
            &graph_task_mismatch,
        ),
        "task_hash",
    );

    let mut kg_budget_mismatch = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    kg_budget_mismatch.token_budget = 999;
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &kg_budget_mismatch, &graph_selection),
        "token budgets",
    );

    let mut graph_budget_mismatch =
        base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);
    graph_budget_mismatch.token_budget = 999;
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000),
            &graph_budget_mismatch,
        ),
        "token budgets",
    );

    let mut source_path_present = kg_memory("plan", 120);
    source_path_present.source_path = Some("docs/plan.md".into());
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![source_path_present], Vec::new(), 1_000),
            &graph_selection,
        ),
        "source material",
    );

    let duplicate_kg = base_kg_preview(
        vec![kg_memory("plan", 120), kg_memory("plan", 130)],
        Vec::new(),
        1_000,
    );
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(&request, &duplicate_kg, &graph_selection),
        "duplicate kg memory ref",
    );

    let duplicate_graph = base_graph_selection(
        vec![graph_memory("plan", 120), graph_memory("plan", 130)],
        Vec::new(),
        1_000,
    );
    assert_invalid_evidence(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000),
            &duplicate_graph,
        ),
        "duplicate graph memory ref",
    );
}

#[test]
fn hybrid_retrieval_contract_rejects_forbidden_overlap_fields() {
    let request = base_request(1_000, 4);
    let graph_selection = base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000);

    let mut forbidden_kg = kg_memory("plan", 120);
    forbidden_kg.citation_handle = "postgres://citation".into();
    assert_invalid_request(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![forbidden_kg], Vec::new(), 1_000),
            &graph_selection,
        ),
        "forbidden fragment",
    );

    let mut empty_graph_reason = graph_memory("plan", 120);
    empty_graph_reason.selection_reason = "   ".into();
    assert_invalid_request(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000),
            &base_graph_selection(vec![empty_graph_reason], Vec::new(), 1_000),
        ),
        "graph_selection_reason",
    );

    let mut empty_kg_reason = kg_memory("plan", 120);
    empty_kg_reason.selection_reasons = vec!["   ".into()];
    assert_invalid_request(
        build_hybrid_retrieval_preview(
            &request,
            &base_kg_preview(vec![empty_kg_reason], Vec::new(), 1_000),
            &graph_selection,
        ),
        "kg_selection_reason",
    );
}

#[test]
fn hybrid_retrieval_contract_includes_agreed_graph_edges_and_enum_labels() {
    let kg_preview = base_kg_preview(
        vec![kg_memory("alpha", 100), kg_memory("beta", 100)],
        vec![KgGraphEdgeRef {
            graph_edge_id: "kg-edge".into(),
            from_memory_id: mem_id("alpha"),
            to_memory_id: mem_id("beta"),
            edge_kind: "depends_on".into(),
            graph_style: "dependency_dag".into(),
            receipt_hash: None,
        }],
        1_000,
    );
    let graph_edges = vec![
        DagDbSelectedGraphEdgeRef {
            graph_edge_id: "graph-edge-derived".into(),
            from_memory_id: mem_id("alpha"),
            to_memory_id: mem_id("beta"),
            edge_kind: MemoryEdgeKind::DerivedFrom,
            graph_style: MemoryGraphStyle::ProvenanceReceiptDag,
            selection_reason: "graph_selected".into(),
        },
        DagDbSelectedGraphEdgeRef {
            graph_edge_id: "graph-edge-semantic".into(),
            from_memory_id: mem_id("alpha"),
            to_memory_id: mem_id("beta"),
            edge_kind: MemoryEdgeKind::Supports,
            graph_style: MemoryGraphStyle::SemanticCatalogGraph,
            selection_reason: "graph_selected".into(),
        },
    ];
    let graph_selection = base_graph_selection(
        vec![graph_memory("alpha", 100), graph_memory("beta", 100)],
        graph_edges,
        1_000,
    );

    let preview =
        build_hybrid_retrieval_preview(&base_request(1_000, 4), &kg_preview, &graph_selection)
            .expect("hybrid preview with edges");
    assert_eq!(preview.selected_memory_refs.len(), 2);
    assert_eq!(preview.selected_graph_edges.len(), 3);
    assert!(
        preview
            .selected_graph_edges
            .iter()
            .any(|edge| edge.source == "kg_retrieval_preview")
    );
    assert!(preview.selected_graph_edges.iter().any(|edge| {
        edge.source == "graph_context_selection"
            && edge.edge_kind == "derived_from"
            && edge.graph_style == "provenance_receipt_dag"
    }));
    assert!(preview.selected_graph_edges.iter().any(|edge| {
        edge.source == "graph_context_selection"
            && edge.edge_kind == "supports"
            && edge.graph_style == "semantic_catalog_graph"
    }));
}

#[test]
fn hybrid_retrieval_contract_filters_edges_to_selected_subgraph() {
    let kg_preview = base_kg_preview(
        vec![kg_memory("alpha", 100)],
        vec![KgGraphEdgeRef {
            graph_edge_id: "kg-edge-unselected".into(),
            from_memory_id: mem_id("alpha"),
            to_memory_id: mem_id("beta"),
            edge_kind: "depends_on".into(),
            graph_style: "dependency_dag".into(),
            receipt_hash: None,
        }],
        1_000,
    );
    let graph_selection = base_graph_selection(
        vec![graph_memory("alpha", 100)],
        vec![DagDbSelectedGraphEdgeRef {
            graph_edge_id: "graph-edge-unselected".into(),
            from_memory_id: mem_id("alpha"),
            to_memory_id: mem_id("beta"),
            edge_kind: MemoryEdgeKind::DependsOn,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "graph_selected".into(),
        }],
        1_000,
    );

    let preview =
        build_hybrid_retrieval_preview(&base_request(1_000, 4), &kg_preview, &graph_selection)
            .expect("filtered edge preview");
    assert!(preview.selected_graph_edges.is_empty());
}

#[test]
fn hybrid_retrieval_contract_emits_both_truncation_warnings() {
    let kg_preview = base_kg_preview(
        vec![
            kg_memory("one", 300),
            kg_memory("two", 300),
            kg_memory("three", 200),
            kg_memory("four", 100),
        ],
        Vec::new(),
        500,
    );
    let graph_selection = base_graph_selection(
        vec![
            graph_memory("one", 300),
            graph_memory("two", 300),
            graph_memory("three", 200),
            graph_memory("four", 100),
        ],
        Vec::new(),
        500,
    );
    let preview =
        build_hybrid_retrieval_preview(&base_request(500, 2), &kg_preview, &graph_selection)
            .expect("dual truncation preview");
    assert!(
        preview
            .warnings
            .iter()
            .any(|w| w == "context_truncated_by_token_budget")
    );
    assert!(
        preview
            .warnings
            .iter()
            .any(|w| w == "context_truncated_by_max_memory_refs")
    );
}

#[test]
fn hybrid_retrieval_contract_overlap_allows_none_catalog_ids() {
    let mut kg = kg_memory("plan", 120);
    kg.catalog_id = None;
    let mut graph = graph_memory("plan", 120);
    graph.catalog_id = None;
    let preview = build_hybrid_retrieval_preview(
        &base_request(1_000, 4),
        &base_kg_preview(vec![kg], Vec::new(), 1_000),
        &base_graph_selection(vec![graph], Vec::new(), 1_000),
    )
    .expect("none catalog ids");
    assert!(preview.selected_memory_refs[0].kg_catalog_id.is_none());
    assert!(preview.selected_memory_refs[0].graph_catalog_id.is_none());
}

#[test]
fn hybrid_retrieval_contract_emits_no_overlap_and_input_warning_signals() {
    let kg_preview = base_kg_preview(vec![kg_memory("kg-only", 100)], Vec::new(), 1_000);
    let mut graph_selection =
        base_graph_selection(vec![graph_memory("graph-only", 100)], Vec::new(), 1_000);
    graph_selection.boundary_warnings = vec!["graph boundary warning".into()];

    let no_overlap =
        build_hybrid_retrieval_preview(&base_request(1_000, 4), &kg_preview, &graph_selection)
            .expect("no overlap preview");
    assert!(no_overlap.selected_memory_refs.is_empty());
    assert!(
        no_overlap
            .warnings
            .iter()
            .any(|w| w == "no_selected_hybrid_memory_refs")
    );
    assert!(no_overlap.warnings.iter().any(|w| w == "no_hybrid_overlap"));
    assert!(
        no_overlap
            .warnings
            .iter()
            .any(|w| w == "kg_refs_not_graph_selected")
    );
    assert!(
        no_overlap
            .warnings
            .iter()
            .any(|w| w == "graph_refs_missing_from_kg_preview")
    );
    assert!(
        no_overlap
            .warnings
            .iter()
            .any(|w| w == "graph_selection_warnings_present")
    );
    assert!(no_overlap.warnings.iter().any(|w| w == "no_hybrid_overlap"));

    let graph_only_inputs = build_hybrid_retrieval_preview(
        &base_request(1_000, 4),
        &base_kg_preview(Vec::new(), Vec::new(), 1_000),
        &base_graph_selection(vec![graph_memory("graph-only", 100)], Vec::new(), 1_000),
    )
    .expect("graph-only inputs");
    assert!(graph_only_inputs.selected_memory_refs.is_empty());
    assert!(
        !graph_only_inputs
            .warnings
            .iter()
            .any(|w| w == "no_hybrid_overlap")
    );

    let mut kg_with_warning = base_kg_preview(vec![kg_memory("plan", 120)], Vec::new(), 1_000);
    kg_with_warning.warnings = vec!["kg warning".into()];
    let overlap = build_hybrid_retrieval_preview(
        &base_request(1_000, 4),
        &kg_with_warning,
        &base_graph_selection(vec![graph_memory("plan", 120)], Vec::new(), 1_000),
    )
    .expect("overlap preview");
    assert!(
        overlap
            .warnings
            .iter()
            .any(|w| w == "kg_preview_warnings_present")
    );
}

#[test]
fn hybrid_retrieval_contract_preview_validate_rejects_forged_shape() {
    let mut preview = valid_hybrid_preview();
    preview.schema_version = "forged".into();
    assert_preview_invalid(&preview, "schema_version");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.hybrid_selected_memory_count = 99;
    assert_preview_invalid(&preview, "diagnostic counts");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.selected_graph_edge_count = 99;
    assert_preview_invalid(&preview, "diagnostic counts");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.omitted_memory_count = 99;
    assert_preview_invalid(&preview, "diagnostic counts");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.deterministic_ordering = false;
    assert_preview_invalid(&preview, "boundary flags");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.preview_only = false;
    assert_preview_invalid(&preview, "boundary flags");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.provider_calls_made = true;
    assert_preview_invalid(&preview, "boundary flags");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.live_database_required = true;
    assert_preview_invalid(&preview, "boundary flags");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.raw_material_returned = true;
    assert_preview_invalid(&preview, "boundary flags");

    let mut preview = valid_hybrid_preview();
    preview.acceptance.preview_only = false;
    assert_preview_invalid(&preview, "acceptance flags");

    let mut preview = valid_hybrid_preview();
    preview.acceptance.provider_retrieval_implemented = true;
    assert_preview_invalid(&preview, "acceptance flags");

    let mut preview = valid_hybrid_preview();
    preview.acceptance.live_database_reads_implemented = true;
    assert_preview_invalid(&preview, "acceptance flags");

    let mut preview = valid_hybrid_preview();
    preview.acceptance.live_database_writes_implemented = true;
    assert_preview_invalid(&preview, "acceptance flags");

    let mut preview = valid_hybrid_preview();
    preview.acceptance.route_activation_implemented = true;
    assert_preview_invalid(&preview, "acceptance flags");

    let mut preview = valid_hybrid_preview();
    preview.acceptance.raw_material_returned = true;
    assert_preview_invalid(&preview, "acceptance flags");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.token_estimate = preview.diagnostics.token_budget + 1;
    assert_preview_invalid(&preview, "token_estimate exceeds");

    let mut preview = valid_hybrid_preview();
    preview.diagnostics.max_memory_refs = 0;
    assert_preview_invalid(&preview, "max_memory_refs");
}

#[test]
fn hybrid_retrieval_contract_preview_validate_rejects_duplicate_and_invalid_refs() {
    let mut preview = valid_hybrid_preview();
    let duplicate = preview.selected_memory_refs[0].clone();
    preview.selected_memory_refs.push(duplicate);
    assert_preview_invalid(&preview, "duplicate selected");

    let mut preview = valid_hybrid_preview();
    preview.omitted_memory_refs.push(HybridRetrievalOmittedRef {
        memory_id: mem_id("dup"),
        reason: "graph_selection_missing".into(),
        token_estimate_if_selected: Some(10),
    });
    preview.omitted_memory_refs.push(HybridRetrievalOmittedRef {
        memory_id: mem_id("dup"),
        reason: "kg_preview_missing".into(),
        token_estimate_if_selected: Some(10),
    });
    assert_preview_invalid(&preview, "duplicate omitted");

    let mut preview = valid_hybrid_preview();
    preview.selected_memory_refs[0].selection_reasons.clear();
    assert_preview_invalid(&preview, "selection_reason");

    let mut preview = valid_hybrid_preview();
    preview.selected_memory_refs[0].kg_token_estimate = 10;
    preview.selected_memory_refs[0].graph_token_estimate = 30;
    preview.selected_memory_refs[0].hybrid_token_estimate = 20;
    assert_preview_invalid(&preview, "hybrid_token_estimate must cover");

    let mut preview = valid_hybrid_preview();
    preview
        .selected_graph_edges
        .push(HybridRetrievalGraphEdgeRef {
            graph_edge_id: "   ".into(),
            from_memory_id: mem_id("plan"),
            to_memory_id: mem_id("plan"),
            edge_kind: "depends_on".into(),
            graph_style: "dependency_dag".into(),
            source: "forged".into(),
        });
    assert_preview_invalid(&preview, "graph_edge_id");

    let mut preview = valid_hybrid_preview();
    preview.warnings.push("   ".into());
    assert_preview_invalid(&preview, "warning");

    let mut preview = valid_hybrid_preview();
    preview.selected_memory_refs[0].selection_reasons = vec!["dup".into(), "dup".into()];
    assert_preview_invalid(&preview, "duplicate selection_reason");
}

#[test]
fn hybrid_retrieval_contract_covers_graph_edge_kind_and_style_labels() {
    let graph_edges = [
        (
            MemoryEdgeKind::DerivedFrom,
            MemoryGraphStyle::ProvenanceReceiptDag,
        ),
        (
            MemoryEdgeKind::Summarizes,
            MemoryGraphStyle::CanonicalMemoryGraph,
        ),
        (
            MemoryEdgeKind::Supports,
            MemoryGraphStyle::SemanticCatalogGraph,
        ),
        (
            MemoryEdgeKind::Contradicts,
            MemoryGraphStyle::SimilarityOverlayGraph,
        ),
        (MemoryEdgeKind::Supersedes, MemoryGraphStyle::DependencyDag),
        (MemoryEdgeKind::Replaces, MemoryGraphStyle::RoutingViewGraph),
        (
            MemoryEdgeKind::DuplicateOf,
            MemoryGraphStyle::ContradictionSupersessionGraph,
        ),
        (
            MemoryEdgeKind::NearDuplicateOf,
            MemoryGraphStyle::ContextPacketGraph,
        ),
        (MemoryEdgeKind::RelatedTo, MemoryGraphStyle::DependencyDag),
        (
            MemoryEdgeKind::AlternativeSummaryOf,
            MemoryGraphStyle::SemanticCatalogGraph,
        ),
        (MemoryEdgeKind::DependsOn, MemoryGraphStyle::DependencyDag),
        (
            MemoryEdgeKind::PartOf,
            MemoryGraphStyle::CanonicalMemoryGraph,
        ),
        (MemoryEdgeKind::OwnedBy, MemoryGraphStyle::RoutingViewGraph),
        (
            MemoryEdgeKind::AccessGrantedBy,
            MemoryGraphStyle::ProvenanceReceiptDag,
        ),
        (
            MemoryEdgeKind::VerifiedBy,
            MemoryGraphStyle::CanonicalMemoryGraph,
        ),
        (
            MemoryEdgeKind::UsedByRoute,
            MemoryGraphStyle::RoutingViewGraph,
        ),
        (
            MemoryEdgeKind::IncludedInContextPacket,
            MemoryGraphStyle::ContextPacketGraph,
        ),
        (
            MemoryEdgeKind::RevokedBy,
            MemoryGraphStyle::ContradictionSupersessionGraph,
        ),
    ];

    let mut edge_id = 0u8;
    let graph_edges: Vec<DagDbSelectedGraphEdgeRef> = graph_edges
        .into_iter()
        .map(|(edge_kind, graph_style)| {
            edge_id += 1;
            DagDbSelectedGraphEdgeRef {
                graph_edge_id: format!("graph-edge-{edge_id}"),
                from_memory_id: mem_id("alpha"),
                to_memory_id: mem_id("beta"),
                edge_kind,
                graph_style,
                selection_reason: "graph_selected".into(),
            }
        })
        .collect();
    let edge_count = graph_edges.len();

    let preview = build_hybrid_retrieval_preview(
        &base_request(10_000, 8),
        &base_kg_preview(
            vec![kg_memory("alpha", 50), kg_memory("beta", 50)],
            Vec::new(),
            10_000,
        ),
        &base_graph_selection(
            vec![graph_memory("alpha", 50), graph_memory("beta", 50)],
            graph_edges,
            10_000,
        ),
    )
    .expect("label coverage preview");

    assert_eq!(preview.selected_graph_edges.len(), edge_count);
}

#[test]
fn hybrid_retrieval_contract_preview_validate_rejects_forbidden_field_values() {
    let mut preview = valid_hybrid_preview();
    preview.namespace = "file://bad".into();
    let error = preview.validate().expect_err("forbidden namespace");
    assert!(
        matches!(error, HybridRetrievalError::InvalidRequest { .. }),
        "expected InvalidRequest, got {error:?}"
    );
    assert!(error.to_string().contains("forbidden fragment"));

    let mut preview = valid_hybrid_preview();
    preview.selected_memory_refs[0].kg_catalog_id = Some("   ".into());
    assert_preview_invalid(&preview, "kg_catalog_id");
}
