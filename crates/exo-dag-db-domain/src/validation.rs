//! Pure validation-report domain service.

use exo_core::{Hash256, Timestamp};
use exo_dag_db_api::{
    DagFinalityStatus, RiskClass, SubjectKind, ValidationDecision, ValidationStatus,
};

use crate::{
    council::ApprovalScope,
    metadata::{MetadataField, sanitize_runtime_metadata},
    model::{CouncilDecision, DagDbAuthorizedScope, ValidationReport},
    scoring::{
        DomainError, DomainGateContext, DomainResult, ensure_authority_and_consent,
        ensure_tenant_scope, hash_error, require_durable_approval, risk_class_requires_approval,
    },
};

/// Validation request material after gateway scope verification.
#[derive(Debug, Clone)]
pub struct ValidationDomainInput {
    pub tenant_id: String,
    pub namespace: String,
    pub subject_kind: SubjectKind,
    pub subject_id: Hash256,
    pub validator_did: String,
    pub input_hash: Hash256,
    pub policy_hash: Hash256,
    pub validation_status: ValidationStatus,
    pub decision: ValidationDecision,
    pub risk_class: RiskClass,
    pub risk_bp: u16,
    pub notes_text: String,
    pub contradictory_report_ids: Vec<Hash256>,
    pub subject_finality_status: DagFinalityStatus,
    pub latest_receipt_hash: Hash256,
    pub created_at: Timestamp,
}

/// Create a durable validation report record.
pub fn create_validation_report(
    scope: &DagDbAuthorizedScope,
    gate: &DomainGateContext,
    input: ValidationDomainInput,
    council_decision: Option<&CouncilDecision>,
) -> DomainResult<ValidationReport> {
    ensure_tenant_scope(scope, &input.tenant_id, &input.namespace)?;
    ensure_authority_and_consent(scope, gate)?;
    if input.subject_finality_status != DagFinalityStatus::Committed {
        return Err(DomainError::NonCommittedFinality {
            subject_id: input.subject_id,
        });
    }
    if risk_class_requires_approval(input.risk_class) {
        let approval_scope = ApprovalScope {
            tenant_id: input.tenant_id.clone(),
            namespace: input.namespace.clone(),
            subject_kind: input.subject_kind,
            subject_id: input.subject_id,
            requested_action: "dagdb:validate".into(),
            approved_scope_hash: scope.authority_scope_hash,
            risk_class: input.risk_class,
            council_decision_id: council_decision.map(|decision| decision.decision_id),
        };
        require_durable_approval(&approval_scope, council_decision, input.created_at)?;
    }
    let notes = sanitize_runtime_metadata(MetadataField::ValidationNotes, &input.notes_text)?;
    let validation_report_id = crate::hash::ValidationReportIdMaterial::new(
        input.tenant_id.clone(),
        input.namespace.clone(),
        input.subject_kind,
        input.subject_id,
        input.validator_did.clone(),
        input.input_hash,
        input.policy_hash,
    )
    .hash()
    .map_err(hash_error)?;
    Ok(ValidationReport {
        validation_report_id,
        tenant_id: input.tenant_id,
        namespace: input.namespace,
        subject_kind: input.subject_kind,
        subject_id: input.subject_id,
        validator_did: input.validator_did,
        input_hash: input.input_hash,
        policy_hash: input.policy_hash,
        validation_status: input.validation_status,
        risk_class: input.risk_class,
        risk_bp: input.risk_bp,
        decision: input.decision,
        notes,
        contradictory_report_ids: input.contradictory_report_ids,
        latest_receipt_hash: input.latest_receipt_hash,
        created_at: input.created_at,
        council_decision_id: council_decision.map(|decision| decision.decision_id),
    })
}

#[cfg(test)]
mod tests {
    use exo_authority::Permission;
    use exo_avc::AuthorityScope;
    use exo_consent::ConsentDecision;

    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256::from_bytes([byte; 32])
    }

    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    fn scope() -> DagDbAuthorizedScope {
        DagDbAuthorizedScope {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            actor_did: "did:exo:validator".into(),
            authority_scope_hash: h(0x90),
            consent_scope_hash: h(0x91),
            permitted_actions: vec!["dagdb:validate".into()],
            expires_at: ts(20_000),
        }
    }

    fn gate() -> DomainGateContext {
        DomainGateContext {
            action: "dagdb:validate".into(),
            authority_scope: AuthorityScope {
                permissions: vec![Permission::Challenge],
                tools: Vec::new(),
                data_classes: Vec::new(),
                counterparties: Vec::new(),
                jurisdictions: Vec::new(),
            },
            consent_decision: ConsentDecision::Granted { expires: None },
        }
    }

    fn input() -> ValidationDomainInput {
        ValidationDomainInput {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            subject_kind: SubjectKind::Memory,
            subject_id: h(0x10),
            validator_did: "did:exo:validator".into(),
            input_hash: h(0x11),
            policy_hash: h(0x12),
            validation_status: ValidationStatus::Passed,
            decision: ValidationDecision::Allow,
            risk_class: RiskClass::R1,
            risk_bp: 1_000,
            notes_text: "validated".into(),
            contradictory_report_ids: Vec::new(),
            subject_finality_status: DagFinalityStatus::Committed,
            latest_receipt_hash: h(0x13),
            created_at: ts(1_000),
        }
    }

    #[test]
    fn validation_report_uses_safe_notes_and_canonical_id() {
        let report = create_validation_report(&scope(), &gate(), input(), None)
            .expect("validation succeeds");
        assert_eq!(report.validation_status, ValidationStatus::Passed);
        assert_eq!(report.notes.text, "validated");
        assert_eq!(report.council_decision_id, None);
    }

    #[test]
    fn validation_failure_paths_fail_closed() {
        let pending_finality = ValidationDomainInput {
            subject_finality_status: DagFinalityStatus::Pending,
            ..input()
        };
        assert_eq!(
            create_validation_report(&scope(), &gate(), pending_finality, None),
            Err(DomainError::NonCommittedFinality {
                subject_id: h(0x10),
            })
        );

        let contradictory = ValidationDomainInput {
            validation_status: ValidationStatus::Contradictory,
            decision: ValidationDecision::Invalidate,
            contradictory_report_ids: vec![h(0x55)],
            ..input()
        };
        let report =
            create_validation_report(&scope(), &gate(), contradictory, None).expect("report");
        assert_eq!(report.validation_status, ValidationStatus::Contradictory);
        assert_eq!(report.contradictory_report_ids, vec![h(0x55)]);

        let high_risk = ValidationDomainInput {
            risk_class: RiskClass::R3,
            risk_bp: 5_000,
            ..input()
        };
        assert_eq!(
            create_validation_report(&scope(), &gate(), high_risk, None),
            Err(DomainError::ApprovalRequired)
        );

        let unsafe_notes = ValidationDomainInput {
            notes_text: "fn main() { println!(\"raw code\"); }".into(),
            ..input()
        };
        assert!(matches!(
            create_validation_report(&scope(), &gate(), unsafe_notes, None),
            Err(DomainError::Metadata(_))
        ));
    }
}
