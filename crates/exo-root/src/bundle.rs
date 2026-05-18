//! Root trust bundle assembly and verification.

use std::fmt::Display;

use exo_authority::permission::Permission;
use exo_core::{Did, Hash256, PublicKey, Timestamp, hash::hash_structured};
use serde::{Deserialize, Serialize};

use crate::{
    GenesisCeremonyConfig, Result, RootError, RootPublicKeyPackage, verify_root_signature,
};

/// Operational AVC issuer authority delegated by the root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootIssuerDelegation {
    /// Operational issuer DID.
    pub issuer_did: Did,
    /// Operational issuer public key.
    pub issuer_public_key: PublicKey,
    /// Permissions granted by the root authority.
    pub granted_permissions: Vec<Permission>,
    /// HLC activation timestamp.
    pub effective_at: Timestamp,
    /// Optional HLC expiry timestamp.
    pub expires_at: Option<Timestamp>,
    /// Human-readable bounded purpose.
    pub purpose: String,
}

/// Root trust bundle produced by genesis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootTrustBundle {
    /// Ceremony configuration.
    pub config: GenesisCeremonyConfig,
    /// Public FROST package and root key.
    pub public_key_package: RootPublicKeyPackage,
    /// Root-signed AVC issuer delegation.
    pub issuer_delegation: RootIssuerDelegation,
    /// Canonical transcript hash.
    pub transcript_hash: Hash256,
    /// Root threshold signature over the trust artifact payload.
    pub root_signature: Vec<u8>,
    /// Canonical bundle content identifier.
    pub bundle_id: Hash256,
}

#[derive(Serialize)]
struct RootArtifactPayload<'a> {
    domain: &'static str,
    config_hash: Hash256,
    public_key_package_hash: Hash256,
    transcript_hash: Hash256,
    issuer_delegation_hash: Hash256,
    issuer_did: &'a Did,
}

#[derive(Serialize)]
struct RootBundleIdPayload<'a> {
    domain: &'static str,
    artifact_payload_hash: Hash256,
    root_signature: &'a [u8],
}

fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).map_err(canonical_encoding_error)?;
    Ok(bytes)
}

fn structured_hash<T: Serialize>(value: &T) -> Result<Hash256> {
    hash_structured(value).map_err(canonical_encoding_error)
}

fn canonical_encoding_error(error: impl Display) -> RootError {
    RootError::CanonicalEncoding {
        detail: error.to_string(),
    }
}

impl RootIssuerDelegation {
    /// Canonical payload signed by the root threshold authority.
    pub fn root_artifact_payload(
        &self,
        config: &GenesisCeremonyConfig,
        public_key_package: &RootPublicKeyPackage,
        transcript_hash: Hash256,
    ) -> Result<Vec<u8>> {
        config.validate()?;
        if self.purpose.trim().is_empty() {
            return Err(RootError::BundleRejected {
                reason: "issuer delegation purpose must not be empty".to_owned(),
            });
        }
        if self.granted_permissions.is_empty() {
            return Err(RootError::BundleRejected {
                reason: "issuer delegation must grant at least one permission".to_owned(),
            });
        }
        let payload = RootArtifactPayload {
            domain: "EXOCHAIN_ROOT_ARTIFACT_V1",
            config_hash: structured_hash(config)?,
            public_key_package_hash: structured_hash(public_key_package)?,
            transcript_hash,
            issuer_delegation_hash: structured_hash(self)?,
            issuer_did: &self.issuer_did,
        };
        canonical_bytes(&payload)
    }
}

fn bundle_id(
    delegation: &RootIssuerDelegation,
    config: &GenesisCeremonyConfig,
    public_key_package: &RootPublicKeyPackage,
    transcript_hash: Hash256,
    root_signature: &[u8],
) -> Result<Hash256> {
    let artifact_payload =
        delegation.root_artifact_payload(config, public_key_package, transcript_hash)?;
    let id_payload = RootBundleIdPayload {
        domain: "EXOCHAIN_ROOT_BUNDLE_V1",
        artifact_payload_hash: Hash256::digest(&artifact_payload),
        root_signature,
    };
    structured_hash(&id_payload)
}

/// Assemble and verify a root trust bundle.
pub fn assemble_root_bundle(
    config: GenesisCeremonyConfig,
    public_key_package: RootPublicKeyPackage,
    issuer_delegation: RootIssuerDelegation,
    transcript_hash: Hash256,
    root_signature: Vec<u8>,
) -> Result<RootTrustBundle> {
    let payload =
        issuer_delegation.root_artifact_payload(&config, &public_key_package, transcript_hash)?;
    verify_root_signature(
        &public_key_package.root_public_key,
        &payload,
        root_signature.as_slice(),
    )?;
    let bundle_id = bundle_id(
        &issuer_delegation,
        &config,
        &public_key_package,
        transcript_hash,
        root_signature.as_slice(),
    )?;
    Ok(RootTrustBundle {
        config,
        public_key_package,
        issuer_delegation,
        transcript_hash,
        root_signature,
        bundle_id,
    })
}

/// Verify that a root trust bundle is self-consistent and root-signed.
pub fn verify_root_bundle(bundle: &RootTrustBundle) -> Result<()> {
    bundle.config.validate()?;
    let payload = bundle.issuer_delegation.root_artifact_payload(
        &bundle.config,
        &bundle.public_key_package,
        bundle.transcript_hash,
    )?;
    verify_root_signature(
        &bundle.public_key_package.root_public_key,
        &payload,
        bundle.root_signature.as_slice(),
    )?;
    let expected_id = bundle_id(
        &bundle.issuer_delegation,
        &bundle.config,
        &bundle.public_key_package,
        bundle.transcript_hash,
        bundle.root_signature.as_slice(),
    )?;
    if expected_id != bundle.bundle_id {
        return Err(RootError::BundleRejected {
            reason: "bundle identifier does not match contents".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_error_conversion_is_diagnostic() {
        let error = canonical_encoding_error("encoder failed");
        assert!(error.to_string().contains("encoder failed"));
    }
}
