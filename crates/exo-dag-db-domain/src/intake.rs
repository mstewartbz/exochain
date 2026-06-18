//! Pure intake domain service for governed DAG DB memory.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    ConsentPurpose, CouncilReviewStatus, DagFinalityStatus, MemoryEdgeType, MemoryNodeType,
    MemoryStatus, SourceType, SubjectKind, ValidationStatus,
};

use crate::{
    council::ApprovalScope,
    hash::{ParentLink, ReceiptMemoryObjectIdMaterial},
    metadata::{MetadataField, sanitize_keywords, sanitize_runtime_metadata},
    model::{CouncilDecision, DagDbAuthorizedScope, ReceiptMemoryObject},
    scoring::{
        DomainGateContext, DomainResult, ensure_authority_and_consent, ensure_tenant_scope,
        hash_error, require_durable_approval, risk_class_for_bp,
    },
};

/// Intake material after gateway authentication and before persistence.
#[derive(Debug, Clone)]
pub struct IntakeDomainInput {
    pub tenant_id: String,
    pub namespace: String,
    pub source_type: SourceType,
    pub source_hash: Hash256,
    pub payload_hash: Hash256,
    pub owner_did: String,
    pub controller_did: String,
    pub submitted_by_did: String,
    pub consent_purpose: ConsentPurpose,
    pub requested_action: String,
    pub title_text: String,
    pub summary_text: String,
    pub keyword_texts: Vec<String>,
    pub parent_memory_ids: Vec<Hash256>,
    pub payload_uri_hash: Option<Hash256>,
    pub access_policy_hash: Option<Hash256>,
    pub declared_rights_hash: Option<Hash256>,
    pub risk_bp: u32,
    pub created_at: Timestamp,
    pub latest_receipt_hash: Hash256,
}

/// Built intake record plus deterministic duplicate status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakeDomainResult {
    pub memory: ReceiptMemoryObject,
    pub council_status: CouncilReviewStatus,
}

/// Build a safe memory object or fail closed before persistence.
pub fn intake_memory(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
    input: IntakeDomainInput,
    existing_memory: &[ReceiptMemoryObject],
    council_decision: Option<&CouncilDecision>,
) -> DomainResult<IntakeDomainResult> {
    ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
    ensure_authority_and_consent(scope, gate)?;

    if let Some(duplicate) = existing_memory
        .iter()
        .find(|memory| is_active_duplicate(memory, &input))
    {
        return Err(crate::scoring::DomainError::DuplicateActiveMemory {
            memory_id: duplicate.memory_id,
        });
    }

    let title = sanitize_runtime_metadata(MetadataField::Title, &input.title_text)?;
    let summary = sanitize_runtime_metadata(MetadataField::Summary, &input.summary_text)?;
    let keywords = sanitize_keywords(&input.keyword_texts)?;
    let risk_class = risk_class_for_bp(input.risk_bp)?;
    let risk_bp = u16::try_from(input.risk_bp.min(10_000)).map_err(|_| {
        crate::scoring::DomainError::ArithmeticOverflow {
            operation: "intake_risk_bp",
        }
    })?;
    let parent_links: Vec<ParentLink> = input
        .parent_memory_ids
        .iter()
        .copied()
        .map(|memory_id| ParentLink::new(memory_id, MemoryEdgeType::Parent))
        .collect();
    let memory_id = ReceiptMemoryObjectIdMaterial::new(
        input.tenant_id.clone(),
        input.namespace.clone(),
        MemoryNodeType::Source,
        input.source_type,
        input.source_hash,
        input.payload_hash,
        input.owner_did.clone(),
        input.controller_did.clone(),
        input.consent_purpose,
        parent_links,
    )
    .hash()
    .map_err(hash_error)?;
    let approval_scope = ApprovalScope {
        tenant_id: input.tenant_id.clone(),
        namespace: input.namespace.clone(),
        subject_kind: SubjectKind::Memory,
        subject_id: memory_id,
        requested_action: input.requested_action.clone(),
        approved_scope_hash: scope.authority_scope_hash,
        risk_class,
        council_decision_id: council_decision.map(|decision| decision.decision_id),
    };
    let council_status =
        require_durable_approval(&approval_scope, council_decision, input.created_at)?;

    Ok(IntakeDomainResult {
        memory: ReceiptMemoryObject {
            memory_id,
            tenant_id: input.tenant_id,
            namespace: input.namespace,
            node_type: MemoryNodeType::Source,
            source_type: input.source_type,
            source_hash: input.source_hash,
            payload_hash: input.payload_hash,
            owner_did: input.owner_did,
            controller_did: input.controller_did,
            submitted_by_did: input.submitted_by_did,
            consent_purpose: input.consent_purpose,
            title,
            summary,
            keywords,
            risk_class,
            risk_bp,
            status: MemoryStatus::Pending,
            validation_status: ValidationStatus::Pending,
            council_status,
            dag_finality_status: DagFinalityStatus::Pending,
            parent_memory_ids: input.parent_memory_ids,
            latest_receipt_hash: input.latest_receipt_hash,
            created_at: input.created_at,
            updated_at: input.created_at,
            payload_uri_hash: input.payload_uri_hash,
            access_policy_hash: input.access_policy_hash,
            declared_rights_hash: input.declared_rights_hash,
            revoked_at: None,
            superseded_by_memory_id: None,
        },
        council_status,
    })
}

