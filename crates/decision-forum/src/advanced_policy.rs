use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_POLICY_VERSION: &str = "2026-03-14.council.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BayesianAssessment {
    pub prior: f64,
    pub posterior: f64,
    pub confidence_interval: f64,
    pub sensitivity_instability: f64,
    pub teacher_student_disagreement: f64,
    pub symbolic_rule_trace_hash: String,
    pub evidence_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdvancedThresholds {
    pub min_confidence_interval: f64,
    pub max_sensitivity_instability: f64,
    pub max_teacher_student_disagreement: f64,
}

impl Default for AdvancedThresholds {
    fn default() -> Self {
        Self {
            min_confidence_interval: 0.85,
            max_sensitivity_instability: 0.10,
            max_teacher_student_disagreement: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecursiveTrainingPolicy {
    /// Process-level rule: only teacher-verified / council-ratified examples
    /// may enter future distillation or supervised fine-tuning corpora.
    TeacherVerifiedCouncilRatifiedOnly,
}

impl Default for RecursiveTrainingPolicy {
    fn default() -> Self {
        Self::TeacherVerifiedCouncilRatifiedOnly
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanReviewStatus {
    pub required: bool,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

impl Default for HumanReviewStatus {
    fn default() -> Self {
        Self {
            required: true,
            reviewed_by: None,
            reviewed_at: None,
        }
    }
}

impl HumanReviewStatus {
    pub fn approved_by(reviewer: impl Into<String>) -> Self {
        Self {
            required: true,
            reviewed_by: Some(reviewer.into()),
            reviewed_at: Some(Utc::now()),
        }
    }

    pub fn is_satisfied(&self) -> bool {
        !self.required || (self.reviewed_by.is_some() && self.reviewed_at.is_some())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdvancedReasoningPolicy {
    pub policy_version: String,
    pub thresholds: AdvancedThresholds,
    pub assessment: BayesianAssessment,
    pub human_review: HumanReviewStatus,
    /// This is governance/process metadata, not runtime model-training enforcement.
    pub recursive_training_policy: RecursiveTrainingPolicy,
}

impl AdvancedReasoningPolicy {
    pub fn new(assessment: BayesianAssessment) -> Self {
        Self {
            policy_version: DEFAULT_POLICY_VERSION.to_string(),
            thresholds: AdvancedThresholds::default(),
            assessment,
            human_review: HumanReviewStatus::default(),
            recursive_training_policy: RecursiveTrainingPolicy::default(),
        }
    }

    /// Validates the advanced Bayesian parameters against constitutional thresholds.
    ///
    /// The neuro-symbolic reasoning is ONLY an advisory speed layer. It can NEVER
    /// replace symbolic verification. If confidence is too low, if there is
    /// instability, missing evidence, or disagreement between the neural proposal
    /// and symbolic rules, the decision MUST escalate/reject.
    pub fn validate_thresholds(&self) -> Result<(), EscalationReason> {
        #[cfg(test)]
        crate::requirements::Requirement::AdvancedPolicyValidation.mark_covered();

        if self.assessment.confidence_interval < self.thresholds.min_confidence_interval {
            return Err(EscalationReason::LowConfidence);
        }

        if self.assessment.sensitivity_instability > self.thresholds.max_sensitivity_instability {
            return Err(EscalationReason::HighInstability);
        }

        if self.assessment.teacher_student_disagreement
            > self.thresholds.max_teacher_student_disagreement
        {
            return Err(EscalationReason::HighDisagreement);
        }

        if !is_valid_trace_hash(&self.assessment.symbolic_rule_trace_hash) {
            return Err(EscalationReason::VerifierMismatch);
        }

        if self.assessment.evidence_references.is_empty() {
            return Err(EscalationReason::MissingEvidenceReference);
        }

        if self
            .assessment
            .evidence_references
            .iter()
            .any(|reference| !is_valid_evidence_reference(reference))
        {
            return Err(EscalationReason::InvalidEvidenceReference);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EscalationReason {
    LowConfidence,
    HighInstability,
    VerifierMismatch,
    HighDisagreement,
    MissingEvidenceReference,
    InvalidEvidenceReference,
}

pub fn is_valid_trace_hash(value: &str) -> bool {
    normalize_hash_hex(value).is_some()
}

pub fn is_valid_evidence_reference(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || is_valid_trace_hash(trimmed)
}

fn normalize_hash_hex(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    let stripped = trimmed
        .strip_prefix("sha256:")
        .or_else(|| trimmed.strip_prefix("0x"))
        .unwrap_or(trimmed);

    if stripped.len() == 64 && stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(stripped)
    } else {
        None
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::requirements::Requirement;

    fn valid_assessment() -> BayesianAssessment {
        BayesianAssessment {
            prior: 0.5,
            posterior: 0.9,
            confidence_interval: 0.90,
            sensitivity_instability: 0.05,
            teacher_student_disagreement: 0.02,
            symbolic_rule_trace_hash:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            evidence_references: vec!["https://evidence.exochain.local/doc/1".to_string()],
        }
    }

    #[test]
    pub fn test_valid_advanced_policy() {
        let policy = AdvancedReasoningPolicy::new(valid_assessment());
        assert_eq!(policy.validate_thresholds(), Ok(()));
        Requirement::AdvancedPolicyValidation.mark_covered();
    }

    #[test]
    pub fn test_invalid_advanced_policy_low_confidence() {
        let mut assessment = valid_assessment();
        assessment.confidence_interval = 0.80;
        let policy = AdvancedReasoningPolicy::new(assessment);
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::LowConfidence));
    }

    #[test]
    pub fn test_invalid_advanced_policy_high_instability() {
        let mut assessment = valid_assessment();
        assessment.sensitivity_instability = 0.15;
        let policy = AdvancedReasoningPolicy::new(assessment);
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::HighInstability));
    }

    #[test]
    pub fn test_invalid_advanced_policy_high_disagreement() {
        let mut assessment = valid_assessment();
        assessment.teacher_student_disagreement = 0.10;
        let policy = AdvancedReasoningPolicy::new(assessment);
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::HighDisagreement));
    }

    #[test]
    pub fn test_invalid_advanced_policy_verifier_mismatch() {
        let mut assessment = valid_assessment();
        assessment.symbolic_rule_trace_hash = "0x123".to_string();
        let policy = AdvancedReasoningPolicy::new(assessment);
        assert_eq!(policy.validate_thresholds(), Err(EscalationReason::VerifierMismatch));
    }

    #[test]
    pub fn test_invalid_advanced_policy_missing_evidence() {
        let mut assessment = valid_assessment();
        assessment.evidence_references.clear();
        let policy = AdvancedReasoningPolicy::new(assessment);
        assert_eq!(
            policy.validate_thresholds(),
            Err(EscalationReason::MissingEvidenceReference)
        );
    }

    #[test]
    pub fn test_invalid_advanced_policy_invalid_evidence_reference() {
        let mut assessment = valid_assessment();
        assessment.evidence_references = vec!["not-a-hash-or-url".to_string()];
        let policy = AdvancedReasoningPolicy::new(assessment);
        assert_eq!(
            policy.validate_thresholds(),
            Err(EscalationReason::InvalidEvidenceReference)
        );
    }

    #[test]
    pub fn test_trace_hash_accepts_sha256_and_0x_formats() {
        assert!(is_valid_trace_hash(
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        ));
        assert!(is_valid_trace_hash(
            "0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        ));
        assert!(!is_valid_trace_hash("0x123"));
    }

    #[test]
    pub fn test_human_review_status_approval_helper() {
        let review = HumanReviewStatus::approved_by("council:alice");
        assert!(review.is_satisfied());
    }
}
