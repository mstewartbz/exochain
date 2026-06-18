#![allow(clippy::expect_used, clippy::unwrap_used)]

use exo_core::Hash256;
use exo_dag_db_api::{
    DagDbGraphContextSelectionRequest, DagDbGraphContextSelectionStatus, MemoryEdgeKind,
    MemoryGraphStyle, SafeMetadata, SafeMetadataDecision, ValidationStatus,
};
use exo_dag_db_retrieval::{
    graph::MemoryGraphEdge,
    graph_context_selection::{
        GraphContextMemoryCandidate, GraphContextSelectionState, MAX_FAMILY_REF_SHARE,
        MAX_SELECTED_GRAPH_EDGES_PER_PACKET, TaskBudgetClass, classify_task_budget_class,
        select_graph_context, task_budget_tokens,
    },
    query::graph_route_planner_order,
    scoring::DomainError,
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

fn scoped_edge(
    tenant_id: &str,
    namespace: &str,
    from: u8,
    to: u8,
    graph_style: MemoryGraphStyle,
) -> MemoryGraphEdge {
    MemoryGraphEdge::new(
        tenant_id.into(),
        namespace.into(),
        h(from),
        h(to),
        MemoryEdgeKind::DependsOn,
        graph_style,
        Some(h(0xac)),
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
                "Implementation plan for the next bounded M01 phase",
                &["04_Plans", "Next Steps"],
                180,
            ),
            candidate(
                0x02,
                "blocker",
                "Open blocker",
                "Open question blocking M01 ship",
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

fn base_request(task: &str) -> DagDbGraphContextSelectionRequest {
    DagDbGraphContextSelectionRequest {
        tenant_id: "tenant-a".into(),
        namespace: "primary".into(),
        request_id: "req-contract-1".into(),
        task: task.into(),
        task_hash: h(0x21).to_string(),
        token_budget: 1_000,
        max_memory_refs: 4,
        catalog_hints: Vec::new(),
        requested_memory_ids: Vec::new(),
        force_revalidate: false,
    }
}

#[test]
fn graph_context_selection_contract_next_step_task_selects_plan_like_refs() {
    let response = select_graph_context(
        &base_request("What is the next implementation step for M01?"),
        &base_state(),
    )
    .expect("next-step selection");

    assert_eq!(
        response.selection_status,
        DagDbGraphContextSelectionStatus::Selected
    );
    assert!(
        response
            .selected_memory_refs
            .iter()
            .any(|selected| selected.memory_id == mem_id(0x01)),
        "next-step task should select the plan-like memory"
    );
    assert!(
        response
            .selected_memory_refs
            .first()
            .is_some_and(|selected| selected.document_type == "plan"),
        "plan-like ref should rank first for next-step task"
    );
}

#[test]
fn graph_context_selection_contract_blocker_task_selects_blocker_like_refs() {
    let response = select_graph_context(
        &base_request("What blocker prevents shipping M01?"),
        &base_state(),
    )
    .expect("blocker selection");

    assert_eq!(
        response.selection_status,
        DagDbGraphContextSelectionStatus::Selected
    );
    assert!(
        response
            .selected_memory_refs
            .iter()
            .any(|selected| selected.memory_id == mem_id(0x02)),
        "blocker task should select blocker-like memory"
    );
    assert!(
        response
            .selected_memory_refs
            .first()
            .is_some_and(|selected| selected.document_type == "blocker"),
        "blocker-like ref should rank first for blocker task"
    );
}

#[test]
fn graph_context_selection_contract_task_aliases_boost_expected_document_types() {
    for task in [
        "What is the next step?",
        "What is the next-step?",
        "What is the implementation step?",
    ] {
        let response = select_graph_context(&base_request(task), &base_state())
            .expect("next-step alias selection");
        assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x01));
        assert!(
            response.selected_memory_refs[0]
                .selection_reason
                .contains("next_step_document_type"),
            "next-step alias should boost plan refs: {task}"
        );
    }

    for task in ["What is blocking M01?", "What open question remains?"] {
        let response =
            select_graph_context(&base_request(task), &base_state()).expect("blocker alias");
        assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x02));
        assert!(
            response.selected_memory_refs[0]
                .selection_reason
                .contains("blocker_document_type"),
            "blocker alias should boost blocker refs: {task}"
        );
    }
}

