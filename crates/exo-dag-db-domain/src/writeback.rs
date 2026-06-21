//! Pure writeback domain service for governed agent output memory.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    ConsentPurpose, CouncilReviewStatus, DagFinalityStatus, MemoryEdgeType, MemoryNodeType,
    MemoryStatus, RouteStatus, SourceType, SubjectKind, ValidationStatus,
};

use crate::{
    council::ApprovalScope,
    hash::{ParentLink, ReceiptMemoryObjectIdMaterial},
    metadata::{MetadataField, sanitize_keywords, sanitize_runtime_metadata},
    model::{
        ContextPacket, CouncilDecision, DagDbAuthorizedScope, ReceiptMemoryObject,
        RouteMemoryReceipt, ValidationReport,
    },
    scoring::{
        DomainError, DomainGateContext, DomainResult, ensure_authority_and_consent,
        ensure_tenant_scope, hash_error, require_durable_approval, risk_class_for_bp,
        risk_class_requires_approval,
    },
};

/// Writeback request material after gateway verification.
#[derive(Debug, Clone)]
pub struct WritebackDomainInput {
    pub tenant_id: String,
    pub namespace: String,
    pub requesting_agent_did: String,
    pub answer_hash: Hash256,
    pub summary_text: String,
    pub keyword_texts: Vec<String>,
    pub citation_hashes: Vec<Hash256>,
    pub safety_score_id: Option<Hash256>,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
}

/// Existing validated records required for writeback.
#[derive(Debug, Clone, Copy)]
pub struct WritebackDomainRefs<'a> {
    pub route: &'a RouteMemoryReceipt,
    pub packet: &'a ContextPacket,
    pub validation_report: &'a ValidationReport,
    pub council_decision: Option<&'a CouncilDecision>,
}

/// Create governed memory from validated agent output.
pub fn create_writeback_memory(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
    input: WritebackDomainInput,
    refs: WritebackDomainRefs<'_>,
    now: Timestamp,
) -> DomainResult<ReceiptMemoryObject> {
    ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
    ensure_authority_and_consent(scope, gate)?;
    ensure_route_and_packet_ready(&input, refs.route, refs.packet, refs.validation_report, now)?;
    let risk_class = risk_class_for_bp(u32::from(refs.route.risk_bp))?;
    if risk_class_requires_approval(risk_class) {
        let approval_scope = ApprovalScope {
            tenant_id: input.tenant_id.clone(),
            namespace: input.namespace.clone(),
            subject_kind: SubjectKind::Route,
            subject_id: refs.route.route_id,
            requested_action: "dagdb:writeback".into(),
            approved_scope_hash: refs.route.approved_scope_hash,
            risk_class,
            council_decision_id: refs.council_decision.map(|decision| decision.decision_id),
        };
        require_durable_approval(&approval_scope, refs.council_decision, now)?;
    }
    let title = sanitize_runtime_metadata(MetadataField::Title, "agent writeback")?;
    let summary = sanitize_runtime_metadata(MetadataField::Summary, &input.summary_text)?;
    let keywords = sanitize_keywords(&input.keyword_texts)?;
    let mut parent_links: Vec<ParentLink> = refs
        .packet
        .memory_refs
        .iter()
        .copied()
        .map(|memory_id| ParentLink::new(memory_id, MemoryEdgeType::Cites))
        .collect();
    parent_links.extend(
        input
            .citation_hashes
            .iter()
            .copied()
            .map(|hash| ParentLink::new(hash, MemoryEdgeType::Cites)),
    );
    let memory_id = ReceiptMemoryObjectIdMaterial::new(
        input.tenant_id.clone(),
        input.namespace.clone(),
        MemoryNodeType::Answer,
        SourceType::Generated,
        refs.packet.packet_hash,
        input.answer_hash,
        input.requesting_agent_did.clone(),
        input.requesting_agent_did.clone(),
        ConsentPurpose::Writeback,
        parent_links,
    )
    .hash()
    .map_err(hash_error)?;

    Ok(ReceiptMemoryObject {
        memory_id,
        tenant_id: input.tenant_id,
        namespace: input.namespace,
        node_type: MemoryNodeType::Answer,
        source_type: SourceType::Generated,
        source_hash: refs.packet.packet_hash,
        payload_hash: input.answer_hash,
        owner_did: input.requesting_agent_did.clone(),
        controller_did: input.requesting_agent_did.clone(),
        submitted_by_did: input.requesting_agent_did,
        consent_purpose: ConsentPurpose::Writeback,
        title,
        summary,
        keywords,
        risk_class,
        risk_bp: refs.route.risk_bp,
        status: MemoryStatus::Pending,
        validation_status: ValidationStatus::Pending,
        council_status: refs
            .council_decision
            .map_or(CouncilReviewStatus::NotRequired, |_| {
                CouncilReviewStatus::Approved
            }),
        dag_finality_status: DagFinalityStatus::Pending,
        parent_memory_ids: refs.packet.memory_refs.clone(),
        latest_receipt_hash: input.latest_receipt_hash,
        created_at: input.created_at,
        updated_at: input.created_at,
        payload_uri_hash: input.safety_score_id,
        access_policy_hash: None,
        declared_rights_hash: None,
        revoked_at: None,
        superseded_by_memory_id: None,
    })
}

