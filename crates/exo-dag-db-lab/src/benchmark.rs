//! Deterministic synthetic benchmark fixtures and runners for DAG DB.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::{Hash256, hash::hash_structured};
use exo_dag_db_api::{RiskClass, SourceType, ValidationStatus};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::scoring::{BenchmarkGateInput, DomainError, evaluate_benchmark_gates};

/// Benchmark fixtures are the only DAG DB path where raw synthetic text is allowed.
pub const BENCHMARK_FIXTURE_ROOT: &str = "crates/exo-dag-db-lab/fixtures/benchmarks/";

const TOTAL_CORPUS_MIN: usize = 120;
const PUBLIC_MIN: usize = 15;
const PRIVATE_CUSTOMER_MIN: usize = 15;
const IP_SENSITIVE_MIN: usize = 10;
const GENERATED_MIN: usize = 10;
const OPEN_SOURCE_MIN: usize = 10;
const UNKNOWN_PROVENANCE_MIN: usize = 10;
const STALE_MIN: usize = 10;
const REVOKED_MIN: usize = 10;
const CONTRADICTED_MIN: usize = 10;
const DUPLICATE_MIN: usize = 10;
const APPROVAL_REQUIRED_TASK_MIN: usize = 15;
const TOKENIZER_CONFIG: &str = "exo-dagdb-benchmark-tokenizer-v1";
const TEMPERATURE_BP: u16 = 0;
const TOP_P_BP: u16 = 10_000;
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 256;
const OPTIMIZED_CONTEXT_TOKENS_PER_REF: u32 = 6;
const OPTIMIZED_OVERHEAD_TOKENS_PER_TASK: u32 = 16;
const SCALE_FIXTURE_ID: &str = "dagdb_scale_10x_v1";
const SCALE_DETERMINISTIC_SEED: u64 = 424_243;
const SCALE_CORPUS_COUNT: u32 = 1_200;
const SCALE_TASK_COUNT: u32 = 150;
const SCALE_TENANT_COUNT: u32 = 3;
const SCALE_NAMESPACE_COUNT: u32 = 6;
const SCALE_FRESH_VALID_COUNT: u32 = 720;
const SCALE_STALE_COUNT: u32 = 180;
const SCALE_REVOKED_COUNT: u32 = 120;
const SCALE_CONTRADICTED_COUNT: u32 = 120;
const SCALE_DUPLICATE_COUNT: u32 = 60;

/// Benchmark fixture file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkFixture {
    pub fixture_id: String,
    pub deterministic_seed: u64,
    pub corpus: Vec<BenchmarkCorpusItem>,
    pub tasks: Vec<BenchmarkTask>,
}

/// Synthetic corpus item for benchmark-only raw text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkCorpusItem {
    pub corpus_item_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub source_type: SourceType,
    pub payload_hash: String,
    pub source_hash: String,
    pub title_text: String,
    pub summary_text: String,
    pub risk_class: RiskClass,
    pub expected_validation_status: ValidationStatus,
    pub labels: Vec<String>,
    pub memory_id: Option<String>,
    pub revoked: bool,
    pub stale: bool,
    pub contradicts: Vec<String>,
    pub duplicates: Vec<String>,
}

/// Synthetic benchmark task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkTask {
    pub task_id: String,
    pub question_text: String,
    pub task_signature_hash: String,
    pub expected_citations: Vec<String>,
    pub allowed_memory_ids: Vec<String>,
    pub prohibited_memory_ids: Vec<String>,
    #[serde(default)]
    pub expected_citation_ids: Vec<String>,
    #[serde(default)]
    pub prohibited_ref_ids: Vec<String>,
    #[serde(default)]
    pub contradiction_ref_ids: Vec<String>,
    pub risk_labels: Vec<RiskClass>,
    pub expected_validation_outcome: ValidationStatus,
}

/// Deterministic runner names persisted in `dagdb_benchmark_runs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkRunnerName {
    NoMemory,
    LongContextDump,
    FlatRag,
    DagDbRouting,
    GovernedDagDbRouting,
    GovernedDagDbOptimized,
}

impl BenchmarkRunnerName {
    /// All MVP runners in deterministic order.
    #[must_use]
    pub const fn all() -> [Self; 6] {
        [
            Self::NoMemory,
            Self::LongContextDump,
            Self::FlatRag,
            Self::DagDbRouting,
            Self::GovernedDagDbRouting,
            Self::GovernedDagDbOptimized,
        ]
    }
}

/// Deterministic route budget used by the optimized runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteBudgetPolicy {
    pub max_refs_per_task: u32,
    pub max_required_citations_per_task: u32,
    pub max_contradictions_per_task: u32,
    pub max_related_refs_per_task: u32,
    pub max_context_tokens_per_task: u32,
}

impl RouteBudgetPolicy {
    /// Locked MVP optimized policy.
    #[must_use]
    pub const fn optimized_mvp() -> Self {
        Self {
            max_refs_per_task: 4,
            max_required_citations_per_task: 3,
            max_contradictions_per_task: 1,
            max_related_refs_per_task: 1,
            max_context_tokens_per_task: 24,
        }
    }
}

/// Evidence-derived quality components for one task or aggregated fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceQualityBreakdown {
    pub required_citation_recall_bp: u16,
    pub selected_ref_precision_bp: u16,
    pub prohibited_ref_rejection_bp: u16,
    pub contradiction_exposure_bp: u16,
    pub validation_pass_bp: u16,
    pub freshness_bp: u16,
    pub quality_score_bp: u16,
    pub citation_accuracy_bp: u16,
    pub unsupported_claim_rate_bp: u16,
    pub claim_allowed: bool,
}

/// Deterministic scale fixture configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaleFixtureConfig {
    pub fixture_id: String,
    pub deterministic_seed: u64,
    pub corpus_count: u32,
    pub task_count: u32,
    pub tenant_count: u32,
    pub namespace_count: u32,
}

impl ScaleFixtureConfig {
    /// Locked scale fixture configuration.
    #[must_use]
    pub fn locked() -> Self {
        Self {
            fixture_id: SCALE_FIXTURE_ID.into(),
            deterministic_seed: SCALE_DETERMINISTIC_SEED,
            corpus_count: SCALE_CORPUS_COUNT,
            task_count: SCALE_TASK_COUNT,
            tenant_count: SCALE_TENANT_COUNT,
            namespace_count: SCALE_NAMESPACE_COUNT,
        }
    }
}

/// Required overhead components before a savings claim can be reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkOverheadAccounting {
    pub route_scoring_micro_exo: u64,
    pub validation_micro_exo: u64,
    pub redaction_micro_exo: u64,
    pub idempotency_lookup_micro_exo: u64,
    pub postgres_query_micro_exo: u64,
    pub dag_outbox_enqueue_micro_exo: u64,
    pub context_packet_micro_exo: u64,
    pub prompt_context_micro_exo: u64,
}

impl BenchmarkOverheadAccounting {
    /// Sum every overhead component with saturating integer arithmetic.
    #[must_use]
    pub fn total_micro_exo(self) -> u64 {
        self.route_scoring_micro_exo
            .saturating_add(self.validation_micro_exo)
            .saturating_add(self.redaction_micro_exo)
            .saturating_add(self.idempotency_lookup_micro_exo)
            .saturating_add(self.postgres_query_micro_exo)
            .saturating_add(self.dag_outbox_enqueue_micro_exo)
            .saturating_add(self.context_packet_micro_exo)
            .saturating_add(self.prompt_context_micro_exo)
    }
}

/// Deterministic benchmark runner report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkRunReport {
    pub fixture_id: String,
    pub runner_name: BenchmarkRunnerName,
    pub deterministic_seed: u64,
    pub config_hash: String,
    pub tokenizer_config_hash: String,
    pub temperature_bp: u16,
    pub top_p_bp: u16,
    pub max_output_tokens: u32,
    pub selected_memory_ids_by_task: BTreeMap<String, Vec<String>>,
    pub quality_score_bp: u16,
    pub citation_accuracy_bp: u16,
    pub unsupported_claim_rate_bp: u16,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub overhead_tokens: u32,
    pub gross_savings_micro_exo: u64,
    pub overhead: BenchmarkOverheadAccounting,
    pub overhead_micro_exo: u64,
    pub net_savings_micro_exo: u64,
    pub savings_claim_allowed: bool,
    pub output_hash: String,
}

/// Benchmark harness failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BenchmarkError {
    #[error("invalid_benchmark_fixture: {reason}")]
    InvalidFixture { reason: String },
    #[error("invalid_runner_output: {reason}")]
    InvalidRunnerOutput { reason: String },
    #[error("benchmark_json_invalid: {reason}")]
    Json { reason: String },
    #[error(transparent)]
    Scoring(#[from] DomainError),
}

/// Parse and validate a benchmark fixture from JSON.
pub fn load_benchmark_fixture_json(json: &str) -> Result<BenchmarkFixture, BenchmarkError> {
    let fixture: BenchmarkFixture =
        serde_json::from_str(json).map_err(|error| BenchmarkError::Json {
            reason: error.to_string(),
        })?;
    validate_benchmark_fixture(&fixture)?;
    Ok(fixture)
}

