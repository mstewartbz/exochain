//! Threshold root signing helpers.

use std::collections::BTreeMap;

use frost_ristretto255 as frost;
use serde::{Deserialize, Serialize};

use crate::{
    GenesisCeremonyConfig, Result, RootError, RootKeyPackage, RootPublicKeyPackage,
    dkg::{deserialize_frost, serialize_frost},
};

/// Serialized threshold signature over a root artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootSignature {
    /// Serialized FROST signature.
    pub signature: Vec<u8>,
    /// Signer identifiers used for this signature.
    pub signer_ids: Vec<u16>,
}

fn frost_error(error: frost::Error) -> RootError {
    RootError::Frost {
        detail: error.to_string(),
    }
}

/// Create a FROST threshold signature from at least seven rostered shares.
pub fn threshold_sign<R>(
    config: &GenesisCeremonyConfig,
    public_key_package: &RootPublicKeyPackage,
    shares: BTreeMap<u16, RootKeyPackage>,
    message: &[u8],
    rng: &mut R,
) -> Result<RootSignature>
where
    R: frost::rand_core::RngCore + frost::rand_core::CryptoRng,
{
    config.validate()?;
    if shares.len() < usize::from(config.threshold) {
        let supplied = shares
            .keys()
            .take(usize::from(u16::MAX))
            .fold(0u16, |count, _| count.saturating_add(1));
        let error = RootError::ThresholdNotMet {
            required: config.threshold,
            supplied,
        };
        return Err(error);
    }

    let public = deserialize_frost(public_key_package.public_key_package.as_slice())?;

    let mut key_packages = BTreeMap::new();
    let mut signing_nonces = BTreeMap::new();
    let mut signing_commitments = BTreeMap::new();
    let mut signer_ids = Vec::new();

    for (identifier, share) in shares {
        if config.certifier_by_identifier(identifier).is_none() {
            return Err(RootError::InvalidConfig {
                reason: format!("signer {identifier} is not rostered"),
            });
        }
        if share.frost_identifier != identifier {
            let share_id = share.frost_identifier;
            let detail = format!("share id {share_id} mismatches key {identifier}");
            return Err(RootError::Frost { detail });
        }
        let frost_identifier = crate::dkg::frost_identifier(identifier)?;
        let key_package: frost::keys::KeyPackage = deserialize_frost(share.key_package.as_slice())?;
        if *key_package.identifier() != frost_identifier {
            return Err(RootError::Frost {
                detail: "deserialized key package identifier mismatch".to_owned(),
            });
        }
        let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), rng);
        signing_nonces.insert(frost_identifier, nonces);
        signing_commitments.insert(frost_identifier, commitments);
        key_packages.insert(frost_identifier, key_package);
        signer_ids.push(identifier);
    }

    let signing_package = frost::SigningPackage::new(signing_commitments, message);
    let mut signature_shares = BTreeMap::new();
    let sign_share = frost::round2::sign;
    for (identifier, key_package) in &key_packages {
        let nonces = &signing_nonces[identifier];
        let share = sign_share(&signing_package, nonces, key_package).map_err(frost_error)?;
        signature_shares.insert(*identifier, share);
    }

    let aggregate = frost::aggregate;
    let sig = aggregate(&signing_package, &signature_shares, &public).map_err(frost_error)?;
    let signature = serialize_frost(&sig)?;

    verify_root_signature(&public_key_package.root_public_key, message, &signature)?;

    Ok(RootSignature {
        signature,
        signer_ids,
    })
}

/// Verify a serialized root threshold signature against a root public key.
pub fn verify_root_signature(
    root_public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    let verifying_key: frost::VerifyingKey = deserialize_frost(root_public_key)?;
    let signature: frost::Signature = deserialize_frost(signature)?;
    verifying_key
        .verify(message, &signature)
        .map_err(signature_rejected)
}

fn signature_rejected(error: frost::Error) -> RootError {
    RootError::SignatureRejected {
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use exo_core::{Did, Hash256, PublicKey, Timestamp};
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::CertifierContact;

    fn test_config() -> GenesisCeremonyConfig {
        let certifiers = (1..=13)
            .map(|identifier| {
                let byte = u8::try_from(identifier).expect("identifier fits");
                CertifierContact {
                    did: Did::new(&format!("did:exo:signing-unit-{identifier:02}"))
                        .expect("valid did"),
                    frost_identifier: identifier,
                    signing_public_key: PublicKey::from_bytes([byte; 32]),
                    transport_public_key: [byte; 32],
                }
            })
            .collect();
        GenesisCeremonyConfig {
            ceremony_id: "unit-root".into(),
            network_id: "unit-net".into(),
            repo_commit: "d8927686a34bdc28ba36d53938f665685d2c4c04".into(),
            constitution_hash: Hash256::digest(b"constitution"),
            threshold: 7,
            max_signers: 13,
            created_at: Timestamp::new(1, 0),
            certifiers,
        }
    }

    #[test]
    fn frost_error_conversion_is_diagnostic() {
        let error = frost::Identifier::try_from(0).expect_err("zero identifier");
        let converted = frost_error(error);
        assert!(converted.to_string().contains("frost operation failed"));
    }

    #[test]
    fn signature_rejection_conversion_is_diagnostic() {
        let error = frost::Identifier::try_from(0).expect_err("zero identifier");
        let converted = signature_rejected(error);
        assert!(
            converted
                .to_string()
                .contains("signature verification failed")
        );
    }

    #[test]
    fn threshold_sign_rejects_too_few_shares_before_public_deserialization() {
        let config = test_config();
        let public_key_package = RootPublicKeyPackage {
            public_key_package: b"not a public package".to_vec(),
            root_public_key: b"not a verifying key".to_vec(),
            verifying_shares: BTreeMap::new(),
        };
        let mut rng = StdRng::seed_from_u64(7);
        let error = threshold_sign(
            &config,
            &public_key_package,
            BTreeMap::new(),
            b"root artifact",
            &mut rng,
        )
        .expect_err("empty signer set");
        assert_eq!(
            error,
            RootError::ThresholdNotMet {
                required: 7,
                supplied: 0
            }
        );
    }

    #[test]
    fn threshold_sign_covers_success_and_share_mismatch_paths() {
        let config = test_config();
        let mut rng = StdRng::seed_from_u64(71);
        let dkg = crate::run_complete_dkg(&config, &mut rng).expect("dkg");
        let selected: BTreeMap<u16, _> = dkg
            .key_packages
            .iter()
            .take(7)
            .map(|(identifier, share)| (*identifier, share.clone()))
            .collect();
        let message = b"unit root signing artifact";
        let signature = threshold_sign(
            &config,
            &dkg.public_key_package,
            selected.clone(),
            message,
            &mut rng,
        )
        .expect("signature");

        verify_root_signature(
            &dkg.public_key_package.root_public_key,
            message,
            &signature.signature,
        )
        .expect("signature verifies");

        let mut mismatched = selected;
        let mut share = mismatched.remove(&1).expect("share one");
        share.frost_identifier = 2;
        mismatched.insert(1, share);
        let error = threshold_sign(
            &config,
            &dkg.public_key_package,
            mismatched,
            message,
            &mut rng,
        )
        .expect_err("share identifier mismatch");
        assert!(error.to_string().contains("mismatches key 1"));
    }
}
