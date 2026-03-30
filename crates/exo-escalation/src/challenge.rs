//! Challenge paths for Sybil adjudication — CR-001 section 8.5.
//!
//! Any credible allegation of one of the four formal Sybil challenge grounds
//! is admitted through `admit_challenge`, which immediately places the
//! contested action in `ContestStatus::PauseEligible` and opens an audit
//! trail.  The caller then signals the CGR Kernel that the action is under
//! active challenge by setting `active_challenge_reason` on the
//! `AdjudicationContext`, causing the kernel to return `Verdict::Escalated`
//! rather than `Verdict::Denied`.

use exo_core::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::EscalationError;

// ---------------------------------------------------------------------------
// Challenge grounds (CR-001 §8.5)
// ---------------------------------------------------------------------------

/// The four formal Sybil challenge grounds recognised by EXOCHAIN.
///
/// Any credible allegation on any of these grounds is admissible and places
/// the contested action in a pause-eligible hold pending review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SybilChallengeGround {
    /// One underlying actor or control plane appears as multiple independent
    /// approvers, reviewers, or DIDs.
    ConcealedCommonControl,
    /// Multiple actors behave in lockstep so as to inflate apparent consensus
    /// or quorum without genuine independent judgment.
    CoordinatedManipulation,
    /// The counted quorum is tainted by non-independent, coordinated, or
    /// synthetic participants.
    QuorumContamination,
    /// A synthetic (AI-generated) opinion or entity is presented as if it
    /// were an independent human participant.
    SyntheticHumanMisrepresentation,
}

impl std::fmt::Display for SybilChallengeGround {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcealedCommonControl => write!(f, "ConcealedCommonControl"),
            Self::CoordinatedManipulation => write!(f, "CoordinatedManipulation"),
            Self::QuorumContamination => write!(f, "QuorumContamination"),
            Self::SyntheticHumanMisrepresentation => write!(f, "SyntheticHumanMisrepresentation"),
        }
    }
}

// ---------------------------------------------------------------------------
// Contest status
// ---------------------------------------------------------------------------

/// Lifecycle status of a contested action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContestStatus {
    /// Challenge admitted; action is paused pending review.  Callers MUST
    /// propagate this into `AdjudicationContext::active_challenge_reason` so
    /// the CGR Kernel returns `Verdict::Escalated`.
    PauseEligible,
    /// Evidentiary review is in progress.
    UnderReview,
    /// Challenge resolved: the contested action may proceed (or was reversed).
    Resolved,
    /// Challenge dismissed: insufficient grounds; action is unblocked.
    Dismissed,
}

// ---------------------------------------------------------------------------
// ContestHold
// ---------------------------------------------------------------------------

/// A hold placed on a contested action upon challenge admission.
///
/// Every state transition appends an entry to `audit_log`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestHold {
    pub id: Uuid,
    /// Identifies the action under challenge (matches kernel `action_id`).
    pub action_id: [u8; 32],
    pub ground: SybilChallengeGround,
    pub status: ContestStatus,
    pub admitted_at: Timestamp,
    /// Append-only audit trail of status transitions.
    pub audit_log: Vec<String>,
}

impl ContestHold {
    /// Returns a human-readable reason string suitable for embedding in
    /// `AdjudicationContext::active_challenge_reason`.
    #[must_use]
    pub fn escalation_reason(&self) -> String {
        format!(
            "SybilChallenge/{}: action {:?} is pause-eligible under active review",
            self.ground, self.action_id
        )
    }
}

// ---------------------------------------------------------------------------
// Challenge admission
// ---------------------------------------------------------------------------

/// Admit a credible Sybil challenge and return a `ContestHold` in
/// `PauseEligible` status.
///
/// The caller is responsible for:
/// 1. Storing the `ContestHold` in a durable audit store.
/// 2. Passing `hold.escalation_reason()` into the kernel's
///    `AdjudicationContext::active_challenge_reason` so the CGR Kernel
///    returns `Verdict::Escalated` (not `Verdict::Denied`) while review is
///    pending.
///
/// `admitted_at` is supplied by the caller to avoid internal clock calls.
#[must_use]
pub fn admit_challenge(
    action_id: &[u8; 32],
    ground: SybilChallengeGround,
    admitted_at: Timestamp,
) -> ContestHold {
    let entry = format!("admitted at {admitted_at:?}: ground {ground}");
    ContestHold {
        id: Uuid::new_v4(),
        action_id: *action_id,
        ground,
        status: ContestStatus::PauseEligible,
        admitted_at,
        audit_log: vec![entry],
    }
}

/// Advance a contest hold to `UnderReview`.
pub fn begin_review(hold: &mut ContestHold, at: Timestamp) -> Result<(), EscalationError> {
    if hold.status != ContestStatus::PauseEligible {
        return Err(EscalationError::InvalidStateTransition {
            from: format!("{:?}", hold.status),
            to: "UnderReview".into(),
        });
    }
    hold.audit_log.push(format!("review started at {at:?}"));
    hold.status = ContestStatus::UnderReview;
    Ok(())
}

