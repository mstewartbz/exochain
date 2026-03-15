use super::*;
use crate::advanced_policy::{
    AdvancedReasoningPolicy, BayesianAssessment, EscalationReason, HumanReviewStatus,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionObject {
    pub id: String,
    pub title: String,
    pub constitution_hash: String,
    pub authority_chain: Vec<authority::AuthorityLink>,
    pub merkle_root: String,
    pub status: Status,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub required_quorum: usize,
    pub decision_class: DecisionClass,
    pub sync_version: u64,
    pub expected_sync_version: Option<u64>,
    pub ratified_by: Option<String>,
    pub ratified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub human_review: HumanReviewStatus,
    pub evidence: Vec<Evidence>,
    pub advanced_reasoning: Option<AdvancedReasoningPolicy>,
    pub audit_log: Vec<AuditEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DecisionClass {
    Operational,
    Policy,
    Sovereignty,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Draft,
    Pending,
    Approved,
    Rejected,
    Contested,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Evidence {
    pub hash: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuditEventType {
    SealAttempt,
    EscalationRejection,
    TncEnforcementFailed,
    SealApproved,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
    pub reason: String,
    pub escalation_reason: Option<EscalationReason>,
    pub assessment_snapshot: Option<BayesianAssessment>,
}

impl DecisionObject {
    pub fn new(title: &str) -> Self {
        #[cfg(test)]
        crate::requirements::Requirement::DecisionObjectCreation.mark_covered();

        let id = Uuid::new_v4().to_string();
        let constitution_hash = crate::constitution::constitution_hash();

        let mut obj = Self {
            id,
            title: title.to_string(),
            constitution_hash,
            authority_chain: vec![],
            merkle_root: String::new(),
            status: Status::Draft,
            created_at: chrono::Utc::now(),
            required_quorum: 1,
            decision_class: DecisionClass::Operational,
            sync_version: 0,
            expected_sync_version: None,
            ratified_by: None,
            ratified_at: None,
            human_review: HumanReviewStatus::default(),
            evidence: vec![],
            advanced_reasoning: None,
            audit_log: vec![],
        };
        obj.recompute_merkle_root();
        obj
    }

    pub fn recompute_merkle_root(&mut self) {
        self.merkle_root = compute_merkle_root(self);
    }

    pub fn seal(&mut self) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::DecisionObjectSealing.mark_covered();

        self.audit_log.push(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SealAttempt,
            reason: "Seal requested".to_string(),
            escalation_reason: None,
            assessment_snapshot: None,
        });

        if let Some(adv) = self.advanced_reasoning.clone() {
            if let Err(reason) = adv.validate_thresholds() {
                self.audit_log.push(AuditEvent {
                    timestamp: chrono::Utc::now(),
                    event_type: AuditEventType::EscalationRejection,
                    reason: format!("Advanced reasoning threshold violated: {:?}", reason),
                    escalation_reason: Some(reason.clone()),
                    assessment_snapshot: Some(adv.assessment.clone()),
                });
                return Err(format!(
                    "Advanced Reasoning Threshold Violated: {:?}",
                    reason
                ));
            }
        }

        self.recompute_merkle_root();

        let original_status = self.status.clone();
        self.status = Status::Approved;

        if let Err(error) = TNCEnforcer::enforce_all(self) {
            self.status = original_status;
            self.audit_log.push(AuditEvent {
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::TncEnforcementFailed,
                reason: error.clone(),
                escalation_reason: None,
                assessment_snapshot: None,
            });
            return Err(error);
        }

        self.audit_log.push(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SealApproved,
            reason: "Decision approved".to_string(),
            escalation_reason: None,
            assessment_snapshot: None,
        });

        Ok(())
    }
}

fn compute_merkle_root(obj: &DecisionObject) -> String {
    let mut leaves = Vec::new();
    leaves.push(hash_leaf(format!("title:{}", obj.title)));
    leaves.push(hash_leaf(format!("constitution:{}", obj.constitution_hash)));
    leaves.push(hash_leaf(format!("class:{:?}", obj.decision_class)));

    for evidence in &obj.evidence {
        leaves.push(hash_leaf(format!(
            "evidence:{}:{}",
            evidence.hash, evidence.description
        )));
    }

    for link in &obj.authority_chain {
        leaves.push(hash_leaf(format!(
            "authority:{}:{}:{:?}",
            link.pubkey, link.signature, link.actor_kind
        )));
    }

    merkle_reduce(leaves)
}

fn hash_leaf(value: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_ref());
    format!("{:x}", hasher.finalize())
}

