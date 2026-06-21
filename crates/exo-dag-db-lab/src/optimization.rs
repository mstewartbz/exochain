//! Deterministic Phase 2A optimization helpers for DAG DB graph reads.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::Hash256;
use exo_dag_db_api::{CanonicalizationDecision, GraphView, SafeMetadata, SafeMetadataDecision};
use serde_json::Value;

use crate::{
    benchmark::{
        BenchmarkFixture, BenchmarkRunnerName, EvidenceQualityBreakdown,
        audit_mvp_fixture_evidence_fields, run_benchmark_fixture,
    },
    diagnostics::LatencyBreakdown,
    graph::MemoryGraphEdge,
    metadata::{MetadataError, MetadataField, sanitize_runtime_metadata},
    model::{ContextPacket, RouteMemoryReceipt},
};

/// Redaction policy version used in deterministic cache keys.
pub(crate) const REDACTION_POLICY_VERSION: u16 = 1;

/// MVP locked redaction cache hit ratio for deterministic latency modeling.
pub const MVP_REDACTION_CACHE_HIT_RATIO_BP: u64 = 5_000;

/// Predicted optimized MVP metrics used by the floor feasibility check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictedOptimizedMetrics {
    pub quality_score_bp: u16,
    pub citation_accuracy_bp: u16,
    pub unsupported_claim_rate_bp: u16,
    pub prompt_tokens_total: u32,
    pub overhead_tokens_total: u32,
    pub net_savings_micro_exo_total: u64,
    pub deterministic_latency_ms_total: u64,
    pub claim_allowed: bool,
}

/// Floor feasibility failure with the locked output shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfeasibleFloor {
    pub metric_name: &'static str,
    pub predicted: u64,
    pub floor: u64,
}

impl std::fmt::Display for InfeasibleFloor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "INFEASIBLE_FLOOR: {} predicted={} floor={}",
            self.metric_name, self.predicted, self.floor
        )
    }
}

/// Predict MVP optimized metrics without invoking the optimized runner variant.
pub fn predict_optimized_metrics(
    fixture: &BenchmarkFixture,
) -> Result<PredictedOptimizedMetrics, InfeasibleFloor> {
    let metrics = predicted_metrics_unchecked(fixture)?;
    check_floor_ge(
        "quality_score_bp",
        u64::from(metrics.quality_score_bp),
        9_300,
        true,
    )?;
    check_floor_ge(
        "citation_accuracy_bp",
        u64::from(metrics.citation_accuracy_bp),
        9_850,
        true,
    )?;
    check_floor_le(
        "unsupported_claim_rate_bp",
        u64::from(metrics.unsupported_claim_rate_bp),
        60,
    )?;
    check_floor_le(
        "prompt_tokens_total",
        u64::from(metrics.prompt_tokens_total),
        520,
    )?;
    check_floor_le(
        "overhead_tokens_total",
        u64::from(metrics.overhead_tokens_total),
        360,
    )?;
    check_floor_ge(
        "net_savings_micro_exo_total",
        metrics.net_savings_micro_exo_total,
        3_300,
        false,
    )?;
    check_floor_le(
        "deterministic_latency_ms_total",
        metrics.deterministic_latency_ms_total,
        320,
    )?;
    if !metrics.claim_allowed {
        return Err(InfeasibleFloor {
            metric_name: "claim_allowed",
            predicted: 0,
            floor: 1,
        });
    }
    Ok(metrics)
}

/// Batch lookup request for graph, route, and context-packet references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBatchLookupRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub memory_ids: Vec<Hash256>,
    pub route_ids: Vec<Hash256>,
    pub context_packet_ids: Vec<Hash256>,
    pub view_ids: Vec<String>,
}

/// Tenant-scoped canonicalization record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalizationDecisionRecord {
    pub tenant_id: String,
    pub namespace: String,
    pub decision: CanonicalizationDecision,
}

/// Tenant-scoped graph view record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphViewRecord {
    pub tenant_id: String,
    pub namespace: String,
    pub view: GraphView,
}

/// Source-of-truth records for a pure batch lookup.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphBatchLookupSource {
    pub canonicalization_decisions: Vec<CanonicalizationDecisionRecord>,
    pub graph_views: Vec<GraphViewRecord>,
    pub edges: Vec<MemoryGraphEdge>,
    pub routes: Vec<RouteMemoryReceipt>,
    pub context_packets: Vec<ContextPacket>,
}

/// Batch lookup result for graph, route, and context-packet references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBatchLookupResult {
    pub tenant_id: String,
    pub namespace: String,
    pub canonical_by_memory_id: BTreeMap<String, String>,
    pub views_by_id: BTreeMap<String, GraphView>,
    pub edges_by_memory_id: BTreeMap<String, Vec<MemoryGraphEdge>>,
    pub routes_by_id: BTreeMap<String, RouteMemoryReceipt>,
    pub context_packets_by_id: BTreeMap<String, ContextPacket>,
    pub missing_ids: Vec<String>,
}

