//! ExoChain DAG DB API DTOs.
//!
//! `exo-dag-db-api` owns these wire shapes. `exo-api::dagdb` re-exports them
//! as the stable compatibility path for gateway, node, SDK, and external
//! callers.
//!
//! Every consumer-facing REST **response** DTO carries a stable
//! `schema_version` string constant so a non-Rust integrator can detect the
//! wire-contract version directly from a response body. The constants below are
//! the single source of truth; the checked-in machine contract under
//! `docs/dagdb/api/` and the fixture round-trip tests are asserted against them.

#![cfg_attr(test, allow(clippy::expect_used))]

use serde::{Deserialize, Serialize};

/// Schema version emitted on every `DagDbIntakeResponse`.
pub const DAGDB_INTAKE_RESPONSE_SCHEMA_VERSION: &str = "dagdb_intake_response_v1";
/// Schema version emitted on every `DagDbRouteResponse`.
pub const DAGDB_ROUTE_RESPONSE_SCHEMA_VERSION: &str = "dagdb_route_response_v1";
/// Schema version emitted on every `DagDbContextPacketResponse`.
pub const DAGDB_CONTEXT_PACKET_RESPONSE_SCHEMA_VERSION: &str = "dagdb_context_packet_response_v1";
/// Schema version emitted on every `DagDbValidateResponse`.
pub const DAGDB_VALIDATE_RESPONSE_SCHEMA_VERSION: &str = "dagdb_validate_response_v1";
/// Schema version emitted on every `DagDbWritebackResponse`.
pub const DAGDB_WRITEBACK_RESPONSE_SCHEMA_VERSION: &str = "dagdb_writeback_response_v1";
/// Schema version emitted on every `DagDbImportResponse`.
pub const DAGDB_IMPORT_RESPONSE_SCHEMA_VERSION: &str = "dagdb_import_response_v1";
/// Schema version emitted on every `DagDbExportResponse`.
pub const DAGDB_EXPORT_RESPONSE_SCHEMA_VERSION: &str = "dagdb_export_response_v1";
/// Schema version emitted on every `DagDbTrustCheckResponse`.
pub const DAGDB_TRUST_CHECK_RESPONSE_SCHEMA_VERSION: &str = "dagdb_trust_check_response_v1";
/// Schema version emitted on every `DagDbCouncilDecisionResponse`.
pub const DAGDB_COUNCIL_DECISION_RESPONSE_SCHEMA_VERSION: &str =
    "dagdb_council_decision_response_v1";
/// Schema version emitted on every `DagDbReceiptLookupResponse`.
pub const DAGDB_RECEIPT_LOOKUP_RESPONSE_SCHEMA_VERSION: &str = "dagdb_receipt_lookup_response_v1";
/// Schema version emitted on every `DagDbCatalogLookupResponse`.
pub const DAGDB_CATALOG_LOOKUP_RESPONSE_SCHEMA_VERSION: &str = "dagdb_catalog_lookup_response_v1";
/// Schema version emitted on every `DagDbRouteLookupResponse`.
pub const DAGDB_ROUTE_LOOKUP_RESPONSE_SCHEMA_VERSION: &str = "dagdb_route_lookup_response_v1";

/// Safe metadata decision recorded after server-side sanitization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeMetadataDecision {
    /// Metadata is safe and stored as-is within the configured length bound.
    Allow,
    /// Metadata was deterministically redacted before storage or response.
    Redact,
    /// Metadata must not persist.
    Reject,
}

/// Deterministic metadata redaction code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionCode {
    /// US Social Security number marker.
    Ssn,
    /// Payment card number marker.
    Card,
    /// NDA or confidential marker.
    ConfidentialMarker,
    /// Protected health information marker.
    Phi,
    /// Private customer marker.
    CustomerPrivate,
    /// Raw source code excerpt marker.
    CodeExcerpt,
    /// Length truncation marker.
    LengthTruncation,
}

/// Trusted stored metadata shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeMetadata {
    pub decision: SafeMetadataDecision,
    pub text: String,
    pub redaction_codes: Vec<RedactionCode>,
    pub original_hash: String,
    pub truncated: bool,
    pub byte_len: u32,
}

/// Memory node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryNodeType {
    Source,
    Excerpt,
    Embedding,
    Summary,
    Answer,
    ValidationReport,
    Catalog,
    Route,
    ContextPacket,
}

/// Inbound memory source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    PublicWeb,
    PrivateCustomer,
    IpSensitive,
    Generated,
    OpenSource,
    UnknownProvenance,
    BenchmarkFixture,
}

/// Consent purpose for the requested DAG DB action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentPurpose {
    Retrieval,
    Validation,
    Writeback,
    Benchmark,
    TrustCheck,
}

/// Memory status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Pending,
    Approved,
    Routable,
    Blocked,
    Revoked,
    Superseded,
    Rejected,
}

/// Validation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    NotRequired,
    Pending,
    Passed,
    Failed,
    Contradictory,
    Expired,
    NeedsCouncil,
}

/// Route status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStatus {
    Pending,
    Active,
    Stale,
    Invalidated,
    Blocked,
}

/// Council review status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilReviewStatus {
    NotRequired,
    Required,
    Pending,
    Approved,
    Denied,
    Expired,
    Escalated,
}

/// Durable council decision status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilDecisionStatus {
    Approved,
    Denied,
    Expired,
    Escalated,
    Revoked,
}

