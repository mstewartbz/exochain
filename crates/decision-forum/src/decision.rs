//! Decision objects.
use exo_core::{Hash256, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{ForumError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionOutcome { Approved, Rejected, Tabled, Amended }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: Uuid,
    pub proposal_hash: Hash256,
    pub deliberation_id: Uuid,
    pub outcome: DecisionOutcome,
    pub evidence: Vec<Hash256>,
    pub rationale_hash: Hash256,
    pub enacted_at: Option<Timestamp>,
}

#[must_use]
pub fn create_decision(proposal_hash: Hash256, deliberation_id: Uuid, outcome: DecisionOutcome, rationale: &[u8]) -> Decision {
    Decision { id: Uuid::new_v4(), proposal_hash, deliberation_id, outcome,
        evidence: Vec::new(), rationale_hash: Hash256::digest(rationale), enacted_at: None }
}

pub fn enact(decision: &mut Decision) -> Result<()> {
    if decision.enacted_at.is_some() {
        return Err(ForumError::EnactmentFailed { reason: "already enacted".into() });
    }
    if decision.outcome == DecisionOutcome::Rejected {
        return Err(ForumError::EnactmentFailed { reason: "cannot enact rejected decision".into() });
    }
    decision.enacted_at = Some(Timestamp::ZERO);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn create_decision_not_enacted() { let d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Approved, b"why"); assert!(d.enacted_at.is_none()); assert_eq!(d.outcome, DecisionOutcome::Approved); }
    #[test] fn enact_approved() { let mut d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Approved, b"r"); enact(&mut d).unwrap(); assert!(d.enacted_at.is_some()); }
    #[test] fn enact_tabled() { let mut d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Tabled, b"r"); enact(&mut d).unwrap(); }
    #[test] fn enact_amended() { let mut d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Amended, b"r"); enact(&mut d).unwrap(); }
    #[test] fn enact_rejected_fails() { let mut d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Rejected, b"r"); assert!(enact(&mut d).is_err()); }
    #[test] fn enact_twice_fails() { let mut d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Approved, b"r"); enact(&mut d).unwrap(); assert!(enact(&mut d).is_err()); }
    #[test] fn outcome_serde() { for o in [DecisionOutcome::Approved, DecisionOutcome::Rejected, DecisionOutcome::Tabled, DecisionOutcome::Amended] { let j = serde_json::to_string(&o).unwrap(); let r: DecisionOutcome = serde_json::from_str(&j).unwrap(); assert_eq!(r, o); } }
    #[test] fn rationale_hash() { let d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Approved, b"rationale"); assert_eq!(d.rationale_hash, Hash256::digest(b"rationale")); }
    #[test] fn evidence_empty() { let d = create_decision(Hash256::ZERO, Uuid::nil(), DecisionOutcome::Approved, b"r"); assert!(d.evidence.is_empty()); }
}
