//! Root genesis CLI command implementation.

use std::{collections::BTreeMap, fs, net::SocketAddr};

use exo_core::{Did, Hash256, Timestamp, crypto::KeyPair};
use exo_root::{
    CertifierContact, GenesisCeremonyConfig, RootIssuerDelegation, RootKeyPackage,
    RootPublicKeyPackage, RootTrustBundle, assemble_root_bundle, dkg_finalize_participant,
    dkg_round1, dkg_round2, seal_share, threshold_sign, unseal_share, verify_root_bundle,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::{
    cli::{
        GenesisCeremonyCommand, GenesisCeremonyInitArgs, GenesisCertifierCommand,
        GenesisCertifierInitArgs, GenesisCommand, GenesisIoArgs, GenesisPortalArgs,
    },
    root_genesis::{RootGenesisApiState, root_genesis_router},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrivateCertifierMaterial {
    did: Did,
    frost_identifier: u16,
    signing_secret_hex: String,
    transport_secret_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Round1CommandInput {
    config: GenesisCeremonyConfig,
    frost_identifier: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Round2CommandInput {
    config: GenesisCeremonyConfig,
    frost_identifier: u16,
    round1_secret_package_hex: String,
    round1_packages_hex: BTreeMapStringBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FinalizeDkgCommandInput {
    config: GenesisCeremonyConfig,
    frost_identifier: u16,
    round2_secret_package_hex: String,
    round1_packages_hex: BTreeMapStringBytes,
    round2_packages_hex: BTreeMapStringBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SignRootArtifactCommandInput {
    config: GenesisCeremonyConfig,
    public_key_package: RootPublicKeyPackage,
    key_packages: BTreeMap<u16, RootKeyPackage>,
    artifact_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AssembleBundleCommandInput {
    config: GenesisCeremonyConfig,
    public_key_package: RootPublicKeyPackage,
    issuer_delegation: RootIssuerDelegation,
    transcript_hash: Hash256,
    root_signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct VerifyBundleCommandInput {
    bundle: RootTrustBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SealShareCommandInput {
    share_hex: String,
    passphrase_hex: String,
    associated_data_hex: String,
    salt_hex: String,
    nonce_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UnsealShareCommandInput {
    sealed: exo_root::SealedShare,
    passphrase_hex: String,
    associated_data_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HexBytesOutput {
    bytes_hex: String,
}

type BTreeMapStringBytes = BTreeMap<u16, String>;

/// Execute a root genesis CLI command.
pub async fn run_genesis_command(command: GenesisCommand) -> anyhow::Result<()> {
    match command {
        GenesisCommand::Certifier { command } => run_certifier_command(command),
        GenesisCommand::Ceremony { command } => run_ceremony_command(command),
        GenesisCommand::Portal(args) => serve_portal(args).await,
        GenesisCommand::Round1(args) => run_round1(args),
        GenesisCommand::Round2(args) => run_round2(args),
        GenesisCommand::FinalizeDkg(args) => run_finalize_dkg(args),
        GenesisCommand::SignRootArtifact(args) => run_sign_root_artifact(args),
        GenesisCommand::AssembleBundle(args) => run_assemble_bundle(args),
        GenesisCommand::VerifyBundle(args) => run_verify_bundle(args),
        GenesisCommand::SealShare(args) => run_seal_share(args),
        GenesisCommand::UnsealShare(args) => run_unseal_share(args),
    }
}

fn run_certifier_command(command: GenesisCertifierCommand) -> anyhow::Result<()> {
    match command {
        GenesisCertifierCommand::Init(args) => init_certifier(args),
    }
}

fn run_ceremony_command(command: GenesisCeremonyCommand) -> anyhow::Result<()> {
    match command {
        GenesisCeremonyCommand::Init(args) => init_ceremony(args),
    }
}

fn init_certifier(args: GenesisCertifierInitArgs) -> anyhow::Result<()> {
    let did = Did::new(&args.did)?;
    let mut signing_seed = [0u8; 32];
    let mut transport_secret = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut signing_seed);
    rand::rngs::OsRng.fill_bytes(&mut transport_secret);
    let signing_keypair = KeyPair::from_secret_bytes(signing_seed)?;
    let transport_public = X25519PublicKey::from(&StaticSecret::from(transport_secret));
    let contact = CertifierContact {
        did: did.clone(),
        frost_identifier: args.frost_identifier,
        signing_public_key: *signing_keypair.public_key(),
        transport_public_key: *transport_public.as_bytes(),
    };
    let private = PrivateCertifierMaterial {
        did,
        frost_identifier: args.frost_identifier,
        signing_secret_hex: hex::encode(signing_seed),
        transport_secret_hex: hex::encode(transport_secret),
    };
    write_json(&args.certifier_out, &contact)?;
    write_json(&args.private_out, &private)?;
    Ok(())
}

fn init_ceremony(args: GenesisCeremonyInitArgs) -> anyhow::Result<()> {
    let certifiers: Vec<CertifierContact> = read_json(&args.roster)?;
    let constitution_hash = parse_hash_hex(&args.constitution_hash)?;
    let config = GenesisCeremonyConfig {
        ceremony_id: args.ceremony_id,
        network_id: args.network_id,
        repo_commit: args.repo_commit,
        constitution_hash,
        threshold: exo_root::ROOT_GENESIS_THRESHOLD,
        max_signers: exo_root::ROOT_GENESIS_SIGNERS,
        created_at: Timestamp::new(args.created_physical_ms, 0),
        certifiers,
    };
    config.validate()?;
    write_json(&args.out, &config)?;
    Ok(())
}

async fn serve_portal(args: GenesisPortalArgs) -> anyhow::Result<()> {
    let config: GenesisCeremonyConfig = read_json(&args.config)?;
    config.validate()?;
    let address = args.bind.parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    let router = root_genesis_router(RootGenesisApiState::new(config));
    axum::serve(listener, router).await?;
    Ok(())
}

fn run_round1(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: Round1CommandInput = read_json(&required_input(&args)?)?;
    input.config.validate()?;
    let mut rng = rand::rngs::OsRng;
    let output = dkg_round1(&input.config, input.frost_identifier, &mut rng)?;
    write_output(&args, &output)
}

fn run_round2(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: Round2CommandInput = read_json(&required_input(&args)?)?;
    let output = dkg_round2(
        &input.config,
        input.frost_identifier,
        decode_hex(&input.round1_secret_package_hex)?.as_slice(),
        decode_package_map(input.round1_packages_hex)?,
    )?;
    write_output(&args, &output)
}

fn run_finalize_dkg(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: FinalizeDkgCommandInput = read_json(&required_input(&args)?)?;
    let output = dkg_finalize_participant(
        &input.config,
        input.frost_identifier,
        decode_hex(&input.round2_secret_package_hex)?.as_slice(),
        decode_package_map(input.round1_packages_hex)?,
        decode_package_map(input.round2_packages_hex)?,
    )?;
    write_output(&args, &output)
}

fn run_sign_root_artifact(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: SignRootArtifactCommandInput = read_json(&required_input(&args)?)?;
    let artifact = decode_hex(&input.artifact_hex)?;
    let mut rng = rand::rngs::OsRng;
    let signature = threshold_sign(
        &input.config,
        &input.public_key_package,
        input.key_packages,
        artifact.as_slice(),
        &mut rng,
    )?;
    write_output(&args, &signature)
}

fn run_assemble_bundle(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: AssembleBundleCommandInput = read_json(&required_input(&args)?)?;
    let bundle = assemble_root_bundle(
        input.config,
        input.public_key_package,
        input.issuer_delegation,
        input.transcript_hash,
        decode_hex(&input.root_signature_hex)?,
    )?;
    write_output(&args, &bundle)
}

fn run_verify_bundle(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: VerifyBundleCommandInput = read_json(&required_input(&args)?)?;
    verify_root_bundle(&input.bundle)?;
    write_output(&args, &serde_json::json!({ "verified": true }))
}

fn run_seal_share(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: SealShareCommandInput = read_json(&required_input(&args)?)?;
    let salt = decode_fixed_16(&input.salt_hex)?;
    let nonce = decode_fixed_24(&input.nonce_hex)?;
    let sealed = seal_share(
        decode_hex(&input.share_hex)?.as_slice(),
        decode_hex(&input.passphrase_hex)?.as_slice(),
        decode_hex(&input.associated_data_hex)?.as_slice(),
        &salt,
        &nonce,
    )?;
    write_output(&args, &sealed)
}

fn run_unseal_share(args: GenesisIoArgs) -> anyhow::Result<()> {
    let input: UnsealShareCommandInput = read_json(&required_input(&args)?)?;
    let opened = unseal_share(
        &input.sealed,
        decode_hex(&input.passphrase_hex)?.as_slice(),
        decode_hex(&input.associated_data_hex)?.as_slice(),
    )?;
    write_output(
        &args,
        &HexBytesOutput {
            bytes_hex: hex::encode(opened),
        },
    )
}

fn parse_hash_hex(value: &str) -> anyhow::Result<Hash256> {
    let bytes = hex::decode(value)?;
    if bytes.len() != 32 {
        anyhow::bail!("constitution hash must be 32 bytes");
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(Hash256::from_bytes(hash))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &std::path::Path) -> anyhow::Result<T> {
    let bytes = fs::read(path)?;
    let value = serde_json::from_slice(&bytes)?;
    Ok(value)
}

fn write_json<T: Serialize>(path: &std::path::Path, value: &T) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn required_input(args: &GenesisIoArgs) -> anyhow::Result<std::path::PathBuf> {
    args.input
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--input is required for this command"))
}

fn write_output<T: Serialize>(args: &GenesisIoArgs, value: &T) -> anyhow::Result<()> {
    match &args.output {
        Some(path) => write_json(path, value),
        None => {
            println!("{}", serde_json::to_string_pretty(value)?);
            Ok(())
        }
    }
}

fn decode_hex(value: &str) -> anyhow::Result<Vec<u8>> {
    Ok(hex::decode(value)?)
}

fn decode_fixed_16(value: &str) -> anyhow::Result<[u8; 16]> {
    let bytes = decode_hex(value)?;
    if bytes.len() != 16 {
        anyhow::bail!("expected 16 bytes");
    }
    let mut result = [0u8; 16];
    result.copy_from_slice(&bytes);
    Ok(result)
}

fn decode_fixed_24(value: &str) -> anyhow::Result<[u8; 24]> {
    let bytes = decode_hex(value)?;
    if bytes.len() != 24 {
        anyhow::bail!("expected 24 bytes");
    }
    let mut result = [0u8; 24];
    result.copy_from_slice(&bytes);
    Ok(result)
}

fn decode_package_map(packages: BTreeMapStringBytes) -> anyhow::Result<BTreeMap<u16, Vec<u8>>> {
    let mut decoded = BTreeMap::new();
    for (identifier, package_hex) in packages {
        decoded.insert(identifier, decode_hex(&package_hex)?);
    }
    Ok(decoded)
}
