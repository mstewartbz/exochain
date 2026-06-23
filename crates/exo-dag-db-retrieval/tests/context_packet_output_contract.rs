#![allow(clippy::expect_used, clippy::unwrap_used)]

use exo_core::Hash256;
use exo_dag_db_api::{
    DagDbContextPacketImportTrackingStatus, DagDbGraphContextPacketBuildRequest,
    DagDbGraphContextSelectionRequest, DagDbGraphContextSelectionStatus, MemoryEdgeKind,
    MemoryGraphStyle, SafeMetadata, SafeMetadataDecision, ValidationStatus,
};
use exo_dag_db_retrieval::{
    context_packet_output::build_graph_context_packet,
    graph::MemoryGraphEdge,
    graph_context_selection::{
        GraphContextMemoryCandidate, GraphContextSelectionState, select_graph_context,
    },
    scoring::DomainError,
};

const FORBIDDEN_FRAGMENTS: &[&str] = &[
    "/Users/",
    "DATABASE_URL",
    "PRIVATE KEY",
    ".env",
    "raw_markdown",
    "raw_body",
    "raw_private_payload",
    "source_excerpt",
    "postgres://",
    "file://",
];

const CITATION_LOCATOR_BLOCKED: &str = "omitted_citation_locator_blocked";

fn h(byte: u8) -> Hash256 {
    Hash256::from_bytes([byte; 32])
}

fn mem_id(byte: u8) -> String {
    h(byte).to_string()
}

fn safe(text: &str) -> SafeMetadata {
    SafeMetadata {
        decision: SafeMetadataDecision::Allow,
        text: text.into(),
        redaction_codes: Vec::new(),
        original_hash: h(0xee).to_string(),
        truncated: false,
        byte_len: u32::try_from(text.len()).expect("fixture fits"),
    }
}

fn candidate(
    byte: u8,
    document_type: &str,
    title: &str,
    summary: &str,
    catalog_path: &[&str],
    token_estimate: u32,
) -> GraphContextMemoryCandidate {
    let memory_id = mem_id(byte);
    GraphContextMemoryCandidate {
        memory_id,
        tenant_id: "tenant-a".into(),
        namespace: "primary".into(),
        catalog_id: Some(format!("catalog-{byte:02x}")),
        title: safe(title),
        summary: safe(summary),
        catalog_path: catalog_path.iter().map(|part| (*part).to_owned()).collect(),
        document_type: document_type.into(),
        token_estimate,
        validation_status: ValidationStatus::Passed,
        citation_ref: format!("citation:{byte:02x}"),
        boundary_flags: vec!["repository_test_only".into()],
    }
}

fn edge(from: u8, to: u8, graph_style: MemoryGraphStyle) -> MemoryGraphEdge {
    MemoryGraphEdge::new(
        "tenant-a".into(),
        "primary".into(),
        h(from),
        h(to),
        MemoryEdgeKind::DependsOn,
        graph_style,
        Some(h(0xab)),
    )
    .expect("edge")
}

fn base_state() -> GraphContextSelectionState {
    GraphContextSelectionState {
        memory_candidates: vec![
            candidate(
                0x01,
                "plan",
                "Next Steps",
                "Implementation plan for the next bounded M02 phase",
                &["04_Plans", "Next Steps"],
                180,
            ),
            candidate(
                0x02,
                "blocker",
                "Open blocker",
                "Open question blocking M02 ship",
                &["08_Open_Questions", "Blockers"],
                160,
            ),
            candidate(
                0x03,
                "summary",
                "Catalog contract",
                "General catalog contract summary",
                &["00_Index"],
                140,
            ),
        ],
        graph_edges: vec![
            edge(0x01, 0x03, MemoryGraphStyle::SemanticCatalogGraph),
            edge(0x02, 0x03, MemoryGraphStyle::ContradictionSupersessionGraph),
            edge(0x01, 0x02, MemoryGraphStyle::DependencyDag),
        ],
        receipt_ids: vec![h(0xaa)],
    }
}

fn base_selection_request(task: &str) -> DagDbGraphContextSelectionRequest {
    DagDbGraphContextSelectionRequest {
        tenant_id: "tenant-a".into(),
        namespace: "primary".into(),
        request_id: "req-packet-1".into(),
        task: task.into(),
        task_hash: h(0x31).to_string(),
        token_budget: 1_000,
        max_memory_refs: 4,
        catalog_hints: Vec::new(),
        requested_memory_ids: Vec::new(),
        force_revalidate: false,
    }
}

