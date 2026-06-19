//! Sign a DAG DB writeback request for local gateway submission.
//!
//! Reads a `DagDbWritebackRequest` JSON file path, builds the persistent graph
//! selection payload hash plus the derived D5 lifecycle/continuation payload
//! hashes, and prints every required gateway signature header to stdout.

use std::{env, fs, process};

use exo_api::dagdb::DagDbWritebackRequest;
use exo_core::crypto::KeyPair;
use exo_dag_db_postgres::persistent_context::build_persistent_graph_context_selection;
use exo_gatekeeper::{sign_write_payload, usage_event_payload_hash};
use exo_gateway::dagdb::{
    selection_request_from_writeback, writeback_continuation_payload_hash,
    writeback_lifecycle_payload_hash,
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;

const LOCAL_DEV_AGENT_DID: &str = "did:exo:cursor-mcp-agent";
const DEV_KEY_SEED_REL: &str = "crates/exo-gatekeeper/tests/fixtures/dev_private_key.seed";
const LOCAL_DEV_GATEKEEPER_ENV: &str = "DAGDB_LOCAL_DEV_GATEKEEPER";
const KEY_SOURCE_ENV_SEED: &str = "env_seed_file";
const KEY_SOURCE_DEFAULT_SEED: &str = "default_seed_file";
#[cfg(debug_assertions)]
const KEY_SOURCE_DETERMINISTIC_FALLBACK: &str = "deterministic_local_dev_fallback";

struct LocalDevKeypair {
    keypair: KeyPair,
    source: &'static str,
}

struct WritebackSignatureSet {
    writeback_signature: String,
    lifecycle_signature: String,
    continuation_signature: String,
}

#[tokio::main]
async fn main() {
    let request_path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: dagdb_writeback_sign <writeback-request.json>");
            process::exit(2);
        }
    };

    let request_json = match fs::read_to_string(&request_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("writeback_request_read_failed: {error}");
            process::exit(1);
        }
    };

    let request: DagDbWritebackRequest = match serde_json::from_str(&request_json) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("writeback_request_json_invalid: {error}");
            process::exit(1);
        }
    };

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("gateway database unavailable");
        process::exit(1);
    });

    let pool = match PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url.as_str())
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("writeback_sign_connect_failed: {error}");
            process::exit(1);
        }
    };

    let selection_request = match selection_request_from_writeback(&request) {
        Ok(selection_request) => selection_request,
        Err(error) => {
            eprintln!("writeback_selection_request_failed: {error}");
            process::exit(1);
        }
    };
    let selection = match build_persistent_graph_context_selection(&pool, &selection_request).await
    {
        Ok(selection) => selection,
        Err(error) => {
            eprintln!("writeback_selection_failed: {error}");
            process::exit(1);
        }
    };

    let payload_hash = match usage_event_payload_hash(&selection.selection) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("writeback_payload_hash_failed: {error}");
            process::exit(1);
        }
    };
    let lifecycle_payload_hash = match writeback_lifecycle_payload_hash(&request) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("writeback_lifecycle_payload_hash_failed: {error}");
            process::exit(1);
        }
    };
    let continuation_payload_hash = match writeback_continuation_payload_hash(&request) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("writeback_continuation_payload_hash_failed: {error}");
            process::exit(1);
        }
    };

    let local_keypair = match load_local_dev_keypair() {
        Ok(local_keypair) => local_keypair,
        Err(error) => {
            eprintln!("writeback_sign_key_failed: {error}");
            process::exit(1);
        }
    };

    let signatures = match sign_writeback_payloads(
        &local_keypair.keypair,
        &payload_hash,
        &lifecycle_payload_hash,
        &continuation_payload_hash,
    ) {
        Ok(signatures) => signatures,
        Err(error) => {
            eprintln!("writeback_sign_failed: {error}");
            process::exit(1);
        }
    };

    pool.close().await;
    println!(
        "{}",
        signature_output_json(LOCAL_DEV_AGENT_DID, local_keypair.source, &signatures)
    );
}