/// DAG finality status for route/context eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DagFinalityStatus {
    Pending,
    Committed,
    Failed,
    Compensated,
}

/// Risk class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
}

/// DAG DB subject kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Memory,
    Catalog,
    Route,
    ContextPacket,
    ValidationReport,
    AgentSafetyScore,
    InboundAgentCredential,
    CouncilDecision,
}

/// Receipt event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptEventType {
    IntakeCreated,
    DuplicateRejected,
    ValidationCreated,
    ValidationPassed,
    ValidationFailed,
    MemoryApproved,
    MemoryRoutable,
    MemoryRevoked,
    MemorySuperseded,
    RouteCreated,
    RouteActivated,
    RouteStale,
    RouteInvalidated,
    ContextPacketCreated,
    WritebackCreated,
    TrustCheckCreated,
    CouncilDecisionRecorded,
    DagFinalityCommitted,
    DagFinalityFailed,
    DagFinalityCompensated,
}

/// Decision source for a durable council decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    Human,
    Council,
    Policy,
}

/// Memory edge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEdgeType {
    Parent,
    DerivedFrom,
    Cites,
    Contradicts,
    Supersedes,
    Validates,
}

/// Validation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationDecision {
    Allow,
    Block,
    NeedsCouncil,
    Invalidate,
    Revoke,
    Supersede,
}

/// Credential status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatus {
    Pending,
    Active,
    Expired,
    Revoked,
    Blocked,
}

/// Memory graph style names used by the graph-organization layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryGraphStyle {
    ProvenanceReceiptDag,
    CanonicalMemoryGraph,
    SemanticCatalogGraph,
    SimilarityOverlayGraph,
    DependencyDag,
    RoutingViewGraph,
    ContradictionSupersessionGraph,
    ContextPacketGraph,
}

/// Memory graph node taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryNodeKind {
    Raw,
    Chunk,
    Summary,
    Concept,
    Canonical,
    DuplicateReference,
    Related,
    Replacement,
    Contradiction,
    Supersession,
    AlternateSummary,
    Decision,
    Route,
    ValidationReport,
    SavingsReport,
}

/// Memory graph edge taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEdgeKind {
    DerivedFrom,
    Summarizes,
    Supports,
    Contradicts,
    Supersedes,
    Replaces,
    DuplicateOf,
    NearDuplicateOf,
    RelatedTo,
    AlternativeSummaryOf,
    DependsOn,
    PartOf,
    OwnedBy,
    AccessGrantedBy,
    VerifiedBy,
    UsedByRoute,
    IncludedInContextPacket,
    RevokedBy,
}

/// Compact memory-candidate kind emitted after task completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCandidateKind {
    Decision,
    Summary,
    Plan,
    Schema,
    RouteFeedback,
    Contradiction,
    Preference,
    SavingsObservation,
}

/// Allowed future use for a compact memory candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCandidateUse {
    Inference,
    Routing,
    Audit,
}

/// Compact system-side writeback signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryCandidate {
    #[serde(rename = "type")]
    pub candidate_type: String,
    pub source_request_id: String,
    pub candidate_kind: MemoryCandidateKind,
    pub summary: String,
    pub full_output_hash: String,
    pub parent_context_packet_id: String,
    pub evidence_receipts: Vec<String>,
    pub risk_hint: RiskClass,
    pub allowed_future_uses: Vec<MemoryCandidateUse>,
    pub reason_to_remember: String,
}

/// Similarity class found before canonicalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityType {
    ExactHash,
    NearDuplicate,
    ConceptOverlap,
    WeakRelated,
}

/// Similarity result for one candidate-memory comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimilarityResult {
    pub candidate_memory_id: String,
    pub similarity_type: SimilarityType,
    pub similarity_bp: u16,
    pub matched_fields: Vec<String>,
    pub reason: String,
}

/// Canonicalization decision values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalizationDecisionKind {
    NewCanonical,
    ExactDuplicate,
    NearDuplicate,
    Related,
    Replacement,
    Contradiction,
    Supersession,
    AlternateSummary,
    RejectedNeedsReview,
}

/// Edge the canonicalization decision requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphEdgeRef {
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub edge_kind: MemoryEdgeKind,
}

/// Explicit canonicalization decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalizationDecision {
    pub decision_id: String,
    pub input_memory_id: String,
    pub canonical_memory_id: Option<String>,
    pub matched_memory_ids: Vec<String>,
    pub decision_kind: CanonicalizationDecisionKind,
    pub decision_reason: String,
    pub confidence_bp: u16,
    pub risk_class: RiskClass,
    pub validator_status: ValidationStatus,
    pub required_edges_to_create: Vec<GraphEdgeRef>,
    pub receipt_intent: String,
    pub receipt_id: Option<String>,
}

/// Rebuildable graph view kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphViewType {
    FullProvenance,
    RoutingView,
    CanonicalView,
    DependencyView,
    ContradictionView,
    ContextPacketView,
}

/// Rebuildable graph view artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphView {
    pub view_id: String,
    pub graph_style: MemoryGraphStyle,
    pub source_root_id: String,
    pub included_node_ids: Vec<String>,
    pub included_edge_ids: Vec<String>,
    pub view_type: GraphViewType,
    pub topological_order: Vec<String>,
    pub transitive_reduction_edges: Vec<GraphEdgeRef>,
    pub omitted_edges: Vec<GraphEdgeRef>,
    pub reason_edges_omitted: Vec<String>,
}

