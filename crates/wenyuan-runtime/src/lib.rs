use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, oneshot};
use tracing::info;
use uuid::Uuid;
use rust_embed::RustEmbed;
use wenyuan_agent::AgentRunner;
use wenyuan_core::SearchBackend;
use wenyuan_server::{
    AppState, app, provider_from_settings_or_env, provider_timeout_from_env,
    search_backend_from_env, settings::SettingsManager,
};

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

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

/// Handle returned after starting the local server.
pub struct LocalServerHandle {
    pub addr: SocketAddr,
    pub token: String,
    pub shutdown_tx: oneshot::Sender<()>,
    pub data_dir: PathBuf,
}

/// Configuration for starting the local server.
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub preferences_path: PathBuf,
    pub web_dist: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let data_dir = directories::ProjectDirs::from("com", "wenyuan", "Wenyuan")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let web_dist = data_dir.join("web-dist");
        Self {
            preferences_path: data_dir.join("wenyuan-preferences.json"),
            web_dist,
            data_dir,
        }
    }
}

/// Start the local Axum server on `127.0.0.1:0` (random port).
pub async fn start_local_server(config: ServerConfig) -> anyhow::Result<LocalServerHandle> {
    tokio::fs::create_dir_all(&config.data_dir).await?;
    tokio::fs::create_dir_all(&config.web_dist).await?;
    info!("data directory: {}", config.data_dir.display());

    // Load .env from data directory
    let env_path = config.data_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
        info!("loaded env from {}", env_path.display());
    }

    // Extract embedded web assets (always overwrite)
    tokio::fs::create_dir_all(&config.web_dist).await?;
    extract_web_dist(&config.web_dist);
    info!("web assets extracted");

    let db_path = config.data_dir.join("wenyuan.db");
    let db_url = format!("sqlite://{}", db_path.display());
    let store = wenyuan_store::Store::connect(&db_url).await?;
    let recovered = store.recover_stale_executions().await?;
    if recovered > 0 {
        info!("marked {recovered} stale session execution(s) as retry_required");
    }

    let settings_manager = SettingsManager::new(config.data_dir.clone());
    let settings_config = settings_manager.load_config();
    let (provider, config_status) = provider_from_settings_or_env(&settings_manager, &db_url);
    let local_token = Uuid::new_v4().to_string();
    let search_pool = search_backend_from_env(Some(&settings_config));
    let search_backend: Option<Arc<dyn SearchBackend>> =
        search_pool.map(|p| p as Arc<dyn SearchBackend>);

    let state = AppState {
        store,
        runner: AgentRunner::new(provider).with_timeout(provider_timeout_from_env()),
        running: Arc::new(Mutex::new(HashMap::new())),
        event_tx: broadcast::channel(128).0,
        config: config_status,
        search_backend,
        preferences_path: Arc::new(config.preferences_path),
        web_dist: Arc::new(config.web_dist),
        settings: Arc::new(settings_manager),
        local_token: local_token.clone(),
    };

    let router = app(state);

    let addr: SocketAddr = "127.0.0.1:0".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;
    info!("Wenyuan server listening on http://{}", actual_addr);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
    });

    Ok(LocalServerHandle {
        addr: actual_addr,
        token: local_token,
        shutdown_tx,
        data_dir: config.data_dir,
    })
}
