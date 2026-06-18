//! Domain record shells for ExoChain DAG DB.
//!
//! These types model the canonical records named by the MVP plan. Slice 1 uses
//! them only for contracts and ID material tests; runtime persistence and state
//! mutation are implemented by later slices.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    ConsentPurpose, CouncilDecisionStatus, CouncilReviewStatus, DagFinalityStatus, DecisionSource,
    MemoryCandidate, MemoryCandidateKind, MemoryCandidateUse, MemoryNodeType, MemoryStatus,
    RiskClass, SafeMetadata, SourceType, SubjectKind, ValidationDecision, ValidationStatus,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::hash::{
    AgentMemorySafetyScoreIdMaterial, CatalogEntryIdMaterial, ContextPacketIdMaterial,
    CouncilDecisionIdMaterial, InboundAgentCredentialIdMaterial, ParentLink,
    ReceiptMemoryObjectIdMaterial, RouteIdMaterial, ValidationReportIdMaterial,
};

const MEMORY_CANDIDATE_TYPE: &str = "MemoryCandidate";
const MEMORY_CANDIDATE_SUMMARY_MAX_BYTES: usize = 700;
const MEMORY_CANDIDATE_REASON_MAX_BYTES: usize = 300;

/// Compact task-agent writeback hint consumed by the system-side emitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAgentWritebackHint {
    pub candidate_kind: MemoryCandidateKind,
    pub summary: String,
    pub evidence_receipts: Vec<String>,
    pub risk_hint: RiskClass,
    pub allowed_future_uses: Vec<MemoryCandidateUse>,
    pub reason_to_remember: String,
}

/// Errors raised by compact memory-candidate validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MemoryCandidateValidationError {
    /// Candidate has the wrong discriminator.
    #[error("invalid_memory_candidate_type")]
    InvalidType,
    /// Candidate contains an empty required field.
    #[error("empty_memory_candidate_field: {field}")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },
    /// Candidate text exceeds its compact byte budget.
    #[error("memory_candidate_field_too_long: {field}")]
    TooLong {
        /// Field name.
        field: &'static str,
    },
    /// Candidate JSON includes a graph-allocation field or other unknown field.
    #[error("memory_candidate_shape_invalid: {reason}")]
    ShapeInvalid {
        /// Stable serde reason.
        reason: String,
    },
}

/// System-side observer that emits compact memory candidates after task completion.
pub struct OutputObserver;

/// Alias for the system-side candidate emitter named by the graph plan.
pub type MemoryCandidateEmitter = OutputObserver;

impl OutputObserver {
    /// Create a compact candidate from completed task output and a task-agent hint.
    pub fn observe_completed_task_output(
        source_request_id: String,
        parent_context_packet_id: String,
        full_output_hash: String,
        hint: TaskAgentWritebackHint,
    ) -> std::result::Result<MemoryCandidate, MemoryCandidateValidationError> {
        let candidate = MemoryCandidate {
            candidate_type: MEMORY_CANDIDATE_TYPE.to_owned(),
            source_request_id,
            candidate_kind: hint.candidate_kind,
            summary: hint.summary,
            full_output_hash,
            parent_context_packet_id,
            evidence_receipts: hint.evidence_receipts,
            risk_hint: hint.risk_hint,
            allowed_future_uses: hint.allowed_future_uses,
            reason_to_remember: hint.reason_to_remember,
        };
        Self::validate_compact_candidate(&candidate)?;
        Ok(candidate)
    }

    /// Parse a candidate JSON shape and reject graph-allocation fields through serde.
    pub fn reject_graph_allocation_fields(
        candidate_json: &str,
    ) -> std::result::Result<MemoryCandidate, MemoryCandidateValidationError> {
        let candidate: MemoryCandidate = serde_json::from_str(candidate_json).map_err(|error| {
            MemoryCandidateValidationError::ShapeInvalid {
                reason: error.to_string(),
            }
        })?;
        Self::validate_compact_candidate(&candidate)?;
        Ok(candidate)
    }

