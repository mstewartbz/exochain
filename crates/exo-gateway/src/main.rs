//! decision.forum API server binary.

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    exo_gateway::server::run_server(port).await;
}