#[test]
fn graph_context_selection_contract_requested_memory_overrides_weaker_task_matching() {
    let mut request = base_request("What is the next implementation step for M01?");
    request.requested_memory_ids = vec![mem_id(0x03)];

    let response = select_graph_context(&request, &base_state()).expect("requested override");

    assert_eq!(response.selected_memory_refs.len(), 1);
    assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x03));
    assert!(
        response.selected_memory_refs[0]
            .selection_reason
            .contains("requested_memory_id")
    );
}

#[test]
fn graph_context_selection_contract_catalog_hints_match_path_segments_and_catalog_ids() {
    let mut state = base_state();
    state.memory_candidates[2].catalog_id = Some("decision-catalog".into());

    let mut path_request = base_request("unrelated task text");
    path_request.catalog_hints = vec!["/04_Plans/Next Steps/".into()];
    let path_response = select_graph_context(&path_request, &state).expect("path hint selection");
    assert_eq!(
        path_response.selected_memory_refs[0].memory_id,
        mem_id(0x01)
    );
    assert!(
        path_response.selected_memory_refs[0]
            .selection_reason
            .contains("catalog_hint_match")
    );

    let mut catalog_id_request = base_request("unrelated task text");
    catalog_id_request.catalog_hints = vec!["decision-catalog".into()];
    let catalog_id_response =
        select_graph_context(&catalog_id_request, &state).expect("catalog-id hint selection");
    assert_eq!(
        catalog_id_response.selected_memory_refs[0].memory_id,
        mem_id(0x03)
    );
    assert!(
        catalog_id_response.selected_memory_refs[0]
            .selection_reason
            .contains("catalog_hint_match")
    );
}

#[test]
fn graph_context_selection_contract_token_budget_truncation_omits_with_reason() {
    let mut request = base_request("What is the next implementation step for M01?");
    request.token_budget = 200;
    request.max_memory_refs = 4;

    let response = select_graph_context(&request, &base_state()).expect("token truncation");

    assert!(
        response
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.omission_reason == "token_budget_exceeded"),
        "token-budget truncation must emit token_budget_exceeded omissions"
    );
    assert!(
        response
            .boundary_warnings
            .contains(&"context_truncated_by_token_budget".to_owned())
    );
    assert!(response.selected_token_estimate <= response.token_budget);
}

#[test]
fn graph_context_selection_contract_max_refs_truncation_omits_with_reason() {
    let mut request = base_request("General catalog contract summary");
    request.max_memory_refs = 1;

    let response = select_graph_context(&request, &base_state()).expect("max refs truncation");

    assert_eq!(response.selected_memory_refs.len(), 1);
    assert!(
        response
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.omission_reason == "max_memory_refs_exceeded")
    );
    assert!(
        response
            .boundary_warnings
            .contains(&"context_truncated_by_max_memory_refs".to_owned())
    );
}

#[test]
fn graph_context_selection_contract_is_deterministic() {
    let request = base_request("What is the next implementation step for M01?");
    let state = base_state();
    let first = select_graph_context(&request, &state).expect("first");
    let second = select_graph_context(&request, &state).expect("second");
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("json"),
        serde_json::to_string(&second).expect("json")
    );
}

#[test]
fn graph_context_selection_contract_trace_matches_route_planner_order() {
    let response = select_graph_context(
        &base_request("What is the next implementation step for M01?"),
        &base_state(),
    )
    .expect("selection");

    assert_eq!(
        response
            .selection_trace
            .iter()
            .map(|step| step.graph_style)
            .collect::<Vec<_>>(),
        graph_route_planner_order().to_vec()
    );
}

#[test]
fn graph_context_selection_contract_trace_counts_all_candidates_when_no_edges_exist() {
    let mut state = base_state();
    state.graph_edges.clear();

    let response = select_graph_context(&base_request("General catalog contract summary"), &state)
        .expect("selection without edges");

    assert!(response.selected_graph_edges.is_empty());
    assert!(
        response
            .boundary_warnings
            .contains(&"selected_graph_edges_empty".to_owned())
    );
    for step in &response.selection_trace {
        assert_eq!(step.candidate_count_before, 3);
        assert_eq!(step.candidate_count_after, 3);
    }
}