/// Route invalidation trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteInvalidationTrigger {
    Revoked,
    Superseded,
    Contradicted,
    Replaced,
    PermissionChanged,
    RiskChanged,
}

/// Route invalidation target status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteInvalidationStatus {
    Stale,
    Invalidated,
    NeedsReview,
    Superseded,
}

/// Governed route invalidation event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteInvalidationReceipt {
    pub route_id: String,
    pub affected_memory_ids: Vec<String>,
    pub trigger_type: RouteInvalidationTrigger,
    pub triggering_receipt_id: String,
    pub prior_route_status: RouteStatus,
    pub new_route_status: RouteInvalidationStatus,
    pub invalidation_reason: String,
    pub created_at: String,
    pub validator_id: Option<String>,
    pub validation_report_id: Option<String>,
    pub receipt_intent: String,
    pub receipt_id: Option<String>,
}

/// Placement result returned by the graph organization layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementResult {
    pub input_memory_id: String,
    pub canonicalization_decision: CanonicalizationDecision,
    pub similarity_results: Vec<SimilarityResult>,
    pub proposed_canonical_node: Option<String>,
    pub edges_to_create: Vec<GraphEdgeRef>,
    pub catalog_updates: Vec<String>,
    pub graph_views_to_refresh: Vec<MemoryGraphStyle>,
    pub route_invalidations: Vec<RouteInvalidationReceipt>,
    pub validator_report: String,
    pub receipt_id: Option<String>,
    pub receipt_intent: Option<String>,
}

/// Shared error envelope for every DAG DB route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbErrorEnvelope {
    pub error_code: String,
    pub message: String,
    pub receipt_hash: Option<String>,
    pub validation_report_id: Option<String>,
    pub requires_council_review: bool,
}

/// Intake request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbIntakeRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub source_type: SourceType,
    pub source_hash: String,
    pub payload_hash: String,
    pub owner_did: String,
    pub controller_did: String,
    pub submitted_by_did: String,
    pub consent_purpose: ConsentPurpose,
    pub requested_action: String,
    pub title_text: String,
    pub summary_text: String,
    pub payload_uri_hash: Option<String>,
    pub parent_memory_ids: Option<Vec<String>>,
    pub edge_types: Option<Vec<MemoryEdgeType>>,
    pub access_policy_hash: Option<String>,
    pub declared_rights_hash: Option<String>,
    pub keyword_texts: Option<Vec<String>>,
}

/// Route request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbRouteRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub requesting_agent_did: String,
    pub task_signature_hash: String,
    pub approved_scope_hash: String,
    pub token_budget: u32,
    pub start_catalog_id: Option<String>,
    pub requested_memory_ids: Option<Vec<String>>,
    pub credential_id: Option<String>,
}

/// Context-packet request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbContextPacketRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub request_id: String,
    pub route_id: String,
    pub task_hash: String,
    pub requesting_agent_did: String,
    pub token_budget: u32,
    pub force_revalidate: Option<bool>,
    pub max_memory_refs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layered_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_layer_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_layer_evidence: Option<bool>,
    /// Depth-on-demand reserve, in basis points of the token budget (D1-S4).
    /// When set with a non-off `layered_mode`, the breadth pass runs at the
    /// reserved (reduced) budget so membership-triggered drilldown has room to
    /// spend depth-on-demand up to the full budget. Absent / `0` is byte-
    /// identical to the prior leftover-budget behavior; the off-mode packet is
    /// unaffected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drilldown_reserve_bp: Option<u32>,
}

/// Validation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbValidateRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub subject_kind: SubjectKind,
    pub subject_id: String,
    pub validator_did: String,
    pub requested_status: Option<ValidationStatus>,
    pub council_decision_id: Option<String>,
    pub validation_notes_text: Option<String>,
}

/// Writeback request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbWritebackRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub requesting_agent_did: String,
    pub parent_memory_ids: Vec<String>,
    pub answer_hash: String,
    pub route_id: String,
    pub context_packet_id: String,
    pub validation_report_id: String,
    pub summary_text: Option<String>,
    pub citation_hashes: Option<Vec<String>>,
    pub safety_score_id: Option<String>,
    pub keyword_texts: Option<Vec<String>>,
    /// Optional typed-knowledge class (`decision`, `finding`, `fix`,
    /// `constraint`, `handoff`). When absent the writeback is plain
    /// usage-event telemetry and every existing client/signature path is
    /// unchanged. When present it describes WHAT the memory is for later
    /// recall; it never influences deterministic placement/organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layered_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_layer_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_layer_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_layer_reason: Option<String>,
}

/// Runtime import request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbImportRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub db_set_version: String,
    pub source_hash: String,
    pub requester_did: String,
    pub import_report: serde_json::Value,
}

/// Runtime export request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbExportRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub db_set_version: String,
    pub requester_did: String,
    pub included_memory_ids: Vec<String>,
    pub included_graph_styles: Vec<String>,
    pub included_writeback_idempotency_keys: Vec<String>,
    pub source_commit_or_repo_ref: Option<String>,
    pub include_preview_context: bool,
}

