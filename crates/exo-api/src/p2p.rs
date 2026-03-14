use libp2p::{
    noise,
    swarm::NetworkBehaviour,
    tcp, yamux,
};
use std::time::Duration;

/// Custom Network Behaviour (Stub).
/// In future, this will include Gossipsub and Kademlia.
#[derive(NetworkBehaviour)]
pub struct ExoBehaviour {
    pub ping: libp2p::ping::Behaviour,
}

pub async fn start_p2p_node() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create Identity
    let id_keys = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = id_keys.public().to_peer_id();

    tracing::info!("Local Peer ID: {}", peer_id);

    // 2. Build swarm with transport + behaviour
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_| ExoBehaviour {
            ping: libp2p::ping::Behaviour::default(),
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // 3. Listen
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // 4. Event Loop (Stub — future: Gossipsub + Kademlia DHT for node discovery)
    Ok(())
}
