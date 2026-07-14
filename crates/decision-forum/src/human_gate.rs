// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Human oversight enforcement (GOV-007, TNC-02, TNC-09).
//!
//! Enforces that certain decision classes require human approval,
//! distinguishes verified human voters from declared actor metadata, blocks AI
//! from satisfying HUMAN_GATE_REQUIRED, and enforces AI delegation ceilings.

use std::collections::BTreeSet;

use exo_core::types::Did;
use serde::{Deserialize, Serialize};

use crate::{
    decision_object::{ActorKind, DecisionClass, DecisionObject, Vote},
    error::{ForumError, Result},
};

/// Policy defining which decision classes require human approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanGatePolicy {
    /// Decision classes that always require at least one human approval.
    pub human_required_classes: Vec<DecisionClass>,
    /// Maximum decision class an AI agent can approve without human co-sign.
    pub ai_ceiling: DecisionClass,
}

impl Default for HumanGatePolicy {
    fn default() -> Self {
        Self {
            human_required_classes: vec![DecisionClass::Strategic, DecisionClass::Constitutional],
            ai_ceiling: DecisionClass::Operational,
        }
    }
}

/// Check whether a decision requires human approval per the gate policy.
#[must_use]
pub fn requires_human_approval(policy: &HumanGatePolicy, class: DecisionClass) -> bool {
    policy.human_required_classes.contains(&class)
}

/// Check whether an AI actor's ceiling allows it to act on this decision class.
#[must_use]
pub fn ai_within_ceiling(policy: &HumanGatePolicy, class: DecisionClass) -> bool {
    class <= policy.ai_ceiling
}

/// Validate that a decision's votes satisfy the human gate policy.
/// Returns Ok(()) if the gate is satisfied, or an error if not.
pub fn enforce_human_gate(policy: &HumanGatePolicy, decision: &DecisionObject) -> Result<()> {
    enforce_human_gate_with_verified_humans(policy, decision, &BTreeSet::new())
}

/// Validate the human gate using a trusted set of externally verified human
/// voter DIDs. The set must come from an identity registry, credential
/// verifier, or runtime adapter that authenticated the DID as human; the
/// self-declared `Vote.actor_kind` field is never sufficient by itself.
pub fn enforce_human_gate_with_verified_humans(
    policy: &HumanGatePolicy,
    decision: &DecisionObject,
    verified_human_voters: &BTreeSet<Did>,
) -> Result<()> {
    let has_verified_human_vote = decision
        .votes
        .iter()
        .any(|vote| is_verified_human_vote(vote, verified_human_voters));

    // Check AI ceiling: if decision class exceeds AI ceiling, AI votes alone
    // are not sufficient.
    if decision.class > policy.ai_ceiling {
        let has_ai_vote = decision.votes.iter().any(is_ai_vote);
        if has_ai_vote && !has_verified_human_vote {
            return Err(ForumError::AiCeilingExceeded {
                reason: format!(
                    "{} exceeds AI ceiling {}",
                    decision.class.quorum_policy_key(),
                    policy.ai_ceiling.quorum_policy_key()
                ),
            });
        }
    }

    // Check human gate: classes requiring human approval must have at least
    // one human vote.
    if requires_human_approval(policy, decision.class) {
        let human_count = decision
            .votes
            .iter()
            .filter(|vote| is_verified_human_vote(vote, verified_human_voters))
            .count();
        if human_count == 0 {
            return Err(ForumError::HumanGateRequired);
        }
    }

    Ok(())
}

/// Determine if a vote was cast by a verified human actor.
///
/// This legacy helper has no trusted identity registry argument, so it fails
/// closed. Use [`is_verified_human_vote`] when a verified DID set is available.
#[must_use]
pub fn is_human_vote(vote: &Vote) -> bool {
    is_verified_human_vote(vote, &BTreeSet::new())
}

/// Determine if a vote declares a human actor kind without treating that
/// declaration as verification.
#[must_use]
pub fn is_declared_human_vote(vote: &Vote) -> bool {
    matches!(vote.actor_kind, ActorKind::Human)
}

/// Determine if a vote is both declared human and backed by a trusted human
/// identity record for the same voter DID.
#[must_use]
pub fn is_verified_human_vote(vote: &Vote, verified_human_voters: &BTreeSet<Did>) -> bool {
    matches!(vote.actor_kind, ActorKind::Human) && verified_human_voters.contains(&vote.voter_did)
}

/// Determine if a vote was cast by an AI agent.
#[must_use]
pub fn is_ai_vote(vote: &Vote) -> bool {
    matches!(vote.actor_kind, ActorKind::AiAgent { .. })
}