#[test]
fn graph_context_selection_contract_selected_edge_endpoints_are_selected_memory_ids() {
    let response = select_graph_context(
        &base_request("What is the next implementation step for M01?"),
        &base_state(),
    )
    .expect("selection");

    let selected_ids = response
        .selected_memory_refs
        .iter()
        .map(|selected| selected.memory_id.clone())
        .collect::<std::collections::BTreeSet<_>>();

    for edge in &response.selected_graph_edges {
        assert!(selected_ids.contains(&edge.from_memory_id));
        assert!(selected_ids.contains(&edge.to_memory_id));
    }
}

#[test]
fn graph_context_selection_contract_selected_edges_ignore_unselected_endpoints() {
    let state = GraphContextSelectionState {
        memory_candidates: vec![
            candidate(
                0x01,
                "plan",
                "Next Steps",
                "Implementation plan for the next bounded M01 phase",
                &["04_Plans"],
                100,
            ),
            candidate(
                0x02,
                "blocker",
                "Open blocker",
                "Open question blocking M01 ship",
                &["08_Open_Questions"],
                100,
            ),
        ],
        graph_edges: vec![
            edge(0x01, 0x02, MemoryGraphStyle::DependencyDag),
            edge(0x01, 0x03, MemoryGraphStyle::SemanticCatalogGraph),
        ],
        receipt_ids: Vec::new(),
    };
    let response = select_graph_context(&base_request("General catalog contract summary"), &state)
        .expect("selection");

    assert_eq!(response.selected_graph_edges.len(), 1);
    assert_eq!(
        response.selected_graph_edges[0].from_memory_id,
        mem_id(0x01)
    );
    assert_eq!(response.selected_graph_edges[0].to_memory_id, mem_id(0x02));
}

#[test]
fn graph_context_selection_contract_caps_selected_edges_per_packet() {
    let mut state = GraphContextSelectionState {
        memory_candidates: (1..=5)
            .map(|byte| {
                candidate(
                    byte,
                    "plan",
                    "Dense graph memory",
                    "Dense graph memory used to prove selected edge cap",
                    &["04_Plans"],
                    50,
                )
            })
            .collect(),
        graph_edges: Vec::new(),
        receipt_ids: Vec::new(),
    };
    for from in 1..=5 {
        for to in 1..=5 {
            if from != to {
                state
                    .graph_edges
                    .push(edge(from, to, MemoryGraphStyle::DependencyDag));
            }
        }
    }
    let mut request = base_request("Dense graph packet budget");
    request.max_memory_refs = 5;

    let response = select_graph_context(&request, &state).expect("dense graph selection");

    assert_eq!(
        response.selected_graph_edges.len(),
        MAX_SELECTED_GRAPH_EDGES_PER_PACKET
    );
    assert!(
        response
            .boundary_warnings
            .contains(&"selected_graph_edges_truncated_by_budget".to_owned())
    );
}

#[test]
fn graph_context_selection_contract_rejects_toxic_forbidden_material() {
    for fragment in FORBIDDEN_FRAGMENTS {
        let mut state = base_state();
        state.memory_candidates[0].title = safe(&format!("Leaked path {fragment}"));
        assert_eq!(
            select_graph_context(
                &base_request("What is the next implementation step for M01?"),
                &state,
            ),
            Err(DomainError::ValidationFailed),
            "title containing forbidden fragment must fail closed: {fragment}"
        );

        let mut state = base_state();
        state.memory_candidates[0].summary = safe(&format!("Leaked secret {fragment}"));
        assert_eq!(
            select_graph_context(
                &base_request("What is the next implementation step for M01?"),
                &state,
            ),
            Err(DomainError::ValidationFailed),
            "summary containing forbidden fragment must fail closed: {fragment}"
        );
    }
}

#[test]
fn graph_context_selection_contract_rejects_forbidden_material_case_insensitively() {
    let mut state = base_state();
    state.memory_candidates[0].summary.text =
        "contains database_url and private key material".to_owned();

    assert!(
        select_graph_context(&base_request("General catalog contract summary"), &state).is_err()
    );
}

