#![allow(clippy::expect_used, clippy::unwrap_used)]

use exo_dag_db_exchange::import_drift_repair::{
    DriftRepairError, DriftRepairInput, build_drift_repair_record, validate_drift_repair_record,
};
use exo_dag_db_retrieval::{
    citation_locator::{
        CitationLocatorError, CitationLocatorInput, build_citation_locator,
        validate_citation_locator,
    },
    source_adapter::{
        CitationPolicy, ExportPolicy, ImportPolicy, SourceAdapterError, SourceManifest,
        validate_source_manifest,
    },
};

fn hex(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn manifest(adapter_id: &str) -> SourceManifest {
    SourceManifest {
        schema_version: "dagdb_prd17_source_manifest_v1".to_owned(),
        source_adapter_id: adapter_id.to_owned(),
        source_id: format!("{adapter_id}_source_001"),
        source_type: adapter_id.to_owned(),
        owner: "did:exo:prd17e-owner".to_owned(),
        digest: hex('a'),
        redaction_status: "redacted_safe".to_owned(),
        citation_policy: CitationPolicy {
            required: true,
            locator_policy: "deterministic_span_hash".to_owned(),
        },
        import_policy: ImportPolicy {
            source_refs: vec![format!(
                "tools/fixtures/dagdb_prd17/{adapter_id}/manifest.json"
            )],
            chunking_policy: "bounded_chunks".to_owned(),
            placement_policy: "catalog_graph_layered".to_owned(),
            duplicate_replay_safe: true,
        },
        export_policy: ExportPolicy {
            exportable: true,
            reimport_required: true,
        },
        leakage_scope: "private".to_owned(),
        tenant_id: "dag_db-local".to_owned(),
        project_id: "dag_db".to_owned(),
        memory_namespace: "project_memory_v3".to_owned(),
    }
}

#[test]
fn source_adapter_accepts_all_prd17e_adapter_classes() {
    for adapter_id in [
        "repo_file_bundle",
        "document_bundle",
        "structured_table_bundle",
        "operator_external_bundle",
    ] {
        let accepted =
            validate_source_manifest(&manifest(adapter_id)).expect("valid source manifest");
        assert_eq!(accepted.adapter_id, adapter_id);
        assert!(accepted.citation_locator_required);
        assert!(accepted.duplicate_replay_safe);
        assert_eq!(accepted.idempotency_key.len(), 64);
    }
}

#[test]
fn source_adapter_rejects_unsafe_or_overclaimed_manifests() {
    let mut unsupported = manifest("repo_file_bundle");
    unsupported.source_adapter_id = "arbitrary_web".to_owned();
    unsupported.source_type = "arbitrary_web".to_owned();
    assert_eq!(
        validate_source_manifest(&unsupported),
        Err(SourceAdapterError::UnsupportedSourceType)
    );

    let mut unredacted_private = manifest("document_bundle");
    unredacted_private.redaction_status = "unredacted_private".to_owned();
    assert_eq!(
        validate_source_manifest(&unredacted_private),
        Err(SourceAdapterError::UnredactedPrivateSource)
    );

    let mut path_traversal = manifest("structured_table_bundle");
    path_traversal.import_policy.source_refs = vec!["../private/source.csv".to_owned()];
    assert_eq!(
        validate_source_manifest(&path_traversal),
        Err(SourceAdapterError::PathTraversal)
    );

    let mut missing_citation_policy = manifest("operator_external_bundle");
    missing_citation_policy.citation_policy.required = false;
    assert_eq!(
        validate_source_manifest(&missing_citation_policy),
        Err(SourceAdapterError::MissingCitationPolicy)
    );
}

#[test]
fn citation_locator_is_deterministic_and_fail_closed() {
    let input = CitationLocatorInput {
        source_id: "repo_file_bundle_source_001",
        source_digest: &hex('b'),
        memory_id: "memory-001",
        span_ref: "docs/dagdb/prd17e.md#L10-L12",
        citation_text: "PRD17E requires source adapter citation locators.",
        redaction_status: "redacted_safe",
    };
    let first = build_citation_locator(input.clone()).expect("build locator");
    let second = build_citation_locator(input).expect("build locator");
    assert_eq!(first, second);
    validate_citation_locator(&first).expect("valid locator");

    let mut tampered = first.clone();
    tampered.locator_id = hex('c');
    assert_eq!(
        validate_citation_locator(&tampered),
        Err(CitationLocatorError::LocatorIdMismatch)
    );

    let mut unsafe_span = first;
    unsafe_span.span_ref = "../private.md#L1".to_owned();
    assert_eq!(
        validate_citation_locator(&unsafe_span),
        Err(CitationLocatorError::UnsafeField("span_ref"))
    );
}

#[test]
fn import_drift_repair_preserves_old_digest_evidence() {
    let record = build_drift_repair_record(DriftRepairInput {
        source_id: "repo_file_bundle_source_001",
        old_digest: &hex('d'),
        new_digest: &hex('e'),
        old_digest_evidence_ref: "tools/fixtures/dagdb_prd17/source_ingestion/citation_repair/old-digest.json",
        affected_memory_ids: vec!["memory-001", "memory-002"],
        repair_action: "citation_locator_repair",
        created_at: "2026-06-07T00:00:00Z",
    })
    .expect("build drift repair");
    validate_drift_repair_record(&record).expect("valid drift repair");

    let mut missing_old_digest_evidence = record.clone();
    missing_old_digest_evidence.old_digest_evidence_ref = "none".to_owned();
    assert_eq!(
        validate_drift_repair_record(&missing_old_digest_evidence),
        Err(DriftRepairError::OldDigestEvidenceMissing)
    );

    let mut no_drift = record;
    no_drift.new_digest = no_drift.old_digest.clone();
    assert_eq!(
        validate_drift_repair_record(&no_drift),
        Err(DriftRepairError::DigestNotDrifted)
    );
}

#[test]
fn import_drift_repair_rejects_newline_in_old_digest_evidence_ref() {
    // A newline in old_digest_evidence_ref could smuggle a forged affected_memory_id
    // across the '\n'-joined deterministic_repair_id, producing a hash collision.
    for forged in ["evidence.json\nmemory-001", "evidence.json\rmemory-001"] {
        assert_eq!(
            build_drift_repair_record(DriftRepairInput {
                source_id: "repo_file_bundle_source_001",
                old_digest: &hex('d'),
                new_digest: &hex('e'),
                old_digest_evidence_ref: forged,
                affected_memory_ids: vec!["memory-002"],
                repair_action: "citation_locator_repair",
                created_at: "2026-06-07T00:00:00Z",
            })
            .err(),
            Some(DriftRepairError::OldDigestEvidenceUnsafe),
            "newline/cr in old_digest_evidence_ref must be rejected",
        );
    }
}

#[test]
fn import_drift_repair_rejects_unsorted_or_nonadjacent_duplicate_memory_ids() {
    let record = build_drift_repair_record(DriftRepairInput {
        source_id: "repo_file_bundle_source_001",
        old_digest: &hex('d'),
        new_digest: &hex('e'),
        old_digest_evidence_ref: "tools/fixtures/dagdb_prd17/source_ingestion/citation_repair/old-digest.json",
        affected_memory_ids: vec!["memory-001", "memory-002"],
        repair_action: "citation_locator_repair",
        created_at: "2026-06-07T00:00:00Z",
    })
    .expect("build drift repair");

    let mut nonadjacent_duplicate = record.clone();
    nonadjacent_duplicate.affected_memory_ids = vec![
        "memory-002".to_owned(),
        "memory-001".to_owned(),
        "memory-002".to_owned(),
    ];
    assert_eq!(
        validate_drift_repair_record(&nonadjacent_duplicate),
        Err(DriftRepairError::AffectedMemoryUnsorted)
    );

    let mut unsorted = record;
    unsorted.affected_memory_ids = vec!["memory-002".to_owned(), "memory-001".to_owned()];
    assert_eq!(
        validate_drift_repair_record(&unsorted),
        Err(DriftRepairError::AffectedMemoryUnsorted)
    );
}