    /// Validate the compact candidate budget and required fields.
    pub fn validate_compact_candidate(
        candidate: &MemoryCandidate,
    ) -> std::result::Result<(), MemoryCandidateValidationError> {
        if candidate.candidate_type != MEMORY_CANDIDATE_TYPE {
            return Err(MemoryCandidateValidationError::InvalidType);
        }
        require_non_empty("source_request_id", &candidate.source_request_id)?;
        require_non_empty("summary", &candidate.summary)?;
        require_non_empty("full_output_hash", &candidate.full_output_hash)?;
        require_non_empty(
            "parent_context_packet_id",
            &candidate.parent_context_packet_id,
        )?;
        require_non_empty("reason_to_remember", &candidate.reason_to_remember)?;
        if candidate.summary.len() > MEMORY_CANDIDATE_SUMMARY_MAX_BYTES {
            return Err(MemoryCandidateValidationError::TooLong { field: "summary" });
        }
        if candidate.reason_to_remember.len() > MEMORY_CANDIDATE_REASON_MAX_BYTES {
            return Err(MemoryCandidateValidationError::TooLong {
                field: "reason_to_remember",
            });
        }
        Ok(())
    }
}

fn require_non_empty(
    field: &'static str,
    value: &str,
) -> std::result::Result<(), MemoryCandidateValidationError> {
    if value.is_empty() {
        return Err(MemoryCandidateValidationError::EmptyField { field });
    }
    Ok(())
}

/// Trusted authorization scope supplied by gateway/auth code or tests.
///
/// Runtime clients never submit this type. It binds the authorized tenant and
/// namespace to the actor, authority scope, consent scope, and allowed actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagDbAuthorizedScope {
    pub tenant_id: String,
    pub namespace: String,
    pub actor_did: String,
    pub authority_scope_hash: Hash256,
    pub consent_scope_hash: Hash256,
    pub permitted_actions: Vec<String>,
    pub expires_at: Timestamp,
}

/// Governed memory object stored by ExoChain DAG DB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptMemoryObject {
    pub memory_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub node_type: MemoryNodeType,
    pub source_type: SourceType,
    pub source_hash: Hash256,
    pub payload_hash: Hash256,
    pub owner_did: String,
    pub controller_did: String,
    pub submitted_by_did: String,
    pub consent_purpose: ConsentPurpose,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub keywords: Vec<SafeMetadata>,
    pub risk_class: RiskClass,
    pub risk_bp: u16,
    pub status: MemoryStatus,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub parent_memory_ids: Vec<Hash256>,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub payload_uri_hash: Option<Hash256>,
    pub access_policy_hash: Option<Hash256>,
    pub declared_rights_hash: Option<Hash256>,
    pub revoked_at: Option<Timestamp>,
    pub superseded_by_memory_id: Option<Hash256>,
}

impl ReceiptMemoryObject {
    /// Build the canonical entity ID material, excluding generated and mutable fields.
    #[must_use]
    pub fn id_material(&self, parent_links: Vec<ParentLink>) -> ReceiptMemoryObjectIdMaterial {
        ReceiptMemoryObjectIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.node_type,
            self.source_type,
            self.source_hash,
            self.payload_hash,
            self.owner_did.clone(),
            self.controller_did.clone(),
            self.consent_purpose,
            parent_links,
        )
    }
}

/// Safe catalog row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub catalog_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub catalog_level: u32,
    pub title: SafeMetadata,
    pub summary: SafeMetadata,
    pub keywords: Vec<SafeMetadata>,
    pub payload_hash: Hash256,
    pub source_hash: Hash256,
    pub status: MemoryStatus,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub memory_id: Option<Hash256>,
    pub parent_catalog_id: Option<Hash256>,
}

impl CatalogEntry {
    /// Build canonical catalog ID material.
    #[must_use]
    pub fn id_material(&self) -> CatalogEntryIdMaterial {
        CatalogEntryIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.memory_id,
            self.parent_catalog_id,
            self.catalog_level,
            self.payload_hash,
            self.source_hash,
        )
    }
}