fn selection_for_task(task: &str) -> exo_dag_db_api::DagDbGraphContextSelectionResponse {
    select_graph_context(&base_selection_request(task), &base_state()).expect("selection")
}

fn packet_build_request(
    selection: exo_dag_db_api::DagDbGraphContextSelectionResponse,
    import_tracking_status: Option<DagDbContextPacketImportTrackingStatus>,
) -> DagDbGraphContextPacketBuildRequest {
    DagDbGraphContextPacketBuildRequest {
        tenant_id: selection.tenant_id.clone(),
        namespace: selection.namespace.clone(),
        request_id: selection.request_id.clone(),
        task: "Build bounded graph context packet for M02".into(),
        task_hash: selection.task_hash.clone(),
        audit_id: "audit-packet-contract".into(),
        token_budget: selection.token_budget,
        max_memory_refs: None,
        selection,
        import_tracking_status,
    }
}

fn import_tracking_status() -> DagDbContextPacketImportTrackingStatus {
    DagDbContextPacketImportTrackingStatus {
        manifest_json: "{\"tracked_clean_evidence\":true}".into(),
        manifest_status: "clean_manifest".into(),
        tracked_clean_evidence_enforced: true,
        source_path_status: CITATION_LOCATOR_BLOCKED.into(),
    }
}

fn assert_no_forbidden_output(serialized: &str) {
    for fragment in FORBIDDEN_FRAGMENTS {
        assert!(
            !serialized.contains(fragment),
            "output leaked forbidden fragment: {fragment}"
        );
    }
    assert!(!serialized.contains("\"source_path\":"));
}

fn assert_blocked_boundaries(packet: &exo_dag_db_api::DagDbGraphContextPacket) {
    assert!(packet.boundaries.repository_test_level_only);
    assert_eq!(packet.boundaries.production_runtime, "blocked");
    assert_eq!(packet.boundaries.default_context_replacement, "blocked");
    assert_eq!(packet.boundaries.billing_savings, "blocked");
    assert_eq!(
        packet.boundaries.citation_locator_status,
        CITATION_LOCATOR_BLOCKED
    );
    assert_eq!(packet.packet_metrics.end_to_end_savings_status, "blocked");
    assert_eq!(packet.packet_metrics.cost_savings_status, "blocked");
}

fn assert_validation_failed(request: &DagDbGraphContextPacketBuildRequest) {
    assert_eq!(
        build_graph_context_packet(request),
        Err(DomainError::ValidationFailed)
    );
}

#[test]
fn context_packet_output_contract_normal_build() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let packet = build_graph_context_packet(&packet_build_request(selection.clone(), None))
        .expect("normal packet");

    assert_eq!(packet.tenant_id, "tenant-a");
    assert_eq!(packet.request_id, selection.request_id);
    assert_eq!(
        packet.selected_memory_refs.len(),
        selection.selected_memory_refs.len()
    );
    assert_eq!(
        packet.selected_memory_refs, selection.selected_memory_refs,
        "packet must preserve M01 selected ref order"
    );
    assert_eq!(
        packet.citation_refs.len(),
        packet.selected_memory_refs.len()
    );
    for citation in &packet.citation_refs {
        assert_eq!(citation.citation_status, "metadata_only_no_locator");
    }

    let selected_ids = packet
        .selected_memory_refs
        .iter()
        .map(|selected| selected.memory_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for edge in &packet.selected_graph_edges {
        assert!(selected_ids.contains(&edge.from_memory_id));
        assert!(selected_ids.contains(&edge.to_memory_id));
    }
    assert_blocked_boundaries(&packet);
    assert_no_forbidden_output(&serde_json::to_string(&packet).expect("json"));
    assert_no_forbidden_output(&packet.markdown);
}