/// Return tenant-scoped graph records in one deterministic pass.
#[must_use]
pub fn batch_graph_lookup(
    request: &GraphBatchLookupRequest,
    source: &GraphBatchLookupSource,
) -> GraphBatchLookupResult {
    let mut result = GraphBatchLookupResult {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        canonical_by_memory_id: BTreeMap::new(),
        views_by_id: BTreeMap::new(),
        edges_by_memory_id: BTreeMap::new(),
        routes_by_id: BTreeMap::new(),
        context_packets_by_id: BTreeMap::new(),
        missing_ids: Vec::new(),
    };

    for memory_id in sorted_hashes(&request.memory_ids) {
        let memory_hex = memory_id.to_string();
        for record in &source.canonicalization_decisions {
            if record.decision.input_memory_id != memory_hex {
                continue;
            }
            if same_scope(&record.tenant_id, &record.namespace, request) {
                result.canonical_by_memory_id.insert(
                    memory_hex.clone(),
                    record
                        .decision
                        .canonical_memory_id
                        .clone()
                        .unwrap_or_else(|| memory_hex.clone()),
                );
            }
        }

        let matching_edges = source
            .edges
            .iter()
            .filter(|edge| edge.from_memory_id == memory_id)
            .collect::<Vec<_>>();
        let scoped_edges = matching_edges
            .iter()
            .filter(|edge| same_scope(&edge.tenant_id, &edge.namespace, request))
            .map(|edge| (*edge).clone())
            .collect::<Vec<_>>();
        if !scoped_edges.is_empty() {
            result
                .edges_by_memory_id
                .insert(memory_hex.clone(), scoped_edges);
        }

        if !result.canonical_by_memory_id.contains_key(&memory_hex) {
            push_missing(&mut result.missing_ids, memory_hex);
        }
    }

    for route_id in sorted_hashes(&request.route_ids) {
        let route_hex = route_id.to_string();
        let matching = source.routes.iter().find(|route| {
            route.route_id == route_id && same_scope(&route.tenant_id, &route.namespace, request)
        });
        if let Some(route) = matching {
            result.routes_by_id.insert(route_hex, route.clone());
        } else {
            push_missing(&mut result.missing_ids, route_hex);
        }
    }

    for context_packet_id in sorted_hashes(&request.context_packet_ids) {
        let packet_hex = context_packet_id.to_string();
        let matching = source.context_packets.iter().find(|packet| {
            packet.context_packet_id == context_packet_id
                && same_scope(&packet.tenant_id, &packet.namespace, request)
        });
        if let Some(packet) = matching {
            result
                .context_packets_by_id
                .insert(packet_hex, packet.clone());
        } else {
            push_missing(&mut result.missing_ids, packet_hex);
        }
    }

    let requested_views = request.view_ids.iter().cloned().collect::<BTreeSet<_>>();
    for view_id in &requested_views {
        let matching = source.graph_views.iter().find(|record| {
            record.view.view_id == *view_id
                && same_scope(&record.tenant_id, &record.namespace, request)
        });
        if let Some(record) = matching {
            result
                .views_by_id
                .insert(view_id.clone(), record.view.clone());
        } else {
            push_missing(&mut result.missing_ids, view_id.clone());
        }
    }

    result.missing_ids.sort();
    result.missing_ids.dedup();
    result
}

fn same_scope(tenant_id: &str, namespace: &str, request: &GraphBatchLookupRequest) -> bool {
    tenant_id == request.tenant_id && namespace == request.namespace
}

fn sorted_hashes(ids: &[Hash256]) -> Vec<Hash256> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted
}

fn push_missing(missing_ids: &mut Vec<String>, id: String) {
    if !missing_ids.iter().any(|existing| existing == &id) {
        missing_ids.push(id);
    }
}

/// Deterministic redaction cache key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionCacheKey {
    pub policy_version: u16,
    pub field: MetadataField,
    pub input_hash: String,
    pub byte_limit: u32,
}

impl PartialOrd for RedactionCacheKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RedactionCacheKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.policy_version,
            metadata_field_rank(self.field),
            &self.input_hash,
            self.byte_limit,
        )
            .cmp(&(
                other.policy_version,
                metadata_field_rank(other.field),
                &other.input_hash,
                other.byte_limit,
            ))
    }
}

const fn metadata_field_rank(field: MetadataField) -> u8 {
    match field {
        MetadataField::Title => 0,
        MetadataField::Summary => 1,
        MetadataField::Keyword => 2,
        MetadataField::ValidationNotes => 3,
        MetadataField::CouncilNotes => 4,
        MetadataField::ReceiptFreeText => 5,
        MetadataField::ResponseExcerpt => 6,
    }
}

impl RedactionCacheKey {
    /// Build a cache key from the original UTF-8 input bytes.
    #[must_use]
    pub fn new(field: MetadataField, input: &str, byte_limit: u32) -> Self {
        Self {
            policy_version: REDACTION_POLICY_VERSION,
            field,
            input_hash: Hash256::digest(input.as_bytes()).to_string(),
            byte_limit,
        }
    }
}

/// Safe redaction cache entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionCacheEntry {
    pub key: RedactionCacheKey,
    pub metadata: SafeMetadata,
    pub decision: SafeMetadataDecision,
}