#[test]
fn graph_context_selection_contract_rejects_forbidden_material_in_all_metadata_fields() {
    for mutate in [
        |candidate: &mut GraphContextMemoryCandidate| {
            candidate.catalog_path = vec!["source_path".into()]
        },
        |candidate: &mut GraphContextMemoryCandidate| candidate.document_type = "raw_body".into(),
        |candidate: &mut GraphContextMemoryCandidate| {
            candidate.citation_ref = "file://citation".into()
        },
        |candidate: &mut GraphContextMemoryCandidate| {
            candidate.catalog_id = Some("postgres://catalog".into())
        },
        |candidate: &mut GraphContextMemoryCandidate| {
            candidate.boundary_flags = vec!["PRIVATE KEY".into()]
        },
    ] {
        let mut state = base_state();
        mutate(&mut state.memory_candidates[0]);
        assert_eq!(
            select_graph_context(
                &base_request("What is the next implementation step for M01?"),
                &state,
            ),
            Err(DomainError::ValidationFailed)
        );
    }
}

#[test]
fn graph_context_selection_contract_allows_candidate_without_catalog_id() {
    let mut state = base_state();
    state.memory_candidates[0].catalog_id = None;

    let response = select_graph_context(
        &base_request("What is the next implementation step for M01?"),
        &state,
    )
    .expect("selection without catalog id");

    let selected = response
        .selected_memory_refs
        .iter()
        .find(|selected| selected.memory_id == mem_id(0x01))
        .expect("selected plan ref");
    assert!(selected.catalog_id.is_none());
}

#[test]
fn graph_context_selection_contract_forbidden_output_is_not_serialized() {
    let response = select_graph_context(
        &base_request("What is the next implementation step for M01?"),
        &base_state(),
    )
    .expect("selection");

    let serialized = serde_json::to_string(&response).expect("serialize response");
    for fragment in FORBIDDEN_FRAGMENTS {
        assert!(
            !serialized.contains(fragment),
            "serialized response leaked forbidden fragment: {fragment}"
        );
    }
    assert!(!serialized.contains("\"source_path\""));
    assert!(!serialized.contains("\"raw_markdown\""));
    assert!(!serialized.contains("\"raw_body\""));
}

#[test]
fn graph_context_selection_contract_rejects_tenant_namespace_mismatch() {
    let mut state = base_state();
    state.memory_candidates[0].namespace = "other".into();

    assert!(matches!(
        select_graph_context(
            &base_request("What is the next implementation step for M01?"),
            &state
        ),
        Err(DomainError::TenantScopeMismatch { .. })
    ));
}

#[test]
fn graph_context_selection_contract_rejects_edge_scope_mismatches() {
    let mut tenant_state = base_state();
    tenant_state.graph_edges = vec![scoped_edge(
        "tenant-b",
        "primary",
        0x01,
        0x02,
        MemoryGraphStyle::DependencyDag,
    )];
    assert!(matches!(
        select_graph_context(
            &base_request("What is the next implementation step for M01?"),
            &tenant_state
        ),
        Err(DomainError::TenantScopeMismatch { .. })
    ));

    let mut namespace_state = base_state();
    namespace_state.graph_edges = vec![scoped_edge(
        "tenant-a",
        "other",
        0x01,
        0x02,
        MemoryGraphStyle::DependencyDag,
    )];
    assert!(matches!(
        select_graph_context(
            &base_request("What is the next implementation step for M01?"),
            &namespace_state
        ),
        Err(DomainError::TenantScopeMismatch { .. })
    ));
}

#[test]
fn graph_context_selection_contract_rejects_empty_task_zero_budget_and_zero_max_refs() {
    let state = base_state();

    let mut empty_task = base_request("task");
    empty_task.task.clear();
    assert_eq!(
        select_graph_context(&empty_task, &state),
        Err(DomainError::ValidationFailed)
    );

    let mut zero_budget = base_request("task");
    zero_budget.token_budget = 0;
    assert_eq!(
        select_graph_context(&zero_budget, &state),
        Err(DomainError::ValidationFailed)
    );

    let mut zero_refs = base_request("task");
    zero_refs.max_memory_refs = 0;
    assert_eq!(
        select_graph_context(&zero_refs, &state),
        Err(DomainError::ValidationFailed)
    );
}

