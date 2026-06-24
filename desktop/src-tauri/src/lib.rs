use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    Manager,
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tokio::sync::oneshot;
use url::Url;
use wenyuan_runtime::{start_local_server, ServerConfig};

struct DesktopState {
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

fn app_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "wenyuan", "Wenyuan")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn lock_path() -> PathBuf {
    app_data_dir().join("desktop.lock")
}

/// Try to acquire a single-instance lock. Returns true if we're the only instance.
fn acquire_lock() -> bool {
    let path = lock_path();
    if let Ok(pid_str) = std::fs::read_to_string(&path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if pid_exists(pid) {
                return false;
            }
        }
    }
    let pid = std::process::id();
    if let Err(e) = std::fs::write(&path, pid.to_string()) {
        tracing::warn!("failed to write lock file: {e}");
    }
    true
}

#[cfg(unix)]
fn pid_exists(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}/status")).exists()
}

#[cfg(windows)]
fn pid_exists(pid: u32) -> bool {
    use std::process::Command;
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.contains(&pid.to_string()))
        .unwrap_or(false)
}

fn release_lock() {
    let _ = std::fs::remove_file(lock_path());
}

fn load_icon() -> Option<Image<'static>> {
    let img = image::load_from_memory(include_bytes!("../icons/icon.png")).ok()?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    let data: &'static [u8] = Box::leak(rgba.into_raw().into_boxed_slice());
    Some(Image::new(data, w, h))
}

fn setup_logging(data_dir: &PathBuf) {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).ok();
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

    if !acquire_lock() {
        tracing::warn!("another instance is already running");
        return;
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DesktopState {
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        })
        .setup(|app| {
            tracing::info!("Wenyuan desktop starting...");

            // Build tray menu
            let open = MenuItem::with_id(app, "open", "打开文渊阁", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, Some("CmdOrCtrl+Q"))?;
            let menu = Menu::with_items(app, &[&open, &quit])?;

            TrayIconBuilder::new()
                .icon(load_icon().unwrap_or_else(|| {
                    Image::new(&[0u8; 4], 1, 1) // fallback 1x1 transparent pixel
                }))
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            shutdown_app(app);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(event, tauri::tray::TrayIconEvent::Click { .. }) {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

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
                        // Wait for shutdown signal
                        let _ = shutdown_rx.await;
                        tracing::info!("shutting down");
                        std::process::exit(0);
                    }
                    Err(e) => {
                        tracing::error!("failed to start server: {e}");
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                // Hide instead of close
                let _ = window.hide();
                // Prevent actual close
                // If user wants to quit, they use tray menu
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Wenyuan desktop");
}

fn shutdown_app(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<DesktopState>() {
        if let Some(tx) = state.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
    release_lock();
    std::process::exit(0);
}
