#![allow(clippy::expect_used)]

use exo_dag_db_exchange::{
    kg_export::{
        KgExportBuildInput, KgExportRecord, KgExportScope, KgPortableExport, build_portable_export,
        parse_portable_export_json,
    },
    kg_import::{
        KG_IMPORT_CANDIDATES_SCHEMA, KG_IMPORT_DRY_RUN_REPORT_SCHEMA, KgImportDryRunReport,
        required_trace, stable_hash,
    },
};
use serde_json::{Value as JsonValue, json};

const TENANT_ID: &str = "dag_db-local";
const NAMESPACE: &str = "dag_db";
const DB_SET_VERSION: &str = "project_memory_v3";
const EXPECTED_IMPORT_INPUT_HASH: &str =
    "e56140aadd1debeb3741530ff1f35de394958076614aac50c8ae384b587dcc76";
const EXPECTED_EXPORT_ARTIFACT_HASH: &str =
    "ba6094a1241a678a13082deb4d7663032b516aef476646e994b1d514b0406153";
const EXPECTED_REIMPORT_HASH: &str =
    "e98ac55c96de64963dfe20da36dfc2a4f4499d4178e59de593500b16791a61ef";

#[test]
fn m47_import_export_reimport_hash_chain_is_deterministic_without_live_db() {
    let import_input = import_report_json();
    let import_input_body = import_input.to_string();
    let import_report =
        KgImportDryRunReport::parse_json(&import_input_body).expect("valid import fixture");
    assert_eq!(import_report.tenant_id, TENANT_ID);
    assert_eq!(import_report.namespace, NAMESPACE);

    let first_export = export_from_import_fixture(&import_input);
    let second_export = export_from_import_fixture(&import_input);
    assert_eq!(first_export, second_export);
    assert_eq!(first_export.tenant_id, TENANT_ID);
    assert_eq!(first_export.namespace, NAMESPACE);
    assert_eq!(
        first_export
            .export_scope
            .source_commit_or_repo_ref
            .as_deref(),
        Some(DB_SET_VERSION)
    );
    assert!(first_export.acceptance.report_only);
    assert!(!first_export.verification.export_persistence_implemented);
    assert!(!first_export.verification.gateway_api_exposure_implemented);
    assert!(
        !first_export
            .verification
            .production_route_activation_implemented
    );
    assert!(!first_export.verification.exo_dag_tables_mutated);

    let export_body = serde_json::to_string_pretty(&first_export).expect("serialize export");
    let parsed_export = parse_portable_export_json(&export_body).expect("parse export artifact");
    let reimport_input = reimport_report_json(&parsed_export);
    let reimport_body = reimport_input.to_string();
    let reimport_report =
        KgImportDryRunReport::parse_json(&reimport_body).expect("valid reimport fixture");
    assert_eq!(reimport_report.schema_version, import_report.schema_version);
    assert_eq!(reimport_report.tenant_id, import_report.tenant_id);
    assert_eq!(reimport_report.namespace, import_report.namespace);
    assert_eq!(reimport_report.proposed_memory_records.len(), 1);
    assert_eq!(reimport_report.proposed_catalog_entries.len(), 1);
    assert_eq!(reimport_report.proposed_graph_nodes.len(), 1);
    assert_eq!(reimport_report.proposed_graph_edges.len(), 1);
    assert_eq!(reimport_report.proposed_placement_decisions.len(), 1);
    assert_eq!(reimport_report.proposed_receipt_intents.len(), 1);
    assert_eq!(reimport_report.proposed_validation_reports.len(), 1);

    let import_input_hash = stable_hash("exo.dagdb.m47.import_input", &[&import_input_body])
        .expect("hash import input")
        .to_string();
    let export_artifact_hash = parsed_export.hashes.whole_export_hash.clone();
    let reimport_hash = stable_hash("exo.dagdb.m47.reimport_input", &[&reimport_body])
        .expect("hash reimport input")
        .to_string();

    assert_eq!(import_input_hash, EXPECTED_IMPORT_INPUT_HASH);
    assert_eq!(export_artifact_hash, EXPECTED_EXPORT_ARTIFACT_HASH);
    assert_eq!(reimport_hash, EXPECTED_REIMPORT_HASH);
}

