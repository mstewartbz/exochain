//! EXOCHAIN Gateway server binary.
//!
//! This binary is a placeholder for the full HTTP gateway server.
//! The gateway library (`exo_gateway`) provides configuration, routing,
//! and middleware types. The actual server runtime (tokio, database pool)
//! is not yet integrated. See docs/guides/DEPLOYMENT.md for the current
//! deployment approach using the Node.js demo platform.
//!
//! TODO: Integrate tokio runtime, database pool, and HTTP listener
//! once the gateway API surface is stabilized.

fn main() {
    let config = exo_gateway::server::GatewayConfig::default();
    println!("[EXOCHAIN] Gateway configured: {}", config.bind_address);
    println!(
        "[EXOCHAIN] Gateway binary not yet implemented — use demo/services/gateway-api for the current API server."
    );
    println!("[EXOCHAIN] See docs/guides/DEPLOYMENT.md for deployment instructions.");
}
