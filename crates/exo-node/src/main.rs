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
//! exochain start --validator               # start as a BFT validator
//! exochain join --seed=seed1.exochain.io   # join an existing network
//! exochain status                          # show node status
//! exochain peers                           # list connected peers
//! ```

mod cli;
mod config;
mod identity;
mod network;
mod reactor;
mod store;
mod wire;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use clap::Parser;
use cli::{Cli, Command};
use exo_core::types::Did;
use network::{NetworkConfig, NetworkHandle};
use reactor::{ReactorConfig, ReactorEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        tracing::error!("{e:#}");
        std::process::exit(1);
    }
}

/// Parse a list of validator DID strings, falling back to just this node's DID.
fn parse_validator_set(
    cli_validators: &Option<Vec<String>>,
    node_did: &Did,
) -> BTreeSet<Did> {
    if let Some(vals) = cli_validators {
        vals.iter()
            .filter_map(|s| match Did::new(s) {
                Ok(d) => Some(d),
                Err(e) => {
                    tracing::warn!(did = %s, err = %e, "Invalid validator DID — skipping");
                    None
                }
            })
            .collect()
    } else {
        // Default: this node is the sole validator (standalone mode).
        let mut set = BTreeSet::new();
        set.insert(node_did.clone());
        set
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Start {
            api_port,
            p2p_port,
            data_dir,
            validator,
            validators,
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
                validator,
                did = %node_identity.did,
                "Starting exochain node"
            );

            // Start P2P networking.
            let net_config = NetworkConfig {
                tcp_port: p2p_port,
                quic_port: p2p_port + 1,
                seed_addrs: vec![],
                node_did: node_identity.did.clone(),
            };

            let mut swarm = network::build_swarm(&net_config)?;
            network::start_listening(&mut swarm, &net_config)?;

            let (cmd_tx, cmd_rx) = mpsc::channel(256);
            let (event_tx, event_rx) = mpsc::channel(256);
            let net_handle = NetworkHandle::new(cmd_tx);

            // Spawn the P2P network event loop.
            tokio::spawn(network::run_network_loop(swarm, cmd_rx, event_tx));
            tracing::info!(p2p_port, "P2P network started");

            // Initialize the consensus reactor.
            let validator_set = parse_validator_set(&validators, &node_identity.did);
            let reactor_config = ReactorConfig {
                node_did: node_identity.did.clone(),
                is_validator: validator,
                validators: validator_set.clone(),
                round_timeout_ms: 5000,
            };

            // Create a sign function from the node identity.
            let sign_fn: Arc<dyn Fn(&[u8]) -> exo_core::types::Signature + Send + Sync> = {
                // Capture the identity for signing.
                let identity = identity::load_or_create(&data_dir)?;
                Arc::new(move |data: &[u8]| identity.sign(data))
            };

            let reactor_state = reactor::create_reactor_state(&reactor_config, sign_fn);
            let shared_store = Arc::new(Mutex::new(dag_store));
            let (reactor_tx, mut reactor_rx) = mpsc::channel::<ReactorEvent>(256);

            // Spawn the consensus reactor.
            tokio::spawn(reactor::run_reactor(
                reactor_state.clone(),
                shared_store,
                net_handle,
                event_rx,
                reactor_tx,
            ));

            if validator {
                tracing::info!(
                    validators = validator_set.len(),
                    "Consensus reactor started (validator mode)"
                );
            } else {
                tracing::info!("Consensus reactor started (observer mode)");
            }

            // Spawn reactor event logger.
            tokio::spawn(async move {
                while let Some(event) = reactor_rx.recv().await {
                    match event {
                        ReactorEvent::NodeCommitted { hash, height, round } => {
                            tracing::info!(%hash, height, round, "Committed");
                        }
                        ReactorEvent::RoundAdvanced { round } => {
                            tracing::trace!(round, "Round advanced");
                        }
                        ReactorEvent::GovernanceEventReceived { event } => {
                            tracing::info!(
                                sender = %event.sender,
                                event_type = ?event.event_type,
                                "Governance event received"
                            );
                        }
                    }
                }
            });

            // Start the gateway HTTP server (blocks).
            let bind_address = format!("0.0.0.0:{api_port}");
            let gateway_config = exo_gateway::server::GatewayConfig {
                bind_address: bind_address.clone(),
                ..exo_gateway::server::GatewayConfig::default()
            };

            tracing::info!(
                %bind_address,
                "Dashboard at http://localhost:{api_port}"
            );
            exo_gateway::server::serve(gateway_config, None).await?;

            Ok(())
        }

        Command::Join {
            seed,
            api_port,
            p2p_port,
            data_dir,
            validator,
            validators,
        } => {
            let data_dir = config::resolve_data_dir(data_dir)?;
            let cfg = config::load_or_create(&data_dir)?;

            let api_port = api_port.unwrap_or(cfg.api_port);
            let p2p_port = p2p_port.unwrap_or(cfg.p2p_port);

            let node_identity = identity::load_or_create(&data_dir)?;
            tracing::info!(did = %node_identity.did, "Node identity ready");

            let dag_store = store::SqliteDagStore::open(&data_dir)?;
            let height = dag_store.committed_height_value();
            tracing::info!(height, "DAG store opened");

            // Parse seed addresses into multiaddrs.
            let seed_addrs: Vec<libp2p::Multiaddr> = seed
                .iter()
                .filter_map(|s| {
                    // Accept raw multiaddrs or host:port format
                    if s.starts_with('/') {
                        s.parse().ok()
                    } else {
                        // Convert host:port to /ip4/host/tcp/port
                        let parts: Vec<&str> = s.split(':').collect();
                        if parts.len() == 2 {
                            format!("/ip4/{}/tcp/{}", parts[0], parts[1]).parse().ok()
                        } else {
                            tracing::warn!(seed = %s, "Invalid seed address format");
                            None
                        }
                    }
                })
                .collect();

            // Start P2P networking.
            let net_config = NetworkConfig {
                tcp_port: p2p_port,
                quic_port: p2p_port + 1,
                seed_addrs: seed_addrs.clone(),
                node_did: node_identity.did.clone(),
            };

            let mut swarm = network::build_swarm(&net_config)?;
            network::start_listening(&mut swarm, &net_config)?;

            // Dial seed nodes.
            let dialed = network::dial_seeds(&mut swarm, &seed_addrs)?;
            tracing::info!(dialed, "Dialed seed nodes");

            let (cmd_tx, cmd_rx) = mpsc::channel(256);
            let (event_tx, event_rx) = mpsc::channel(256);
            let net_handle = NetworkHandle::new(cmd_tx);

            // Spawn the P2P network event loop.
            tokio::spawn(network::run_network_loop(swarm, cmd_rx, event_tx));
            tracing::info!(p2p_port, seeds = dialed, "P2P network started");

            // Initialize the consensus reactor.
            let validator_set = parse_validator_set(&validators, &node_identity.did);
            let reactor_config = ReactorConfig {
                node_did: node_identity.did.clone(),
                is_validator: validator,
                validators: validator_set.clone(),
                round_timeout_ms: 5000,
            };

            let sign_fn: Arc<dyn Fn(&[u8]) -> exo_core::types::Signature + Send + Sync> = {
                let identity = identity::load_or_create(&data_dir)?;
                Arc::new(move |data: &[u8]| identity.sign(data))
            };

            let reactor_state = reactor::create_reactor_state(&reactor_config, sign_fn);
            let shared_store = Arc::new(Mutex::new(dag_store));
            let (reactor_tx, mut reactor_rx) = mpsc::channel::<ReactorEvent>(256);

            // Spawn the consensus reactor.
            tokio::spawn(reactor::run_reactor(
                reactor_state.clone(),
                shared_store,
                net_handle,
                event_rx,
                reactor_tx,
            ));

            if validator {
                tracing::info!(
                    validators = validator_set.len(),
                    "Consensus reactor started (validator mode)"
                );
            } else {
                tracing::info!("Consensus reactor started (observer mode)");
            }

            // Spawn reactor event logger.
            tokio::spawn(async move {
                while let Some(event) = reactor_rx.recv().await {
                    match event {
                        ReactorEvent::NodeCommitted { hash, height, round } => {
                            tracing::info!(%hash, height, round, "Committed");
                        }
                        ReactorEvent::RoundAdvanced { round } => {
                            tracing::trace!(round, "Round advanced");
                        }
                        ReactorEvent::GovernanceEventReceived { event } => {
                            tracing::info!(
                                sender = %event.sender,
                                event_type = ?event.event_type,
                                "Governance event received"
                            );
                        }
                    }
                }
            });

            // Start the gateway HTTP server (blocks).
            let bind_address = format!("0.0.0.0:{api_port}");
            let gateway_config = exo_gateway::server::GatewayConfig {
                bind_address: bind_address.clone(),
                ..exo_gateway::server::GatewayConfig::default()
            };

            tracing::info!(
                %bind_address,
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
            // TODO(Phase 4): connect to a running node's API to query peers
            println!("Peer listing requires a running node. Use `exochain start` first.");
            Ok(())
        }
    }
}
