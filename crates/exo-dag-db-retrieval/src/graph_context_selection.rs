//! Pure Rust graph context selection over in-memory DAG DB graph state.
//!
//! This module selects safe metadata refs and graph edges for a task. It does
//! not render context packets, read Postgres, expose gateway routes, or claim
//! production/runtime approval.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::Hash256;
use exo_dag_db_api::{
    DagDbGraphContextSelectionRequest, DagDbGraphContextSelectionResponse,
    DagDbGraphContextSelectionStatus, DagDbGraphSelectionTraceStep, DagDbOmittedContextRef,
    DagDbSelectedContextRef, DagDbSelectedGraphEdgeRef, MemoryGraphStyle, SafeMetadata,
    ValidationStatus,
};

use crate::{
    graph::MemoryGraphEdge,
    query::graph_route_planner_order,
    scoring::{DomainError, DomainResult},
};

const REQUESTED_SCORE: u32 = 3_000_000;
const CATALOG_HINT_SCORE: u32 = 2_000_000;
const TASK_TERM_SCORE: u32 = 1_000_000;
const DOCUMENT_TYPE_BOOST: u32 = 500;
/// Q2-S2 coverage reward (basis points) added once per distinct identifier-like
/// token (snake_case, dotted path, hex id, CamelCase) extracted from the task
/// text that also appears verbatim in a candidate's searchable metadata. This
/// is an additive term layered on top of the existing route/term/document-type
/// signals: it never reorders the `REQUESTED`/`CATALOG_HINT`/`TASK_TERM` tiers
/// relative to each other, it only differentiates candidates within a tier by
/// rewarding obligation coverage. Exact-token matching only, no embeddings.
const COVERAGE_TERM_BP: u32 = 250;
/// Hard ceiling on the coverage contribution so a ref dense in identifiers can
/// never leapfrog a higher-tier signal (e.g. a catalog-hint or requested ref).
const MAX_COVERAGE_TERM_BP: u32 = 100_000;
/// PRD-D2 (dimension3-prd-02) S4 roll-up scoring share, in basis points of the
/// strongest member score. A layer root earns this fraction of its strongest
/// member's relevance so a relevant layer's root can win selection and anchor a
/// drilldown — without ever out-scoring its own strongest member.
///
/// SHARED CROSS-LANE RULE (authoritative source: the Python measured-benchmark
/// lane `tools/dagdb_agent_brain_context_utility.py::aggregate_member_relevance_bp`,
/// `ROLLUP_SCORING_SHARE_BP=5000`, `ROLLUP_SCORING_MAX_BONUS=4000`):
/// `bonus = max(0, best_member_score) * SHARE_BP / 10_000`, then capped at
/// `MAX_ROLLUP_SCORE_BP`. This Rust pure function mirrors that lane byte-for-byte
/// so the two roll-up scorers can never diverge. The Python lane is the measured
/// benchmark lane and stays authoritative; this constant tracks it.
pub const ROLLUP_SCORING_SHARE_BP: u32 = 5_000;
/// Hard ceiling on the roll-up bonus a layer root can earn, so an aggregate can
/// never leapfrog a higher-tier signal. Mirrors the authoritative Python lane's
/// `ROLLUP_SCORING_MAX_BONUS` (4_000); the two lanes must stay equal.
pub const MAX_ROLLUP_SCORE_BP: u32 = 4_000;
pub const MAX_SELECTED_GRAPH_EDGES_PER_PACKET: usize = 12;
// PRD-D4: the telemetry-ref quota (`MAX_TELEMETRY_REF_SHARE_BP`) and the
// read-side title-prefix heuristic (`TELEMETRY_TITLE_PREFIX` /
// `is_telemetry_candidate`) were retired. Telemetry is now excluded from
// packet selection by STRUCTURE at the loader (`kg_context_selection.rs` filters
// `node_type NOT IN ('usage_event','context_packet')`), so telemetry rows never
// reach this in-memory candidate pool and the prior `usage_event_ratio_too_high`
// regression cannot recur.

/// Q2-S2 per-family diversity cap. A "family" is the first two `catalog_path`
/// segments (or the single segment when only one exists). At most this many
/// task-routed refs may share one family in a single packet, so 64 slots carry
/// distinct material instead of one catalog branch crowding out the rest. The
/// check runs inside the selection loop: over-cap refs are omitted with reason
/// `family_diversity_cap_exceeded`,
/// and explicit `requested_memory_ids` selections are exempt (relink retrieval
/// must return exactly what was asked for). The cap is also RELEVANCE-AWARE: a
/// coverage-positive ref (the deterministic coverage term contributed a nonzero
/// bonus for this task) is exempt from the cap and does not increment the family
/// count, so a narrow-domain task that legitimately wants many refs from one
/// family is no longer forced to swap task-relevant material for irrelevant
/// diversity (2026-06-10 matrix: capped packets displaced topically relevant
/// families). `8` of the `64`-slot envelope is one
/// eighth: large enough to keep a deep branch usable, small enough to force
/// breadth across the eight `MAX/PER_FAMILY` distinct families the cap admits.
/// The cap is a SOFT preference: when the first pass leaves open slots (a
/// corpus dominated by one family), family-capped omissions are backfilled in
/// score order so the envelope never starves below `max_memory_refs`.
pub const MAX_FAMILY_REF_SHARE: usize = 8;

const FORBIDDEN_MATERIAL_FRAGMENTS: &[&str] = &[
    "/users/",
    "database_url",
    "private key",
    ".env",
    "raw_markdown",
    "raw_body",
    "source_path",
    "postgres://",
    "file://",
];

/// Q2-S1 deterministic task budget class.
///
/// Mirrors the six benchmark task families
/// (`tools/dagdb_agent_brain_semantic_benchmark.py::TASK_FAMILIES`:
/// `repo_navigation`, `code_change`, `debugging`, `planning`, `evidence_review`,
/// `handoff_continuation`) and the metadata-only worker-specialty router
/// (`tools/dagdb_worker_specialty_router.py`). The router classifies into nine
/// worker specialties; here we collapse to the six packet budget classes the
/// benchmark scores. No agent or LLM judgment is involved: classification is
/// pure keyword matching over normalized task tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskBudgetClass {
    /// Repo navigation / discovery — smallest packets.
    Navigation,
    /// Code change / implementation.
    CodeChange,
    /// Debugging a failure — largest packets (deep context).
    Debugging,
    /// Planning a sequence of work.
    Planning,
    /// Reviewing evidence / proofs.
    EvidenceReview,
    /// Handoff / continuation for future agents.
    Handoff,
}

/// Fallback/floor token budget. Matches the current hardcoded gateway budget so
/// adopting the budget-class function never lowers an existing packet below the
/// pre-Q2 envelope; classes only raise the budget from here.
pub const TASK_BUDGET_FLOOR_TOKENS: u32 = 2_048;
/// Hard cap shared by every class (matches the highest budget-sweep level
/// characterized in `target/dagdb/agent_brain/budget_sweep_experiment/`).
pub const TASK_BUDGET_CAP_TOKENS: u32 = 16_384;
/// Navigation packets stay at the floor: discovery needs breadth of pointers,
/// not deep payloads.
pub const NAVIGATION_BUDGET_TOKENS: u32 = 2_048;
/// Code-change packets get the first budget-sweep step above the floor.
pub const CODE_CHANGE_BUDGET_TOKENS: u32 = 4_096;
/// Debugging packets get the deepest budget: failure diagnosis needs the most
/// context (sweep characterized 8192/16384 for the harder families).
pub const DEBUGGING_BUDGET_TOKENS: u32 = 8_192;
/// Planning packets sit between code-change and debugging.
pub const PLANNING_BUDGET_TOKENS: u32 = 4_096;
/// Evidence-review packets need room for multiple proof refs.
pub const EVIDENCE_REVIEW_BUDGET_TOKENS: u32 = 8_192;
/// Handoff packets carry a continuation digest: floor is sufficient.
pub const HANDOFF_BUDGET_TOKENS: u32 = 2_048;

impl TaskBudgetClass {
    /// Per-class token budget, bounded to `[FLOOR, CAP]`.
    #[must_use]
    pub const fn token_budget(self) -> u32 {
        let raw = match self {
            TaskBudgetClass::Navigation => NAVIGATION_BUDGET_TOKENS,
            TaskBudgetClass::CodeChange => CODE_CHANGE_BUDGET_TOKENS,
            TaskBudgetClass::Debugging => DEBUGGING_BUDGET_TOKENS,
            TaskBudgetClass::Planning => PLANNING_BUDGET_TOKENS,
            TaskBudgetClass::EvidenceReview => EVIDENCE_REVIEW_BUDGET_TOKENS,
            TaskBudgetClass::Handoff => HANDOFF_BUDGET_TOKENS,
        };
        // Clamp without floats: floor then cap.
        let floored = if raw < TASK_BUDGET_FLOOR_TOKENS {
            TASK_BUDGET_FLOOR_TOKENS
        } else {
            raw
        };
        if floored > TASK_BUDGET_CAP_TOKENS {
            TASK_BUDGET_CAP_TOKENS
        } else {
            floored
        }
    }

    /// Stable label for traces and tests.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            TaskBudgetClass::Navigation => "navigation",
            TaskBudgetClass::CodeChange => "code_change",
            TaskBudgetClass::Debugging => "debugging",
            TaskBudgetClass::Planning => "planning",
            TaskBudgetClass::EvidenceReview => "evidence_review",
            TaskBudgetClass::Handoff => "handoff",
        }
    }
}

