//! Trusted Execution Environment (TEE) attestation.
//!
//! Provides attestation verification for hardware TEE platforms
//! and a simulated platform for testing.

use serde::{Deserialize, Serialize};

use crate::error::GatekeeperError;

// ---------------------------------------------------------------------------
// TEE platform
// ---------------------------------------------------------------------------

/// Supported TEE platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeePlatform {
    /// Intel SGX.
    Sgx,
    /// ARM TrustZone.
    TrustZone,
    /// AMD SEV.
    Sev,
    /// Simulated TEE for testing.
    Simulated,
}

// ---------------------------------------------------------------------------
// TEE attestation
// ---------------------------------------------------------------------------

/// An attestation from a TEE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAttestation {
    /// The platform that produced this attestation.
    pub platform: TeePlatform,
    /// Blake3 hash of the enclave measurement.
    pub measurement_hash: [u8; 32],
    /// Timestamp (milliseconds since epoch).
    pub timestamp: u64,
    /// Signature over the measurement + timestamp.
    pub signature: Vec<u8>,
}

// ---------------------------------------------------------------------------
// TEE policy
// ---------------------------------------------------------------------------

/// Policy defining acceptable TEE attestations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeePolicy {
    /// Accepted platforms.
    pub accepted_platforms: Vec<TeePlatform>,
    /// Required measurement hashes (if non-empty, attestation must match one).
    pub required_measurements: Vec<[u8; 32]>,
    /// Maximum attestation age in milliseconds (0 = no age limit).
    pub max_age_ms: u64,
    /// Current time in milliseconds (for age checking).
    pub current_time_ms: u64,
}

// ---------------------------------------------------------------------------
// Attestation generation
// ---------------------------------------------------------------------------

/// Generate an attestation for the given platform and measurement.
///
/// In production, this would interface with the actual TEE hardware.
/// For now, it produces a deterministic attestation with a blake3-based signature.
#[must_use]
pub fn generate_attestation(
    platform: &TeePlatform,
    measurement: &[u8],
    timestamp: u64,
) -> TeeAttestation {
    let measurement_hash = *blake3::hash(measurement).as_bytes();

    // Deterministic signature: hash(measurement_hash || timestamp || platform).
    let mut sig_input = Vec::new();
    sig_input.extend_from_slice(&measurement_hash);
    sig_input.extend_from_slice(&timestamp.to_le_bytes());
    sig_input.extend_from_slice(format!("{:?}", platform).as_bytes());
    let signature = blake3::hash(&sig_input).as_bytes().to_vec();

    TeeAttestation {
        platform: *platform,
        measurement_hash,
        timestamp,
        signature,
    }
}

// ---------------------------------------------------------------------------
// Attestation verification
// ---------------------------------------------------------------------------