/// Trust-check request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbTrustCheckRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub agent_did: String,
    pub operator_did: String,
    pub model_name: String,
    pub model_version: String,
    pub provider_or_builder: String,
    pub requested_action: String,
    pub requested_scope_hash: String,
    pub purpose: ConsentPurpose,
    pub autonomy_level: String,
    pub nonce: String,
    pub expires_at: String,
    pub signature: String,
    pub checkpoint_hash: Option<String>,
    pub attestation_hash: Option<String>,
    pub evidence_receipt_hashes: Option<Vec<String>>,
    pub prior_trust_receipt_hash: Option<String>,
}

/// Council decision request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbCouncilDecisionRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub subject_kind: SubjectKind,
    pub subject_id: String,
    pub requested_action: String,
    pub approved_scope_hash: String,
    pub risk_class: RiskClass,
    pub approver_did: String,
    pub decision_source: DecisionSource,
    pub decision_status: CouncilDecisionStatus,
    pub reason_code: String,
    pub created_at: String,
    pub expires_at: String,
    pub validation_report_id: Option<String>,
    pub route_id: Option<String>,
    pub context_packet_id: Option<String>,
    pub notes_text: Option<String>,
}

/// Receipt lookup request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbReceiptLookupRequest {
    pub receipt_hash: String,
    pub tenant_id: String,
    pub namespace: String,
    pub include_body: Option<bool>,
}

/// Catalog lookup request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbCatalogLookupRequest {
    pub catalog_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub include_children: Option<bool>,
    pub include_routes: Option<bool>,
}

/// Route lookup request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbRouteLookupRequest {
    pub route_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub include_memory_refs: Option<bool>,
    pub include_validation: Option<bool>,
}

/// Intake response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbIntakeResponse {
    /// Stable wire-contract version (`dagdb_intake_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub memory_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub risk_class: RiskClass,
    pub risk_bp: u16,
    pub created_new: bool,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub keywords: Vec<SafeMetadata>,
    pub validation_report_id: Option<String>,
    pub council_decision_id: Option<String>,
    pub duplicate_of_memory_id: Option<String>,
}

/// Route response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbRouteResponse {
    /// Stable wire-contract version (`dagdb_route_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub route_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub route_status: RouteStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub selected_memory_ids: Vec<String>,
    pub route_score_bp: u16,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub stale_at: String,
    pub created_new: bool,
    pub validation_report_id: Option<String>,
    pub council_decision_id: Option<String>,
    pub rejected_memory_ids: Option<Vec<String>>,
}

/// Context packet memory reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketMemoryRef {
    pub memory_id: String,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub keywords: Vec<SafeMetadata>,
    pub latest_receipt_hash: String,
}

/// Context packet layered graph reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketLayerRef {
    pub layer_id: String,
    pub layer_path: String,
    pub layer_depth: u32,
    pub layer_kind: String,
    pub selected_ref_count: u32,
}

/// Context packet layered graph edge reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketLayerEdgeRef {
    pub layer_edge_id: String,
    pub from_layer_id: String,
    pub to_layer_id: String,
    pub edge_kind: String,
}

/// Context packet layered budget report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketLayerBudgetReport {
    pub layered_mode: String,
    pub max_layer_depth: u32,
    pub required_layer_evidence: bool,
    pub budget_status: String,
}

/// Context packet response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbContextPacketResponse {
    /// Stable wire-contract version (`dagdb_context_packet_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub context_packet_id: String,
    pub route_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub memory_refs: Vec<ContextPacketMemoryRef>,
    pub packet_hash: String,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub created_new: bool,
    pub validation_report_id: Option<String>,
    pub council_decision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_packet_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layered_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_layers: Option<Vec<ContextPacketLayerRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_layer_edges: Option<Vec<ContextPacketLayerEdgeRef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_budget_report: Option<ContextPacketLayerBudgetReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flat_fallback_used: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layered_status: Option<String>,
    /// Selected graph edges surfaced from the internal context packet. Empty on
    /// the no-database scaffold path; populated on the persistent (governed) path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_graph_edges: Vec<DagDbSelectedGraphEdgeRef>,
    /// Citation references surfaced from the internal context packet. Empty on
    /// the no-database scaffold path; populated on the persistent (governed) path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citation_refs: Vec<DagDbContextPacketCitationRef>,
    /// Packet metrics surfaced from the internal context packet, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_metrics: Option<DagDbContextPacketMetrics>,
    /// Blocked-claim boundaries surfaced from the internal context packet, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<DagDbContextPacketBoundaries>,
    /// Rendered agent-facing markdown surfaced from the internal context packet, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_markdown: Option<String>,
}

/// Validate response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbValidateResponse {
    /// Stable wire-contract version (`dagdb_validate_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub validation_report_id: String,
    pub subject_kind: SubjectKind,
    pub subject_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub risk_class: RiskClass,
    pub risk_bp: u16,
    pub decision: ValidationDecision,
    pub created_new: bool,
    pub council_decision_id: Option<String>,
    pub contradictory_report_ids: Option<Vec<String>>,
    pub notes: Option<SafeMetadata>,
}

/// Writeback response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbWritebackResponse {
    /// Stable wire-contract version (`dagdb_writeback_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub memory_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub risk_class: RiskClass,
    pub risk_bp: u16,
    pub created_new: bool,
    pub validation_report_id: Option<String>,
    pub council_decision_id: Option<String>,
    pub summary: Option<SafeMetadata>,
    pub keywords: Option<Vec<SafeMetadata>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_layer_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_layer_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_layer_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_child_layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layered_writeback_status: Option<String>,
}

