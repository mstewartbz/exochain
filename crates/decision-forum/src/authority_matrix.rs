//! Real-time delegated authority matrix (GOV-003, GOV-004).
//!
//! Maps Actor -> Vec<DelegatedAuthority>. Each delegation is signed,
//! scoped, time-bound, and revocable. Auto-expiry enforcement (TNC-05),
//! sub-delegation control, and sunset/renewal tracking with 90/60/30/14/7-day
//! expiry warnings.

use exo_core::types::{DeterministicMap, Did, Hash256, Timestamp};
use serde::{Deserialize, Serialize};

use crate::{
    decision_object::DecisionClass,
    error::{ForumError, Result},
};

/// Scope of a delegation — what actions the delegate can perform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationScope {
    pub decision_classes: Vec<DecisionClass>,
    pub description: String,
}

/// A single delegated authority record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedAuthority {
    pub id: String,
    pub delegator: Did,
    pub delegate: Did,
    pub scope: DelegationScope,
    pub granted_at: Timestamp,
    pub expires_at: Timestamp,
    pub revoked: bool,
    pub allows_sub_delegation: bool,
    pub signature_hash: Hash256,
}

impl DelegatedAuthority {
    /// Check whether this delegation is currently active at the given time.
    #[must_use]
    pub fn is_active(&self, now: &Timestamp) -> bool {
        !self.revoked && !self.expires_at.is_expired(now)
    }

    /// Check whether this delegation covers a given decision class.
    #[must_use]
    pub fn covers_class(&self, class: DecisionClass) -> bool {
        self.scope.decision_classes.contains(&class)
    }

    /// Calculate days until expiry from a given timestamp.
    /// Returns 0 if already expired.
    #[must_use]
    pub fn days_until_expiry(&self, now: &Timestamp) -> u64 {
        if self.expires_at.physical_ms <= now.physical_ms {
            return 0;
        }
        let diff_ms = self.expires_at.physical_ms - now.physical_ms;
        diff_ms / (24 * 60 * 60 * 1000)
    }
}

/// Warning thresholds for delegation expiry (days).
pub const EXPIRY_WARNING_DAYS: &[u64] = &[90, 60, 30, 14, 7];

/// The authority matrix for a tenant — maps actors to their delegations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityMatrix {
    pub delegations: DeterministicMap<String, Vec<DelegatedAuthority>>,
}

impl AuthorityMatrix {
    /// Create an empty authority matrix.
    #[must_use]
    pub fn new() -> Self {
        Self {
            delegations: DeterministicMap::new(),
        }
    }

    /// Grant a new delegation.
    pub fn grant(&mut self, delegation: DelegatedAuthority) -> Result<()> {
        let key = delegation.delegate.as_str().to_owned();
        let entries = self.delegations.get(&key).cloned().unwrap_or_default();
        let mut entries = entries;
        entries.push(delegation);
        self.delegations.insert(key, entries);
        Ok(())
    }

    /// Revoke a delegation by ID for a specific delegate DID.
    pub fn revoke(&mut self, delegate_did: &Did, delegation_id: &str) -> Result<()> {
        let key = delegate_did.as_str().to_owned();
        if let Some(entries) = self.delegations.get(&key) {
            let mut entries = entries.clone();
            let mut found = false;
            for entry in &mut entries {
                if entry.id == delegation_id {
                    entry.revoked = true;
                    found = true;
                }
            }
            if !found {
                return Err(ForumError::AuthorityInvalid {
                    reason: format!("delegation {delegation_id} not found"),
                });
            }
            self.delegations.insert(key, entries);
            Ok(())
        } else {
            Err(ForumError::AuthorityInvalid {
                reason: format!("no delegations for {delegate_did}"),
            })
        }
    }

    /// Get all active delegations for a delegate at a given time.
    #[must_use]
    pub fn active_delegations(
        &self,
        delegate_did: &Did,
        now: &Timestamp,
    ) -> Vec<&DelegatedAuthority> {
        let key = delegate_did.as_str().to_owned();
        self.delegations
            .get(&key)
            .map(|entries| entries.iter().filter(|d| d.is_active(now)).collect())
            .unwrap_or_default()
    }