/// Verify a TEE attestation against a policy.
pub fn verify_attestation(
    attestation: &TeeAttestation,
    policy: &TeePolicy,
) -> Result<(), GatekeeperError> {
    // Check platform.
    if !policy.accepted_platforms.contains(&attestation.platform) {
        return Err(GatekeeperError::TeeError(format!(
            "Platform {:?} is not accepted by policy",
            attestation.platform
        )));
    }

    // Check measurement (if policy specifies required measurements).
    if !policy.required_measurements.is_empty()
        && !policy
            .required_measurements
            .contains(&attestation.measurement_hash)
    {
        return Err(GatekeeperError::TeeError(
            "Measurement hash does not match any required measurement".into(),
        ));
    }

    // Check signature is non-empty.
    if attestation.signature.is_empty() {
        return Err(GatekeeperError::TeeError(
            "Attestation signature is empty".into(),
        ));
    }

    // Check age.
    if policy.max_age_ms > 0 {
        let age = policy.current_time_ms.saturating_sub(attestation.timestamp);
        if age > policy.max_age_ms {
            return Err(GatekeeperError::TeeError(format!(
                "Attestation is too old: {} ms (max: {} ms)",
                age, policy.max_age_ms
            )));
        }
    }

    // Verify deterministic signature.
    let mut sig_input = Vec::new();
    sig_input.extend_from_slice(&attestation.measurement_hash);
    sig_input.extend_from_slice(&attestation.timestamp.to_le_bytes());
    sig_input.extend_from_slice(format!("{:?}", attestation.platform).as_bytes());
    let expected_sig = blake3::hash(&sig_input).as_bytes().to_vec();

    if attestation.signature != expected_sig {
        return Err(GatekeeperError::TeeError(
            "Attestation signature verification failed".into(),
        ));
    }

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const MEASUREMENT: &[u8] = b"enclave-binary-v1.0";
    const TIMESTAMP: u64 = 1_700_000_000_000;

    fn valid_attestation() -> TeeAttestation {
        generate_attestation(&TeePlatform::Simulated, MEASUREMENT, TIMESTAMP)
    }

    fn permissive_policy() -> TeePolicy {
        TeePolicy {
            accepted_platforms: vec![
                TeePlatform::Sgx,
                TeePlatform::TrustZone,
                TeePlatform::Sev,
                TeePlatform::Simulated,
            ],
            required_measurements: vec![],
            max_age_ms: 0,
            current_time_ms: TIMESTAMP,
        }
    }

    // --- Generation ---

    #[test]
    fn generate_produces_valid_attestation() {
        let att = valid_attestation();
        assert_eq!(att.platform, TeePlatform::Simulated);
        assert_eq!(att.timestamp, TIMESTAMP);
        assert!(!att.signature.is_empty());
        assert_eq!(att.measurement_hash, *blake3::hash(MEASUREMENT).as_bytes());
    }

    #[test]
    fn generate_is_deterministic() {
        let att1 = generate_attestation(&TeePlatform::Sgx, MEASUREMENT, TIMESTAMP);
        let att2 = generate_attestation(&TeePlatform::Sgx, MEASUREMENT, TIMESTAMP);
        assert_eq!(att1, att2);
    }

    #[test]
    fn generate_different_platforms_produce_different_signatures() {
        let att_sgx = generate_attestation(&TeePlatform::Sgx, MEASUREMENT, TIMESTAMP);
        let att_sev = generate_attestation(&TeePlatform::Sev, MEASUREMENT, TIMESTAMP);
        assert_ne!(att_sgx.signature, att_sev.signature);
    }

    #[test]
    fn generate_different_measurements_produce_different_hashes() {
        let att1 = generate_attestation(&TeePlatform::Simulated, b"binary-v1", TIMESTAMP);
        let att2 = generate_attestation(&TeePlatform::Simulated, b"binary-v2", TIMESTAMP);
        assert_ne!(att1.measurement_hash, att2.measurement_hash);
    }

    // --- Verification: passing ---

    #[test]
    fn verify_passes_for_valid_attestation() {
        let att = valid_attestation();
        let policy = permissive_policy();
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    #[test]
    fn verify_passes_for_sgx_attestation() {
        let att = generate_attestation(&TeePlatform::Sgx, MEASUREMENT, TIMESTAMP);
        let policy = permissive_policy();
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    #[test]
    fn verify_passes_for_trustzone_attestation() {
        let att = generate_attestation(&TeePlatform::TrustZone, MEASUREMENT, TIMESTAMP);
        let policy = permissive_policy();
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    #[test]
    fn verify_passes_for_sev_attestation() {
        let att = generate_attestation(&TeePlatform::Sev, MEASUREMENT, TIMESTAMP);
        let policy = permissive_policy();
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    // --- Verification: platform rejection ---

    #[test]
    fn verify_rejects_unaccepted_platform() {
        let att = valid_attestation(); // Simulated
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Sgx], // only SGX
            required_measurements: vec![],
            max_age_ms: 0,
            current_time_ms: TIMESTAMP,
        };
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not accepted"));
    }

    // --- Verification: measurement mismatch ---

    #[test]
    fn verify_rejects_measurement_mismatch() {
        let att = valid_attestation();
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Simulated],
            required_measurements: vec![[0u8; 32]], // wrong hash
            max_age_ms: 0,
            current_time_ms: TIMESTAMP,
        };
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("Measurement"));
    }

    #[test]
    fn verify_passes_when_measurement_matches() {
        let att = valid_attestation();
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Simulated],
            required_measurements: vec![att.measurement_hash],
            max_age_ms: 0,
            current_time_ms: TIMESTAMP,
        };
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    // --- Verification: empty signature ---

    #[test]
    fn verify_rejects_empty_signature() {
        let mut att = valid_attestation();
        att.signature = vec![]; // tamper
        let policy = permissive_policy();
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("empty"));
    }

    // --- Verification: age ---

    #[test]
    fn verify_rejects_expired_attestation() {
        let att = valid_attestation(); // timestamp = TIMESTAMP
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Simulated],
            required_measurements: vec![],
            max_age_ms: 1000,                         // 1 second max
            current_time_ms: TIMESTAMP + 5000,        // 5 seconds later
        };
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("too old"));
    }

    #[test]
    fn verify_passes_within_age_limit() {
        let att = valid_attestation();
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Simulated],
            required_measurements: vec![],
            max_age_ms: 10_000,
            current_time_ms: TIMESTAMP + 5_000,
        };
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    #[test]
    fn verify_no_age_limit_passes_old_attestation() {
        let att = valid_attestation();
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Simulated],
            required_measurements: vec![],
            max_age_ms: 0, // no limit
            current_time_ms: TIMESTAMP + 999_999_999,
        };
        assert!(verify_attestation(&att, &policy).is_ok());
    }

    // --- Verification: tampered signature ---

    #[test]
    fn verify_rejects_tampered_signature() {
        let mut att = valid_attestation();
        att.signature = vec![0xDE, 0xAD, 0xBE, 0xEF]; // wrong
        let policy = permissive_policy();
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("verification failed"));
    }

    #[test]
    fn verify_rejects_tampered_timestamp() {
        let mut att = valid_attestation();
        att.timestamp += 1; // tamper — signature won't match
        let policy = permissive_policy();
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn verify_rejects_tampered_measurement() {
        let mut att = valid_attestation();
        att.measurement_hash[0] ^= 0xFF; // tamper
        let policy = permissive_policy();
        let result = verify_attestation(&att, &policy);
        assert!(result.is_err());
    }

    // --- Multiple required measurements: any match passes ---

    #[test]
    fn verify_passes_when_one_of_multiple_measurements_matches() {
        let att = valid_attestation();
        let policy = TeePolicy {
            accepted_platforms: vec![TeePlatform::Simulated],
            required_measurements: vec![[0u8; 32], att.measurement_hash, [1u8; 32]],
            max_age_ms: 0,
            current_time_ms: TIMESTAMP,
        };
        assert!(verify_attestation(&att, &policy).is_ok());
    }
}
