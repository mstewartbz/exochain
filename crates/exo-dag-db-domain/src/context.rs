//! Pure context-packet domain service.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{DagFinalityStatus, RouteStatus, ValidationStatus};
use serde::Serialize;

use crate::{
    model::{ContextPacket, DagDbAuthorizedScope, ReceiptMemoryObject, RouteMemoryReceipt},
    route::ensure_memory_eligible,
    scoring::{
        DomainError, DomainGateContext, DomainResult, ensure_authority_and_consent,
        ensure_tenant_scope, ensure_token_budget, hash_error, hash_event_body,
    },
};

/// Context-packet request material after gateway scope verification.
#[derive(Debug, Clone)]
pub struct ContextPacketDomainInput {
    pub tenant_id: String,
    pub namespace: String,
    pub request_id: String,
    pub task_hash: Hash256,
    pub requesting_agent_did: String,
    pub token_budget: u32,
    pub max_memory_refs: Option<usize>,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
}

#[derive(Debug, Serialize)]
struct PacketHashMaterial<'a> {
    tenant_id: &'a str,
    namespace: &'a str,
    request_id: &'a str,
    route_id: Hash256,
    task_hash: Hash256,
    memory_refs: &'a [Hash256],
}

/// Build a bounded context packet from an active committed route.
pub fn build_context_packet(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
    input: ContextPacketDomainInput,
    route: &RouteMemoryReceipt,
    memory: &[ReceiptMemoryObject],
    now: Timestamp,
) -> DomainResult<ContextPacket> {
    ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
    ensure_authority_and_consent(scope, gate)?;
    if route.tenant_id != input.tenant_id || route.namespace != input.namespace {
        return Err(DomainError::TenantScopeMismatch {
            expected_tenant_id: input.tenant_id,
            expected_namespace: input.namespace,
            actual_tenant_id: route.tenant_id.clone(),
            actual_namespace: route.namespace.clone(),
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

    let mut memory_refs = Vec::new();
    let mut token_estimate = 0u32;
    for memory_id in &route.selected_memory_ids {
        if input
            .max_memory_refs
            .is_some_and(|max_refs| memory_refs.len() >= max_refs)
        {
            break;
        }
        let selected = memory
            .iter()
            .find(|candidate| candidate.memory_id == *memory_id)
            .ok_or(DomainError::NonCommittedFinality {
                subject_id: *memory_id,
            })?;
        ensure_memory_eligible(&input.tenant_id, &input.namespace, selected)?;
        memory_refs.push(*memory_id);
        token_estimate =
            token_estimate
                .checked_add(256)
                .ok_or(DomainError::ArithmeticOverflow {
                    operation: "context_packet_token_estimate",
                })?;
    }
    if memory_refs.is_empty() {
        return Err(DomainError::NoEligibleMemory);
    }
    ensure_token_budget(token_estimate, input.token_budget)?;

    let packet_hash = hash_event_body(&PacketHashMaterial {
        tenant_id: &input.tenant_id,
        namespace: &input.namespace,
        request_id: &input.request_id,
        route_id: route.route_id,
        task_hash: input.task_hash,
        memory_refs: &memory_refs,
    })?;
    let context_packet_id = crate::hash::ContextPacketIdMaterial::new(
        input.tenant_id.clone(),
        input.namespace.clone(),
        input.request_id.clone(),
        route.route_id,
        input.task_hash,
        memory_refs.clone(),
        input.token_budget,
    )
    .hash()
    .map_err(hash_error)?;

    Ok(ContextPacket {
        context_packet_id,
        tenant_id: input.tenant_id,
        namespace: input.namespace,
        request_id: input.request_id,
        route_id: route.route_id,
        task_hash: input.task_hash,
        requesting_agent_did: input.requesting_agent_did,
        memory_refs,
        packet_hash,
        token_budget: input.token_budget,
        token_estimate,
        validation_status: ValidationStatus::Pending,
        council_status: route.council_status,
        dag_finality_status: DagFinalityStatus::Pending,
        latest_receipt_hash: input.latest_receipt_hash,
        created_at: input.created_at,
        validation_report_id: route.validation_report_id,
        council_decision_id: route.council_decision_id,
    })
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;
    use exo_dag_db_api::{
        ConsentPurpose, CouncilReviewStatus, MemoryNodeType, MemoryStatus, SafeMetadata,
        SafeMetadataDecision, SourceType,
    };

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
            original_hash: h(0xee).to_string(),
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
            permitted_actions: vec!["dagdb:context_packet".into()],
            expires_at: ts(20_000),
        }
    }

    fn gate() -> DomainGateContext {
        DomainGateContext {
            action: "dagdb:context_packet".into(),
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Read],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn memory(id: Hash256) -> ReceiptMemoryObject {
        ReceiptMemoryObject {
            memory_id: id,
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            node_type: MemoryNodeType::Source,
            source_type: SourceType::PublicWeb,
            source_hash: h(0x10),
            payload_hash: h(0x11),
            owner_did: "did:exo:owner".into(),
            controller_did: "did:exo:controller".into(),
            submitted_by_did: "did:exo:submitter".into(),
            consent_purpose: ConsentPurpose::Retrieval,
            title: safe(),
            summary: safe(),
            keywords: Vec::new(),
            risk_class: exo_dag_db_api::RiskClass::R1,
            risk_bp: 1_000,
            status: MemoryStatus::Routable,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: DagFinalityStatus::Committed,
            parent_memory_ids: Vec::new(),
            latest_receipt_hash: h(0x12),
            created_at: ts(1_000),
            updated_at: ts(1_000),
            payload_uri_hash: None,
            access_policy_hash: None,
            declared_rights_hash: None,
            revoked_at: None,
            superseded_by_memory_id: None,
        }
    }

    fn route(finality: DagFinalityStatus, stale_at: Timestamp) -> RouteMemoryReceipt {
        RouteMemoryReceipt {
            route_id: h(0x40),
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            requesting_agent_did: "did:exo:agent".into(),
            task_signature_hash: h(0x41),
            approved_scope_hash: h(0x90),
            candidate_memory_ids: vec![h(0x20)],
            selected_memory_ids: vec![h(0x20)],
            rejected_memory_ids: Vec::new(),
            route_score_bp: 9_000,
            token_budget: 2_000,
            token_estimate: 500,
            risk_bp: 1_000,
            status: RouteStatus::Active,
            validation_status: ValidationStatus::Passed,
            council_status: CouncilReviewStatus::NotRequired,
            dag_finality_status: finality,
            stale_at,
            latest_receipt_hash: h(0x42),
            created_at: ts(1_000),
            credential_id: None,
            validation_report_id: None,
            council_decision_id: None,
        }
    }

    fn input() -> ContextPacketDomainInput {
        ContextPacketDomainInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "request-1".into(),
            task_hash: h(0x43),
            requesting_agent_did: "did:exo:agent".into(),
            token_budget: 1_000,
            max_memory_refs: None,
            latest_receipt_hash: h(0x44),
            created_at: ts(2_000),
        }
    }

    #[test]
    fn context_packet_builds_from_active_committed_route_and_memory() {
        let packet = build_context_packet(
            &scope(),
            &gate(),
            input(),
            &route(DagFinalityStatus::Committed, ts(3_000)),
            &[memory(h(0x20))],
            ts(2_000),
        )
        .expect("packet builds");
        assert_eq!(packet.memory_refs, vec![h(0x20)]);
        assert_eq!(packet.validation_status, ValidationStatus::Pending);
        assert_eq!(packet.token_estimate, 256);
    }

    #[test]
    fn context_packet_failure_paths_fail_closed() {
        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &route(DagFinalityStatus::Pending, ts(3_000)),
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0x40),
            })
        );
        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &route(DagFinalityStatus::Committed, ts(2_000)),
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::StaleRoute)
        );
        let mut revoked = memory(h(0x20));
        revoked.status = MemoryStatus::Revoked;
        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &route(DagFinalityStatus::Committed, ts(3_000)),
                &[revoked],
                ts(2_000),
            ),
            Err(DomainError::RevokedMemory { memory_id: h(0x20) })
        );

        let mut inactive_route = route(DagFinalityStatus::Committed, ts(3_000));
        inactive_route.status = RouteStatus::Pending;
        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &inactive_route,
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::RouteNotActive)
        );

        let mut other_tenant_route = route(DagFinalityStatus::Committed, ts(3_000));
        other_tenant_route.tenant_id = "tenant-b".into();
        assert!(matches!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &other_tenant_route,
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut other_namespace_route = route(DagFinalityStatus::Committed, ts(3_000));
        other_namespace_route.namespace = "other".into();
        assert!(matches!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &other_namespace_route,
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let mut pending_memory = memory(h(0x20));
        pending_memory.dag_finality_status = DagFinalityStatus::Pending;
        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &route(DagFinalityStatus::Committed, ts(3_000)),
                &[pending_memory],
                ts(2_000),
            ),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0x20),
            })
        );

        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                ContextPacketDomainInput {
                    token_budget: 300,
                    ..input()
                },
                &route(DagFinalityStatus::Committed, ts(3_000)),
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::TokenBudgetExceeded {
                token_estimate: 256,
                token_budget: 300,
            })
        );

        let limited = build_context_packet(
            &scope(),
            &gate(),
            ContextPacketDomainInput {
                max_memory_refs: Some(1),
                ..input()
            },
            &RouteMemoryReceipt {
                selected_memory_ids: vec![h(0x20), h(0x21)],
                candidate_memory_ids: vec![h(0x20), h(0x21)],
                ..route(DagFinalityStatus::Committed, ts(3_000))
            },
            &[memory(h(0x20)), memory(h(0x21))],
            ts(2_000),
        )
        .expect("max refs limits packet");
        assert_eq!(limited.memory_refs, vec![h(0x20)]);

        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                ContextPacketDomainInput {
                    max_memory_refs: Some(0),
                    ..input()
                },
                &route(DagFinalityStatus::Committed, ts(3_000)),
                &[memory(h(0x20))],
                ts(2_000),
            ),
            Err(DomainError::NoEligibleMemory)
        );

        assert_eq!(
            build_context_packet(
                &scope(),
                &gate(),
                input(),
                &route(DagFinalityStatus::Committed, ts(3_000)),
                &[],
                ts(2_000),
            ),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0x20),
            })
        );
    }
}