/// Runtime import response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbImportResponse {
    /// Stable wire-contract version (`dagdb_import_response_v1`).
    pub schema_version: String,
    pub operation_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub db_set_version: String,
    pub import_status: String,
    pub import_receipt_id: Option<String>,
    pub source_hash: String,
    pub imported_record_count: u32,
    pub receipt_path: Option<String>,
    pub non_claims: Vec<String>,
}

/// Runtime export response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbExportResponse {
    /// Stable wire-contract version (`dagdb_export_response_v1`).
    pub schema_version: String,
    pub operation_id: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub db_set_version: String,
    pub export_status: String,
    pub export_artifact_id: Option<String>,
    pub export_hash: Option<String>,
    pub exported_record_count: u32,
    pub report_path: Option<String>,
    pub non_claims: Vec<String>,
}

/// Trust-check response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbTrustCheckResponse {
    /// Stable wire-contract version (`dagdb_trust_check_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub credential_id: String,
    pub safety_score_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub credential_status: CredentialStatus,
    pub total_score_bp: u16,
    pub created_new: bool,
    pub block_reason: Option<String>,
    pub expires_at: Option<String>,
}

/// Council decision response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbCouncilDecisionResponse {
    /// Stable wire-contract version (`dagdb_council_decision_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub idempotency_key: String,
    pub decision_id: String,
    pub subject_kind: SubjectKind,
    pub subject_id: String,
    pub receipt_hash: String,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub decision_status: CouncilDecisionStatus,
    pub approved_scope_hash: String,
    pub risk_class: RiskClass,
    pub expires_at: String,
    pub created_new: bool,
    pub validation_report_id: Option<String>,
    pub route_id: Option<String>,
    pub context_packet_id: Option<String>,
    pub notes: Option<SafeMetadata>,
}

/// Receipt lookup response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbReceiptLookupResponse {
    /// Stable wire-contract version (`dagdb_receipt_lookup_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub receipt_hash: String,
    pub subject_kind: SubjectKind,
    pub subject_id: String,
    pub prev_receipt_hash: String,
    pub seq: u64,
    pub event_type: ReceiptEventType,
    pub actor_did: String,
    pub event_hlc: String,
    pub created_at: String,
    pub receipt_body: Option<serde_json::Value>,
    pub validation_report_id: Option<String>,
}

/// Child catalog entry returned by lookup responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntryResponse {
    pub catalog_id: String,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
}

/// Catalog lookup response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbCatalogLookupResponse {
    /// Stable wire-contract version (`dagdb_catalog_lookup_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub catalog_id: String,
    pub catalog_level: u32,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub keywords: Vec<SafeMetadata>,
    pub status: MemoryStatus,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub latest_receipt_hash: String,
    pub memory_id: Option<String>,
    pub parent_catalog_id: Option<String>,
    pub children: Option<Vec<CatalogEntryResponse>>,
    pub routes: Option<Vec<String>>,
}

/// Graph context selection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DagDbGraphContextSelectionStatus {
    Selected,
    Empty,
    Failed,
}

/// Graph context selection request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbGraphContextSelectionRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task: String,
    pub task_hash: String,
    pub token_budget: u32,
    pub max_memory_refs: u32,
    pub catalog_hints: Vec<String>,
    pub requested_memory_ids: Vec<String>,
    pub force_revalidate: bool,
}

/// Selected memory reference returned by graph context selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbSelectedContextRef {
    pub memory_id: String,
    pub catalog_id: Option<String>,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub catalog_path: Vec<String>,
    pub document_type: String,
    pub selection_reason: String,
    pub token_estimate: u32,
    pub validation_status: ValidationStatus,
    pub citation_ref: String,
    pub boundary_flags: Vec<String>,
}

/// Selected graph edge reference returned by graph context selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbSelectedGraphEdgeRef {
    pub graph_edge_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub edge_kind: MemoryEdgeKind,
    pub graph_style: MemoryGraphStyle,
    pub selection_reason: String,
}

/// Omitted memory reference returned by graph context selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbOmittedContextRef {
    pub memory_id: String,
    pub omission_reason: String,
    pub token_estimate_if_selected: u32,
}

/// Route explanation step for graph context selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbGraphSelectionTraceStep {
    pub graph_style: MemoryGraphStyle,
    pub candidate_count_before: u32,
    pub candidate_count_after: u32,
    pub selected_count_after: u32,
    pub reason: String,
}

/// Graph context selection response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbGraphContextSelectionResponse {
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task_hash: String,
    pub selection_status: DagDbGraphContextSelectionStatus,
    pub selected_memory_refs: Vec<DagDbSelectedContextRef>,
    pub selected_graph_edges: Vec<DagDbSelectedGraphEdgeRef>,
    pub omitted_memory_refs: Vec<DagDbOmittedContextRef>,
    pub selection_trace: Vec<DagDbGraphSelectionTraceStep>,
    pub selected_token_estimate: u32,
    pub token_budget: u32,
    pub boundary_warnings: Vec<String>,
}

/// Graph context packet build request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbGraphContextPacketBuildRequest {
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task: String,
    pub task_hash: String,
    pub audit_id: String,
    pub token_budget: u32,
    pub selection: DagDbGraphContextSelectionResponse,
    pub import_tracking_status: Option<DagDbContextPacketImportTrackingStatus>,
}

