//! Run a live-local agent-brain writeback growth smoke proof.

use std::{env, process};

use exo_dag_db_api::{MemoryCandidateKind, MemoryCandidateUse, RiskClass};
use exo_dag_db_exchange::{
    kg_import::{KG_IMPORT_DATABASE_URL_ENV, stable_hash},
    kg_writeback::{
        KgAgentWritebackHint, KgWritebackExistingMemory, KgWritebackProposalRequest,
        build_writeback_dry_run_report,
    },
};
use exo_dag_db_postgres::postgres::{
    DAGDB_GRAPH_SCHEMA_SQL, DAGDB_SCHEMA_SQL, kg_retrieval::retrieve_kg_context_packet,
    kg_writeback::persist_kg_writeback_report,
};
use exo_dag_db_retrieval::kg_retrieval::KgRetrievalRequest;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

const TENANT_ID: &str = "dag-db-local";
const NAMESPACE: &str = "dag-db";

#[tokio::main]
async fn main() {
    let task_id = parse_task_id();
    match run(task_id.as_str()).await {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn parse_task_id() -> String {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--task-id"), Some(task_id)) if !task_id.trim().is_empty() => task_id,
        _ => "writeback-growth-smoke".to_owned(),
    }
}

async fn run(task_id: &str) -> Result<String, String> {
    let database_url = env::var(KG_IMPORT_DATABASE_URL_ENV)
        .map_err(|_| "gateway database unavailable".to_owned())?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url.as_str())
        .await
        .map_err(|error| format!("writeback_growth_postgres_connect_failed: {error}"))?;
    sqlx::raw_sql(DAGDB_SCHEMA_SQL)
        .execute(&pool)
        .await
        .map_err(|error| format!("writeback_growth_schema_apply_failed: {error}"))?;
    sqlx::raw_sql(DAGDB_GRAPH_SCHEMA_SQL)
        .execute(&pool)
        .await
        .map_err(|error| format!("writeback_growth_graph_schema_apply_failed: {error}"))?;

    let task_hash = stable_hash(
        "exo.dagdb.agent_brain.writeback_growth.task_hash",
        &[TENANT_ID, NAMESPACE, task_id],
    )
    .map_err(|error| error.to_string())?
    .to_string();
    let initial_request = KgRetrievalRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        task_hash: Some(task_hash.clone()),
        task_description: None,
        token_budget: 800,
        requested_memory_ids: Vec::new(),
        catalog_path: Some(vec![
            "KnowledgeGraphs".to_owned(),
            "dag-db".to_owned(),
            "00_Pinned_Mission".to_owned(),
        ]),
        max_memory_refs: Some(2),
        layer_path: None,
        max_layer_depth: None,
        max_layers_selected: None,
        max_nodes_per_layer: None,
        max_layer_edges: None,
    };
    let initial_preview = retrieve_kg_context_packet(&pool, &initial_request)
        .await
        .map_err(|error| error.to_string())?;
    let Some(citation) = initial_preview.citation_handles.first().cloned() else {
        return Err("writeback_growth_missing_parent_citation".to_owned());
    };
    let output_hash = stable_hash(
        "exo.dagdb.agent_brain.writeback_growth.output_hash",
        &[
            TENANT_ID,
            NAMESPACE,
            task_id,
            &initial_preview.context_packet_id,
        ],
    )
    .map_err(|error| error.to_string())?
    .to_string();
    let source_request_id = format!("agent-brain-writeback-{task_id}");
    let report = build_writeback_dry_run_report(KgWritebackProposalRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        requesting_agent_did: "did:exo:agent-brain-writeback".to_owned(),
        context_packet: initial_preview.clone(),
        hint: KgAgentWritebackHint {
            source_request_id: source_request_id.clone(),
            parent_context_packet_id: initial_preview.context_packet_id.clone(),
            route_hint_id: initial_preview.route_hint_id.clone(),
            task_hash,
            answer_hash: Some(output_hash),
            output_hash: None,
            candidate_kind: MemoryCandidateKind::Summary,
            summary: "Agent brain writeback growth smoke summary.".to_owned(),
            citation_handles: vec![citation.handle.clone()],
            evidence_receipts: vec![citation.latest_receipt_hash.clone()],
            risk_hint: RiskClass::R1,
            allowed_future_uses: vec![MemoryCandidateUse::Routing, MemoryCandidateUse::Audit],
            reason_to_remember: "Prove live-local graph growth from selected parent context."
                .to_owned(),
            keyword_texts: Vec::new(),
            contradiction_refs: Vec::new(),
            supersession_refs: Vec::new(),
        },
        existing_memory: Vec::<KgWritebackExistingMemory>::new(),
    })
    .map_err(|error| error.to_string())?;
    let report_json = serde_json::to_string(&report).map_err(|error| error.to_string())?;
    let first_summary = persist_kg_writeback_report(&pool, &report_json)
        .await
        .map_err(|error| error.to_string())?;
    let second_summary = persist_kg_writeback_report(&pool, &report_json)
        .await
        .map_err(|error| error.to_string())?;

    let parent_memory_ids = report.evidence_binding.selected_memory_ids.clone();
    // The written child (report.candidate_id) is bound explicitly into the later
    // retrieval's requested_memory_ids because this proof asserts retrievability/relink of
    // the fresh writeback and its created edge (the M48 growth property), NOT that the
    // child out-ranks the entire corpus for a residual slot. The child legitimately holds
    // no catalog-cluster layer membership, so relying on incidental residual-slot ranking
    // is fragile once the corpus grows layers; requesting it by id is the honest claim.
    let mut requested_memory_ids = vec![report.candidate_id.clone()];
    requested_memory_ids.extend(parent_memory_ids.iter().cloned());
    requested_memory_ids.sort();
    requested_memory_ids.dedup();
    let later_request = KgRetrievalRequest {
        tenant_id: TENANT_ID.to_owned(),
        namespace: NAMESPACE.to_owned(),
        task_hash: None,
        task_description: Some(format!("later retrieval for {task_id}")),
        token_budget: 800,
        requested_memory_ids,
        catalog_path: None,
        max_memory_refs: Some(3),
        layer_path: None,
        max_layer_depth: None,
        max_layers_selected: None,
        max_nodes_per_layer: None,
        max_layer_edges: None,
    };
    let later_preview = retrieve_kg_context_packet(&pool, &later_request)
        .await
        .map_err(|error| error.to_string())?;
    let selected_written_memory = later_preview
        .memory_refs
        .iter()
        .any(|memory| memory.memory_id == report.candidate_id);
    if !second_summary.replayed {
        return Err("writeback_growth_replay_not_idempotent".to_owned());
    }
    if !selected_written_memory {
        return Err("writeback_growth_later_retrieval_missing_written_memory".to_owned());
    }

    let mut created_edges = Vec::new();
    let mut expected_edge_ids = Vec::new();
    for parent_memory_id in &parent_memory_ids {
        let memory_edge_id = stable_hash(
            "exo.dagdb.agent_brain.writeback_growth.memory_edge_id",
            &[
                TENANT_ID,
                NAMESPACE,
                &report.candidate_id,
                parent_memory_id,
                "derived_from",
            ],
        )
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())?;
        let Some(created_edge) = later_preview.graph_edges.iter().find(|edge| {
            edge.from_memory_id == report.candidate_id
                && edge.to_memory_id == *parent_memory_id
                && edge.edge_kind == "derived_from"
                && edge.graph_style == "canonical_memory_graph"
        }) else {
            return Err("writeback_growth_created_edge_missing".to_owned());
        };
        let Some(receipt_hash) = created_edge.receipt_hash.clone() else {
            return Err("writeback_growth_created_edge_receipt_missing".to_owned());
        };
        expected_edge_ids.push(created_edge.graph_edge_id.clone());
        created_edges.push(json!({
            "db_edge_kind": "derived_from",
            "edge_id": created_edge.graph_edge_id,
            "from_memory_id": report.candidate_id,
            "to_memory_id": parent_memory_id,
            "edge_kind": "derived_from_context_ref",
            "graph_style": "canonical_memory_graph",
            "receipt_hash": receipt_hash,
            "quality_seed_bp": 1000,
            "policy_status": "created_or_replayed",
            "db_memory_edge_type": "derived_from",
            "memory_edge_id": memory_edge_id,
            "memory_edge_storage": "compound_key"
        }));
    }
    let selected_edge_ids = later_preview
        .graph_edges
        .iter()
        .filter(|edge| expected_edge_ids.contains(&edge.graph_edge_id))
        .map(|edge| edge.graph_edge_id.clone())
        .collect::<Vec<_>>();
    if selected_edge_ids.is_empty() {
        return Err("writeback_growth_later_retrieval_missing_created_edge".to_owned());
    }
    let receipt_hashes = later_preview
        .graph_edges
        .iter()
        .filter_map(|edge| edge.receipt_hash.clone())
        .collect::<Vec<_>>();
    let output = json!({
        "schema_version": "dagdb_agent_brain_writeback_growth_smoke_v1",
        "task_id": task_id,
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "context_packet_id": initial_preview.context_packet_id,
        "route_hint_id": initial_preview.route_hint_id,
        "writeback_request_id": source_request_id,
        "persisted_memory_id": report.candidate_id,
        "selected_parent_memory_ids": parent_memory_ids,
        "selected_parent_citation_handles": report.evidence_binding.citation_handles,
        "accepted_writeback": {
            "gateway_validation_status": first_summary
                .diagnostics
                .validation_risk_council
                .validation_status,
            "dag_finality_status": "pending",
            "validation_decision": first_summary
                .diagnostics
                .validation_risk_council
                .decision
        },
        "created_edges": created_edges,
        "receipt_hashes": receipt_hashes,
        "first_persist_summary": first_summary,
        "second_persist_summary": second_summary,
        "later_retrieval": {
            "packet_id": later_preview.context_packet_id,
            "selected_written_memory": selected_written_memory,
            "selected_edge_count": selected_edge_ids.len(),
            "selected_edge_ids": selected_edge_ids,
            "selected_memory_ids": later_preview
                .memory_refs
                .iter()
                .map(|memory| memory.memory_id.clone())
                .collect::<Vec<_>>(),
            "selection_reason": "requested_written_memory_and_parent_context_ref"
        }
    });
    pool.close().await;
    serde_json::to_string(&output).map_err(|error| error.to_string())
}
