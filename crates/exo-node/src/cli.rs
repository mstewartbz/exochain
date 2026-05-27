// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Command-line interface for the exochain node.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

pub const ROOT_GENESIS_LONG_HELP: &str = "\
Root genesis creates a 7-of-13 institutional root authority. Genesis DKG \
requires all 13 rostered certifiers to complete the ceremony; if any \
certifier fails, abort and restart with a new signed roster.

Certifier rules: keep private material offline, maintain an offline backup, \
never submit plaintext shares, encrypt round-two payloads per recipient, and \
run verify-bundle before trusting the result. Secret-producing commands require \
explicit unique --output files and refuse stdout.";

pub const DEFAULT_ROUND_TIMEOUT_MS: u64 = 5_000;
pub const MIN_ROUND_TIMEOUT_MS: u64 = 250;
pub const MAX_ROUND_TIMEOUT_MS: u64 = 300_000;

fn parse_round_timeout_ms(value: &str) -> Result<u64, String> {
    let timeout_ms = value
        .parse::<u64>()
        .map_err(|error| format!("round timeout must be a millisecond integer: {error}"))?;
    if !(MIN_ROUND_TIMEOUT_MS..=MAX_ROUND_TIMEOUT_MS).contains(&timeout_ms) {
        return Err(format!(
            "round timeout must be between {MIN_ROUND_TIMEOUT_MS} and {MAX_ROUND_TIMEOUT_MS} milliseconds"
        ));
    }
    Ok(timeout_ms)
}