/// Tie-break order for budget classes when keyword scores are equal. Higher
/// index wins ties so the deeper-budget class is preferred when a task is
/// ambiguous (a debugging+navigation task gets the debugging budget). This
/// mirrors the router's deterministic `selection_rank` tie-break: fixed order,
/// no judgment.
const BUDGET_CLASS_ORDER: [TaskBudgetClass; 6] = [
    TaskBudgetClass::Navigation,
    TaskBudgetClass::Handoff,
    TaskBudgetClass::CodeChange,
    TaskBudgetClass::Planning,
    TaskBudgetClass::EvidenceReview,
    TaskBudgetClass::Debugging,
];

/// Per-class keyword rules. Parity with the Python sources:
/// - normalization matches `dagdb_worker_specialty_router.normalize_terms`
///   (lowercase, strip one trailing `s`, alphanumeric runs, len >= 3) via
///   [`normalized_task_terms`];
/// - the keyword vocabulary is drawn from the benchmark family prompts/terms in
///   `tools/dagdb_agent_brain_context_utility.py::PROBES` and
///   `tools/dagdb_agent_brain_semantic_benchmark.py` so each class fires on the
///   same words the live families use. Keywords are stored already normalized
///   (singular, lowercase) so they compare directly against normalized terms.
const BUDGET_CLASS_KEYWORDS: &[(TaskBudgetClass, &[&str])] = &[
    (
        TaskBudgetClass::Navigation,
        &[
            "navigation",
            "navigate",
            "find",
            "locate",
            "where",
            "repo",
            "explore",
        ],
    ),
    (
        TaskBudgetClass::CodeChange,
        &[
            "code",
            "change",
            "implement",
            "implementation",
            "edit",
            "patch",
            "refactor",
            "rust",
        ],
    ),
    (
        TaskBudgetClass::Debugging,
        &[
            "debug",
            "debugging",
            "failing",
            "fail",
            "bug",
            "error",
            "diagnose",
            "blocker",
        ],
    ),
    (
        TaskBudgetClass::Planning,
        &[
            "plan", "planning", "step", "sequence", "next", "roadmap", "schedule",
        ],
    ),
    (
        TaskBudgetClass::EvidenceReview,
        &[
            "evidence", "review", "proof", "audit", "verify", "validate", "trimming",
        ],
    ),
    (
        TaskBudgetClass::Handoff,
        &[
            "handoff",
            "continuation",
            "future",
            "resume",
            "status",
            "summary",
        ],
    ),
];

/// Deterministically classify a task into a budget class by keyword match.
///
/// Pure system-side policy: integer scoring over normalized tokens, fixed
/// tie-break order, default to [`TaskBudgetClass::Navigation`] (the floor
/// budget) when nothing matches. No agent or LLM judgment.
#[must_use]
pub fn classify_task_budget_class(task: &str) -> TaskBudgetClass {
    let terms = normalized_task_terms(task);
    let mut best = TaskBudgetClass::Navigation;
    let mut best_score = 0usize;
    let mut best_rank = budget_class_rank(best);
    for (class, keywords) in BUDGET_CLASS_KEYWORDS {
        let score = keywords
            .iter()
            .filter(|keyword| terms.contains(**keyword))
            .count();
        if score == 0 {
            continue;
        }
        let rank = budget_class_rank(*class);
        if score > best_score || (score == best_score && rank > best_rank) {
            best = *class;
            best_score = score;
            best_rank = rank;
        }
    }
    best
}

/// Per-class token budget for a task, ready to plumb into a selection request.
#[must_use]
pub fn task_budget_tokens(task: &str) -> u32 {
    classify_task_budget_class(task).token_budget()
}

fn budget_class_rank(class: TaskBudgetClass) -> usize {
    BUDGET_CLASS_ORDER
        .iter()
        .position(|candidate| *candidate == class)
        .unwrap_or(0)
}

/// Normalize task text into terms matching the Python router's `normalize_terms`
/// (lowercase, strip one trailing `s`, alphanumeric runs of length >= 3).
fn normalized_task_terms(task: &str) -> BTreeSet<String> {
    task.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(|token| {
            let lowered = token.to_ascii_lowercase();
            lowered
                .strip_suffix('s')
                .map(str::to_owned)
                .unwrap_or(lowered)
        })
        .collect()
}

/// In-memory memory candidate used by graph context selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphContextMemoryCandidate {
    pub memory_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub catalog_id: Option<String>,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub catalog_path: Vec<String>,
    pub document_type: String,
    pub token_estimate: u32,
    pub validation_status: ValidationStatus,
    pub citation_ref: String,
    pub boundary_flags: Vec<String>,
}

/// In-memory graph state used by graph context selection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphContextSelectionState {
    pub memory_candidates: Vec<GraphContextMemoryCandidate>,
    pub graph_edges: Vec<MemoryGraphEdge>,
    pub receipt_ids: Vec<Hash256>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScoredCandidate<'a> {
    candidate: &'a GraphContextMemoryCandidate,
    score: u32,
    selection_reason: String,
    /// True when the deterministic coverage term contributed a nonzero bonus to
    /// this ref's score for THIS task (i.e. at least one identifier/noun-phrase
    /// token extracted from the task text matched the ref's searchable
    /// metadata). Threaded to the family-cap site so coverage-positive refs are
    /// exempt from the diversity cap (see the cap check in
    /// [`select_graph_context`]).
    coverage_positive: bool,
}