fn ensure_route_and_packet_ready(
    input: &WritebackDomainInput,
    route: &RouteMemoryReceipt,
    packet: &ContextPacket,
    validation_report: &ValidationReport,
    now: Timestamp,
) -> DomainResult<()> {
    if route.tenant_id != input.tenant_id || route.namespace != input.namespace {
        return Err(DomainError::TenantScopeMismatch {
            expected_tenant_id: input.tenant_id.clone(),
            expected_namespace: input.namespace.clone(),
            actual_tenant_id: route.tenant_id.clone(),
            actual_namespace: route.namespace.clone(),
        });
    }
    if packet.tenant_id != input.tenant_id || packet.namespace != input.namespace {
        return Err(DomainError::TenantScopeMismatch {
            expected_tenant_id: input.tenant_id.clone(),
            expected_namespace: input.namespace.clone(),
            actual_tenant_id: packet.tenant_id.clone(),
            actual_namespace: packet.namespace.clone(),
        });
    }
    if route.status != RouteStatus::Active {
        return Err(DomainError::RouteNotActive);
    }
    if route.stale_at <= now {
        return Err(DomainError::StaleRoute);
    }
    if route.dag_finality_status != DagFinalityStatus::Committed {
        return Err(DomainError::NonCommittedFinality {
            subject_id: route.route_id,
        });
    }
    if packet.validation_status != ValidationStatus::Passed
        || validation_report.validation_status != ValidationStatus::Passed
    {
        return Err(DomainError::ContextPacketNotValidated);
    }
    if packet.dag_finality_status != DagFinalityStatus::Committed {
        return Err(DomainError::NonCommittedFinality {
            subject_id: packet.context_packet_id,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;
    use exo_dag_db_api::{RiskClass, SafeMetadata, SafeMetadataDecision, ValidationDecision};

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn safe() -> SafeMetadata {
        SafeMetadata {
            decision: SafeMetadataDecision::Allow,
            text: "safe".into(),
            redaction_codes: Vec::new(),
            original_hash: h(0xaa).to_string(),
            truncated: false,
            byte_len: 4,
        }
    }

    fn scope() -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:agent".into(),
            authority_scope_hash: h(0x90),
            consent_scope_hash: h(0x91),
            permitted_actions: vec!["dagdb:writeback".into()],
            expires_at: ts(20_000),
        }
    }

    fn gate() -> DomainGateContext {
        DomainGateContext {
            action: "dagdb:writeback".into(),
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Write],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn route(stale_at: Timestamp, finality: DagFinalityStatus) -> RouteMemoryReceipt {
        RouteMemoryReceipt {
            route_id: h(0x20),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            requesting_agent_did: "did:exo:agent".into(),
            task_signature_hash: h(0x21),
            approved_scope_hash: h(0x90),
            candidate_memory_ids: vec![h(0x10)],
            selected_memory_ids: vec![h(0x10)],
            rejected_memory_ids: Vec::new(),
            route_score_bp: 9_000,
            token_budget: 2_000,
            token_estimate: 512,
            risk_bp: 1_000,
            status: RouteStatus::Active,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: finality,
            stale_at,
            latest_receipt_hash: h(0x22),
            created_at: ts(1_000),
            credential_id: None,
            validation_report_id: Some(h(0x50)),
            council_decision_id: None,
        }
    }

    fn packet(validation_status: ValidationStatus, finality: DagFinalityStatus) -> ContextPacket {
        ContextPacket {
            context_packet_id: h(0x30),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "request-1".into(),
            route_id: h(0x20),
            task_hash: h(0x31),
            requesting_agent_did: "did:exo:agent".into(),
            memory_refs: vec![h(0x10)],
            packet_hash: h(0x32),
            token_budget: 1_000,
            token_estimate: 256,
            validation_status,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: finality,
            latest_receipt_hash: h(0x33),
            created_at: ts(1_100),
            validation_report_id: Some(h(0x50)),
            council_decision_id: None,
        }
    }

    fn report(status: ValidationStatus) -> ValidationReport {
        ValidationReport {
            validation_report_id: h(0x50),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            subject_kind: SubjectKind::ContextPacket,
            subject_id: h(0x30),
            validator_did: "did:exo:validator".into(),
            input_hash: h(0x51),
            policy_hash: h(0x52),
            validation_status: status,
            risk_class: RiskClass::R1,
            risk_bp: 1_000,
            decision: ValidationDecision::Allow,
            notes: safe(),
            contradictory_report_ids: Vec::new(),
            latest_receipt_hash: h(0x53),
            created_at: ts(1_200),
            council_decision_id: None,
        }
    }

    fn input() -> WritebackDomainInput {
        WritebackDomainInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            requesting_agent_did: "did:exo:agent".into(),
            answer_hash: h(0x60),
            summary_text: "safe answer summary".into(),
            keyword_texts: vec!["answer".into()],
            citation_hashes: vec![h(0x10)],
            safety_score_id: None,
            latest_receipt_hash: h(0x61),
            created_at: ts(1_300),
        }
    }

    #[test]
    fn writeback_creates_safe_memory_from_validated_route_and_packet() {
        let memory = create_writeback_memory(
            &scope(),
            &gate(),
            input(),
            WritebackDomainRefs {
                route: &route(ts(3_000), DagFinalityStatus::Committed),
                packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                validation_report: &report(ValidationStatus::Passed),
                council_decision: None,
            },
            ts(2_000),
        )
        .expect("writeback succeeds");
        assert_eq!(memory.node_type, MemoryNodeType::Answer);
        assert_eq!(memory.source_type, SourceType::Generated);
        assert_eq!(memory.summary.text, "safe answer summary");
        assert_eq!(memory.parent_memory_ids, vec![h(0x10)]);
    }

    #[test]
    fn writeback_failure_paths_fail_closed() {
        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(2_000), DagFinalityStatus::Committed),
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::StaleRoute)
        );
        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Pending),
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0x20),
            })
        );
        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Committed),
                    packet: &packet(ValidationStatus::Pending, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::ContextPacketNotValidated)
        );
        let unsafe_input = WritebackDomainInput {
            summary_text: "fn main() { println!(\"raw code\"); }".into(),
            ..input()
        };
        assert!(matches!(
            create_writeback_memory(
                &scope(),
                &gate(),
                unsafe_input,
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Committed),
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::Metadata(_))
        ));

        let mut inactive_route = route(ts(3_000), DagFinalityStatus::Committed);
        inactive_route.status = RouteStatus::Pending;
        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &inactive_route,
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::RouteNotActive)
        );

        let mut other_tenant_packet =
            packet(ValidationStatus::Passed, DagFinalityStatus::Committed);
        other_tenant_packet.namespace = "other".into();
        assert!(matches!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Committed),
                    packet: &other_tenant_packet,
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut other_tenant_route = route(ts(3_000), DagFinalityStatus::Committed);
        other_tenant_route.tenant_id = "tenant-b".into();
        assert!(matches!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &other_tenant_route,
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut other_namespace_route = route(ts(3_000), DagFinalityStatus::Committed);
        other_namespace_route.namespace = "other".into();
        assert!(matches!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &other_namespace_route,
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut other_tenant_packet =
            packet(ValidationStatus::Passed, DagFinalityStatus::Committed);
        other_tenant_packet.tenant_id = "tenant-b".into();
        assert!(matches!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Committed),
                    packet: &other_tenant_packet,
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Committed),
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Pending),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0x30),
            })
        );

        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &route(ts(3_000), DagFinalityStatus::Committed),
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Failed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::ContextPacketNotValidated)
        );

        assert_eq!(
            create_writeback_memory(
                &scope(),
                &gate(),
                input(),
                WritebackDomainRefs {
                    route: &RouteMemoryReceipt {
                        risk_bp: 5_000,
                        ..route(ts(3_000), DagFinalityStatus::Committed)
                    },
                    packet: &packet(ValidationStatus::Passed, DagFinalityStatus::Committed),
                    validation_report: &report(ValidationStatus::Passed),
                    council_decision: None,
                },
                ts(2_000),
            ),
            Err(DomainError::ApprovalRequired)
        );
    }
}
