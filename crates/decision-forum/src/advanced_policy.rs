use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesianAssessment {
    pub prior: f64,
    pub posterior: f64,
    pub confidence_interval: f64,
    pub sensitivity_instability: f64,
    pub teacher_student_disagreement: f64,
    pub symbolic_rule_trace_hash: String,
    pub evidence_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedReasoningPolicy {
    pub assessment: BayesianAssessment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EscalationReason {
    LowConfidence,
    HighInstability,
    VerifierMismatch,
    HighDisagreement,
}

impl AdvancedReasoningPolicy {
    /// Validates the advanced Bayesian parameters against constitutional thresholds.
    ///
    /// The neuro-symbolic reasoning is ONLY an advisory speed layer. It can NEVER
    /// replace symbolic verification. If confidence is too low, or if there is
    /// instability or disagreement between the neural proposal and symbolic rules,
    /// the decision MUST escalate/reject.
    pub fn validate_thresholds(&self) -> Result<(), EscalationReason> {
        #[cfg(test)]
        crate::requirements::Requirement::AdvancedPolicyValidation.mark_covered();

        // Hard Thresholds (as per Bayesian Policy Spec)
        if self.assessment.confidence_interval < 0.85 {
            return Err(EscalationReason::LowConfidence);
        }
        if self.assessment.sensitivity_instability > 0.10 {
            return Err(EscalationReason::HighInstability);
        }
        if self.assessment.teacher_student_disagreement > 0.05 {
            return Err(EscalationReason::HighDisagreement);
        }
        if self.assessment.symbolic_rule_trace_hash.is_empty() {
            return Err(EscalationReason::VerifierMismatch);
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::requirements::Requirement;

    #[test]
    pub fn test_valid_advanced_policy() {
        let policy = AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.90, // > 0.85
                sensitivity_instability: 0.05, // < 0.10
                teacher_student_disagreement: 0.02, // < 0.05
                symbolic_rule_trace_hash: "0xABCDEF".to_string(),
                evidence_references: vec!["doc_1".to_string()],
            },
        };
        assert_eq!(policy.validate_thresholds(), Ok(()));
        Requirement::AdvancedPolicyValidation.mark_covered();
    }

    #[test]
    pub fn test_invalid_advanced_policy_low_confidence() {
        let policy = AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.80, // Fails: < 0.85
                sensitivity_instability: 0.05,
                teacher_student_disagreement: 0.02,
                symbolic_rule_trace_hash: "0xABCDEF".to_string(),
                evidence_references: vec![],
            },
        };
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::LowConfidence));
    }

    #[test]
    pub fn test_invalid_advanced_policy_high_instability() {
        let policy = AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.90,
                sensitivity_instability: 0.15, // Fails: > 0.10
                teacher_student_disagreement: 0.02,
                symbolic_rule_trace_hash: "0xABCDEF".to_string(),
                evidence_references: vec![],
            },
        };
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::HighInstability));
    }

    #[test]
    pub fn test_invalid_advanced_policy_high_disagreement() {
        let policy = AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.90,
                sensitivity_instability: 0.05,
                teacher_student_disagreement: 0.10, // Fails: > 0.05
                symbolic_rule_trace_hash: "0xABCDEF".to_string(),
                evidence_references: vec![],
            },
        };
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::HighDisagreement));
    }

    #[test]
    pub fn test_invalid_advanced_policy_verifier_mismatch() {
        let policy = AdvancedReasoningPolicy {
            assessment: BayesianAssessment {
                prior: 0.5,
                posterior: 0.9,
                confidence_interval: 0.90,
                sensitivity_instability: 0.05,
                teacher_student_disagreement: 0.02,
                symbolic_rule_trace_hash: "".to_string(), // Fails: Empty hash implies verifier mismatch/failure
                evidence_references: vec![],
            },
        };
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::VerifierMismatch));
    }
}
