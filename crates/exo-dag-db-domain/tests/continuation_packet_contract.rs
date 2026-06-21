#![allow(clippy::expect_used)]

use exo_dag_db_domain::continuation_packet::{
    DAGDB_CONTINUATION_PACKET_SCHEMA_VERSION, DagDbContinuationPacket, DagDbContinuationPacketError,
};
use serde_json::{Value as JsonValue, json};

const PYTHON_COMPAT_REPORT: &str = include_str!(
    "../../../docs/dagdb/catalog-governed-memory/self-development/continuation-packets.dag_db-project_memory_v3.json"
);

fn tracked_packet_json() -> JsonValue {
    let report: JsonValue = serde_json::from_str(PYTHON_COMPAT_REPORT).expect("compat report json");
    report["packets"][0].clone()
}

fn tracked_report_json() -> JsonValue {
    serde_json::from_str(PYTHON_COMPAT_REPORT).expect("compat report json")
}

fn parse_tracked_packet() -> DagDbContinuationPacket {
    DagDbContinuationPacket::parse_json(&tracked_packet_json().to_string())
        .expect("tracked Python v1 packet parses")
}

fn refresh_token_estimate(packet: &mut JsonValue) {
    let blockers = packet["blockers"]
        .as_array()
        .expect("blockers array")
        .iter()
        .map(|value| value.as_str().expect("blocker string"))
        .collect::<Vec<_>>()
        .join(" ");
    let changed_paths = packet["changed_paths"]
        .as_array()
        .expect("changed_paths array")
        .iter()
        .map(|value| value.as_str().expect("changed path string"))
        .collect::<Vec<_>>()
        .join(" ");
    let material = [
        packet["stopped_at"].as_str().expect("stopped_at string"),
        packet["next_steps"].as_str().expect("next_steps string"),
        blockers.as_str(),
        changed_paths.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n");
    packet["token_estimate"] = json!(material.len().div_ceil(4));
}

#[test]
fn continuation_accepts_existing_python_v1_packet_and_report() {
    let packet = parse_tracked_packet();
    assert_eq!(
        packet.schema_version,
        DAGDB_CONTINUATION_PACKET_SCHEMA_VERSION
    );
    assert_eq!(packet.source_task_id, "batch12_memory_loop_task_1");
    assert_eq!(packet.memory_refs.len(), 4);
    assert_eq!(
        packet.relink_refs,
        vec!["blocked_missing_live_relink:writeback_blocked_fallback_lineage".to_owned()]
    );
    assert!(
        packet
            .non_claims
            .contains(&"production_runtime_not_approved".to_owned())
    );

    let packets = DagDbContinuationPacket::parse_report_json(PYTHON_COMPAT_REPORT)
        .expect("tracked Python v1 report parses");
    assert_eq!(packets.len(), 2);
    assert_eq!(packets[1].source_task_id, "batch12_memory_loop_task_2");
}

#[test]
fn continuation_rejects_incompatible_rust_only_same_version_shape() {
    let rust_only = json!({
        "schema_version": DAGDB_CONTINUATION_PACKET_SCHEMA_VERSION,
        "packet_id": "m49-m63-continuation-packet-001",
        "source_task_id": "batch12_memory_loop_task_1",
        "memory_refs": [
            "1573772dd54b7b5b849895a75e966f76bc035f55ece4c17d251576b1da2ec1bd"
        ],
        "relink_refs": ["m31-loop-report"],
        "continuation_prompt": "Resume bounded continuation work.",
        "non_claims": [
            "default_memory_activation_blocked",
            "final_thesis_acceptance_blocked",
            "production_runtime_not_approved",
            "route_activation_not_approved"
        ]
    });

    assert!(
        matches!(
            DagDbContinuationPacket::parse_json(&rust_only.to_string()),
            Err(DagDbContinuationPacketError::Json { .. })
        ),
        "same-version Rust-only shape must not be silently accepted as Python v1"
    );
}

#[test]
fn continuation_rejects_missing_memory_refs() {
    let mut packet = tracked_packet_json();
    packet["memory_ref_ids"] = json!([]);

    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::EmptyList {
            field: "memory_ref_ids".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_missing_relink_equivalent_evidence() {
    let mut packet = tracked_packet_json();
    packet["boundary_warnings"] = json!(["gateway_offline", "repository_test_level_only"]);

    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::RelinkEvidenceMissing)
    );

    let mut canonical = parse_tracked_packet();
    canonical.relink_refs.clear();
    assert_eq!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::EmptyList {
            field: "relink_refs".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_missing_required_blocked_non_claims() {
    let mut canonical = parse_tracked_packet();
    canonical
        .non_claims
        .retain(|claim| claim != "final_thesis_acceptance_blocked");

    assert_eq!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::MissingBlockedNonClaim {
            claim: "final_thesis_acceptance_blocked".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_runtime_default_route_and_final_thesis_overclaims() {
    for phrase in [
        "production runtime approved",
        "production runtime approved without restrictions",
        "production runtime without restrictions approved",
        "production runtime is approved",
        "default memory approved",
        "default memory activated",
        "default memory approval granted",
        "route approved",
        "route approval",
        "route activation approved",
        "route activation approved not blocked",
        "final thesis accepted",
        "final thesis acceptance granted",
        "M63 accepted",
        "M63 approval",
        "production runtime approval",
    ] {
        let mut packet = tracked_packet_json();
        packet["next_steps"] = json!(format!("Continue after {phrase}."));
        assert!(
            matches!(
                DagDbContinuationPacket::parse_json(&packet.to_string()),
                Err(DagDbContinuationPacketError::ApprovalOverclaim { .. })
            ),
            "expected approval overclaim rejection for {phrase}"
        );
    }

    let mut canonical = parse_tracked_packet();
    canonical
        .non_claims
        .push("final thesis accepted".to_owned());
    canonical.non_claims.sort();
    assert!(matches!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::ApprovalOverclaim { .. })
    ));

    for valid_non_claim in [
        "production runtime not approved",
        "default memory activation blocked",
        "route activation blocked",
        "final thesis acceptance not approved",
    ] {
        let mut canonical = parse_tracked_packet();
        canonical.non_claims.push(valid_non_claim.to_owned());
        canonical.non_claims.sort();
        assert!(
            canonical.validate().is_ok(),
            "expected blocked non-claim to remain valid: {valid_non_claim}"
        );
    }
}

#[test]
fn continuation_forbidden_material_matching_is_case_insensitive() {
    for forbidden_material in [
        "Do not embed RAW_MARKDOWN in continuation packets.",
        "Never include sk-proj-example API keys.",
        "Drop Authorization headers from continuation packets.",
        "Do not include PASSWORD values.",
        "Redact every secret before handoff.",
        "Omit mysql://user:pass@example/db URLs.",
        "Omit sqlite://local.db URLs.",
        "Omit mongodb://example/db URLs.",
        "Omit redis://localhost URLs.",
    ] {
        let mut packet = tracked_packet_json();
        packet["next_steps"] = json!(forbidden_material);
        assert!(
            matches!(
                DagDbContinuationPacket::parse_json(&packet.to_string()),
                Err(DagDbContinuationPacketError::ForbiddenMaterial { .. })
            ),
            "expected forbidden material rejection for {forbidden_material}"
        );
    }

    let mut local_path = tracked_packet_json();
    local_path["boundary_warnings"] = json!([
        "gateway_offline",
        "repository_test_level_only",
        "writeback_blocked",
        "/USERS/example/leaked-path"
    ]);
    assert!(matches!(
        DagDbContinuationPacket::parse_json(&local_path.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial { .. })
    ));
}

#[test]
fn continuation_rejects_raw_source_body_leakage() {
    let mut packet = tracked_packet_json();
    packet["stopped_at"] =
        json!("user: paste the raw transcript\nassistant: here is the source body");

    assert!(matches!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial { field, .. })
            if field == "stopped_at"
    ));
}

#[test]
fn continuation_rejects_malformed_json_bad_schema_and_bad_report_counts() {
    assert!(matches!(
        DagDbContinuationPacket::parse_json("{not json"),
        Err(DagDbContinuationPacketError::Json { .. })
    ));

    let mut packet = tracked_packet_json();
    packet["schema_version"] = json!("wrong_schema");
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::SchemaVersion {
            actual: "wrong_schema".to_owned(),
        })
    );

    let mut report: JsonValue =
        serde_json::from_str(PYTHON_COMPAT_REPORT).expect("compat report json");
    report["packet_count"] = json!(1);
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "packet_count mismatch".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_unsupported_report_schema_and_invalid_report_material() {
    let mut report = tracked_report_json();
    report["schema_version"] = json!("unsupported_report_schema");
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "report schema_version mismatch".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["tenant_id"] = json!("other-tenant");
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::InvalidValue {
            field: "tenant_id".to_owned(),
            reason: "expected dag_db-local".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["valid_count"] = json!(1);
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "report contains invalid packets".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["invalid_count"] = json!(1);
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "report contains invalid packets".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["failure_codes"] = json!(["packet_failed"]);
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "report contains failure material".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["invalid_packets"] = json!([{"task_id": "bad"}]);
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "report contains failure material".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["source_loop_report_path"] =
        json!("docs/dagdb/repo-cleanup/sk-proj-example-loop-report.json");
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial {
            field: "source_loop_report_path".to_owned(),
            reason: "contains forbidden fragment sk-proj-".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["source_promotion_report_path"] =
        json!("docs/dagdb/repo-cleanup/postgres://user:pass@example/db");
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial {
            field: "source_promotion_report_path".to_owned(),
            reason: "contains forbidden fragment postgres://".to_owned(),
        })
    );

    let mut report = tracked_report_json();
    report["resume_round_trip"] = json!("missing");
    assert_eq!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ReportInvalid {
            reason: "resume_round_trip must be present".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_nested_forbidden_material_in_resume_round_trip() {
    // A nested string value carrying a forbidden fragment must be rejected.
    let mut report = tracked_report_json();
    report["resume_round_trip"] = json!({
        "round": 1,
        "context": {
            "items": ["clean", "database_url=postgres://user:pass@host/db"]
        }
    });
    assert!(matches!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial { .. })
    ));

    // A nested object KEY carrying a forbidden fragment must also be rejected.
    let mut report = tracked_report_json();
    report["resume_round_trip"] = json!({
        "round": 1,
        "Authorization": "redacted"
    });
    assert!(matches!(
        DagDbContinuationPacket::parse_report_json(&report.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial { .. })
    ));
}