fn safe_text(text: &str) -> JsonValue {
    json!({
        "decision": "allow",
        "text": text,
        "redaction_codes": [],
        "original_hash": "c".repeat(64),
        "truncated": false,
        "byte_len": text.len(),
    })
}

fn import_report_json() -> JsonValue {
    let memory_id = "1".repeat(64);
    let catalog_id = "2".repeat(64);
    let graph_node_id = "3".repeat(64);
    let graph_edge_id = "4".repeat(64);
    let placement_decision_id = "6".repeat(64);
    let receipt_intent_id = "7".repeat(64);
    let validation_report_id = "8".repeat(64);
    let source_hash = "9".repeat(64);
    let payload_hash = "a".repeat(64);
    let policy_hash = "b".repeat(64);

    json!({
        "schema_version": KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
        "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
        "graph_root": "docs/dagdb/catalog-governed-memory",
        "tenant_id": TENANT_ID,
        "namespace": NAMESPACE,
        "actor_did": "did:exo:m47-importer",
        "batch_id": "d".repeat(64),
        "dry_run_only": true,
        "postgres_writes": false,
        "raw_markdown_included": false,
        "proposed_memory_records": [{
            "memory_id": memory_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "source_path": "docs/dagdb/catalog-governed-memory/m47-fixture.md",
            "candidate_id": "m47-candidate-001",
            "node_type": "source",
            "source_type": "generated",
            "source_hash": source_hash,
            "payload_hash": payload_hash,
            "owner_did": "did:exo:m47-owner",
            "controller_did": "did:exo:m47-controller",
            "submitted_by_did": "did:exo:m47-submitter",
            "consent_purpose": "retrieval",
            "title": safe_text("M47 import export fixture"),
            "summary": safe_text("Safe metadata fixture for repository round trip"),
            "keywords": [safe_text("m47"), safe_text(DB_SET_VERSION)],
            "catalog_path": ["docs", "dagdb", DB_SET_VERSION],
            "risk_class": "R1",
            "risk_bp": 100,
            "validation_status": "pending",
            "council_status": "not_required",
            "dag_finality_status": "pending",
            "status": "pending",
            "receipt_intent_id": receipt_intent_id
        }],
        "proposed_catalog_entries": [{
            "catalog_id": catalog_id,
            "memory_id": memory_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "catalog_path": ["docs", "dagdb", DB_SET_VERSION],
            "catalog_level": 3,
            "title": safe_text("M47 catalog"),
            "summary": safe_text("Catalog metadata fixture"),
            "payload_hash": payload_hash,
            "source_hash": source_hash,
            "status": "pending",
            "validation_status": "pending",
            "council_status": "not_required",
            "dag_finality_status": "pending",
            "receipt_intent_id": receipt_intent_id
        }],
        "proposed_graph_nodes": [{
            "graph_node_id": graph_node_id,
            "memory_id": memory_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "graph_style": "semantic_catalog_graph",
            "node_kind": "raw",
            "catalog_path": ["docs", "dagdb", DB_SET_VERSION]
        }],
        "proposed_graph_edges": [{
            "graph_edge_id": graph_edge_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "graph_style": "semantic_catalog_graph",
            "from_memory_id": memory_id,
            "to_memory_id": memory_id,
            "edge_kind": "related_to",
            "source_edge_kind": "wikilink"
        }],
        "proposed_placement_decisions": [{
            "placement_decision_id": placement_decision_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "input_memory_id": memory_id,
            "placement_trace": required_trace(),
            "canonicalization_decision": {
                "canonical_memory_id": memory_id,
                "confidence_bp": 9000,
                "decision_kind": "new_canonical",
                "decision_reason": "safe metadata fixture",
                "matched_memory_ids": [memory_id],
                "required_edges_to_create": [],
                "risk_class": "R1",
                "validator_status": "passed"
            },
            "similarity_results": [],
            "validator_report": "passed",
            "receipt_intent_id": receipt_intent_id
        }],
        "proposed_receipt_intents": [{
            "receipt_intent_id": receipt_intent_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "subject_kind": "memory",
            "subject_id": memory_id,
            "event_type": "intake_created",
            "actor_did": "did:exo:m47-importer",
            "reason": "repository test fixture"
        }],
        "proposed_validation_reports": [{
            "validation_report_id": validation_report_id,
            "tenant_id": TENANT_ID,
            "namespace": NAMESPACE,
            "subject_kind": "memory",
            "subject_id": memory_id,
            "validator_did": "did:exo:m47-validator",
            "input_hash": payload_hash,
            "policy_hash": policy_hash,
            "validation_status": "passed",
            "risk_class": "R1",
            "risk_bp": 100,
            "decision": "allow",
            "notes": safe_text("validation metadata")
        }],
        "proposed_governance_reviews": [],
        "proposed_graph_view_refreshes": [],
        "proposed_route_invalidations": [],
        "proposed_subdag_boundaries": [],
        "rollback_plan": {},
        "placement_governance_summary": {},
        "review_items": [],
        "warnings": []
    })
}