#[test]
fn context_packet_output_contract_empty_selection() {
    let mut selection = selection_for_task("What is the next implementation step for M02?");
    selection.selection_status = DagDbGraphContextSelectionStatus::Empty;
    selection.selected_memory_refs.clear();
    selection.selected_graph_edges.clear();
    selection.selected_token_estimate = 0;

    let packet =
        build_graph_context_packet(&packet_build_request(selection, None)).expect("empty packet");
    assert!(packet.selected_memory_refs.is_empty());
    assert!(packet.selected_graph_edges.is_empty());
    assert!(packet.citation_refs.is_empty());
    assert_eq!(packet.packet_metrics.selected_memory_ref_count, 0);
    assert!(packet.markdown.contains("## Selected Memory Refs"));
    assert!(packet.markdown.contains("- none"));
}

#[test]
fn context_packet_output_contract_import_tracking_present() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let packet = build_graph_context_packet(&packet_build_request(
        selection,
        Some(import_tracking_status()),
    ))
    .expect("import present");

    assert!(
        packet
            .agent_usage_instructions
            .iter()
            .any(|line| line.contains("import-tracking"))
    );
    assert!(packet.markdown.contains("import_manifest_status"));
    assert!(packet.markdown.contains(CITATION_LOCATOR_BLOCKED));
}

#[test]
fn context_packet_output_contract_import_tracking_absent() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let packet =
        build_graph_context_packet(&packet_build_request(selection, None)).expect("import absent");

    assert!(
        !packet
            .agent_usage_instructions
            .iter()
            .any(|line| line.contains("import-tracking"))
    );
    assert!(!packet.markdown.contains("import_manifest_status"));
}

#[test]
fn context_packet_output_contract_rejects_forbidden_material() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let mut request = packet_build_request(selection, None);
    request.task = "Leaked DATABASE_URL in task text".into();

    assert_validation_failed(&request);
}

#[test]
fn context_packet_output_contract_is_deterministic() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let request = packet_build_request(selection, None);
    let first = build_graph_context_packet(&request).expect("first");
    let second = build_graph_context_packet(&request).expect("second");
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("json"),
        serde_json::to_string(&second).expect("json")
    );
    assert_eq!(first.markdown, second.markdown);
    assert_eq!(first.packet_hash, second.packet_hash);
}

#[test]
fn context_packet_output_contract_rejects_token_budget_violation() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let mut request = packet_build_request(selection, None);
    request.token_budget = 1;
    assert_validation_failed(&request);
}

#[test]
fn context_packet_output_contract_rejects_max_memory_refs_violation() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let Some(template) = selection.selected_memory_refs.first().cloned() else {
        return;
    };
    let mut forged = selection;
    let mut first = template.clone();
    first.memory_id = mem_id(0xd1);
    first.citation_ref = "citation:max-memory-refs-1".into();
    first.token_estimate = 0;
    let mut second = template;
    second.memory_id = mem_id(0xd2);
    second.citation_ref = "citation:max-memory-refs-2".into();
    second.token_estimate = 0;
    forged.selected_memory_refs = vec![first, second];
    forged.selected_token_estimate = 0;

    let mut request = packet_build_request(forged, None);
    request.max_memory_refs = Some(1);
    assert_eq!(
        build_graph_context_packet(&request),
        Err(DomainError::ValidationFailed)
    );
}

#[test]
fn context_packet_output_contract_filters_edges_with_non_selected_endpoints() {
    let mut selection = selection_for_task("What is the next implementation step for M02?");
    if selection.selected_memory_refs.is_empty() {
        return;
    }
    let from = selection.selected_memory_refs[0].memory_id.clone();
    selection
        .selected_graph_edges
        .push(exo_dag_db_api::DagDbSelectedGraphEdgeRef {
            graph_edge_id: "edge-forged".into(),
            from_memory_id: from,
            to_memory_id: mem_id(0xff),
            edge_kind: MemoryEdgeKind::RelatedTo,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "forged_edge".into(),
        });

    let packet = build_graph_context_packet(&packet_build_request(selection, None))
        .expect("filtered packet");
    assert!(
        packet
            .selected_graph_edges
            .iter()
            .all(|edge| edge.to_memory_id != mem_id(0xff))
    );
}

#[test]
fn context_packet_output_contract_rejects_empty_task_and_zero_budget() {
    let selection = selection_for_task("What is the next implementation step for M02?");
    let mut empty_task = packet_build_request(selection.clone(), None);
    empty_task.task = "   ".into();
    assert_validation_failed(&empty_task);

    let mut zero_budget = packet_build_request(selection, None);
    zero_budget.token_budget = 0;
    assert_validation_failed(&zero_budget);
}