    /// Check if an actor has authority for a given decision class at a given time.
    #[must_use]
    pub fn has_authority(&self, actor: &Did, class: DecisionClass, now: &Timestamp) -> bool {
        self.active_delegations(actor, now)
            .iter()
            .any(|d| d.covers_class(class))
    }

    /// Purge all expired delegations. Returns the number removed.
    pub fn purge_expired(&mut self, now: &Timestamp) -> usize {
        let mut count = 0;
        let keys: Vec<String> = self.delegations.keys().cloned().collect();
        for key in keys {
            if let Some(entries) = self.delegations.get(&key) {
                let before = entries.len();
                let remaining: Vec<DelegatedAuthority> = entries
                    .iter()
                    .filter(|d| d.is_active(now))
                    .cloned()
                    .collect();
                count += before - remaining.len();
                self.delegations.insert(key, remaining);
            }
        }
        count
    }

    /// Collect all delegations approaching expiry within any warning threshold.
    #[must_use]
    pub fn expiry_warnings(&self, now: &Timestamp) -> Vec<(&DelegatedAuthority, u64)> {
        let mut warnings = Vec::new();
        for (_, entries) in self.delegations.iter() {
            for d in entries {
                if !d.is_active(now) {
                    continue;
                }
                let days = d.days_until_expiry(now);
                for &threshold in EXPIRY_WARNING_DAYS {
                    if days <= threshold {
                        warnings.push((d, days));
                        break;
                    }
                }
            }
        }
        warnings
    }

    /// Attempt sub-delegation: a delegate creating a new delegation.
    pub fn sub_delegate(
        &mut self,
        parent_delegate: &Did,
        parent_delegation_id: &str,
        new_delegation: DelegatedAuthority,
        now: &Timestamp,
    ) -> Result<()> {
        let key = parent_delegate.as_str().to_owned();
        let parent = self
            .delegations
            .get(&key)
            .and_then(|entries| entries.iter().find(|d| d.id == parent_delegation_id))
            .ok_or_else(|| ForumError::AuthorityInvalid {
                reason: "parent delegation not found".into(),
            })?;

        if !parent.is_active(now) {
            return Err(ForumError::DelegationExpired);
        }
        if !parent.allows_sub_delegation {
            return Err(ForumError::SubDelegationNotPermitted);
        }

        // Sub-delegation scope must be a subset of parent scope
        for class in &new_delegation.scope.decision_classes {
            if !parent.covers_class(*class) {
                return Err(ForumError::DelegationScopeExceeded {
                    reason: format!("{class:?} not in parent scope"),
                });
            }
        }

        self.grant(new_delegation)
    }
}