/// Select graph context refs and edges for a task from in-memory graph state.
pub fn select_graph_context(
    request: &DagDbGraphContextSelectionRequest,
    state: &GraphContextSelectionState,
) -> DomainResult<DagDbGraphContextSelectionResponse> {
    validate_request(request, state)?;

    let requested_ids = request
        .requested_memory_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let candidates_by_id = index_candidates(state)?;
    let graph_styles_by_memory = graph_styles_for_memories(state);
    let task_terms = task_terms(&request.task);
    let identifier_tokens = identifier_tokens(&request.task);
    let next_step_task = is_next_step_task(&request.task);
    let blocker_task = is_blocker_task(&request.task);

    let mut scored = score_candidates(
        &candidates_by_id,
        request,
        &requested_ids,
        &task_terms,
        &identifier_tokens,
        next_step_task,
        blocker_task,
    );
    scored.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.candidate.memory_id.cmp(&right.candidate.memory_id))
    });

    let mut selected_refs = Vec::<DagDbSelectedContextRef>::new();
    let mut omitted_refs = BTreeMap::<String, DagDbOmittedContextRef>::new();
    let mut selected_token_estimate = 0u32;
    let mut truncated_by_token_budget = false;
    let mut truncated_by_max_memory_refs = false;
    let max_memory_refs =
        usize::try_from(request.max_memory_refs).map_err(|_| DomainError::ArithmeticOverflow {
            operation: "graph_context_max_memory_refs_usize",
        })?;
    let mut selected_family_counts = BTreeMap::<String, usize>::new();

    for entry in &scored {
        let memory_id = &entry.candidate.memory_id;
        if !requested_ids.is_empty() && !requested_ids.contains(memory_id) {
            continue;
        }
        if selected_refs.len() >= max_memory_refs {
            truncated_by_max_memory_refs = true;
            omitted_refs.insert(
                memory_id.clone(),
                DagDbOmittedContextRef {
                    memory_id: memory_id.clone(),
                    omission_reason: "max_memory_refs_exceeded".into(),
                    token_estimate_if_selected: entry.candidate.token_estimate,
                },
            );
            continue;
        }
        let family_key = family_key(entry.candidate);
        // Relevance-aware family cap: a coverage-positive ref (the deterministic
        // coverage term contributed a nonzero bonus for THIS task) is EXEMPT
        // from the diversity cap and does not increment the family count, so it
        // never pushes a coverage-zero sibling out. Diversity pressure exists to
        // stop recency/volume monoculture, not to evict refs the task is asking
        // for; a measured regression (2026-06-10 matrix, capped packets
        // displaced topically relevant families) drove this redesign.
        // Coverage-zero refs keep the existing cap-and-backfill behavior.
        let family_capped = requested_ids.is_empty()
            && !entry.coverage_positive
            && selected_family_counts
                .get(&family_key)
                .is_some_and(|count| *count >= MAX_FAMILY_REF_SHARE);
        if family_capped {
            omitted_refs.insert(
                memory_id.clone(),
                DagDbOmittedContextRef {
                    memory_id: memory_id.clone(),
                    omission_reason: "family_diversity_cap_exceeded".into(),
                    token_estimate_if_selected: entry.candidate.token_estimate,
                },
            );
            continue;
        }
        let next_total = selected_token_estimate.saturating_add(entry.candidate.token_estimate);
        if next_total > request.token_budget {
            truncated_by_token_budget = true;
            omitted_refs.insert(
                memory_id.clone(),
                DagDbOmittedContextRef {
                    memory_id: memory_id.clone(),
                    omission_reason: "token_budget_exceeded".into(),
                    token_estimate_if_selected: entry.candidate.token_estimate,
                },
            );
            continue;
        }

        selected_token_estimate = next_total; // pragma-allowlist-secret
        // Coverage-positive refs bypass the cap AND do not increment the family
        // count, so they never displace coverage-zero siblings of the same
        // family. Only coverage-zero refs contribute to the diversity tally.
        if requested_ids.is_empty() && !entry.coverage_positive {
            *selected_family_counts.entry(family_key).or_insert(0) += 1;
        }
        selected_refs.push(to_selected_ref(
            entry.candidate,
            entry.selection_reason.clone(),
        ));
    }

    // The family cap is a diversity PREFERENCE, not a starvation rule: on a
    // corpus dominated by one catalog family (e.g. everything under
    // docs/dagdb), a hard cap collapses the envelope far below
    // max_memory_refs. Backfill remaining slots from family-capped omissions in
    // score order so packets stay full; diversity still holds whenever
    // alternatives exist because the first pass already preferred other
    // families. (PRD-D4: telemetry is excluded by structure upstream, so the
    // candidate pool here carries no telemetry rows to gate on.)
    if requested_ids.is_empty() && selected_refs.len() < max_memory_refs {
        for entry in &scored {
            if selected_refs.len() >= max_memory_refs {
                break;
            }
            let memory_id = &entry.candidate.memory_id;
            let capped_omission = omitted_refs
                .get(memory_id)
                .is_some_and(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded");
            if !capped_omission {
                continue;
            }
            let next_total = selected_token_estimate.saturating_add(entry.candidate.token_estimate);
            if next_total > request.token_budget {
                continue;
            }
            selected_token_estimate = next_total; // pragma-allowlist-secret
            omitted_refs.remove(memory_id);
            selected_refs.push(to_selected_ref(
                entry.candidate,
                "family_diversity_backfill".to_owned(),
            ));
        }
    }

    for entry in &scored {
        let memory_id = &entry.candidate.memory_id;
        if !requested_ids.is_empty() && !requested_ids.contains(memory_id) {
            omitted_refs.insert(
                memory_id.clone(),
                DagDbOmittedContextRef {
                    memory_id: memory_id.clone(),
                    omission_reason: "requested_memory_filter_mismatch".into(),
                    token_estimate_if_selected: entry.candidate.token_estimate,
                },
            );
            continue;
        }
        if selected_refs
            .iter()
            .any(|selected| selected.memory_id == *memory_id)
            || omitted_refs.contains_key(memory_id)
        {
            continue;
        }
        omitted_refs.insert(
            memory_id.clone(),
            DagDbOmittedContextRef {
                memory_id: memory_id.clone(),
                omission_reason: "not_selected_by_graph_route".into(),
                token_estimate_if_selected: entry.candidate.token_estimate,
            },
        );
    }

    let selected_ids = selected_refs
        .iter()
        .map(|selected| selected.memory_id.clone())
        .collect::<BTreeSet<_>>();
    let trace = graph_route_planner_order()
        .into_iter()
        .map(|graph_style| {
            let style_candidates = candidates_by_id
                .keys()
                .filter(|memory_id| {
                    candidate_matches_graph_style(
                        memory_id,
                        graph_style,
                        &graph_styles_by_memory,
                        state.graph_edges.is_empty(),
                    )
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            let selected_in_style = selected_ids
                .iter()
                .filter(|memory_id| style_candidates.contains(*memory_id))
                .count();
            build_selection_trace_step(graph_style, style_candidates.len(), selected_in_style)
        })
        .collect::<DomainResult<Vec<_>>>()?;

    let mut omitted_memory_refs = omitted_refs.into_values().collect::<Vec<_>>();
    sort_omitted_refs(&mut omitted_memory_refs);

    let (selected_graph_edges, selected_graph_edges_truncated) =
        selected_graph_edges(state, &selected_ids, &request.tenant_id, &request.namespace);
    let mut boundary_warnings = base_boundary_warnings();
    if truncated_by_token_budget {
        push_warning(&mut boundary_warnings, "context_truncated_by_token_budget");
    }
    if truncated_by_max_memory_refs {
        push_warning(
            &mut boundary_warnings,
            "context_truncated_by_max_memory_refs",
        );
    }
    if selected_refs.is_empty() {
        push_warning(&mut boundary_warnings, "no_selected_memory_refs");
    }
    if selected_graph_edges.is_empty() && selected_refs.len() > 1 {
        push_warning(&mut boundary_warnings, "selected_graph_edges_empty");
    }
    if selected_graph_edges_truncated {
        push_warning(
            &mut boundary_warnings,
            "selected_graph_edges_truncated_by_budget",
        );
    }

    let selection_status = if selected_refs.is_empty() {
        DagDbGraphContextSelectionStatus::Empty
    } else {
        DagDbGraphContextSelectionStatus::Selected
    };

    Ok(DagDbGraphContextSelectionResponse {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task_hash: request.task_hash.clone(),
        selection_status,
        selected_memory_refs: selected_refs,
        selected_graph_edges,
        omitted_memory_refs,
        selection_trace: trace,
        selected_token_estimate,
        token_budget: request.token_budget,
        boundary_warnings,
    })
}

fn validate_request(
    request: &DagDbGraphContextSelectionRequest,
    state: &GraphContextSelectionState,
) -> DomainResult<()> {
    if request.task.trim().is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    if request.token_budget == 0 {
        return Err(DomainError::ValidationFailed);
    }
    if request.max_memory_refs == 0 {
        return Err(DomainError::ValidationFailed);
    }

    let mut seen = BTreeSet::<String>::new();
    for candidate in &state.memory_candidates {
        if !seen.insert(candidate.memory_id.clone()) {
            return Err(DomainError::ValidationFailed);
        }
        validate_candidate_no_forbidden_material(candidate)?;
        if candidate.tenant_id != request.tenant_id || candidate.namespace != request.namespace {
            return Err(DomainError::TenantScopeMismatch {
                expected_tenant_id: request.tenant_id.clone(),
                expected_namespace: request.namespace.clone(),
                actual_tenant_id: candidate.tenant_id.clone(),
                actual_namespace: candidate.namespace.clone(),
            });
        }
    }

    for edge in &state.graph_edges {
        if edge.tenant_id != request.tenant_id || edge.namespace != request.namespace {
            return Err(DomainError::TenantScopeMismatch {
                expected_tenant_id: request.tenant_id.clone(),
                expected_namespace: request.namespace.clone(),
                actual_tenant_id: edge.tenant_id.clone(),
                actual_namespace: edge.namespace.clone(),
            });
        }
    }

    let available = state
        .memory_candidates
        .iter()
        .map(|candidate| candidate.memory_id.clone())
        .collect::<BTreeSet<_>>();
    for memory_id in &request.requested_memory_ids {
        if !available.contains(memory_id) {
            return Err(DomainError::ValidationFailed);
        }
    }

    Ok(())
}

fn validate_no_forbidden_material(text: &str) -> DomainResult<()> {
    let normalized = text.to_ascii_lowercase();
    if FORBIDDEN_MATERIAL_FRAGMENTS
        .iter()
        .any(|fragment| normalized.contains(fragment))
    {
        return Err(DomainError::ValidationFailed);
    }
    Ok(())
}

fn validate_candidate_no_forbidden_material(
    candidate: &GraphContextMemoryCandidate,
) -> DomainResult<()> {
    validate_no_forbidden_material(&candidate.title.text)?;
    validate_no_forbidden_material(&candidate.summary.text)?;
    for segment in &candidate.catalog_path {
        validate_no_forbidden_material(segment)?;
    }
    validate_no_forbidden_material(&candidate.document_type)?;
    validate_no_forbidden_material(&candidate.citation_ref)?;
    if let Some(catalog_id) = &candidate.catalog_id {
        validate_no_forbidden_material(catalog_id)?;
    }
    for flag in &candidate.boundary_flags {
        validate_no_forbidden_material(flag)?;
    }
    Ok(())
}

fn index_candidates(
    state: &GraphContextSelectionState,
) -> DomainResult<BTreeMap<String, &GraphContextMemoryCandidate>> {
    let mut indexed = BTreeMap::<String, &GraphContextMemoryCandidate>::new();
    for candidate in &state.memory_candidates {
        if indexed
            .insert(candidate.memory_id.clone(), candidate)
            .is_some()
        {
            return Err(DomainError::ValidationFailed);
        }
    }
    Ok(indexed)
}

/// PRD-D2 (dimension3-prd-02) S4: deterministic roll-up score for one layer
/// root from its members' relevance scores.
///
/// Pure function of the member scores — no new judgment, no LLM. The roll-up is
/// the MAX member score scaled by [`ROLLUP_SCORING_SHARE_BP`] and capped at
/// [`MAX_ROLLUP_SCORE_BP`]: MAX (not mean) so one strongly-relevant member lifts
/// the root, while the scale and cap keep the root from out-ranking its own
/// strongest member or leapfrogging a higher-tier signal. An empty member set
/// or all-zero members earn no bonus, so an irrelevant layer's root gains
/// nothing. This is the system-side aggregate referenced by PRD-D2 S4; the
/// caller adds the returned bonus to the layer root's selection score.
///
/// WIRING NOTE (orchestrator): this pure function is intentionally NOT yet
/// wired into `score_candidates` / `select_graph_context`. The runtime breadth
/// scorer operates over `GraphContextMemoryCandidate`
/// (`graph_context_selection.rs:315-328`), which carries NO layer membership or
/// root linkage, so wiring requires (1) threading each candidate's
/// `is_layer_root` flag + member-id set into the selection input — an upstream
/// query/struct change in `persistent_context.rs` selection internals (the file
/// a sibling PRD is heavily modifying) — and (2) a two-pass scorer in
/// `score_candidates` (score members first, aggregate onto roots, re-score),
/// mirroring the Python lane's two-pass in
/// `dagdb_agent_brain_context_utility.py::select_packet`. Both lanes use the
/// same MAX-scaled-capped rule, so once the candidate carries member linkage the
/// wiring is: build `BTreeMap<root_id, Vec<member_score>>`, call
/// `rollup_score_for_root` per root, and `score.saturating_add(bonus)` at the
/// root with reason `"rollup_member_aggregate"`.
#[must_use]
pub fn rollup_score_for_root(member_scores: &[u32]) -> u32 {
    let Some(best) = member_scores.iter().copied().max() else {
        return 0;
    };
    if best == 0 {
        return 0;
    }
    let scaled = (u64::from(best) * u64::from(ROLLUP_SCORING_SHARE_BP)) / 10_000;
    u32::try_from(scaled.min(u64::from(MAX_ROLLUP_SCORE_BP))).unwrap_or(MAX_ROLLUP_SCORE_BP)
}

fn score_candidates<'a>(
    candidates_by_id: &'a BTreeMap<String, &GraphContextMemoryCandidate>,
    request: &DagDbGraphContextSelectionRequest,
    requested_ids: &BTreeSet<String>,
    task_terms: &BTreeSet<String>,
    identifier_tokens: &BTreeSet<String>, // pragma-allowlist-secret
    next_step_task: bool,
    blocker_task: bool,
) -> Vec<ScoredCandidate<'a>> {
    candidates_by_id
        .values()
        .map(|candidate| {
            let mut score = 0u32;
            let mut reasons = Vec::<String>::new();

            if requested_ids.contains(&candidate.memory_id) {
                score = score.saturating_add(REQUESTED_SCORE);
                reasons.push("requested_memory_id".into());
            }

            if request
                .catalog_hints
                .iter()
                .any(|hint| catalog_hint_matches(hint, candidate))
            {
                score = score.saturating_add(CATALOG_HINT_SCORE);
                reasons.push("catalog_hint_match".into());
            }

            let matched_terms = matched_task_terms(task_terms, candidate);
            if !matched_terms.is_empty() {
                score = score.saturating_add(
                    TASK_TERM_SCORE.saturating_add(u32_from_usize_lossy(matched_terms.len())),
                );
                reasons.push("task_term_match".into());
            }

            let covered_identifiers = matched_identifier_tokens(identifier_tokens, candidate);
            let coverage_positive = !covered_identifiers.is_empty();
            if coverage_positive {
                let coverage_bp = COVERAGE_TERM_BP
                    .saturating_mul(u32_from_usize_lossy(covered_identifiers.len()))
                    .min(MAX_COVERAGE_TERM_BP);
                score = score.saturating_add(coverage_bp);
                reasons.push("identifier_coverage".into());
            }

            if next_step_task && document_type_matches_next_step(&candidate.document_type) {
                score = score.saturating_add(DOCUMENT_TYPE_BOOST.saturating_mul(2));
                reasons.push("next_step_document_type".into());
            }
            if blocker_task && document_type_matches_blocker(&candidate.document_type) {
                score = score.saturating_add(DOCUMENT_TYPE_BOOST.saturating_mul(2));
                reasons.push("blocker_document_type".into());
            }

            let selection_reason = if reasons.is_empty() {
                "graph_route_candidate".into()
            } else {
                reasons.join("|")
            };

            ScoredCandidate {
                candidate,
                score,
                selection_reason,
                coverage_positive,
            }
        })
        .collect()
}

