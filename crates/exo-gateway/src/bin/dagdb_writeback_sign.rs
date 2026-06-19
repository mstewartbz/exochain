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
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

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
    let request_path = match request_path_from_args(env::args().skip(1)) {
        Some(path) => path,
        None => {
            eprintln!("usage: dagdb_writeback_sign <writeback-request.json>");
            process::exit(2);
        }
    };

    match run_writeback_sign(&request_path).await {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

async fn run_writeback_sign(request_path: &str) -> Result<Value, String> {
    let request = match read_writeback_request(request_path) {
        Ok(request) => request,
        Err(error) => return Err(error),
    };

    let database_url = match database_url_from_env() {
        Ok(database_url) => database_url,
        Err(error) => return Err(error.to_owned()),
    };

    let pool = match connect_database_pool(database_url.as_str()).await {
        Ok(pool) => pool,
        Err(error) => return Err(error),
    };

    let selection_request = match selection_request_from_writeback(&request) {
        Ok(selection_request) => selection_request,
        Err(error) => return Err(format!("writeback_selection_request_failed: {error}")),
    };
    let selection = match build_persistent_graph_context_selection(&pool, &selection_request).await
    {
        Ok(selection) => selection,
        Err(error) => return Err(format!("writeback_selection_failed: {error}")),
    };

    let payload_hash = match usage_event_payload_hash(&selection.selection) {
        Ok(hash) => hash,
        Err(error) => return Err(format!("writeback_payload_hash_failed: {error}")),
    };
    let (lifecycle_payload_hash, continuation_payload_hash) =
        match writeback_d5_payload_hashes(&request) {
            Ok(payload_hashes) => payload_hashes,
            Err(error) => return Err(error),
        };

    let local_keypair = match load_local_dev_keypair() {
        Ok(local_keypair) => local_keypair,
        Err(error) => return Err(format!("writeback_sign_key_failed: {error}")),
    };

    let signatures = match sign_writeback_payloads(
        &local_keypair.keypair,
        &payload_hash,
        &lifecycle_payload_hash,
        &continuation_payload_hash,
    ) {
        Ok(signatures) => signatures,
        Err(error) => return Err(format!("writeback_sign_failed: {error}")),
    };

    pool.close().await;
    Ok(signature_output_json(
        LOCAL_DEV_AGENT_DID,
        local_keypair.source,
        &signatures,
    ))
}

fn request_path_from_args(args: impl IntoIterator<Item = String>) -> Option<String> {
    args.into_iter().next()
}

fn read_writeback_request(request_path: &str) -> Result<DagDbWritebackRequest, String> {
    let request_json = fs::read_to_string(request_path)
        .map_err(|error| format!("writeback_request_read_failed: {error}"))?;
    serde_json::from_str(&request_json)
        .map_err(|error| format!("writeback_request_json_invalid: {error}"))
}

fn database_url_from_env() -> Result<String, &'static str> {
    database_url_from_env_result(env::var("DATABASE_URL"))
}

fn database_url_from_env_result(
    database_url: Result<String, env::VarError>,
) -> Result<String, &'static str> {
    database_url.map_err(|_| "gateway database unavailable")
}

async fn connect_database_pool(database_url: &str) -> Result<Pool<Postgres>, String> {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .map_err(|error| format!("writeback_sign_connect_failed: {error}"))
}

fn writeback_d5_payload_hashes(
    request: &DagDbWritebackRequest,
) -> Result<([u8; 32], [u8; 32]), String> {
    let lifecycle_payload_hash = writeback_lifecycle_payload_hash(request)
        .map_err(|error| format!("writeback_lifecycle_payload_hash_failed: {error}"))?;
    let continuation_payload_hash = writeback_continuation_payload_hash(request)
        .map_err(|error| format!("writeback_continuation_payload_hash_failed: {error}"))?;
    Ok((lifecycle_payload_hash, continuation_payload_hash))
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
    let (seed_path, seed_source) = keypair_seed_path_from_env(env::var("DAGDB_DEV_KEY_SEED").ok());
    load_local_dev_keypair_from_seed_path(&seed_path, seed_source, fallback_enabled())
}