#[test]
fn continuation_rejects_missing_empty_and_unknown_python_packet_fields() {
    let mut packet = tracked_packet_json();
    packet
        .as_object_mut()
        .expect("packet object")
        .remove("next_steps");
    assert!(matches!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::Json { .. })
    ));

    let mut packet = tracked_packet_json();
    packet
        .as_object_mut()
        .expect("packet object")
        .remove("blockers");
    assert!(matches!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::Json { .. })
    ));

    let mut packet = tracked_packet_json();
    packet["metadata"] = json!({"unexpected": true});
    assert!(matches!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::Json { .. })
    ));

    let mut packet = tracked_packet_json();
    packet["task_id"] = json!(" ");
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::EmptyField {
            field: "task_id".to_owned(),
        })
    );

    let mut packet = tracked_packet_json();
    packet["blockers"] = json!([""]);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::EmptyField {
            field: "blockers".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_duplicate_unsorted_refs_and_unsafe_changed_paths() {
    let mut packet = tracked_packet_json();
    packet["memory_ref_ids"] = json!([
        "1573772dd54b7b5b849895a75e966f76bc035f55ece4c17d251576b1da2ec1bd",
        "1573772dd54b7b5b849895a75e966f76bc035f55ece4c17d251576b1da2ec1bd"
    ]);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::ListNotSortedUnique {
            field: "memory_ref_ids".to_owned(),
        })
    );

    let mut packet = tracked_packet_json();
    packet["memory_ref_ids"] = json!([
        "331107b19df26e0bc6b016fc416205ed801e0b2e4e4b08338416083e06b3a074",
        "1573772dd54b7b5b849895a75e966f76bc035f55ece4c17d251576b1da2ec1bd"
    ]);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::ListNotSortedUnique {
            field: "memory_ref_ids".to_owned(),
        })
    );

    let mut canonical = parse_tracked_packet();
    canonical.memory_refs = vec!["b".to_owned(), "a".to_owned()];
    assert_eq!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::ListNotSortedUnique {
            field: "memory_refs".to_owned(),
        })
    );

    let mut packet = tracked_packet_json();
    packet["changed_paths"] = json!(["/tmp/source.md"]);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::InvalidValue {
            field: "changed_paths[0]".to_owned(),
            reason: "changed path must be repo-relative".to_owned(),
        })
    );

    let mut packet = tracked_packet_json();
    packet["changed_paths"] = json!(["docs\\source.md"]);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::InvalidValue {
            field: "changed_paths[0]".to_owned(),
            reason: "changed path must be repo-relative".to_owned(),
        })
    );

    let mut packet = tracked_packet_json();
    packet["changed_paths"] = json!(["~/leaked-source.md"]);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::InvalidValue {
            field: "changed_paths[0]".to_owned(),
            reason: "changed path must be repo-relative".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_token_estimate_and_relink_evidence_edge_cases() {
    let mut packet = tracked_packet_json();
    packet["token_estimate"] = json!(0);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::InvalidValue {
            field: "token_estimate".to_owned(),
            reason: "must match compact material estimate".to_owned(),
        })
    );

    let mut packet = tracked_packet_json();
    packet["blockers"] = json!(["Writeback blocked at repository test level; relink unavailable"]);
    refresh_token_estimate(&mut packet);
    assert!(
        DagDbContinuationPacket::parse_json(&packet.to_string()).is_ok(),
        "relink wording is accepted as blocked lineage evidence"
    );

    let mut packet = tracked_packet_json();
    packet["blockers"] = json!(["Writeback blocked at repository test level"]);
    refresh_token_estimate(&mut packet);
    assert_eq!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::RelinkEvidenceMissing)
    );
}