/// Route receipt recording candidate and selected memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMemoryReceipt {
    pub route_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub requesting_agent_did: String,
    pub task_signature_hash: Hash256,
    pub approved_scope_hash: Hash256,
    pub candidate_memory_ids: Vec<Hash256>,
    pub selected_memory_ids: Vec<Hash256>,
    pub rejected_memory_ids: Vec<Hash256>,
    pub route_score_bp: u16,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub risk_bp: u16,
    pub status: exo_dag_db_api::RouteStatus,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub stale_at: Timestamp,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
    pub credential_id: Option<Hash256>,
    pub validation_report_id: Option<Hash256>,
    pub council_decision_id: Option<Hash256>,
}

impl RouteMemoryReceipt {
    /// Build canonical route ID material.
    #[must_use]
    pub fn id_material(&self) -> RouteIdMaterial {
        RouteIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.requesting_agent_did.clone(),
            self.task_signature_hash,
            self.approved_scope_hash,
            self.selected_memory_ids.clone(),
            self.token_budget,
        )
    }
}

/// Bounded context packet built from an active route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPacket {
    pub context_packet_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub route_id: Hash256,
    pub task_hash: Hash256,
    pub requesting_agent_did: String,
    pub memory_refs: Vec<Hash256>,
    pub packet_hash: Hash256,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub dag_finality_status: DagFinalityStatus,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
    pub validation_report_id: Option<Hash256>,
    pub council_decision_id: Option<Hash256>,
}

impl ContextPacket {
    /// Build canonical context packet ID material.
    #[must_use]
    pub fn id_material(&self) -> ContextPacketIdMaterial {
        ContextPacketIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.request_id.clone(),
            self.route_id,
            self.task_hash,
            self.memory_refs.clone(),
            self.token_budget,
        )
    }
}

/// Validation report for a DAG DB subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub validation_report_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub validator_did: String,
    pub input_hash: Hash256,
    pub policy_hash: Hash256,
    pub validation_status: ValidationStatus,
    pub risk_class: RiskClass,
    pub risk_bp: u16,
    pub decision: ValidationDecision,
    pub notes: SafeMetadata,
    pub contradictory_report_ids: Vec<Hash256>,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
    pub council_decision_id: Option<Hash256>,
}

impl ValidationReport {
    /// Build canonical validation report ID material.
    #[must_use]
    pub fn id_material(&self) -> ValidationReportIdMaterial {
        ValidationReportIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.subject_kind,
            self.subject_id,
            self.validator_did.clone(),
            self.input_hash,
            self.policy_hash,
        )
    }
}

/// Agent memory safety score record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMemorySafetyScore {
    pub safety_score_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub agent_did: String,
    pub operator_did: String,
    pub window_start: Timestamp,
    pub window_end: Timestamp,
    pub evidence_hash: Hash256,
    pub identity_bp: u16,
    pub authority_bp: u16,
    pub consent_bp: u16,
    pub provenance_bp: u16,
    pub validation_bp: u16,
    pub recency_bp: u16,
    pub revocation_bp: u16,
    pub route_quality_bp: u16,
    pub incident_penalty_bp: u16,
    pub total_score_bp: u16,
    pub validation_status: ValidationStatus,
    pub council_status: CouncilReviewStatus,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
}

impl AgentMemorySafetyScore {
    /// Build canonical safety score ID material.
    #[must_use]
    pub fn id_material(&self) -> AgentMemorySafetyScoreIdMaterial {
        AgentMemorySafetyScoreIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.agent_did.clone(),
            self.operator_did.clone(),
            self.window_start,
            self.window_end,
            self.evidence_hash,
        )
    }
}

