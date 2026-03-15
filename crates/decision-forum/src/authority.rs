use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConflictDisclosure {
    pub has_conflict: bool,
    pub description: Option<String>,
    pub disclosed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthorityLink {
    pub pubkey: String,
    pub signature: String,
    pub actor_kind: ActorKind,
    pub expires_at: Option<DateTime<Utc>>,
    pub conflict_disclosure: Option<ConflictDisclosure>,
}