/// Enforce the benchmark fixture schema and minimum corpus composition.
pub fn validate_benchmark_fixture(fixture: &BenchmarkFixture) -> Result<(), BenchmarkError> {
    require_non_empty("fixture_id", &fixture.fixture_id)?;
    if fixture.corpus.len() < TOTAL_CORPUS_MIN {
        return invalid_fixture(format!(
            "corpus_item_count {} below {}",
            fixture.corpus.len(),
            TOTAL_CORPUS_MIN
        ));
    }
    if fixture.tasks.is_empty() {
        return invalid_fixture("tasks must not be empty");
    }

    let mut corpus_ids = BTreeSet::new();
    let mut memory_ids = BTreeSet::new();
    let mut source_counts = BTreeMap::new();
    let mut stale_count = 0usize;
    let mut revoked_count = 0usize;
    let mut contradicted_count = 0usize;
    let mut duplicate_count = 0usize;

    for item in &fixture.corpus {
        require_non_empty("corpus_item_id", &item.corpus_item_id)?;
        require_non_empty("tenant_id", &item.tenant_id)?;
        require_non_empty("namespace", &item.namespace)?;
        require_non_empty("title_text", &item.title_text)?;
        require_non_empty("summary_text", &item.summary_text)?;
        validate_hash_hex("payload_hash", &item.payload_hash)?;
        validate_hash_hex("source_hash", &item.source_hash)?;
        if !corpus_ids.insert(item.corpus_item_id.clone()) {
            return invalid_fixture(format!("duplicate corpus_item_id {}", item.corpus_item_id));
        }
        let memory_id = item_memory_id(item);
        validate_hash_hex("memory_id", memory_id)?;
        if !memory_ids.insert(memory_id.to_owned()) {
            return invalid_fixture(format!("duplicate memory_id {memory_id}"));
        }
        *source_counts.entry(item.source_type).or_insert(0usize) += 1;
        stale_count += usize::from(item.stale);
        revoked_count += usize::from(item.revoked);
        contradicted_count += usize::from(!item.contradicts.is_empty());
        duplicate_count += usize::from(!item.duplicates.is_empty());
    }

    require_count(
        "public_web",
        *source_counts.get(&SourceType::PublicWeb).unwrap_or(&0),
        PUBLIC_MIN,
    )?;
    require_count(
        "private_customer",
        *source_counts
            .get(&SourceType::PrivateCustomer)
            .unwrap_or(&0),
        PRIVATE_CUSTOMER_MIN,
    )?;
    require_count(
        "ip_sensitive",
        *source_counts.get(&SourceType::IpSensitive).unwrap_or(&0),
        IP_SENSITIVE_MIN,
    )?;
    require_count(
        "generated",
        *source_counts.get(&SourceType::Generated).unwrap_or(&0),
        GENERATED_MIN,
    )?;
    require_count(
        "open_source",
        *source_counts.get(&SourceType::OpenSource).unwrap_or(&0),
        OPEN_SOURCE_MIN,
    )?;
    require_count(
        "unknown_provenance",
        *source_counts
            .get(&SourceType::UnknownProvenance)
            .unwrap_or(&0),
        UNKNOWN_PROVENANCE_MIN,
    )?;
    require_count("stale", stale_count, STALE_MIN)?;
    require_count("revoked", revoked_count, REVOKED_MIN)?;
    require_count("contradicted", contradicted_count, CONTRADICTED_MIN)?;
    require_count("duplicate", duplicate_count, DUPLICATE_MIN)?;

    let mut approval_required_tasks = 0usize;
    for task in &fixture.tasks {
        require_non_empty("task_id", &task.task_id)?;
        require_non_empty("question_text", &task.question_text)?;
        validate_hash_hex("task_signature_hash", &task.task_signature_hash)?;
        for memory_id in task
            .allowed_memory_ids
            .iter()
            .chain(task.prohibited_memory_ids.iter())
        {
            validate_hash_hex("task_memory_id", memory_id)?;
        }
        for citation in &task.expected_citations {
            if !corpus_ids.contains(citation) {
                return invalid_fixture(format!("unknown expected citation {citation}"));
            }
        }
        approval_required_tasks += usize::from(
            task.risk_labels
                .iter()
                .any(|risk| matches!(risk, RiskClass::R3 | RiskClass::R4 | RiskClass::R5)),
        );
    }
    require_count(
        "approval_required_tasks",
        approval_required_tasks,
        APPROVAL_REQUIRED_TASK_MIN,
    )?;
    Ok(())
}

/// Assert that raw fixture text is only read from the benchmark fixture path.
pub fn validate_raw_fixture_text_path(path: &str) -> Result<(), BenchmarkError> {
    if path.starts_with(BENCHMARK_FIXTURE_ROOT) {
        Ok(())
    } else {
        invalid_fixture(format!(
            "raw fixture text outside benchmark fixture path: {path}"
        ))
    }
}

