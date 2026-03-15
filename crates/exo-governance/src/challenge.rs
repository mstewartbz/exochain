//! Challenge Objects — contestation, reversal, and resolution.
//!
//! Satisfies: GOV-008

use crate::types::*;
use exo_core::crypto::Blake3Hash;
use exo_core::hlc::HybridLogicalClock;
use serde::{Deserialize, Serialize};

/// Status of a challenge.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChallengeStatus {
    /// Challenge filed — decision execution is paused.
    Filed,
    /// Under review by appropriate authority.
    UnderReview,
    /// Challenge upheld — decision will be reversed.
    Upheld,
    /// Challenge denied — decision stands.
    Denied,
    /// Challenge withdrawn by challenger.
    Withdrawn,
}

/// A Challenge Object linked to a contested decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeObject {
    /// Unique identifier.
    pub id: Blake3Hash,
    /// Tenant context.
    pub tenant_id: TenantId,
    /// The decision being challenged.
    pub contested_decision_id: Blake3Hash,
    /// Who filed the challenge.
    pub challenger: Did,
    /// Grounds for the challenge.
    pub grounds: ChallengeGrounds,
    /// Detailed rationale.
    pub rationale: String,
    /// Supporting evidence.
    pub evidence: Vec<EvidenceRef>,
    /// Current status.
    pub status: ChallengeStatus,
    /// Resolution (if resolved).
    pub resolution: Option<ChallengeResolution>,
    /// Filing timestamp.
    pub filed_at: HybridLogicalClock,
    /// Challenger's signature.
    pub signature: GovernanceSignature,
}

/// Grounds for challenging a decision.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChallengeGrounds {
    /// Authority chain was invalid at time of decision.
    AuthorityChainInvalid,
    /// Constitutional constraint was violated.
    ConstitutionalViolation,
    /// Quorum was not properly verified.
    QuorumViolation,
    /// Conflict of interest was undisclosed.
    UndisclosedConflict,
    /// Procedural error in the decision process.
    ProceduralError,
    /// New evidence materially changes the outcome.
    NewEvidence,
    /// Other grounds.
    Other(String),
}

/// Resolution of a challenge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeResolution {
    /// Who resolved the challenge.
    pub resolver: Did,
    /// The resolution decision (itself a new Decision Object).
    pub resolution_decision_id: Blake3Hash,
    /// Whether the original decision is reversed.
    pub reversal: bool,
    /// If reversed, the reversal linkage (immutable).
    pub reversal_decision_id: Option<Blake3Hash>,
    /// Resolution timestamp.
    pub resolved_at: HybridLogicalClock,
    /// Explanation.
    pub explanation: String,
    /// Resolver's signature.
    pub signature: GovernanceSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_statuses() {
        let statuses = vec![
            ChallengeStatus::Filed,
            ChallengeStatus::UnderReview,
            ChallengeStatus::Upheld,
            ChallengeStatus::Denied,
            ChallengeStatus::Withdrawn,
        ];
        // All statuses should be distinct
        for (i, a) in statuses.iter().enumerate() {
            for (j, b) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_challenge_grounds_variants() {
        let g = ChallengeGrounds::ConstitutionalViolation;
        assert_eq!(g, ChallengeGrounds::ConstitutionalViolation);
        assert_ne!(g, ChallengeGrounds::AuthorityChainInvalid);
    }
}
