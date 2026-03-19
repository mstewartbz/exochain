//! Bailment model — the legal foundation of consent in EXOCHAIN.
//!
//! A bailment is a trust relationship where a bailor entrusts property (data/authority)
//! to a bailee under specific terms. No action may proceed without an active bailment.

use exo_core::{Did, Hash256, Signature, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ConsentError;

/// The type of bailment relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BailmentType {
    /// Bailee holds data/authority in custody without processing rights.
    Custody,
    /// Bailee may process data under defined terms.
    Processing,
    /// Bailee may delegate authority to sub-bailees.
    Delegation,
    /// Emergency access — time-limited, requires justification.
    Emergency,
}

/// The lifecycle state of a bailment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BailmentStatus {
    Proposed,
    Active,
    Suspended,
    Terminated,
    Expired,
}

impl std::fmt::Display for BailmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "Proposed"),
            Self::Active => write!(f, "Active"),
            Self::Suspended => write!(f, "Suspended"),
            Self::Terminated => write!(f, "Terminated"),
            Self::Expired => write!(f, "Expired"),
        }
    }
}

/// A bailment record — the binding consent agreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bailment {
    pub id: String,
    pub bailor_did: Did,
    pub bailee_did: Did,
    pub bailment_type: BailmentType,
    pub terms_hash: Hash256,
    pub created: Timestamp,
    pub expires: Option<Timestamp>,
    pub status: BailmentStatus,
    pub signature: Signature,
}

/// Propose a new bailment. Returns a bailment in `Proposed` status.
#[must_use]
pub fn propose(bailor: &Did, bailee: &Did, terms: &[u8], bailment_type: BailmentType) -> Bailment {
    Bailment {
        id: Uuid::new_v4().to_string(),
        bailor_did: bailor.clone(),
        bailee_did: bailee.clone(),
        bailment_type,
        terms_hash: Hash256::digest(terms),
        created: Timestamp::now_utc(),
        expires: None,
        status: BailmentStatus::Proposed,
        signature: Signature::empty(),
    }
}

/// Accept a proposed bailment. Transitions `Proposed` -> `Active`.
///
/// # Errors
/// - `InvalidState` if not in `Proposed` status.
/// - `InvalidSignature` if the signature is the empty placeholder.
pub fn accept(bailment: &mut Bailment, bailee_signature: &Signature) -> Result<(), ConsentError> {
    if bailment.status != BailmentStatus::Proposed {
        return Err(ConsentError::InvalidState {
            expected: "Proposed".into(),
            actual: bailment.status.to_string(),
        });
    }
    if bailee_signature.is_empty() {
        return Err(ConsentError::InvalidSignature);
    }
    bailment.signature = *bailee_signature;
    bailment.status = BailmentStatus::Active;
    Ok(())
}

/// Terminate a bailment. Either bailor or bailee may terminate.
///
/// # Errors
/// - `InvalidState` if already terminated or expired.
/// - `Unauthorized` if actor is neither bailor nor bailee.
pub fn terminate(bailment: &mut Bailment, actor: &Did) -> Result<(), ConsentError> {
    if bailment.status == BailmentStatus::Terminated || bailment.status == BailmentStatus::Expired {
        return Err(ConsentError::InvalidState {
            expected: "Active, Proposed, or Suspended".into(),
            actual: bailment.status.to_string(),
        });
    }
    if *actor != bailment.bailor_did && *actor != bailment.bailee_did {
        return Err(ConsentError::Unauthorized(format!(
            "DID {actor} is neither bailor nor bailee"
        )));
    }
    bailment.status = BailmentStatus::Terminated;
    Ok(())
}

