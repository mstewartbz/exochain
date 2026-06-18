#![allow(clippy::expect_used)]

use std::{collections::BTreeMap, fs, path::Path};

use exo_dag_db_lab::benchmark_isolation::{
    BenchmarkRunMode, CONTROL_BENCH_DATABASE_URL_ALIAS_ENV, COST_BREAKDOWN_JSON,
    ContaminationEvidence, DatabaseSiloConfig, DatabaseSiloRole, GOVERNED_BENCH_DATABASE_URL_ENV,
    LATENCY_BREAKDOWN_JSON, MAIN_DATABASE_URL_ENV, NEUTRAL_BENCH_DATABASE_URL_ENV,
    PER_TASK_RESULTS_JSON, QUALITY_BREAKDOWN_JSON, RECOMMENDATIONS_MD, SUMMARY_JSON,
    SiloContaminationStatus, all_benchmark_run_modes, benchmark_silo_assignment,
    build_benchmark_isolation_report, build_contamination_check, default_silo_configs_from_env,
    render_benchmark_isolation_artifacts, unobservable_contamination_evidence,
    validate_benchmark_run_silo_assignment, validate_benchmark_silo_config,
};

fn env_with(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect()
}

fn lookup<'a>(env: &'a BTreeMap<String, String>) -> impl Fn(&str) -> Option<String> + 'a {
    move |key| env.get(key).cloned()
}

fn all_configured_env() -> BTreeMap<String, String> {
    env_with(&[
        (MAIN_DATABASE_URL_ENV, "postgres://main"),
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "postgres://neutral"),
        (GOVERNED_BENCH_DATABASE_URL_ENV, "postgres://governed"),
    ])
}

fn configs() -> Vec<DatabaseSiloConfig> {
    default_silo_configs_from_env(lookup(&all_configured_env()))
}

fn config_by_role(configs: &[DatabaseSiloConfig], role: DatabaseSiloRole) -> DatabaseSiloConfig {
    configs
        .iter()
        .find(|config| config.role == role)
        .cloned()
        .expect("silo role exists")
}

#[test]
fn benchmark_silo_roles_are_distinct() {
    let configs = configs();
    assert_eq!(configs[0].role, DatabaseSiloRole::Main);
    assert_eq!(configs[1].role, DatabaseSiloRole::NeutralBenchmark);
    assert_eq!(configs[2].role, DatabaseSiloRole::GovernedBenchmark);
}

#[test]
fn main_db_disallows_benchmark_writes() {
    let main = config_by_role(&configs(), DatabaseSiloRole::Main);
    assert!(!main.allow_benchmark_writes);
    assert!(!main.allow_synthetic_data);
}

#[test]
fn neutral_benchmark_disables_graph_features() {
    let neutral = config_by_role(&configs(), DatabaseSiloRole::NeutralBenchmark);
    assert!(!neutral.allow_graph_routing);
    assert!(!neutral.allow_route_memory);
    assert!(!neutral.allow_canonical_graph);
    assert!(!neutral.allow_context_packet_graph);
}

#[test]
fn governed_benchmark_enables_graph_features() {
    let governed = config_by_role(&configs(), DatabaseSiloRole::GovernedBenchmark);
    assert!(governed.allow_graph_routing);
    assert!(governed.allow_route_memory);
    assert!(governed.allow_canonical_graph);
    assert!(governed.allow_context_packet_graph);
}

#[test]
fn neutral_runners_assign_to_neutral_silo() {
    for run_mode in [
        BenchmarkRunMode::NeutralLongContext,
        BenchmarkRunMode::NeutralFlatRag,
    ] {
        assert_eq!(
            benchmark_silo_assignment(run_mode).required_silo_role,
            DatabaseSiloRole::NeutralBenchmark
        );
    }
}

#[test]
fn governed_runners_assign_to_governed_silo() {
    for run_mode in [
        BenchmarkRunMode::RawDagRouting,
        BenchmarkRunMode::GovernedDagdb,
        BenchmarkRunMode::GovernedDagdbOptimized,
    ] {
        assert_eq!(
            benchmark_silo_assignment(run_mode).required_silo_role,
            DatabaseSiloRole::GovernedBenchmark
        );
    }
}

