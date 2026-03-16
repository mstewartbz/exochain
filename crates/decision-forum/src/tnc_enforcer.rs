use super::decision_object::{DecisionClass, DecisionObject, Status};
use crate::authority::ActorKind;
use std::collections::HashSet;

const MIN_KEY_MATERIAL_LEN: usize = 8;
const MAX_AI_SIGNER_RATIO: f64 = 0.49;
use super::decision_object::{DecisionClass, DecisionObject, SignerType, Status, VoteChoice};

/// All 10 Trust-Critical Non-Negotiable Controls enforced here.
pub struct TNCEnforcer;

const MAX_AUTHORITY_CHAIN_DEPTH: usize = 5;

impl TNCEnforcer {
    pub fn enforce_all(obj: &DecisionObject) -> Result<(), String> {
        Self::tnc01_authority_chain(obj)?;
        Self::tnc02_human_gate(obj)?;
        Self::tnc03_audit_continuity(obj)?;
        Self::tnc04_sync_constraints(obj)?;
        Self::tnc05_delegation_expiry(obj)?;
        Self::tnc06_conflict_disclosure(obj)?;
        Self::tnc07_quorum(obj)?;
        Self::tnc08_immutability(obj)?;
        Self::tnc09_ai_ceiling(obj)?;
        Self::tnc10_ratification(obj)?;
        Ok(())
    }

    fn tnc01_authority_chain(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc01AuthorityChain.mark_covered();

        for (index, link) in obj.authority_chain.iter().enumerate() {
            let pubkey = link.pubkey.trim();
            let signature = link.signature.trim();

            if pubkey.is_empty() {
                return Err(format!(
                    "TNC-01 violated: Authority link [{}] has empty pubkey",
                    index
                ));
            }
            if pubkey.len() < MIN_KEY_MATERIAL_LEN {
                return Err(format!(
                    "TNC-01 violated: Authority link [{}] pubkey too short (min {} chars)",
                    index, MIN_KEY_MATERIAL_LEN
                ));
            }
            if signature.is_empty() {
                return Err(format!(
                    "TNC-01 violated: Authority link [{}] has empty signature",
                    index
                ));
            }
            if signature.len() < MIN_KEY_MATERIAL_LEN {
                return Err(format!(
                    "TNC-01 violated: Authority link [{}] signature too short (min {} chars)",
                    index, MIN_KEY_MATERIAL_LEN
                ));
            }
        }

        Ok(())
    }

    fn tnc02_human_gate(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc02HumanGate.mark_covered();

        let requires_human_gate = obj.advanced_reasoning.is_some()
            || matches!(
                obj.decision_class,
                DecisionClass::Policy | DecisionClass::Sovereignty
            );

        if requires_human_gate && !obj.human_review.is_satisfied() {
            return Err(
                "TNC-02 violated: decision requires completed human review before approval".into(),
            );
        }

        Ok(())
    }

