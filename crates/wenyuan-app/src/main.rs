use tracing::info;
use wenyuan_runtime::{start_local_server, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wenyuan_runtime=info".into()),
        )
        .init();

    let config = ServerConfig::default();
    let handle = start_local_server(config).await?;

    let url = format!("http://{}", handle.addr);
    info!("Opening browser at {}", url);
    let _ = open::that(&url);

    // Block until shutdown signal (never sent in CLI mode - wait forever)
    std::future::pending::<()>().await;
    Ok(())
}