/// Inbound credential for agent memory access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundAgentCredential {
    pub credential_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub agent_did: String,
    pub operator_did: String,
    pub model_name: String,
    pub model_version: String,
    pub provider_or_builder: String,
    pub requested_action: String,
    pub requested_scope_hash: Hash256,
    pub purpose: ConsentPurpose,
    pub autonomy_level: String,
    pub nonce: String,
    pub expires_at: Timestamp,
    pub signature_hash: Hash256,
    pub credential_status: exo_dag_db_api::CredentialStatus,
    pub created_at: Timestamp,
    pub checkpoint_hash: Option<Hash256>,
    pub attestation_hash: Option<Hash256>,
    pub prior_trust_receipt_hash: Option<Hash256>,
}

impl InboundAgentCredential {
    /// Build canonical credential ID material.
    #[must_use]
    pub fn id_material(&self) -> InboundAgentCredentialIdMaterial {
        InboundAgentCredentialIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.agent_did.clone(),
            self.operator_did.clone(),
            self.model_name.clone(),
            self.model_version.clone(),
            self.provider_or_builder.clone(),
            self.requested_action.clone(),
            self.requested_scope_hash,
            self.purpose,
            self.autonomy_level.clone(),
            self.nonce.clone(),
            self.expires_at,
        )
    }
}

/// Durable council decision for R3-R5 actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouncilDecision {
    pub decision_id: Hash256,
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub requested_action: String,
    pub approved_scope_hash: Hash256,
    pub risk_class: RiskClass,
    pub approver_did: String,
    pub decision_source: DecisionSource,
    pub decision_status: CouncilDecisionStatus,
    pub reason_code: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub receipt_hash: Hash256,
    pub validation_report_id: Option<Hash256>,
    pub route_id: Option<Hash256>,
    pub context_packet_id: Option<Hash256>,
    pub notes: Option<SafeMetadata>,
}

impl CouncilDecision {
    /// Build canonical council decision ID material.
    #[must_use]
    pub fn id_material(&self) -> CouncilDecisionIdMaterial {
        CouncilDecisionIdMaterial::new(
            self.tenant_id.clone(),
            self.namespace.clone(),
            self.subject_kind,
            self.subject_id,
            self.requested_action.clone(),
            self.approved_scope_hash,
            self.risk_class,
            self.approver_did.clone(),
            self.decision_source,
            self.created_at,
            self.expires_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use exo_dag_db_api::{CredentialStatus, RouteStatus, SafeMetadataDecision};

    use super::*;
    use crate::hash::ParentLink;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(physical_ms: u64, logical: u32) -> Timestamp {
        Timestamp::new(physical_ms, logical)
    }

    fn safe(text: &str) -> SafeMetadata {
        SafeMetadata {
            decision: SafeMetadataDecision::Allow,
            text: text.into(),
            redaction_codes: Vec::new(),
            original_hash: h(0xaa).to_string(),
            truncated: false,
            byte_len: u32::try_from(text.len()).expect("fixture text length fits u32"),
        }
    }

    #[test]
    fn authorized_scope_roundtrips_without_client_metadata() {
        let scope = DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:agent".into(),
            authority_scope_hash: h(0x01),
            consent_scope_hash: h(0x02),
            permitted_actions: vec!["dagdb:intake".into()],
            expires_at: ts(2_000, 0),
        };

        let encoded = serde_json::to_string(&scope).expect("serialize authorized scope");
        assert!(!encoded.contains("SafeMetadata"));
        let decoded: DagDbAuthorizedScope =
            serde_json::from_str(&encoded).expect("deserialize authorized scope");
        assert_eq!(decoded, scope);
    }

    #[test]
    fn memory_candidate_compact_and_agent_boundary() {
        let candidate = OutputObserver::observe_completed_task_output(
            "request-1".into(),
            h(0x40).to_string(),
            h(0x41).to_string(),
            TaskAgentWritebackHint {
                candidate_kind: MemoryCandidateKind::Decision,
                summary: "Keep graph allocation system-side.".into(),
                evidence_receipts: vec![h(0x42).to_string()],
                risk_hint: RiskClass::R1,
                allowed_future_uses: vec![MemoryCandidateUse::Routing, MemoryCandidateUse::Audit],
                reason_to_remember: "Architecture decision affects future routing.".into(),
            },
        )
        .expect("compact candidate is valid");
        assert_eq!(candidate.candidate_type, "MemoryCandidate");
        assert!(candidate.summary.len() <= 700);
        assert!(candidate.reason_to_remember.len() <= 300);

        let oversized = OutputObserver::observe_completed_task_output(
            "request-1".into(),
            h(0x40).to_string(),
            h(0x41).to_string(),
            TaskAgentWritebackHint {
                candidate_kind: MemoryCandidateKind::Plan,
                summary: "x".repeat(701),
                evidence_receipts: Vec::new(),
                risk_hint: RiskClass::R0,
                allowed_future_uses: vec![MemoryCandidateUse::Audit],
                reason_to_remember: "compact".into(),
            },
        );
        assert_eq!(
            oversized,
            Err(MemoryCandidateValidationError::TooLong { field: "summary" })
        );

        let graph_allocating = r#"{
          "type": "MemoryCandidate",
          "source_request_id": "request-1",
          "candidate_kind": "decision",
          "summary": "compact",
          "full_output_hash": "4141414141414141414141414141414141414141414141414141414141414141",
          "parent_context_packet_id": "4040404040404040404040404040404040404040404040404040404040404040",
          "evidence_receipts": [],
          "risk_hint": "R1",
          "allowed_future_uses": ["routing"],
          "reason_to_remember": "compact",
          "canonical_memory_id": "forbidden"
        }"#;
        assert!(matches!(
            OutputObserver::reject_graph_allocation_fields(graph_allocating),
            Err(MemoryCandidateValidationError::ShapeInvalid { .. })
        ));
    }

