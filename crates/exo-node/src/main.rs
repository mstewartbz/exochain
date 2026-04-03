//! EXOCHAIN distributed node binary.
//!
//! Single binary for joining and participating in the constitutional
//! governance network. Every node runs the same verified code with
//! identical CGR kernel enforcement.
//!
//! ## Usage
//!
//! ```bash
//! exochain start                           # start a standalone node
//! exochain join --seed=seed1.exochain.io   # join an existing network
//! exochain status                          # show node status
//! exochain peers                           # list connected peers
//! ```

mod cli;
mod config;
mod identity;
mod store;

use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        tracing::error!("{e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Start {
            api_port,
            p2p_port,
            data_dir,
        } => {
            let data_dir = config::resolve_data_dir(data_dir)?;
            let cfg = config::load_or_create(&data_dir)?;

            let api_port = api_port.unwrap_or(cfg.api_port);
            let p2p_port = p2p_port.unwrap_or(cfg.p2p_port);

            // Bootstrap node identity.
            let node_identity = identity::load_or_create(&data_dir)?;
            tracing::info!(did = %node_identity.did, "Node identity ready");

            // Open local DAG store.
            let dag_store = store::SqliteDagStore::open(&data_dir)?;
            let height = dag_store.committed_height_value();
            tracing::info!(height, "DAG store opened");

            tracing::info!(
                api_port,
                p2p_port,
                did = %node_identity.did,
                "Starting exochain node"
            );

            // Start the gateway HTTP server.
            let bind_address = format!("0.0.0.0:{api_port}");
            let gateway_config = exo_gateway::server::GatewayConfig {
                bind_address: bind_address.clone(),
                ..exo_gateway::server::GatewayConfig::default()
            };

            tracing::info!(%bind_address, "Gateway listening");
            exo_gateway::server::serve(gateway_config, None).await?;

            Ok(())
        }

        Command::Join {
            seed,
            api_port,
            p2p_port,
            data_dir,
        } => {
            let data_dir = config::resolve_data_dir(data_dir)?;
            let _cfg = config::load_or_create(&data_dir)?;

            let api_port = api_port.unwrap_or(8080);
            let p2p_port = p2p_port.unwrap_or(4001);

            let node_identity = identity::load_or_create(&data_dir)?;
            tracing::info!(did = %node_identity.did, "Node identity ready");

            let dag_store = store::SqliteDagStore::open(&data_dir)?;
            let height = dag_store.committed_height_value();
            tracing::info!(height, "DAG store opened");

            // Register seed peers.
            let mut peer_registry = exo_api::p2p::PeerRegistry::new();
            let discovered = exo_api::p2p::discover_peers(&mut peer_registry, &seed)?;
            tracing::info!(
                seed_count = discovered.len(),
                "Discovered seed peers — P2P transport will be wired in Phase 2"
            );

            // Start the gateway HTTP server.
            let bind_address = format!("0.0.0.0:{api_port}");
            let gateway_config = exo_gateway::server::GatewayConfig {
                bind_address: bind_address.clone(),
                ..exo_gateway::server::GatewayConfig::default()
            };

            tracing::info!(
                %bind_address,
                p2p_port,
                peers = peer_registry.len(),
                "Node joined — dashboard at http://localhost:{api_port}"
            );

            exo_gateway::server::serve(gateway_config, None).await?;

            Ok(())
        }

        Command::Status { data_dir } => {
            let data_dir = config::resolve_data_dir(data_dir)?;
            let node_identity = identity::load_or_create(&data_dir)?;
            let dag_store = store::SqliteDagStore::open(&data_dir)?;

            println!("Node:   {}", node_identity.did);
            println!("Height: {}", dag_store.committed_height_value());
            println!("Data:   {}", data_dir.display());

            Ok(())
        }

        Command::Peers { data_dir: _ } => {
            println!("No peers connected — P2P transport will be wired in Phase 2");
            Ok(())
        }
    }
}