#[test]
fn no_memory_assigns_to_neutral_silo() {
    assert_eq!(
        benchmark_silo_assignment(BenchmarkRunMode::NoMemoryLowerBound).required_silo_role,
        DatabaseSiloRole::NeutralBenchmark
    );
}

#[test]
fn main_db_not_assigned_to_benchmark_runners() {
    assert!(
        all_benchmark_run_modes()
            .iter()
            .all(|mode| benchmark_silo_assignment(*mode).required_silo_role
                != DatabaseSiloRole::Main)
    );
}

#[test]
fn neutral_runner_rejected_from_governed_silo() {
    let governed = config_by_role(&configs(), DatabaseSiloRole::GovernedBenchmark);
    assert!(
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::NeutralLongContext, &governed)
            .is_err()
    );
}

#[test]
fn governed_runner_rejected_from_neutral_silo() {
    let neutral = config_by_role(&configs(), DatabaseSiloRole::NeutralBenchmark);
    assert!(
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::GovernedDagdb, &neutral).is_err()
    );
}

#[test]
fn benchmark_runner_rejected_from_main_silo() {
    let main = config_by_role(&configs(), DatabaseSiloRole::Main);
    assert!(
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::GovernedDagdb, &main).is_err()
    );
}

#[test]
fn run_guard_blocks_neutral_graph_routing() {
    let mut neutral = config_by_role(&configs(), DatabaseSiloRole::NeutralBenchmark);
    neutral.allow_graph_routing = true;
    assert!(
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::NeutralLongContext, &neutral)
            .is_err()
    );
}

#[test]
fn run_guard_blocks_governed_without_graph_routing() {
    let mut governed = config_by_role(&configs(), DatabaseSiloRole::GovernedBenchmark);
    governed.allow_graph_routing = false;
    assert!(
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::GovernedDagdb, &governed).is_err()
    );
}

#[test]
fn run_guard_accepts_valid_neutral_and_governed_assignments() {
    let current = configs();
    let neutral = config_by_role(&current, DatabaseSiloRole::NeutralBenchmark);
    let governed = config_by_role(&current, DatabaseSiloRole::GovernedBenchmark);
    let neutral_assignment =
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::NeutralFlatRag, &neutral)
            .expect("neutral guard");
    let governed_assignment =
        validate_benchmark_run_silo_assignment(BenchmarkRunMode::GovernedDagdbOptimized, &governed)
            .expect("governed guard");
    assert_eq!(
        neutral_assignment.required_silo_role,
        DatabaseSiloRole::NeutralBenchmark
    );
    assert_eq!(
        governed_assignment.required_silo_role,
        DatabaseSiloRole::GovernedBenchmark
    );
}

#[test]
fn run_guard_blocks_each_neutral_graph_feature() {
    let mutators: [fn(&mut DatabaseSiloConfig); 4] = [
        |config: &mut DatabaseSiloConfig| config.allow_graph_routing = true,
        |config: &mut DatabaseSiloConfig| config.allow_route_memory = true,
        |config: &mut DatabaseSiloConfig| config.allow_canonical_graph = true,
        |config: &mut DatabaseSiloConfig| config.allow_context_packet_graph = true,
    ];
    for mutate in mutators {
        let mut neutral = config_by_role(&configs(), DatabaseSiloRole::NeutralBenchmark);
        mutate(&mut neutral);
        assert!(
            validate_benchmark_run_silo_assignment(BenchmarkRunMode::NeutralLongContext, &neutral)
                .is_err()
        );
    }
}

#[test]
fn validation_reports_missing_and_mutating_main_silos() {
    let env = all_configured_env();
    let mut current = configs();
    current.retain(|config| config.role != DatabaseSiloRole::GovernedBenchmark);
    current[0].allow_benchmark_writes = true;
    let validation =
        validate_benchmark_silo_config(&current, &all_benchmark_run_modes(), lookup(&env));
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .contains(&"main_database_allows_benchmark_mutation".into())
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("missing_silo_role"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("missing_assignment_silo"))
    );
}

