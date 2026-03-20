//! Challenge mechanism — constitutional brake per CR-001 section 8.5.

use exo_core::{Did, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::GovernanceError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeGround {
    AuthorityChainInvalid,
    QuorumViolation,
    UndisclosedConflict,
    ProceduralError,
    SybilAllegation,
    ConsentViolation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeStatus {
    Filed,
    UnderReview,
    Sustained,
    Overruled,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeVerdict {
    Sustain,
    Overrule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: Uuid,
    pub challenger_did: Did,
    pub target_action_id: [u8; 32],
    pub ground: ChallengeGround,
    pub evidence: Vec<u8>,
    pub status: ChallengeStatus,
    pub created: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseOrder {
    pub challenge_id: Uuid,
    pub target_action_id: [u8; 32],
    pub reason: String,
    pub issued: Timestamp,
}

#[must_use]
pub fn file_challenge(
    challenger: &Did,
    target: &[u8; 32],
    ground: ChallengeGround,
    evidence: &[u8],
) -> Challenge {
    Challenge {
        id: Uuid::new_v4(),
        challenger_did: challenger.clone(),
        target_action_id: *target,
        ground,
        evidence: evidence.to_vec(),
        status: ChallengeStatus::Filed,
        created: Timestamp::now_utc(),
    }
}

#[must_use]
pub fn pause_action(challenge: &Challenge) -> PauseOrder {
    PauseOrder {
        challenge_id: challenge.id,
        target_action_id: challenge.target_action_id,
        reason: format!("challenged on ground: {:?}", challenge.ground),
        issued: Timestamp::now_utc(),
    }
}

pub fn adjudicate(
    challenge: &mut Challenge,
    verdict: ChallengeVerdict,
) -> Result<(), GovernanceError> {
    match challenge.status {
        ChallengeStatus::Filed | ChallengeStatus::UnderReview => {
            challenge.status = match verdict {
                ChallengeVerdict::Sustain => ChallengeStatus::Sustained,
                ChallengeVerdict::Overrule => ChallengeStatus::Overruled,
            };
            Ok(())
        }
        _ => Err(GovernanceError::InvalidStateTransition {
            from: format!("{:?}", challenge.status),
            to: format!("{verdict:?}"),
        }),
    }
}

pub fn withdraw(challenge: &mut Challenge) -> Result<(), GovernanceError> {
    match challenge.status {
        ChallengeStatus::Filed | ChallengeStatus::UnderReview => {
            challenge.status = ChallengeStatus::Withdrawn;
            Ok(())
        }
        _ => Err(GovernanceError::InvalidStateTransition {
            from: format!("{:?}", challenge.status),
            to: "Withdrawn".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> [u8; 32] {
        [42u8; 32]
    }
    fn challenger() -> Did {
        Did::new("did:exo:challenger").expect("ok")
    }

    #[test]
    fn file_creates_filed() {
        let c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::QuorumViolation,
            b"ev",
        );
        assert_eq!(c.status, ChallengeStatus::Filed);
        assert_eq!(c.ground, ChallengeGround::QuorumViolation);
    }
    #[test]
    fn pause_order() {
        let c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::SybilAllegation,
            b"",
        );
        let o = pause_action(&c);
        assert_eq!(o.challenge_id, c.id);
        assert!(o.reason.contains("SybilAllegation"));
    }
    #[test]
    fn adjudicate_sustain() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::ProceduralError,
            b"",
        );
        assert!(adjudicate(&mut c, ChallengeVerdict::Sustain).is_ok());
        assert_eq!(c.status, ChallengeStatus::Sustained);
    }
    #[test]
    fn adjudicate_overrule() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::ConsentViolation,
            b"",
        );
        assert!(adjudicate(&mut c, ChallengeVerdict::Overrule).is_ok());
        assert_eq!(c.status, ChallengeStatus::Overruled);
    }
    #[test]
    fn adjudicate_from_under_review() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::UndisclosedConflict,
            b"",
        );
        c.status = ChallengeStatus::UnderReview;
        assert!(adjudicate(&mut c, ChallengeVerdict::Sustain).is_ok());
    }
    #[test]
    fn adjudicate_from_sustained_fails() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::AuthorityChainInvalid,
            b"",
        );
        c.status = ChallengeStatus::Sustained;
        assert!(adjudicate(&mut c, ChallengeVerdict::Overrule).is_err());
    }
    #[test]
    fn adjudicate_from_overruled_fails() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::AuthorityChainInvalid,
            b"",
        );
        c.status = ChallengeStatus::Overruled;
        assert!(adjudicate(&mut c, ChallengeVerdict::Sustain).is_err());
    }
    #[test]
    fn adjudicate_from_withdrawn_fails() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::AuthorityChainInvalid,
            b"",
        );
        c.status = ChallengeStatus::Withdrawn;
        assert!(adjudicate(&mut c, ChallengeVerdict::Sustain).is_err());
    }
    #[test]
    fn withdraw_from_filed() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::QuorumViolation,
            b"",
        );
        assert!(withdraw(&mut c).is_ok());
        assert_eq!(c.status, ChallengeStatus::Withdrawn);
    }
    #[test]
    fn withdraw_from_under_review() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::QuorumViolation,
            b"",
        );
        c.status = ChallengeStatus::UnderReview;
        assert!(withdraw(&mut c).is_ok());
    }
    #[test]
    fn withdraw_from_sustained_fails() {
        let mut c = file_challenge(
            &challenger(),
            &target(),
            ChallengeGround::QuorumViolation,
            b"",
        );
        c.status = ChallengeStatus::Sustained;
        assert!(withdraw(&mut c).is_err());
    }
    #[test]
    fn all_grounds() {
        for g in [
            ChallengeGround::AuthorityChainInvalid,
            ChallengeGround::QuorumViolation,
            ChallengeGround::UndisclosedConflict,
            ChallengeGround::ProceduralError,
            ChallengeGround::SybilAllegation,
            ChallengeGround::ConsentViolation,
        ] {
            assert_eq!(
                file_challenge(&challenger(), &target(), g, b"").status,
                ChallengeStatus::Filed
            );
        }
    }
}
