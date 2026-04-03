//! Command-line interface for the exochain node.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "exochain",
    about = "EXOCHAIN distributed constitutional governance node",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a standalone node.
    Start {
        /// HTTP API port.
        #[arg(long, default_value = None)]
        api_port: Option<u16>,

        /// P2P listen port.
        #[arg(long, default_value = None)]
        p2p_port: Option<u16>,

        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Join an existing network via seed node(s).
    Join {
        /// Seed node addresses (e.g., seed1.exochain.io:4001).
        #[arg(long, required = true, num_args = 1..)]
        seed: Vec<String>,

        /// HTTP API port.
        #[arg(long, default_value = None)]
        api_port: Option<u16>,

        /// P2P listen port.
        #[arg(long, default_value = None)]
        p2p_port: Option<u16>,

        /// Data directory (default: ~/.exochain).
        #[arg(long)]
        data_dir: Option<PathBuf>,
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
}