fn is_active_duplicate(memory: &ReceiptMemoryObject, input: &IntakeDomainInput) -> bool {
    memory.tenant_id == input.tenant_id
        && memory.namespace == input.namespace
        && memory.payload_hash == input.payload_hash
        && memory.source_hash == input.source_hash
        && memory.node_type == MemoryNodeType::Source
        && matches!(
            memory.status,
            MemoryStatus::Pending | MemoryStatus::Approved | MemoryStatus::Routable
        )
        && memory.revoked_at.is_none()
        && memory.superseded_by_memory_id.is_none()
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;
    use exo_dag_db_api::SafeMetadataDecision;

    use super::*;
    use crate::scoring::DomainError;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn scope(actions: &[&str]) -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:agent".into(),
            authority_scope_hash: h(0x90),
            consent_scope_hash: h(0x91),
            permitted_actions: actions.iter().map(|action| (*action).to_owned()).collect(),
            expires_at: ts(10_000),
        }
    }

    fn gate(action: &str, permissions: &[Permission]) -> DomainGateContext {
        DomainGateContext {
            action: action.into(),
            authority_scope: AuthorityScope {
                permissions: permissions.to_vec(),
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn input() -> IntakeDomainInput {
        IntakeDomainInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            source_type: SourceType::PublicWeb,
            source_hash: h(0x10),
            payload_hash: h(0x11),
            owner_did: "did:exo:owner".into(),
            controller_did: "did:exo:controller".into(),
            submitted_by_did: "did:exo:submitter".into(),
            consent_purpose: ConsentPurpose::Retrieval,
            requested_action: "dagdb:intake".into(),
            title_text: "safe title".into(),
            summary_text: "safe summary".into(),
            keyword_texts: vec!["alpha".into()],
            parent_memory_ids: vec![h(0x12)],
            payload_uri_hash: None,
            access_policy_hash: None,
            declared_rights_hash: None,
            risk_bp: 1_200,
            created_at: ts(1_000),
            latest_receipt_hash: h(0x13),
        }
    }

    #[test]
    fn intake_builds_sanitized_memory_and_rejects_duplicates() {
        let result = intake_memory(
            &scope(&["dagdb:intake"]),
            &gate("dagdb:intake", &[Permission::Write]),
            input(),
            &[],
            None,
        )
        .expect("safe intake succeeds");
        assert_eq!(result.memory.tenant_id, "tenant-a");
        assert_eq!(result.memory.title.decision, SafeMetadataDecision::Allow);
        assert_eq!(result.memory.summary.text, "safe summary");
        assert_eq!(result.memory.status, MemoryStatus::Pending);
        assert_eq!(
            result.memory.council_status,
            CouncilReviewStatus::NotRequired
        );

        let duplicate = intake_memory(
            &scope(&["dagdb:intake"]),
            &gate("dagdb:intake", &[Permission::Write]),
            input(),
            &[result.memory],
            None,
        );
        assert!(matches!(
            duplicate,
            Err(DomainError::DuplicateActiveMemory { .. })
        ));
    }

    #[test]
    fn intake_failure_paths_fail_before_domain_mutation() {
        let mismatch = IntakeDomainInput {
            tenant_id: "tenant-b".into(),
            ..input()
        };
        assert!(matches!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                mismatch,
                &[],
                None,
            ),
            Err(DomainError::TenantScopeMismatch { .. })
        ));

        let denied = DomainGateContext {
            consent_decision: ConsentDecision::Denied {
                reason: "missing".into(),
            },
            ..gate("dagdb:intake", &[Permission::Write])
        };
        assert!(matches!(
            intake_memory(&scope(&["dagdb:intake"]), &denied, input(), &[], None),
            Err(DomainError::ConsentDenied { .. })
        ));

        let unsafe_metadata = IntakeDomainInput {
            summary_text: "fn main() { println!(\"raw code\"); }".into(),
            ..input()
        };
        assert!(matches!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                unsafe_metadata,
                &[],
                None,
            ),
            Err(DomainError::Metadata(_))
        ));

        let high_risk = IntakeDomainInput {
            risk_bp: 5_000,
            ..input()
        };
        assert_eq!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                high_risk,
                &[],
                None,
            ),
            Err(DomainError::ApprovalRequired)
        );

        assert!(matches!(
            intake_memory(
                &scope(&[]),
                &gate("dagdb:intake", &[Permission::Write]),
                input(),
                &[],
                None,
            ),
            Err(DomainError::AuthorityDenied { .. })
        ));

        assert!(matches!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Read]),
                input(),
                &[],
                None,
            ),
            Err(DomainError::AuthorityDenied { .. })
        ));

        let too_many_keywords = IntakeDomainInput {
            keyword_texts: (0..33).map(|index| format!("keyword-{index}")).collect(),
            ..input()
        };
        assert!(matches!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                too_many_keywords,
                &[],
                None,
            ),
            Err(DomainError::Metadata(_))
        ));
    }

    #[test]
    fn intake_duplicate_predicate_only_matches_active_equivalent_memory() {
        let active = intake_memory(
            &scope(&["dagdb:intake"]),
            &gate("dagdb:intake", &[Permission::Write]),
            input(),
            &[],
            None,
        )
        .expect("active memory");

        let mut revoked = active.memory.clone();
        revoked.status = MemoryStatus::Revoked;
        revoked.revoked_at = Some(ts(2_000));
        assert!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                input(),
                &[revoked],
                None,
            )
            .is_ok()
        );

        let mut superseded = active.memory.clone();
        superseded.status = MemoryStatus::Superseded;
        superseded.superseded_by_memory_id = Some(h(0x44));
        assert!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                input(),
                &[superseded],
                None,
            )
            .is_ok()
        );

        let mut different_source = active.memory;
        different_source.source_hash = h(0x55);
        assert!(
            intake_memory(
                &scope(&["dagdb:intake"]),
                &gate("dagdb:intake", &[Permission::Write]),
                input(),
                &[different_source],
                None,
            )
            .is_ok()
        );
    }

    #[test]
    fn intake_duplicate_predicate_checks_each_active_identity_field() {
        let active = intake_memory(
            &scope(&["dagdb:intake"]),
            &gate("dagdb:intake", &[Permission::Write]),
            input(),
            &[],
            None,
        )
        .expect("active memory")
        .memory;
        let candidate = input();

        assert!(is_active_duplicate(&active, &candidate));

        let mut different_tenant = active.clone();
        different_tenant.tenant_id = "tenant-b".into();
        assert!(!is_active_duplicate(&different_tenant, &candidate));

        let mut different_namespace = active.clone();
        different_namespace.namespace = "other".into();
        assert!(!is_active_duplicate(&different_namespace, &candidate));

        let mut different_payload = active.clone();
        different_payload.payload_hash = h(0x56);
        assert!(!is_active_duplicate(&different_payload, &candidate));

        let mut different_source = active.clone();
        different_source.source_hash = h(0x57);
        assert!(!is_active_duplicate(&different_source, &candidate));

        let mut different_node_type = active.clone();
        different_node_type.node_type = MemoryNodeType::Summary;
        assert!(!is_active_duplicate(&different_node_type, &candidate));

        let mut inactive_status = active.clone();
        inactive_status.status = MemoryStatus::Blocked;
        assert!(!is_active_duplicate(&inactive_status, &candidate));

        let mut revoked_at = active.clone();
        revoked_at.revoked_at = Some(ts(2_000));
        assert!(!is_active_duplicate(&revoked_at, &candidate));

        let mut superseded_by = active;
        superseded_by.superseded_by_memory_id = Some(h(0x58));
        assert!(!is_active_duplicate(&superseded_by, &candidate));
    }
}
