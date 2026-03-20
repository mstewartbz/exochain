//! Emergency action protocol (GOV-009).
//!
//! Emergency actions with EMERGENCY_AUTHORITY delegation, monetary caps,
//! enumerated actions, auto-created RATIFICATION_REQUIRED, and frequency
//! monitoring (>3/quarter triggers governance review).

use exo_core::types::{Did, Hash256, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ForumError, Result};

/// An enumerated emergency action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmergencyActionType {
    SystemHalt,
    AccessRevocation,
    DataFreeze,
    EmergencyPatch,
    RoleEscalation,
}

/// Status of an emergency action's ratification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RatificationStatus {
    Required,
    Ratified,
    Expired,
}

/// An emergency action record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyAction {
    pub id: Uuid,
    pub action_type: EmergencyActionType,
    pub actor_did: Did,
    pub justification: String,
    pub monetary_cap_cents: u64,
    pub actual_cost_cents: u64,
    pub created_at: Timestamp,
    pub ratification_status: RatificationStatus,
    pub ratification_deadline: Timestamp,
    pub ratification_decision_id: Option<Uuid>,
    pub evidence_hash: Hash256,
}

/// Policy governing emergency actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyPolicy {
    /// Maximum monetary cap per emergency action (in cents).
    pub max_monetary_cap_cents: u64,
    /// Allowed emergency action types.
    pub allowed_actions: Vec<EmergencyActionType>,
    /// Ratification deadline offset in milliseconds from creation.
    pub ratification_window_ms: u64,
    /// Maximum emergencies per quarter before governance review is triggered.
    pub max_per_quarter: usize,
}

impl Default for EmergencyPolicy {
    fn default() -> Self {
        Self {
            max_monetary_cap_cents: 10_000_000, // $100,000
            allowed_actions: vec![
                EmergencyActionType::SystemHalt,
                EmergencyActionType::AccessRevocation,
                EmergencyActionType::DataFreeze,
                EmergencyActionType::EmergencyPatch,
                EmergencyActionType::RoleEscalation,
            ],
            ratification_window_ms: 72 * 60 * 60 * 1000, // 72 hours
            max_per_quarter: 3,
        }
    }
}

/// Create an emergency action, validating against the policy.
pub fn create_emergency_action(
    action_type: EmergencyActionType,
    actor: &Did,
    justification: &str,
    monetary_cap_cents: u64,
    evidence_hash: Hash256,
    policy: &EmergencyPolicy,
    timestamp: Timestamp,
) -> Result<EmergencyAction> {
    if !policy.allowed_actions.contains(&action_type) {
        return Err(ForumError::EmergencyInvalid {
            reason: format!("{action_type:?} not in allowed actions"),
        });
    }
    if monetary_cap_cents > policy.max_monetary_cap_cents {
        return Err(ForumError::EmergencyCapExceeded {
            reason: format!(
                "cap {monetary_cap_cents} exceeds policy max {}",
                policy.max_monetary_cap_cents
            ),
        });
    }

    let deadline_ms = timestamp
        .physical_ms
        .saturating_add(policy.ratification_window_ms);
    Ok(EmergencyAction {
        id: Uuid::new_v4(),
        action_type,
        actor_did: actor.clone(),
        justification: justification.to_owned(),
        monetary_cap_cents,
        actual_cost_cents: 0,
        created_at: timestamp,
        ratification_status: RatificationStatus::Required,
        ratification_deadline: Timestamp::new(deadline_ms, 0),
        ratification_decision_id: None,
        evidence_hash,
    })
}

/// Ratify an emergency action.
pub fn ratify_emergency(
    action: &mut EmergencyAction,
    decision_id: Uuid,
    timestamp: Timestamp,
) -> Result<()> {
    if action.ratification_status != RatificationStatus::Required {
        return Err(ForumError::EmergencyInvalid {
            reason: format!("cannot ratify from status {:?}", action.ratification_status),
        });
    }
    if timestamp > action.ratification_deadline {
        action.ratification_status = RatificationStatus::Expired;
        return Err(ForumError::EmergencyInvalid {
            reason: "ratification deadline passed".into(),
        });
    }
    action.ratification_status = RatificationStatus::Ratified;
    action.ratification_decision_id = Some(decision_id);
    Ok(())
}

/// Check and expire unratified emergency actions.
pub fn check_expiry(action: &mut EmergencyAction, now: &Timestamp) -> bool {
    if action.ratification_status == RatificationStatus::Required
        && *now > action.ratification_deadline
    {
        action.ratification_status = RatificationStatus::Expired;
        true
    } else {
        false
    }
}