/// Check whether a bailment is currently active (status Active + not expired).
#[must_use]
pub fn is_active(bailment: &Bailment, now: &Timestamp) -> bool {
    if bailment.status != BailmentStatus::Active {
        return false;
    }
    match &bailment.expires {
        Some(exp) => !exp.is_expired(now),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> Did { Did::new("did:exo:alice").unwrap() }
    fn bob() -> Did { Did::new("did:exo:bob").unwrap() }
    fn charlie() -> Did { Did::new("did:exo:charlie").unwrap() }
    fn sig() -> Signature { Signature::from_bytes([1u8; 64]) }
    fn ts(ms: u64) -> Timestamp { Timestamp::new(ms, 0) }

    #[test]
    fn propose_creates_proposed() {
        let b = propose(&alice(), &bob(), b"terms", BailmentType::Custody);
        assert_eq!(b.status, BailmentStatus::Proposed);
        assert_eq!(b.bailor_did, alice());
        assert_eq!(b.bailee_did, bob());
        assert_eq!(b.bailment_type, BailmentType::Custody);
        assert!(b.signature.is_empty());
        assert!(b.expires.is_none());
        assert!(!b.id.is_empty());
    }

    #[test]
    fn propose_hashes_terms_deterministically() {
        let a = propose(&alice(), &bob(), b"terms-a", BailmentType::Processing);
        let b = propose(&alice(), &bob(), b"terms-b", BailmentType::Processing);
        assert_ne!(a.terms_hash, b.terms_hash);
        let c = propose(&alice(), &bob(), b"terms-a", BailmentType::Processing);
        assert_eq!(a.terms_hash, c.terms_hash);
    }

    #[test]
    fn accept_transitions_to_active() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        assert!(accept(&mut b, &sig()).is_ok());
        assert_eq!(b.status, BailmentStatus::Active);
        assert!(!b.signature.is_empty());
    }

    #[test]
    fn accept_rejects_non_proposed() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        b.status = BailmentStatus::Active;
        assert_eq!(
            accept(&mut b, &sig()),
            Err(ConsentError::InvalidState { expected: "Proposed".into(), actual: "Active".into() })
        );
    }

    #[test]
    fn accept_rejects_empty_signature() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        assert_eq!(accept(&mut b, &Signature::empty()), Err(ConsentError::InvalidSignature));
    }

    #[test]
    fn terminate_by_bailor() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        assert!(terminate(&mut b, &alice()).is_ok());
        assert_eq!(b.status, BailmentStatus::Terminated);
    }

    #[test]
    fn terminate_by_bailee() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        assert!(terminate(&mut b, &bob()).is_ok());
        assert_eq!(b.status, BailmentStatus::Terminated);
    }

    #[test]
    fn terminate_rejects_unauthorized() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        assert!(matches!(terminate(&mut b, &charlie()), Err(ConsentError::Unauthorized(_))));
    }

    #[test]
    fn terminate_rejects_already_terminated() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        terminate(&mut b, &alice()).ok();
        assert!(matches!(terminate(&mut b, &alice()), Err(ConsentError::InvalidState { .. })));
    }

    #[test]
    fn terminate_rejects_expired() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        b.status = BailmentStatus::Expired;
        assert!(matches!(terminate(&mut b, &alice()), Err(ConsentError::InvalidState { .. })));
    }

    #[test]
    fn terminate_proposed() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        assert!(terminate(&mut b, &alice()).is_ok());
    }

    #[test]
    fn terminate_suspended() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        b.status = BailmentStatus::Suspended;
        assert!(terminate(&mut b, &alice()).is_ok());
    }

    #[test]
    fn is_active_with_no_expiry() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        assert!(is_active(&b, &ts(5000)));
    }

    #[test]
    fn is_active_before_expiry() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        b.expires = Some(ts(10000));
        assert!(is_active(&b, &ts(5000)));
    }

    #[test]
    fn not_active_after_expiry() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        b.expires = Some(ts(1000));
        assert!(!is_active(&b, &ts(5000)));
    }

    #[test]
    fn not_active_at_exact_expiry() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        b.expires = Some(ts(5000));
        assert!(!is_active(&b, &ts(5000)));
    }

    #[test]
    fn not_active_when_proposed() {
        let b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        assert!(!is_active(&b, &ts(1000)));
    }

    #[test]
    fn not_active_when_terminated() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        accept(&mut b, &sig()).ok();
        terminate(&mut b, &alice()).ok();
        assert!(!is_active(&b, &ts(1000)));
    }

    #[test]
    fn not_active_when_suspended() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        b.status = BailmentStatus::Suspended;
        assert!(!is_active(&b, &ts(1000)));
    }

    #[test]
    fn not_active_when_expired_status() {
        let mut b = propose(&alice(), &bob(), b"t", BailmentType::Custody);
        b.status = BailmentStatus::Expired;
        assert!(!is_active(&b, &ts(1000)));
    }

    #[test]
    fn bailment_type_variants() {
        assert_ne!(BailmentType::Custody, BailmentType::Processing);
        assert_ne!(BailmentType::Delegation, BailmentType::Emergency);
        assert_eq!(BailmentType::Custody, BailmentType::Custody);
    }

    #[test]
    fn bailment_status_display() {
        assert_eq!(BailmentStatus::Proposed.to_string(), "Proposed");
        assert_eq!(BailmentStatus::Active.to_string(), "Active");
        assert_eq!(BailmentStatus::Suspended.to_string(), "Suspended");
        assert_eq!(BailmentStatus::Terminated.to_string(), "Terminated");
        assert_eq!(BailmentStatus::Expired.to_string(), "Expired");
    }
}