fn export_from_import_fixture(import_input: &JsonValue) -> KgPortableExport {
    build_portable_export(KgExportBuildInput {
        scope: KgExportScope {
            tenant_id: TENANT_ID.to_owned(),
            namespace: NAMESPACE.to_owned(),
            included_memory_ids: Vec::new(),
            included_graph_styles: Vec::new(),
            included_writeback_idempotency_keys: Vec::new(),
            source_commit_or_repo_ref: Some(DB_SET_VERSION.to_owned()),
            include_preview_context: false,
        },
        // The production export adapter never persists origin paths
        // (`origin_path_not_persisted`), so the fixture conversion drops
        // `source_path` before building export material.
        memory_records: records(import_input, "proposed_memory_records")
            .into_iter()
            .map(|mut record| {
                record.remove("source_path");
                record
            })
            .collect(),
        catalog_entries: records(import_input, "proposed_catalog_entries"),
        graph_nodes: records(import_input, "proposed_graph_nodes"),
        graph_edges: records(import_input, "proposed_graph_edges"),
        similarity_results: Vec::new(),
        canonicalization_decisions: Vec::new(),
        placement_traces: records(import_input, "proposed_placement_decisions"),
        validation_reports: records(import_input, "proposed_validation_reports"),
        receipts: records(import_input, "proposed_receipt_intents"),
        subject_receipt_heads: Vec::new(),
        context_packet_previews: Vec::new(),
        context_packet_records: Vec::new(),
        route_receipts: Vec::new(),
        writeback_summaries: Vec::new(),
        idempotency_references: Vec::new(),
        citation_index: Vec::new(),
        provenance_index: Vec::new(),
    })
    .expect("build portable export")
}

fn reimport_report_json(export: &KgPortableExport) -> JsonValue {
    json!({
        "schema_version": KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
        "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
        "graph_root": "docs/dagdb/catalog-governed-memory",
        "tenant_id": export.tenant_id,
        "namespace": export.namespace,
        "actor_did": "did:exo:m47-importer",
        "batch_id": "e".repeat(64),
        "dry_run_only": true,
        "postgres_writes": false,
        "raw_markdown_included": false,
        // Exports never carry origin paths, so a reimport re-attaches the
        // local source path before proposing records.
        "proposed_memory_records": export
            .memory_records
            .iter()
            .cloned()
            .map(|mut record| {
                record.insert(
                    "source_path".to_owned(),
                    json!("docs/dagdb/catalog-governed-memory/m47-fixture.md"),
                );
                record
            })
            .collect::<Vec<_>>(),
        "proposed_catalog_entries": export.catalog_entries,
        "proposed_graph_nodes": export.graph_nodes,
        "proposed_graph_edges": export.graph_edges,
        "proposed_placement_decisions": export.placement_traces,
        "proposed_receipt_intents": export.receipts,
        "proposed_validation_reports": export.validation_reports,
        "proposed_governance_reviews": [],
        "proposed_graph_view_refreshes": [],
        "proposed_route_invalidations": [],
        "proposed_subdag_boundaries": [],
        "rollback_plan": {},
        "placement_governance_summary": {},
        "review_items": [],
        "warnings": []
    })
}

fn records(value: &JsonValue, field: &str) -> Vec<KgExportRecord> {
    value
        .get(field)
        .and_then(JsonValue::as_array)
        .expect("fixture array")
        .iter()
        .map(record)
        .collect()
}

fn record(value: &JsonValue) -> KgExportRecord {
    value
        .as_object()
        .expect("fixture object")
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}
