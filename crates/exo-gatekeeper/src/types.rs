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

//! Gatekeeper governance types.
//!
//! Types specific to the judicial branch that are not part of exo-core.

use std::collections::{BTreeMap, BTreeSet};

use exo_core::Did;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Permission & capability types
// ---------------------------------------------------------------------------

/// A named permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission(pub String);

impl Permission {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// A set of permissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PermissionSet {
    pub permissions: Vec<Permission>,
}

impl PermissionSet {
    #[must_use]
    pub fn new(permissions: Vec<Permission>) -> Self {
        Self { permissions }
    }

    pub fn contains(&self, p: &Permission) -> bool {
        self.permissions.contains(p)
    }

    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Government branches & roles
// ---------------------------------------------------------------------------

/// Branch of government.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GovernmentBranch {
    Legislative,
    Executive,
    Judicial,
}

impl GovernmentBranch {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Legislative => "legislative",
            Self::Executive => "executive",
            Self::Judicial => "judicial",
        }
    }
}

/// Governed role names recognized by the constitutional fabric.
///
/// The names are intentionally finite.  Adjudication may carry zero roles, but
/// any supplied role must be one of these governed names and must match the
/// branch assigned below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GovernedRoleName {
    Senator,
    Legislator,
    Voter,
    Executive,
    ExecutiveAdmin,
    Operator,
    Worker,
    Judge,
    TransitionJudge,
}

impl GovernedRoleName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Senator => "senator",
            Self::Legislator => "legislator",
            Self::Voter => "voter",
            Self::Executive => "executive",
            Self::ExecutiveAdmin => "executive-admin",
            Self::Operator => "operator",
            Self::Worker => "worker",
            Self::Judge => "judge",
            Self::TransitionJudge => "transition-judge",
        }
    }

    #[must_use]
    pub const fn branch(self) -> GovernmentBranch {
        match self {
            Self::Senator | Self::Legislator | Self::Voter => GovernmentBranch::Legislative,
            Self::Executive | Self::ExecutiveAdmin | Self::Operator | Self::Worker => {
                GovernmentBranch::Executive
            }
            Self::Judge | Self::TransitionJudge => GovernmentBranch::Judicial,
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "senator" => Some(Self::Senator),
            "legislator" => Some(Self::Legislator),
            "voter" => Some(Self::Voter),
            "executive" => Some(Self::Executive),
            "executive-admin" => Some(Self::ExecutiveAdmin),
            "operator" => Some(Self::Operator),
            "worker" => Some(Self::Worker),
            "judge" => Some(Self::Judge),
            "transition-judge" => Some(Self::TransitionJudge),
            _ => None,
        }
    }
}

/// Role validation failure with enough structured context for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleValidationError {
    #[error("unknown governed role name")]
    UnknownName { name: String },
    #[error(
        "role name does not match governed branch: expected {expected_branch}, actual {actual_branch}"
    )]
    BranchMismatch {
        name: String,
        expected_branch: &'static str,
        actual_branch: &'static str,
    },
}

/// Role held by an actor in the constitutional fabric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub branch: GovernmentBranch,
}

impl Role {
    #[must_use]
    pub fn governed(name: GovernedRoleName) -> Self {
        Self {
            name: name.as_str().to_owned(),
            branch: name.branch(),
        }
    }

    /// Validate a role supplied from storage, API, MCP, or WASM context.
    ///
    /// # Errors
    ///
    /// Returns [`RoleValidationError::UnknownName`] when `self.name` is not a
    /// governed role name, or [`RoleValidationError::BranchMismatch`] when the
    /// governed role belongs to a different branch than `self.branch`.
    pub fn validate_governed(&self) -> Result<GovernedRoleName, RoleValidationError> {
        let Some(governed_name) = GovernedRoleName::parse(&self.name) else {
            return Err(RoleValidationError::UnknownName {
                name: self.name.clone(),
            });
        };
        let expected_branch = governed_name.branch();
        if expected_branch != self.branch {
            return Err(RoleValidationError::BranchMismatch {
                name: self.name.clone(),
                expected_branch: expected_branch.as_str(),
                actual_branch: self.branch.as_str(),
            });
        }
        Ok(governed_name)
    }

