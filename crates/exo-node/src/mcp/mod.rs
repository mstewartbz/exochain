//! MCP (Model Context Protocol) server — constitutional AI interface.
//!
//! Embeds the MCP server directly in the exo-node process, giving AI agents
//! access to governance operations through constitutionally enforced tools.
//! Every tool invocation is verified by the CGR Kernel and MCP enforcement rules.
//!
//! ## Usage
//!
//! ```bash
//! exochain mcp                        # start MCP server on stdio
//! exochain mcp --actor-did did:exo:x  # use a specific DID
//! ```

pub mod error;
pub mod handler;
pub mod middleware;
pub mod protocol;
pub mod tools;

pub use handler::McpServer;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Run the MCP server on stdio (stdin/stdout).
///
/// Reads newline-delimited JSON-RPC messages from stdin,
/// processes them through the `McpServer`, and writes responses to stdout.
/// This is the primary transport for Claude Code and similar MCP clients.
///
/// All diagnostic logging goes to stderr so stdout remains a clean JSON-RPC channel.
pub async fn serve_stdio(server: McpServer) -> std::io::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    eprintln!("[exochain-mcp] Constitutional MCP server ready on stdio");
    eprintln!("[exochain-mcp] Actor: {}", server.actor_did());
    eprintln!("[exochain-mcp] Tools: {}", server.tool_count());

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Some(response) = server.handle_message(&line) {
            stdout.write_all(response.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