fn selected_graph_edges(
    state: &GraphContextSelectionState,
    selected_ids: &BTreeSet<String>,
    tenant_id: &str,
    namespace: &str,
) -> (Vec<DagDbSelectedGraphEdgeRef>, bool) {
    let mut edges = state
        .graph_edges
        .iter()
        .filter(|edge| {
            edge.tenant_id == tenant_id
                && edge.namespace == namespace
                && selected_ids.contains(&edge.from_memory_id.to_string())
                && selected_ids.contains(&edge.to_memory_id.to_string())
        })
        .map(|edge| DagDbSelectedGraphEdgeRef {
            graph_edge_id: edge.edge_id.to_string(),
            from_memory_id: edge.from_memory_id.to_string(),
            to_memory_id: edge.to_memory_id.to_string(),
            edge_kind: edge.edge_kind,
            graph_style: edge.graph_style,
            selection_reason: "selected_edge_between_selected_memories".into(),
        })
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.graph_edge_id
            .cmp(&right.graph_edge_id)
            .then(left.from_memory_id.cmp(&right.from_memory_id))
            .then(left.to_memory_id.cmp(&right.to_memory_id))
    });
    let truncated = edges.len() > MAX_SELECTED_GRAPH_EDGES_PER_PACKET;
    edges.truncate(MAX_SELECTED_GRAPH_EDGES_PER_PACKET);
    (edges, truncated)
}

fn graph_styles_for_memories(
    state: &GraphContextSelectionState,
) -> BTreeMap<String, BTreeSet<MemoryGraphStyle>> {
    let mut styles = BTreeMap::<String, BTreeSet<MemoryGraphStyle>>::new();
    for edge in &state.graph_edges {
        styles
            .entry(edge.from_memory_id.to_string())
            .or_default()
            .insert(edge.graph_style);
        styles
            .entry(edge.to_memory_id.to_string())
            .or_default()
            .insert(edge.graph_style);
    }
    styles
}

fn candidate_matches_graph_style(
    memory_id: &str,
    graph_style: MemoryGraphStyle,
    graph_styles_by_memory: &BTreeMap<String, BTreeSet<MemoryGraphStyle>>,
    no_edges: bool,
) -> bool {
    if no_edges {
        return true;
    }
    graph_styles_by_memory
        .get(memory_id)
        .is_some_and(|styles| styles.contains(&graph_style))
}

/// Q2-S2 family key: the first two `catalog_path` segments joined by `/`
/// (lowercased), or the single segment when only one exists. Candidates with no
/// catalog path share the empty-path family so an undifferentiated bucket cannot
/// crowd the packet either.
fn family_key(candidate: &GraphContextMemoryCandidate) -> String {
    candidate
        .catalog_path
        .iter()
        .take(2)
        .map(|segment| segment.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("/")
}

fn to_selected_ref(
    candidate: &GraphContextMemoryCandidate,
    selection_reason: String,
) -> DagDbSelectedContextRef {
    DagDbSelectedContextRef {
        memory_id: candidate.memory_id.clone(),
        catalog_id: candidate.catalog_id.clone(),
        title: candidate.title.clone(),
        summary: candidate.summary.clone(),
        catalog_path: candidate.catalog_path.clone(),
        document_type: candidate.document_type.clone(),
        selection_reason,
        token_estimate: candidate.token_estimate,
        validation_status: candidate.validation_status,
        citation_ref: candidate.citation_ref.clone(),
        boundary_flags: candidate.boundary_flags.clone(),
    }
}

fn sort_omitted_refs(values: &mut [DagDbOmittedContextRef]) {
    values.sort_by(|left, right| {
        left.omission_reason
            .cmp(&right.omission_reason)
            .then(left.memory_id.cmp(&right.memory_id))
    });
}

fn task_terms(task: &str) -> BTreeSet<String> {
    task.split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() >= 3)
        .collect()
}

fn matched_task_terms(
    task_terms: &BTreeSet<String>,
    candidate: &GraphContextMemoryCandidate,
) -> BTreeSet<String> {
    let haystack = searchable_text(candidate);
    task_terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .cloned()
        .collect()
}

/// Q2-S2: extract identifier-like tokens from task text. A token qualifies when
/// it looks like code/data the task names rather than a prose word:
/// snake_case (`token_budget`), a dotted path (`mod.fn`, `a.b.c`), a hex id
/// (>= 8 hex chars), or CamelCase (`MemoryGraphStyle`). Returned lowercased for
/// exact-token, case-insensitive matching against candidate metadata — no
/// embeddings, no fuzzy matching.
fn identifier_tokens(task: &str) -> BTreeSet<String> {
    task.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
    })
    .map(str::trim)
    .filter(|token| token.len() >= 3)
    .filter(|token| is_identifier_like(token))
    .map(str::to_ascii_lowercase)
    .collect()
}

fn is_identifier_like(token: &str) -> bool {
    is_snake_case(token) || is_dotted_path(token) || is_hex_id(token) || is_camel_case(token)
}

fn is_snake_case(token: &str) -> bool {
    token.contains('_')
        && token
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        && token
            .chars()
            .any(|character| character.is_ascii_alphanumeric())
}

fn is_dotted_path(token: &str) -> bool {
    token.contains('.')
        && token
            .split('.')
            .all(|segment| !segment.is_empty() && segment.chars().all(is_identifier_char))
        && token
            .chars()
            .any(|character| character.is_ascii_alphabetic())
}

fn is_hex_id(token: &str) -> bool {
    token.len() >= 8 && token.chars().all(|character| character.is_ascii_hexdigit())
}

fn is_camel_case(token: &str) -> bool {
    let alpha_only = token
        .chars()
        .all(|character| character.is_ascii_alphabetic());
    let has_lower = token
        .chars()
        .any(|character| character.is_ascii_lowercase());
    let inner_upper = token
        .chars()
        .skip(1)
        .any(|character| character.is_ascii_uppercase());
    alpha_only && has_lower && inner_upper
}

fn is_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn matched_identifier_tokens(
    identifier_tokens: &BTreeSet<String>, // pragma-allowlist-secret
    candidate: &GraphContextMemoryCandidate,
) -> BTreeSet<String> {
    if identifier_tokens.is_empty() {
        return BTreeSet::new();
    }
    let haystack = searchable_text(candidate);
    identifier_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .cloned()
        .collect()
}

fn searchable_text(candidate: &GraphContextMemoryCandidate) -> String {
    let path = candidate.catalog_path.join("/");
    format!(
        "{} {} {} {} {}",
        candidate.title.text.to_ascii_lowercase(),
        candidate.summary.text.to_ascii_lowercase(),
        path.to_ascii_lowercase(),
        candidate.document_type.to_ascii_lowercase(),
        candidate
            .catalog_id
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    )
}

fn catalog_hint_matches(hint: &str, candidate: &GraphContextMemoryCandidate) -> bool {
    let normalized_hint = hint.trim().trim_matches('/').to_ascii_lowercase();
    if normalized_hint.is_empty() {
        return false;
    }
    candidate.catalog_path.iter().any(|segment| {
        segment.to_ascii_lowercase().contains(&normalized_hint)
            || normalized_hint.contains(&segment.to_ascii_lowercase())
    }) || candidate
        .catalog_id
        .as_deref()
        .is_some_and(|catalog_id| catalog_id.to_ascii_lowercase().contains(&normalized_hint))
}

fn is_next_step_task(task: &str) -> bool {
    let normalized = task.to_ascii_lowercase();
    normalized.contains("next step")
        || normalized.contains("next-step")
        || normalized.contains("next implementation")
        || normalized.contains("implementation step")
        || normalized.contains("next phase")
}