fn merkle_reduce(mut nodes: Vec<String>) -> String {
    if nodes.is_empty() {
        return hash_leaf("empty");
    }

    while nodes.len() > 1 {
        let mut next = Vec::new();
        let mut idx = 0;
        while idx < nodes.len() {
            let left = &nodes[idx];
            let right = nodes.get(idx + 1).unwrap_or(left);
            next.push(hash_leaf(format!("{}{}", left, right)));
            idx += 2;
        }
        nodes = next;
    }

    nodes.pop().unwrap_or_else(|| hash_leaf("empty"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::advanced_policy::AdvancedThresholds;
    use crate::authority::{ActorKind, AuthorityLink, ConflictDisclosure};
    use crate::requirements::Requirement;

    fn no_conflict() -> ConflictDisclosure {
        ConflictDisclosure {
            has_conflict: false,
            description: None,
            disclosed_at: chrono::Utc::now(),
        }
    }

    fn human_link(pubkey: &str, signature: &str) -> AuthorityLink {
        AuthorityLink {
            pubkey: pubkey.to_string(),
            signature: signature.to_string(),
            actor_kind: ActorKind::Human,
            expires_at: None,
            conflict_disclosure: Some(no_conflict()),
        }
    }

    fn valid_advanced_policy() -> AdvancedReasoningPolicy {
        let mut policy = AdvancedReasoningPolicy::new(BayesianAssessment {
            prior: 0.5,
            posterior: 0.9,
            confidence_interval: 0.90,
            sensitivity_instability: 0.05,
            teacher_student_disagreement: 0.01,
            symbolic_rule_trace_hash:
                "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            evidence_references: vec!["https://evidence.exochain.local/doc/1".to_string()],
        });
        policy.thresholds = AdvancedThresholds::default();
        policy
    }

    #[test]
    pub fn test_decision_object_creation() {
        let title = "Test Creation";
        let obj = DecisionObject::new(title);
        assert_eq!(obj.title, title);
        assert_eq!(obj.status, Status::Draft);
        assert!(!obj.id.is_empty());
        assert!(!obj.constitution_hash.is_empty());
        assert!(obj.authority_chain.is_empty());
        assert!(obj.evidence.is_empty());
        assert!(obj.advanced_reasoning.is_none());
        assert!(obj.audit_log.is_empty());
        assert_eq!(obj.required_quorum, 1);
        assert_eq!(obj.decision_class, DecisionClass::Operational);
        assert_eq!(obj.sync_version, 0);
        assert!(obj.expected_sync_version.is_none());
        assert!(!obj.merkle_root.is_empty());
        Requirement::DecisionObjectCreation.mark_covered();
    }

    #[test]
    pub fn test_decision_object_seal_success() {
        let mut obj = DecisionObject::new("Test Seal");
        obj.authority_chain
            .push(human_link("human-pubkey-0001", "human-signature-0001"));
        obj.human_review = HumanReviewStatus::approved_by("council:seal");

        let res = obj.seal();
        assert!(res.is_ok());
        assert_eq!(obj.status, Status::Approved);
        assert!(obj
            .audit_log
            .iter()
            .any(|event| event.event_type == AuditEventType::SealApproved));
        Requirement::DecisionObjectSealing.mark_covered();
    }

    #[test]
    pub fn test_decision_object_seal_failure_records_tnc_audit_event() {
        let mut obj = DecisionObject::new("Test Seal Failure");
        let res = obj.seal();
        assert!(res.is_err());
        assert_eq!(obj.status, Status::Draft);
        assert!(obj
            .audit_log
            .iter()
            .any(|event| event.event_type == AuditEventType::TncEnforcementFailed));
    }

    #[test]
    pub fn test_decision_object_advanced_seal_rejection_records_snapshot() {
        let mut obj = DecisionObject::new("Test Advanced Seal Failure");
        obj.authority_chain
            .push(human_link("human-pubkey-0002", "human-signature-0002"));
        obj.human_review = HumanReviewStatus::approved_by("council:advanced");

        let mut policy = valid_advanced_policy();
        policy.assessment.sensitivity_instability = 0.20;
        obj.advanced_reasoning = Some(policy);

        let res = obj.seal();
        assert!(res.is_err());
        assert_eq!(obj.status, Status::Draft);
        let event = obj
            .audit_log
            .iter()
            .find(|event| event.event_type == AuditEventType::EscalationRejection)
            .expect("expected escalation rejection audit event");
        assert_eq!(
            event.escalation_reason,
            Some(EscalationReason::HighInstability)
        );
        assert!(event.assessment_snapshot.is_some());
    }

    #[test]
    pub fn test_decision_object_advanced_seal_success() {
        let mut obj = DecisionObject::new("Test Advanced Seal Success");
        obj.authority_chain
            .push(human_link("human-pubkey-0003", "human-signature-0003"));
        obj.human_review = HumanReviewStatus::approved_by("council-member-1");
        obj.advanced_reasoning = Some(valid_advanced_policy());

        let res = obj.seal();
        assert!(res.is_ok());
        assert_eq!(obj.status, Status::Approved);
    }

    #[test]
    pub fn test_merkle_root_changes_when_evidence_changes() {
        let mut obj = DecisionObject::new("Merkle Test");
        let original = obj.merkle_root.clone();
        obj.evidence.push(Evidence {
            hash: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                .to_string(),
            description: "supporting exhibit".to_string(),
        });
        obj.recompute_merkle_root();
        assert_ne!(original, obj.merkle_root);
    }
}
