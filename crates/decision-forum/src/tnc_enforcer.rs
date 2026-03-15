use super::decision_object::{DecisionClass, DecisionObject, Status};
use crate::authority::ActorKind;
use std::collections::HashSet;

const MIN_KEY_MATERIAL_LEN: usize = 8;
const MAX_AI_SIGNER_RATIO: f64 = 0.49;

/// All 10 Trust-Critical Non-Negotiable Controls enforced here.
pub struct TNCEnforcer;

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
        }
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
