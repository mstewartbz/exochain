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