    /// Construct and validate a governed role from external string input.
    ///
    /// # Errors
    ///
    /// Returns [`RoleValidationError`] if `name` is not governed or if it is
    /// paired with the wrong branch.
    pub fn try_new(
        name: impl Into<String>,
        branch: GovernmentBranch,
    ) -> Result<Self, RoleValidationError> {
        let role = Self {
            name: name.into(),
            branch,
        };
        role.validate_governed()?;
        Ok(role)
    }
}

// ---------------------------------------------------------------------------
// Bailment state (gatekeeper view — simpler than BCTS lifecycle)
// ---------------------------------------------------------------------------

/// Canonical DAG DB writeback bailment scope.
pub const DAGDB_WRITEBACK_SCOPE: &str = "dag-db:writeback";

/// Whether an active bailment + consent exists for a data scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BailmentState {
    /// No bailment established.
    None,
    /// Active bailment with consent.
    Active {
        bailor: Did,
        bailee: Did,
        scope: String,
    },
    /// Bailment suspended.
    Suspended { reason: String },
    /// Bailment terminated.
    Terminated,
}

impl BailmentState {
    pub fn is_active(&self) -> bool {
        matches!(self, BailmentState::Active { .. })
    }

    pub fn authorizes_writeback(&self, agent_did: &str) -> bool {
        matches!(
            self,
            BailmentState::Active { bailee, scope, .. }
                if bailee.as_str() == agent_did && scope == DAGDB_WRITEBACK_SCOPE
        )
    }
}

// ---------------------------------------------------------------------------
// Consent record
// ---------------------------------------------------------------------------

/// A consent record for the gatekeeper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub subject: Did,
    pub granted_to: Did,
    pub scope: String,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Authority chain
// ---------------------------------------------------------------------------

/// Authority chain — the delegation path from root to actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuthorityChain {
    pub links: Vec<AuthorityLink>,
}

impl AuthorityChain {
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    pub fn depth(&self) -> usize {
        self.links.len()
    }
}

/// A single link in an authority chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityLink {
    pub grantor: Did,
    pub grantee: Did,
    pub permissions: PermissionSet,
    pub signature: Vec<u8>,
    /// Ed25519 public key (32 bytes) of the grantor.
    ///
    /// `check_authority_chain_valid` requires this key and performs Ed25519
    /// signature verification over the domain-separated canonical CBOR link
    /// payload. Links without this key fail closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grantor_public_key: Option<Vec<u8>>,
}

/// DID-resolved Ed25519 public keys trusted by the runtime context.
///
/// Authority links still carry the key used for signature verification, but
/// the invariant engine only accepts that key when it matches independently
/// resolved key material for the claimed grantor DID.
pub type TrustedAuthorityKeys = BTreeMap<Did, Vec<Vec<u8>>>;

/// Maximum number of signed delegation links accepted by gatekeeper core.
///
/// Adapters may reject an over-depth chain earlier, but direct kernel callers
/// must receive the same fail-closed bound.
pub const MAX_AUTHORITY_CHAIN_LINKS: usize = 5;

/// DID-resolved Ed25519 public keys trusted for actor provenance.
///
/// Provenance objects still carry the key used for signature verification, but
/// the invariant engine only accepts that key when it matches independently
/// resolved key material for the claimed actor DID.
pub type TrustedProvenanceKeys = BTreeMap<Did, Vec<Vec<u8>>>;

// ---------------------------------------------------------------------------
// Quorum evidence
// ---------------------------------------------------------------------------

/// Quorum decision evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumEvidence {
    pub threshold: u32,
    pub votes: Vec<QuorumVote>,
}

impl QuorumEvidence {
    /// Returns `true` if the raw approval count (all votes, regardless of
    /// provenance) meets the threshold. Used for legacy/simple quorum checks.
    pub fn is_met(&self) -> bool {
        let Some(threshold) = usize::try_from(self.threshold).ok() else {
            return false;
        };
        self.distinct_approved_voter_count() >= threshold
    }

    /// Returns `true` if the threshold is met counting only non-synthetic
    /// approved votes with explicit human, independent, first-order provenance.
    /// Missing, system, synthetic, coordinated, or derivative provenance is
    /// excluded from the human count per CR-001 §8.3.
    pub fn is_met_authentic(&self) -> bool {
        let Some(threshold) = usize::try_from(self.threshold).ok() else {
            return false;
        };
        self.distinct_authentic_approved_voter_count() >= threshold
    }

