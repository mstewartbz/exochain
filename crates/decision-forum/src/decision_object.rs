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
        TNCEnforcer::enforce_all(self)?;
        self.status = Status::Approved;
        Ok(())
    }
}
