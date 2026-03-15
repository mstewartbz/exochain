use super::*;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::advanced_policy::{AdvancedReasoningPolicy};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionObject {
    pub id: String,
    pub title: String,
    pub constitution_hash: String,
    pub authority_chain: Vec<authority::AuthorityLink>,
    pub merkle_root: String,
    pub status: Status,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub evidence: Vec<Evidence>,
    // Advanced Bayesian/Neuro-Symbolic Option
    pub advanced_reasoning: Option<AdvancedReasoningPolicy>,
    // Audit Observability
    pub audit_log: Vec<AuditEvent>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Status { Draft, Pending, Approved, Rejected, Contested, Void }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Evidence { pub hash: String, pub description: String }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub reason: String,
}

impl DecisionObject {
    pub fn new(title: &str) -> Self {
        #[cfg(test)]
        crate::requirements::Requirement::DecisionObjectCreation.mark_covered();

        let id = Uuid::new_v4().to_string();
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        let merkle_root = format!("{:x}", hasher.finalize());

        Self {
            id,
            title: title.to_string(),
            constitution_hash: "genesis-constitution-hash".to_string(),
            authority_chain: vec![],
            merkle_root,
            status: Status::Draft,
            created_at: chrono::Utc::now(),
            evidence: vec![],
            advanced_reasoning: None,
            audit_log: vec![],
        }
    }

    pub fn seal(&mut self) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::DecisionObjectSealing.mark_covered();

        // Advanced Mode Fast-Fail Validation
        if let Some(adv) = &self.advanced_reasoning {
            if let Err(reason) = adv.validate_thresholds() {
                self.audit_log.push(AuditEvent {
                    timestamp: chrono::Utc::now(),
                    event_type: "ESCALATION_REJECTION".to_string(),
                    reason: format!("{:?}", reason),
                });
                return Err(format!("Advanced Reasoning Threshold Violated: {:?}", reason));
            }
        }

        // Tentatively mark as Approved to check validation rules for Approved state
        let original_status = self.status.clone();
        self.status = Status::Approved;

        if let Err(e) = TNCEnforcer::enforce_all(self) {
            self.status = original_status;
            self.audit_log.push(AuditEvent {
                timestamp: chrono::Utc::now(),
                event_type: "TNC_ENFORCEMENT_FAILED".to_string(),
                reason: e.clone(),
            });
            return Err(e);
        }
        
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::requirements::Requirement;
    use crate::advanced_policy::{AdvancedReasoningPolicy, BayesianAssessment, HumanReviewStatus};

    #[test]
    pub fn test_decision_object_creation() {
        let title = "Test Creation";
        let obj = DecisionObject::new(title);
        assert_eq!(obj.title, title);
        assert_eq!(obj.status, Status::Draft);
        assert!(!obj.id.is_empty());
        assert_eq!(obj.constitution_hash, "genesis-constitution-hash");
        assert!(obj.authority_chain.is_empty());
        assert!(obj.evidence.is_empty());
        assert!(obj.advanced_reasoning.is_none());
        assert!(obj.audit_log.is_empty());
        Requirement::DecisionObjectCreation.mark_covered();
    }

    #[test]
    pub fn test_decision_object_seal_success() {
        let mut obj = DecisionObject::new("Test Seal");
        // TNC08 requires authority_chain to be non-empty when status is Approved
        obj.authority_chain.push(crate::authority::AuthorityLink {
            pubkey: "PK1".into(),
            signature: "SIG1".into(),
        });

        let res = obj.seal();
        assert!(res.is_ok());
        assert_eq!(obj.status, Status::Approved);
        Requirement::DecisionObjectSealing.mark_covered();
    }

    #[test]
    pub fn test_decision_object_seal_failure() {
        let mut obj = DecisionObject::new("Test Seal Failure");
        // TNC08 will fail because authority_chain is empty
        let res = obj.seal();
        assert!(res.is_err());
        // Status should be reverted to Draft
        assert_eq!(obj.status, Status::Draft);
        assert_eq!(obj.audit_log.len(), 1);
        assert_eq!(obj.audit_log[0].event_type, "TNC_ENFORCEMENT_FAILED");
    }

    #[test]
    pub fn test_decision_object_advanced_seal_rejection() {
        let mut obj = DecisionObject::new("Test Advanced Seal Failure");
        obj.authority_chain.push(crate::authority::AuthorityLink {
            pubkey: "PK1".into(),
            signature: "SIG1".into(),
        });
        // Fails due to high instability
        let policy = AdvancedReasoningPolicy::new(BayesianAssessment {
            prior: 0.5,
            posterior: 0.9,
            confidence_interval: 0.90,
            sensitivity_instability: 0.20, // Threshold is 0.10
            teacher_student_disagreement: 0.01,
            symbolic_rule_trace_hash: "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            evidence_references: vec!["http://evidence.link".to_string()],
        });
        obj.advanced_reasoning = Some(policy);
        
        let res = obj.seal();
        assert!(res.is_err());
        let err_str = res.unwrap_err();
        assert!(err_str.contains("HighInstability"));
        assert_eq!(obj.status, Status::Draft);
        assert_eq!(obj.audit_log.len(), 1);
        assert_eq!(obj.audit_log[0].event_type, "ESCALATION_REJECTION");
    }

    #[test]
    pub fn test_decision_object_advanced_seal_success() {
        let mut obj = DecisionObject::new("Test Advanced Seal Success");
        obj.authority_chain.push(crate::authority::AuthorityLink {
            pubkey: "PK1".into(),
            signature: "SIG1".into(),
        });
        
        let mut policy = AdvancedReasoningPolicy::new(BayesianAssessment {
            prior: 0.5,
            posterior: 0.9,
            confidence_interval: 0.90,
            sensitivity_instability: 0.05,
            teacher_student_disagreement: 0.01,
            symbolic_rule_trace_hash: "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            evidence_references: vec!["http://evidence.link".to_string()],
        });
        policy.human_review = HumanReviewStatus::approved_by("council-member-1");
        
        obj.advanced_reasoning = Some(policy);
        
        let res = obj.seal();
        assert!(res.is_ok());
        assert_eq!(obj.status, Status::Approved);
    }
}