    /// Count of votes where provenance explicitly marks the voter as synthetic.
    pub fn synthetic_vote_count(&self) -> usize {
        self.votes
            .iter()
            .filter(|v| v.provenance.as_ref().is_some_and(|p| p.is_synthetic()))
            .count()
    }

    /// Voter DIDs that appear more than once in the evidence.
    #[must_use]
    pub fn duplicate_voters(&self) -> BTreeSet<Did> {
        let mut seen = BTreeSet::new();
        let mut duplicates = BTreeSet::new();
        for vote in &self.votes {
            if !seen.insert(vote.voter.clone()) {
                duplicates.insert(vote.voter.clone());
            }
        }
        duplicates
    }

    /// Count distinct approved voter DIDs, regardless of provenance.
    #[must_use]
    pub fn distinct_approved_voter_count(&self) -> usize {
        self.votes
            .iter()
            .filter(|vote| vote.approved)
            .map(|vote| vote.voter.clone())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Count distinct approved voter DIDs that are not synthetic.
    #[must_use]
    pub fn distinct_authentic_approved_voter_count(&self) -> usize {
        self.votes
            .iter()
            .filter(|vote| {
                vote.approved
                    && vote
                        .provenance
                        .as_ref()
                        .is_some_and(Provenance::is_authentic_human_quorum_voice)
            })
            .map(|vote| vote.voter.clone())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

/// A single quorum vote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumVote {
    pub voter: Did,
    pub approved: bool,
    pub signature: Vec<u8>,
    /// Optional provenance for this vote.
    ///
    /// When `voice_kind` is `Synthetic`, this vote SHALL NOT count as a
    /// distinct human approval in quorum (CR-001 §8.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Provenance metadata — voice taxonomy (CR-001 §8.3)
// ---------------------------------------------------------------------------

/// Whether an actor is a human, synthetic (AI), or system process.
///
/// Used by governance surfaces accepting plural input to prevent synthetic
/// voices from being counted as distinct humans (CR-001 §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoiceKind {
    /// A natural human actor providing genuine review or approval.
    Human,
    /// A synthetic (AI-generated) opinion or action. SHALL NOT be counted as
    /// a distinct human vote in any quorum or clearance computation.
    Synthetic,
    /// An automated system process (not human or AI opinion).
    System,
}

/// Independence claim for a reviewer or voter.
///
/// Coordinated actors sharing common control SHALL NOT be double-counted in
/// quorum (CR-001 §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IndependenceClaim {
    /// Actor claims independent judgment with no undisclosed common control.
    Independent,
    /// Actor discloses coordination with another entity.
    Coordinated,
}

/// Order of review for a governance opinion.
///
/// Derivative or echoed reviews SHALL NOT be counted equivalently to
/// first-order independent review (CR-001 §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReviewOrder {
    /// Direct, first-hand review of the original subject.
    FirstOrder,
    /// Derivative review — based on another review, summary, or echo.
    Derivative,
}

// ---------------------------------------------------------------------------
// Provenance metadata
// ---------------------------------------------------------------------------

/// Provenance metadata for an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub actor: Did,
    pub timestamp: String,
    pub action_hash: Vec<u8>,
    pub signature: Vec<u8>,
    /// Ed25519 public key (32 bytes) of the actor.
    ///
    /// `check_provenance_verifiable` requires this key and performs Ed25519
    /// signature verification over the domain-separated canonical CBOR
    /// provenance payload. Provenance without this key fails closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<Vec<u8>>,
    /// Whether this actor is human, synthetic (AI), or a system process.
    ///
    /// Governance surfaces accepting plural input MUST use this field to
    /// prevent synthetic voices from counting as distinct humans (CR-001 §8.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_kind: Option<VoiceKind>,
    /// Whether the actor claims to act independently (no undisclosed common
    /// control with other reviewers or voters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independence: Option<IndependenceClaim>,
    /// Whether this is a first-order review or derivative (echo/summary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_order: Option<ReviewOrder>,
}

