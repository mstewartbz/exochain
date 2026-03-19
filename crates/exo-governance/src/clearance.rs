//! Clearance with independence enforcement.

use std::collections::HashMap;
use exo_core::Did;
use serde::{Deserialize, Serialize};
use crate::quorum::QuorumPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClearanceLevel { None, ReadOnly, Contributor, Reviewer, Steward, Governor }

impl std::fmt::Display for ClearanceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"), Self::ReadOnly => write!(f, "ReadOnly"),
            Self::Contributor => write!(f, "Contributor"), Self::Reviewer => write!(f, "Reviewer"),
            Self::Steward => write!(f, "Steward"), Self::Governor => write!(f, "Governor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPolicy { pub required_level: ClearanceLevel, pub quorum_policy: Option<QuorumPolicy> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClearancePolicy { pub actions: HashMap<String, ActionPolicy> }

#[derive(Debug, Clone, Default)]
pub struct ClearanceRegistry { pub entries: HashMap<Did, ClearanceLevel> }

impl ClearanceRegistry {
    #[must_use]
    pub fn get_level(&self, actor: &Did) -> ClearanceLevel {
        self.entries.get(actor).copied().unwrap_or(ClearanceLevel::None)
    }
    pub fn set_level(&mut self, actor: Did, level: ClearanceLevel) {
        self.entries.insert(actor, level);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClearanceDecision {
    Granted,
    Denied { missing_level: ClearanceLevel },
    InsufficientIndependence { details: String },
}

#[must_use]
pub fn check_clearance(actor: &Did, action: &str, policy: &ClearancePolicy, registry: &ClearanceRegistry) -> ClearanceDecision {
    let ap = match policy.actions.get(action) {
        Some(ap) => ap,
        None => return ClearanceDecision::Denied { missing_level: ClearanceLevel::Governor },
    };
    if registry.get_level(actor) < ap.required_level {
        return ClearanceDecision::Denied { missing_level: ap.required_level };
    }
    ClearanceDecision::Granted
}

#[cfg(test)]
mod tests {
    use super::*;
    fn did(name: &str) -> Did { Did::new(&format!("did:exo:{name}")).expect("ok") }

    fn setup() -> (ClearancePolicy, ClearanceRegistry) {
        let mut p = ClearancePolicy::default();
        p.actions.insert("read".into(), ActionPolicy { required_level: ClearanceLevel::ReadOnly, quorum_policy: None });
        p.actions.insert("write".into(), ActionPolicy { required_level: ClearanceLevel::Contributor, quorum_policy: None });
        p.actions.insert("review".into(), ActionPolicy { required_level: ClearanceLevel::Reviewer, quorum_policy: None });
        p.actions.insert("govern".into(), ActionPolicy { required_level: ClearanceLevel::Governor, quorum_policy: None });
        let mut r = ClearanceRegistry::default();
        r.set_level(did("alice"), ClearanceLevel::Governor);
        r.set_level(did("bob"), ClearanceLevel::Contributor);
        r.set_level(did("carol"), ClearanceLevel::ReadOnly);
        (p, r)
    }

    #[test] fn governor_can_do_everything() {
        let (p, r) = setup(); let a = did("alice");
        assert_eq!(check_clearance(&a, "read", &p, &r), ClearanceDecision::Granted);
        assert_eq!(check_clearance(&a, "write", &p, &r), ClearanceDecision::Granted);
        assert_eq!(check_clearance(&a, "review", &p, &r), ClearanceDecision::Granted);
        assert_eq!(check_clearance(&a, "govern", &p, &r), ClearanceDecision::Granted);
    }
    #[test] fn contributor_cannot_review() {
        let (p, r) = setup(); let b = did("bob");
        assert_eq!(check_clearance(&b, "read", &p, &r), ClearanceDecision::Granted);
        assert_eq!(check_clearance(&b, "write", &p, &r), ClearanceDecision::Granted);
        assert_eq!(check_clearance(&b, "review", &p, &r), ClearanceDecision::Denied { missing_level: ClearanceLevel::Reviewer });
        assert_eq!(check_clearance(&b, "govern", &p, &r), ClearanceDecision::Denied { missing_level: ClearanceLevel::Governor });
    }
    #[test] fn readonly_can_only_read() {
        let (p, r) = setup(); let c = did("carol");
        assert_eq!(check_clearance(&c, "read", &p, &r), ClearanceDecision::Granted);
        assert_eq!(check_clearance(&c, "write", &p, &r), ClearanceDecision::Denied { missing_level: ClearanceLevel::Contributor });
    }
    #[test] fn unknown_actor_denied() {
        let (p, r) = setup();
        assert_eq!(check_clearance(&did("unknown"), "read", &p, &r), ClearanceDecision::Denied { missing_level: ClearanceLevel::ReadOnly });
    }
    #[test] fn unknown_action_denied() {
        let (p, r) = setup();
        assert_eq!(check_clearance(&did("alice"), "nonexistent", &p, &r), ClearanceDecision::Denied { missing_level: ClearanceLevel::Governor });
    }
    #[test] fn level_ordering() {
        assert!(ClearanceLevel::None < ClearanceLevel::ReadOnly);
        assert!(ClearanceLevel::ReadOnly < ClearanceLevel::Contributor);
        assert!(ClearanceLevel::Contributor < ClearanceLevel::Reviewer);
        assert!(ClearanceLevel::Reviewer < ClearanceLevel::Steward);
        assert!(ClearanceLevel::Steward < ClearanceLevel::Governor);
    }
    #[test] fn level_display() {
        assert_eq!(ClearanceLevel::None.to_string(), "None");
        assert_eq!(ClearanceLevel::Governor.to_string(), "Governor");
    }
    #[test] fn registry_defaults_to_none() {
        assert_eq!(ClearanceRegistry::default().get_level(&did("nobody")), ClearanceLevel::None);
    }
    #[test] fn registry_set_get() {
        let mut r = ClearanceRegistry::default();
        let d = did("test");
        r.set_level(d.clone(), ClearanceLevel::Steward);
        assert_eq!(r.get_level(&d), ClearanceLevel::Steward);
    }
    #[test] fn insufficient_independence_variant() {
        let d = ClearanceDecision::InsufficientIndependence { details: "test".into() };
        assert!(matches!(d, ClearanceDecision::InsufficientIndependence { .. }));
    }
}