fn sign_writeback_payloads(
    keypair: &KeyPair,
    writeback_payload_hash: &[u8; 32],
    lifecycle_payload_hash: &[u8; 32],
    continuation_payload_hash: &[u8; 32],
) -> Result<WritebackSignatureSet, String> {
    Ok(WritebackSignatureSet {
        writeback_signature: sign_write_payload(keypair, writeback_payload_hash)
            .map_err(|error| error.to_string())?,
        lifecycle_signature: sign_write_payload(keypair, lifecycle_payload_hash)
            .map_err(|error| error.to_string())?,
        continuation_signature: sign_write_payload(keypair, continuation_payload_hash)
            .map_err(|error| error.to_string())?,
    })
}

fn signature_output_json(
    agent_did: &str,
    key_source: &str,
    signatures: &WritebackSignatureSet,
) -> Value {
    json!({
        "signature": signatures.writeback_signature.clone(),
        "writeback_signature": signatures.writeback_signature.clone(),
        "lifecycle_signature": signatures.lifecycle_signature.clone(),
        "continuation_signature": signatures.continuation_signature.clone(),
        "headers": {
            "x-exo-write-signature": signatures.writeback_signature.clone(),
            "x-exo-lifecycle-signature": signatures.lifecycle_signature.clone(),
            "x-exo-continuation-signature": signatures.continuation_signature.clone()
        },
        "agent_did": agent_did,
        "key_source": key_source
    })
}

fn load_local_dev_keypair() -> Result<LocalDevKeypair, String> {
    let env_seed = env::var("DAGDB_DEV_KEY_SEED").ok();
    let seed_path = env_seed
        .clone()
        .unwrap_or_else(|| DEV_KEY_SEED_REL.to_owned());
    let seed_source = if env_seed.is_some() {
        KEY_SOURCE_ENV_SEED
    } else {
        KEY_SOURCE_DEFAULT_SEED
    };
    load_local_dev_keypair_from_seed_path(&seed_path, seed_source, fallback_enabled())
}

fn fallback_enabled() -> bool {
    env::var(LOCAL_DEV_GATEKEEPER_ENV)
        .map(|value| value == "1")
        .unwrap_or(false)
}