#[test]
fn graph_context_selection_contract_rejects_duplicate_and_missing_requested_memory_ids() {
    let mut duplicate_state = base_state();
    duplicate_state
        .memory_candidates
        .push(duplicate_state.memory_candidates[0].clone());
    assert_eq!(
        select_graph_context(&base_request("task"), &duplicate_state),
        Err(DomainError::ValidationFailed)
    );

    let mut missing_requested = base_request("task");
    missing_requested.requested_memory_ids = vec![mem_id(0xff)];
    assert_eq!(
        select_graph_context(&missing_requested, &base_state()),
        Err(DomainError::ValidationFailed)
    );
}

#[test]
fn graph_context_selection_contract_empty_state_returns_empty_status_and_warning() {
    let state = GraphContextSelectionState::default();

    let response = select_graph_context(&base_request("valid task"), &state)
        .expect("empty graph context selection");

    assert_eq!(
        response.selection_status,
        DagDbGraphContextSelectionStatus::Empty
    );
    assert!(response.selected_memory_refs.is_empty());
    assert!(
        response
            .boundary_warnings
            .contains(&"no_selected_memory_refs".to_owned())
    );
}

#[test]
fn graph_context_selection_contract_end_to_end_preview_only_boundaries() {
    let request = base_request("What is the next implementation step for M01?");
    let response = select_graph_context(&request, &base_state()).expect("e2e selection");

    assert!(
        response.boundary_warnings.iter().any(|warning| {
            warning.contains("not_approved") || warning.contains("not_required")
        })
    );
    assert!(!response.selected_memory_refs.is_empty());
    assert_eq!(response.request_id, request.request_id);
    assert_eq!(response.task_hash, request.task_hash);
}

// ---- Q2-S1 budget class contract goldens --------------------------------

#[test]
fn graph_context_selection_contract_budget_class_goldens() {
    let cases = [
        (
            "Navigate the repo to find the writeback surface",
            TaskBudgetClass::Navigation,
            2_048u32,
        ),
        (
            "Implement the code change to patch selection scoring",
            TaskBudgetClass::CodeChange,
            4_096,
        ),
        (
            "Debug the failing context packet and diagnose the error",
            TaskBudgetClass::Debugging,
            8_192,
        ),
        (
            "Plan the next implementation step sequence",
            TaskBudgetClass::Planning,
            4_096,
        ),
        (
            "Review evidence and proof for the import audit",
            TaskBudgetClass::EvidenceReview,
            8_192,
        ),
        (
            "Prepare a handoff continuation status summary",
            TaskBudgetClass::Handoff,
            2_048,
        ),
    ];
    for (task, expected_class, expected_budget) in cases {
        assert_eq!(classify_task_budget_class(task), expected_class, "{task}");
        assert_eq!(task_budget_tokens(task), expected_budget, "{task}");
    }
    // Unmatched task defaults to navigation floor.
    assert_eq!(
        classify_task_budget_class("zzz qqq vvv"),
        TaskBudgetClass::Navigation
    );
    assert_eq!(task_budget_tokens("zzz qqq vvv"), 2_048);
}

// ---- Q2-S2 coverage scoring contract ------------------------------------

#[test]
fn graph_context_selection_contract_coverage_term_ranks_identifier_match_first() {
    // Two refs with otherwise-equal base score (no task-term/hint/doc-type
    // signal). The one whose summary contains the task identifier `token_budget`
    // ranks above the non-matching ref via the additive coverage term.
    let state = GraphContextSelectionState {
        memory_candidates: vec![
            candidate(
                0x10,
                "summary",
                "Match ref",
                "mentions the token_budget knob specifically",
                &["00_Index"],
                10,
            ),
            candidate(
                0x11,
                "summary",
                "Non-match ref",
                "mentions an entirely different concern",
                &["00_Index"],
                10,
            ),
        ],
        graph_edges: Vec::new(),
        receipt_ids: Vec::new(),
    };
    let response = select_graph_context(&base_request("Inspect token_budget"), &state)
        .expect("coverage selection");
    assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x10));
    assert!(
        response.selected_memory_refs[0]
            .selection_reason
            .contains("identifier_coverage")
    );
}

