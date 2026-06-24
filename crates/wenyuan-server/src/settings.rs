use axum::{Json, Router, extract::State, routing::{get, post}};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tracing::info;
use wenyuan_core::{SecretString, mask_api_key};

use crate::{ApiError, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatProviderConfig {
    pub base_url: String,
    pub api_key: String,
    pub api_key_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub search_provider: String,
    #[serde(default)]
    pub search_api_url: String,
    #[serde(default)]
    pub seat_providers: std::collections::HashMap<String, SeatProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key_configured: bool,
    pub api_key_hint: Option<String>,
    pub api_key_source: String,
    pub search_provider: String,
    pub search_api_url: String,
    pub search_api_key_configured: bool,
    #[serde(default)]
    pub seat_providers: std::collections::HashMap<String, SeatProviderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderSettingsRequest {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub clear_api_key: bool,
    #[serde(default)]
    pub search_provider: String,
    #[serde(default)]
    pub search_api_url: String,
    #[serde(default)]
    pub seat_providers: std::collections::HashMap<String, SeatProviderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TestProviderRequest {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub use_saved_key: bool,
}

#[derive(Debug, Serialize)]
pub struct TestProviderResponse {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
}

pub struct SettingsManager {
    data_dir: PathBuf,
    config_path: PathBuf,
    secrets_path: PathBuf,
}

impl SettingsManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let secrets_path = data_dir.join("secrets.json");
        Self {
            config_path: data_dir.join("settings.json"),
            secrets_path,
            data_dir,
        }
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }

    pub fn load_config(&self) -> ProviderConfig {
        self.ensure_dir().ok();
        std::fs::read_to_string(&self.config_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_else(|| ProviderConfig {
                provider: "openai_compatible".into(),
                base_url: String::new(),
                model: String::new(),
                search_provider: String::new(),
                search_api_url: String::new(),
                seat_providers: std::collections::HashMap::new(),
            })
    }

    pub fn save_config(&self, config: &ProviderConfig) -> Result<(), std::io::Error> {
        self.ensure_dir()?;
        let raw = serde_json::to_string_pretty(config)?;
        std::fs::write(&self.config_path, raw)
    }

    pub fn load_api_key(&self) -> Option<SecretString> {
        self.ensure_dir().ok();
        std::fs::read_to_string(&self.secrets_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| v.get("api_key").and_then(|k| k.as_str().map(|s| SecretString::new(s.to_string()))))
    }

    pub fn save_api_key(&self, key: &str) -> Result<(), std::io::Error> {
        self.ensure_dir()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o600)
                .open(&self.secrets_path)?;
            let raw = serde_json::json!({ "api_key": key }).to_string();
            use std::io::Write;
            let mut file = file;
            file.write_all(raw.as_bytes())?;
            return Ok(());
        }
        #[cfg(not(unix))]
        {
            let raw = serde_json::json!({ "api_key": key }).to_string();
            std::fs::write(&self.secrets_path, raw)
        }
    }

    pub fn delete_api_key(&self) -> Result<(), std::io::Error> {
        if self.secrets_path.exists() {
            std::fs::remove_file(&self.secrets_path)?;
        }
        Ok(())
    }

    pub fn get_settings(&self) -> ProviderSettings {
        let config = self.load_config();
        let (api_key, source) = {
            let saved = self.load_api_key();
            if saved.is_some() {
                (saved, "user".to_string())
            } else {
                let env_key = env::var("WENYUAN_LLM_API_KEY").unwrap_or_default();
                if env_key.is_empty() {
                    (None, "none".to_string())
                } else {
                    (Some(SecretString::new(env_key)), "env".to_string())
                }
            }
        };
        let env_search = env::var("WENYUAN_SEARCH_PROVIDER").unwrap_or_default();
        let seats = ["MOUYUAN", "JINGSHI", "CHIZHENG"];
        let seat_providers: std::collections::HashMap<String, SeatProviderConfig> = seats
            .iter()
            .map(|&s| {
                let sp = config.seat_providers.get(s).cloned().unwrap_or(SeatProviderConfig {
                    base_url: String::new(),
                    api_key: String::new(),
                    api_key_configured: false,
                });
                let env_base = env::var(format!("WENYUAN_LLM_BASE_URL_{s}")).unwrap_or_default();
                let env_key = env::var(format!("WENYUAN_LLM_API_KEY_{s}")).unwrap_or_default();
                (s.to_string(), SeatProviderConfig {
                    base_url: if sp.base_url.is_empty() { env_base } else { sp.base_url },
                    api_key: String::new(),
                    api_key_configured: sp.api_key_configured || !env_key.is_empty(),
                })
            })
            .collect();
        ProviderSettings {
            provider: config.provider,
            base_url: config.base_url,
            model: config.model,
            api_key_configured: api_key.is_some(),
            api_key_hint: if api_key.is_some() { Some("已配置".into()) } else { None },
            api_key_source: source,
            search_provider: config.search_provider,
            search_api_url: config.search_api_url,
            search_api_key_configured: !env_search.is_empty(),
            seat_providers,
        }
    }
}