/// Verify that MVP optimized evidence fields are present.
pub fn audit_mvp_fixture_evidence_fields(fixture: &BenchmarkFixture) -> Result<(), BenchmarkError> {
    let mut missing = Vec::new();
    for task in &fixture.tasks {
        if task.expected_citation_ids.is_empty() {
            missing.push(format!("{}.expected_citation_ids", task.task_id));
        }
        if task.prohibited_ref_ids.is_empty() {
            missing.push(format!("{}.prohibited_ref_ids", task.task_id));
        }
        if task.contradiction_ref_ids.is_empty() {
            missing.push(format!("{}.contradiction_ref_ids", task.task_id));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        invalid_fixture(format!(
            "mvp evidence fields missing: {}",
            missing.join(", ")
        ))
    }
}

/// Populate MVP evidence fields from the locked source fields.
#[must_use]
pub fn populate_mvp_evidence_fields(fixture: &mut BenchmarkFixture) -> Vec<String> {
    let mut note_task_ids = Vec::new();
    for task in &mut fixture.tasks {
        task.expected_citation_ids = task.expected_citations.clone();
        task.prohibited_ref_ids = task.prohibited_memory_ids.clone();
        task.contradiction_ref_ids = task
            .allowed_memory_ids
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        if task.expected_citations.is_empty()
            || task.allowed_memory_ids.is_empty()
            || task.prohibited_memory_ids.is_empty()
        {
            note_task_ids.push(task.task_id.clone());
        }
    }
    note_task_ids
}

/// Generate the locked deterministic 10x scale benchmark fixture.
#[must_use]
pub fn generate_scale_fixture() -> BenchmarkFixture {
    let config = ScaleFixtureConfig::locked();
    let mut corpus = Vec::new();
    for index in 0..config.corpus_count {
        corpus.push(scale_corpus_item(index, &config));
    }
    let mut tasks = Vec::new();
    for index in 0..config.task_count {
        tasks.push(scale_task(index, &config, &corpus));
    }
    BenchmarkFixture {
        fixture_id: config.fixture_id,
        deterministic_seed: config.deterministic_seed,
        corpus,
        tasks,
    }
}

/// Compute evidence quality for one task and deterministic selected refs.
#[must_use]
pub fn evidence_quality_for_task(
    fixture: &BenchmarkFixture,
    task: &BenchmarkTask,
    selected_refs: &[String],
    citation_refs: &[String],
    validation_pass_bp: u16,
) -> EvidenceQualityBreakdown {
    let expected_refs = sorted_unique_strings(evidence_refs_to_memory_ids(
        fixture,
        &task.expected_citation_ids,
    ));
    let prohibited_refs = sorted_unique_strings(evidence_refs_to_memory_ids(
        fixture,
        &task.prohibited_ref_ids,
    ));
    let contradiction_refs = sorted_unique_strings(evidence_refs_to_memory_ids(
        fixture,
        &task.contradiction_ref_ids,
    ));
    let selected = selected_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let citations = citation_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    let expected_selected = expected_refs
        .iter()
        .filter(|expected| selected.contains(expected.as_str()))
        .count();
    let required_citation_recall_bp = ratio_bp_or(expected_selected, expected_refs.len(), 10_000);

    let expected_or_contradiction = expected_refs
        .iter()
        .chain(contradiction_refs.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let precise_selected = selected_refs
        .iter()
        .filter(|selected| expected_or_contradiction.contains(*selected))
        .count();
    let selected_ref_precision_bp = ratio_bp_or(precise_selected, selected_refs.len(), 0);

    let prohibited_ref_rejection_bp = if prohibited_refs
        .iter()
        .any(|prohibited| selected.contains(prohibited.as_str()))
    {
        0
    } else {
        10_000
    };

    let exposed_contradictions = contradiction_refs
        .iter()
        .filter(|contradiction| selected.contains(contradiction.as_str()))
        .count();
    let contradiction_exposure_bp =
        ratio_bp_or(exposed_contradictions, contradiction_refs.len(), 10_000);

    let fresh_selected = selected_refs
        .iter()
        .filter(|selected| {
            fixture
                .corpus
                .iter()
                .find(|item| item_memory_id(item) == selected.as_str())
                .is_some_and(fresh_valid)
        })
        .count();
    let freshness_bp = ratio_bp_or(fresh_selected, selected_refs.len(), 10_000);

    let correct_citations = expected_refs
        .iter()
        .filter(|expected| citations.contains(expected.as_str()))
        .count();
    let citation_accuracy_bp = ratio_bp_or(correct_citations, citation_refs.len(), 10_000);
    let unsupported_claims = expected_refs.len().saturating_sub(expected_selected);
    let unsupported_claim_rate_bp = ratio_bp_or(unsupported_claims, expected_refs.len(), 0);

    let quality_score_bp = optimized_quality_score_bp(
        required_citation_recall_bp,
        selected_ref_precision_bp,
        prohibited_ref_rejection_bp,
        contradiction_exposure_bp,
        validation_pass_bp,
        freshness_bp,
    );
    let claim_allowed = prohibited_ref_rejection_bp > 0
        && validation_pass_bp >= 9_000
        && quality_score_bp >= 9_300
        && citation_accuracy_bp >= 9_850
        && unsupported_claim_rate_bp <= 60;

    EvidenceQualityBreakdown {
        required_citation_recall_bp,
        selected_ref_precision_bp,
        prohibited_ref_rejection_bp,
        contradiction_exposure_bp,
        validation_pass_bp,
        freshness_bp,
        quality_score_bp,
        citation_accuracy_bp,
        unsupported_claim_rate_bp,
        claim_allowed,
    }
}

/// Compute the pinned optimized quality formula.
#[must_use]
pub fn optimized_quality_score_bp(
    required_citation_recall_bp: u16,
    selected_ref_precision_bp: u16,
    prohibited_ref_rejection_bp: u16,
    contradiction_exposure_bp: u16,
    validation_pass_bp: u16,
    freshness_bp: u16,
) -> u16 {
    if prohibited_ref_rejection_bp == 0 {
        return 0;
    }
    let weighted = u32::from(required_citation_recall_bp)
        .saturating_mul(35)
        .saturating_add(u32::from(selected_ref_precision_bp).saturating_mul(20))
        .saturating_add(u32::from(prohibited_ref_rejection_bp).saturating_mul(20))
        .saturating_add(u32::from(contradiction_exposure_bp).saturating_mul(10))
        .saturating_add(u32::from(validation_pass_bp).saturating_mul(10))
        .saturating_add(u32::from(freshness_bp).saturating_mul(5))
        / 100;
    u16::try_from(weighted).unwrap_or(u16::MAX)
}

/// Run a deterministic synthetic benchmark runner.
pub fn run_benchmark_fixture(
    fixture: &BenchmarkFixture,
    runner_name: BenchmarkRunnerName,
) -> Result<BenchmarkRunReport, BenchmarkError> {
    validate_benchmark_fixture(fixture)?;
    let selected_memory_ids_by_task = select_memory_by_task(fixture, runner_name)?;
    let prompt_tokens =
        prompt_tokens_for_runner(fixture, &selected_memory_ids_by_task, runner_name);
    let completion_tokens = completion_tokens_for_fixture(fixture);
    let overhead_tokens = overhead_tokens_for_runner(fixture, runner_name, prompt_tokens);
    let overhead = overhead_for_runner(fixture, runner_name, overhead_tokens);
    let baseline_cost = long_context_baseline_micro_exo(fixture);
    let runner_cost = micro_exo_for_tokens(prompt_tokens, completion_tokens);
    let gross_savings_micro_exo = baseline_cost.saturating_sub(runner_cost);
    let overhead_micro_exo = overhead.total_micro_exo();
    let (quality_score_bp, citation_accuracy_bp, unsupported_claim_rate_bp, formula_claim_allowed) =
        if runner_name == BenchmarkRunnerName::GovernedDagDbOptimized {
            let quality = optimized_fixture_quality(fixture, &selected_memory_ids_by_task);
            (
                quality.quality_score_bp,
                quality.citation_accuracy_bp,
                quality.unsupported_claim_rate_bp,
                quality.claim_allowed,
            )
        } else {
            let (quality_score_bp, citation_accuracy_bp, unsupported_claim_rate_bp) =
                runner_quality(runner_name);
            (
                quality_score_bp,
                citation_accuracy_bp,
                unsupported_claim_rate_bp,
                true,
            )
        };
    let gate = evaluate_benchmark_gates(BenchmarkGateInput {
        quality_score_bp,
        citation_accuracy_bp,
        unsupported_claim_rate_bp,
        gross_savings_micro_exo,
        overhead_micro_exo,
    })?;
    let config_hash = config_hash(fixture, runner_name)?;
    let tokenizer_config_hash = Hash256::digest(TOKENIZER_CONFIG.as_bytes()).to_string();
    let mut report = BenchmarkRunReport {
        fixture_id: fixture.fixture_id.clone(),
        runner_name,
        deterministic_seed: fixture.deterministic_seed,
        config_hash,
        tokenizer_config_hash,
        temperature_bp: TEMPERATURE_BP,
        top_p_bp: TOP_P_BP,
        max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        selected_memory_ids_by_task,
        quality_score_bp,
        citation_accuracy_bp,
        unsupported_claim_rate_bp,
        prompt_tokens,
        completion_tokens,
        overhead_tokens,
        gross_savings_micro_exo,
        overhead,
        overhead_micro_exo,
        net_savings_micro_exo: gate.net_savings_micro_exo,
        savings_claim_allowed: gate.savings_claim_allowed && formula_claim_allowed,
        output_hash: String::new(),
    };
    report.output_hash = output_hash(&report)?;
    validate_benchmark_report(fixture, &report)?;
    Ok(report)
}

/// Validate that a report does not claim savings through prohibited or lower-quality output.
pub fn validate_benchmark_report(
    fixture: &BenchmarkFixture,
    report: &BenchmarkRunReport,
) -> Result<(), BenchmarkError> {
    if fixture.fixture_id != report.fixture_id
        || fixture.deterministic_seed != report.deterministic_seed
    {
        return invalid_runner_output("report fixture identity does not match input fixture");
    }
    let expected_hash = output_hash(&BenchmarkRunReport {
        output_hash: String::new(),
        ..report.clone()
    })?;
    if report.output_hash != expected_hash {
        return invalid_runner_output("output_hash does not match report material");
    }
    let items_by_memory_id = items_by_memory_id(fixture);
    let tasks_by_id = fixture
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect::<BTreeMap<_, _>>();

    for (task_id, selected_ids) in &report.selected_memory_ids_by_task {
        let Some(task) = tasks_by_id.get(task_id.as_str()) else {
            return invalid_runner_output(format!("unknown task_id {task_id}"));
        };
        for selected_id in selected_ids {
            if task
                .prohibited_memory_ids
                .iter()
                .any(|id| id == selected_id)
            {
                return invalid_runner_output(format!(
                    "prohibited memory {selected_id} selected for task {task_id}"
                ));
            }
            if matches!(
                report.runner_name,
                BenchmarkRunnerName::DagDbRouting
                    | BenchmarkRunnerName::GovernedDagDbRouting
                    | BenchmarkRunnerName::GovernedDagDbOptimized
            ) {
                let Some(item) = items_by_memory_id.get(selected_id.as_str()) else {
                    return invalid_runner_output(format!("unknown selected memory {selected_id}"));
                };
                let optimized_contradiction_evidence = report.runner_name
                    == BenchmarkRunnerName::GovernedDagDbOptimized
                    && task.contradiction_ref_ids.iter().any(|id| {
                        id == selected_id
                            || corpus_ref_to_memory_id(fixture, id) == selected_id.as_str()
                    });
                if item.revoked && !optimized_contradiction_evidence {
                    return invalid_runner_output(format!("revoked memory selected {selected_id}"));
                }
                if item.stale && !optimized_contradiction_evidence {
                    return invalid_runner_output(format!("stale memory selected {selected_id}"));
                }
                if !item.contradicts.is_empty() && !optimized_contradiction_evidence {
                    return invalid_runner_output(format!(
                        "contradicted memory selected {selected_id}"
                    ));
                }
                if item.expected_validation_status != ValidationStatus::Passed {
                    return invalid_runner_output(format!(
                        "unvalidated memory selected {selected_id}"
                    ));
                }
                if report.runner_name == BenchmarkRunnerName::GovernedDagDbRouting
                    && matches!(
                        item.risk_class,
                        RiskClass::R3 | RiskClass::R4 | RiskClass::R5
                    )
                {
                    return invalid_runner_output(format!(
                        "approval-required memory selected {selected_id}"
                    ));
                }
            }
        }
    }
    if report.savings_claim_allowed
        && (report.quality_score_bp < 8_500
            || report.citation_accuracy_bp < 9_500
            || report.unsupported_claim_rate_bp > 500
            || report.net_savings_micro_exo == 0)
    {
        return invalid_runner_output("savings claim did not satisfy quality gates");
    }
    Ok(())
}

fn select_memory_by_task(
    fixture: &BenchmarkFixture,
    runner_name: BenchmarkRunnerName,
) -> Result<BTreeMap<String, Vec<String>>, BenchmarkError> {
    let mut selected = BTreeMap::new();
    for task in &fixture.tasks {
        let selected_for_task = match runner_name {
            BenchmarkRunnerName::NoMemory => Vec::new(),
            BenchmarkRunnerName::LongContextDump => allowed_in_corpus_order(fixture, task, false),
            BenchmarkRunnerName::FlatRag => flat_rag_order(fixture, task),
            BenchmarkRunnerName::DagDbRouting => allowed_in_corpus_order(fixture, task, true),
            BenchmarkRunnerName::GovernedDagDbRouting => governed_order(fixture, task),
            BenchmarkRunnerName::GovernedDagDbOptimized => optimized_order(fixture, task),
        };
        selected.insert(task.task_id.clone(), selected_for_task);
    }
    Ok(selected)
}

fn allowed_in_corpus_order(
    fixture: &BenchmarkFixture,
    task: &BenchmarkTask,
    require_route_safety: bool,
) -> Vec<String> {
    let allowed = task
        .allowed_memory_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    fixture
        .corpus
        .iter()
        .filter(|item| {
            let memory_id = item_memory_id(item);
            allowed.contains(memory_id)
                && !task.prohibited_memory_ids.iter().any(|id| id == memory_id)
                && (!require_route_safety || route_safe(item))
        })
        .map(|item| item_memory_id(item).to_owned())
        .collect()
}

fn flat_rag_order(fixture: &BenchmarkFixture, task: &BenchmarkTask) -> Vec<String> {
    let expected = task
        .expected_citations
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let allowed = task
        .allowed_memory_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut selected = fixture
        .corpus
        .iter()
        .filter(|item| {
            let memory_id = item_memory_id(item);
            expected.contains(item.corpus_item_id.as_str())
                && !task.prohibited_memory_ids.iter().any(|id| id == memory_id)
        })
        .map(|item| item_memory_id(item).to_owned())
        .collect::<Vec<_>>();
    for item in &fixture.corpus {
        let memory_id = item_memory_id(item);
        if allowed.contains(memory_id)
            && !selected.iter().any(|id| id == memory_id)
            && !task.prohibited_memory_ids.iter().any(|id| id == memory_id)
        {
            selected.push(memory_id.to_owned());
        }
    }
    selected
}

fn governed_order(fixture: &BenchmarkFixture, task: &BenchmarkTask) -> Vec<String> {
    allowed_in_corpus_order(fixture, task, true)
        .into_iter()
        .filter(|memory_id| {
            fixture
                .corpus
                .iter()
                .find(|item| item_memory_id(item) == memory_id)
                .is_some_and(|item| {
                    !matches!(
                        item.risk_class,
                        RiskClass::R3 | RiskClass::R4 | RiskClass::R5
                    )
                })
        })
        .collect()
}

fn optimized_order(fixture: &BenchmarkFixture, task: &BenchmarkTask) -> Vec<String> {
    let policy = RouteBudgetPolicy::optimized_mvp();
    let mut selected = Vec::new();
    let max_refs = usize::try_from(policy.max_refs_per_task).unwrap_or(usize::MAX);

    for citation in evidence_refs_to_memory_ids(fixture, &task.expected_citation_ids)
        .into_iter()
        .take(usize::try_from(policy.max_required_citations_per_task).unwrap_or(usize::MAX))
    {
        if optimized_ref_allowed(fixture, task, &citation, false) {
            push_unique_ref(&mut selected, citation, max_refs);
        }
    }

    for contradiction in evidence_refs_to_memory_ids(fixture, &task.contradiction_ref_ids)
        .into_iter()
        .take(usize::try_from(policy.max_contradictions_per_task).unwrap_or(usize::MAX))
    {
        if optimized_ref_allowed(fixture, task, &contradiction, true) {
            push_unique_ref(&mut selected, contradiction, max_refs);
        }
    }

    let related_candidates = governed_order(fixture, task)
        .into_iter()
        .filter(|related| !selected.iter().any(|existing| existing == related))
        .take(usize::try_from(policy.max_related_refs_per_task).unwrap_or(usize::MAX))
        .collect::<Vec<_>>();
    for related in related_candidates {
        push_unique_ref(&mut selected, related, max_refs);
    }

    selected
}

fn push_unique_ref(selected: &mut Vec<String>, value: String, max_refs: usize) {
    if selected.iter().any(|existing| existing == &value) || selected.len() >= max_refs {
        return;
    }
    selected.push(value);
}

fn optimized_ref_allowed(
    fixture: &BenchmarkFixture,
    task: &BenchmarkTask,
    memory_id: &str,
    contradiction_evidence: bool,
) -> bool {
    if task.prohibited_memory_ids.iter().any(|id| id == memory_id)
        || task
            .prohibited_ref_ids
            .iter()
            .any(|id| id == memory_id || corpus_ref_to_memory_id(fixture, id) == memory_id)
    {
        return false;
    }
    let Some(item) = fixture
        .corpus
        .iter()
        .find(|item| item_memory_id(item) == memory_id)
    else {
        return false;
    };
    item.expected_validation_status == ValidationStatus::Passed
        && (route_safe(item) || contradiction_evidence)
}

fn route_safe(item: &BenchmarkCorpusItem) -> bool {
    !item.revoked
        && !item.stale
        && item.contradicts.is_empty()
        && item.expected_validation_status == ValidationStatus::Passed
}

fn prompt_tokens_for_runner(
    fixture: &BenchmarkFixture,
    selected: &BTreeMap<String, Vec<String>>,
    runner_name: BenchmarkRunnerName,
) -> u32 {
    match runner_name {
        BenchmarkRunnerName::NoMemory => 0,
        BenchmarkRunnerName::LongContextDump => token_count(
            &fixture
                .corpus
                .iter()
                .map(|item| item.summary_text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        BenchmarkRunnerName::FlatRag
        | BenchmarkRunnerName::DagDbRouting
        | BenchmarkRunnerName::GovernedDagDbRouting => selected
            .values()
            .map(|ids| {
                u32::try_from(ids.len())
                    .unwrap_or(u32::MAX)
                    .saturating_mul(8)
            })
            .sum(),
        BenchmarkRunnerName::GovernedDagDbOptimized => selected
            .values()
            .map(|ids| {
                u32::try_from(ids.len())
                    .unwrap_or(u32::MAX)
                    .saturating_mul(OPTIMIZED_CONTEXT_TOKENS_PER_REF)
            })
            .sum(),
    }
}

fn completion_tokens_for_fixture(fixture: &BenchmarkFixture) -> u32 {
    u32::try_from(fixture.tasks.len())
        .unwrap_or(u32::MAX)
        .saturating_mul(64)
}

fn overhead_tokens_for_runner(
    fixture: &BenchmarkFixture,
    runner_name: BenchmarkRunnerName,
    prompt_tokens: u32,
) -> u32 {
    let task_count = u32::try_from(fixture.tasks.len()).unwrap_or(u32::MAX);
    match runner_name {
        BenchmarkRunnerName::NoMemory => 0,
        BenchmarkRunnerName::LongContextDump => prompt_tokens,
        BenchmarkRunnerName::FlatRag => task_count.saturating_mul(16),
        BenchmarkRunnerName::DagDbRouting => task_count.saturating_mul(24),
        BenchmarkRunnerName::GovernedDagDbRouting => task_count.saturating_mul(32),
        BenchmarkRunnerName::GovernedDagDbOptimized => {
            task_count.saturating_mul(OPTIMIZED_OVERHEAD_TOKENS_PER_TASK)
        }
    }
}

fn overhead_for_runner(
    fixture: &BenchmarkFixture,
    runner_name: BenchmarkRunnerName,
    overhead_tokens: u32,
) -> BenchmarkOverheadAccounting {
    let task_count = u64::try_from(fixture.tasks.len()).unwrap_or(u64::MAX);
    match runner_name {
        BenchmarkRunnerName::NoMemory => BenchmarkOverheadAccounting {
            route_scoring_micro_exo: 0,
            validation_micro_exo: 0,
            redaction_micro_exo: 0,
            idempotency_lookup_micro_exo: 0,
            postgres_query_micro_exo: 0,
            dag_outbox_enqueue_micro_exo: 0,
            context_packet_micro_exo: 0,
            prompt_context_micro_exo: 0,
        },
        BenchmarkRunnerName::LongContextDump => BenchmarkOverheadAccounting {
            route_scoring_micro_exo: 0,
            validation_micro_exo: 0,
            redaction_micro_exo: 0,
            idempotency_lookup_micro_exo: 0,
            postgres_query_micro_exo: 0,
            dag_outbox_enqueue_micro_exo: 0,
            context_packet_micro_exo: 0,
            prompt_context_micro_exo: u64::from(overhead_tokens).saturating_mul(2),
        },
        BenchmarkRunnerName::FlatRag => BenchmarkOverheadAccounting {
            route_scoring_micro_exo: task_count.saturating_mul(2),
            validation_micro_exo: 0,
            redaction_micro_exo: 0,
            idempotency_lookup_micro_exo: task_count,
            postgres_query_micro_exo: task_count.saturating_mul(4),
            dag_outbox_enqueue_micro_exo: 0,
            context_packet_micro_exo: 0,
            prompt_context_micro_exo: u64::from(overhead_tokens),
        },
        BenchmarkRunnerName::DagDbRouting => BenchmarkOverheadAccounting {
            route_scoring_micro_exo: task_count.saturating_mul(4),
            validation_micro_exo: task_count.saturating_mul(3),
            redaction_micro_exo: task_count,
            idempotency_lookup_micro_exo: task_count,
            postgres_query_micro_exo: task_count.saturating_mul(6),
            dag_outbox_enqueue_micro_exo: task_count,
            context_packet_micro_exo: task_count.saturating_mul(2),
            prompt_context_micro_exo: u64::from(overhead_tokens),
        },
        BenchmarkRunnerName::GovernedDagDbRouting => BenchmarkOverheadAccounting {
            route_scoring_micro_exo: task_count.saturating_mul(5),
            validation_micro_exo: task_count.saturating_mul(5),
            redaction_micro_exo: task_count.saturating_mul(2),
            idempotency_lookup_micro_exo: task_count,
            postgres_query_micro_exo: task_count.saturating_mul(8),
            dag_outbox_enqueue_micro_exo: task_count.saturating_mul(2),
            context_packet_micro_exo: task_count.saturating_mul(3),
            prompt_context_micro_exo: u64::from(overhead_tokens),
        },
        BenchmarkRunnerName::GovernedDagDbOptimized => BenchmarkOverheadAccounting {
            route_scoring_micro_exo: task_count.saturating_mul(2),
            validation_micro_exo: task_count.saturating_mul(5),
            redaction_micro_exo: task_count,
            idempotency_lookup_micro_exo: 0,
            postgres_query_micro_exo: task_count.saturating_mul(4),
            dag_outbox_enqueue_micro_exo: task_count,
            context_packet_micro_exo: task_count,
            prompt_context_micro_exo: u64::from(overhead_tokens),
        },
    }
}

fn runner_quality(runner_name: BenchmarkRunnerName) -> (u16, u16, u16) {
    match runner_name {
        BenchmarkRunnerName::NoMemory => (4_000, 0, 3_000),
        BenchmarkRunnerName::LongContextDump => (8_600, 9_500, 400),
        BenchmarkRunnerName::FlatRag => (8_200, 9_000, 900),
        BenchmarkRunnerName::DagDbRouting => (8_500, 9_500, 500),
        BenchmarkRunnerName::GovernedDagDbRouting => (9_000, 9_800, 100),
        BenchmarkRunnerName::GovernedDagDbOptimized => (9_300, 9_850, 60),
    }
}

fn long_context_baseline_micro_exo(fixture: &BenchmarkFixture) -> u64 {
    micro_exo_for_tokens(
        prompt_tokens_for_runner(
            fixture,
            &BTreeMap::new(),
            BenchmarkRunnerName::LongContextDump,
        ),
        completion_tokens_for_fixture(fixture),
    )
}

fn micro_exo_for_tokens(prompt_tokens: u32, completion_tokens: u32) -> u64 {
    u64::from(prompt_tokens)
        .saturating_mul(2)
        .saturating_add(u64::from(completion_tokens).saturating_mul(4))
}

fn token_count(text: &str) -> u32 {
    let bytes = u32::try_from(text.len()).unwrap_or(u32::MAX);
    bytes.saturating_add(3) / 4
}

fn config_hash(
    fixture: &BenchmarkFixture,
    runner_name: BenchmarkRunnerName,
) -> Result<String, BenchmarkError> {
    #[derive(Serialize)]
    struct ConfigMaterial<'a> {
        domain_tag: &'static str,
        schema_version: u16,
        fixture_id: &'a str,
        deterministic_seed: u64,
        runner_name: BenchmarkRunnerName,
        tokenizer_config: &'static str,
        temperature_bp: u16,
        top_p_bp: u16,
        max_output_tokens: u32,
    }
    hash_structured(&ConfigMaterial {
        domain_tag: "exo.dagdb.benchmark.config",
        schema_version: 1,
        fixture_id: &fixture.fixture_id,
        deterministic_seed: fixture.deterministic_seed,
        runner_name,
        tokenizer_config: TOKENIZER_CONFIG,
        temperature_bp: TEMPERATURE_BP,
        top_p_bp: TOP_P_BP,
        max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
    })
    .map(|hash| hash.to_string())
    .map_err(|error| BenchmarkError::InvalidRunnerOutput {
        reason: format!("config hash failed: {error}"),
    })
}

fn output_hash(report: &BenchmarkRunReport) -> Result<String, BenchmarkError> {
    #[derive(Serialize)]
    struct ReportMaterial<'a> {
        domain_tag: &'static str,
        schema_version: u16,
        report: &'a BenchmarkRunReport,
    }
    hash_structured(&ReportMaterial {
        domain_tag: "exo.dagdb.benchmark.output",
        schema_version: 1,
        report,
    })
    .map(|hash| hash.to_string())
    .map_err(|error| BenchmarkError::InvalidRunnerOutput {
        reason: format!("output hash failed: {error}"),
    })
}

fn items_by_memory_id(fixture: &BenchmarkFixture) -> BTreeMap<&str, &BenchmarkCorpusItem> {
    fixture
        .corpus
        .iter()
        .map(|item| (item_memory_id(item), item))
        .collect()
}

fn optimized_fixture_quality(
    fixture: &BenchmarkFixture,
    selected_by_task: &BTreeMap<String, Vec<String>>,
) -> EvidenceQualityBreakdown {
    let task_count = fixture.tasks.len().max(1);
    let mut required_citation_recall_sum = 0u32;
    let mut selected_ref_precision_sum = 0u32;
    let mut prohibited_ref_rejection_sum = 0u32;
    let mut contradiction_exposure_sum = 0u32;
    let mut validation_pass_sum = 0u32;
    let mut freshness_sum = 0u32;
    let mut quality_sum = 0u32;
    let mut citation_accuracy_sum = 0u32;
    let mut unsupported_claim_sum = 0u32;
    let mut claim_allowed = true;

    for task in &fixture.tasks {
        let selected = selected_by_task
            .get(&task.task_id)
            .cloned()
            .unwrap_or_default();
        let citations = selected_citation_refs(fixture, task, &selected);
        let row = evidence_quality_for_task(
            fixture,
            task,
            &selected,
            &citations,
            validation_pass_bp_for_task(task),
        );
        required_citation_recall_sum += u32::from(row.required_citation_recall_bp);
        selected_ref_precision_sum += u32::from(row.selected_ref_precision_bp);
        prohibited_ref_rejection_sum += u32::from(row.prohibited_ref_rejection_bp);
        contradiction_exposure_sum += u32::from(row.contradiction_exposure_bp);
        validation_pass_sum += u32::from(row.validation_pass_bp);
        freshness_sum += u32::from(row.freshness_bp);
        quality_sum += u32::from(row.quality_score_bp);
        citation_accuracy_sum += u32::from(row.citation_accuracy_bp);
        unsupported_claim_sum += u32::from(row.unsupported_claim_rate_bp);
        claim_allowed &= row.claim_allowed;
    }

    let divisor = u32::try_from(task_count).unwrap_or(u32::MAX).max(1);
    EvidenceQualityBreakdown {
        required_citation_recall_bp: avg_bp(required_citation_recall_sum, divisor),
        selected_ref_precision_bp: avg_bp(selected_ref_precision_sum, divisor),
        prohibited_ref_rejection_bp: avg_bp(prohibited_ref_rejection_sum, divisor),
        contradiction_exposure_bp: avg_bp(contradiction_exposure_sum, divisor),
        validation_pass_bp: avg_bp(validation_pass_sum, divisor),
        freshness_bp: avg_bp(freshness_sum, divisor),
        quality_score_bp: avg_bp(quality_sum, divisor),
        citation_accuracy_bp: avg_bp(citation_accuracy_sum, divisor),
        unsupported_claim_rate_bp: avg_bp(unsupported_claim_sum, divisor),
        claim_allowed,
    }
}

fn selected_citation_refs(
    fixture: &BenchmarkFixture,
    task: &BenchmarkTask,
    selected: &[String],
) -> Vec<String> {
    let expected = evidence_refs_to_memory_ids(fixture, &task.expected_citation_ids)
        .into_iter()
        .collect::<BTreeSet<_>>();
    selected
        .iter()
        .filter(|selected| expected.contains(*selected))
        .cloned()
        .collect()
}

fn validation_pass_bp_for_task(task: &BenchmarkTask) -> u16 {
    match task.expected_validation_outcome {
        ValidationStatus::Passed | ValidationStatus::NotRequired => 10_000,
        ValidationStatus::NeedsCouncil | ValidationStatus::Pending => 8_000,
        ValidationStatus::Expired => 5_000,
        ValidationStatus::Failed | ValidationStatus::Contradictory => 0,
    }
}

fn avg_bp(sum: u32, divisor: u32) -> u16 {
    u16::try_from(sum / divisor).unwrap_or(u16::MAX)
}

fn ratio_bp_or(numerator: usize, denominator: usize, default: u16) -> u16 {
    if denominator == 0 {
        return default;
    }
    let scaled = u32::try_from(numerator)
        .unwrap_or(u32::MAX)
        .saturating_mul(10_000)
        / u32::try_from(denominator).unwrap_or(u32::MAX).max(1);
    u16::try_from(scaled).unwrap_or(u16::MAX)
}

fn evidence_refs_to_memory_ids(fixture: &BenchmarkFixture, refs: &[String]) -> Vec<String> {
    refs.iter()
        .map(|reference| corpus_ref_to_memory_id(fixture, reference))
        .collect()
}

fn sorted_unique_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn corpus_ref_to_memory_id(fixture: &BenchmarkFixture, reference: &str) -> String {
    fixture
        .corpus
        .iter()
        .find(|item| item.corpus_item_id == reference)
        .map_or_else(
            || reference.to_owned(),
            |item| item_memory_id(item).to_owned(),
        )
}

fn fresh_valid(item: &BenchmarkCorpusItem) -> bool {
    !item.revoked
        && !item.stale
        && item.contradicts.is_empty()
        && item.duplicates.is_empty()
        && item.expected_validation_status == ValidationStatus::Passed
}

fn scale_corpus_item(index: u32, config: &ScaleFixtureConfig) -> BenchmarkCorpusItem {
    let corpus_item_id = format!("scale-c{index:04}");
    let stale_start = SCALE_FRESH_VALID_COUNT;
    let revoked_start = stale_start + SCALE_STALE_COUNT;
    let contradicted_start = revoked_start + SCALE_REVOKED_COUNT;
    let duplicate_start = contradicted_start + SCALE_CONTRADICTED_COUNT;
    let stale = (stale_start..revoked_start).contains(&index);
    let revoked = (revoked_start..contradicted_start).contains(&index);
    let contradicted = (contradicted_start..duplicate_start).contains(&index);
    let duplicate = index >= duplicate_start;
    let risk_class = match index % 4 {
        0 => RiskClass::R1,
        1 => RiskClass::R2,
        2 => RiskClass::R3,
        _ => RiskClass::R4,
    };
    let source_type = match index % 6 {
        0 => SourceType::PublicWeb,
        1 => SourceType::PrivateCustomer,
        2 => SourceType::IpSensitive,
        3 => SourceType::Generated,
        4 => SourceType::OpenSource,
        _ => SourceType::UnknownProvenance,
    };
    BenchmarkCorpusItem {
        corpus_item_id,
        tenant_id: format!("tenant-{}", index % config.tenant_count),
        namespace: format!(
            "namespace-{}",
            (index / config.tenant_count) % config.namespace_count
        ),
        source_type,
        payload_hash: scale_hex(200_000, index),
        source_hash: scale_hex(300_000, index),
        title_text: format!("Scale benchmark title {index:04}"),
        summary_text: format!(
            "Scale benchmark summary {index:04} for deterministic optimized DAG DB routing only."
        ),
        risk_class,
        expected_validation_status: if contradicted {
            ValidationStatus::Contradictory
        } else {
            ValidationStatus::Passed
        },
        labels: scale_labels(stale, revoked, contradicted, duplicate),
        memory_id: Some(scale_hex(100_000, index)),
        revoked,
        stale,
        contradicts: if contradicted {
            vec![scale_hex(100_000, index.saturating_sub(1))]
        } else {
            Vec::new()
        },
        duplicates: if duplicate {
            vec![scale_hex(
                100_000,
                index.saturating_sub(SCALE_DUPLICATE_COUNT),
            )]
        } else {
            Vec::new()
        },
    }
}

fn scale_task(
    index: u32,
    config: &ScaleFixtureConfig,
    corpus: &[BenchmarkCorpusItem],
) -> BenchmarkTask {
    let expected_indices = [
        (config.deterministic_seed + u64::from(index).saturating_mul(7)) % 720,
        (config.deterministic_seed + u64::from(index).saturating_mul(11)) % 720,
        (config.deterministic_seed + u64::from(index).saturating_mul(13)) % 720,
    ];
    let expected_citation_ids = expected_indices
        .iter()
        .map(|expected| {
            corpus[usize::try_from(*expected).unwrap_or(0)]
                .corpus_item_id
                .clone()
        })
        .collect::<Vec<_>>();
    let prohibited_index = 120 + (index.saturating_mul(3) % 240);
    let contradiction_index = 840 + (index % 120);
    let mut allowed_memory_ids = expected_indices
        .iter()
        .map(|expected| item_memory_id(&corpus[usize::try_from(*expected).unwrap_or(0)]).to_owned())
        .collect::<Vec<_>>();
    allowed_memory_ids.push(
        item_memory_id(&corpus[usize::try_from(contradiction_index).unwrap_or(0)]).to_owned(),
    );
    let prohibited_ref =
        item_memory_id(&corpus[usize::try_from(prohibited_index).unwrap_or(0)]).to_owned();
    BenchmarkTask {
        task_id: format!("scale-t{:03}", index + 1),
        question_text: format!("Scale benchmark question {:03}", index + 1),
        task_signature_hash: scale_hex(400_000, index),
        expected_citations: expected_citation_ids.clone(),
        allowed_memory_ids,
        prohibited_memory_ids: vec![prohibited_ref.clone()],
        expected_citation_ids,
        prohibited_ref_ids: vec![prohibited_ref],
        contradiction_ref_ids: vec![
            item_memory_id(&corpus[usize::try_from(contradiction_index).unwrap_or(0)]).to_owned(),
        ],
        risk_labels: vec![RiskClass::R3],
        expected_validation_outcome: ValidationStatus::Passed,
    }
}

fn scale_labels(stale: bool, revoked: bool, contradicted: bool, duplicate: bool) -> Vec<String> {
    let mut labels = Vec::new();
    if stale {
        labels.push("stale".into());
    } else if revoked {
        labels.push("revoked".into());
    } else if contradicted {
        labels.push("contradicted".into());
    } else if duplicate {
        labels.push("duplicate".into());
    } else {
        labels.push("fresh_valid".into());
    }
    labels
}

fn scale_hex(base: u64, index: u32) -> String {
    format!("{:064x}", base.saturating_add(u64::from(index)))
}

fn item_memory_id(item: &BenchmarkCorpusItem) -> &str {
    item.memory_id.as_deref().unwrap_or(&item.payload_hash)
}

fn validate_hash_hex(field: &'static str, value: &str) -> Result<(), BenchmarkError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        invalid_fixture(format!("{field} must be 64 lowercase hex characters"))
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), BenchmarkError> {
    if value.is_empty() {
        invalid_fixture(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn require_count(name: &'static str, actual: usize, minimum: usize) -> Result<(), BenchmarkError> {
    if actual < minimum {
        invalid_fixture(format!("{name} count {actual} below {minimum}"))
    } else {
        Ok(())
    }
}

fn invalid_fixture<T>(reason: impl Into<String>) -> Result<T, BenchmarkError> {
    Err(BenchmarkError::InvalidFixture {
        reason: reason.into(),
    })
}

fn invalid_runner_output<T>(reason: impl Into<String>) -> Result<T, BenchmarkError> {
    Err(BenchmarkError::InvalidRunnerOutput {
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::MemoryGraphStyle;

    use super::*;
    use crate::query::graph_route_planner_order;

    const FIXTURE_JSON: &str = include_str!("../fixtures/benchmarks/mvp_minimum.json");

    #[test]
    fn benchmark_fixture_validates_schema_and_composition() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        assert_eq!(fixture.corpus.len(), 120);
        assert!(
            validate_raw_fixture_text_path(
                "crates/exo-dag-db-lab/fixtures/benchmarks/mvp_minimum.json"
            )
            .is_ok()
        );
    }

    #[test]
    fn benchmark_fixture_rejects_missing_required_field() {
        let json = r#"{
            "fixture_id": "bad",
            "deterministic_seed": 1,
            "corpus": [{
                "corpus_item_id": "c001",
                "tenant_id": "tenant_benchmark",
                "namespace": "mvp",
                "source_type": "public_web",
                "payload_hash": "0000000000000000000000000000000000000000000000000000000000000001",
                "source_hash": "0000000000000000000000000000000000000000000000000000000000000002",
                "title_text": "Synthetic title",
                "risk_class": "R0",
                "expected_validation_status": "passed",
                "labels": [],
                "memory_id": "0000000000000000000000000000000000000000000000000000000000000003",
                "revoked": false,
                "stale": false,
                "contradicts": [],
                "duplicates": []
            }],
            "tasks": []
        }"#;
        let error = load_benchmark_fixture_json(json).expect_err("missing field fails");
        assert!(matches!(error, BenchmarkError::Json { .. }));
    }

    #[test]
    fn benchmark_fixture_rejects_low_composition_and_bad_path() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        fixture.corpus.truncate(119);
        assert!(matches!(
            validate_benchmark_fixture(&fixture),
            Err(BenchmarkError::InvalidFixture { .. })
        ));
        assert!(matches!(
            validate_raw_fixture_text_path("crates/exo-dag-db-core/fixtures/metadata/raw.json"),
            Err(BenchmarkError::InvalidFixture { .. })
        ));
    }

    #[test]
    fn benchmark_fixture_rejects_duplicate_empty_and_unknown_material() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");

        let mut no_tasks = fixture.clone();
        no_tasks.tasks.clear();
        assert!(matches!(
            validate_benchmark_fixture(&no_tasks),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut duplicate_corpus = fixture.clone();
        duplicate_corpus.corpus[1].corpus_item_id =
            duplicate_corpus.corpus[0].corpus_item_id.clone();
        assert!(matches!(
            validate_benchmark_fixture(&duplicate_corpus),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut duplicate_memory = fixture.clone();
        duplicate_memory.corpus[1].memory_id = duplicate_memory.corpus[0].memory_id.clone();
        assert!(matches!(
            validate_benchmark_fixture(&duplicate_memory),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut unknown_citation = fixture.clone();
        unknown_citation.tasks[0].expected_citations = vec!["missing".to_owned()];
        assert!(matches!(
            validate_benchmark_fixture(&unknown_citation),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut bad_hash = fixture.clone();
        bad_hash.tasks[0].task_signature_hash = "not_hex".to_owned();
        assert!(matches!(
            validate_benchmark_fixture(&bad_hash),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut bad_hex_char = fixture.clone();
        bad_hex_char.tasks[0].task_signature_hash =
            "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg".to_owned();
        assert!(matches!(
            validate_benchmark_fixture(&bad_hex_char),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut empty_fixture_id = fixture.clone();
        empty_fixture_id.fixture_id.clear();
        assert!(matches!(
            validate_benchmark_fixture(&empty_fixture_id),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut low_public_count = fixture.clone();
        for item in low_public_count.corpus.iter_mut().take(11) {
            item.source_type = SourceType::Generated;
        }
        assert!(matches!(
            validate_benchmark_fixture(&low_public_count),
            Err(BenchmarkError::InvalidFixture { .. })
        ));

        let mut missing_memory_id = fixture.clone();
        missing_memory_id.corpus[0].memory_id = None;
        assert_eq!(
            item_memory_id(&missing_memory_id.corpus[0]),
            missing_memory_id.corpus[0].payload_hash
        );
        assert!(validate_benchmark_fixture(&missing_memory_id).is_ok());
    }

    #[test]
    fn benchmark_runners_are_deterministic_and_cover_all_runner_names() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        for runner in BenchmarkRunnerName::all() {
            let first = run_benchmark_fixture(&fixture, runner).expect("first run");
            let second = run_benchmark_fixture(&fixture, runner).expect("second run");
            assert_eq!(first, second);
            assert_eq!(first.fixture_id, fixture.fixture_id);
            assert_eq!(first.deterministic_seed, fixture.deterministic_seed);
            assert_eq!(first.config_hash.len(), 64);
            assert_eq!(first.tokenizer_config_hash.len(), 64);
            assert_eq!(first.output_hash.len(), 64);
        }
    }

    #[test]
    fn benchmark_runners_reject_seed_drift_and_invalid_selection() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let mut report = run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
            .expect("governed run");
        report.deterministic_seed = report.deterministic_seed.saturating_add(1);
        assert!(matches!(
            validate_benchmark_report(&fixture, &report),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        let mut prohibited =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                .expect("governed run");
        let task = &fixture.tasks[0];
        prohibited
            .selected_memory_ids_by_task
            .insert(task.task_id.clone(), task.prohibited_memory_ids.clone());
        prohibited.output_hash = output_hash(&prohibited).expect("rehash prohibited");
        assert!(matches!(
            validate_benchmark_report(&fixture, &prohibited),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));
    }

    #[test]
    fn benchmark_runners_filter_prohibited_memory_before_selection() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let mut task = fixture.tasks[0].clone();
        let prohibited = item_memory_id(&fixture.corpus[0]).to_owned();
        task.allowed_memory_ids = vec![prohibited.clone()];
        task.prohibited_memory_ids = vec![prohibited.clone()];

        assert!(!allowed_in_corpus_order(&fixture, &task, false).contains(&prohibited));
        assert!(!allowed_in_corpus_order(&fixture, &task, true).contains(&prohibited));
        assert!(!flat_rag_order(&fixture, &task).contains(&prohibited));
    }

    #[test]
    fn benchmark_runners_use_graph_route_order_for_governed_dag_db() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let governed = run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
            .expect("governed run");
        assert_eq!(
            graph_route_planner_order(),
            [
                MemoryGraphStyle::SemanticCatalogGraph,
                MemoryGraphStyle::CanonicalMemoryGraph,
                MemoryGraphStyle::ProvenanceReceiptDag,
                MemoryGraphStyle::ContradictionSupersessionGraph,
                MemoryGraphStyle::RoutingViewGraph,
                MemoryGraphStyle::DependencyDag,
                MemoryGraphStyle::ContextPacketGraph,
            ]
        );
        assert!(governed.overhead.route_scoring_micro_exo > 0);
        assert!(governed.overhead.context_packet_micro_exo > 0);
    }

    #[test]
    fn benchmark_runners_reject_unknown_unvalidated_high_risk_and_bad_savings_reports() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");

        let mut wrong_fixture =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                .expect("governed run");
        wrong_fixture.fixture_id = "other_fixture".to_owned();
        assert!(matches!(
            validate_benchmark_report(&fixture, &wrong_fixture),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        let mut unknown_task =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                .expect("governed run");
        unknown_task
            .selected_memory_ids_by_task
            .insert("missing_task".to_owned(), Vec::new());
        rehash(&mut unknown_task);
        assert!(matches!(
            validate_benchmark_report(&fixture, &unknown_task),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        let mut unknown_memory =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                .expect("governed run");
        unknown_memory.selected_memory_ids_by_task.insert(
            fixture.tasks[0].task_id.clone(),
            vec!["ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned()],
        );
        rehash(&mut unknown_memory);
        assert!(matches!(
            validate_benchmark_report(&fixture, &unknown_memory),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        fixture.corpus[0].expected_validation_status = ValidationStatus::Failed;
        let mut unvalidated =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::NoMemory).expect("no memory run");
        unvalidated.selected_memory_ids_by_task.insert(
            fixture.tasks[0].task_id.clone(),
            vec![item_memory_id(&fixture.corpus[0]).to_owned()],
        );
        unvalidated.runner_name = BenchmarkRunnerName::GovernedDagDbRouting;
        rehash(&mut unvalidated);
        assert!(matches!(
            validate_benchmark_report(&fixture, &unvalidated),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let mut high_risk =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                .expect("governed run");
        high_risk.selected_memory_ids_by_task.insert(
            fixture.tasks[0].task_id.clone(),
            vec![item_memory_id(&fixture.corpus[20]).to_owned()],
        );
        rehash(&mut high_risk);
        assert!(matches!(
            validate_benchmark_report(&fixture, &high_risk),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        let mut tampered_hash =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                .expect("governed run");
        tampered_hash.output_hash = Hash256::ZERO.to_string();
        assert!(matches!(
            validate_benchmark_report(&fixture, &tampered_hash),
            Err(BenchmarkError::InvalidRunnerOutput { .. })
        ));

        let mut bad_savings =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::FlatRag).expect("flat rag run");
        bad_savings.savings_claim_allowed = true;
        rehash(&mut bad_savings);
        assert_invalid_runner_contains(
            validate_benchmark_report(&fixture, &bad_savings),
            "savings claim",
        );
    }

    #[test]
    fn benchmark_runners_reject_revoked_stale_and_contradicted_governed_memory() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        for (forbidden, reason) in [
            (
                "00000000000000000000000000000000000000000000000000000000000186dd",
                "stale",
            ),
            (
                "00000000000000000000000000000000000000000000000000000000000186f1",
                "revoked",
            ),
            (
                "00000000000000000000000000000000000000000000000000000000000186fb",
                "contradicted",
            ),
        ] {
            let mut report =
                run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
                    .expect("governed run");
            report
                .selected_memory_ids_by_task
                .insert(fixture.tasks[0].task_id.clone(), vec![forbidden.to_owned()]);
            rehash(&mut report);
            assert_invalid_runner_contains(validate_benchmark_report(&fixture, &report), reason);
        }
    }

    #[test]
    fn fixture_audit_mvp_evidence_fields_exist() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        audit_mvp_fixture_evidence_fields(&fixture).expect("evidence fields exist");
        assert!(
            fixture
                .tasks
                .iter()
                .all(|task| !task.expected_citation_ids.is_empty())
        );
    }

    #[test]
    fn mvp_fixture_evidence_population_matches_locked_mapping() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        for task in &mut fixture.tasks {
            task.expected_citation_ids.clear();
            task.prohibited_ref_ids.clear();
            task.contradiction_ref_ids.clear();
        }
        let notes = populate_mvp_evidence_fields(&mut fixture);
        assert!(notes.is_empty());
        for task in &fixture.tasks {
            assert_eq!(task.expected_citation_ids, task.expected_citations);
            assert_eq!(task.prohibited_ref_ids, task.prohibited_memory_ids);
            assert_eq!(
                task.contradiction_ref_ids,
                task.allowed_memory_ids
                    .iter()
                    .take(2)
                    .cloned()
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn mvp_fixture_evidence_audit_reports_each_missing_field() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        fixture.tasks[0].expected_citation_ids.clear();
        fixture.tasks[1].prohibited_ref_ids.clear();
        fixture.tasks[2].contradiction_ref_ids.clear();

        let error = audit_mvp_fixture_evidence_fields(&fixture).expect_err("audit fails");
        let BenchmarkError::InvalidFixture { reason } = error else {
            panic!("unexpected error variant");
        };
        assert!(reason.contains(".expected_citation_ids"));
        assert!(reason.contains(".prohibited_ref_ids"));
        assert!(reason.contains(".contradiction_ref_ids"));
    }

    #[test]
    fn mvp_fixture_population_notes_missing_source_fields() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        fixture.tasks[0].expected_citations.clear();
        fixture.tasks[1].allowed_memory_ids.clear();
        fixture.tasks[2].prohibited_memory_ids.clear();

        let notes = populate_mvp_evidence_fields(&mut fixture);
        assert_eq!(
            notes,
            vec![
                fixture.tasks[0].task_id.clone(),
                fixture.tasks[1].task_id.clone(),
                fixture.tasks[2].task_id.clone()
            ]
        );
        assert!(fixture.tasks[0].expected_citation_ids.is_empty());
        assert!(fixture.tasks[1].contradiction_ref_ids.is_empty());
        assert!(fixture.tasks[2].prohibited_ref_ids.is_empty());
    }

    #[test]
    fn optimized_runner_is_additive() {
        assert_eq!(
            BenchmarkRunnerName::all().last(),
            Some(&BenchmarkRunnerName::GovernedDagDbOptimized)
        );
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let governed = run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
            .expect("governed");
        let optimized =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbOptimized)
                .expect("optimized");
        assert_eq!(
            governed.runner_name,
            BenchmarkRunnerName::GovernedDagDbRouting
        );
        assert_eq!(
            optimized.runner_name,
            BenchmarkRunnerName::GovernedDagDbOptimized
        );
    }

    #[test]
    fn optimized_component_formulas_per_task_match_locked_definitions() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let expected = corpus_ref_to_memory_id(&fixture, &task.expected_citation_ids[0]);
        let contradiction = task.contradiction_ref_ids[1].clone();
        let selected = vec![expected.clone(), contradiction.clone()];
        let row = evidence_quality_for_task(
            &fixture,
            task,
            &selected,
            std::slice::from_ref(&expected),
            10_000,
        );
        assert_eq!(row.required_citation_recall_bp, 10_000);
        assert_eq!(row.selected_ref_precision_bp, 10_000);
        assert_eq!(row.prohibited_ref_rejection_bp, 10_000);
        assert_eq!(row.contradiction_exposure_bp, 10_000);
        assert_eq!(row.validation_pass_bp, 10_000);
        assert_eq!(row.freshness_bp, 10_000);
    }

    #[test]
    fn optimized_citation_accuracy_and_unsupported_claim_rate_match_locked_definitions() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let expected = corpus_ref_to_memory_id(&fixture, &task.expected_citation_ids[0]);
        let extra_citation = task.allowed_memory_ids[1].clone();
        let row = evidence_quality_for_task(
            &fixture,
            task,
            std::slice::from_ref(&expected),
            &[expected.clone(), extra_citation],
            10_000,
        );
        assert_eq!(row.required_citation_recall_bp, 10_000);
        assert_eq!(row.citation_accuracy_bp, 5_000);
        assert_eq!(row.unsupported_claim_rate_bp, 0);

        let unsupported = evidence_quality_for_task(
            &fixture,
            task,
            std::slice::from_ref(&task.allowed_memory_ids[1]),
            &[],
            10_000,
        );
        assert_eq!(unsupported.required_citation_recall_bp, 0);
        assert_eq!(unsupported.unsupported_claim_rate_bp, 10_000);
    }

    #[test]
    fn optimized_quality_formula_components_combine_correctly() {
        assert_eq!(
            optimized_quality_score_bp(10_000, 9_000, 10_000, 8_000, 10_000, 9_000),
            9_550
        );
    }

    #[test]
    fn optimized_prohibited_ref_rejection_kill_switch() {
        assert_eq!(
            optimized_quality_score_bp(10_000, 10_000, 0, 10_000, 10_000, 10_000),
            0
        );
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let row = evidence_quality_for_task(
            &fixture,
            task,
            std::slice::from_ref(&task.prohibited_ref_ids[0]),
            &[],
            10_000,
        );
        assert_eq!(row.prohibited_ref_rejection_bp, 0);
        assert_eq!(row.quality_score_bp, 0);
    }

    #[test]
    fn optimized_validation_below_9000_blocks_claim() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let selected = optimized_order(&fixture, task);
        let citations = selected_citation_refs(&fixture, task, &selected);
        let row = evidence_quality_for_task(&fixture, task, &selected, &citations, 8_000);
        assert!(!row.claim_allowed);
    }

    #[test]
    fn optimized_route_budget_preserves_required_citations() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let report = run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbOptimized)
            .expect("optimized");
        for task in &fixture.tasks {
            let selected = report
                .selected_memory_ids_by_task
                .get(&task.task_id)
                .expect("task selection");
            for citation in &task.expected_citation_ids {
                let memory_id = corpus_ref_to_memory_id(&fixture, citation);
                assert!(selected.contains(&memory_id));
            }
            assert!(
                selected.len()
                    <= usize::try_from(RouteBudgetPolicy::optimized_mvp().max_refs_per_task)
                        .unwrap_or(usize::MAX)
            );
        }
    }

    #[test]
    fn optimized_route_rejects_blocked_unknown_and_duplicate_refs() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let stale_memory_id = fixture
            .corpus
            .iter()
            .find(|item| item.stale && item.expected_validation_status == ValidationStatus::Passed)
            .map(item_memory_id)
            .expect("stale validated item")
            .to_owned();
        let revoked_memory_id = fixture
            .corpus
            .iter()
            .find(|item| {
                item.revoked && item.expected_validation_status == ValidationStatus::Passed
            })
            .map(item_memory_id)
            .expect("revoked validated item")
            .to_owned();
        let expected_memory_id =
            corpus_ref_to_memory_id(&fixture, &fixture.tasks[0].expected_citation_ids[0]);
        let prohibited_memory_id =
            corpus_ref_to_memory_id(&fixture, &fixture.tasks[0].prohibited_ref_ids[0]);
        let task = &mut fixture.tasks[0];
        task.expected_citation_ids = vec![
            expected_memory_id.clone(),
            expected_memory_id.clone(),
            task.prohibited_ref_ids[0].clone(),
            "unknown-memory-id".into(),
        ];
        task.contradiction_ref_ids = vec![stale_memory_id.clone(), revoked_memory_id];
        task.allowed_memory_ids.push(stale_memory_id.clone());

        let selected = optimized_order(&fixture, &fixture.tasks[0]);
        assert!(selected.contains(&expected_memory_id));
        assert!(selected.contains(&stale_memory_id));
        assert_eq!(
            selected
                .iter()
                .filter(|memory_id| *memory_id == &expected_memory_id)
                .count(),
            1
        );
        assert!(!selected.contains(&prohibited_memory_id));
        assert!(
            !selected
                .iter()
                .any(|memory_id| memory_id == "unknown-memory-id")
        );
    }

    #[test]
    fn optimized_route_rejects_unvalidated_non_contradiction_refs() {
        let mut fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let unvalidated_memory_id =
            corpus_ref_to_memory_id(&fixture, &fixture.tasks[0].expected_citation_ids[0]);
        for item in &mut fixture.corpus {
            if item_memory_id(item) == unvalidated_memory_id {
                item.expected_validation_status = ValidationStatus::Failed;
            }
        }
        fixture.tasks[0].expected_citation_ids = vec![unvalidated_memory_id.clone()];
        fixture.tasks[0].contradiction_ref_ids.clear();

        let selected = optimized_order(&fixture, &fixture.tasks[0]);
        assert!(!selected.contains(&unvalidated_memory_id));
    }

    #[test]
    fn scale_fixture_generation_is_deterministic() {
        let first = generate_scale_fixture();
        let second = generate_scale_fixture();
        assert_eq!(first, second);
        validate_benchmark_fixture(&first).expect("scale fixture validates");
    }

    #[test]
    fn scale_fixture_composition_matches_locked_distribution() {
        let fixture = generate_scale_fixture();
        assert_eq!(fixture.corpus.len(), 1_200);
        assert_eq!(fixture.tasks.len(), 150);
        assert_eq!(
            fixture
                .corpus
                .iter()
                .filter(|item| fresh_valid(item))
                .count(),
            720
        );
        assert_eq!(fixture.corpus.iter().filter(|item| item.stale).count(), 180);
        assert_eq!(
            fixture.corpus.iter().filter(|item| item.revoked).count(),
            120
        );
        assert_eq!(
            fixture
                .corpus
                .iter()
                .filter(|item| !item.contradicts.is_empty())
                .count(),
            120
        );
        assert_eq!(
            fixture
                .corpus
                .iter()
                .filter(|item| !item.duplicates.is_empty())
                .count(),
            60
        );
    }

    #[test]
    fn optimized_adversarial_unsafe_skip_validation() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let selected = optimized_order(&fixture, task);
        let citations = selected_citation_refs(&fixture, task, &selected);
        let row = evidence_quality_for_task(&fixture, task, &selected, &citations, 0);
        assert!(!row.claim_allowed);
    }

    #[test]
    fn optimized_adversarial_unsafe_select_prohibited() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let row = evidence_quality_for_task(
            &fixture,
            task,
            std::slice::from_ref(&task.prohibited_ref_ids[0]),
            &[],
            10_000,
        );
        assert_eq!(row.quality_score_bp, 0);
    }

    #[test]
    fn optimized_adversarial_unsafe_hide_contradictions() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let selected = optimized_order(&fixture, task);
        let citations = selected_citation_refs(&fixture, task, &selected);
        let exposed = evidence_quality_for_task(&fixture, task, &selected, &citations, 10_000);
        let expected_only = vec![corpus_ref_to_memory_id(
            &fixture,
            &task.expected_citation_ids[0],
        )];
        let hidden =
            evidence_quality_for_task(&fixture, task, &expected_only, &expected_only, 10_000);
        assert!(
            exposed
                .contradiction_exposure_bp
                .saturating_sub(hidden.contradiction_exposure_bp)
                >= 1_000
        );
    }

    #[test]
    fn optimized_adversarial_unsafe_drop_required_citation() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let task = &fixture.tasks[0];
        let dropped = vec![task.allowed_memory_ids[1].clone()];
        let row = evidence_quality_for_task(&fixture, task, &dropped, &[], 10_000);
        assert!(row.required_citation_recall_bp <= 6_700);
    }

    #[test]
    fn savings_gates_require_quality_and_net_positive() {
        let fixture = load_benchmark_fixture_json(FIXTURE_JSON).expect("fixture validates");
        let governed = run_benchmark_fixture(&fixture, BenchmarkRunnerName::GovernedDagDbRouting)
            .expect("governed run");
        assert!(governed.savings_claim_allowed);
        assert!(governed.overhead.route_scoring_micro_exo > 0);
        assert!(governed.overhead.validation_micro_exo > 0);
        assert!(governed.overhead.redaction_micro_exo > 0);
        assert!(governed.overhead.idempotency_lookup_micro_exo > 0);
        assert!(governed.overhead.postgres_query_micro_exo > 0);
        assert!(governed.overhead.dag_outbox_enqueue_micro_exo > 0);
        assert!(governed.overhead.context_packet_micro_exo > 0);

        let flat =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::FlatRag).expect("flat rag run");
        assert!(!flat.savings_claim_allowed);

        let no_memory =
            run_benchmark_fixture(&fixture, BenchmarkRunnerName::NoMemory).expect("no memory run");
        assert!(!no_memory.savings_claim_allowed);
    }

    fn rehash(report: &mut BenchmarkRunReport) {
        report.output_hash = String::new();
        report.output_hash = output_hash(report).expect("rehash report");
    }

    fn assert_invalid_runner_contains(result: Result<(), BenchmarkError>, expected_reason: &str) {
        match result {
            Err(BenchmarkError::InvalidRunnerOutput { reason }) => {
                assert!(
                    reason.contains(expected_reason),
                    "expected {reason:?} to contain {expected_reason:?}"
                );
            }
            other => panic!("expected invalid runner output, got {other:?}"),
        }
    }
}