fn keypair_seed_path_from_env(env_seed: Option<String>) -> (String, &'static str) {
    match env_seed {
        Some(seed_path) => (seed_path, KEY_SOURCE_ENV_SEED),
        None => (DEV_KEY_SEED_REL.to_owned(), KEY_SOURCE_DEFAULT_SEED),
    }
}

fn fallback_enabled() -> bool {
    fallback_enabled_from_env(env::var(LOCAL_DEV_GATEKEEPER_ENV))
}

fn fallback_enabled_from_env(value: Result<String, env::VarError>) -> bool {
    value.map(|value| value == "1").unwrap_or(false)
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

    use exo_api::dagdb::DagDbWritebackRequest;
    use exo_core::crypto::KeyPair;

    use super::{
        DEV_KEY_SEED_REL, KEY_SOURCE_DEFAULT_SEED, KEY_SOURCE_ENV_SEED, LOCAL_DEV_AGENT_DID,
        LOCAL_DEV_GATEKEEPER_ENV, connect_database_pool, database_url_from_env,
        database_url_from_env_result, fallback_enabled, fallback_enabled_from_env,
        keypair_seed_path_from_env, load_local_dev_keypair, load_local_dev_keypair_from_seed_path,
        read_writeback_request, request_path_from_args, run_writeback_sign,
        sign_writeback_payloads, signature_output_json, writeback_d5_payload_hashes,
    };

    fn temp_seed_path(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dagdb_writeback_sign_{test_name}_{}_{}.seed",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ))
    }

    fn temp_request_path(test_name: &str) -> PathBuf {
        temp_seed_path(test_name).with_extension("json")
    }

    fn fixture_writeback_value() -> serde_json::Value {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json"
        ))
        .expect("parse dagdb fixtures");
        fixtures
            .get("requests")
            .and_then(|requests| requests.get("writeback"))
            .expect("writeback fixture")
            .clone()
    }

    fn fixture_writeback_json() -> String {
        serde_json::to_string(&fixture_writeback_value()).expect("serialize writeback fixture")
    }

    fn fixture_writeback_request() -> DagDbWritebackRequest {
        serde_json::from_value(fixture_writeback_value()).expect("parse writeback fixture")
    }

    #[test]
    fn dagdb_writeback_sign_accepts_first_request_path_argument() {
        assert_eq!(
            request_path_from_args(["request.json".to_owned(), "ignored.json".to_owned()]),
            Some("request.json".to_owned())
        );
    }

    #[test]
    fn dagdb_writeback_sign_requires_request_path_argument() {
        assert_eq!(request_path_from_args(Vec::<String>::new()), None);
    }

    #[tokio::test]
    async fn dagdb_writeback_sign_runner_returns_read_error_without_exiting() {
        let request_path = temp_request_path("runner_missing_request_file");
        let _ = fs::remove_file(&request_path);

        let error = run_writeback_sign(request_path.to_str().expect("utf8 request path"))
            .await
            .expect_err("missing request file must fail");

        assert!(
            error.contains("writeback_request_read_failed"),
            "expected read error prefix, got {error}"
        );
    }

    #[tokio::test]
    async fn dagdb_writeback_sign_runner_returns_real_environment_or_database_result() {
        let request_path = temp_request_path("runner_environment_or_database_result");
        fs::write(&request_path, fixture_writeback_json()).expect("write request fixture");

        let result = run_writeback_sign(request_path.to_str().expect("utf8 request path")).await;

        match result {
            Ok(output) => {
                assert_eq!(
                    output.get("agent_did").and_then(|value| value.as_str()),
                    Some(LOCAL_DEV_AGENT_DID)
                );
                assert!(
                    output
                        .pointer("/headers/x-exo-write-signature")
                        .and_then(|value| value.as_str())
                        .is_some(),
                    "successful signer output must include the write signature header"
                );
            }
            Err(error) => assert_runner_error_intent(&error),
        }

        fs::remove_file(request_path).expect("remove request fixture");
    }

    #[test]
    fn dagdb_writeback_sign_reads_writeback_request_json_file() {
        let request_path = temp_request_path("reads_writeback_request_json_file");
        fs::write(&request_path, fixture_writeback_json()).expect("write request fixture");

        let request = read_writeback_request(request_path.to_str().expect("utf8 request path"))
            .expect("read writeback request");

        assert_eq!(request.tenant_id, "tenant-a");
        assert_eq!(request.namespace, "primary");
        assert_eq!(request.idempotency_key, "idem-writeback-1");
        assert_eq!(
            request.parent_memory_ids,
            vec!["7070707070707070707070707070707070707070707070707070707070707070"]
        );
        assert_eq!(request.knowledge_class, None);
        assert_eq!(request.layered_mode, None);

        fs::remove_file(request_path).expect("remove request fixture");
    }

    #[test]
    fn dagdb_writeback_sign_read_error_names_missing_request_path() {
        let request_path = temp_request_path("missing_request_file");
        let _ = fs::remove_file(&request_path);

        let error = match read_writeback_request(request_path.to_str().expect("utf8 request path"))
        {
            Ok(_) => panic!("missing request file must fail"),
            Err(error) => error,
        };

        assert!(
            error.contains("writeback_request_read_failed"),
            "expected read error prefix, got {error}"
        );
    }

    #[test]
    fn dagdb_writeback_sign_rejects_malformed_request_json_file() {
        let request_path = temp_request_path("malformed_request_json_file");
        fs::write(&request_path, "{not valid json").expect("write malformed request");

        let error = match read_writeback_request(request_path.to_str().expect("utf8 request path"))
        {
            Ok(_) => panic!("malformed request json must fail"),
            Err(error) => error,
        };

        assert!(
            error.contains("writeback_request_json_invalid"),
            "expected json error prefix, got {error}"
        );

        fs::remove_file(request_path).expect("remove malformed request");
    }

    #[test]
    fn dagdb_writeback_sign_rejects_unknown_request_json_fields() {
        let request_path = temp_request_path("unknown_request_json_fields");
        let mut request = fixture_writeback_value();
        request
            .as_object_mut()
            .expect("object fixture")
            .insert("unexpected_field".to_owned(), serde_json::json!(true));
        fs::write(
            &request_path,
            serde_json::to_string(&request).expect("serialize mutated request"),
        )
        .expect("write mutated request");

        let error = match read_writeback_request(request_path.to_str().expect("utf8 request path"))
        {
            Ok(_) => panic!("unknown request field must fail"),
            Err(error) => error,
        };

        assert!(
            error.contains("writeback_request_json_invalid"),
            "expected json error prefix, got {error}"
        );
        assert!(
            error.contains("unexpected_field"),
            "expected unknown field name in error, got {error}"
        );

        fs::remove_file(request_path).expect("remove mutated request");
    }

    #[test]
    fn dagdb_writeback_sign_parses_database_url_env_result() {
        let database_url = "postgres://localhost/exochain".to_owned();
        assert_eq!(
            database_url_from_env_result(Ok(database_url.clone())).expect("database url"),
            database_url
        );
        assert_eq!(
            database_url_from_env_result(Err(std::env::VarError::NotPresent))
                .expect_err("missing database url must fail"),
            "gateway database unavailable"
        );
    }

    #[test]
    fn dagdb_writeback_sign_process_env_wrappers_match_helper_results() {
        match (database_url_from_env(), std::env::var("DATABASE_URL")) {
            (Ok(actual), Ok(expected)) => assert_eq!(actual, expected),
            (Err(actual), Err(_)) => assert_eq!(actual, "gateway database unavailable"),
            (actual, expected) => {
                panic!("database env changed during test: {actual:?} {expected:?}")
            }
        }

        assert_eq!(
            fallback_enabled(),
            fallback_enabled_from_env(std::env::var(LOCAL_DEV_GATEKEEPER_ENV))
        );

        let (seed_path, seed_source) =
            keypair_seed_path_from_env(std::env::var("DAGDB_DEV_KEY_SEED").ok());
        match load_local_dev_keypair() {
            Ok(local_keypair) => {
                if local_keypair.source != seed_source {
                    assert!(
                        fallback_enabled(),
                        "only the explicit local-dev fallback may change the seed source"
                    );
                }
            }
            Err(error) => assert!(!error.is_empty(), "expected key load error text"),
        }
        assert!(!seed_path.is_empty());
    }

    #[tokio::test]
    async fn dagdb_writeback_sign_rejects_invalid_database_url() {
        let error = match connect_database_pool("not a postgres database url").await {
            Ok(_) => panic!("invalid database url must fail"),
            Err(error) => error,
        };

        assert!(
            error.starts_with("writeback_sign_connect_failed: "),
            "expected connect error prefix, got {error}"
        );
    }

    fn assert_runner_error_intent(error: &str) {
        let exact_messages = ["gateway database unavailable"];
        let prefixed_messages = [
            "writeback_sign_connect_failed: ",
            "writeback_selection_request_failed: ",
            "writeback_selection_failed: ",
            "writeback_payload_hash_failed: ",
            "writeback_lifecycle_payload_hash_failed: ",
            "writeback_continuation_payload_hash_failed: ",
            "writeback_sign_key_failed: ",
            "writeback_sign_failed: ",
        ];
        assert!(
            exact_messages.contains(&error)
                || prefixed_messages
                    .iter()
                    .any(|prefix| error.starts_with(prefix)),
            "runner must preserve a known CLI error intent, got {error}"
        );
    }

    #[test]
    fn dagdb_writeback_sign_parses_key_seed_env_source() {
        let (seed_path, seed_source) =
            keypair_seed_path_from_env(Some("/tmp/dagdb-dev.seed".to_owned()));
        assert_eq!(seed_path, "/tmp/dagdb-dev.seed");
        assert_eq!(seed_source, KEY_SOURCE_ENV_SEED);

        let (seed_path, seed_source) = keypair_seed_path_from_env(None);
        assert_eq!(seed_path, DEV_KEY_SEED_REL);
        assert_eq!(seed_source, KEY_SOURCE_DEFAULT_SEED);
    }

    #[test]
    fn dagdb_writeback_sign_parses_local_dev_fallback_env_gate() {
        assert!(fallback_enabled_from_env(Ok("1".to_owned())));
        assert!(!fallback_enabled_from_env(Ok("true".to_owned())));
        assert!(!fallback_enabled_from_env(Err(
            std::env::VarError::NotPresent
        )));
    }

    #[test]
    fn dagdb_writeback_sign_derives_d5_payload_hashes_from_fixture_request() {
        let request = fixture_writeback_request();
        let payload_hashes =
            writeback_d5_payload_hashes(&request).expect("derive d5 payload hashes");
        let repeated_hashes =
            writeback_d5_payload_hashes(&request).expect("derive repeated d5 payload hashes");

        assert_eq!(
            payload_hashes, repeated_hashes,
            "D5 payload hash derivation must be deterministic"
        );
        assert_ne!(payload_hashes.0, [0_u8; 32]);
        assert_ne!(payload_hashes.1, [0_u8; 32]);
        assert_ne!(
            payload_hashes.0, payload_hashes.1,
            "lifecycle and continuation payloads must bind different records"
        );
    }

    #[test]
    fn dagdb_writeback_sign_rejects_d5_hashes_without_parent_memory() {
        let request = DagDbWritebackRequest {
            parent_memory_ids: Vec::new(),
            ..fixture_writeback_request()
        };

        let error = match writeback_d5_payload_hashes(&request) {
            Ok(_) => panic!("writeback with no parent memory must fail"),
            Err(error) => error,
        };

        assert!(
            error.contains("writeback_lifecycle_payload_hash_failed"),
            "expected lifecycle hash prefix, got {error}"
        );
        assert!(
            error.contains("lifecycle action request rejected"),
            "expected lifecycle rejection detail, got {error}"
        );
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
                .get("lifecycle_signature")
                .and_then(|value| value.as_str()),
            Some(signatures.lifecycle_signature.as_str())
        );
        assert_eq!(
            output
                .get("continuation_signature")
                .and_then(|value| value.as_str()),
            Some(signatures.continuation_signature.as_str())
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
        assert_eq!(
            output.get("key_source").and_then(|value| value.as_str()),
            Some("test_seed")
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
