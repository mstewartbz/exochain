//! Ceremony policy and certifier roster validation.

use std::collections::BTreeSet;

use exo_core::{Did, Hash256, PublicKey, Timestamp};
use serde::{Deserialize, Serialize};

use crate::{Result, RootError};

/// Institutional root threshold.
pub const ROOT_GENESIS_THRESHOLD: u16 = 7;

/// Institutional root roster size.
pub const ROOT_GENESIS_SIGNERS: u16 = 13;

/// Public contact and verification material for a root certifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CertifierContact {
    /// Certifier DID.
    pub did: Did,
    /// FROST signer identifier in the inclusive range 1..=13.
    pub frost_identifier: u16,
    /// Ed25519 public key used for signed portal envelopes.
    pub signing_public_key: PublicKey,
    /// X25519 public key used for recipient-bound round-two payloads.
    pub transport_public_key: [u8; 32],
}

/// Root genesis ceremony configuration bound into every transcript and bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisCeremonyConfig {
    /// Ceremony identifier chosen before DKG starts.
    pub ceremony_id: String,
    /// EXOCHAIN network identifier.
    pub network_id: String,
    /// Repository commit reviewed for the ceremony.
    pub repo_commit: String,
    /// Canonical hash of the governing constitution.
    pub constitution_hash: Hash256,
    /// Threshold required to sign root artifacts after genesis.
    pub threshold: u16,
    /// Total roster size.
    pub max_signers: u16,
    /// HLC timestamp supplied by the operator.
    pub created_at: Timestamp,
    /// Full certifier roster.
    pub certifiers: Vec<CertifierContact>,
    /// Predeclared deterministic signing set: exactly `threshold` rostered FROST
    /// identifiers chosen before commitments are emitted. Root artifacts are
    /// signed by this set unless a primary is unavailable before commitments, in
    /// which case the next unused `signing_alternates` member substitutes, in
    /// declared order. Fixing the set in the bound config removes coordinator
    /// discretion over which signers participate.
    pub signing_set: Vec<u16>,
    /// Ordered alternate signers (rostered, disjoint from `signing_set`) used only
    /// to replace a primary that is unavailable before commitments, taken in order.
    pub signing_alternates: Vec<u16>,
}

impl GenesisCeremonyConfig {
    /// Validate the constitutional root policy and roster uniqueness.
    pub fn validate(&self) -> Result<()> {
        if self.threshold != ROOT_GENESIS_THRESHOLD {
            return Err(RootError::InvalidConfig {
                reason: format!("threshold must be {ROOT_GENESIS_THRESHOLD}"),
            });
        }
        if self.max_signers != ROOT_GENESIS_SIGNERS {
            return Err(RootError::InvalidConfig {
                reason: format!("max_signers must be {ROOT_GENESIS_SIGNERS}"),
            });
        }
        if self.certifiers.len() != usize::from(ROOT_GENESIS_SIGNERS) {
            return Err(RootError::InvalidConfig {
                reason: format!("roster must contain {ROOT_GENESIS_SIGNERS} certifiers"),
            });
        }
        if self.ceremony_id.trim().is_empty() {
            return Err(RootError::InvalidConfig {
                reason: "ceremony_id must not be empty".to_owned(),
            });
        }
        if self.network_id.trim().is_empty() {
            return Err(RootError::InvalidConfig {
                reason: "network_id must not be empty".to_owned(),
            });
        }
        if self.repo_commit.len() != 40 || !self.repo_commit.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(RootError::InvalidConfig {
                reason: "repo_commit must be a 40-character hex commit".to_owned(),
            });
        }

        let mut dids = BTreeSet::new();
        let mut frost_ids = BTreeSet::new();
        let mut signing_keys = BTreeSet::new();
        let mut transport_keys = BTreeSet::new();
        for certifier in &self.certifiers {
            if certifier.frost_identifier == 0 || certifier.frost_identifier > ROOT_GENESIS_SIGNERS
            {
                return Err(RootError::InvalidConfig {
                    reason: format!(
                        "frost_identifier {} is outside 1..={ROOT_GENESIS_SIGNERS}",
                        certifier.frost_identifier
                    ),
                });
            }
            if !dids.insert(certifier.did.clone()) {
                return Err(RootError::InvalidConfig {
                    reason: format!("duplicate certifier DID {}", certifier.did),
                });
            }
            if !frost_ids.insert(certifier.frost_identifier) {
                return Err(RootError::InvalidConfig {
                    reason: format!("duplicate FROST identifier {}", certifier.frost_identifier),
                });
            }
            if !signing_keys.insert(certifier.signing_public_key) {
                return Err(RootError::InvalidConfig {
                    reason: "duplicate signing public key".to_owned(),
                });
            }
            if !transport_keys.insert(certifier.transport_public_key) {
                return Err(RootError::InvalidConfig {
                    reason: "duplicate transport public key".to_owned(),
                });
            }
        }