/// GitHub login for Executive Chairman / CTO (Bob Stewart).
pub const PRINCIPAL_BOB_GITHUB: &str = "bob-stewart";

/// GitHub login for co-principal (Max Stewart).
pub const PRINCIPAL_MAX_GITHUB: &str = "mstewartbz";

/// Locked two-person / two-system gate policy for irreversible presidential acts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TwoPersonGatePolicy {
    pub principal_a: Did,
    pub principal_b: Did,
    pub github_a: String,
    pub github_b: String,
}

impl TwoPersonGatePolicy {
    /// Construct the Bob (`bob-stewart`) + Max (`mstewartbz`) presidential gate.
    pub fn presidential() -> Result<Self> {
        Ok(Self {
            principal_a: Did::new("did:exo:principal:bob-stewart")
                .map_err(|e| ForumError::Core(e.to_string()))?,
            principal_b: Did::new("did:exo:principal:mstewartbz")
                .map_err(|e| ForumError::Core(e.to_string()))?,
            github_a: PRINCIPAL_BOB_GITHUB.into(),
            github_b: PRINCIPAL_MAX_GITHUB.into(),
        })
    }
}

fn verified_principal_approve(
    decision: &DecisionObject,
    verified_human_voters: &BTreeSet<Did>,
    principal: &Did,
) -> bool {
    decision.votes.iter().any(|vote| {
        &vote.voter_did == principal
            && matches!(vote.choice, crate::decision_object::VoteChoice::Approve)
            && is_verified_human_vote(vote, verified_human_voters)
    })
}

/// Enforce Bob+Max dual attestation for irreversible ratification.
///
/// Both principals must cast verified human `Approve` votes. Agent identities
/// and non-principal humans cannot satisfy either half of the gate.
pub fn enforce_two_person_ratification(
    decision: &DecisionObject,
    verified_human_voters: &BTreeSet<Did>,
    policy: &TwoPersonGatePolicy,
) -> Result<()> {
    if two_person_veto_present(decision, verified_human_voters, policy) {
        return Err(ForumError::TwoPersonGateRequired {
            reason: "principal veto present; ratification blocked".into(),
        });
    }

    let a_ok = verified_principal_approve(decision, verified_human_voters, &policy.principal_a);
    let b_ok = verified_principal_approve(decision, verified_human_voters, &policy.principal_b);

    if a_ok && b_ok {
        return Ok(());
    }

    if !a_ok && !b_ok {
        return Err(ForumError::TwoPersonGateRequired {
            reason: format!(
                "missing attestations from {} and {}",
                policy.github_a, policy.github_b
            ),
        });
    }
    if !a_ok {
        return Err(ForumError::TwoPersonGateRequired {
            reason: format!("missing attestation from {}", policy.github_a),
        });
    }
    Err(ForumError::TwoPersonGateRequired {
        reason: format!("missing attestation from {}", policy.github_b),
    })
}