async fn get_provider_settings(State(state): State<AppState>) -> Result<Json<ProviderSettings>, ApiError> {
    let settings = state.settings.get_settings();
    Ok(Json(settings))
}

async fn update_provider_settings(
    State(state): State<AppState>,
    Json(req): Json<UpdateProviderSettingsRequest>,
) -> Result<Json<ProviderSettings>, ApiError> {
    if req.clear_api_key {
        state.settings.delete_api_key().map_err(|e| ApiError::internal(e.to_string()))?;
    } else if let Some(ref key) = req.api_key {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(ApiError::bad_request("API Key 不能为空"));
        }
        info!("updating API key: {}", mask_api_key(trimmed));
        state.settings.save_api_key(trimmed).map_err(|e| ApiError::internal(e.to_string()))?;
    }

    let mut seat_providers = state.settings.load_config().seat_providers;
    for (seat, sp) in &req.seat_providers {
        let entry = seat_providers.entry(seat.clone()).or_insert(SeatProviderConfig {
            base_url: String::new(),
            api_key: String::new(),
            api_key_configured: false,
        });
        entry.base_url = sp.base_url.clone();
        if !sp.api_key.is_empty() {
            entry.api_key = sp.api_key.clone();
            entry.api_key_configured = true;
        }
    }
    let config = ProviderConfig {
        provider: req.provider,
        base_url: req.base_url,
        model: req.model,
        search_provider: req.search_provider,
        search_api_url: req.search_api_url,
        seat_providers,
    };
    state.settings.save_config(&config).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(state.settings.get_settings()))
}

async fn test_provider(
    State(_state): State<AppState>,
    Json(req): Json<TestProviderRequest>,
) -> Json<TestProviderResponse> {
    let api_key = if req.use_saved_key {
        _state.settings.load_api_key().map(|k| k.expose().to_string())
    } else {
        req.api_key.clone()
    };

    let Some(key) = api_key else {
        return Json(TestProviderResponse {
            ok: false,
            latency_ms: None,
            message: "未配置 API Key".into(),
            error_kind: Some("unauthorized".into()),
        });
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    let body = serde_json::json!({
        "model": req.model,
        "messages": [{"role": "user", "content": "只返回 JSON：{\"ok\":true}"}],
        "max_tokens": 20
    });

    let start = std::time::Instant::now();
    let result = client
        .post(format!("{}/chat/completions", req.base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Json(TestProviderResponse {
                    ok: true,
                    latency_ms: Some(elapsed),
                    message: "连接成功".into(),
                    error_kind: None,
                })
            } else if status == 401 || status == 403 {
                Json(TestProviderResponse {
                    ok: false,
                    latency_ms: Some(elapsed),
                    message: "API Key 无效或无权限".into(),
                    error_kind: Some("unauthorized".into()),
                })
            } else if status == 429 {
                Json(TestProviderResponse {
                    ok: false,
                    latency_ms: Some(elapsed),
                    message: "请求过于频繁或额度不足".into(),
                    error_kind: Some("rate_limited".into()),
                })
            } else if status.as_u16() >= 500 {
                Json(TestProviderResponse {
                    ok: false,
                    latency_ms: Some(elapsed),
                    message: format!("服务端错误 (HTTP {})", status),
                    error_kind: Some("server_error".into()),
                })
            } else {
                Json(TestProviderResponse {
                    ok: false,
                    latency_ms: Some(elapsed),
                    message: format!("请求失败 (HTTP {})", status),
                    error_kind: Some("unknown".into()),
                })
            }
        }
        Err(e) => {
            let kind = if e.is_timeout() { "timeout" } else if e.is_connect() { "network" } else { "unknown" };
            let msg = match kind {
                "timeout" => "服务响应超时".into(),
                "network" => "无法连接到服务器，请检查 Base URL".into(),
                _ => format!("连接失败: {}", e),
            };
            Json(TestProviderResponse {
                ok: false,
                latency_ms: Some(elapsed),
                message: msg,
                error_kind: Some(kind.into()),
            })
        }
    }
}

async fn get_local_token(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "token": state.local_token }))
}

pub fn settings_routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings/provider", get(get_provider_settings).post(update_provider_settings))
        .route("/api/settings/test-provider", post(test_provider))
        .route("/api/settings/local-token", get(get_local_token))
}
