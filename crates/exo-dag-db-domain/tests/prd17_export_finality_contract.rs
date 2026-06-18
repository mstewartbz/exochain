#![allow(clippy::expect_used, clippy::unwrap_used)]

use exo_dag_db_domain::export_finality::{
    ExportFinalityError, ExportFinalityRecord, ReimportContinuity, validate_export_finality_record,
};

fn hex(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn continuity() -> ReimportContinuity {
    ReimportContinuity {
        memory_ids: vec!["memory-001".to_owned()],
        catalog_entry_ids: vec!["catalog-001".to_owned()],
        graph_node_ids: vec!["graph-001".to_owned()],
        layer_membership_ids: vec!["layer-membership-001".to_owned()],
        validation_report_ids: vec!["validation-001".to_owned()],
        citation_locator_ids: vec!["citation-001".to_owned()],
        provenance_preserved: true,
    }
}

fn record(finality_state: &str) -> ExportFinalityRecord {
    ExportFinalityRecord {
        schema_version: "dagdb_prd17_export_finality_record_v1".to_owned(),
        export_id: "export-001".to_owned(),
        artifact_digest: hex('a'),
        metadata_digest: hex('b'),
        receipt_id: "receipt-001".to_owned(),
        local_outbox_ref: Some(
            "target/dagdb/thesis_10/source_ingestion/local-outbox-ref.json".to_owned(),
        ),
        production_finality_ref: None,
        finality_state: finality_state.to_owned(),
        reimport_id: "reimport-001".to_owned(),
        retrieval_reuse_status: "retrieval_reuse_passed".to_owned(),
        leakage_status: "passed_zero_leakage".to_owned(),
        reimport_continuity: continuity(),
    }
}

#[test]
fn export_finality_accepts_local_outbox_without_claiming_production_finality() {
    let local = record("local_outbox_accepted");
    validate_export_finality_record(&local, &hex('a'), &hex('b')).expect("local finality accepted");

    let mut production = record("production_finality_accepted");
    production.production_finality_ref =
        Some("docs/dagdb/operator-evidence/finality-receipt.json".to_owned());
    validate_export_finality_record(&production, &hex('a'), &hex('b'))
        .expect("production finality accepted when receipt ref is present");

    let mut deferred = record("operator_deferred");
    deferred.local_outbox_ref =
        Some("target/dagdb/thesis_10/source_ingestion/local-outbox-ref.json".to_owned());
    validate_export_finality_record(&deferred, &hex('a'), &hex('b'))
        .expect("operator deferred allows local outbox diagnostics only");
}

#[test]
fn export_finality_rejects_digest_mismatch_missing_refs_and_leakage() {
    let local = record("local_outbox_accepted");
    assert_eq!(
        validate_export_finality_record(&local, &hex('c'), &hex('b')),
        Err(ExportFinalityError::ArtifactDigestMismatch)
    );

    let mut missing_outbox = record("local_outbox_accepted");
    missing_outbox.local_outbox_ref = None;
    assert_eq!(
        validate_export_finality_record(&missing_outbox, &hex('a'), &hex('b')),
        Err(ExportFinalityError::MissingLocalOutboxRef)
    );

    let mut missing_production = record("production_finality_accepted");
    missing_production.production_finality_ref = None;
    assert_eq!(
        validate_export_finality_record(&missing_production, &hex('a'), &hex('b')),
        Err(ExportFinalityError::MissingProductionFinalityRef)
    );

    let mut leaked = record("local_outbox_accepted");
    leaked.leakage_status = "leakage_detected".to_owned();
    assert_eq!(
        validate_export_finality_record(&leaked, &hex('a'), &hex('b')),
        Err(ExportFinalityError::LeakageStatusInvalid)
    );
}

#[test]
fn export_finality_requires_reimport_continuity_refs() {
    let mut incomplete = record("local_outbox_accepted");
    incomplete.reimport_continuity.citation_locator_ids.clear();
    assert_eq!(
        validate_export_finality_record(&incomplete, &hex('a'), &hex('b')),
        Err(ExportFinalityError::ReimportContinuityIncomplete)
    );

    let mut unpreserved = record("local_outbox_accepted");
    unpreserved.reimport_continuity.provenance_preserved = false;
    assert_eq!(
        validate_export_finality_record(&unpreserved, &hex('a'), &hex('b')),
        Err(ExportFinalityError::ReimportContinuityIncomplete)
    );
}

#[test]
fn export_finality_rejects_unsorted_or_nonadjacent_duplicate_continuity_ids() {
    let mut nonadjacent_duplicate = record("local_outbox_accepted");
    nonadjacent_duplicate.reimport_continuity.memory_ids = vec![
        "memory-b".to_owned(),
        "memory-a".to_owned(),
        "memory-b".to_owned(),
    ];
    assert_eq!(
        validate_export_finality_record(&nonadjacent_duplicate, &hex('a'), &hex('b')),
        Err(ExportFinalityError::UnsafeField("memory_ids"))
    );

    let mut unsorted = record("local_outbox_accepted");
    unsorted.reimport_continuity.graph_node_ids = vec!["graph-b".to_owned(), "graph-a".to_owned()];
    assert_eq!(
        validate_export_finality_record(&unsorted, &hex('a'), &hex('b')),
        Err(ExportFinalityError::UnsafeField("graph_node_ids"))
    );
}