#[test]
fn validation_reports_neutral_and_governed_feature_mismatch() {
    let env = all_configured_env();
    let mut current = configs();
    let neutral = current
        .iter_mut()
        .find(|config| config.role == DatabaseSiloRole::NeutralBenchmark)
        .expect("neutral");
    neutral.allow_graph_routing = true;
    neutral.allow_route_memory = true;
    neutral.allow_canonical_graph = true;
    neutral.allow_context_packet_graph = true;
    let governed = current
        .iter_mut()
        .find(|config| config.role == DatabaseSiloRole::GovernedBenchmark)
        .expect("governed");
    governed.allow_graph_routing = false;
    let validation =
        validate_benchmark_silo_config(&current, &all_benchmark_run_modes(), lookup(&env));
    for expected in [
        "neutral_benchmark_graph_routing_enabled",
        "neutral_benchmark_route_memory_enabled",
        "neutral_benchmark_canonical_graph_enabled",
        "neutral_benchmark_context_packet_graph_enabled",
        "governed_benchmark_graph_routing_disabled",
    ] {
        assert!(validation.errors.contains(&expected.into()));
    }
}

#[test]
fn configured_benchmark_silo_requires_tenant_and_namespace() {
    let env = all_configured_env();
    let mut current = configs();
    let neutral = current
        .iter_mut()
        .find(|config| config.role == DatabaseSiloRole::NeutralBenchmark)
        .expect("neutral");
    neutral.tenant_id.clear();
    neutral.namespace.clear();
    let validation =
        validate_benchmark_silo_config(&current, &all_benchmark_run_modes(), lookup(&env));
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("configured_benchmark_silo_missing_tenant"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("configured_benchmark_silo_missing_namespace"))
    );
}

#[test]
fn neutral_env_var_precedes_control_alias() {
    let env = env_with(&[
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "postgres://neutral"),
        (CONTROL_BENCH_DATABASE_URL_ALIAS_ENV, "postgres://control"),
    ]);
    let neutral = config_by_role(
        &default_silo_configs_from_env(lookup(&env)),
        DatabaseSiloRole::NeutralBenchmark,
    );
    assert_eq!(neutral.database_url_env, NEUTRAL_BENCH_DATABASE_URL_ENV);
}

#[test]
fn control_alias_maps_to_neutral_when_preferred_missing() {
    let env = env_with(&[(CONTROL_BENCH_DATABASE_URL_ALIAS_ENV, "postgres://control")]);
    let neutral = config_by_role(
        &default_silo_configs_from_env(lookup(&env)),
        DatabaseSiloRole::NeutralBenchmark,
    );
    assert_eq!(
        neutral.database_url_env,
        CONTROL_BENCH_DATABASE_URL_ALIAS_ENV
    );
    assert!(neutral.configured);
}

#[test]
fn serialized_output_uses_neutral_not_control() {
    let json = serde_json::to_string(&configs()).expect("json");
    assert!(json.contains("neutral_benchmark"));
    assert!(!json.contains("control_benchmark"));
}

#[test]
fn identical_neutral_and_governed_urls_fail_validation() {
    let env = env_with(&[
        (MAIN_DATABASE_URL_ENV, "postgres://main"),
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "postgres://same"),
        (GOVERNED_BENCH_DATABASE_URL_ENV, "postgres://same"),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .contains(&"neutral_and_governed_database_urls_match".into())
    );
}

#[test]
fn main_db_equal_to_benchmark_url_fails_when_benchmark_writes_enabled() {
    let env = env_with(&[
        (MAIN_DATABASE_URL_ENV, "postgres://same"),
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "postgres://same"),
        (GOVERNED_BENCH_DATABASE_URL_ENV, "postgres://governed"),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .contains(&"main_and_benchmark_database_urls_match".into())
    );
}