#[test]
fn graph_context_selection_contract_coverage_term_exempts_nothing_but_is_additive_only() {
    // The coverage term must not change which refs are valid, only their order.
    // A pure-prose task yields no identifier tokens and no coverage reasons.
    let response = select_graph_context(
        &base_request("summarize the general catalog contract summary"),
        &base_state(),
    )
    .expect("no-identifier selection");
    assert!(
        response
            .selected_memory_refs
            .iter()
            .all(|selected| !selected.selection_reason.contains("identifier_coverage"))
    );
}

// ---- Q2-S2 family diversity cap contract --------------------------------

#[test]
fn graph_context_selection_contract_family_cap_enforced_with_omission_records() {
    let mut memory_candidates = Vec::new();
    for byte in 0x20u8..0x2cu8 {
        memory_candidates.push(candidate(
            byte,
            "summary",
            "Crowded family ref",
            "actionable memory crowding one catalog family",
            &["04_Plans", "Next Steps"],
            10,
        ));
    }
    let state = GraphContextSelectionState {
        memory_candidates,
        graph_edges: Vec::new(),
        receipt_ids: Vec::new(),
    };
    let mut request = base_request("actionable memory family");
    // Envelope smaller than the candidate pool so both contract halves are
    // visible: the first pass admits MAX_FAMILY_REF_SHARE from the crowded
    // family, then the soft backfill fills the remaining slots (the cap is a
    // diversity preference, never a starvation rule).
    request.max_memory_refs = 10;
    request.token_budget = 100_000;
    let response = select_graph_context(&request, &state).expect("family cap selection");
    assert_eq!(response.selected_memory_refs.len(), 10);
    let backfilled = response
        .selected_memory_refs
        .iter()
        .filter(|selected| selected.selection_reason == "family_diversity_backfill")
        .count();
    assert_eq!(backfilled, 10 - MAX_FAMILY_REF_SHARE);
    // Candidates beyond the envelope remain omitted with the cap reason.
    assert!(
        response
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded")
    );
}

#[test]
fn graph_context_selection_contract_family_cap_exempts_requested_ids() {
    let mut memory_candidates = Vec::new();
    let mut requested = Vec::new();
    for byte in 0x30u8..0x3cu8 {
        memory_candidates.push(candidate(
            byte,
            "summary",
            "Requested family ref",
            "requested memory in one catalog family",
            &["04_Plans", "Next Steps"],
            10,
        ));
        requested.push(mem_id(byte));
    }
    let state = GraphContextSelectionState {
        memory_candidates,
        graph_edges: Vec::new(),
        receipt_ids: Vec::new(),
    };
    let mut request = base_request("requested memory family");
    request.max_memory_refs = 64;
    request.token_budget = 100_000;
    request.requested_memory_ids = requested.clone();
    let response = select_graph_context(&request, &state).expect("requested selection");
    assert_eq!(response.selected_memory_refs.len(), requested.len());
    assert!(
        !response
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded")
    );
}

// ---- M48-style relink: requested-id path unchanged ----------------------

#[test]
fn graph_context_selection_contract_m48_relink_requested_ids_path_unchanged() {
    // Relink retrieval requests an explicit id set. Neither the coverage term
    // nor the family cap may alter that path: requested refs are returned in
    // full, exempt from family diversity, with the requested-id reason intact.
    let mut state = base_state();
    // Make every base candidate share one family and contain task identifiers,
    // so both new mechanisms WOULD fire if they applied to requested ids.
    for memory in &mut state.memory_candidates {
        memory.catalog_path = vec!["04_Plans".into(), "Next Steps".into()];
        memory.summary.text = "covers token_budget and MemoryGraphStyle identifiers".into();
    }
    let requested = vec![mem_id(0x01), mem_id(0x02), mem_id(0x03)];
    let mut request = base_request("token_budget MemoryGraphStyle relink");
    request.requested_memory_ids = requested.clone();
    request.max_memory_refs = 64;
    request.token_budget = 100_000;
    let response = select_graph_context(&request, &state).expect("relink selection");
    assert_eq!(response.selected_memory_refs.len(), requested.len());
    for selected in &response.selected_memory_refs {
        assert!(
            selected.selection_reason.contains("requested_memory_id"),
            "requested id must keep its selection reason: {}",
            selected.selection_reason
        );
    }
    assert!(
        !response
            .omitted_memory_refs
            .iter()
            .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded")
    );
}
