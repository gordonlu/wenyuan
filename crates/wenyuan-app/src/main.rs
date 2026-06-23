use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tracing::info;
use uuid::Uuid;
use rust_embed::RustEmbed;
use wenyuan_agent::AgentRunner;
use wenyuan_server::{
    AppState, app, provider_from_env, provider_timeout_from_env, settings::SettingsManager,
};

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

fn app_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "wenyuan", "Wenyuan")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn extract_web_dist(target: &PathBuf) {
    use std::io::Write;
    for file in WebAssets::iter() {
        let path = target.join(file.as_ref());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Some(asset) = WebAssets::get(file.as_ref()) {
            if let Ok(mut f) = std::fs::File::create(&path) {
                f.write_all(&asset.data).ok();
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wenyuan_app=info".into()),
        )
        .init();

    let data_dir = app_data_dir();
    tokio::fs::create_dir_all(&data_dir).await?;
    info!("data directory: {}", data_dir.display());

    let web_dist = data_dir.join("web-dist");
    if !web_dist.join("index.html").exists() {
        std::fs::create_dir_all(&web_dist).ok();
        extract_web_dist(&web_dist);
        info!("web assets extracted");
    }

    let db_path = data_dir.join("wenyuan.db");
    let db_url = format!("sqlite://{}", db_path.display());
    let store = wenyuan_store::Store::connect(&db_url).await?;
    let recovered = store.recover_stale_executions().await?;
    if recovered > 0 {
        info!("marked {recovered} stale session execution(s) as retry_required");
    }

    let (provider, config) = provider_from_env(&db_url);

    let state = AppState {
        store,
        runner: AgentRunner::new(provider).with_timeout(provider_timeout_from_env()),
        running: Arc::new(Mutex::new(HashMap::new())),
        event_tx: broadcast::channel(128).0,
        config,
        search_backend: None,
        preferences_path: Arc::new(data_dir.join("wenyuan-preferences.json")),
        web_dist: Arc::new(web_dist),
        settings: Arc::new(SettingsManager::new(data_dir)),
        local_token: Uuid::new_v4().to_string(),
    };

    let app = app(state);

    let addr: SocketAddr = "127.0.0.1:0".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;
    info!("Wenyuan listening on http://{}", actual_addr);

    let url = format!("http://{}", actual_addr);
    let _ = open::that(&url);

    axum::serve(listener, app).await?;
    Ok(())
}
