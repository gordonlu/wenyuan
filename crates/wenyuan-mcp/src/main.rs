use std::path::PathBuf;
use wenyuan_mcp::{start_mcp_server, McpServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state_path = std::env::var("WENYUAN_MCP_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("mcp-meetings.json"));
    let handle = start_mcp_server(McpServerConfig::from_env(state_path)).await?;
    println!("Wenyuan MCP meeting server: http://{}/mcp", handle.addr);
    tokio::signal::ctrl_c().await?;
    let _ = handle.shutdown_tx.send(());
    Ok(())
}
