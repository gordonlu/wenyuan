use std::path::PathBuf;
use tauri::Manager;
use url::Url;
use wenyuan_runtime::{start_local_server, ServerConfig};

fn app_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "wenyuan", "Wenyuan")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn setup_logging(data_dir: &PathBuf) {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    // Remove logs older than 7 days
    if let Ok(entries) = std::fs::read_dir(&log_dir) {
        let cutoff = std::time::SystemTime::now()
            - std::time::Duration::from_secs(7 * 24 * 3600);
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        std::fs::remove_file(entry.path()).ok();
                    }
                }
            }
        }
    }

    let file_appender = tracing_appender::rolling::daily(&log_dir, "desktop.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wenyuan_runtime=info,wenyuan_desktop=info".into()),
        )
        .with_writer(non_blocking)
        .init();

    tracing::info!("logs: {}", log_dir.join("desktop.log").display());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = app_data_dir();
    setup_logging(&data_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tracing::info!("Wenyuan desktop starting...");
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match start_local_server(ServerConfig::default()).await {
                    Ok(server) => {
                        let url_str = format!("http://{}/", server.addr);
                        tracing::info!("server ready, navigating to {}", url_str);
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        if let Some(window) = handle.get_webview_window("main") {
                            if let Ok(url) = Url::parse(&url_str) {
                                tracing::info!("navigating WebView to {url_str}");
                                if let Err(e) = window.navigate(url) {
                                    tracing::error!("WebView navigation failed: {e}");
                                }
                            }
                        } else {
                            tracing::error!("main window not found");
                        }
                        std::future::pending::<()>().await;
                    }
                    Err(e) => {
                        tracing::error!("failed to start server: {e}");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Wenyuan desktop");
}