fn is_blocker_task(task: &str) -> bool {
    let normalized = task.to_ascii_lowercase();
    normalized.contains("blocker")
        || normalized.contains("blocking")
        || normalized.contains("open question")
        || normalized.contains("what blocks")
}

fn document_type_matches_next_step(document_type: &str) -> bool {
    matches!(
        document_type.to_ascii_lowercase().as_str(),
        "plan" | "implementation_plan" | "next_step" | "route"
    )
}

fn document_type_matches_blocker(document_type: &str) -> bool {
    matches!(
        document_type.to_ascii_lowercase().as_str(),
        "blocker" | "open_question" | "risk" | "contradiction"
    )
}

fn graph_style_label(graph_style: MemoryGraphStyle) -> &'static str {
    match graph_style {
        MemoryGraphStyle::SemanticCatalogGraph => "semantic_catalog_graph",
        MemoryGraphStyle::CanonicalMemoryGraph => "canonical_memory_graph",
        MemoryGraphStyle::ProvenanceReceiptDag => "provenance_receipt_dag",
        MemoryGraphStyle::ContradictionSupersessionGraph => "contradiction_supersession_graph",
        MemoryGraphStyle::RoutingViewGraph => "routing_view_graph",
        MemoryGraphStyle::DependencyDag => "dependency_dag",
        MemoryGraphStyle::ContextPacketGraph => "context_packet_graph",
        MemoryGraphStyle::SimilarityOverlayGraph => "similarity_overlay_graph",
    }
}

fn graph_style_reason(graph_style: MemoryGraphStyle) -> String {
    format!("{}_considered", graph_style_label(graph_style))
}

fn base_boundary_warnings() -> Vec<String> {
    vec![
        "production_runtime_not_approved".into(),
        "gateway_api_not_approved".into(),
        "route_activation_not_approved".into(),
        "postgres_read_not_required_for_m01".into(),
    ]
}