/// Bounded graph context packet emitted by the Rust packet builder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbGraphContextPacket {
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task: String,
    pub task_hash: String,
    pub packet_hash: String,
    pub selected_memory_refs: Vec<DagDbSelectedContextRef>,
    pub selected_graph_edges: Vec<DagDbSelectedGraphEdgeRef>,
    pub citation_refs: Vec<DagDbContextPacketCitationRef>,
    pub packet_metrics: DagDbContextPacketMetrics,
    pub boundaries: DagDbContextPacketBoundaries,
    pub agent_usage_instructions: Vec<String>,
    pub markdown: String,
}

/// Citation reference included in a graph context packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbContextPacketCitationRef {
    pub citation_ref: String,
    pub memory_id: String,
    pub citation_status: String,
}

/// Metrics recorded for a graph context packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbContextPacketMetrics {
    pub token_budget: u32,
    pub selected_token_estimate: u32,
    pub selected_memory_ref_count: u32,
    pub selected_graph_edge_count: u32,
    pub citation_ref_count: u32,
    pub end_to_end_savings_status: String,
    pub cost_savings_status: String,
}

/// Blocked claim boundaries for a graph context packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbContextPacketBoundaries {
    pub repository_test_level_only: bool,
    pub production_runtime: String,
    pub default_context_replacement: String,
    pub citation_locator_status: String,
    pub billing_savings: String,
}

/// Import-tracking summary status attached to a graph context packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbContextPacketImportTrackingStatus {
    pub manifest_json: String,
    pub manifest_status: String,
    pub tracked_clean_evidence_enforced: bool,
    pub source_path_status: String,
}

/// Route lookup response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DagDbRouteLookupResponse {
    /// Stable wire-contract version (`dagdb_route_lookup_response_v1`).
    pub schema_version: String,
    pub tenant_id: String,
    pub namespace: String,
    pub route_id: String,
    pub route_status: RouteStatus,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub selected_memory_ids: Vec<String>,
    pub route_score_bp: u16,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub stale_at: String,
    pub latest_receipt_hash: String,
    pub memory_refs: Option<Vec<ContextPacketMemoryRef>>,
    pub validation_report: Option<DagDbValidateResponse>,
}

#[cfg(test)]
mod tests {
    use serde::{Serialize, de::DeserializeOwned};

    use super::*;

    #[test]
    fn dagdb_json_fixtures() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/json/all_dto_fixtures.json"))
                .expect("parse complete DAG DB fixture set");

        assert_fixture::<DagDbIntakeRequest>(&fixtures, "requests", "intake");
        assert_fixture::<DagDbRouteRequest>(&fixtures, "requests", "route");
        assert_fixture::<DagDbContextPacketRequest>(&fixtures, "requests", "context_packet");
        assert_fixture::<DagDbValidateRequest>(&fixtures, "requests", "validate");
        assert_fixture::<DagDbWritebackRequest>(&fixtures, "requests", "writeback");
        assert_fixture::<DagDbTrustCheckRequest>(&fixtures, "requests", "trust_check");
        assert_fixture::<DagDbCouncilDecisionRequest>(&fixtures, "requests", "council_decision");
        assert_fixture::<DagDbReceiptLookupRequest>(&fixtures, "requests", "receipt_lookup");
        assert_fixture::<DagDbCatalogLookupRequest>(&fixtures, "requests", "catalog_lookup");
        assert_fixture::<DagDbRouteLookupRequest>(&fixtures, "requests", "route_lookup");

        assert_fixture::<DagDbIntakeResponse>(&fixtures, "responses", "intake");
        assert_fixture::<DagDbRouteResponse>(&fixtures, "responses", "route");
        assert_fixture::<DagDbContextPacketResponse>(&fixtures, "responses", "context_packet");
        assert_fixture::<DagDbValidateResponse>(&fixtures, "responses", "validate");
        assert_fixture::<DagDbWritebackResponse>(&fixtures, "responses", "writeback");
        assert_fixture::<DagDbImportResponse>(&fixtures, "responses", "import");
        assert_fixture::<DagDbExportResponse>(&fixtures, "responses", "export");
        assert_fixture::<DagDbTrustCheckResponse>(&fixtures, "responses", "trust_check");
        assert_fixture::<DagDbCouncilDecisionResponse>(&fixtures, "responses", "council_decision");
        assert_fixture::<DagDbReceiptLookupResponse>(&fixtures, "responses", "receipt_lookup");
        assert_fixture::<DagDbCatalogLookupResponse>(&fixtures, "responses", "catalog_lookup");
        assert_fixture::<DagDbRouteLookupResponse>(&fixtures, "responses", "route_lookup");