#[test]
fn context_packet_output_contract_rejects_selection_token_mismatches() {
    let selection = selection_for_task("What is the next implementation step for M02?");

    let mut selected_estimate_over_budget = packet_build_request(selection.clone(), None);
    selected_estimate_over_budget
        .selection
        .selected_token_estimate = selected_estimate_over_budget.token_budget + 1;
    assert_validation_failed(&selected_estimate_over_budget);

    let mut selection_budget_mismatch = packet_build_request(selection.clone(), None);
    selection_budget_mismatch.selection.token_budget += 1;
    assert_validation_failed(&selection_budget_mismatch);

    let mut selected_sum_mismatch = packet_build_request(selection, None);
    selected_sum_mismatch.selection.selected_token_estimate += 1;
    assert_validation_failed(&selected_sum_mismatch);
}

#[test]
fn context_packet_output_contract_rejects_selection_scope_mismatches() {
    let selection = selection_for_task("What is the next implementation step for M02?");

    let mut wrong_tenant = packet_build_request(selection.clone(), None);
    wrong_tenant.tenant_id = "tenant-b".into();
    assert_validation_failed(&wrong_tenant);

    let mut wrong_namespace = packet_build_request(selection.clone(), None);
    wrong_namespace.namespace = "secondary".into();
    assert_validation_failed(&wrong_namespace);

    let mut wrong_request_id = packet_build_request(selection.clone(), None);
    wrong_request_id.request_id = "req-packet-2".into();
    assert_validation_failed(&wrong_request_id);

    let mut wrong_task_hash = packet_build_request(selection, None);
    wrong_task_hash.task_hash = h(0x77).to_string();
    assert_validation_failed(&wrong_task_hash);
}

#[test]
fn context_packet_output_contract_rejects_selected_token_sum_overflow() {
    let mut selection = selection_for_task("What is the next implementation step for M02?");
    let Some(template) = selection.selected_memory_refs.first().cloned() else {
        return;
    };
    let mut first = template.clone();
    first.memory_id = mem_id(0xa1);
    first.citation_ref = "citation:overflow-a".into();
    first.token_estimate = u32::MAX;
    let mut second = template;
    second.memory_id = mem_id(0xa2);
    second.citation_ref = "citation:overflow-b".into();
    second.token_estimate = 1;
    selection.selected_memory_refs = vec![first, second];
    selection.selected_graph_edges.clear();
    selection.selected_token_estimate = u32::MAX;
    selection.token_budget = u32::MAX;

    assert_eq!(
        build_graph_context_packet(&packet_build_request(selection, None)),
        Err(DomainError::ArithmeticOverflow {
            operation: "context_packet_selected_token_sum"
        })
    );
}

#[test]
fn context_packet_output_contract_rejects_forbidden_selected_ref_fields() {
    let selection = selection_for_task("What is the next implementation step for M02?");

    let mut forbidden_catalog_path = selection.clone();
    forbidden_catalog_path.selected_memory_refs[0].catalog_path = vec!["raw_body".into()];
    assert_validation_failed(&packet_build_request(forbidden_catalog_path, None));

    let mut forbidden_document_type = selection.clone();
    forbidden_document_type.selected_memory_refs[0].document_type = "file://plan".into();
    assert_validation_failed(&packet_build_request(forbidden_document_type, None));

    let mut forbidden_selection_reason = selection.clone();
    forbidden_selection_reason.selected_memory_refs[0].selection_reason =
        "contains source_excerpt".into();
    assert_validation_failed(&packet_build_request(forbidden_selection_reason, None));

    let mut forbidden_citation_ref = selection.clone();
    forbidden_citation_ref.selected_memory_refs[0].citation_ref = "PRIVATE KEY citation".into();
    assert_validation_failed(&packet_build_request(forbidden_citation_ref, None));

    let mut forbidden_boundary_flag = selection.clone();
    forbidden_boundary_flag.selected_memory_refs[0].boundary_flags =
        vec!["Leaked .env fragment".into()];
    assert_validation_failed(&packet_build_request(forbidden_boundary_flag, None));

    let mut no_catalog_id = selection;
    no_catalog_id.selected_memory_refs[0].catalog_id = None;
    let packet =
        build_graph_context_packet(&packet_build_request(no_catalog_id, None)).expect("packet");
    assert!(packet.selected_memory_refs[0].catalog_id.is_none());
}