#[derive(Parser)]
#[command(
    name = "exochain",
    about = "EXOCHAIN distributed constitutional governance node",
    version,
    propagate_version = true
)]
/// Top-level CLI argument parser for the exochain node.
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
/// Subcommands available to the exochain node (start, join, status, peers).
pub enum Command {
    /// Start a standalone node.
    Start {
        /// HTTP API port.
        #[arg(long, default_value = None)]
        api_port: Option<u16>,

        /// HTTP API bind host. Default is `127.0.0.1` (loopback only).
        /// Set to `0.0.0.0` to expose the admin-write API on all
        /// interfaces — do so ONLY when you have a front-door TLS
        /// terminator AND your admin bearer token is appropriately
        /// scoped (see auth.rs). Exposing 0.0.0.0 with the default
        /// bearer model is equivalent to publishing your node's
        /// governance-write credential on the open internet.
        #[arg(long, default_value = "127.0.0.1")]
        api_host: String,

        /// P2P listen port.
        #[arg(long, default_value = None)]
        p2p_port: Option<u16>,

        /// Consensus round timeout in milliseconds.
        #[arg(
            long,
            default_value_t = DEFAULT_ROUND_TIMEOUT_MS,
            value_parser = parse_round_timeout_ms
        )]
        round_timeout_ms: u64,

        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,

        /// Run as a BFT consensus validator.
        #[arg(long, default_value_t = false)]
        validator: bool,

        /// Validator DIDs for the initial validator set (comma-separated).
        /// If not provided, this node's DID is used as the sole validator.
        #[arg(long, value_delimiter = ',')]
        validators: Option<Vec<String>>,

        /// Validator public keys as `did:exo:...=<64 hex bytes>` entries.
        /// Required for every non-local validator DID because consensus
        /// proposals/votes/certificates are verified against these keys.
        #[arg(long = "validator-public-key", value_delimiter = ',')]
        validator_public_keys: Option<Vec<String>>,
    },

    /// Join an existing network via seed node(s).
    Join {
        /// Seed node addresses (e.g., seed1.exochain.io:4001).
        #[arg(long, required = true, num_args = 1..)]
        seed: Vec<String>,

        /// HTTP API port.
        #[arg(long, default_value = None)]
        api_port: Option<u16>,

        /// HTTP API bind host. Default is `127.0.0.1` (loopback only).
        /// See `Start --api-host` for rationale on 0.0.0.0.
        #[arg(long, default_value = "127.0.0.1")]
        api_host: String,

        /// P2P listen port.
        #[arg(long, default_value = None)]
        p2p_port: Option<u16>,

        /// Consensus round timeout in milliseconds.
        #[arg(
            long,
            default_value_t = DEFAULT_ROUND_TIMEOUT_MS,
            value_parser = parse_round_timeout_ms
        )]
        round_timeout_ms: u64,

        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,

        /// Run as a BFT consensus validator.
        #[arg(long, default_value_t = false)]
        validator: bool,

        /// Validator DIDs for the initial validator set (comma-separated).
        #[arg(long, value_delimiter = ',')]
        validators: Option<Vec<String>>,

        /// Validator public keys as `did:exo:...=<64 hex bytes>` entries.
        /// Required for every non-local validator DID because consensus
        /// proposals/votes/certificates are verified against these keys.
        #[arg(long = "validator-public-key", value_delimiter = ',')]
        validator_public_keys: Option<Vec<String>>,
    },

    /// Show node status.
    Status {
        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// List connected peers.
    Peers {
        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Start the MCP (Model Context Protocol) server on stdio or HTTP+SSE.
    /// Enables AI agents to interact with the governance fabric.
    Mcp {
        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,

        /// DID for the MCP actor. If not provided, uses the node's identity.
        #[arg(long)]
        actor_did: Option<String>,

        /// Use HTTP+SSE transport instead of stdio. The value is the bind
        /// address (host:port). Example: `--sse 127.0.0.1:3030`.
        #[arg(long)]
        sse: Option<String>,
    },

    /// Run root genesis FROST DKG and root trust bundle operations.
    Genesis {
        #[command(subcommand)]
        command: GenesisCommand,
    },
}

#[derive(Subcommand)]
#[command(after_long_help = ROOT_GENESIS_LONG_HELP)]
/// Root genesis ceremony commands.
pub enum GenesisCommand {
    /// Certifier-local setup commands.
    Certifier {
        #[command(subcommand)]
        command: GenesisCertifierCommand,
    },

    /// Ceremony operator setup commands.
    Ceremony {
        #[command(subcommand)]
        command: GenesisCeremonyCommand,
    },

    /// Serve the untrusted root genesis relay portal.
    Portal(GenesisPortalArgs),

    /// Produce or verify DKG round-one material.
    Round1(GenesisIoArgs),

    /// Produce encrypted DKG round-two material.
    Round2(GenesisIoArgs),

    /// Finalize DKG once all thirteen certifiers have completed both rounds.
    #[command(name = "finalize-dkg")]
    FinalizeDkg(GenesisIoArgs),

    /// Build CBOR payload bytes for this certifier's final key confirmation.
    #[command(name = "build-final-key-confirmation")]
    BuildFinalKeyConfirmation(GenesisIoArgs),

    /// Sign a root-governance artifact with the exact predeclared signing set.
    #[command(name = "sign-root-artifact")]
    SignRootArtifact(GenesisIoArgs),

    /// Assemble a root trust bundle after artifact signing.
    #[command(name = "assemble-bundle")]
    AssembleBundle(GenesisIoArgs),

    /// Verify a root trust bundle before trusting any AVC issuer delegation.
    #[command(name = "verify-bundle")]
    VerifyBundle(GenesisIoArgs),

    /// Seal a serialized certifier share artifact.
    #[command(name = "seal-share")]
    SealShare(GenesisIoArgs),

    /// Open a sealed certifier share artifact.
    #[command(name = "unseal-share")]
    UnsealShare(GenesisIoArgs),

    /// Sign a ceremony portal envelope ready for submission.
    #[command(name = "sign-envelope")]
    SignEnvelope(GenesisSignEnvelopeArgs),

    /// Encrypt a DKG round-two payload for exactly one recipient.
    #[command(name = "encrypt-pairwise")]
    EncryptPairwise(GenesisIoArgs),

    /// Decrypt a recipient-bound DKG round-two payload.
    #[command(name = "decrypt-pairwise")]
    DecryptPairwise(GenesisIoArgs),

    /// Emit the exact root-artifact bytes that `sign-root-artifact` must sign.
    #[command(name = "emit-artifact-bytes")]
    EmitArtifactBytes(GenesisIoArgs),

    /// Submit a signed envelope to a running root genesis portal.
    #[command(name = "submit-envelope")]
    SubmitEnvelope(GenesisSubmitEnvelopeArgs),

    /// Pull accepted envelopes back from a running portal (read half of the relay).
    #[command(name = "pull-envelopes")]
    PullEnvelopes(GenesisPullEnvelopesArgs),

    /// Replay accepted envelopes and emit the completed DKG transcript hash.
    #[command(name = "compute-dkg-transcript-hash")]
    ComputeDkgTranscriptHash(GenesisIoArgs),

    /// Replay accepted envelopes and emit the final ceremony transcript hash.
    #[command(name = "compute-final-transcript-hash")]
    ComputeFinalTranscriptHash(GenesisIoArgs),

    /// CBOR-encode an encrypted round-two payload into envelope `payload_bytes`.
    #[command(name = "encode-encrypted-payload")]
    EncodeEncryptedPayload(GenesisIoArgs),

    /// CBOR-decode envelope `payload_bytes` back into an encrypted round-two payload.
    #[command(name = "decode-encrypted-payload")]
    DecodeEncryptedPayload(GenesisIoArgs),

    /// Distributed signing — produce one signer's public commitment (and a
    /// separate local-only secret-nonces file).
    #[command(name = "sign-commit")]
    SignCommit(GenesisSignCommitArgs),

    /// Distributed signing — coordinator builds the signing package from the declared set.
    #[command(name = "build-signing-package")]
    BuildSigningPackage(GenesisIoArgs),

    /// Distributed signing — produce one signer's signature share.
    #[command(name = "sign-share")]
    SignShare(GenesisSignShareArgs),

    /// Distributed signing — coordinator aggregates the declared set into the root signature.
    #[command(name = "aggregate-signature")]
    AggregateSignature(GenesisIoArgs),
}

#[derive(Subcommand)]
/// Certifier-local root genesis commands.
pub enum GenesisCertifierCommand {
    /// Generate certifier signing and transport material.
    Init(GenesisCertifierInitArgs),
}

#[derive(Subcommand)]
/// Ceremony-operator root genesis commands.
pub enum GenesisCeremonyCommand {
    /// Build a signed-roster ceremony configuration.
    Init(GenesisCeremonyInitArgs),
}

#[derive(Args)]
/// Generate local certifier key material.
pub struct GenesisCertifierInitArgs {
    /// Certifier DID.
    #[arg(long)]
    pub did: String,

    /// FROST identifier in the inclusive range 1..=13.
    #[arg(long)]
    pub frost_identifier: u16,

    /// Public certifier contact output path.
    #[arg(long)]
    pub certifier_out: PathBuf,

    /// Private certifier material output path.
    #[arg(long)]
    pub private_out: PathBuf,
}

#[derive(Args)]
/// Build a ceremony configuration from a roster.
pub struct GenesisCeremonyInitArgs {
    /// Ceremony identifier.
    #[arg(long)]
    pub ceremony_id: String,

    /// EXOCHAIN network identifier.
    #[arg(long)]
    pub network_id: String,

    /// Reviewed repository commit.
    #[arg(long)]
    pub repo_commit: String,

    /// 32-byte constitution hash as lowercase or uppercase hex.
    #[arg(long)]
    pub constitution_hash: String,

    /// HLC physical milliseconds supplied by the operator.
    #[arg(long)]
    pub created_physical_ms: u64,

    /// JSON roster path containing thirteen certifier contacts.
    #[arg(long)]
    pub roster: PathBuf,

    /// Predeclared signing set: exactly `threshold` (7) rostered FROST identifiers
    /// that will sign root artifacts, comma-separated (e.g. `1,2,3,4,5,6,7`).
    #[arg(long, value_delimiter = ',')]
    pub signing_set: Vec<u16>,

    /// Ceremony configuration output path.
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(Args)]
/// Serve the root genesis relay portal.
pub struct GenesisPortalArgs {
    /// Ceremony configuration JSON path.
    #[arg(long)]
    pub config: PathBuf,

    /// Portal bind address.
    #[arg(long, default_value = "127.0.0.1:3017")]
    pub bind: String,
}

#[derive(Args)]
/// File-based root genesis command inputs.
pub struct GenesisIoArgs {
    /// Input JSON or binary path.
    #[arg(long)]
    pub input: Option<PathBuf>,

    /// Output JSON or binary path.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
/// Sign a ceremony portal envelope.
///
/// The signing secret is read from the certifier's `0600` private-material file
/// (`--private-input`, the file written by `certifier init --private-out`), never
/// passed on the command line. Passing key material through argv would leak it
/// through process listings, shell history, terminal capture, and operator
/// transcripts — unacceptable for a ceremony artifact that may later be
/// challenged by a third-party auditor.
pub struct GenesisSignEnvelopeArgs {
    /// Envelope draft JSON path (ceremony_id, phase, payload_kind, sender_did,
    /// recipient_did, sequence, payload_bytes).
    #[arg(long)]
    pub input: Option<PathBuf>,

    /// Signed envelope output path. Omit to print to stdout.
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Path to the certifier's private-material JSON (`certifier-NN.private.json`,
    /// `0600`). The 32-byte Ed25519 signing secret is read from this file's
    /// `signing_secret_hex` field; it is never accepted as a CLI argument.
    #[arg(long)]
    pub private_input: PathBuf,
}

#[derive(Args)]
/// Pull accepted envelopes back from a running portal.
pub struct GenesisPullEnvelopesArgs {
    /// Portal base URL (for example `http://127.0.0.1:3017`). The
    /// `/api/v1/root-genesis/portal/envelopes` path is appended automatically.
    #[arg(long)]
    pub portal_url: String,

    /// Optional ceremony phase filter (Round1, Round2, Finalize, RootSigning, ...).
    #[arg(long)]
    pub phase: Option<String>,

    /// Optional payload-kind filter (Round1Package, Round2EncryptedPackage, ...).
    #[arg(long)]
    pub payload_kind: Option<String>,

    /// Optional recipient DID filter (for recipient-bound round-two packages).
    #[arg(long)]
    pub recipient_did: Option<String>,

    /// Output path for the JSON array of matching envelopes.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
/// Submit a signed envelope to a running portal.
pub struct GenesisSubmitEnvelopeArgs {
    /// Portal base URL (for example `http://127.0.0.1:3017`). The
    /// `/api/v1/root-genesis/portal/envelopes` path is appended automatically
    /// when it is not already present.
    #[arg(long)]
    pub portal_url: String,

    /// Signed envelope JSON path to POST.
    #[arg(long)]
    pub input: Option<PathBuf>,
}

#[derive(Args)]
/// Distributed signing round one — emit a public commitment and a separate
/// local-only secret-nonces file.
///
/// The commitment is relay-safe and is broadcast to the coordinator; the nonces
/// file is SECRET, written `0600`, and must stay on the signer's host until
/// `sign-share`. They are written to two distinct paths so the secret nonces are
/// never co-located with the artifact that leaves the signer.
pub struct GenesisSignCommitArgs {
    /// Input JSON path (`config`, `key_package`).
    #[arg(long)]
    pub input: Option<PathBuf>,

    /// Output path for the PUBLIC `RootSigningCommitment` (safe to transmit to
    /// the coordinator). Refused if it already exists.
    #[arg(long)]
    pub commitment_out: PathBuf,

    /// Output path for the SECRET `RootSigningNonces` (`0600`, local-only, fed to
    /// `sign-share`). Refused if it already exists.
    #[arg(long)]
    pub nonces_out: PathBuf,
}

#[derive(Args)]
/// Distributed signing round two — produce one signer's signature share.
pub struct GenesisSignShareArgs {
    /// Input JSON path (`config`, `key_package`, `signing_package_hex`).
    #[arg(long)]
    pub input: Option<PathBuf>,

    /// Path to this signer's local-only `RootSigningNonces` file produced by
    /// `sign-commit --nonces-out`.
    #[arg(long)]
    pub nonces: PathBuf,

    /// Signature-share output path. Omit to print to stdout (the share is public).
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    fn long_help_for(path: &[&str]) -> String {
        let mut command = Cli::command();
        let mut current = &mut command;
        for segment in path {
            current = current
                .find_subcommand_mut(segment)
                .expect("subcommand should exist");
        }
        let mut help = Vec::new();
        current
            .write_long_help(&mut help)
            .expect("help should render");
        String::from_utf8(help).expect("help should be utf8")
    }

    #[test]
    fn genesis_cli_exposes_complete_operator_command_set() {
        let help = long_help_for(&["genesis"]);
        for command in [
            "certifier",
            "ceremony",
            "portal",
            "round1",
            "round2",
            "finalize-dkg",
            "build-final-key-confirmation",
            "sign-root-artifact",
            "assemble-bundle",
            "verify-bundle",
            "seal-share",
            "unseal-share",
            "sign-envelope",
            "encrypt-pairwise",
            "decrypt-pairwise",
            "emit-artifact-bytes",
            "submit-envelope",
            "pull-envelopes",
            "compute-dkg-transcript-hash",
            "compute-final-transcript-hash",
            "encode-encrypted-payload",
            "decode-encrypted-payload",
            "sign-commit",
            "build-signing-package",
            "sign-share",
            "aggregate-signature",
        ] {
            assert!(help.contains(command), "missing genesis command {command}");
        }
    }

    #[test]
    fn genesis_build_round1_attestation_command_is_absent_pending_ratification() {
        // The unratified round-one set attestation command was removed; it must
        // not reappear on the production ceremony CLI until a ratified payload
        // shape lands and the portal validates it.
        let help = long_help_for(&["genesis"]);
        assert!(!help.contains("build-round1-attestation"));
    }

    #[test]
    fn genesis_sign_envelope_takes_no_signing_secret_through_argv() {
        use clap::Parser;

        // The signing secret comes from a `0600` private-material file path.
        let with_file = Cli::try_parse_from([
            "exochain",
            "genesis",
            "sign-envelope",
            "--input",
            "draft.json",
            "--private-input",
            "certifier-01.private.json",
            "--output",
            "envelope.json",
        ]);
        assert!(
            with_file.is_ok(),
            "private-material file path must be accepted"
        );

        // A 32-byte hex secret cannot be passed as a CLI argument: the old flag
        // is gone and any attempt to supply key material on argv is rejected.
        let with_argv_secret = Cli::try_parse_from([
            "exochain",
            "genesis",
            "sign-envelope",
            "--input",
            "draft.json",
            "--signing-key-hex",
            &"ab".repeat(32),
        ]);
        assert!(
            with_argv_secret.is_err(),
            "no CLI argument may carry 32-byte signing-secret material"
        );
    }

    #[test]
    fn genesis_cli_help_warns_certifiers_about_secret_handling_and_restart_rules() {
        let help = long_help_for(&["genesis"]);
        for required in [
            "7-of-13",
            "all 13 rostered certifiers",
            "abort and restart",
            "offline backup",
            "never submit plaintext shares",
            "explicit unique --output files",
            "verify-bundle",
        ] {
            assert!(help.contains(required), "missing help text {required}");
        }
    }
}