/// Check if the frequency threshold is exceeded (>3/quarter triggers review).
#[must_use]
pub fn needs_governance_review(actions: &[EmergencyAction], policy: &EmergencyPolicy) -> bool {
    // Count non-expired actions (we treat a "quarter" as all provided actions).
    let active_count = actions
        .iter()
        .filter(|a| a.ratification_status != RatificationStatus::Expired)
        .count();
    active_count > policy.max_per_quarter
}

#[cfg(test)]
mod tests {
    use super::*;

    fn did() -> Did {
        Did::new("did:exo:emergency-officer").expect("ok")
    }
    fn ts() -> Timestamp {
        Timestamp::new(1_000_000, 0)
    }
    fn policy() -> EmergencyPolicy {
        EmergencyPolicy::default()
    }

    #[test]
    fn create_valid_action() {
        let a = create_emergency_action(
            EmergencyActionType::SystemHalt,
            &did(),
            "critical outage",
            5_000_000,
            Hash256::digest(b"evidence"),
            &policy(),
            ts(),
        )
        .expect("ok");
        assert_eq!(a.ratification_status, RatificationStatus::Required);
        assert_eq!(a.action_type, EmergencyActionType::SystemHalt);
    }

    #[test]
    fn cap_exceeded() {
        let err = create_emergency_action(
            EmergencyActionType::SystemHalt,
            &did(),
            "too expensive",
            99_999_999,
            Hash256::ZERO,
            &policy(),
            ts(),
        )
        .unwrap_err();
        assert!(matches!(err, ForumError::EmergencyCapExceeded { .. }));
    }

    #[test]
    fn disallowed_action() {
        let p = EmergencyPolicy {
            allowed_actions: vec![EmergencyActionType::SystemHalt],
            ..policy()
        };
        let err = create_emergency_action(
            EmergencyActionType::RoleEscalation,
            &did(),
            "not allowed",
            0,
            Hash256::ZERO,
            &p,
            ts(),
        )
        .unwrap_err();
        assert!(matches!(err, ForumError::EmergencyInvalid { .. }));
    }

    #[test]
    fn ratify_ok() {
        let mut a = create_emergency_action(
            EmergencyActionType::DataFreeze,
            &did(),
            "breach",
            0,
            Hash256::ZERO,
            &policy(),
            ts(),
        )
        .expect("ok");
        let ratify_ts = Timestamp::new(ts().physical_ms + 1000, 0);
        ratify_emergency(&mut a, Uuid::new_v4(), ratify_ts).expect("ok");
        assert_eq!(a.ratification_status, RatificationStatus::Ratified);
        assert!(a.ratification_decision_id.is_some());
    }

    #[test]
    fn ratify_expired() {
        let mut a = create_emergency_action(
            EmergencyActionType::DataFreeze,
            &did(),
            "breach",
            0,
            Hash256::ZERO,
            &policy(),
            ts(),
        )
        .expect("ok");
        let late = Timestamp::new(a.ratification_deadline.physical_ms + 1000, 0);
        let err = ratify_emergency(&mut a, Uuid::new_v4(), late).unwrap_err();
        assert!(matches!(err, ForumError::EmergencyInvalid { .. }));
        assert_eq!(a.ratification_status, RatificationStatus::Expired);
    }

    #[test]
    fn check_expiry_marks_expired() {
        let mut a = create_emergency_action(
            EmergencyActionType::SystemHalt,
            &did(),
            "test",
            0,
            Hash256::ZERO,
            &policy(),
            ts(),
        )
        .expect("ok");
        let before = Timestamp::new(a.ratification_deadline.physical_ms - 1000, 0);
        assert!(!check_expiry(&mut a, &before));
        let after = Timestamp::new(a.ratification_deadline.physical_ms + 1000, 0);
        assert!(check_expiry(&mut a, &after));
        assert_eq!(a.ratification_status, RatificationStatus::Expired);
    }

    #[test]
    fn frequency_monitoring() {
        let p = policy();
        let actions: Vec<EmergencyAction> = (0..4)
            .map(|_| {
                create_emergency_action(
                    EmergencyActionType::SystemHalt,
                    &did(),
                    "test",
                    0,
                    Hash256::ZERO,
                    &p,
                    ts(),
                )
                .expect("ok")
            })
            .collect();
        assert!(needs_governance_review(&actions, &p));
        assert!(!needs_governance_review(&actions[..3], &p));
    }

    #[test]
    fn double_ratify_fails() {
        let mut a = create_emergency_action(
            EmergencyActionType::SystemHalt,
            &did(),
            "test",
            0,
            Hash256::ZERO,
            &policy(),
            ts(),
        )
        .expect("ok");
        ratify_emergency(
            &mut a,
            Uuid::new_v4(),
            Timestamp::new(ts().physical_ms + 100, 0),
        )
        .expect("ok");
        assert!(
            ratify_emergency(
                &mut a,
                Uuid::new_v4(),
                Timestamp::new(ts().physical_ms + 200, 0)
            )
            .is_err()
        );
    }
}