        assert_fixture::<DagDbErrorEnvelope>(&fixtures, "errors", "tenant_scope_mismatch");
    }

    #[test]
    fn dagdb_runtime_import_export_dtos_deny_unknown_fields() {
        let import_request = DagDbImportRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            idempotency_key: "idem-import-1".into(),
            db_set_version: "dag_db-project_memory_v3".into(),
            source_hash: "1111111111111111111111111111111111111111111111111111111111111111".into(),
            requester_did: "did:exo:importer".into(),
            import_report: serde_json::json!({
                "schema_version": "dagdb_kg_dry_run_import_report_v1",
                "tenant_id": "tenant-a",
                "namespace": "primary"
            }),
        };
        let parsed: DagDbImportRequest = serde_json::from_value(
            serde_json::to_value(&import_request).expect("serialize import request"),
        )
        .expect("deserialize import request");
        assert_eq!(parsed, import_request);
        let import_err = serde_json::from_str::<DagDbImportRequest>(
            r#"{
              "tenant_id": "tenant-a",
              "namespace": "primary",
              "idempotency_key": "idem-import-1",
              "db_set_version": "dag_db-project_memory_v3",
              "source_hash": "1111111111111111111111111111111111111111111111111111111111111111",
              "requester_did": "did:exo:importer",
              "import_report": {},
              "raw_source_body": "forbidden"
            }"#,
        )
        .expect_err("unknown import request field must fail");
        assert!(import_err.to_string().contains("unknown field"));

        let export_request = DagDbExportRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            idempotency_key: "idem-export-1".into(),
            db_set_version: "dag_db-project_memory_v3".into(),
            requester_did: "did:exo:exporter".into(),
            included_memory_ids: vec![
                "2222222222222222222222222222222222222222222222222222222222222222".into(),
            ],
            included_graph_styles: vec!["semantic_catalog_graph".into()],
            included_writeback_idempotency_keys: Vec::new(),
            source_commit_or_repo_ref: Some("c706242d36f1c275e05d8a132778491da08f61c7".into()),
            include_preview_context: false,
        };
        let parsed: DagDbExportRequest = serde_json::from_value(
            serde_json::to_value(&export_request).expect("serialize export request"),
        )
        .expect("deserialize export request");
        assert_eq!(parsed, export_request);
        let export_err = serde_json::from_str::<DagDbExportRequest>(
            r#"{
              "tenant_id": "tenant-a",
              "namespace": "primary",
              "idempotency_key": "idem-export-1",
              "db_set_version": "dag_db-project_memory_v3",
              "requester_did": "did:exo:exporter",
              "included_memory_ids": [],
              "included_graph_styles": [],
              "included_writeback_idempotency_keys": [],
              "source_commit_or_repo_ref": null,
              "include_preview_context": false,
              "receipt_path": "/Users/example/private"
            }"#,
        )
        .expect_err("unknown export request field must fail");
        assert!(export_err.to_string().contains("unknown field"));
    }

    #[test]
    fn dagdb_graph_json_fixtures() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/json/all_dto_fixtures.json"))
                .expect("parse complete DAG DB fixture set");

        assert_fixture::<MemoryCandidate>(&fixtures, "graph", "memory_candidate");
        assert_fixture::<SimilarityResult>(&fixtures, "graph", "similarity_result");
        assert_fixture::<CanonicalizationDecision>(&fixtures, "graph", "canonicalization_decision");
        assert_fixture::<GraphView>(&fixtures, "graph", "graph_view");
        assert_fixture::<RouteInvalidationReceipt>(
            &fixtures,
            "graph",
            "route_invalidation_receipt",
        );
        assert_fixture::<PlacementResult>(&fixtures, "graph", "placement_result");

        let all_styles = [
            MemoryGraphStyle::ProvenanceReceiptDag,
            MemoryGraphStyle::CanonicalMemoryGraph,
            MemoryGraphStyle::SemanticCatalogGraph,
            MemoryGraphStyle::SimilarityOverlayGraph,
            MemoryGraphStyle::DependencyDag,
            MemoryGraphStyle::RoutingViewGraph,
            MemoryGraphStyle::ContradictionSupersessionGraph,
            MemoryGraphStyle::ContextPacketGraph,
        ];
        let encoded = serde_json::to_value(all_styles).expect("serialize graph styles");
        assert_eq!(
            encoded,
            serde_json::json!([
                "provenance_receipt_dag",
                "canonical_memory_graph",
                "semantic_catalog_graph",
                "similarity_overlay_graph",
                "dependency_dag",
                "routing_view_graph",
                "contradiction_supersession_graph",
                "context_packet_graph"
            ])
        );
    }

    #[test]
    fn dagdb_graph_context_selection_dtos_deny_unknown_fields() {
        let request = DagDbGraphContextSelectionRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task: "Select next implementation step".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            token_budget: 1_000,
            max_memory_refs: 4,
            catalog_hints: vec!["04_Plans".into()],
            requested_memory_ids: Vec::new(),
            force_revalidate: false,
        };
        let serialized = serde_json::to_value(&request).expect("serialize request");
        let parsed: DagDbGraphContextSelectionRequest =
            serde_json::from_value(serialized).expect("deserialize request");
        assert_eq!(parsed, request);

        let forged = r#"{
          "tenant_id": "tenant-a",
          "namespace": "primary",
          "request_id": "req-1",
          "task": "Select next implementation step",
          "task_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "token_budget": 1000,
          "max_memory_refs": 4,
          "catalog_hints": [],
          "requested_memory_ids": [],
          "force_revalidate": false,
          "raw_markdown": "forbidden"
        }"#;
        let err = serde_json::from_str::<DagDbGraphContextSelectionRequest>(forged)
            .expect_err("unknown request field must fail");
        assert!(err.to_string().contains("unknown field"));

        let response = DagDbGraphContextSelectionResponse {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            selection_status: DagDbGraphContextSelectionStatus::Selected,
            selected_memory_refs: vec![DagDbSelectedContextRef {
                memory_id: "mem-plan".into(),
                catalog_id: Some("catalog-plan".into()),
                title: SafeMetadata {
                    decision: SafeMetadataDecision::Allow,
                    text: "Plan".into(),
                    redaction_codes: Vec::new(),
                    original_hash:
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
                    truncated: false,
                    byte_len: 4,
                },
                summary: SafeMetadata {
                    decision: SafeMetadataDecision::Allow,
                    text: "Safe summary".into(),
                    redaction_codes: Vec::new(),
                    original_hash:
                        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
                    truncated: false,
                    byte_len: 12,
                },
                catalog_path: vec!["04_Plans".into(), "Next Steps".into()],
                document_type: "plan".into(),
                selection_reason: "task_term_match".into(),
                token_estimate: 120,
                validation_status: ValidationStatus::Passed,
                citation_ref: "citation:plan".into(),
                boundary_flags: vec!["repository_test_only".into()],
            }],
            selected_graph_edges: Vec::new(),
            omitted_memory_refs: Vec::new(),
            selection_trace: vec![DagDbGraphSelectionTraceStep {
                graph_style: MemoryGraphStyle::SemanticCatalogGraph,
                candidate_count_before: 1,
                candidate_count_after: 1,
                selected_count_after: 1,
                reason: "semantic_catalog_graph_considered".into(),
            }],
            selected_token_estimate: 120,
            token_budget: 1_000,
            boundary_warnings: vec!["production_runtime_not_approved".into()],
        };
        let serialized = serde_json::to_value(&response).expect("serialize response");
        let parsed: DagDbGraphContextSelectionResponse =
            serde_json::from_value(serialized).expect("deserialize response");
        assert_eq!(parsed, response);
    }

    #[test]
    fn dagdb_graph_context_packet_dtos_deny_unknown_fields() {
        let selection = DagDbGraphContextSelectionResponse {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            selection_status: DagDbGraphContextSelectionStatus::Selected,
            selected_memory_refs: Vec::new(),
            selected_graph_edges: Vec::new(),
            omitted_memory_refs: Vec::new(),
            selection_trace: Vec::new(),
            selected_token_estimate: 0,
            token_budget: 1_000,
            boundary_warnings: vec!["production_runtime_not_approved".into()],
        };
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task: "Build bounded context packet".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            audit_id: "audit-1".into(),
            token_budget: 1_000,
            selection,
            import_tracking_status: None,
        };
        let serialized = serde_json::to_value(&request).expect("serialize request");
        let parsed: DagDbGraphContextPacketBuildRequest =
            serde_json::from_value(serialized).expect("deserialize request");
        assert_eq!(parsed, request);

        let forged = r#"{
          "tenant_id": "tenant-a",
          "namespace": "primary",
          "request_id": "req-1",
          "task": "Build bounded context packet",
          "task_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "audit_id": "audit-1",
          "token_budget": 1000,
          "selection": {
            "tenant_id": "tenant-a",
            "namespace": "primary",
            "request_id": "req-1",
            "task_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "selection_status": "empty",
            "selected_memory_refs": [],
            "selected_graph_edges": [],
            "omitted_memory_refs": [],
            "selection_trace": [],
            "selected_token_estimate": 0,
            "token_budget": 1000,
            "boundary_warnings": []
          },
          "import_tracking_status": null,
          "raw_markdown": "forbidden"
        }"#;
        let err = serde_json::from_str::<DagDbGraphContextPacketBuildRequest>(forged)
            .expect_err("unknown request field must fail");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn dagdb_graph_context_selection_status_serializes_all_variants() {
        let variants = [
            DagDbGraphContextSelectionStatus::Selected,
            DagDbGraphContextSelectionStatus::Empty,
            DagDbGraphContextSelectionStatus::Failed,
        ];
        let encoded = serde_json::to_value(variants).expect("serialize statuses");
        assert_eq!(encoded, serde_json::json!(["selected", "empty", "failed"]));
    }

    #[test]
    fn dagdb_rejects_forged_safe_metadata() {
        let forged = r#"{
          "tenant_id": "tenant-a",
          "namespace": "primary",
          "idempotency_key": "idem-1",
          "source_type": "public_web",
          "source_hash": "1111111111111111111111111111111111111111111111111111111111111111",
          "payload_hash": "2222222222222222222222222222222222222222222222222222222222222222",
          "owner_did": "did:exo:owner",
          "controller_did": "did:exo:controller",
          "submitted_by_did": "did:exo:submitter",
          "consent_purpose": "retrieval",
          "requested_action": "memory:intake",
          "title_text": "Safe public title",
          "summary_text": "Safe public summary",
          "title": {
            "decision": "allow",
            "text": "forged trusted value",
            "redaction_codes": [],
            "original_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "truncated": false,
            "byte_len": 20
          }
        }"#;
        let err = serde_json::from_str::<DagDbIntakeRequest>(forged)
            .expect_err("unknown trusted SafeMetadata field must fail");
        assert!(err.to_string().contains("unknown field"));
    }

    fn assert_fixture<T>(fixtures: &serde_json::Value, section: &str, name: &str)
    where
        T: DeserializeOwned + Serialize,
    {
        let fixture = fixtures
            .get(section)
            .and_then(|section| section.get(name))
            .unwrap_or_else(|| panic!("missing fixture {section}.{name}"));
        let parsed: T = serde_json::from_value(fixture.clone())
            .unwrap_or_else(|err| panic!("parse fixture {section}.{name}: {err}"));
        let serialized = serde_json::to_value(parsed)
            .unwrap_or_else(|err| panic!("serialize fixture {section}.{name}: {err}"));
        assert_eq!(serialized, *fixture, "fixture {section}.{name} drifted");
    }
}