#[test]
fn context_packet_output_contract_rejects_forbidden_safe_metadata() {
    let selection = selection_for_task("What is the next implementation step for M02?");

    let mut forbidden_title = selection.clone();
    forbidden_title.selected_memory_refs[0].title.text = "DATABASE_URL title".into();
    assert_validation_failed(&packet_build_request(forbidden_title, None));

    let mut forbidden_summary_hash = selection;
    forbidden_summary_hash.selected_memory_refs[0]
        .summary
        .original_hash = "postgres://hash".into();
    assert_validation_failed(&packet_build_request(forbidden_summary_hash, None));
}

#[test]
fn context_packet_output_contract_rejects_forbidden_edge_and_import_fields() {
    let mut selection = selection_for_task("What is the next implementation step for M02?");
    let from = selection.selected_memory_refs[0].memory_id.clone();
    let to = selection.selected_memory_refs[1].memory_id.clone();
    selection
        .selected_graph_edges
        .push(exo_dag_db_api::DagDbSelectedGraphEdgeRef {
            graph_edge_id: "edge DATABASE_URL".into(),
            from_memory_id: from,
            to_memory_id: to,
            edge_kind: MemoryEdgeKind::RelatedTo,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "selected edge".into(),
        });
    assert_validation_failed(&packet_build_request(selection.clone(), None));

    let mut forbidden_edge_reason = selection.clone();
    forbidden_edge_reason.selected_graph_edges[0].graph_edge_id = "edge-safe".into();
    forbidden_edge_reason.selected_graph_edges[0].selection_reason = "raw_private_payload".into();
    assert_validation_failed(&packet_build_request(forbidden_edge_reason, None));

    let mut forbidden_manifest = import_tracking_status();
    forbidden_manifest.manifest_json = "{\"source_path\":\"/Users/max/project\"}".into();
    let clean_selection = selection_for_task("What is the next implementation step for M02?");
    build_graph_context_packet(&packet_build_request(
        clean_selection.clone(),
        Some(import_tracking_status()),
    ))
    .expect("clean import status");
    assert_validation_failed(&packet_build_request(
        clean_selection.clone(),
        Some(forbidden_manifest),
    ));

    let mut forbidden_manifest_status = import_tracking_status();
    forbidden_manifest_status.manifest_status = "raw_markdown present".into();
    assert_validation_failed(&packet_build_request(
        clean_selection.clone(),
        Some(forbidden_manifest_status),
    ));

    let mut forbidden_locator = import_tracking_status();
    forbidden_locator.source_path_status = "locator_ready".into();
    assert_validation_failed(&packet_build_request(
        clean_selection,
        Some(forbidden_locator),
    ));
}

#[test]
fn context_packet_output_contract_sorts_selected_edges_deterministically() {
    let mut selection = selection_for_task("What is the next implementation step for M02?");
    let first = selection.selected_memory_refs[0].memory_id.clone();
    let second = selection.selected_memory_refs[1].memory_id.clone();
    selection.selected_graph_edges = vec![
        exo_dag_db_api::DagDbSelectedGraphEdgeRef {
            graph_edge_id: "edge-b".into(),
            from_memory_id: second.clone(),
            to_memory_id: first.clone(),
            edge_kind: MemoryEdgeKind::RelatedTo,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "selected_edge".into(),
        },
        exo_dag_db_api::DagDbSelectedGraphEdgeRef {
            graph_edge_id: "edge-a".into(),
            from_memory_id: second,
            to_memory_id: first,
            edge_kind: MemoryEdgeKind::RelatedTo,
            graph_style: MemoryGraphStyle::DependencyDag,
            selection_reason: "selected_edge".into(),
        },
    ];

    let packet =
        build_graph_context_packet(&packet_build_request(selection, None)).expect("sorted packet");
    assert_eq!(packet.selected_graph_edges[0].graph_edge_id, "edge-a");
    assert_eq!(packet.selected_graph_edges[1].graph_edge_id, "edge-b");
}