    #[test]
    fn memory_candidate_validation_rejects_each_compact_boundary() {
        let mut candidate = OutputObserver::observe_completed_task_output(
            "request-compact".into(),
            h(0x60).to_string(),
            h(0x61).to_string(),
            TaskAgentWritebackHint {
                candidate_kind: MemoryCandidateKind::Preference,
                summary: "compact preference".into(),
                evidence_receipts: Vec::new(),
                risk_hint: RiskClass::R0,
                allowed_future_uses: vec![MemoryCandidateUse::Inference],
                reason_to_remember: "future routing preference".into(),
            },
        )
        .expect("valid candidate");

        candidate.candidate_type = "GraphAllocation".into();
        assert_eq!(
            OutputObserver::validate_compact_candidate(&candidate),
            Err(MemoryCandidateValidationError::InvalidType)
        );
        candidate.candidate_type = "MemoryCandidate".into();

        for field in [
            "source_request_id",
            "summary",
            "full_output_hash",
            "parent_context_packet_id",
            "reason_to_remember",
        ] {
            let mut empty = candidate.clone();
            match field {
                "source_request_id" => empty.source_request_id.clear(),
                "summary" => empty.summary.clear(),
                "full_output_hash" => empty.full_output_hash.clear(),
                "parent_context_packet_id" => empty.parent_context_packet_id.clear(),
                "reason_to_remember" => empty.reason_to_remember.clear(),
                _ => unreachable!("fixture field is known"),
            }
            assert_eq!(
                OutputObserver::validate_compact_candidate(&empty),
                Err(MemoryCandidateValidationError::EmptyField { field })
            );
        }

        let mut long_reason = candidate;
        long_reason.reason_to_remember = "x".repeat(301);
        assert_eq!(
            OutputObserver::validate_compact_candidate(&long_reason),
            Err(MemoryCandidateValidationError::TooLong {
                field: "reason_to_remember"
            })
        );
    }

    #[test]
    fn output_observer_emits_compact_memory_candidate() {
        let candidate = MemoryCandidateEmitter::observe_completed_task_output(
            "request-2".into(),
            h(0x50).to_string(),
            h(0x51).to_string(),
            TaskAgentWritebackHint {
                candidate_kind: MemoryCandidateKind::SavingsObservation,
                summary: "DAG DB routing saved prompt tokens in the benchmark.".into(),
                evidence_receipts: vec![h(0x52).to_string(), h(0x53).to_string()],
                risk_hint: RiskClass::R0,
                allowed_future_uses: vec![MemoryCandidateUse::Inference],
                reason_to_remember: "Future benchmark comparisons need this result.".into(),
            },
        )
        .expect("emitter produces valid candidate");
        assert_eq!(candidate.source_request_id, "request-2");
        assert_eq!(
            candidate.candidate_kind,
            MemoryCandidateKind::SavingsObservation
        );
        assert_eq!(candidate.evidence_receipts.len(), 2);
    }