#[test]
fn identical_urls_fail_without_serializing_url() {
    let env = env_with(&[
        (
            NEUTRAL_BENCH_DATABASE_URL_ENV,
            "postgres://user:secret@host/db",
        ),
        (
            GOVERNED_BENCH_DATABASE_URL_ENV,
            "postgres://user:secret@host/db",
        ),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    let serialized = serde_json::to_string(&validation).expect("json");
    assert!(!validation.valid);
    assert!(!serialized.contains("postgres://"));
    assert!(!serialized.contains("secret@host"));
}

#[test]
fn validation_errors_do_not_contain_url_material() {
    let env = env_with(&[
        (MAIN_DATABASE_URL_ENV, "postgres://user:secret@host/db"),
        (
            NEUTRAL_BENCH_DATABASE_URL_ENV,
            "postgres://user:secret@host/db",
        ),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(validation.errors.iter().all(|error| {
        !error.contains("postgres://") && !error.contains("secret") && !error.contains("host")
    }));
}

#[test]
fn missing_db_urls_enter_demo_mode_warning() {
    let env = BTreeMap::new();
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(
        validation
            .warnings
            .contains(&"deterministic_report_file_mode".into())
    );
}

#[test]
fn missing_url_evidence_does_not_claim_separation() {
    let env = BTreeMap::new();
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(!validation.neutral_governed_separated);
    assert!(
        validation
            .warnings
            .contains(&"neutral_governed_separation_unobservable".into())
    );
}

#[test]
fn partial_url_evidence_does_not_claim_separation() {
    let env = env_with(&[(MAIN_DATABASE_URL_ENV, "postgres://main")]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(!validation.neutral_governed_separated);
    assert!(
        validation
            .warnings
            .contains(&"neutral_governed_separation_unobservable".into())
    );
}

#[test]
fn equivalent_urls_differing_only_textually_are_not_separated() {
    // Same database reached via scheme alias, default vs explicit port, host case,
    // and reordered query params. A raw string compare would call these "separated";
    // canonicalization must detect that they point at the SAME database.
    let env = env_with(&[
        (
            NEUTRAL_BENCH_DATABASE_URL_ENV,
            "postgres://user:pw@DB.Example:5432/bench?sslmode=require&app=x",
        ),
        (
            GOVERNED_BENCH_DATABASE_URL_ENV,
            "postgresql://user:pw@db.example/bench?app=x&sslmode=require",
        ),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(
        !validation.neutral_governed_separated,
        "equivalent neutral/governed URLs must NOT be reported as separated"
    );
    assert!(
        validation
            .errors
            .contains(&"neutral_and_governed_database_urls_match".into())
    );
    // The error envelope must never leak the raw URL material.
    assert!(validation.errors.iter().all(|error| {
        !error.contains("db.example") && !error.contains("pw") && !error.contains("postgres")
    }));
}

#[test]
fn unparseable_silo_url_is_a_hard_error_not_a_warning() {
    let env = env_with(&[
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "not-a-valid-url"),
        (GOVERNED_BENCH_DATABASE_URL_ENV, "postgres://governed"),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(
        !validation.valid,
        "an unparseable silo URL must fail validation"
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.starts_with("benchmark_silo_url_unparseable")),
        "an unparseable URL must be a hard error, not a soft warning"
    );
    assert!(!validation.neutral_governed_separated);
}

#[test]
fn unobservable_main_url_with_benchmark_writes_is_flagged() {
    let env = env_with(&[
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "postgres://neutral"),
        (GOVERNED_BENCH_DATABASE_URL_ENV, "postgres://governed"),
    ]);
    let configs = default_silo_configs_from_env(lookup(&env));
    let validation =
        validate_benchmark_silo_config(&configs, &all_benchmark_run_modes(), lookup(&env));
    assert!(validation.neutral_governed_separated);
    assert!(
        validation
            .warnings
            .contains(&"main_benchmark_separation_unobservable".into())
    );
}

#[test]
fn db_urls_are_not_serialized() {
    let env = env_with(&[
        (
            MAIN_DATABASE_URL_ENV,
            "postgres://user:password@example.invalid/main?secret=true",
        ),
        (
            NEUTRAL_BENCH_DATABASE_URL_ENV,
            "postgres://user:password@example.invalid/neutral?secret=true",
        ),
        (
            GOVERNED_BENCH_DATABASE_URL_ENV,
            "postgres://user:password@example.invalid/governed?secret=true",
        ),
    ]);
    let report = build_benchmark_isolation_report(
        Path::new("target/dagdb/end_to_end_diagnostics"),
        "test_run".into(),
        lookup(&env),
    );
    let serialized = serde_json::to_string(&report).expect("json");
    assert!(!serialized.contains("postgres://"));
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("example.invalid"));
    assert!(!serialized.contains("\"database_url\""));
    assert!(!serialized.contains("connection_string"));
}

#[test]
fn clean_requires_observable_evidence() {
    let check = build_contamination_check(
        "run".into(),
        &ContaminationEvidence {
            demo_mode: false,
            main_db_used_for_benchmark: Some(false),
            neutral_used_graph_routing: None,
            governed_used_neutral_state: None,
            cross_silo_route_memory_reuse: None,
            cross_silo_context_packet_reuse: None,
            cross_silo_receipt_reuse: None,
        },
    );
    assert_eq!(
        check.main_db_used_for_benchmark,
        SiloContaminationStatus::Clean
    );
    assert_eq!(
        check.neutral_used_graph_routing,
        SiloContaminationStatus::UnknownNotObservable
    );
}

#[test]
fn unobservable_fields_never_report_clean() {
    let check =
        build_contamination_check("run".into(), &unobservable_contamination_evidence(false));
    assert_eq!(
        check.main_db_used_for_benchmark,
        SiloContaminationStatus::UnknownNotObservable
    );
    assert_eq!(
        check.cross_silo_receipt_reuse,
        SiloContaminationStatus::UnknownNotObservable
    );
}

#[test]
fn demo_mode_uses_not_configured_demo_mode() {
    let check = build_contamination_check("run".into(), &unobservable_contamination_evidence(true));
    assert_eq!(
        check.main_db_used_for_benchmark,
        SiloContaminationStatus::NotConfiguredDemoMode
    );
}

#[test]
fn contamination_unknown_when_artifacts_lack_evidence() {
    let check =
        build_contamination_check("run".into(), &unobservable_contamination_evidence(false));
    assert!(
        check
            .findings
            .contains(&"one_or_more_fields_unknown_not_observable".into())
    );
}

#[test]
fn contamination_clean_only_when_observable_and_clean() {
    let check = build_contamination_check(
        "run".into(),
        &ContaminationEvidence {
            demo_mode: false,
            main_db_used_for_benchmark: Some(false),
            neutral_used_graph_routing: Some(false),
            governed_used_neutral_state: Some(false),
            cross_silo_route_memory_reuse: Some(false),
            cross_silo_context_packet_reuse: Some(false),
            cross_silo_receipt_reuse: Some(false),
        },
    );
    assert!(!check.contamination_detected);
    assert_eq!(
        check.cross_silo_receipt_reuse,
        SiloContaminationStatus::Clean
    );
}

#[test]
fn contamination_detected_when_observable_evidence_proves_contamination() {
    let check = build_contamination_check(
        "run".into(),
        &ContaminationEvidence {
            demo_mode: false,
            main_db_used_for_benchmark: Some(false),
            neutral_used_graph_routing: Some(true),
            governed_used_neutral_state: Some(false),
            cross_silo_route_memory_reuse: Some(false),
            cross_silo_context_packet_reuse: Some(false),
            cross_silo_receipt_reuse: Some(false),
        },
    );
    assert!(check.contamination_detected);
    assert_eq!(
        check.neutral_used_graph_routing,
        SiloContaminationStatus::Contaminated
    );
}

#[test]
fn runner_silo_assignment_report_is_deterministic() {
    let report = build_benchmark_isolation_report(
        Path::new("target/dagdb/end_to_end_diagnostics"),
        "run".into(),
        |_| None,
    );
    let first = render_benchmark_isolation_artifacts(&report)
        .expect("first")
        .runner_silo_assignment_json;
    let second = render_benchmark_isolation_artifacts(&report)
        .expect("second")
        .runner_silo_assignment_json;
    assert_eq!(first, second);
}

#[test]
fn silo_validation_report_omits_secrets() {
    let env = env_with(&[
        (
            NEUTRAL_BENCH_DATABASE_URL_ENV,
            "postgres://user:secret@host/db",
        ),
        (
            GOVERNED_BENCH_DATABASE_URL_ENV,
            "postgres://user:secret@host/db",
        ),
    ]);
    let report = build_benchmark_isolation_report(
        Path::new("target/dagdb/end_to_end_diagnostics"),
        "run".into(),
        lookup(&env),
    );
    let artifacts = render_benchmark_isolation_artifacts(&report).expect("artifacts");
    assert!(
        !artifacts
            .silo_validation_report_json
            .contains("postgres://")
    );
    assert!(!artifacts.silo_validation_report_json.contains("secret"));
}

#[test]
fn isolation_markdown_covers_empty_and_non_empty_validation_sections() {
    let clean_report = build_benchmark_isolation_report(
        Path::new("target/dagdb/end_to_end_diagnostics"),
        "run".into(),
        lookup(&all_configured_env()),
    );
    let clean_markdown = render_benchmark_isolation_artifacts(&clean_report)
        .expect("clean artifacts")
        .benchmark_isolation_summary_md;
    assert!(clean_markdown.contains("## Validation Errors\n\n- none"));
    // With distinct, parseable silo URLs there is nothing to warn about: the
    // separation is normalized and proven, not deferred.
    assert!(clean_markdown.contains("## Validation Warnings\n\n- none"));

    let bad_env = env_with(&[
        (NEUTRAL_BENCH_DATABASE_URL_ENV, "postgres://same"),
        (GOVERNED_BENCH_DATABASE_URL_ENV, "postgres://same"),
    ]);
    let bad_report = build_benchmark_isolation_report(
        Path::new("target/dagdb/end_to_end_diagnostics"),
        "run".into(),
        lookup(&bad_env),
    );
    let bad_markdown = render_benchmark_isolation_artifacts(&bad_report)
        .expect("bad artifacts")
        .benchmark_isolation_summary_md;
    assert!(bad_markdown.contains("- neutral_and_governed_database_urls_match"));
}

#[test]
fn isolation_report_references_source_artifact_hashes() {
    let dir = std::env::temp_dir().join("exo_dagdb_isolation_hashes");
    fs::create_dir_all(&dir).expect("dir");
    fs::write(dir.join(SUMMARY_JSON), r#"{"fixture_ids":["fixture-a"]}"#).expect("summary");
    fs::write(dir.join(PER_TASK_RESULTS_JSON), "[]").expect("per task");
    fs::write(dir.join(COST_BREAKDOWN_JSON), "[]").expect("cost");
    fs::write(dir.join(QUALITY_BREAKDOWN_JSON), "[]").expect("quality");
    fs::write(dir.join(LATENCY_BREAKDOWN_JSON), "[]").expect("latency");
    fs::write(dir.join(RECOMMENDATIONS_MD), "# Recommendations\n").expect("recommendations");
    let report = build_benchmark_isolation_report(&dir, "run".into(), |_| None);
    assert_eq!(report.fixture_id, "fixture-a");
    assert_eq!(report.diagnostic_summary_hash.status, "available");
    assert!(report.diagnostic_summary_hash.hash.is_some());
    assert_eq!(report.source_artifact_paths.len(), 6);
}

#[test]
fn missing_artifact_hash_is_reported_as_missing() {
    let dir = std::env::temp_dir().join("exo_dagdb_isolation_missing_hash");
    fs::create_dir_all(&dir).expect("dir");
    fs::write(dir.join(SUMMARY_JSON), r#"{"fixture_ids":["fixture-a"]}"#).expect("summary");
    let report = build_benchmark_isolation_report(&dir, "run".into(), |_| None);
    assert_eq!(report.per_task_results_hash.status, "missing");
    assert!(report.per_task_results_hash.hash.is_none());
}