        self.validate_signing_set()?;

        Ok(())
    }

    /// Validate the predeclared signing set and alternates.
    fn validate_signing_set(&self) -> Result<()> {
        if self.signing_set.len() != usize::from(self.threshold) {
            return Err(RootError::InvalidConfig {
                reason: format!(
                    "signing_set must contain exactly {} signers",
                    self.threshold
                ),
            });
        }
        let mut primaries = BTreeSet::new();
        for identifier in &self.signing_set {
            if self.certifier_by_identifier(*identifier).is_none() {
                return Err(RootError::InvalidConfig {
                    reason: format!("signing_set member {identifier} is not rostered"),
                });
            }
            if !primaries.insert(*identifier) {
                return Err(RootError::InvalidConfig {
                    reason: format!("duplicate signing_set member {identifier}"),
                });
            }
        }
        let mut alternates = BTreeSet::new();
        for identifier in &self.signing_alternates {
            if self.certifier_by_identifier(*identifier).is_none() {
                return Err(RootError::InvalidConfig {
                    reason: format!("signing_alternate {identifier} is not rostered"),
                });
            }
            if primaries.contains(identifier) {
                return Err(RootError::InvalidConfig {
                    reason: format!("signing_alternate {identifier} is also a primary signer"),
                });
            }
            if !alternates.insert(*identifier) {
                return Err(RootError::InvalidConfig {
                    reason: format!("duplicate signing_alternate {identifier}"),
                });
            }
        }
        Ok(())
    }

    /// Validate that `submitted` is the canonical signing selection for this
    /// ceremony: exactly `threshold` signers, every one a declared primary or
    /// alternate, and any primary absent from the selection replaced by the
    /// leading unused alternates in declared order. This removes coordinator
    /// discretion — the only permitted deviation from the predeclared set is
    /// ordered alternate substitution for a primary unavailable before commitments.
    pub fn validate_signing_selection(&self, submitted: &BTreeSet<u16>) -> Result<()> {
        if submitted.len() != usize::from(self.threshold) {
            return Err(RootError::InvalidConfig {
                reason: format!(
                    "signing selection must contain exactly {} signers",
                    self.threshold
                ),
            });
        }
        let primaries: BTreeSet<u16> = self.signing_set.iter().copied().collect();
        let alternates: BTreeSet<u16> = self.signing_alternates.iter().copied().collect();
        for identifier in submitted {
            if !primaries.contains(identifier) && !alternates.contains(identifier) {
                return Err(RootError::InvalidConfig {
                    reason: format!(
                        "signer {identifier} is not in the declared signing set or alternates"
                    ),
                });
            }
        }
        let absent_primaries = self
            .signing_set
            .iter()
            .filter(|primary| !submitted.contains(primary))
            .count();
        // The canonical selection keeps present primaries and substitutes the
        // leading `absent_primaries` alternates, in declared order. If too few
        // alternates are declared, the expected set is smaller than the selection
        // and the equality check below rejects it.
        let mut expected: BTreeSet<u16> = self
            .signing_set
            .iter()
            .copied()
            .filter(|primary| submitted.contains(primary))
            .collect();
        for alternate in self.signing_alternates.iter().take(absent_primaries) {
            expected.insert(*alternate);
        }
        if &expected != submitted {
            return Err(RootError::InvalidConfig {
                reason: "signing selection must use declared alternates in order to replace \
                         unavailable primaries"
                    .to_owned(),
            });
        }
        Ok(())
    }

    /// Return the certifier with the supplied DID, if rostered.
    #[must_use]
    pub fn certifier_by_did(&self, did: &Did) -> Option<&CertifierContact> {
        self.certifiers
            .iter()
            .find(|certifier| &certifier.did == did)
    }

    /// Return the certifier with the supplied FROST identifier, if rostered.
    #[must_use]
    pub fn certifier_by_identifier(&self, identifier: u16) -> Option<&CertifierContact> {
        self.certifiers
            .iter()
            .find(|certifier| certifier.frost_identifier == identifier)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use exo_core::{Did, Hash256, PublicKey, Timestamp};

    use super::*;

    fn config() -> GenesisCeremonyConfig {
        let certifiers = (1..=ROOT_GENESIS_SIGNERS)
            .map(|index| {
                let byte = u8::try_from(index).expect("index fits");
                CertifierContact {
                    did: Did::new(&format!("did:exo:ceremony-unit-{index:02}")).expect("did"),
                    frost_identifier: index,
                    signing_public_key: PublicKey::from_bytes([byte; 32]),
                    transport_public_key: [byte; 32],
                }
            })
            .collect();
        GenesisCeremonyConfig {
            ceremony_id: "ceremony-unit".into(),
            network_id: "exochain-test".into(),
            repo_commit: "d8927686a34bdc28ba36d53938f665685d2c4c04".into(),
            constitution_hash: Hash256::digest(b"constitution"),
            threshold: ROOT_GENESIS_THRESHOLD,
            max_signers: ROOT_GENESIS_SIGNERS,
            created_at: Timestamp::new(1, 0),
            certifiers,
            signing_set: (1..=7).collect(),
            signing_alternates: (8..=13).collect(),
        }
    }

    fn selection(ids: &[u16]) -> BTreeSet<u16> {
        ids.iter().copied().collect()
    }

    #[test]
    fn valid_config_with_signing_set_validates() {
        config().validate().expect("config validates");
    }

    #[test]
    fn validate_rejects_malformed_signing_set_and_alternates() {
        let mut wrong_len = config();
        wrong_len.signing_set = (1..=6).collect();
        assert!(wrong_len.validate().is_err());

        let mut unrostered_primary = config();
        unrostered_primary.signing_set = vec![1, 2, 3, 4, 5, 6, 99];
        assert!(unrostered_primary.validate().is_err());

        let mut duplicate_primary = config();
        duplicate_primary.signing_set = vec![1, 2, 3, 4, 5, 6, 6];
        assert!(duplicate_primary.validate().is_err());

        let mut unrostered_alternate = config();
        unrostered_alternate.signing_alternates = vec![99];
        assert!(unrostered_alternate.validate().is_err());

        let mut alternate_is_primary = config();
        alternate_is_primary.signing_alternates = vec![1];
        assert!(alternate_is_primary.validate().is_err());

        let mut duplicate_alternate = config();
        duplicate_alternate.signing_alternates = vec![8, 8];
        assert!(duplicate_alternate.validate().is_err());
    }

    #[test]
    fn validate_signing_selection_accepts_declared_set_and_ordered_alternates() {
        let config = config();
        // The full predeclared primary set.
        config
            .validate_signing_selection(&selection(&[1, 2, 3, 4, 5, 6, 7]))
            .expect("declared set accepted");
        // Primary 1 unavailable → first alternate (8) substitutes.
        config
            .validate_signing_selection(&selection(&[2, 3, 4, 5, 6, 7, 8]))
            .expect("ordered alternate substitution accepted");
        // Primaries 1 and 2 unavailable → first two alternates (8, 9).
        config
            .validate_signing_selection(&selection(&[3, 4, 5, 6, 7, 8, 9]))
            .expect("two ordered alternates accepted");
    }

    #[test]
    fn validate_signing_selection_rejects_wrong_size_unknown_and_misordered() {
        let config = config();
        // Wrong size.
        assert!(
            config
                .validate_signing_selection(&selection(&[1, 2, 3, 4, 5, 6, 7, 8]))
                .is_err()
        );
        // Signer outside the declared pool.
        assert!(
            config
                .validate_signing_selection(&selection(&[1, 2, 3, 4, 5, 6, 99]))
                .is_err()
        );
        // Out-of-order alternate: primary 2 absent but alternate 9 used instead of 8.
        assert!(
            config
                .validate_signing_selection(&selection(&[1, 3, 4, 5, 6, 7, 9]))
                .is_err()
        );
    }
}