impl Provenance {
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }

    /// Returns `true` when provenance explicitly identifies this as a human
    /// voice. `None` (unspecified) returns `false` — unattributed provenance
    /// is never assumed to be authentic human judgment.
    pub fn is_human_voice(&self) -> bool {
        self.voice_kind == Some(VoiceKind::Human)
    }

    /// Returns `true` when provenance explicitly claims independence.
    /// `None` (unspecified) is treated as non-independent.
    pub fn is_independent(&self) -> bool {
        self.independence == Some(IndependenceClaim::Independent)
    }

    /// Returns `true` when this provenance is explicitly first-order review.
    /// `None` and derivative review are not equivalent to direct human review.
    pub fn is_first_order_review(&self) -> bool {
        self.review_order == Some(ReviewOrder::FirstOrder)
    }

    /// Returns `true` when all taxonomy fields required for a human quorum
    /// claim are present and explicit. Cryptographic verification is performed
    /// by the invariant engine because it needs trusted DID-resolved keys.
    pub fn is_authentic_human_quorum_voice(&self) -> bool {
        self.is_human_voice() && self.is_independent() && self.is_first_order_review()
    }

    /// Returns `true` when this is explicitly a synthetic (AI-generated) voice.
    pub fn is_synthetic(&self) -> bool {
        self.voice_kind == Some(VoiceKind::Synthetic)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn did(s: &str) -> Did {
        Did::new(s).expect("valid DID")
    }

    #[test]
    fn permission_set_contains() {
        let set = PermissionSet::new(vec![Permission::new("read"), Permission::new("write")]);
        assert!(set.contains(&Permission::new("read")));
        assert!(!set.contains(&Permission::new("admin")));
    }

    #[test]
    fn permission_set_empty() {
        let set = PermissionSet::default();
        assert!(set.is_empty());
    }

    #[test]
    fn bailment_state_is_active() {
        let active = BailmentState::Active {
            bailor: did("did:exo:bailor"),
            bailee: did("did:exo:bailee"),
            scope: "data".into(),
        };
        assert!(active.is_active());
        assert!(!BailmentState::None.is_active());
        assert!(!BailmentState::Terminated.is_active());
        let suspended = BailmentState::Suspended {
            reason: "audit".into(),
        };
        assert!(!suspended.is_active());
    }

    #[test]
    fn bailment_state_authorizes_writeback_for_active_bailee_and_scope() {
        let active = BailmentState::Active {
            bailor: did("did:exo:bailor"),
            bailee: did("did:exo:bailee"),
            scope: DAGDB_WRITEBACK_SCOPE.into(),
        };

        assert!(active.authorizes_writeback("did:exo:bailee"));
    }

    #[test]
    fn bailment_state_authorizes_writeback_rejects_wrong_bailee() {
        let active = BailmentState::Active {
            bailor: did("did:exo:bailor"),
            bailee: did("did:exo:bailee"),
            scope: DAGDB_WRITEBACK_SCOPE.into(),
        };

        assert!(!active.authorizes_writeback("did:exo:other"));
    }

    #[test]
    fn bailment_state_authorizes_writeback_rejects_wrong_scope() {
        let active = BailmentState::Active {
            bailor: did("did:exo:bailor"),
            bailee: did("did:exo:bailee"),
            scope: "dag-db:read".into(),
        };

        assert!(!active.authorizes_writeback("did:exo:bailee"));
    }

    #[test]
    fn bailment_state_authorizes_writeback_rejects_inactive_states() {
        assert!(!BailmentState::None.authorizes_writeback("did:exo:bailee"));
        assert!(!BailmentState::Terminated.authorizes_writeback("did:exo:bailee"));
        assert!(
            !BailmentState::Suspended {
                reason: "audit".into(),
            }
            .authorizes_writeback("did:exo:bailee")
        );
    }

    #[test]
    fn authority_chain_empty() {
        let chain = AuthorityChain::default();
        assert!(chain.is_empty());
        assert_eq!(chain.depth(), 0);
    }

    #[test]
    fn authority_chain_depth() {
        let chain = AuthorityChain {
            links: vec![
                AuthorityLink {
                    grantor: did("did:exo:root"),
                    grantee: did("did:exo:mid"),
                    permissions: PermissionSet::default(),
                    signature: vec![1],
                    grantor_public_key: None,
                },
                AuthorityLink {
                    grantor: did("did:exo:mid"),
                    grantee: did("did:exo:leaf"),
                    permissions: PermissionSet::default(),
                    signature: vec![2],
                    grantor_public_key: None,
                },
            ],
        };
        assert_eq!(chain.depth(), 2);
        assert!(!chain.is_empty());
    }

    fn make_vote(voter: &str, approved: bool, sig: u8, voice: Option<VoiceKind>) -> QuorumVote {
        QuorumVote {
            voter: did(voter),
            approved,
            signature: vec![sig],
            provenance: voice.map(|vk| Provenance {
                actor: did(voter),
                timestamp: "t".into(),
                action_hash: vec![1],
                signature: vec![sig],
                public_key: None,
                voice_kind: Some(vk),
                independence: (vk == VoiceKind::Human).then_some(IndependenceClaim::Independent),
                review_order: (vk == VoiceKind::Human).then_some(ReviewOrder::FirstOrder),
            }),
        }
    }

    #[test]
    fn quorum_evidence_met() {
        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![
                QuorumVote {
                    voter: did("did:exo:v1"),
                    approved: true,
                    signature: vec![1],
                    provenance: None,
                },
                QuorumVote {
                    voter: did("did:exo:v2"),
                    approved: true,
                    signature: vec![2],
                    provenance: None,
                },
                QuorumVote {
                    voter: did("did:exo:v3"),
                    approved: false,
                    signature: vec![3],
                    provenance: None,
                },
            ],
        };
        assert!(ev.is_met());
    }

    #[test]
    fn quorum_evidence_not_met() {
        let ev = QuorumEvidence {
            threshold: 3,
            votes: vec![
                QuorumVote {
                    voter: did("did:exo:v1"),
                    approved: true,
                    signature: vec![1],
                    provenance: None,
                },
                QuorumVote {
                    voter: did("did:exo:v2"),
                    approved: false,
                    signature: vec![2],
                    provenance: None,
                },
            ],
        };
        assert!(!ev.is_met());
    }

    #[test]
    fn quorum_evidence_counts_distinct_voters_only() {
        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![
                QuorumVote {
                    voter: did("did:exo:v1"),
                    approved: true,
                    signature: vec![1],
                    provenance: None,
                },
                QuorumVote {
                    voter: did("did:exo:v1"),
                    approved: true,
                    signature: vec![2],
                    provenance: None,
                },
            ],
        };
        assert!(
            !ev.is_met(),
            "duplicate voter DIDs must not inflate raw quorum evidence"
        );
    }

    // ── CR-001 §8.3: authentic quorum counting ───────────────────────────────

    #[test]
    fn quorum_is_met_authentic_excludes_synthetic() {
        // 2 humans approve, 1 synthetic approves — threshold 3 should fail authentic
        let ev = QuorumEvidence {
            threshold: 3,
            votes: vec![
                make_vote("did:exo:h1", true, 1, Some(VoiceKind::Human)),
                make_vote("did:exo:h2", true, 2, Some(VoiceKind::Human)),
                make_vote("did:exo:ai1", true, 3, Some(VoiceKind::Synthetic)),
            ],
        };
        assert!(ev.is_met(), "raw count should pass (3 approvals)");
        assert!(
            !ev.is_met_authentic(),
            "authentic count should fail (only 2 human)"
        );
        assert_eq!(ev.synthetic_vote_count(), 1);
    }

    #[test]
    fn quorum_is_met_authentic_passes_all_human() {
        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![
                make_vote("did:exo:h1", true, 1, Some(VoiceKind::Human)),
                make_vote("did:exo:h2", true, 2, Some(VoiceKind::Human)),
            ],
        };
        assert!(ev.is_met_authentic());
        assert_eq!(ev.synthetic_vote_count(), 0);
    }

    #[test]
    fn quorum_is_met_authentic_counts_distinct_humans_only() {
        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![
                make_vote("did:exo:h1", true, 1, Some(VoiceKind::Human)),
                make_vote("did:exo:h1", true, 2, Some(VoiceKind::Human)),
            ],
        };
        assert!(
            !ev.is_met_authentic(),
            "duplicate human voter DIDs must not inflate authentic quorum evidence"
        );
    }

    #[test]
    fn quorum_is_met_authentic_rejects_legacy_votes_without_provenance() {
        // Legacy votes (no provenance) are not authentic human quorum votes.
        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![
                QuorumVote {
                    voter: did("did:exo:v1"),
                    approved: true,
                    signature: vec![1],
                    provenance: None,
                },
                QuorumVote {
                    voter: did("did:exo:v2"),
                    approved: true,
                    signature: vec![2],
                    provenance: None,
                },
            ],
        };
        assert!(!ev.is_met_authentic());
    }

    #[test]
    fn quorum_is_met_authentic_rejects_votes_without_human_provenance() {
        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![
                QuorumVote {
                    voter: did("did:exo:v1"),
                    approved: true,
                    signature: vec![1],
                    provenance: None,
                },
                make_vote("did:exo:system1", true, 2, Some(VoiceKind::System)),
            ],
        };

        assert!(
            !ev.is_met_authentic(),
            "authentic quorum must not assume missing or system provenance is human"
        );
    }

    #[test]
    fn quorum_is_met_authentic_requires_independent_first_order_human_votes() {
        let mut coordinated = make_vote("did:exo:h1", true, 1, Some(VoiceKind::Human));
        coordinated
            .provenance
            .as_mut()
            .expect("human provenance")
            .independence = Some(IndependenceClaim::Coordinated);
        coordinated
            .provenance
            .as_mut()
            .expect("human provenance")
            .review_order = Some(ReviewOrder::FirstOrder);

        let mut derivative = make_vote("did:exo:h2", true, 2, Some(VoiceKind::Human));
        derivative
            .provenance
            .as_mut()
            .expect("human provenance")
            .independence = Some(IndependenceClaim::Independent);
        derivative
            .provenance
            .as_mut()
            .expect("human provenance")
            .review_order = Some(ReviewOrder::Derivative);

        let ev = QuorumEvidence {
            threshold: 2,
            votes: vec![coordinated, derivative],
        };

        assert!(
            !ev.is_met_authentic(),
            "coordinated or derivative human claims must not count as authentic quorum"
        );
    }

    // ── Provenance voice/independence helpers ────────────────────────────────

    #[test]
    fn provenance_is_human_voice() {
        let human_prov = Provenance {
            actor: did("did:exo:h1"),
            timestamp: "t".into(),
            action_hash: vec![1],
            signature: vec![1],
            public_key: None,
            voice_kind: Some(VoiceKind::Human),
            independence: Some(IndependenceClaim::Independent),
            review_order: Some(ReviewOrder::FirstOrder),
        };
        assert!(human_prov.is_human_voice());
        assert!(human_prov.is_independent());
        assert!(human_prov.is_first_order_review());
        assert!(human_prov.is_authentic_human_quorum_voice());
        assert!(!human_prov.is_synthetic());
    }

    #[test]
    fn provenance_synthetic_not_human() {
        let ai_prov = Provenance {
            actor: did("did:exo:ai1"),
            timestamp: "t".into(),
            action_hash: vec![1],
            signature: vec![1],
            public_key: None,
            voice_kind: Some(VoiceKind::Synthetic),
            independence: None,
            review_order: None,
        };
        assert!(!ai_prov.is_human_voice());
        assert!(ai_prov.is_synthetic());
        assert!(!ai_prov.is_independent());
    }

    #[test]
    fn provenance_unspecified_voice_not_human() {
        // Unattributed provenance is never assumed to be authentic human judgment
        let prov = Provenance {
            actor: did("did:exo:unknown"),
            timestamp: "t".into(),
            action_hash: vec![1],
            signature: vec![1],
            public_key: None,
            voice_kind: None,
            independence: None,
            review_order: None,
        };
        assert!(!prov.is_human_voice());
        assert!(!prov.is_synthetic());
        assert!(!prov.is_independent());
    }

    #[test]
    fn provenance_is_signed() {
        let signed = Provenance {
            actor: did("did:exo:actor"),
            timestamp: "2025-01-01".into(),
            action_hash: vec![1],
            signature: vec![4, 5, 6],
            public_key: None,
            voice_kind: None,
            independence: None,
            review_order: None,
        };
        assert!(signed.is_signed());

        let unsigned = Provenance {
            actor: did("did:exo:actor"),
            timestamp: "2025-01-01".into(),
            action_hash: vec![1],
            signature: vec![],
            public_key: None,
            voice_kind: None,
            independence: None,
            review_order: None,
        };
        assert!(!unsigned.is_signed());
    }
}