fn push_warning(warnings: &mut Vec<String>, warning: impl Into<String>) {
    let warning = warning.into();
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn build_selection_trace_step(
    graph_style: MemoryGraphStyle,
    style_candidate_count: usize,
    selected_in_style: usize,
) -> DomainResult<DagDbGraphSelectionTraceStep> {
    Ok(DagDbGraphSelectionTraceStep {
        graph_style,
        candidate_count_before: u32_from_usize("candidate_count_before", style_candidate_count)?,
        candidate_count_after: u32_from_usize("candidate_count_after", style_candidate_count)?,
        selected_count_after: u32_from_usize("selected_count_after", selected_in_style)?,
        reason: graph_style_reason(graph_style),
    })
}

fn u32_from_usize(field: &'static str, value: usize) -> DomainResult<u32> {
    u32::try_from(value).map_err(|_| DomainError::ArithmeticOverflow { operation: field })
}

fn u32_from_usize_lossy(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use exo_dag_db_api::{
        DagDbGraphContextSelectionStatus, MemoryEdgeKind, SafeMetadataDecision, ValidationStatus,
    };

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
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
        memory_id: &str,
        document_type: &str,
        title: &str,
        summary: &str,
        catalog_path: &[&str],
        token_estimate: u32,
    ) -> GraphContextMemoryCandidate {
        GraphContextMemoryCandidate {
            memory_id: memory_id.into(),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            catalog_id: Some(format!("catalog-{memory_id}")),
            title: safe(title),
            summary: safe(summary),
            catalog_path: catalog_path.iter().map(|part| (*part).to_owned()).collect(),
            document_type: document_type.into(),
            token_estimate,
            validation_status: ValidationStatus::Passed,
            citation_ref: format!("citation:{memory_id}"),
            boundary_flags: vec!["repository_test_only".into()],
        }
    }

    fn base_request(task: &str) -> DagDbGraphContextSelectionRequest {
        DagDbGraphContextSelectionRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task: task.into(),
            task_hash: h(0x11).to_string(),
            token_budget: 1_000,
            max_memory_refs: 4,
            catalog_hints: Vec::new(),
            requested_memory_ids: Vec::new(),
            force_revalidate: false,
        }
    }

    fn mem_id(byte: u8) -> String {
        h(byte).to_string()
    }

    fn edge(from: u8, to: u8, graph_style: MemoryGraphStyle) -> MemoryGraphEdge {
        MemoryGraphEdge::new(
            "tenant-a".into(),
            "primary".into(),
            h(from),
            h(to),
            MemoryEdgeKind::RelatedTo,
            graph_style,
            None,
        )
        .expect("edge")
    }

    #[test]
    fn graph_context_selection_is_deterministic() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "plan",
                    "Next Steps",
                    "Implementation plan for M01",
                    &["04_Plans", "Next Steps"],
                    120,
                ),
                candidate(
                    &mem_id(0x02),
                    "blocker",
                    "Open blocker",
                    "Blocker for shipping",
                    &["08_Open_Questions"],
                    140,
                ),
            ],
            graph_edges: vec![edge(0x01, 0x02, MemoryGraphStyle::SemanticCatalogGraph)],
            receipt_ids: vec![h(0xaa)],
        };
        let request = base_request("What is the next implementation step for M01?");
        let first = select_graph_context(&request, &state).expect("first selection");
        let second = select_graph_context(&request, &state).expect("second selection");
        assert_eq!(first, second);
        let first_json = serde_json::to_string(&first).expect("json");
        let second_json = serde_json::to_string(&second).expect("json");
        assert_eq!(first_json, second_json);
    }

    #[test]
    fn graph_context_selection_trace_follows_route_planner_order() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![candidate(
                &mem_id(0x01),
                "plan",
                "Next Steps",
                "Implementation plan",
                &["04_Plans"],
                100,
            )],
            graph_edges: vec![edge(0x01, 0x02, MemoryGraphStyle::SemanticCatalogGraph)],
            receipt_ids: Vec::new(),
        };
        let response = select_graph_context(
            &base_request("What is the next implementation step?"),
            &state,
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
    fn graph_context_selection_rejects_invalid_requests() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![candidate(
                &mem_id(0x01),
                "plan",
                "Plan",
                "Plan summary",
                &["04_Plans"],
                100,
            )],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };

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

        let mut missing_requested = base_request("task");
        missing_requested.requested_memory_ids = vec!["missing".into()];
        assert_eq!(
            select_graph_context(&missing_requested, &state),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn graph_context_selection_rejects_duplicate_memory_ids() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100),
                candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        assert_eq!(
            select_graph_context(&base_request("task"), &state),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn graph_context_selection_rejects_tenant_scope_mismatch() {
        let mut candidate = candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100);
        candidate.tenant_id = "tenant-b".into();
        let state = GraphContextSelectionState {
            memory_candidates: vec![candidate],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        assert!(matches!(
            select_graph_context(&base_request("task"), &state),
            Err(DomainError::TenantScopeMismatch { .. })
        ));
    }

    #[test]
    fn graph_context_selection_truncates_by_max_memory_refs() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Alpha",
                    "Alpha",
                    &["00_Index"],
                    10,
                ),
                candidate(&mem_id(0x02), "summary", "Beta", "Beta", &["00_Index"], 10),
                candidate(
                    &mem_id(0x03),
                    "summary",
                    "Gamma",
                    "Gamma",
                    &["00_Index"],
                    10,
                ),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("generic catalog task terms");
        request.max_memory_refs = 1;
        let response = select_graph_context(&request, &state).expect("selection");
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
    fn graph_context_selection_requested_filter_mismatch_omits_non_requested() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Alpha",
                    "Alpha",
                    &["00_Index"],
                    10,
                ),
                candidate(&mem_id(0x02), "summary", "Beta", "Beta", &["00_Index"], 10),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("generic catalog task terms");
        request.requested_memory_ids = vec![mem_id(0x01)];
        let response = select_graph_context(&request, &state).expect("selection");
        assert!(response.omitted_memory_refs.iter().any(|omitted| {
            omitted.memory_id == mem_id(0x02)
                && omitted.omission_reason == "requested_memory_filter_mismatch"
        }));
    }

    #[test]
    fn graph_context_selection_empty_state_returns_empty_status() {
        let state = GraphContextSelectionState {
            memory_candidates: Vec::new(),
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let response =
            select_graph_context(&base_request("task"), &state).expect("empty selection");
        assert_eq!(
            response.selection_status,
            DagDbGraphContextSelectionStatus::Empty
        );
        assert!(
            response
                .boundary_warnings
                .contains(&"no_selected_memory_refs".to_owned())
        );
    }

    #[test]
    fn graph_context_selection_warns_when_selected_edges_missing() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Alpha",
                    "Alpha",
                    &["00_Index"],
                    10,
                ),
                candidate(&mem_id(0x02), "summary", "Beta", "Beta", &["00_Index"], 10),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("generic catalog task terms");
        request.max_memory_refs = 2;
        request.token_budget = 1_000;
        let response = select_graph_context(&request, &state).expect("selection");
        assert_eq!(response.selected_memory_refs.len(), 2);
        assert!(
            response
                .boundary_warnings
                .contains(&"selected_graph_edges_empty".to_owned())
        );
    }

    #[test]
    fn graph_context_selection_rejects_edge_tenant_scope_mismatch() {
        let mismatched_edge = MemoryGraphEdge::new(
            "tenant-b".into(),
            "primary".into(),
            h(0x01),
            h(0x02),
            MemoryEdgeKind::RelatedTo,
            MemoryGraphStyle::DependencyDag,
            None,
        )
        .expect("edge");
        let state = GraphContextSelectionState {
            memory_candidates: vec![candidate(
                &mem_id(0x01),
                "plan",
                "Plan",
                "Plan",
                &["04_Plans"],
                100,
            )],
            graph_edges: vec![mismatched_edge],
            receipt_ids: Vec::new(),
        };
        assert!(matches!(
            select_graph_context(&base_request("task"), &state),
            Err(DomainError::TenantScopeMismatch { .. })
        ));
    }

    #[test]
    fn graph_context_selection_catalog_hint_boosts_matching_candidate() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Alpha",
                    "Alpha",
                    &["04_Plans", "Next Steps"],
                    100,
                ),
                candidate(&mem_id(0x02), "summary", "Beta", "Beta", &["00_Index"], 100),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("unrelated task text");
        request.catalog_hints = vec!["Next Steps".into()];
        let response = select_graph_context(&request, &state).expect("selection");
        assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x01));
        assert!(
            response.selected_memory_refs[0]
                .selection_reason
                .contains("catalog_hint_match")
        );
    }

    #[test]
    fn graph_context_selection_uses_graph_route_candidate_reason_without_signals() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![candidate(
                &mem_id(0x01),
                "summary",
                "Alpha",
                "Alpha",
                &["00_Index"],
                100,
            )],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let response =
            select_graph_context(&base_request("xyzzy qwerty"), &state).expect("selection");
        assert_eq!(
            response.selected_memory_refs[0].selection_reason,
            "graph_route_candidate"
        );
    }

    #[test]
    fn graph_context_selection_omits_lower_ranked_candidates_when_budget_is_tight() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Alpha",
                    "Alpha",
                    &["00_Index"],
                    300,
                ),
                candidate(&mem_id(0x02), "summary", "Beta", "Beta", &["00_Index"], 300),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("generic catalog task terms");
        request.token_budget = 300;
        request.max_memory_refs = 2;
        let response = select_graph_context(&request, &state).expect("selection");
        assert_eq!(response.selected_memory_refs.len(), 1);
        assert!(
            response
                .omitted_memory_refs
                .iter()
                .any(|omitted| omitted.omission_reason == "token_budget_exceeded")
        );
    }

    #[test]
    fn graph_context_selection_all_graph_style_reasons_are_non_empty() {
        for graph_style in graph_route_planner_order() {
            assert!(!graph_style_reason(graph_style).is_empty());
        }
    }

    #[test]
    fn graph_context_selection_push_warning_deduplicates() {
        let mut warnings = vec!["existing".into()];
        push_warning(&mut warnings, "existing");
        push_warning(&mut warnings, "new");
        assert_eq!(warnings, vec!["existing", "new"]);
    }

    #[test]
    fn graph_context_selection_candidate_matches_graph_style_when_no_edges() {
        assert!(candidate_matches_graph_style(
            &mem_id(0x01),
            MemoryGraphStyle::SemanticCatalogGraph,
            &BTreeMap::new(),
            true,
        ));
    }

    #[test]
    fn graph_context_selection_candidate_matches_only_recorded_graph_styles() {
        let mut styles = BTreeMap::<String, BTreeSet<MemoryGraphStyle>>::new();
        styles
            .entry(mem_id(0x01))
            .or_default()
            .insert(MemoryGraphStyle::SemanticCatalogGraph);

        assert!(candidate_matches_graph_style(
            &mem_id(0x01),
            MemoryGraphStyle::SemanticCatalogGraph,
            &styles,
            false,
        ));
        assert!(!candidate_matches_graph_style(
            &mem_id(0x01),
            MemoryGraphStyle::DependencyDag,
            &styles,
            false,
        ));
        assert!(!candidate_matches_graph_style(
            &mem_id(0x02),
            MemoryGraphStyle::SemanticCatalogGraph,
            &styles,
            false,
        ));
    }

    #[test]
    fn graph_context_selection_selected_edges_filter_scope_and_endpoints_then_sort() {
        let selected_ids = [mem_id(0x01), mem_id(0x02)]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let wrong_namespace = MemoryGraphEdge::new(
            "tenant-a".into(),
            "secondary".into(),
            h(0x01),
            h(0x02),
            MemoryEdgeKind::RelatedTo,
            MemoryGraphStyle::DependencyDag,
            None,
        )
        .expect("wrong namespace edge");
        let state = GraphContextSelectionState {
            memory_candidates: Vec::new(),
            graph_edges: vec![
                edge(0x02, 0x01, MemoryGraphStyle::DependencyDag),
                wrong_namespace,
                edge(0x01, 0x03, MemoryGraphStyle::SemanticCatalogGraph),
                edge(0x01, 0x02, MemoryGraphStyle::CanonicalMemoryGraph),
            ],
            receipt_ids: Vec::new(),
        };

        let (edges, truncated) = selected_graph_edges(&state, &selected_ids, "tenant-a", "primary");

        assert!(!truncated);
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|edge| {
            selected_ids.contains(&edge.from_memory_id) && selected_ids.contains(&edge.to_memory_id)
        }));
        assert!(
            edges
                .iter()
                .all(|edge| edge.selection_reason == "selected_edge_between_selected_memories")
        );
        let mut sorted_edge_ids = edges
            .iter()
            .map(|edge| edge.graph_edge_id.clone())
            .collect::<Vec<_>>();
        sorted_edge_ids.sort();
        assert_eq!(
            edges
                .iter()
                .map(|edge| edge.graph_edge_id.clone())
                .collect::<Vec<_>>(),
            sorted_edge_ids
        );
    }

    #[test]
    fn graph_context_selection_catalog_hint_matches_empty_hint_is_false() {
        let candidate = candidate(
            &mem_id(0x01),
            "summary",
            "Alpha",
            "Alpha",
            &["04_Plans"],
            10,
        );
        assert!(!catalog_hint_matches("", &candidate));
        assert!(!catalog_hint_matches("   ", &candidate));
    }

    #[test]
    fn graph_context_selection_catalog_hint_matches_segments_and_catalog_id() {
        let mut candidate = candidate(
            &mem_id(0x01),
            "summary",
            "Alpha",
            "Alpha",
            &["04_Plans", "Next Steps"],
            10,
        );
        candidate.catalog_id = Some("decision-catalog".into());

        assert!(catalog_hint_matches("plans", &candidate));
        assert!(catalog_hint_matches("/04_Plans/Next Steps/", &candidate));
        assert!(catalog_hint_matches("decision", &candidate));
        assert!(!catalog_hint_matches("missing", &candidate));
    }

    #[test]
    fn graph_context_selection_task_term_helpers_cover_aliases() {
        assert!(is_next_step_task("What is the next phase?"));
        assert!(is_blocker_task("What blocks shipping?"));
        assert!(document_type_matches_next_step("implementation_plan"));
        assert!(document_type_matches_blocker("open_question"));
        let terms = task_terms("ab xy implementation");
        assert!(terms.contains("implementation"));
        assert!(!terms.contains("ab"));
    }

    #[test]
    fn graph_context_selection_task_helpers_reject_non_aliases_and_cover_variants() {
        assert!(!is_next_step_task("Summarize current status"));
        assert!(!is_blocker_task("Summarize current status"));
        assert!(document_type_matches_next_step("route"));
        assert!(document_type_matches_blocker("risk"));
        assert!(document_type_matches_blocker("contradiction"));
        assert!(!document_type_matches_next_step("summary"));
        assert!(!document_type_matches_blocker("summary"));
    }

    #[test]
    fn graph_context_selection_similarity_overlay_graph_style_label() {
        assert_eq!(
            graph_style_reason(MemoryGraphStyle::SimilarityOverlayGraph),
            "similarity_overlay_graph_considered"
        );
    }

    #[test]
    fn graph_context_selection_index_candidates_rejects_duplicate_ids() {
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100),
                candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        assert_eq!(index_candidates(&state), Err(DomainError::ValidationFailed));
    }

    #[test]
    fn graph_context_selection_rejects_forbidden_catalog_id_material() {
        let mut blocked = candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100);
        blocked.catalog_id = Some("Leaked postgres:// credential".into());
        let state = GraphContextSelectionState {
            memory_candidates: vec![blocked],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        assert_eq!(
            select_graph_context(&base_request("task"), &state),
            Err(DomainError::ValidationFailed)
        );
    }

    #[test]
    fn graph_context_selection_u32_from_usize_rejects_overflow() {
        assert!(matches!(
            u32_from_usize("trace_candidate_count", usize::MAX),
            Err(DomainError::ArithmeticOverflow { .. })
        ));
    }

    #[test]
    fn graph_context_selection_u32_from_usize_lossy_saturates_on_overflow() {
        assert_eq!(u32_from_usize_lossy(42), 42);
        assert_eq!(u32_from_usize_lossy(usize::MAX), u32::MAX);
    }

    #[test]
    fn graph_context_selection_trace_step_rejects_candidate_count_overflow() {
        assert!(matches!(
            build_selection_trace_step(MemoryGraphStyle::SemanticCatalogGraph, usize::MAX, 0),
            Err(DomainError::ArithmeticOverflow { .. })
        ));
    }

    #[test]
    fn graph_context_selection_trace_step_rejects_selected_count_overflow() {
        assert!(matches!(
            build_selection_trace_step(MemoryGraphStyle::DependencyDag, 1, usize::MAX),
            Err(DomainError::ArithmeticOverflow { .. })
        ));
    }

    #[test]
    fn graph_context_selection_validates_candidate_without_catalog_id() {
        let mut without_catalog =
            candidate(&mem_id(0x01), "plan", "Plan", "Plan", &["04_Plans"], 100);
        without_catalog.catalog_id = None;
        let state = GraphContextSelectionState {
            memory_candidates: vec![without_catalog],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let response = select_graph_context(&base_request("task"), &state).expect("selection");
        assert_eq!(response.selected_memory_refs.len(), 1);
        assert!(response.selected_memory_refs[0].catalog_id.is_none());
    }

    #[test]
    fn telemetry_quota_is_retired_no_special_cap_on_usage_event_titles() {
        // PRD-D4: the read-side telemetry quota and title-prefix heuristic are
        // gone. The pure selection function applies NO special cap on
        // "usage event"-titled candidates: telemetry is excluded by structure
        // at the SQL loader before candidates reach this function, so anything
        // that does arrive is scored and budgeted like any other ref. Sixteen
        // "usage event"-titled candidates with distinct families therefore all
        // fit a 16-slot envelope, and no `telemetry_ref_quota_exceeded` omission
        // is ever produced.
        let mut memory_candidates = Vec::new();
        for index in 0u8..16u8 {
            let sub_family = format!("Sub_{index}");
            memory_candidates.push(candidate(
                &mem_id(index),
                "plan",
                &format!("usage event req-{index}"),
                "selection telemetry for task",
                &["telemetry", &sub_family],
                10,
            ));
        }
        let state = GraphContextSelectionState {
            memory_candidates,
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("task");
        request.max_memory_refs = 16;
        let response = select_graph_context(&request, &state).expect("selection");
        assert_eq!(
            response.selected_memory_refs.len(),
            16,
            "no telemetry quota caps the envelope"
        );
        assert!(
            response
                .omitted_memory_refs
                .iter()
                .all(|omitted| omitted.omission_reason != "telemetry_ref_quota_exceeded"),
            "retired telemetry quota must never produce an omission reason"
        );
    }

    // ---- Q2-S1 budget class goldens -------------------------------------

    #[test]
    fn budget_class_goldens_map_families_to_budgets() {
        // Golden table: representative task per family -> class -> budget.
        let cases = [
            (
                "Find and navigate the repo to locate the writeback module",
                TaskBudgetClass::Navigation,
                2_048u32,
            ),
            (
                "Implement the code change to patch the selection scoring in rust",
                TaskBudgetClass::CodeChange,
                4_096,
            ),
            (
                "Debug the failing retrieval context packet and diagnose the error",
                TaskBudgetClass::Debugging,
                8_192,
            ),
            (
                "Plan the next implementation step sequence for the agent brain",
                TaskBudgetClass::Planning,
                4_096,
            ),
            (
                "Review the evidence and proof for corpus import and verify trimming",
                TaskBudgetClass::EvidenceReview,
                8_192,
            ),
            (
                "Prepare a handoff continuation status summary for future agents",
                TaskBudgetClass::Handoff,
                2_048,
            ),
        ];
        for (task, expected_class, expected_budget) in cases {
            assert_eq!(
                classify_task_budget_class(task),
                expected_class,
                "task did not classify as expected: {task}"
            );
            assert_eq!(
                task_budget_tokens(task),
                expected_budget,
                "budget for {task}"
            );
        }
    }

    #[test]
    fn budget_class_unmatched_task_falls_back_to_navigation_floor() {
        assert_eq!(
            classify_task_budget_class("xyzzy qwerty grault"),
            TaskBudgetClass::Navigation
        );
        assert_eq!(task_budget_tokens("xyzzy qwerty grault"), 2_048);
    }

    #[test]
    fn budget_class_ties_prefer_deeper_budget() {
        // "navigate" (Navigation) + "debug" (Debugging) each score 1; the
        // deeper-budget class (Debugging) wins the tie via fixed order.
        assert_eq!(
            classify_task_budget_class("navigate then debug"),
            TaskBudgetClass::Debugging
        );
    }

    #[test]
    fn budget_class_every_budget_is_within_floor_and_cap() {
        for class in BUDGET_CLASS_ORDER {
            let budget = class.token_budget();
            assert!(
                (TASK_BUDGET_FLOOR_TOKENS..=TASK_BUDGET_CAP_TOKENS).contains(&budget),
                "{} budget {budget} outside [floor, cap]",
                class.label()
            );
        }
    }

    #[test]
    fn budget_class_normalization_matches_router_singular_stripping() {
        // "steps" normalizes to "step" (one trailing 's' stripped), matching the
        // Python router; Planning fires on the singular keyword.
        let terms = normalized_task_terms("plan the remaining steps");
        assert!(terms.contains("step"));
        assert!(terms.contains("plan"));
        assert_eq!(
            classify_task_budget_class("plan the steps"),
            TaskBudgetClass::Planning
        );
    }

    // ---- Q2-S2 identifier-token extraction ------------------------------

    #[test]
    fn identifier_tokens_extract_only_identifier_like_tokens() {
        let tokens = identifier_tokens(
            "Fix token_budget in mod.fn for MemoryGraphStyle id aabbccdd11 but not the plain words",
        );
        assert!(tokens.contains("token_budget"), "snake_case missing");
        assert!(tokens.contains("mod.fn"), "dotted path missing");
        assert!(tokens.contains("memorygraphstyle"), "CamelCase missing");
        assert!(tokens.contains("aabbccdd11"), "hex id missing");
        // Plain prose words are not identifier-like.
        assert!(!tokens.contains("plain"));
        assert!(!tokens.contains("words"));
        assert!(!tokens.contains("fix"));
    }

    #[test]
    fn identifier_predicates_classify_token_shapes() {
        assert!(is_snake_case("token_budget"));
        assert!(!is_snake_case("plainword"));
        assert!(is_dotted_path("a.b.c"));
        assert!(!is_dotted_path("3.14"));
        assert!(is_hex_id("deadbeef"));
        assert!(!is_hex_id("dead"));
        assert!(is_camel_case("MemoryGraphStyle"));
        assert!(is_camel_case("camelCase"));
        assert!(!is_camel_case("alllower"));
        assert!(!is_camel_case("ALLUPPER"));
    }

    #[test]
    fn coverage_term_ranks_identifier_match_above_equal_score_non_match() {
        // Two refs with identical base signals (no task-term, hint, or doc-type
        // match). One mentions the identifier `token_budget` from the task, the
        // other does not. The coverage term must rank the matching ref first.
        let state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Alpha note",
                    "discusses the token_budget knob in detail",
                    &["00_Index"],
                    10,
                ),
                candidate(
                    &mem_id(0x02),
                    "summary",
                    "Beta note",
                    "discusses an unrelated topic entirely",
                    &["00_Index"],
                    10,
                ),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        // Task names the identifier but shares no plain task terms with summaries.
        let response =
            select_graph_context(&base_request("Inspect token_budget behaviour"), &state)
                .expect("coverage selection");
        assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x01));
        assert!(
            response.selected_memory_refs[0]
                .selection_reason
                .contains("identifier_coverage")
        );
    }

    #[test]
    fn coverage_term_does_not_reorder_higher_tier_signals() {
        // A catalog-hint ref must still outrank an identifier-coverage-only ref:
        // the coverage term is additive and capped below the hint tier.
        let mut state = GraphContextSelectionState {
            memory_candidates: vec![
                candidate(
                    &mem_id(0x01),
                    "summary",
                    "Hinted ref",
                    "no identifiers here",
                    &["04_Plans", "Next Steps"],
                    10,
                ),
                candidate(
                    &mem_id(0x02),
                    "summary",
                    "Identifier ref",
                    "mentions token_budget and MemoryGraphStyle and another_ident",
                    &["00_Index"],
                    10,
                ),
            ],
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        state.memory_candidates[0].catalog_id = Some("plain-catalog".into());
        let mut request =
            base_request("token_budget MemoryGraphStyle another_ident unrelated hint task");
        request.catalog_hints = vec!["Next Steps".into()];
        let response = select_graph_context(&request, &state).expect("tiered selection");
        assert_eq!(response.selected_memory_refs[0].memory_id, mem_id(0x01));
        assert!(
            response.selected_memory_refs[0]
                .selection_reason
                .contains("catalog_hint_match")
        );
    }

    // ---- Q2-S2 family diversity cap -------------------------------------

    #[test]
    fn family_cap_limits_refs_sharing_top_two_segments() {
        // 10 refs all under the same top-2 family plus distinct refs in other
        // families. The shared family is capped at MAX_FAMILY_REF_SHARE (8).
        let mut memory_candidates = Vec::new();
        for index in 0u8..10u8 {
            memory_candidates.push(candidate(
                &mem_id(index),
                "summary",
                &format!("Crowded ref {index}"),
                "actionable task memory in one family",
                &["04_Plans", "Next Steps"],
                10,
            ));
        }
        for index in 10u8..14u8 {
            memory_candidates.push(candidate(
                &mem_id(index),
                "summary",
                &format!("Other family ref {index}"),
                "actionable task memory in another family",
                &["08_Open_Questions", "Blockers"],
                10,
            ));
        }
        let state = GraphContextSelectionState {
            memory_candidates,
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("actionable task memory family");
        // Constrain the envelope so the cap can bind: 12 slots for 14
        // candidates. The first pass prefers 8 crowded + 4 other-family refs;
        // soft backfill then fills the remaining slots from the capped family
        // so the envelope never starves.
        request.max_memory_refs = 12;
        request.token_budget = 100_000; // pragma-allowlist-secret
        let response = select_graph_context(&request, &state).expect("family-cap selection");
        assert_eq!(response.selected_memory_refs.len(), 12);
        let crowded_selected = response
            .selected_memory_refs
            .iter()
            .filter(|selected| selected.catalog_path == ["04_Plans", "Next Steps"])
            .count();
        // 8 first-pass + 4 backfilled into the slots the other family could
        // not fill is impossible here (other family has 4 and all fit), so the
        // crowded family lands at exactly 8 from the first pass.
        assert_eq!(crowded_selected, MAX_FAMILY_REF_SHARE);
        assert!(
            response
                .selected_memory_refs
                .iter()
                .all(|selected| selected.selection_reason != "family_diversity_backfill"),
            "no backfill needed when alternatives fill the envelope"
        );
        assert!(
            response
                .omitted_memory_refs
                .iter()
                .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded")
        );
        // The other family is fully retained: diversity is preserved.
        let other_selected = response
            .selected_memory_refs
            .iter()
            .filter(|selected| selected.catalog_path == ["08_Open_Questions", "Blockers"])
            .count();
        assert_eq!(other_selected, 4);
    }

    #[test]
    fn family_cap_backfills_instead_of_starving_on_monolithic_corpus() {
        // A corpus dominated by ONE family must still fill the envelope: the
        // cap yields a diversity preference, then soft backfill tops the
        // packet back up to max_memory_refs in score order.
        let mut memory_candidates = Vec::new();
        for index in 0u8..16u8 {
            memory_candidates.push(candidate(
                &mem_id(index),
                "summary",
                &format!("Monolithic ref {index}"),
                "actionable task memory in one family",
                &["docs", "dagdb"],
                10,
            ));
        }
        let state = GraphContextSelectionState {
            memory_candidates,
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("actionable task memory");
        request.max_memory_refs = 12;
        request.token_budget = 100_000; // pragma-allowlist-secret
        let response = select_graph_context(&request, &state).expect("monolithic selection");
        assert_eq!(
            response.selected_memory_refs.len(),
            12,
            "envelope must not starve below max_memory_refs"
        );
        let backfilled = response
            .selected_memory_refs
            .iter()
            .filter(|selected| selected.selection_reason == "family_diversity_backfill")
            .count();
        assert_eq!(backfilled, 12 - MAX_FAMILY_REF_SHARE);
    }

    #[test]
    fn family_cap_exempts_explicit_requested_ids() {
        // 10 refs in one family, all explicitly requested: the cap must not
        // apply (relink retrieval returns exactly what was asked).
        let mut memory_candidates = Vec::new();
        let mut requested = Vec::new();
        for index in 0u8..10u8 {
            memory_candidates.push(candidate(
                &mem_id(index),
                "summary",
                &format!("Requested ref {index}"),
                "requested family memory",
                &["04_Plans", "Next Steps"],
                10,
            ));
            requested.push(mem_id(index));
        }
        let state = GraphContextSelectionState {
            memory_candidates,
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("requested family memory");
        request.max_memory_refs = 64;
        request.token_budget = 100_000; // pragma-allowlist-secret
        request.requested_memory_ids = requested;
        let response = select_graph_context(&request, &state).expect("requested selection");
        assert_eq!(response.selected_memory_refs.len(), 10);
        assert!(
            !response
                .omitted_memory_refs
                .iter()
                .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded")
        );
    }

    #[test]
    fn family_cap_exempts_coverage_positive_refs_beyond_cap_without_backfill() {
        // Relevance-aware cap: a narrow-domain task names the identifier
        // `token_budget`. 12 refs of ONE family all mention `token_budget` in
        // their summary, so every one is coverage-positive. Coverage-positive
        // refs bypass the family cap AND do not increment the family count, so
        // all 12 are selected as primary picks (never as backfill) and none is
        // omitted with the family-cap reason — the task keeps the material it
        // is asking for instead of having relevant siblings evicted for
        // diversity.
        let mut memory_candidates = Vec::new();
        for index in 0u8..12u8 {
            memory_candidates.push(candidate(
                &mem_id(index),
                "summary",
                &format!("Coverage ref {index}"),
                "mentions the token_budget knob for this task",
                &["04_Plans", "Next Steps"],
                10,
            ));
        }
        let state = GraphContextSelectionState {
            memory_candidates,
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        let mut request = base_request("Inspect token_budget behaviour");
        request.max_memory_refs = 12;
        request.token_budget = 100_000; // pragma-allowlist-secret
        let response = select_graph_context(&request, &state).expect("coverage-exempt selection");
        assert_eq!(response.selected_memory_refs.len(), 12);
        // Every selected ref earned the coverage reason and NONE is a backfill:
        // coverage exemption admits them all on the primary pass.
        assert!(
            response
                .selected_memory_refs
                .iter()
                .all(|selected| selected.selection_reason.contains("identifier_coverage")),
            "coverage-positive refs must keep the identifier_coverage reason"
        );
        assert!(
            response
                .selected_memory_refs
                .iter()
                .all(|selected| selected.selection_reason != "family_diversity_backfill"),
            "coverage-positive refs are primary picks, never backfill"
        );
        // The cap never fired: no family-cap omission was recorded.
        assert!(
            !response
                .omitted_memory_refs
                .iter()
                .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded"),
            "coverage-positive refs must not be omitted by the family cap"
        );
    }

    #[test]
    fn family_cap_still_caps_then_backfills_coverage_zero_ninth_ref() {
        // Coverage-ZERO refs keep the existing cap-and-backfill behavior. The
        // task shares no identifier token with these refs, so none is
        // coverage-positive. 9 refs of one family with a 9-slot envelope: the
        // first pass admits 8 (MAX_FAMILY_REF_SHARE) and caps the 9th
        // (recorded family_diversity_cap_exceeded), then the soft backfill tops
        // the packet up to 9 by reinstating the capped ref as a backfill pick.
        let mut memory_candidates = Vec::new();
        for index in 0u8..9u8 {
            memory_candidates.push(candidate(
                &mem_id(index),
                "summary",
                &format!("Coverage-zero ref {index}"),
                "actionable task memory in one family",
                &["04_Plans", "Next Steps"],
                10,
            ));
        }
        let state = GraphContextSelectionState {
            memory_candidates,
            graph_edges: Vec::new(),
            receipt_ids: Vec::new(),
        };
        // No identifier token in the task, so no ref is coverage-positive.
        let mut request = base_request("actionable task memory family");
        request.max_memory_refs = 9;
        request.token_budget = 100_000; // pragma-allowlist-secret
        let response = select_graph_context(&request, &state).expect("coverage-zero cap selection");
        // None of these refs earned coverage — the cap path applies unchanged.
        assert!(
            response
                .selected_memory_refs
                .iter()
                .all(|selected| !selected.selection_reason.contains("identifier_coverage")),
            "coverage-zero refs must not be tagged identifier_coverage"
        );
        // The 9th ref is capped on the first pass then reinstated as backfill,
        // so the envelope still fills to 9 with exactly one backfilled ref.
        assert_eq!(response.selected_memory_refs.len(), 9);
        let backfilled = response
            .selected_memory_refs
            .iter()
            .filter(|selected| selected.selection_reason == "family_diversity_backfill")
            .count();
        assert_eq!(
            backfilled, 1,
            "the capped 9th ref is reinstated via backfill"
        );
        // The family-cap omission was recorded for the 9th ref before backfill.
        // After backfill reinstates it, it is no longer omitted, so we instead
        // assert the cap mechanism produced exactly one backfill pick above.
        assert!(
            !response
                .omitted_memory_refs
                .iter()
                .any(|omitted| omitted.omission_reason == "family_diversity_cap_exceeded"),
            "the single capped ref was backfilled into the open slot"
        );
    }

    #[test]
    fn family_key_uses_top_two_segments_lowercased() {
        let three = candidate(
            &mem_id(0x01),
            "summary",
            "Alpha",
            "Alpha",
            &["04_Plans", "Next_Steps", "Deep"],
            10,
        );
        assert_eq!(family_key(&three), "04_plans/next_steps");
        let one = candidate(&mem_id(0x02), "summary", "Beta", "Beta", &["00_Index"], 10);
        assert_eq!(family_key(&one), "00_index");
        let none = candidate(&mem_id(0x03), "summary", "Gamma", "Gamma", &[], 10);
        assert_eq!(family_key(&none), "");
    }

    // --- PRD-D2 S4 roll-up scoring pure function -----------------------------

    #[test]
    fn rollup_score_uses_max_member_scaled_and_capped() {
        // MAX member, scaled by share bp: max(6000,1000)=6000 -> 6000*5000/10000.
        assert_eq!(rollup_score_for_root(&[6_000, 1_000]), 3_000);
        // Order does not matter (MAX is symmetric).
        assert_eq!(rollup_score_for_root(&[1_000, 6_000]), 3_000);
    }

    #[test]
    fn rollup_score_holds_the_ceiling() {
        // A huge member is capped at MAX_ROLLUP_SCORE_BP.
        assert_eq!(rollup_score_for_root(&[u32::MAX]), MAX_ROLLUP_SCORE_BP);
        assert_eq!(rollup_score_for_root(&[1_000_000]), MAX_ROLLUP_SCORE_BP);
    }

    #[test]
    fn rollup_score_cap_matches_python_benchmark_lane() {
        // PRD-D2 review F4: this Rust pure function must mirror the authoritative
        // Python measured-benchmark lane
        // (`dagdb_agent_brain_context_utility.py::aggregate_member_relevance_bp`,
        // ROLLUP_SCORING_SHARE_BP=5000, ROLLUP_SCORING_MAX_BONUS=4000). Pin both
        // the share and the cap so the two lanes can never silently diverge.
        assert_eq!(ROLLUP_SCORING_SHARE_BP, 5_000);
        assert_eq!(MAX_ROLLUP_SCORE_BP, 4_000);
        // Worked example below the cap: best=6000 -> 6000*5000/10000 = 3000.
        assert_eq!(rollup_score_for_root(&[6_000]), 3_000);
        // Worked example above the cap: best=10000 -> 5000, capped to 4000.
        assert_eq!(rollup_score_for_root(&[10_000]), 4_000);
    }

    #[test]
    fn rollup_score_zero_for_empty_or_nonpositive_members() {
        assert_eq!(rollup_score_for_root(&[]), 0);
        assert_eq!(rollup_score_for_root(&[0, 0]), 0);
    }

    #[test]
    fn rollup_score_never_outranks_strongest_member() {
        // The bonus is strictly below the strongest member score for any input
        // (share is half), so a root cannot leapfrog its own best member.
        for best in [1u32, 100, 1_000, 50_000, 199_999, 200_000] {
            let bonus = rollup_score_for_root(&[best]);
            assert!(bonus < best || best == 0, "bonus {bonus} >= best {best}");
        }
    }
}