// `fallback_enabled` only gates the debug-only deterministic fallback; in
// release the parameter is intentionally unread (the fallback is compiled out).
#[cfg_attr(not(debug_assertions), allow(unused_variables))]
fn load_local_dev_keypair_from_seed_path(
    seed_path: &str,
    seed_source: &'static str,
    fallback_enabled: bool,
) -> Result<LocalDevKeypair, String> {
    let (seed_bytes, source) = match fs::read(seed_path) {
        Ok(bytes) if bytes.len() >= 32 => {
            let mut seed = [0_u8; 32];
            seed.copy_from_slice(&bytes[..32]);
            (seed, seed_source)
        }
        Ok(_) => return Err(format!("dev key seed at {seed_path} is too short")),
        Err(error) => {
            // T1: the deterministic [0..31] seed is a publicly-derivable private
            // key. It is compiled out of release builds; a release signer with no
            // provisioned seed file fails closed with a typed error and never
            // signs with a fabricated identity.
            #[cfg(debug_assertions)]
            if fallback_enabled {
                return Ok(LocalDevKeypair {
                    keypair: KeyPair::from_secret_bytes(core::array::from_fn(|index| {
                        u8::try_from(index % 256).unwrap_or_default()
                    }))
                    .map_err(|error| error.to_string())?,
                    source: KEY_SOURCE_DETERMINISTIC_FALLBACK,
                });
            }
            return Err(format!(
                "dev key seed unavailable at {seed_path}: {error}; provide a provisioned signing seed (debug builds may set {LOCAL_DEV_GATEKEEPER_ENV}=1 for the deterministic local-dev fallback)"
            ));
        }
    };
    let keypair = KeyPair::from_secret_bytes(seed_bytes).map_err(|error| error.to_string())?;
    Ok(LocalDevKeypair { keypair, source })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::{fs, path::PathBuf};

    use exo_core::crypto::KeyPair;

    use super::{
        KEY_SOURCE_ENV_SEED, LOCAL_DEV_GATEKEEPER_ENV, load_local_dev_keypair_from_seed_path,
        sign_writeback_payloads, signature_output_json,
    };

    fn temp_seed_path(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dagdb_writeback_sign_{test_name}_{}_{}.seed",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ))
    }

    #[test]
    fn dagdb_writeback_sign_outputs_all_required_gateway_headers() {
        let keypair = KeyPair::generate();
        let writeback_payload_hash = [1_u8; 32];
        let lifecycle_payload_hash = [2_u8; 32];
        let continuation_payload_hash = [3_u8; 32];
        let signatures = sign_writeback_payloads(
            &keypair,
            &writeback_payload_hash,
            &lifecycle_payload_hash,
            &continuation_payload_hash,
        )
        .expect("sign all payloads");
        let output = signature_output_json("did:exo:agent", "test_seed", &signatures);

        assert_eq!(
            output.get("signature"),
            output.get("writeback_signature"),
            "legacy signature field must remain the writeback signature"
        );
        assert_eq!(
            output
                .pointer("/headers/x-exo-write-signature")
                .and_then(|value| value.as_str()),
            Some(signatures.writeback_signature.as_str())
        );
        assert_eq!(
            output
                .pointer("/headers/x-exo-lifecycle-signature")
                .and_then(|value| value.as_str()),
            Some(signatures.lifecycle_signature.as_str())
        );
        assert_eq!(
            output
                .pointer("/headers/x-exo-continuation-signature")
                .and_then(|value| value.as_str()),
            Some(signatures.continuation_signature.as_str())
        );
        assert_eq!(
            output.get("agent_did").and_then(|value| value.as_str()),
            Some("did:exo:agent")
        );
        assert_eq!(output.get("seed"), None);
        assert_eq!(output.get("private_key"), None);
    }

    #[test]
    fn dagdb_writeback_sign_signs_each_payload_distinctly() {
        let keypair = KeyPair::from_secret_bytes([7_u8; 32]).expect("keypair from seed");
        let signatures = sign_writeback_payloads(&keypair, &[1_u8; 32], &[2_u8; 32], &[3_u8; 32])
            .expect("sign all payloads");

        assert_ne!(
            signatures.writeback_signature, signatures.lifecycle_signature,
            "writeback and lifecycle payloads must not reuse a signature"
        );
        assert_ne!(
            signatures.writeback_signature, signatures.continuation_signature,
            "writeback and continuation payloads must not reuse a signature"
        );
        assert_ne!(
            signatures.lifecycle_signature, signatures.continuation_signature,
            "lifecycle and continuation payloads must not reuse a signature"
        );
    }

    #[test]
    fn dagdb_writeback_sign_loads_seed_file_and_preserves_source() {
        let seed_path = temp_seed_path("loads_seed_file");
        let mut seed_file = vec![9_u8; 64];
        seed_file[..32].copy_from_slice(&[4_u8; 32]);
        fs::write(&seed_path, seed_file).expect("write seed file");

        let loaded = load_local_dev_keypair_from_seed_path(
            seed_path.to_str().expect("utf8 seed path"),
            KEY_SOURCE_ENV_SEED,
            false,
        )
        .expect("load keypair from seed");
        let expected = KeyPair::from_secret_bytes([4_u8; 32]).expect("expected keypair");
        let payload_hash = [5_u8; 32];
        let loaded_signature =
            sign_writeback_payloads(&loaded.keypair, &payload_hash, &[6_u8; 32], &[7_u8; 32])
                .expect("sign with loaded keypair")
                .writeback_signature;
        let expected_signature =
            sign_writeback_payloads(&expected, &payload_hash, &[6_u8; 32], &[7_u8; 32])
                .expect("sign with expected keypair")
                .writeback_signature;

        assert_eq!(loaded.source, KEY_SOURCE_ENV_SEED);
        assert_eq!(loaded_signature, expected_signature);

        fs::remove_file(seed_path).expect("remove seed file");
    }

    #[test]
    fn dagdb_writeback_sign_rejects_too_short_seed_file() {
        let seed_path = temp_seed_path("too_short_seed_file");
        fs::write(&seed_path, [1_u8; 31]).expect("write short seed file");

        let error = match load_local_dev_keypair_from_seed_path(
            seed_path.to_str().expect("utf8 seed path"),
            KEY_SOURCE_ENV_SEED,
            true,
        ) {
            Ok(_) => panic!("short seed must fail"),
            Err(error) => error,
        };

        assert!(
            error.contains("is too short"),
            "expected too-short seed error, got {error}"
        );
        assert!(
            error.contains(seed_path.to_str().expect("utf8 seed path")),
            "expected seed path in too-short error, got {error}"
        );

        fs::remove_file(seed_path).expect("remove seed file");
    }

    #[test]
    fn dagdb_writeback_sign_missing_seed_error_names_remediation() {
        let missing_seed = "__missing_dagdb_writeback_sign_seed_for_error_text__";
        let error =
            match load_local_dev_keypair_from_seed_path(missing_seed, KEY_SOURCE_ENV_SEED, false) {
                Ok(_) => panic!("missing seed must fail"),
                Err(error) => error,
            };

        assert!(
            error.contains(
                "dev key seed unavailable at __missing_dagdb_writeback_sign_seed_for_error_text__"
            ),
            "expected missing seed path in error, got {error}"
        );
        assert!(
            error.contains("provide a provisioned signing seed"),
            "expected remediation text in error, got {error}"
        );
        assert!(
            error.contains(LOCAL_DEV_GATEKEEPER_ENV),
            "expected debug fallback env name in error, got {error}"
        );
    }

    #[test]
    fn dagdb_writeback_sign_signature_output_does_not_leak_seed_material() {
        let keypair = KeyPair::from_secret_bytes([11_u8; 32]).expect("keypair from seed");
        let signatures = sign_writeback_payloads(&keypair, &[8_u8; 32], &[9_u8; 32], &[10_u8; 32])
            .expect("sign all payloads");
        let output = signature_output_json("did:exo:agent", KEY_SOURCE_ENV_SEED, &signatures);
        let serialized = serde_json::to_string(&output).expect("serialize signature output");

        assert_eq!(output.get("seed"), None);
        assert_eq!(output.get("private_key"), None);
        assert_eq!(output.get("secret_key"), None);
        assert_eq!(output.get("keypair"), None);
        assert!(
            !serialized.contains("dagdb_secret_seed_should_not_leak"),
            "signature output must not include seed paths or seed material"
        );
        assert!(
            !serialized.contains("[11,11,11"),
            "signature output must not include raw seed bytes"
        );
    }

    // Debug-only: the deterministic fallback exists ONLY in debug builds (T1).
    #[cfg(debug_assertions)]
    #[test]
    fn dagdb_writeback_sign_rejects_implicit_deterministic_fallback() {
        use super::KEY_SOURCE_DETERMINISTIC_FALLBACK;
        let missing_seed = "__missing_dagdb_writeback_sign_seed__";
        assert!(
            load_local_dev_keypair_from_seed_path(missing_seed, KEY_SOURCE_ENV_SEED, false)
                .is_err()
        );
        let fallback =
            load_local_dev_keypair_from_seed_path(missing_seed, KEY_SOURCE_ENV_SEED, true)
                .expect("explicit fallback");
        assert_eq!(fallback.source, KEY_SOURCE_DETERMINISTIC_FALLBACK);
    }

    /// T1 release-posture regression. Compiles ONLY under `--release`
    /// (`debug_assertions` off) — the configuration CI runs. The deterministic
    /// `[0..31]` seed is compiled out, so even with the dev env gate "enabled"
    /// the signer fails closed with a typed error and never fabricates a key.
    #[cfg(not(debug_assertions))]
    #[test]
    fn dagdb_writeback_sign_release_has_no_deterministic_fallback() {
        let missing_seed = "__missing_dagdb_writeback_sign_seed__";
        assert!(
            load_local_dev_keypair_from_seed_path(missing_seed, KEY_SOURCE_ENV_SEED, false)
                .is_err()
        );
        // `fallback_enabled = true` must NOT yield a fabricated key in release.
        assert!(
            load_local_dev_keypair_from_seed_path(missing_seed, KEY_SOURCE_ENV_SEED, true).is_err(),
            "release signer must fail closed even with the dev fallback flag set"
        );
    }
}