/// Deterministic in-memory redaction decision cache.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RedactionDecisionCache {
    entries: BTreeMap<RedactionCacheKey, SafeMetadata>,
}

impl RedactionDecisionCache {
    /// Sanitize and cache only safe persisted metadata.
    pub fn get_or_sanitize(
        &mut self,
        field: MetadataField,
        input: &str,
        byte_limit: u32,
    ) -> Result<RedactionCacheEntry, MetadataError> {
        let key = RedactionCacheKey::new(field, input, byte_limit);
        if let Some(metadata) = self.entries.get(&key) {
            return Ok(RedactionCacheEntry {
                key,
                metadata: metadata.clone(),
                decision: metadata.decision,
            });
        }

        let metadata = sanitize_runtime_metadata(field, input)?;
        self.entries.insert(key.clone(), metadata.clone());
        Ok(RedactionCacheEntry {
            key,
            decision: metadata.decision,
            metadata,
        })
    }

    /// Return the number of cached safe entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true when no safe entries are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Idempotency read reuse key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdempotencyReadReuseKey {
    pub tenant_id: String,
    pub namespace: String,
    pub route_name: String,
    pub idempotency_key: String,
    pub request_hash: Hash256,
}

impl IdempotencyReadReuseKey {
    /// Build a key from canonical JSON request-body material only.
    pub fn new(
        tenant_id: String,
        namespace: String,
        route_name: String,
        idempotency_key: String,
        request_body: &Value,
    ) -> Self {
        Self {
            tenant_id,
            namespace,
            route_name,
            idempotency_key,
            request_hash: canonical_json_request_hash(request_body),
        }
    }

    fn same_replay_scope(&self, other: &Self) -> bool {
        self.tenant_id == other.tenant_id
            && self.namespace == other.namespace
            && self.route_name == other.route_name
            && self.idempotency_key == other.idempotency_key
    }
}

/// Cached idempotency read observed before a write path repeats work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyReadReuseRecord {
    pub key: IdempotencyReadReuseKey,
    pub status_code: u16,
    pub cached_failure: bool,
}

/// Pure idempotency read-reuse decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyReadReuseDecision {
    ReuseCachedSuccess,
    ReuseCachedDuplicateFailure,
    Conflict,
    NotCacheable,
}

/// Reuse an idempotency read without changing CAS/write behavior.
#[must_use]
pub fn reuse_idempotency_read(
    existing: Option<&IdempotencyReadReuseRecord>,
    incoming: &IdempotencyReadReuseKey,
) -> IdempotencyReadReuseDecision {
    let Some(existing) = existing else {
        return IdempotencyReadReuseDecision::NotCacheable;
    };
    if !existing.key.same_replay_scope(incoming) {
        return IdempotencyReadReuseDecision::NotCacheable;
    }
    if existing.key.request_hash != incoming.request_hash {
        return IdempotencyReadReuseDecision::Conflict;
    }
    if (200..300).contains(&existing.status_code) && !existing.cached_failure {
        return IdempotencyReadReuseDecision::ReuseCachedSuccess;
    }
    if existing.status_code == 409 && existing.cached_failure {
        return IdempotencyReadReuseDecision::ReuseCachedDuplicateFailure;
    }
    IdempotencyReadReuseDecision::NotCacheable
}

/// Hash canonical JSON request-body material.
#[must_use]
pub fn canonical_json_request_hash(request_body: &Value) -> Hash256 {
    Hash256::digest(canonical_json_bytes(request_body).as_bytes())
}

