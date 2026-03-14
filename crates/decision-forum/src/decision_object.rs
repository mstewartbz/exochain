use super::*;
use sha2::{Sha256, Digest};
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
    pub evidence: Vec<Evidence>,
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
        }
    }

    pub fn seal(&mut self) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::DecisionObjectSealing.mark_covered();

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
}
