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
    /// signed only by this exact set. If any signer is unavailable, the ceremony
    /// aborts and restarts with a new signed config and ceremony id.
    pub signing_set: Vec<u16>,
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

    /// Validate the predeclared signing set.
    fn validate_signing_set(&self) -> Result<()> {
        if self.signing_set.len() != usize::from(self.threshold) {
            return Err(RootError::InvalidConfig {
                reason: format!(
                    "signing_set must contain exactly {} signers",
                    self.threshold
                ),
            });
        }
        let mut declared = BTreeSet::new();
        for identifier in &self.signing_set {
            if self.certifier_by_identifier(*identifier).is_none() {
                return Err(RootError::InvalidConfig {
                    reason: format!("signing_set member {identifier} is not rostered"),
                });
            }
            if !declared.insert(*identifier) {
                return Err(RootError::InvalidConfig {
                    reason: format!("duplicate signing_set member {identifier}"),
                });
            }
        }
        Ok(())
    }

    /// Validate that `submitted` is the canonical signing selection for this
    /// ceremony: exactly the predeclared `signing_set`. There is no in-ceremony
    /// alternate substitution; an unavailable signer aborts the ceremony and forces
    /// a new config/ceremony id.
    pub fn validate_signing_selection(&self, submitted: &BTreeSet<u16>) -> Result<()> {
        if submitted.len() != usize::from(self.threshold) {
            return Err(RootError::InvalidConfig {
                reason: format!(
                    "signing selection must contain exactly {} signers",
                    self.threshold
                ),
            });
        }
        let expected: BTreeSet<u16> = self.signing_set.iter().copied().collect();
        if &expected != submitted {
            return Err(RootError::InvalidConfig {
                reason: "signing selection must exactly match the predeclared signing_set"
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
    fn validate_rejects_malformed_signing_set() {
        let mut wrong_len = config();
        wrong_len.signing_set = (1..=6).collect();
        assert!(wrong_len.validate().is_err());

        let mut unrostered_primary = config();
        unrostered_primary.signing_set = vec![1, 2, 3, 4, 5, 6, 99];
        assert!(unrostered_primary.validate().is_err());

        let mut duplicate_primary = config();
        duplicate_primary.signing_set = vec![1, 2, 3, 4, 5, 6, 6];
        assert!(duplicate_primary.validate().is_err());
    }

    #[test]
    fn validate_signing_selection_accepts_only_declared_set() {
        let config = config();
        config
            .validate_signing_selection(&selection(&[1, 2, 3, 4, 5, 6, 7]))
            .expect("declared set accepted");
    }

    #[test]
    fn validate_signing_selection_rejects_wrong_size_unknown_and_substitution() {
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
        // No alternate substitution: primary 7 absent and alternate 8 present.
        assert!(
            config
                .validate_signing_selection(&selection(&[1, 2, 3, 4, 5, 6, 8]))
                .is_err()
        );
    }
}
