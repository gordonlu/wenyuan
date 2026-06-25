use std::{collections::HashMap, env, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, broadcast};
use tracing::info;
use wenyuan_agent::AgentRunner;
use wenyuan_core::SearchBackend;
use wenyuan_server::{
    AppState, app, preferences_path_from_env,
    provider_from_settings_or_env, provider_timeout_from_env,
    search_backend_from_env, settings::SettingsManager,
};
use wenyuan_store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wenyuan_server=info,wenyuan_agent=info,tower_http=info".into()),
        )
        .init();

    let database_url =
        env::var("WENYUAN_DATABASE_URL").unwrap_or_else(|_| "sqlite://wenyuan.db".into());
    let bind = env::var("WENYUAN_BIND").unwrap_or_else(|_| "127.0.0.1:3210".into());
    let preferences_path = Arc::new(preferences_path_from_env());
    let web_dist = Arc::new(
        env::var("WENYUAN_WEB_DIST")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("web/dist")),
    );
    let store = Store::connect(&database_url).await?;
    let recovered = store.recover_stale_executions().await?;
    if recovered > 0 {
        info!("marked {recovered} stale session execution(s) as retry_required");
    }
    let data_dir = env::var("WENYUAN_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let settings_manager = SettingsManager::new(data_dir.clone());
    let settings_config = settings_manager.load_config();
    let (provider, config) = provider_from_settings_or_env(&settings_manager, &database_url);
    let search_pool = search_backend_from_env(Some(&settings_config));
    let search_backend: Option<Arc<dyn SearchBackend>> =
        search_pool.clone().map(|p| p as Arc<dyn SearchBackend>);
    if search_backend.is_some() {
        info!("search backend configured");
    }
    let state = AppState {
        store,
        runner: AgentRunner::new(provider).with_timeout(provider_timeout_from_env()),
        running: Arc::new(Mutex::new(HashMap::new())),
        event_tx: broadcast::channel(128).0,
        config,
        search_backend,
        preferences_path,
        web_dist,
        settings: Arc::new(settings_manager),
        local_token: uuid::Uuid::new_v4().to_string(),
    };

    let app = app(state);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let addr: SocketAddr = listener.local_addr()?;
    info!("Wenyuan server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