/// Returns true when either locked principal has cast a verified `Reject`.
#[must_use]
pub fn two_person_veto_present(
    decision: &DecisionObject,
    verified_human_voters: &BTreeSet<Did>,
    policy: &TwoPersonGatePolicy,
) -> bool {
    decision.votes.iter().any(|vote| {
        (vote.voter_did == policy.principal_a || vote.voter_did == policy.principal_b)
            && matches!(vote.choice, crate::decision_object::VoteChoice::Reject)
            && is_verified_human_vote(vote, verified_human_voters)
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use exo_core::{
        hlc::HybridClock,
        types::{Did, Hash256},
    };

    use super::*;
    use crate::decision_object::{DecisionObjectInput, VoteChoice};

    fn test_clock() -> HybridClock {
        let counter = AtomicU64::new(1000);
        HybridClock::with_wall_clock(move || counter.fetch_add(1, Ordering::Relaxed))
    }

    fn human_vote(clock: &mut HybridClock) -> Vote {
        Vote {
            voter_did: Did::new("did:exo:human-alice").expect("ok"),
            choice: VoteChoice::Approve,
            actor_kind: ActorKind::Human,
            timestamp: clock.now().expect("HLC timestamp"),
            signature_hash: Hash256::digest(b"human-sig"),
        }
    }

    fn ai_vote(clock: &mut HybridClock) -> Vote {
        Vote {
            voter_did: Did::new("did:exo:ai-agent-1").expect("ok"),
            choice: VoteChoice::Approve,
            actor_kind: ActorKind::AiAgent {
                delegation_id: "d1".into(),
                ceiling_class: DecisionClass::Operational,
            },
            timestamp: clock.now().expect("HLC timestamp"),
            signature_hash: Hash256::digest(b"ai-sig"),
        }
    }

    fn make_decision(class: DecisionClass, clock: &mut HybridClock) -> DecisionObject {
        DecisionObject::new(DecisionObjectInput {
            id: uuid::Uuid::from_u128(100),
            title: "test".into(),
            class,
            constitutional_hash: Hash256::digest(b"constitution"),
            created_at: clock.now().expect("HLC timestamp"),
        })
        .expect("valid decision")
    }

    #[test]
    fn routine_passes_without_human() {
        let mut clock = test_clock();
        let policy = HumanGatePolicy::default();
        let mut d = make_decision(DecisionClass::Routine, &mut clock);
        d.add_vote(ai_vote(&mut clock)).expect("ok");
        assert!(enforce_human_gate(&policy, &d).is_ok());
    }

    #[test]
    fn strategic_requires_human() {
        let mut clock = test_clock();
        let policy = HumanGatePolicy::default();
        let mut d = make_decision(DecisionClass::Strategic, &mut clock);
        d.add_vote(ai_vote(&mut clock)).expect("ok");
        let err = enforce_human_gate(&policy, &d).unwrap_err();
        assert_eq!(
            err.to_string(),
            "AI delegation ceiling exceeded: Strategic exceeds AI ceiling Operational"
        );
        assert!(matches!(
            err,
            ForumError::HumanGateRequired | ForumError::AiCeilingExceeded { .. }
        ));
    }

    #[test]
    fn human_gate_errors_do_not_depend_on_debug_formatting() {
        let production = include_str!("human_gate.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production section");
        assert!(
            !production.contains("{:?} exceeds AI ceiling {:?}"),
            "human-gate ceiling errors must use explicit stable class labels"
        );
    }

    #[test]
    fn strategic_passes_with_human() {
        let mut clock = test_clock();
        let policy = HumanGatePolicy::default();
        let mut d = make_decision(DecisionClass::Strategic, &mut clock);
        let vote = human_vote(&mut clock);
        let mut verified_human_voters = BTreeSet::new();
        verified_human_voters.insert(vote.voter_did.clone());
        d.add_vote(vote).expect("ok");
        assert!(
            enforce_human_gate_with_verified_humans(&policy, &d, &verified_human_voters).is_ok()
        );
    }

    #[test]
    fn human_gate_rejects_unverified_human_actor_kind() {
        let mut clock = test_clock();
        let policy = HumanGatePolicy::default();
        let mut d = make_decision(DecisionClass::Strategic, &mut clock);
        d.add_vote(human_vote(&mut clock))
            .expect("declared human vote");

        let err = enforce_human_gate(&policy, &d)
            .expect_err("self-declared human actor_kind must not satisfy human gate");

        assert!(matches!(err, ForumError::HumanGateRequired));
    }

    #[test]
    fn constitutional_requires_human() {
        let mut clock = test_clock();
        let policy = HumanGatePolicy::default();
        let mut d = make_decision(DecisionClass::Constitutional, &mut clock);
        d.add_vote(ai_vote(&mut clock)).expect("ok");
        assert!(enforce_human_gate(&policy, &d).is_err());
    }

    #[test]
    fn empty_votes_passes_gate() {
        // No votes yet — gate doesn't block (nothing to validate).
        let mut clock = test_clock();
        let policy = HumanGatePolicy::default();
        let d = make_decision(DecisionClass::Strategic, &mut clock);
        // With no votes, the human gate check for human_required_classes fails
        // because human_count == 0, but we allow empty votes since no approval is claimed.
        let result = enforce_human_gate(&policy, &d);
        // Empty votes: human_count == 0, but no one is asserting approval.
        // This should fail — decisions requiring human approval need human votes.
        assert!(result.is_err());
    }

    #[test]
    fn ai_ceiling_check() {
        let policy = HumanGatePolicy::default();
        assert!(ai_within_ceiling(&policy, DecisionClass::Routine));
        assert!(ai_within_ceiling(&policy, DecisionClass::Operational));
        assert!(!ai_within_ceiling(&policy, DecisionClass::Strategic));
        assert!(!ai_within_ceiling(&policy, DecisionClass::Constitutional));
    }

    #[test]
    fn is_human_vs_ai() {
        let mut clock = test_clock();
        let human = human_vote(&mut clock);
        let mut verified_human_voters = BTreeSet::new();
        verified_human_voters.insert(human.voter_did.clone());

        assert!(is_declared_human_vote(&human));
        assert!(!is_human_vote(&human));
        assert!(is_verified_human_vote(&human, &verified_human_voters));
        assert!(!is_declared_human_vote(&ai_vote(&mut clock)));
        assert!(is_ai_vote(&ai_vote(&mut clock)));
        assert!(!is_ai_vote(&human));
    }

    #[test]
    fn default_policy() {
        let p = HumanGatePolicy::default();
        assert_eq!(p.ai_ceiling, DecisionClass::Operational);
        assert!(p.human_required_classes.contains(&DecisionClass::Strategic));
        assert!(
            p.human_required_classes
                .contains(&DecisionClass::Constitutional)
        );
    }

    fn principal_vote(did: Did, choice: VoteChoice, clock: &mut HybridClock) -> Vote {
        Vote {
            voter_did: did,
            choice,
            actor_kind: ActorKind::Human,
            timestamp: clock.now().expect("HLC timestamp"),
            signature_hash: Hash256::digest(b"principal-sig"),
        }
    }

    #[test]
    fn two_person_ratify_requires_bob_and_max() {
        let mut clock = test_clock();
        let policy = TwoPersonGatePolicy::presidential().expect("policy");
        let mut d = make_decision(DecisionClass::Constitutional, &mut clock);
        let mut verified = BTreeSet::new();
        verified.insert(policy.principal_a.clone());
        verified.insert(policy.principal_b.clone());

        d.add_vote(principal_vote(
            policy.principal_a.clone(),
            VoteChoice::Approve,
            &mut clock,
        ))
        .expect("bob vote");
        let only_bob =
            enforce_two_person_ratification(&d, &verified, &policy).expect_err("Max required");
        assert!(matches!(only_bob, ForumError::TwoPersonGateRequired { .. }));
        assert!(only_bob.to_string().contains("mstewartbz"));

        d.add_vote(principal_vote(
            policy.principal_b.clone(),
            VoteChoice::Approve,
            &mut clock,
        ))
        .expect("max vote");
        assert!(enforce_two_person_ratification(&d, &verified, &policy).is_ok());
    }

    #[test]
    fn two_person_ratify_fails_with_only_max() {
        let mut clock = test_clock();
        let policy = TwoPersonGatePolicy::presidential().expect("policy");
        let mut d = make_decision(DecisionClass::Constitutional, &mut clock);
        let mut verified = BTreeSet::new();
        verified.insert(policy.principal_b.clone());
        d.add_vote(principal_vote(
            policy.principal_b.clone(),
            VoteChoice::Approve,
            &mut clock,
        ))
        .expect("max vote");
        let err = enforce_two_person_ratification(&d, &verified, &policy).unwrap_err();
        assert!(err.to_string().contains("bob-stewart"));
    }

    #[test]
    fn two_person_gate_rejects_agent_as_principal_half() {
        let mut clock = test_clock();
        let policy = TwoPersonGatePolicy::presidential().expect("policy");
        let mut d = make_decision(DecisionClass::Constitutional, &mut clock);
        let mut verified = BTreeSet::new();
        verified.insert(policy.principal_a.clone());
        // Agent attempts to vote using Max's DID — actor_kind AI fails verification.
        d.add_vote(Vote {
            voter_did: policy.principal_b.clone(),
            choice: VoteChoice::Approve,
            actor_kind: ActorKind::AiAgent {
                delegation_id: "agent".into(),
                ceiling_class: DecisionClass::Operational,
            },
            timestamp: clock.now().expect("HLC"),
            signature_hash: Hash256::digest(b"ai"),
        })
        .expect("ai vote");
        d.add_vote(principal_vote(
            policy.principal_a.clone(),
            VoteChoice::Approve,
            &mut clock,
        ))
        .expect("bob");
        verified.insert(policy.principal_b.clone());
        let err = enforce_two_person_ratification(&d, &verified, &policy).unwrap_err();
        assert!(matches!(err, ForumError::TwoPersonGateRequired { .. }));
    }

    #[test]
    fn either_principal_veto_blocks_ratification() {
        let mut clock = test_clock();
        let policy = TwoPersonGatePolicy::presidential().expect("policy");
        let mut d = make_decision(DecisionClass::Constitutional, &mut clock);
        let mut verified = BTreeSet::new();
        verified.insert(policy.principal_a.clone());
        verified.insert(policy.principal_b.clone());
        d.add_vote(principal_vote(
            policy.principal_a.clone(),
            VoteChoice::Approve,
            &mut clock,
        ))
        .expect("bob approve");
        d.add_vote(principal_vote(
            policy.principal_b.clone(),
            VoteChoice::Reject,
            &mut clock,
        ))
        .expect("max veto");
        assert!(two_person_veto_present(&d, &verified, &policy));
        assert!(enforce_two_person_ratification(&d, &verified, &policy).is_err());
    }

    #[test]
    fn presidential_policy_locks_github_identities() {
        let policy = TwoPersonGatePolicy::presidential().expect("policy");
        assert_eq!(policy.github_a, "bob-stewart");
        assert_eq!(policy.github_b, "mstewartbz");
    }
}