#[test]
fn continuation_rejects_source_body_leakage_by_size_and_line_count() {
    let mut packet = tracked_packet_json();
    packet["next_steps"] = json!("safe line\n".repeat(13));
    refresh_token_estimate(&mut packet);
    assert!(matches!(
        DagDbContinuationPacket::parse_json(&packet.to_string()),
        Err(DagDbContinuationPacketError::ForbiddenMaterial { field, .. })
            if field == "next_steps"
    ));

    let mut canonical = parse_tracked_packet();
    canonical.continuation_prompt = "a".repeat(4_001);
    assert!(matches!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::ForbiddenMaterial { field, .. })
            if field == "continuation_prompt"
    ));
}

#[test]
fn continuation_allows_negated_overclaim_terms_across_window_positions() {
    for valid_phrase in [
        "production runtime not approved",
        "production runtime approval is blocked",
        "route activation not approved",
        "no final thesis acceptance",
        "M63 approval denied",
        "approval remains blocked for default memory",
    ] {
        let mut canonical = parse_tracked_packet();
        canonical.continuation_prompt = format!("Continue because {valid_phrase}.");
        assert!(
            canonical.validate().is_ok(),
            "expected negated approval phrase to remain valid: {valid_phrase}"
        );
    }

    let mut canonical = parse_tracked_packet();
    canonical.continuation_prompt = "This mentions approval without a blocked surface.".to_owned();
    assert!(canonical.validate().is_ok());
}

#[test]
fn continuation_rejects_canonical_schema_and_empty_fields() {
    let mut canonical = parse_tracked_packet();
    canonical.schema_version = "dagdb_continuation_packet_v2".to_owned();
    assert_eq!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::SchemaVersion {
            actual: "dagdb_continuation_packet_v2".to_owned(),
        })
    );

    let mut canonical = parse_tracked_packet();
    canonical.packet_id = " ".to_owned();
    assert_eq!(
        canonical.validate(),
        Err(DagDbContinuationPacketError::EmptyField {
            field: "packet_id".to_owned(),
        })
    );
}
