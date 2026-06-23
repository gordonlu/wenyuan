use tauri::Manager;
use url::Url;
use wenyuan_runtime::{start_local_server, ServerConfig};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match start_local_server(ServerConfig::default()).await {
                    Ok(server) => {
                        let url_str = format!("http://{}/", server.addr);
                        tracing::info!("navigating to {}", url_str);
                        if let Some(window) = handle.get_webview_window("main") {
                            if let Ok(url) = Url::parse(&url_str) {
                                let _ = window.navigate(url);
                            }
                        }
                        // Keep server alive; Tauri will kill the server on exit
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
