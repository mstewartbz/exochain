use super::*;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::advanced_policy::AdvancedReasoningPolicy;

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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Status { Draft, Pending, Approved, Rejected, Contested, Void }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Evidence { pub hash: String, pub description: String }

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
        }
    }

    pub fn seal(&mut self) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::DecisionObjectSealing.mark_covered();

        // Advanced Mode Fast-Fail Validation
        if let Some(adv) = &self.advanced_reasoning {
            if let Err(reason) = adv.validate_thresholds() {
                return Err(format!("Advanced Reasoning Threshold Violated: {:?}", reason));
            }
        }

        // Tentatively mark as Approved to check validation rules for Approved state
        let original_status = self.status.clone();
        self.status = Status::Approved;

        if let Err(e) = TNCEnforcer::enforce_all(self) {
            self.status = original_status;
            return Err(e);
        }
        
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::requirements::Requirement;
    use crate::advanced_policy::{AdvancedReasoningPolicy, BayesianAssessment};

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
    }

    #[test]
    pub fn test_decision_object_advanced_seal_rejection() {
        let mut obj = DecisionObject::new("Test Advanced Seal Failure");
        obj.authority_chain.push(crate::authority::AuthorityLink {
            pubkey: "PK1".into(),
            signature: "SIG1".into(),
        });
        // Fails due to high instability
        obj.advanced_reasoning = Some(AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.90,
                sensitivity_instability: 0.20, // Threshold is 0.10
                teacher_student_disagreement: 0.01,
                symbolic_rule_trace_hash: "0x123".to_string(),
                evidence_references: vec![],
            }
        });
        
        let res = obj.seal();
        assert!(res.is_err());
        let err_str = res.unwrap_err();
        assert!(err_str.contains("HighInstability"));
        assert_eq!(obj.status, Status::Draft);
    }

    #[test]
    pub fn test_decision_object_advanced_seal_success() {
        let mut obj = DecisionObject::new("Test Advanced Seal Success");
        obj.authority_chain.push(crate::authority::AuthorityLink {
            pubkey: "PK1".into(),
            signature: "SIG1".into(),
        });
        obj.advanced_reasoning = Some(AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.90,
                sensitivity_instability: 0.05,
                teacher_student_disagreement: 0.01,
                symbolic_rule_trace_hash: "0x123".to_string(),
                evidence_references: vec!["doc".to_string()],
            }
        });
        
        let res = obj.seal();
        assert!(res.is_ok());
        assert_eq!(obj.status, Status::Approved);
    }
}