fn canonical_json_bytes(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into()),
        Value::Array(values) => {
            let inner = values
                .iter()
                .map(canonical_json_bytes)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{inner}]")
        }
        Value::Object(values) => {
            let sorted = values.iter().collect::<BTreeMap<_, _>>();
            let inner = sorted
                .iter()
                .map(|(key, value)| {
                    let encoded_key = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".into());
                    format!("{encoded_key}:{}", canonical_json_bytes(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

/// Context packet references before deterministic compaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactContextPacketInput {
    pub memory_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub hash_refs: Vec<String>,
    pub graph_view_ids: Vec<String>,
    pub validator_ids: Vec<String>,
}

/// Context packet references after deterministic compaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactContextPacketResult {
    pub memory_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub hash_refs: Vec<String>,
    pub graph_view_ids: Vec<String>,
    pub validator_ids: Vec<String>,
    pub removed_duplicate_ref_count: u32,
}

/// Deduplicate context packet references while preserving exact evidence IDs.
#[must_use]
pub fn compact_context_packet(input: &CompactContextPacketInput) -> CompactContextPacketResult {
    let memory_refs = sorted_unique(&input.memory_refs);
    let receipt_refs = sorted_unique(&input.receipt_refs);
    let hash_refs = sorted_unique(&input.hash_refs);
    let graph_view_ids = sorted_unique(&input.graph_view_ids);
    let validator_ids = sorted_unique(&input.validator_ids);
    let original_len = input
        .memory_refs
        .len()
        .saturating_add(input.receipt_refs.len())
        .saturating_add(input.hash_refs.len())
        .saturating_add(input.graph_view_ids.len())
        .saturating_add(input.validator_ids.len());
    let compact_len = memory_refs
        .len()
        .saturating_add(receipt_refs.len())
        .saturating_add(hash_refs.len())
        .saturating_add(graph_view_ids.len())
        .saturating_add(validator_ids.len());
    CompactContextPacketResult {
        memory_refs,
        receipt_refs,
        hash_refs,
        graph_view_ids,
        validator_ids,
        removed_duplicate_ref_count: u32::try_from(original_len.saturating_sub(compact_len))
            .unwrap_or(u32::MAX),
    }
}

fn sorted_unique(values: &[String]) -> Vec<String> {
    values
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn predicted_metrics_unchecked(
    fixture: &BenchmarkFixture,
) -> Result<PredictedOptimizedMetrics, InfeasibleFloor> {
    audit_mvp_fixture_evidence_fields(fixture).map_err(|_| InfeasibleFloor {
        metric_name: "mvp_evidence_fields",
        predicted: 0,
        floor: 1,
    })?;
    let optimized = run_benchmark_fixture(fixture, BenchmarkRunnerName::GovernedDagDbOptimized)
        .map_err(|_| InfeasibleFloor {
            metric_name: "optimized_prediction",
            predicted: 0,
            floor: 1,
        })?;
    let latency = optimized_fixture_latency(
        fixture,
        &optimized.selected_memory_ids_by_task,
        MVP_REDACTION_CACHE_HIT_RATIO_BP,
    );
    Ok(PredictedOptimizedMetrics {
        quality_score_bp: optimized.quality_score_bp,
        citation_accuracy_bp: optimized.citation_accuracy_bp,
        unsupported_claim_rate_bp: optimized.unsupported_claim_rate_bp,
        prompt_tokens_total: optimized.prompt_tokens,
        overhead_tokens_total: optimized.overhead_tokens,
        net_savings_micro_exo_total: optimized.net_savings_micro_exo,
        deterministic_latency_ms_total: latency.total_ms,
        claim_allowed: optimized.savings_claim_allowed,
    })
}

/// Compute optimized fixture-level latency from total selected refs.
#[must_use]
pub fn optimized_fixture_latency(
    fixture: &BenchmarkFixture,
    selected_by_task: &BTreeMap<String, Vec<String>>,
    redaction_cache_hit_ratio_bp: u64,
) -> LatencyBreakdown {
    let selected_ref_count = selected_by_task
        .values()
        .map(|ids| u64::try_from(ids.len()).unwrap_or(u64::MAX))
        .sum::<u64>();
    let route_count = selected_by_task
        .values()
        .filter(|ids| !ids.is_empty())
        .count()
        .try_into()
        .unwrap_or(u64::MAX);
    let context_packet_tokens = selected_ref_count.saturating_mul(6);
    let base = LatencyBreakdown::from_inputs(
        u64::try_from(fixture.corpus.len()).unwrap_or(u64::MAX),
        BenchmarkRunnerName::GovernedDagDbRouting,
        selected_ref_count,
        route_count,
        context_packet_tokens,
    );
    LatencyBreakdown::optimized_from_stage_inputs(
        base,
        selected_ref_count,
        context_packet_tokens,
        route_count,
        redaction_cache_hit_ratio_bp,
        true,
    )
}

/// Compute scale redaction cache hit ratio from deterministic duplicate cache keys.
#[must_use]
pub fn scale_redaction_cache_hit_ratio_bp(
    fixture: &BenchmarkFixture,
    selected_by_task: &BTreeMap<String, Vec<String>>,
) -> u64 {
    let mut seen = BTreeSet::new();
    let mut duplicate_count = 0u64;
    let mut selected_count = 0u64;
    let mut tasks = fixture
        .tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<Vec<_>>();
    tasks.sort();
    for task_id in tasks {
        if let Some(selected_refs) = selected_by_task.get(&task_id) {
            for memory_id in selected_refs {
                selected_count = selected_count.saturating_add(1);
                let key = metadata_cache_key_for_ref(fixture, memory_id);
                if !seen.insert(key) {
                    duplicate_count = duplicate_count.saturating_add(1);
                }
            }
        }
    }
    duplicate_count
        .saturating_mul(10_000)
        .checked_div(selected_count)
        .unwrap_or(0)
        .min(7_000)
}

/// Aggregate per-task evidence quality by arithmetic mean.
#[must_use]
pub fn aggregate_quality(rows: &[EvidenceQualityBreakdown]) -> EvidenceQualityBreakdown {
    let divisor = u32::try_from(rows.len()).unwrap_or(u32::MAX).max(1);
    let sum = |value: fn(&EvidenceQualityBreakdown) -> u16| -> u16 {
        u16::try_from(rows.iter().map(value).map(u32::from).sum::<u32>() / divisor)
            .unwrap_or(u16::MAX)
    };
    EvidenceQualityBreakdown {
        required_citation_recall_bp: sum(|row| row.required_citation_recall_bp),
        selected_ref_precision_bp: sum(|row| row.selected_ref_precision_bp),
        prohibited_ref_rejection_bp: sum(|row| row.prohibited_ref_rejection_bp),
        contradiction_exposure_bp: sum(|row| row.contradiction_exposure_bp),
        validation_pass_bp: sum(|row| row.validation_pass_bp),
        freshness_bp: sum(|row| row.freshness_bp),
        quality_score_bp: sum(|row| row.quality_score_bp),
        citation_accuracy_bp: sum(|row| row.citation_accuracy_bp),
        unsupported_claim_rate_bp: sum(|row| row.unsupported_claim_rate_bp),
        claim_allowed: rows.iter().all(|row| row.claim_allowed),
    }
}

fn metadata_cache_key_for_ref(fixture: &BenchmarkFixture, memory_id: &str) -> RedactionCacheKey {
    let input = fixture
        .corpus
        .iter()
        .find(|item| item.memory_id.as_deref().unwrap_or(&item.payload_hash) == memory_id)
        .map_or(memory_id, |item| item.summary_text.as_str());
    RedactionCacheKey::new(MetadataField::Summary, input, 1_000)
}

fn check_floor_ge(
    metric_name: &'static str,
    predicted: u64,
    floor: u64,
    basis_point_metric: bool,
) -> Result<(), InfeasibleFloor> {
    let margin = floor.saturating_mul(5) / 100;
    let threshold = if basis_point_metric {
        floor.saturating_add(margin).min(10_000)
    } else {
        floor.saturating_add(margin)
    };
    if predicted >= threshold {
        Ok(())
    } else {
        Err(InfeasibleFloor {
            metric_name,
            predicted,
            floor,
        })
    }
}

fn check_floor_le(
    metric_name: &'static str,
    predicted: u64,
    floor: u64,
) -> Result<(), InfeasibleFloor> {
    let threshold = floor.saturating_sub(floor.saturating_mul(5) / 100);
    if predicted <= threshold {
        Ok(())
    } else {
        Err(InfeasibleFloor {
            metric_name,
            predicted,
            floor,
        })
    }
}

#[cfg(test)]
mod tests {
    use exo_core::Timestamp;
    use exo_dag_db_api::{
        CanonicalizationDecisionKind, CouncilReviewStatus, DagFinalityStatus, GraphViewType,
        MemoryEdgeKind, MemoryGraphStyle, RouteStatus, SafeMetadataDecision, ValidationStatus,
    };

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn decision(
        input: Hash256,
        canonical: Hash256,
        tenant_id: &str,
    ) -> CanonicalizationDecisionRecord {
        CanonicalizationDecisionRecord {
            tenant_id: tenant_id.into(),
            namespace: "primary".into(),
            decision: CanonicalizationDecision {
                decision_id: h(0xa1).to_string(),
                input_memory_id: input.to_string(),
                canonical_memory_id: Some(canonical.to_string()),
                matched_memory_ids: vec![canonical.to_string()],
                decision_kind: CanonicalizationDecisionKind::NearDuplicate,
                decision_reason: "test".into(),
                confidence_bp: 9_000,
                risk_class: exo_dag_db_api::RiskClass::R1,
                validator_status: ValidationStatus::Passed,
                required_edges_to_create: Vec::new(),
                receipt_intent: "canonicalization_decided".into(),
                receipt_id: None,
            },
        }
    }

    fn route(id: Hash256, tenant_id: &str) -> RouteMemoryReceipt {
        RouteMemoryReceipt {
            route_id: id,
            tenant_id: tenant_id.into(),
            namespace: "primary".into(),
            requesting_agent_did: "did:exo:agent".into(),
            task_signature_hash: h(0x10),
            approved_scope_hash: h(0x11),
            candidate_memory_ids: vec![h(0x20)],
            selected_memory_ids: vec![h(0x20)],
            rejected_memory_ids: Vec::new(),
            route_score_bp: 8_800,
            token_budget: 4096,
            token_estimate: 256,
            risk_bp: 1000,
            status: RouteStatus::Active,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            stale_at: Timestamp::new(10_000, 0),
            latest_receipt_hash: h(0x12),
            created_at: Timestamp::new(1_000, 0),
            credential_id: None,
            validation_report_id: None,
            council_decision_id: None,
        }
    }

    fn packet(id: Hash256, tenant_id: &str) -> ContextPacket {
        ContextPacket {
            context_packet_id: id,
            tenant_id: tenant_id.into(),
            namespace: "primary".into(),
            request_id: "request-1".into(),
            route_id: h(0x30),
            task_hash: h(0x31),
            requesting_agent_did: "did:exo:agent".into(),
            memory_refs: vec![h(0x20)],
            packet_hash: h(0x32),
            token_budget: 4096,
            token_estimate: 256,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            latest_receipt_hash: h(0x33),
            created_at: Timestamp::new(1_000, 0),
            validation_report_id: None,
            council_decision_id: None,
        }
    }

    fn view(id: &str, tenant_id: &str) -> GraphViewRecord {
        GraphViewRecord {
            tenant_id: tenant_id.into(),
            namespace: "primary".into(),
            view: GraphView {
                view_id: id.into(),
                graph_style: MemoryGraphStyle::RoutingViewGraph,
                source_root_id: h(0x44).to_string(),
                included_node_ids: vec![h(0x20).to_string()],
                included_edge_ids: vec![h(0x45).to_string()],
                view_type: GraphViewType::RoutingView,
                topological_order: vec![h(0x20).to_string()],
                transitive_reduction_edges: Vec::new(),
                omitted_edges: Vec::new(),
                reason_edges_omitted: Vec::new(),
            },
        }
    }

    #[test]
    fn phase2a_batch_graph_lookup_preserves_tenant_isolation() {
        let memory_id = h(0x20);
        let route_id = h(0x30);
        let packet_id = h(0x40);
        let scoped_edge = MemoryGraphEdge::new(
            "tenant-a".into(),
            "primary".into(),
            memory_id,
            h(0x21),
            MemoryEdgeKind::DependsOn,
            MemoryGraphStyle::DependencyDag,
            None,
        )
        .expect("edge");
        let cross_edge = MemoryGraphEdge::new(
            "tenant-b".into(),
            "primary".into(),
            memory_id,
            h(0x22),
            MemoryEdgeKind::DependsOn,
            MemoryGraphStyle::DependencyDag,
            None,
        )
        .expect("edge");
        let source = GraphBatchLookupSource {
            canonicalization_decisions: vec![
                decision(memory_id, h(0x21), "tenant-a"),
                decision(h(0x23), h(0x24), "tenant-b"),
            ],
            graph_views: vec![view("view-a", "tenant-a"), view("view-b", "tenant-b")],
            edges: vec![scoped_edge, cross_edge],
            routes: vec![route(route_id, "tenant-a"), route(h(0x31), "tenant-b")],
            context_packets: vec![packet(packet_id, "tenant-a"), packet(h(0x41), "tenant-b")],
        };
        let request = GraphBatchLookupRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            memory_ids: vec![memory_id, h(0x23)],
            route_ids: vec![route_id, h(0x31)],
            context_packet_ids: vec![packet_id, h(0x41)],
            view_ids: vec!["view-a".into(), "view-b".into()],
        };

        let result = batch_graph_lookup(&request, &source);
        assert_eq!(
            result.canonical_by_memory_id.get(&memory_id.to_string()),
            Some(&h(0x21).to_string())
        );
        assert_eq!(result.edges_by_memory_id[&memory_id.to_string()].len(), 1);
        assert!(result.routes_by_id.contains_key(&route_id.to_string()));
        assert!(
            result
                .context_packets_by_id
                .contains_key(&packet_id.to_string())
        );
        assert!(result.views_by_id.contains_key("view-a"));
        assert!(!result.routes_by_id.contains_key(&h(0x31).to_string()));
        assert!(
            !result
                .context_packets_by_id
                .contains_key(&h(0x41).to_string())
        );
        assert!(!result.views_by_id.contains_key("view-b"));
        assert!(result.missing_ids.contains(&h(0x23).to_string()));
        assert!(result.missing_ids.contains(&h(0x31).to_string()));
        assert!(result.missing_ids.contains(&h(0x41).to_string()));
        assert!(result.missing_ids.contains(&"view-b".to_owned()));
    }

    #[test]
    fn phase2a_batch_lookup_branch_vectors_cover_missing_and_edge_only_paths() {
        let memory_id = h(0x50);
        let shared_missing = h(0x51);
        let edge_only = h(0x52);
        let scoped_edge = MemoryGraphEdge::new(
            "tenant-a".into(),
            "primary".into(),
            edge_only,
            h(0x53),
            MemoryEdgeKind::RelatedTo,
            MemoryGraphStyle::SimilarityOverlayGraph,
            None,
        )
        .expect("edge");
        let source = GraphBatchLookupSource {
            canonicalization_decisions: vec![
                CanonicalizationDecisionRecord {
                    tenant_id: "tenant-b".into(),
                    namespace: "primary".into(),
                    decision: decision(memory_id, h(0x54), "tenant-b").decision,
                },
                decision(h(0x55), h(0x56), "tenant-a"),
            ],
            edges: vec![scoped_edge],
            ..GraphBatchLookupSource::default()
        };
        let request = GraphBatchLookupRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            memory_ids: vec![memory_id, edge_only, shared_missing],
            route_ids: vec![shared_missing],
            context_packet_ids: vec![shared_missing],
            view_ids: vec!["missing-view".into()],
        };

        let result = batch_graph_lookup(&request, &source);
        assert!(
            result
                .edges_by_memory_id
                .contains_key(&edge_only.to_string())
        );
        assert!(
            !result
                .canonical_by_memory_id
                .contains_key(&memory_id.to_string())
        );
        assert!(result.missing_ids.contains(&memory_id.to_string()));
        assert!(result.missing_ids.contains(&edge_only.to_string()));
        assert_eq!(
            result
                .missing_ids
                .iter()
                .filter(|missing| *missing == &shared_missing.to_string())
                .count(),
            1
        );
        assert!(same_scope("tenant-a", "primary", &request));
        assert!(!same_scope("tenant-b", "primary", &request));
    }

    #[test]
    fn phase2a_redaction_cache_key_includes_policy_version_hash() {
        let first = RedactionCacheKey::new(MetadataField::Summary, "safe text", 1000);
        let trailing = RedactionCacheKey::new(MetadataField::Summary, "safe text ", 1000);
        let other_limit = RedactionCacheKey::new(MetadataField::Summary, "safe text", 512);

        assert_eq!(first.policy_version, REDACTION_POLICY_VERSION);
        assert_eq!(first.field, MetadataField::Summary);
        assert_eq!(first.byte_limit, 1000);
        assert_eq!(first.input_hash, Hash256::digest(b"safe text").to_string());
        assert_ne!(first.input_hash, trailing.input_hash);
        assert_ne!(first, other_limit);
    }

    #[test]
    fn phase2a_redaction_cache_never_stores_raw_sensitive_payload() {
        let mut cache = RedactionDecisionCache::default();
        let entry = cache
            .get_or_sanitize(MetadataField::Summary, "SSN 123-45-6789", 1000)
            .expect("redactable metadata");

        assert_eq!(entry.decision, SafeMetadataDecision::Redact);
        assert_eq!(cache.len(), 1);
        let debug = format!("{cache:?}");
        assert!(!debug.contains("123-45-6789"));
        assert!(debug.contains("REDACTED_SSN"));

        let rejected = cache.get_or_sanitize(MetadataField::Summary, "fn main() {}", 1000);
        assert!(matches!(rejected, Err(MetadataError::Rejected { .. })));
        assert_eq!(cache.len(), 1);

        let replay = cache
            .get_or_sanitize(MetadataField::Summary, "SSN 123-45-6789", 1000)
            .expect("cached metadata");
        assert_eq!(replay.metadata, entry.metadata);
    }

    #[test]
    fn phase2a_idempotency_read_reuse_preserves_conflict_behavior() {
        let first_body = serde_json::json!({"b": 2, "a": 1});
        let equal_body = serde_json::json!({"a": 1, "b": 2});
        let different_body = serde_json::json!({"a": 1, "b": 3});
        let existing_key = IdempotencyReadReuseKey::new(
            "tenant-a".into(),
            "primary".into(),
            "dagdb.intake".into(),
            "idem-1".into(),
            &first_body,
        );
        let equal_key = IdempotencyReadReuseKey::new(
            "tenant-a".into(),
            "primary".into(),
            "dagdb.intake".into(),
            "idem-1".into(),
            &equal_body,
        );
        let different_key = IdempotencyReadReuseKey::new(
            "tenant-a".into(),
            "primary".into(),
            "dagdb.intake".into(),
            "idem-1".into(),
            &different_body,
        );
        let record = IdempotencyReadReuseRecord {
            key: existing_key,
            status_code: 201,
            cached_failure: false,
        };

        assert_eq!(record.key.request_hash, equal_key.request_hash);
        assert_ne!(record.key.request_hash, different_key.request_hash);
        assert_eq!(
            reuse_idempotency_read(Some(&record), &equal_key),
            IdempotencyReadReuseDecision::ReuseCachedSuccess
        );
        assert_eq!(
            reuse_idempotency_read(Some(&record), &different_key),
            IdempotencyReadReuseDecision::Conflict
        );

        let duplicate = IdempotencyReadReuseRecord {
            status_code: 409,
            cached_failure: true,
            ..record
        };
        assert_eq!(
            reuse_idempotency_read(Some(&duplicate), &equal_key),
            IdempotencyReadReuseDecision::ReuseCachedDuplicateFailure
        );
        assert_eq!(
            reuse_idempotency_read(None, &equal_key),
            IdempotencyReadReuseDecision::NotCacheable
        );

        let different_scope = IdempotencyReadReuseKey::new(
            "tenant-b".into(),
            "primary".into(),
            "dagdb.intake".into(),
            "idem-1".into(),
            &equal_body,
        );
        assert_eq!(
            reuse_idempotency_read(Some(&duplicate), &different_scope),
            IdempotencyReadReuseDecision::NotCacheable
        );

        let success_but_failure_flag = IdempotencyReadReuseRecord {
            status_code: 201,
            cached_failure: true,
            ..duplicate.clone()
        };
        assert_eq!(
            reuse_idempotency_read(Some(&success_but_failure_flag), &equal_key),
            IdempotencyReadReuseDecision::NotCacheable
        );

        let conflict_without_failure_flag = IdempotencyReadReuseRecord {
            status_code: 409,
            cached_failure: false,
            ..duplicate
        };
        assert_eq!(
            reuse_idempotency_read(Some(&conflict_without_failure_flag), &equal_key),
            IdempotencyReadReuseDecision::NotCacheable
        );
    }

    #[test]
    fn phase2a_compact_context_packet_keeps_exact_evidence_ids() {
        let input = CompactContextPacketInput {
            memory_refs: vec!["mem-b".into(), "mem-a".into(), "mem-a".into()],
            receipt_refs: vec!["receipt-1".into(), "receipt-1".into()],
            hash_refs: vec!["hash-2".into(), "hash-1".into(), "hash-2".into()],
            graph_view_ids: vec!["view-1".into(), "view-1".into()],
            validator_ids: vec!["did:exo:v".into(), "did:exo:v".into()],
        };
        let compact = compact_context_packet(&input);
        assert_eq!(compact.memory_refs, vec!["mem-a", "mem-b"]);
        assert_eq!(compact.receipt_refs, vec!["receipt-1"]);
        assert_eq!(compact.hash_refs, vec!["hash-1", "hash-2"]);
        assert_eq!(compact.graph_view_ids, vec!["view-1"]);
        assert_eq!(compact.validator_ids, vec!["did:exo:v"]);
        assert_eq!(compact.removed_duplicate_ref_count, 5);
    }

    #[test]
    fn optimized_context_packet_reduces_repeated_refs() {
        let input = CompactContextPacketInput {
            memory_refs: vec!["m1".into(), "m1".into(), "m2".into()],
            receipt_refs: vec!["r1".into(), "r1".into()],
            hash_refs: vec!["h1".into(), "h1".into(), "h2".into()],
            graph_view_ids: vec!["g1".into(), "g1".into()],
            validator_ids: vec!["v1".into(), "v1".into()],
        };
        let compact = compact_context_packet(&input);
        assert_eq!(compact.memory_refs, vec!["m1", "m2"]);
        assert_eq!(compact.removed_duplicate_ref_count, 5);
    }

    #[test]
    fn phase2a_overhead_optimization_preserves_validation_council_risk() {
        let route = route(h(0x30), "tenant-a");
        let packet = packet(h(0x40), "tenant-a");
        let request = GraphBatchLookupRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            memory_ids: Vec::new(),
            route_ids: vec![route.route_id],
            context_packet_ids: vec![packet.context_packet_id],
            view_ids: Vec::new(),
        };
        let result = batch_graph_lookup(
            &request,
            &GraphBatchLookupSource {
                routes: vec![route],
                context_packets: vec![packet],
                ..GraphBatchLookupSource::default()
            },
        );
        let routed = result.routes_by_id.values().next().expect("route");
        let packet = result
            .context_packets_by_id
            .values()
            .next()
            .expect("packet");
        assert_eq!(routed.validation_status, ValidationStatus::Passed);
        assert_eq!(routed.council_status, CouncilReviewStatus::NotRequired);
        assert_eq!(routed.risk_bp, 1000);
        assert_eq!(packet.validation_status, ValidationStatus::Passed);
        assert_eq!(packet.dag_finality_status, DagFinalityStatus::Committed);
    }

    #[test]
    fn optimized_floor_feasibility_check() {
        let fixture = crate::benchmark::load_benchmark_fixture_json(include_str!(
            "../fixtures/benchmarks/mvp_minimum.json"
        ))
        .expect("fixture");
        let predicted = predict_optimized_metrics(&fixture).expect("feasible floors");
        assert!(predicted.quality_score_bp >= 9_300);
        assert!(predicted.citation_accuracy_bp >= 9_850);
        assert!(predicted.unsupported_claim_rate_bp <= 60);
        assert!(predicted.prompt_tokens_total <= 520);
        assert!(predicted.overhead_tokens_total <= 360);
        assert!(predicted.net_savings_micro_exo_total >= 3_300);
        assert!(predicted.deterministic_latency_ms_total <= 320);
        assert!(predicted.claim_allowed);
    }

    #[test]
    fn optimized_redaction_cache_hit_ratio_is_fixture_derived() {
        let fixture = crate::benchmark::load_benchmark_fixture_json(include_str!(
            "../fixtures/benchmarks/mvp_minimum.json"
        ))
        .expect("fixture");
        let report = crate::benchmark::run_benchmark_fixture(
            &fixture,
            crate::benchmark::BenchmarkRunnerName::GovernedDagDbOptimized,
        )
        .expect("optimized");
        assert_eq!(MVP_REDACTION_CACHE_HIT_RATIO_BP, 5_000);
        let first =
            scale_redaction_cache_hit_ratio_bp(&fixture, &report.selected_memory_ids_by_task);
        let second =
            scale_redaction_cache_hit_ratio_bp(&fixture, &report.selected_memory_ids_by_task);
        assert_eq!(first, second);
    }

    #[test]
    fn scale_redaction_cache_hit_ratio_uses_locked_definition() {
        let fixture = crate::benchmark::generate_scale_fixture();
        let mut selected = BTreeMap::new();
        let first = fixture.tasks[0].allowed_memory_ids[0].clone();
        selected.insert(fixture.tasks[0].task_id.clone(), vec![first.clone()]);
        selected.insert(fixture.tasks[1].task_id.clone(), vec![first]);
        assert_eq!(
            scale_redaction_cache_hit_ratio_bp(&fixture, &selected),
            5_000
        );
        assert_eq!(
            scale_redaction_cache_hit_ratio_bp(&fixture, &BTreeMap::new()),
            0
        );
    }
}