impl Default for AuthorityMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn did(n: &str) -> Did {
        Did::new(&format!("did:exo:{n}")).expect("valid")
    }
    fn now() -> Timestamp {
        Timestamp::new(1_000_000_000, 0)
    }
    fn future() -> Timestamp {
        Timestamp::new(1_100_000_000, 0)
    } // ~1.16 days from now
    fn past() -> Timestamp {
        Timestamp::new(500_000_000, 0)
    }

    fn make_delegation(id: &str, delegator: &str, delegate: &str, sub: bool) -> DelegatedAuthority {
        DelegatedAuthority {
            id: id.into(),
            delegator: did(delegator),
            delegate: did(delegate),
            scope: DelegationScope {
                decision_classes: vec![DecisionClass::Routine, DecisionClass::Operational],
                description: "test".into(),
            },
            granted_at: past(),
            expires_at: future(),
            revoked: false,
            allows_sub_delegation: sub,
            signature_hash: Hash256::digest(id.as_bytes()),
        }
    }

    #[test]
    fn grant_and_query() {
        let mut m = AuthorityMatrix::new();
        m.grant(make_delegation("d1", "root", "alice", false))
            .expect("ok");
        assert!(m.has_authority(&did("alice"), DecisionClass::Routine, &now()));
        assert!(!m.has_authority(&did("alice"), DecisionClass::Strategic, &now()));
        assert!(!m.has_authority(&did("bob"), DecisionClass::Routine, &now()));
    }

    #[test]
    fn revoke() {
        let mut m = AuthorityMatrix::new();
        m.grant(make_delegation("d1", "root", "alice", false))
            .expect("ok");
        m.revoke(&did("alice"), "d1").expect("ok");
        assert!(!m.has_authority(&did("alice"), DecisionClass::Routine, &now()));
    }

    #[test]
    fn revoke_not_found() {
        let mut m = AuthorityMatrix::new();
        m.grant(make_delegation("d1", "root", "alice", false))
            .expect("ok");
        assert!(m.revoke(&did("alice"), "d99").is_err());
    }

    #[test]
    fn expired_delegation_inactive() {
        let mut d = make_delegation("d1", "root", "alice", false);
        d.expires_at = past();
        assert!(!d.is_active(&now()));
    }

    #[test]
    fn purge_expired() {
        let mut m = AuthorityMatrix::new();
        let mut d = make_delegation("d1", "root", "alice", false);
        d.expires_at = past();
        m.grant(d).expect("ok");
        m.grant(make_delegation("d2", "root", "alice", false))
            .expect("ok");
        let purged = m.purge_expired(&now());
        assert_eq!(purged, 1);
        assert_eq!(m.active_delegations(&did("alice"), &now()).len(), 1);
    }

    #[test]
    fn sub_delegation_ok() {
        let mut m = AuthorityMatrix::new();
        m.grant(make_delegation("d1", "root", "alice", true))
            .expect("ok");
        let sub = DelegatedAuthority {
            id: "d2".into(),
            delegator: did("alice"),
            delegate: did("bob"),
            scope: DelegationScope {
                decision_classes: vec![DecisionClass::Routine],
                description: "sub".into(),
            },
            granted_at: now(),
            expires_at: future(),
            revoked: false,
            allows_sub_delegation: false,
            signature_hash: Hash256::digest(b"d2"),
        };
        m.sub_delegate(&did("alice"), "d1", sub, &now())
            .expect("ok");
        assert!(m.has_authority(&did("bob"), DecisionClass::Routine, &now()));
    }

    #[test]
    fn sub_delegation_not_permitted() {
        let mut m = AuthorityMatrix::new();
        m.grant(make_delegation("d1", "root", "alice", false))
            .expect("ok");
        let sub = make_delegation("d2", "alice", "bob", false);
        let err = m
            .sub_delegate(&did("alice"), "d1", sub, &now())
            .unwrap_err();
        assert!(matches!(err, ForumError::SubDelegationNotPermitted));
    }

    #[test]
    fn sub_delegation_scope_exceeded() {
        let mut m = AuthorityMatrix::new();
        m.grant(make_delegation("d1", "root", "alice", true))
            .expect("ok");
        let mut sub = make_delegation("d2", "alice", "bob", false);
        sub.scope.decision_classes = vec![DecisionClass::Strategic];
        let err = m
            .sub_delegate(&did("alice"), "d1", sub, &now())
            .unwrap_err();
        assert!(matches!(err, ForumError::DelegationScopeExceeded { .. }));
    }

    #[test]
    fn days_until_expiry() {
        let d = make_delegation("d1", "root", "alice", false);
        let days = d.days_until_expiry(&now());
        // future() is 2_000_000ms, now() is 1_000_000ms => ~11 days
        assert!(days > 0);
    }

    #[test]
    fn expiry_warnings() {
        let mut m = AuthorityMatrix::new();
        let mut d = make_delegation("d1", "root", "alice", false);
        // Expires in 5 days from now
        let five_days_ms = 5 * 24 * 60 * 60 * 1000;
        d.expires_at = Timestamp::new(now().physical_ms + five_days_ms, 0);
        m.grant(d).expect("ok");
        let warnings = m.expiry_warnings(&now());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].1, 5);
    }

    #[test]
    fn default() {
        let m = AuthorityMatrix::default();
        assert!(m.delegations.is_empty());
    }
}