    fn tnc03_audit_continuity(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc03AuditContinuity.mark_covered();

        match obj.status {
            Status::Approved | Status::Rejected | Status::Void => {
                if obj.audit_log.is_empty() {
                    return Err(
                        "TNC-03 violated: terminal status requires non-empty audit log".into(),
                    );
                }
                for window in obj.audit_log.windows(2) {
                    if window[1].timestamp < window[0].timestamp {
                        return Err(
                            "TNC-03 violated: audit log is not chronologically ordered".into()
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn tnc04_sync_constraints(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc04SyncConstraints.mark_covered();

        if let Some(expected) = obj.expected_sync_version {
            if expected != obj.sync_version {
                return Err(format!(
                    "TNC-04 violated: sync version mismatch (expected {}, actual {})",
                    expected, obj.sync_version
                ));
            }
        }

        Ok(())
    }

    fn tnc05_delegation_expiry(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc05DelegationExpiry.mark_covered();

        let now = chrono::Utc::now();
        for (index, link) in obj.authority_chain.iter().enumerate() {
            if let Some(expires_at) = link.expires_at {
                if now > expires_at {
                    return Err(format!(
                        "TNC-05 violated: Authority link [{}] expired at {}",
                        index, expires_at
                    ));
                }
            }
        }

        Ok(())
    }

    fn tnc06_conflict_disclosure(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc06ConflictDisclosure.mark_covered();

        for (index, link) in obj.authority_chain.iter().enumerate() {
            match &link.conflict_disclosure {
                None => {
                    return Err(format!(
                        "TNC-06 violated: Authority link [{}] is missing conflict disclosure",
                        index
                    ));
                }
                Some(disclosure) if disclosure.has_conflict && disclosure.description.is_none() => {
                    return Err(format!(
                        "TNC-06 violated: Authority link [{}] declared conflict without description",
                        index
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn tnc07_quorum(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc07Quorum.mark_covered();

        if obj.authority_chain.len() < obj.required_quorum {
            return Err(format!(
                "TNC-07 violated: quorum not met (required {}, got {})",
                obj.required_quorum,
                obj.authority_chain.len()
            ));
        }

        let mut seen = HashSet::new();
        for (index, link) in obj.authority_chain.iter().enumerate() {
            if !seen.insert(link.pubkey.as_str()) {
                return Err(format!(
                    "TNC-07 violated: duplicate signer detected at link [{}]",
                    index
                ));
            }
        }

        Ok(())
    }

    fn tnc08_immutability(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc08Immutability.mark_covered();

        if obj.status == Status::Approved && obj.authority_chain.is_empty() {
            Err("TNC-08 violated: terminal status without immutable authority record".into())
        } else {
            Ok(())
    /// TNC-01: Authority chain must be non-empty, all links must have non-empty
    /// pubkey and signature, and chain length must not exceed MAX_AUTHORITY_CHAIN_DEPTH.
    fn tnc01_authority_chain(obj: &DecisionObject) -> Result<(), String> {
        if obj.authority_chain.is_empty() {
            return Err("TNC-01 violated: authority chain is empty".into());
        }
        if obj.authority_chain.len() > MAX_AUTHORITY_CHAIN_DEPTH {
            return Err(format!(
                "TNC-01 violated: authority chain length {} exceeds max depth {}",
                obj.authority_chain.len(),
                MAX_AUTHORITY_CHAIN_DEPTH
            ));
        }
        for (i, link) in obj.authority_chain.iter().enumerate() {
            if link.pubkey.is_empty() {
                return Err(format!(
                    "TNC-01 violated: authority link {} has empty pubkey",
                    i
                ));
            }
            if link.signature.is_empty() {
                return Err(format!(
                    "TNC-01 violated: authority link {} has empty signature",
                    i
                ));
            }
        }
        Ok(())
    }

    /// TNC-02: Strategic and Constitutional decisions require a Human signer.
    fn tnc02_human_gate(obj: &DecisionObject) -> Result<(), String> {
        match obj.decision_class {
            DecisionClass::Strategic | DecisionClass::Constitutional => {
                if let SignerType::AiAgent { .. } = obj.signer_type {
                    return Err(format!(
                        "TNC-02 violated: {:?} decisions require a human signer, got AI agent",
                        obj.decision_class
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// TNC-03: Audit sequence must be > 0 and prev_audit_hash must be non-empty,
    /// ensuring a continuous audit hash chain.
    fn tnc03_audit_continuity(obj: &DecisionObject) -> Result<(), String> {
        if obj.audit_sequence == 0 {
            return Err("TNC-03 violated: audit_sequence must be > 0".into());
        }
        if obj.prev_audit_hash.is_empty() {
            return Err("TNC-03 violated: prev_audit_hash is empty (broken audit chain)".into());
        }
        Ok(())
    }

    /// TNC-04: Constitution binding — constitution_hash and constitution_version
    /// must both be non-empty.
    fn tnc04_sync_constraints(obj: &DecisionObject) -> Result<(), String> {
        if obj.constitution_hash.is_empty() {
            return Err("TNC-04 violated: constitution_hash is empty".into());
        }
        if obj.constitution_version.is_empty() {
            return Err("TNC-04 violated: constitution_version is empty".into());
        }
        Ok(())
    }

    /// TNC-05: Every delegation in the chain must have an expiry in the future
    /// relative to the decision's created_at timestamp.
    fn tnc05_delegation_expiry(obj: &DecisionObject) -> Result<(), String> {
        for (i, d) in obj.delegation_chain.iter().enumerate() {
            if d.expires_at <= obj.created_at {
                return Err(format!(
                    "TNC-05 violated: delegation {} (delegator={}) expired at {} before decision created at {}",
                    i, d.delegator, d.expires_at, obj.created_at
                ));
            }
        }
        Ok(())
    }

    /// TNC-06: For Operational, Strategic, and Constitutional decisions,
    /// at least one conflict disclosure must be filed.
    fn tnc06_conflict_disclosure(obj: &DecisionObject) -> Result<(), String> {
        match obj.decision_class {
            DecisionClass::Operational
            | DecisionClass::Strategic
            | DecisionClass::Constitutional => {
                if obj.conflicts_disclosed.is_empty() {
                    return Err(format!(
                        "TNC-06 violated: {:?} decisions require at least one conflict disclosure",
                        obj.decision_class
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// TNC-07: For terminal statuses (Approved/Rejected), verify vote count
    /// meets quorum and approval percentage meets threshold.
    fn tnc07_quorum(obj: &DecisionObject) -> Result<(), String> {
        match obj.status {
            Status::Approved | Status::Rejected => {
                let vote_count = obj.votes.len() as u32;
                if vote_count < obj.quorum_required {
                    return Err(format!(
                        "TNC-07 violated: {} votes cast but {} required for quorum",
                        vote_count, obj.quorum_required
                    ));
                }
                if obj.quorum_required > 0 && !obj.votes.is_empty() {
                    let approve_count = obj
                        .votes
                        .iter()
                        .filter(|v| v.choice == VoteChoice::Approve)
                        .count() as f64;
                    let total = obj.votes.len() as f64;
                    let pct = (approve_count / total) * 100.0;
                    if obj.status == Status::Approved && pct < obj.quorum_threshold_pct {
                        return Err(format!(
                            "TNC-07 violated: approval percentage {:.1}% is below threshold {:.1}%",
                            pct, obj.quorum_threshold_pct
                        ));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// TNC-08: Terminal statuses (Approved, Rejected, Void) require a non-empty
    /// merkle_root and at least one piece of evidence.
    fn tnc08_immutability(obj: &DecisionObject) -> Result<(), String> {
        match obj.status {
            Status::Approved | Status::Rejected | Status::Void => {
                if obj.merkle_root.is_empty() {
                    return Err(
                        "TNC-08 violated: terminal status without immutable merkle root".into(),
                    );
                }
                if obj.evidence.is_empty() {
                    return Err(
                        "TNC-08 violated: terminal status without evidence record".into(),
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// TNC-09: If the signer is an AI agent, the decision_class must not exceed
    /// the agent's ceiling_class.
    fn tnc09_ai_ceiling(obj: &DecisionObject) -> Result<(), String> {
        if let SignerType::AiAgent {
            ref ceiling_class, ..
        } = obj.signer_type
        {
            if obj.decision_class > *ceiling_class {
                return Err(format!(
                    "TNC-09 violated: decision class {:?} exceeds AI ceiling {:?}",
                    obj.decision_class, ceiling_class
                ));
            }
        }
        Ok(())
    }

    /// TNC-10: If ratification is required, a deadline must be set and be in the future.
    fn tnc10_ratification(obj: &DecisionObject) -> Result<(), String> {
        if obj.requires_ratification {
            match &obj.ratification_deadline {
                None => {
                    return Err(
                        "TNC-10 violated: ratification required but no deadline set".into(),
                    );
                }
                Some(deadline) => {
                    if *deadline <= chrono::Utc::now() {
                        return Err(format!(
                            "TNC-10 violated: ratification deadline {} has passed",
                            deadline
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority::AuthorityLink;
    use crate::decision_object::*;

    fn base_decision() -> DecisionObject {
        DecisionObject::new("Test Decision")
    }

    #[test]
    fn test_tnc01_empty_chain_fails() {
        let mut obj = base_decision();
        obj.authority_chain = vec![];
        let result = TNCEnforcer::tnc01_authority_chain(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TNC-01"));
    }

    #[test]
    fn test_tnc01_too_long_chain_fails() {
        let mut obj = base_decision();
        obj.authority_chain = (0..6)
            .map(|i| AuthorityLink {
                pubkey: format!("key-{}", i),
                signature: format!("sig-{}", i),
            })
            .collect();
        let result = TNCEnforcer::tnc01_authority_chain(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds max depth"));
    }

    #[test]
    fn test_tnc01_empty_pubkey_fails() {
        let mut obj = base_decision();
        obj.authority_chain = vec![AuthorityLink {
            pubkey: "".to_string(),
            signature: "sig".to_string(),
        }];
        let result = TNCEnforcer::tnc01_authority_chain(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty pubkey"));
    }

    #[test]
    fn test_tnc01_valid_chain_passes() {
        let obj = base_decision();
        assert!(TNCEnforcer::tnc01_authority_chain(&obj).is_ok());
    }

    #[test]
    fn test_tnc02_strategic_ai_fails() {
        let mut obj = base_decision();
        obj.decision_class = DecisionClass::Strategic;
        obj.signer_type = SignerType::AiAgent {
            delegation_id: "d1".into(),
            ceiling_class: DecisionClass::Strategic,
        };
        let result = TNCEnforcer::tnc02_human_gate(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TNC-02"));
    }

    #[test]
    fn test_tnc02_routine_ai_passes() {
        let mut obj = base_decision();
        obj.decision_class = DecisionClass::Routine;
        obj.signer_type = SignerType::AiAgent {
            delegation_id: "d1".into(),
            ceiling_class: DecisionClass::Routine,
        };
        assert!(TNCEnforcer::tnc02_human_gate(&obj).is_ok());
    }

    #[test]
    fn test_tnc03_zero_sequence_fails() {
        let mut obj = base_decision();
        obj.audit_sequence = 0;
        let result = TNCEnforcer::tnc03_audit_continuity(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc03_empty_prev_hash_fails() {
        let mut obj = base_decision();
        obj.prev_audit_hash = "".to_string();
        let result = TNCEnforcer::tnc03_audit_continuity(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc04_empty_constitution_hash_fails() {
        let mut obj = base_decision();
        obj.constitution_hash = "".to_string();
        let result = TNCEnforcer::tnc04_sync_constraints(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc04_empty_version_fails() {
        let mut obj = base_decision();
        obj.constitution_version = "".to_string();
        let result = TNCEnforcer::tnc04_sync_constraints(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc05_expired_delegation_fails() {
        let mut obj = base_decision();
        obj.delegation_chain = vec![DelegationRecord {
            delegator: "alice".into(),
            delegate: "bob".into(),
            scope: "all".into(),
            expires_at: obj.created_at - chrono::Duration::hours(1),
            allows_sub_delegation: false,
        }];
        let result = TNCEnforcer::tnc05_delegation_expiry(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TNC-05"));
    }

    #[test]
    fn test_tnc05_valid_delegation_passes() {
        let mut obj = base_decision();
        obj.delegation_chain = vec![DelegationRecord {
            delegator: "alice".into(),
            delegate: "bob".into(),
            scope: "all".into(),
            expires_at: obj.created_at + chrono::Duration::hours(24),
            allows_sub_delegation: false,
        }];
        assert!(TNCEnforcer::tnc05_delegation_expiry(&obj).is_ok());
    }

    #[test]
    fn test_tnc06_operational_no_disclosure_fails() {
        let mut obj = base_decision();
        obj.decision_class = DecisionClass::Operational;
        obj.conflicts_disclosed = vec![];
        let result = TNCEnforcer::tnc06_conflict_disclosure(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc06_routine_no_disclosure_passes() {
        let mut obj = base_decision();
        obj.decision_class = DecisionClass::Routine;
        obj.conflicts_disclosed = vec![];
        assert!(TNCEnforcer::tnc06_conflict_disclosure(&obj).is_ok());
    }

    #[test]
    fn test_tnc07_insufficient_votes_fails() {
        let mut obj = base_decision();
        obj.status = Status::Approved;
        obj.quorum_required = 3;
        obj.quorum_threshold_pct = 50.0;
        obj.votes = vec![Vote {
            voter_did: "did:ex:1".into(),
            choice: VoteChoice::Approve,
            signer_type: SignerType::Human,
        }];
        let result = TNCEnforcer::tnc07_quorum(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TNC-07"));
    }

    #[test]
    fn test_tnc07_below_threshold_fails() {
        let mut obj = base_decision();
        obj.status = Status::Approved;
        obj.quorum_required = 2;
        obj.quorum_threshold_pct = 75.0;
        obj.votes = vec![
            Vote { voter_did: "did:ex:1".into(), choice: VoteChoice::Approve, signer_type: SignerType::Human },
            Vote { voter_did: "did:ex:2".into(), choice: VoteChoice::Reject, signer_type: SignerType::Human },
        ];
        let result = TNCEnforcer::tnc07_quorum(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("approval percentage"));
    }

    #[test]
    fn test_tnc07_draft_skips_quorum() {
        let obj = base_decision();
        assert!(TNCEnforcer::tnc07_quorum(&obj).is_ok());
    }

    #[test]
    fn test_tnc08_approved_no_evidence_fails() {
        let mut obj = base_decision();
        obj.status = Status::Approved;
        obj.evidence = vec![];
        let result = TNCEnforcer::tnc08_immutability(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc08_draft_no_evidence_passes() {
        let mut obj = base_decision();
        obj.status = Status::Draft;
        obj.evidence = vec![];
        assert!(TNCEnforcer::tnc08_immutability(&obj).is_ok());
    }

    #[test]
    fn test_tnc09_ai_exceeds_ceiling_fails() {
        let mut obj = base_decision();
        obj.decision_class = DecisionClass::Strategic;
        obj.signer_type = SignerType::AiAgent {
            delegation_id: "d1".into(),
            ceiling_class: DecisionClass::Operational,
        };
        let result = TNCEnforcer::tnc09_ai_ceiling(&obj);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TNC-09"));
    }

    #[test]
    fn test_tnc09_ai_within_ceiling_passes() {
        let mut obj = base_decision();
        obj.decision_class = DecisionClass::Routine;
        obj.signer_type = SignerType::AiAgent {
            delegation_id: "d1".into(),
            ceiling_class: DecisionClass::Operational,
        };
        assert!(TNCEnforcer::tnc09_ai_ceiling(&obj).is_ok());
    }

    #[test]
    fn test_tnc10_ratification_required_no_deadline_fails() {
        let mut obj = base_decision();
        obj.requires_ratification = true;
        obj.ratification_deadline = None;
        let result = TNCEnforcer::tnc10_ratification(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_tnc10_ratification_not_required_passes() {
        let obj = base_decision();
        assert!(TNCEnforcer::tnc10_ratification(&obj).is_ok());
    }

    #[test]
    fn test_tnc10_ratification_future_deadline_passes() {
        let mut obj = base_decision();
        obj.requires_ratification = true;
        obj.ratification_deadline = Some(chrono::Utc::now() + chrono::Duration::days(30));
        assert!(TNCEnforcer::tnc10_ratification(&obj).is_ok());
    }

    #[test]
    fn test_enforce_all_passes_for_default() {
        let obj = base_decision();
        assert!(TNCEnforcer::enforce_all(&obj).is_ok());
    }

    fn tnc09_ai_ceiling(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc09AiCeiling.mark_covered();

        let total = obj.authority_chain.len();
        if total == 0 {
            return Ok(());
        }

        let ai_count = obj
            .authority_chain
            .iter()
            .filter(|link| link.actor_kind == ActorKind::Agent)
            .count();
        let ai_ratio = ai_count as f64 / total as f64;

        if ai_ratio > MAX_AI_SIGNER_RATIO {
            return Err(format!(
                "TNC-09 violated: AI ceiling exceeded ({}/{}, {:.0}% > {:.0}%)",
                ai_count,
                total,
                ai_ratio * 100.0,
                MAX_AI_SIGNER_RATIO * 100.0
            ));
        }

        Ok(())
    }

    fn tnc10_ratification(obj: &DecisionObject) -> Result<(), String> {
        #[cfg(test)]
        crate::requirements::Requirement::Tnc10Ratification.mark_covered();

        if matches!(
            obj.decision_class,
            DecisionClass::Policy | DecisionClass::Sovereignty
        ) && (obj.ratified_by.is_none() || obj.ratified_at.is_none())
        {
            return Err(
                "TNC-10 violated: policy/sovereignty decisions require explicit ratification"
                    .into(),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::advanced_policy::{AdvancedReasoningPolicy, BayesianAssessment, HumanReviewStatus};
    use crate::authority::{ActorKind, AuthorityLink, ConflictDisclosure};
    use crate::decision_object::{DecisionObject, Evidence};

    fn disclosure(has_conflict: bool, description: Option<&str>) -> ConflictDisclosure {
        ConflictDisclosure {
            has_conflict,
            description: description.map(str::to_string),
            disclosed_at: chrono::Utc::now(),
        }
    }

    fn link(pubkey: &str, signature: &str, actor_kind: ActorKind) -> AuthorityLink {
        AuthorityLink {
            pubkey: pubkey.to_string(),
            signature: signature.to_string(),
            actor_kind,
            expires_at: None,
            conflict_disclosure: Some(disclosure(false, None)),
        }
    }

    fn advanced_assessment() -> BayesianAssessment {
        BayesianAssessment {
            prior: 0.5,
            posterior: 0.9,
            confidence_interval: 0.90,
            sensitivity_instability: 0.05,
            teacher_student_disagreement: 0.01,
            symbolic_rule_trace_hash:
                "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            evidence_references: vec!["https://evidence.exochain.local/doc/1".to_string()],
        }
    }

    #[test]
    pub fn test_tnc01_authority_chain_validation() {
        let mut obj = DecisionObject::new("tnc01");
        obj.authority_chain
            .push(link("short", "valid-signature", ActorKind::Human));
        assert!(TNCEnforcer::tnc01_authority_chain(&obj).is_err());

        obj.authority_chain.clear();
        obj.authority_chain
            .push(link("valid-pubkey-0001", "sig", ActorKind::Human));
        assert!(TNCEnforcer::tnc01_authority_chain(&obj).is_err());

        obj.authority_chain.clear();
        obj.authority_chain.push(link(
            "valid-pubkey-0001",
            "valid-signature-0001",
            ActorKind::Human,
        ));
        assert!(TNCEnforcer::tnc01_authority_chain(&obj).is_ok());
    }

    #[test]
    pub fn test_tnc02_human_gate() {
        let mut obj = DecisionObject::new("tnc02");
        obj.authority_chain.push(link(
            "valid-pubkey-0002",
            "valid-signature-0002",
            ActorKind::Human,
        ));
        obj.advanced_reasoning = Some(AdvancedReasoningPolicy::new(advanced_assessment()));
        assert!(TNCEnforcer::tnc02_human_gate(&obj).is_err());

        obj.human_review = HumanReviewStatus::approved_by("council:alice");
        assert!(TNCEnforcer::tnc02_human_gate(&obj).is_ok());
    }

    #[test]
    pub fn test_tnc03_audit_continuity() {
        let mut obj = DecisionObject::new("tnc03");
        obj.status = Status::Approved;
        assert!(TNCEnforcer::tnc03_audit_continuity(&obj).is_err());

        obj.audit_log.push(crate::decision_object::AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: crate::decision_object::AuditEventType::SealAttempt,
            reason: "attempt".to_string(),
            escalation_reason: None,
            assessment_snapshot: None,
        });
        assert!(TNCEnforcer::tnc03_audit_continuity(&obj).is_ok());
    }

    #[test]
    pub fn test_tnc04_sync_constraints() {
        let mut obj = DecisionObject::new("tnc04");
        obj.sync_version = 1;
        obj.expected_sync_version = Some(1);
        assert!(TNCEnforcer::tnc04_sync_constraints(&obj).is_ok());
        obj.expected_sync_version = Some(2);
        assert!(TNCEnforcer::tnc04_sync_constraints(&obj).is_err());
    }

    #[test]
    pub fn test_tnc05_delegation_expiry() {
        let mut obj = DecisionObject::new("tnc05");
        let mut expired = link(
            "valid-pubkey-0005",
            "valid-signature-0005",
            ActorKind::Human,
        );
        expired.expires_at = Some(chrono::Utc::now() - chrono::Duration::days(1));
        obj.authority_chain.push(expired);
        assert!(TNCEnforcer::tnc05_delegation_expiry(&obj).is_err());
    }

    #[test]
    pub fn test_tnc06_conflict_disclosure() {
        let mut obj = DecisionObject::new("tnc06");
        let mut missing = link(
            "valid-pubkey-0006",
            "valid-signature-0006",
            ActorKind::Human,
        );
        missing.conflict_disclosure = None;
        obj.authority_chain.push(missing);
        assert!(TNCEnforcer::tnc06_conflict_disclosure(&obj).is_err());

        obj.authority_chain.clear();
        let mut conflicted = link(
            "valid-pubkey-0006",
            "valid-signature-0006",
            ActorKind::Human,
        );
        conflicted.conflict_disclosure = Some(disclosure(true, None));
        obj.authority_chain.push(conflicted);
        assert!(TNCEnforcer::tnc06_conflict_disclosure(&obj).is_err());
    }

    #[test]
    pub fn test_tnc07_quorum() {
        let mut obj = DecisionObject::new("tnc07");
        obj.required_quorum = 2;
        obj.authority_chain.push(link(
            "valid-pubkey-0007",
            "valid-signature-0007",
            ActorKind::Human,
        ));
        assert!(TNCEnforcer::tnc07_quorum(&obj).is_err());

        obj.authority_chain.push(link(
            "valid-pubkey-0008",
            "valid-signature-0008",
            ActorKind::Human,
        ));
        assert!(TNCEnforcer::tnc07_quorum(&obj).is_ok());
    }

    #[test]
    pub fn test_tnc08_immutability() {
        let mut obj = DecisionObject::new("tnc08");
        obj.status = Status::Approved;
        assert!(TNCEnforcer::tnc08_immutability(&obj).is_err());
        obj.authority_chain.push(link(
            "valid-pubkey-0009",
            "valid-signature-0009",
            ActorKind::Human,
        ));
        assert!(TNCEnforcer::tnc08_immutability(&obj).is_ok());
    }

    #[test]
    pub fn test_tnc09_ai_ceiling() {
        let mut obj = DecisionObject::new("tnc09");
        obj.authority_chain.push(link(
            "valid-pubkey-0010",
            "valid-signature-0010",
            ActorKind::Human,
        ));
        obj.authority_chain.push(link(
            "valid-pubkey-0011",
            "valid-signature-0011",
            ActorKind::Agent,
        ));
        obj.authority_chain.push(link(
            "valid-pubkey-0012",
            "valid-signature-0012",
            ActorKind::Agent,
        ));
        assert!(TNCEnforcer::tnc09_ai_ceiling(&obj).is_err());
    }

    #[test]
    pub fn test_tnc10_ratification() {
        let mut obj = DecisionObject::new("tnc10");
        obj.decision_class = DecisionClass::Policy;
        assert!(TNCEnforcer::tnc10_ratification(&obj).is_err());
        obj.ratified_by = Some("council:ratifier".to_string());
        obj.ratified_at = Some(chrono::Utc::now());
        assert!(TNCEnforcer::tnc10_ratification(&obj).is_ok());
    }

    #[test]
    pub fn test_enforce_all_happy_path() {
        let mut obj = DecisionObject::new("full-path");
        obj.required_quorum = 1;
        obj.authority_chain.push(link(
            "valid-pubkey-0013",
            "valid-signature-0013",
            ActorKind::Human,
        ));
        obj.human_review = HumanReviewStatus::approved_by("council:happy");
        obj.evidence.push(Evidence {
            hash: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                .to_string(),
            description: "supporting exhibit".to_string(),
        });
        obj.audit_log.push(crate::decision_object::AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: crate::decision_object::AuditEventType::SealAttempt,
            reason: "attempt".to_string(),
            escalation_reason: None,
            assessment_snapshot: None,
        });
        obj.status = Status::Approved;
        assert!(TNCEnforcer::enforce_all(&obj).is_ok());
    }
}