    #[test]
    fn record_id_material_builders_use_canonical_fields() {
        let memory = ReceiptMemoryObject {
            memory_id: h(0x10),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            node_type: MemoryNodeType::Source,
            source_type: SourceType::PublicWeb,
            source_hash: h(0x11),
            payload_hash: h(0x12),
            owner_did: "did:exo:owner".into(),
            controller_did: "did:exo:controller".into(),
            submitted_by_did: "did:exo:submitter".into(),
            consent_purpose: ConsentPurpose::Retrieval,
            title: safe("title"),
            summary: safe("summary"),
            keywords: vec![safe("keyword")],
            risk_class: RiskClass::R1,
            risk_bp: 1200,
            status: MemoryStatus::Pending,
            validation_status: ValidationStatus::Pending,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Pending,
            parent_memory_ids: vec![h(0x13)],
            latest_receipt_hash: h(0x14),
            created_at: ts(1_000, 0),
            updated_at: ts(1_001, 0),
            payload_uri_hash: Some(h(0x15)),
            access_policy_hash: Some(h(0x16)),
            declared_rights_hash: Some(h(0x17)),
            revoked_at: None,
            superseded_by_memory_id: None,
        };
        let memory_material = memory.id_material(vec![ParentLink::new(
            h(0x13),
            exo_dag_db_api::MemoryEdgeType::Parent,
        )]);
        assert_eq!(memory_material.tenant_id, "tenant-a");
        assert_eq!(memory_material.source_hash, h(0x11));
        assert_eq!(memory_material.payload_hash, h(0x12));

        let catalog = CatalogEntry {
            catalog_id: h(0x20),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            catalog_level: 1,
            title: safe("catalog"),
            summary: safe("catalog summary"),
            keywords: Vec::new(),
            payload_hash: h(0x21),
            source_hash: h(0x22),
            status: MemoryStatus::Routable,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            latest_receipt_hash: h(0x23),
            created_at: ts(1_000, 0),
            updated_at: ts(1_001, 0),
            memory_id: Some(h(0x24)),
            parent_catalog_id: Some(h(0x25)),
        };
        let catalog_material = catalog.id_material();
        assert_eq!(catalog_material.catalog_level, 1);
        assert_eq!(catalog_material.memory_id, Some(h(0x24)));

        let route = RouteMemoryReceipt {
            route_id: h(0x30),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            requesting_agent_did: "did:exo:agent".into(),
            task_signature_hash: h(0x31),
            approved_scope_hash: h(0x32),
            candidate_memory_ids: vec![h(0x33), h(0x34)],
            selected_memory_ids: vec![h(0x33)],
            rejected_memory_ids: vec![h(0x34)],
            route_score_bp: 8600,
            token_budget: 4096,
            token_estimate: 1024,
            risk_bp: 1200,
            status: RouteStatus::Active,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            stale_at: ts(86_401_000, 0),
            latest_receipt_hash: h(0x35),
            created_at: ts(1_000, 0),
            credential_id: Some(h(0x36)),
            validation_report_id: Some(h(0x37)),
            council_decision_id: None,
        };
        let route_material = route.id_material();
        assert_eq!(route_material.selected_memory_ids_ordered, vec![h(0x33)]);
        assert_eq!(route_material.token_budget, 4096);

        let packet = ContextPacket {
            context_packet_id: h(0x40),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "request-1".into(),
            route_id: h(0x30),
            task_hash: h(0x41),
            requesting_agent_did: "did:exo:agent".into(),
            memory_refs: vec![h(0x33)],
            packet_hash: h(0x42),
            token_budget: 2048,
            token_estimate: 800,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            latest_receipt_hash: h(0x43),
            created_at: ts(1_000, 0),
            validation_report_id: Some(h(0x44)),
            council_decision_id: None,
        };
        let packet_material = packet.id_material();
        assert_eq!(packet_material.request_id, "request-1");
        assert_eq!(packet_material.memory_refs_ordered, vec![h(0x33)]);

        let report = ValidationReport {
            validation_report_id: h(0x50),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            subject_kind: SubjectKind::Memory,
            subject_id: h(0x33),
            validator_did: "did:exo:validator".into(),
            input_hash: h(0x51),
            policy_hash: h(0x52),
            validation_status: ValidationStatus::Passed,
            risk_class: RiskClass::R1,
            risk_bp: 1200,
            decision: ValidationDecision::Allow,
            notes: safe("notes"),
            contradictory_report_ids: Vec::new(),
            latest_receipt_hash: h(0x53),
            created_at: ts(1_000, 0),
            council_decision_id: None,
        };
        let report_material = report.id_material();
        assert_eq!(report_material.validator_did, "did:exo:validator");
        assert_eq!(report_material.input_hash, h(0x51));

        let score = AgentMemorySafetyScore {
            safety_score_id: h(0x60),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            agent_did: "did:exo:agent".into(),
            operator_did: "did:exo:operator".into(),
            window_start: ts(1_000, 0),
            window_end: ts(2_000, 0),
            evidence_hash: h(0x61),
            identity_bp: 8000,
            authority_bp: 8000,
            consent_bp: 8000,
            provenance_bp: 8000,
            validation_bp: 8000,
            recency_bp: 8000,
            revocation_bp: 10000,
            route_quality_bp: 8000,
            incident_penalty_bp: 0,
            total_score_bp: 8200,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            latest_receipt_hash: h(0x62),
            created_at: ts(1_000, 0),
        };
        let score_material = score.id_material();
        assert_eq!(score_material.agent_did, "did:exo:agent");
        assert_eq!(score_material.evidence_hash, h(0x61));

        let credential = InboundAgentCredential {
            credential_id: h(0x70),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            agent_did: "did:exo:agent".into(),
            operator_did: "did:exo:operator".into(),
            model_name: "exo-agent".into(),
            model_version: "1.0.0".into(),
            provider_or_builder: "exo".into(),
            requested_action: "memory:route".into(),
            requested_scope_hash: h(0x71),
            purpose: ConsentPurpose::TrustCheck,
            autonomy_level: "supervised".into(),
            nonce: "nonce-1".into(),
            expires_at: ts(2_000, 0),
            signature_hash: h(0x72),
            credential_status: CredentialStatus::Active,
            created_at: ts(1_000, 0),
            checkpoint_hash: Some(h(0x73)),
            attestation_hash: Some(h(0x74)),
            prior_trust_receipt_hash: None,
        };
        let credential_material = credential.id_material();
        assert_eq!(credential_material.model_name, "exo-agent");
        assert_eq!(credential_material.requested_scope_hash, h(0x71));

        let decision = CouncilDecision {
            decision_id: h(0x80),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            subject_kind: SubjectKind::Memory,
            subject_id: h(0x33),
            requested_action: "memory:routable".into(),
            approved_scope_hash: h(0x81),
            risk_class: RiskClass::R3,
            approver_did: "did:exo:council".into(),
            decision_source: DecisionSource::Human,
            decision_status: CouncilDecisionStatus::Approved,
            reason_code: "operator_approved".into(),
            created_at: ts(1_000, 0),
            expires_at: ts(2_000, 0),
            receipt_hash: h(0x82),
            validation_report_id: Some(h(0x83)),
            route_id: None,
            context_packet_id: None,
            notes: Some(safe("approval notes")),
        };
        let decision_material = decision.id_material();
        assert_eq!(decision_material.approver_did, "did:exo:council");
        assert_eq!(decision_material.decision_source, DecisionSource::Human);
    }
}