/// Resolve a contest hold (challenge sustained or action reversed).
pub fn resolve_hold(
    hold: &mut ContestHold,
    at: Timestamp,
    outcome: &str,
) -> Result<(), EscalationError> {
    match hold.status {
        ContestStatus::PauseEligible | ContestStatus::UnderReview => {
            hold.audit_log
                .push(format!("resolved at {at:?}: {outcome}"));
            hold.status = ContestStatus::Resolved;
            Ok(())
        }
        _ => Err(EscalationError::InvalidStateTransition {
            from: format!("{:?}", hold.status),
            to: "Resolved".into(),
        }),
    }
}

/// Dismiss a contest hold (insufficient grounds; action unblocked).
pub fn dismiss_hold(
    hold: &mut ContestHold,
    at: Timestamp,
    reason: &str,
) -> Result<(), EscalationError> {
    match hold.status {
        ContestStatus::PauseEligible | ContestStatus::UnderReview => {
            hold.audit_log
                .push(format!("dismissed at {at:?}: {reason}"));
            hold.status = ContestStatus::Dismissed;
            Ok(())
        }
        _ => Err(EscalationError::InvalidStateTransition {
            from: format!("{:?}", hold.status),
            to: "Dismissed".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn action_id() -> [u8; 32] {
        [7u8; 32]
    }
    fn ts(ms: u64) -> Timestamp {
        Timestamp::new(ms, 0)
    }

    #[test]
    fn admit_creates_pause_eligible_hold() {
        let hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::ConcealedCommonControl,
            ts(1000),
        );
        assert_eq!(hold.status, ContestStatus::PauseEligible);
        assert_eq!(hold.ground, SybilChallengeGround::ConcealedCommonControl);
        assert_eq!(hold.action_id, action_id());
        assert!(!hold.audit_log.is_empty());
    }

    #[test]
    fn escalation_reason_contains_ground() {
        let hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::CoordinatedManipulation,
            ts(1000),
        );
        let reason = hold.escalation_reason();
        assert!(reason.contains("CoordinatedManipulation"));
        assert!(reason.contains("SybilChallenge"));
    }

    #[test]
    fn begin_review_transitions_from_pause_eligible() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::QuorumContamination,
            ts(1000),
        );
        assert!(begin_review(&mut hold, ts(2000)).is_ok());
        assert_eq!(hold.status, ContestStatus::UnderReview);
        assert_eq!(hold.audit_log.len(), 2);
    }

    #[test]
    fn begin_review_fails_if_not_pause_eligible() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::QuorumContamination,
            ts(1000),
        );
        hold.status = ContestStatus::Resolved;
        assert!(begin_review(&mut hold, ts(2000)).is_err());
    }

    #[test]
    fn resolve_hold_from_pause_eligible() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::SyntheticHumanMisrepresentation,
            ts(1000),
        );
        assert!(resolve_hold(&mut hold, ts(3000), "challenge sustained").is_ok());
        assert_eq!(hold.status, ContestStatus::Resolved);
    }

    #[test]
    fn resolve_hold_from_under_review() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::ConcealedCommonControl,
            ts(1000),
        );
        begin_review(&mut hold, ts(2000)).unwrap();
        assert!(resolve_hold(&mut hold, ts(3000), "action reversed").is_ok());
        assert_eq!(hold.status, ContestStatus::Resolved);
    }

    #[test]
    fn dismiss_hold_unblocks_action() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::CoordinatedManipulation,
            ts(1000),
        );
        assert!(dismiss_hold(&mut hold, ts(2000), "insufficient evidence").is_ok());
        assert_eq!(hold.status, ContestStatus::Dismissed);
    }

    #[test]
    fn dismiss_after_resolved_fails() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::QuorumContamination,
            ts(1000),
        );
        resolve_hold(&mut hold, ts(2000), "done").unwrap();
        assert!(dismiss_hold(&mut hold, ts(3000), "late").is_err());
    }

    #[test]
    fn all_four_grounds_admissible() {
        for ground in [
            SybilChallengeGround::ConcealedCommonControl,
            SybilChallengeGround::CoordinatedManipulation,
            SybilChallengeGround::QuorumContamination,
            SybilChallengeGround::SyntheticHumanMisrepresentation,
        ] {
            let hold = admit_challenge(&action_id(), ground.clone(), ts(1000));
            assert_eq!(hold.status, ContestStatus::PauseEligible);
        }
    }

    #[test]
    fn audit_log_grows_with_transitions() {
        let mut hold = admit_challenge(
            &action_id(),
            SybilChallengeGround::ConcealedCommonControl,
            ts(1000),
        );
        assert_eq!(hold.audit_log.len(), 1);
        begin_review(&mut hold, ts(2000)).unwrap();
        assert_eq!(hold.audit_log.len(), 2);
        resolve_hold(&mut hold, ts(3000), "confirmed").unwrap();
        assert_eq!(hold.audit_log.len(), 3);
    }

    #[test]
    fn ground_display() {
        assert_eq!(
            SybilChallengeGround::ConcealedCommonControl.to_string(),
            "ConcealedCommonControl"
        );
        assert_eq!(
            SybilChallengeGround::SyntheticHumanMisrepresentation.to_string(),
            "SyntheticHumanMisrepresentation"
        );
    }
}
