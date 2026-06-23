use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, broadcast};
use tokio_stream::{Stream, StreamExt};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{error, info};
use uuid::Uuid;
use wenyuan_agent::{AgentError, AgentRunner, CancellationFlag, ProgressSink};
use wenyuan_core::{
    DeliberationMode, Evidence, SearchBackend, SeatKind, SeatModelConfig,
    Session, SessionPhase, VotePolicy, VoteStrategy,
};
use wenyuan_provider::{
    CustomSearchBackend, DoubaoBackend, GoogleCustomSearchBackend,
    LlmProvider, MockProvider, MockScenario, OpenAiCompatibleConfig, OpenAiCompatibleProvider,
    SearXNGSearchBackend, SearchPool, SeatRoutedProvider, TavilyBackend,
};
use wenyuan_store::{SessionDetails, Store};
use wenyuan_tools::{
    CodeSearchResultSet, ParsedDocument, code_search_to_evidence, document_to_evidence,
    make_tool_run, parse_document_bytes, search_code,
};

pub mod settings;

const MAX_TITLE_CHARS: usize = 200;
const MAX_TOPIC_CHARS: usize = 16_000;
const MAX_CONTEXT_CHARS: usize = 80_000;
const MAX_EXTERNAL_EVIDENCE_ITEMS: usize = 80;
const MAX_EXTERNAL_TOOL_RUNS: usize = 40;
const MAX_EXTERNAL_EVIDENCE_CHARS: usize = 3_500;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub runner: AgentRunner,
    pub running: Arc<Mutex<HashMap<Uuid, CancellationFlag>>>,
    pub event_tx: broadcast::Sender<Uuid>,
    pub config: ConfigStatus,
    pub search_backend: Option<Arc<dyn SearchBackend>>,
    pub preferences_path: Arc<PathBuf>,
    pub web_dist: Arc<PathBuf>,
    pub settings: Arc<settings::SettingsManager>,
}

struct StoreProgressSink {
    session_id: Uuid,
    store: Store,
    event_tx: broadcast::Sender<Uuid>,
}

#[async_trait]
impl ProgressSink for StoreProgressSink {
    async fn emit(&self, event_type: &str, payload: serde_json::Value) {
        if event_type == "phase_started"
            && let Some(phase) = payload
                .get("phase")
                .and_then(|value| serde_json::from_value(value.clone()).ok())
            && let Err(err) = self
                .store
                .update_session_phase(self.session_id, phase)
                .await
        {
            error!("failed to update session phase: {err}");
        }
        if let Err(err) = self
            .store
            .append_event(self.session_id, event_type, payload)
            .await
        {
            error!("failed to append progress event: {err}");
            return;
        }
        let _ = self.event_tx.send(self.session_id);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigStatus {
    pub provider_configured: bool,
    pub provider_kind: String,
    pub model: String,
    pub seat_models: HashMap<String, String>,
    pub database_url: String,
    pub version: String,
    pub search_provider: String,
    pub available_models: Vec<ModelOption>,
    pub seat_available_models: HashMap<String, Vec<ModelOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    #[serde(default)]
    pub defaults: UserPreferenceDefaults,
    #[serde(default)]
    pub models: UserPreferenceModels,
    #[serde(default)]
    pub tools: UserPreferenceTools,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            defaults: UserPreferenceDefaults::default(),
            models: UserPreferenceModels::default(),
            tools: UserPreferenceTools::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferenceDefaults {
    #[serde(default)]
    pub mode: DeliberationMode,
    #[serde(default)]
    pub scribe_enabled: bool,
    #[serde(default)]
    pub search_enabled: bool,
    #[serde(default)]
    pub vote_strategy: VoteStrategy,
    #[serde(default = "default_allow_self_vote")]
    pub allow_self_vote: bool,
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
}

impl Default for UserPreferenceDefaults {
    fn default() -> Self {
        Self {
            mode: DeliberationMode::default(),
            scribe_enabled: false,
            search_enabled: false,
            vote_strategy: VoteStrategy::SimpleMajority,
            allow_self_vote: true,
            view_mode: default_view_mode(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPreferenceModels {
    #[serde(default)]
    pub mouyuan: String,
    #[serde(default)]
    pub jingshi: String,
    #[serde(default)]
    pub chizheng: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferenceTools {
    #[serde(default = "default_code_search_root")]
    pub code_search_root: String,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
}

impl Default for UserPreferenceTools {
    fn default() -> Self {
        Self {
            code_search_root: default_code_search_root(),
            max_file_size_mb: default_max_file_size_mb(),
        }
    }
}

pub fn default_allow_self_vote() -> bool {
    true
}

pub fn default_view_mode() -> String {
    "workbench".into()
}

pub fn default_code_search_root() -> String {
    ".".into()
}

pub fn default_max_file_size_mb() -> u64 {
    max_file_size_mb_from_env()
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub title: String,
    pub topic: String,
    pub context: Option<String>,
    pub mode: Option<DeliberationMode>,
    pub model_config: Option<HashMap<SeatKind, SeatModelConfig>>,
    pub vote_policy: Option<VotePolicy>,
    pub scribe_enabled: Option<bool>,
    pub search_enabled: Option<bool>,
    pub external_evidence: Option<Vec<Evidence>>,
    pub external_tool_runs: Option<Vec<wenyuan_core::ToolRun>>,
}

#[derive(Debug, Serialize)]
pub struct TestTopic {
    pub category: String,
    pub topic: String,
    pub context: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Deserialize)]
pub struct ParseDocumentRequest {
    pub filename: String,
    pub mime_type: Option<String>,
    pub content_base64: String,
}

#[derive(Debug, Serialize)]
pub struct ParseDocumentResponse {
    pub document: ParsedDocument,
    pub evidence: Vec<Evidence>,
    pub tool_run: wenyuan_core::ToolRun,
}

#[derive(Debug, Deserialize)]
pub struct CodeSearchRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct CodeSearchResponse {
    pub result: CodeSearchResultSet,
    pub evidence: Vec<Evidence>,
    pub tool_run: wenyuan_core::ToolRun,
}



pub fn provider_timeout_from_env() -> Duration {
    env::var("WENYUAN_LLM_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(120))
}

pub fn max_request_body_bytes() -> usize {
    env::var("WENYUAN_MAX_REQUEST_BODY_MB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|mb| *mb > 0)
        .unwrap_or(32)
        .saturating_mul(1024 * 1024)
}

pub fn max_file_size_mb_from_env() -> u64 {
    let body_budget_mb = (max_request_body_bytes() as u64).saturating_mul(3) / (4 * 1024 * 1024);
    env::var("WENYUAN_MAX_FILE_SIZE_MB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|mb| *mb > 0)
        .unwrap_or(5)
        .min(body_budget_mb.max(1))
}

pub fn preferences_path_from_env() -> PathBuf {
    env::var("WENYUAN_PREFERENCES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("wenyuan-preferences.json"))
}

pub fn load_preferences(path: &FsPath) -> Result<UserPreferences, ApiError> {
    if !path.exists() {
        return Ok(UserPreferences::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| ApiError::internal(format!("failed to read preferences: {err}")))?;
    let preferences = serde_json::from_str(&raw)
        .map_err(|err| ApiError::bad_request(format!("invalid preferences json: {err}")))?;
    Ok(normalize_preferences(preferences))
}

pub fn save_preferences(path: &FsPath, preferences: &UserPreferences) -> Result<(), ApiError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            ApiError::internal(format!("failed to create preferences dir: {err}"))
        })?;
    }
    let raw = serde_json::to_string_pretty(preferences)
        .map_err(|err| ApiError::internal(format!("failed to encode preferences: {err}")))?;
    fs::write(path, format!("{raw}\n"))
        .map_err(|err| ApiError::internal(format!("failed to write preferences: {err}")))
}

pub fn normalize_preferences(mut preferences: UserPreferences) -> UserPreferences {
    preferences.defaults.view_mode = match preferences.defaults.view_mode.as_str() {
        "report" => "report".into(),
        _ => default_view_mode(),
    };
    if preferences.tools.code_search_root.trim().is_empty() {
        preferences.tools.code_search_root = default_code_search_root();
    }
    preferences.tools.max_file_size_mb = preferences
        .tools
        .max_file_size_mb
        .clamp(1, max_file_size_mb_from_env());
    preferences
}

pub fn search_backend_from_env() -> Option<Arc<SearchPool>> {
    let provider = env::var("WENYUAN_SEARCH_PROVIDER")
        .unwrap_or_default()
        .to_lowercase()
        .trim()
        .to_string();
    if provider.is_empty() || matches!(provider.as_str(), "none" | "off" | "disabled") {
        info!("WENYUAN_SEARCH_PROVIDER not set; search disabled");
        return None;
    }
    let mut backends: Vec<Box<dyn SearchBackend>> = Vec::new();
    for name in provider.split(',') {
        match name.trim() {
            "custom" => {
                let url = env::var("WENYUAN_SEARCH_API_URL").unwrap_or_default();
                if !url.is_empty() {
                    let key = env::var("WENYUAN_SEARCH_API_KEY").ok();
                    backends.push(Box::new(CustomSearchBackend::new(url, key)));
                }
            }
            "doubao" => {
                let key = env::var("WENYUAN_SEARCH_DOUBAO_KEY").unwrap_or_default();
                if !key.is_empty() {
                    backends.push(Box::new(DoubaoBackend::new(key)));
                }
            }
            "tavily" => {
                let key = env::var("WENYUAN_SEARCH_TAVILY_KEY").unwrap_or_default();
                if !key.is_empty() {
                    backends.push(Box::new(TavilyBackend::new(key)));
                }
            }
            "google" => {
                let key = env::var("WENYUAN_SEARCH_GOOGLE_KEY").unwrap_or_default();
                let cx = env::var("WENYUAN_SEARCH_GOOGLE_CX").unwrap_or_default();
                if !key.is_empty() && !cx.is_empty() {
                    backends.push(Box::new(GoogleCustomSearchBackend::new(key, cx)));
                }
            }
            "searxng" => {
                let url = env::var("WENYUAN_SEARCH_SEARXNG_URL").unwrap_or_default();
                if !url.is_empty() {
                    backends.push(Box::new(SearXNGSearchBackend::new(url)));
                }
            }
            _ => info!("unknown search provider: {}", name.trim()),
        }
    }
    if backends.is_empty() {
        info!("WENYUAN_SEARCH_PROVIDER={provider} but no backends could be initialized (missing keys/URLs); search disabled");
        return None;
    }
    Some(Arc::new(SearchPool::new(backends)))
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/config/status", get(config_status))
        .route(
            "/api/preferences",
            get(get_preferences).put(update_preferences),
        )
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/{id}", get(get_session).delete(delete_session))
        .route("/api/sessions/{id}/start", post(start_session))
        .route("/api/sessions/{id}/retry", post(retry_session))
        .route("/api/sessions/{id}/cancel", post(cancel_session))
        .route("/api/sessions/{id}/pause", post(pause_session))
        .route("/api/sessions/{id}/resume", post(resume_session))
        .route("/api/sessions/{id}/context", post(update_context))
        .route("/api/sessions/{id}/retry-seat/{seat}", post(retry_seat))
        .route("/api/sessions/{id}/retry-phase", post(retry_phase))
        .route("/api/sessions/{id}/manual-revision", post(manual_revision))
        .route("/api/sessions/{id}/trajectory", get(phase_trajectory))
        .route("/api/sessions/{id}/events", get(events_sse))
        .route("/api/tools/documents/parse", post(parse_document))
        .route("/api/tools/code/search", post(search_code_tool))

        .route("/api/test-topics", get(test_topics))
        .merge(settings::settings_routes())
        .fallback_service(ServeDir::new(state.web_dist.as_path()).fallback(ServeFile::new(state.web_dist.join("index.html"))))
        .layer(DefaultBodyLimit::max(max_request_body_bytes()))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub fn provider_from_env(database_url: &str) -> (Arc<dyn LlmProvider>, ConfigStatus) {
    let base_url = env::var("WENYUAN_LLM_BASE_URL").unwrap_or_default();
    let api_key = env::var("WENYUAN_LLM_API_KEY").unwrap_or_default();
    let model = env::var("WENYUAN_LLM_MODEL").unwrap_or_else(|_| "mock-model".into());
    let global_provider_configured = !base_url.is_empty() && !api_key.is_empty();
    let seat_provider_configured = SeatKind::ALL.into_iter().any(seat_provider_configured);
    let provider_kind = if global_provider_configured || seat_provider_configured {
        "openai-compatible".to_string()
    } else {
        "mock".to_string()
    };
    let (provider, seat_models): (Arc<dyn LlmProvider>, HashMap<String, String>) =
        if provider_kind == "openai-compatible" {
            routed_provider_from_env(base_url.clone(), api_key.clone(), model.clone())
        } else {
            (
                Arc::new(MockProvider::new(mock_scenario_from_env())),
                SeatKind::ALL
                    .into_iter()
                    .map(|seat| (seat_env_key(seat).to_string(), model.clone()))
                    .collect(),
            )
        };
    let available_models = parse_available_models();
    let seat_available_models = HashMap::from([
        ("MOUYUAN".to_string(), parse_available_models_for("MOUYUAN")),
        ("JINGSHI".to_string(), parse_available_models_for("JINGSHI")),
        (
            "CHIZHENG".to_string(),
            parse_available_models_for("CHIZHENG"),
        ),
    ]);
    (
        provider,
        ConfigStatus {
            provider_configured: provider_kind == "mock"
                || global_provider_configured
                || seat_provider_configured,
            provider_kind,
            model,
            seat_models,
            database_url: database_url.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            search_provider: env::var("WENYUAN_SEARCH_PROVIDER")
                .unwrap_or_default(),
            available_models,
            seat_available_models,
        },
    )
}

pub fn seat_provider_configured(seat: SeatKind) -> bool {
    let suffix = seat_env_key(seat);
    let base_url = env::var(format!("WENYUAN_LLM_BASE_URL_{suffix}")).unwrap_or_default();
    let api_key = env::var(format!("WENYUAN_LLM_API_KEY_{suffix}")).unwrap_or_default();
    !base_url.is_empty() && !api_key.is_empty()
}

pub fn parse_available_models_for(suffix: &str) -> Vec<ModelOption> {
    let key = format!("WENYUAN_AVAILABLE_MODELS_{suffix}");
    if let Ok(val) = env::var(&key) {
        parse_model_list(&val)
    } else {
        vec![]
    }
}

pub fn parse_available_models() -> Vec<ModelOption> {
    parse_model_list(&env::var("WENYUAN_AVAILABLE_MODELS").unwrap_or_default())
}

pub fn parse_model_list(input: &str) -> Vec<ModelOption> {
    input
        .split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            if let Some((value, label)) = entry.split_once(':') {
                Some(ModelOption {
                    value: value.trim().to_string(),
                    label: label.trim().to_string(),
                })
            } else {
                Some(ModelOption {
                    value: entry.to_string(),
                    label: entry.to_string(),
                })
            }
        })
        .collect()
}

pub fn routed_provider_from_env(
    default_base_url: String,
    default_api_key: String,
    default_model: String,
) -> (Arc<dyn LlmProvider>, HashMap<String, String>) {
    let has_default_provider = !default_base_url.is_empty() && !default_api_key.is_empty();
    let default_provider: Arc<dyn LlmProvider> = if has_default_provider {
        Arc::new(OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url: default_base_url.clone(),
            api_key: default_api_key.clone(),
            model: default_model.clone(),
            reasoning_effort: env::var("WENYUAN_LLM_REASONING_EFFORT").ok(),
            max_tokens: env_u32("WENYUAN_LLM_MAX_TOKENS"),
        }))
    } else {
        Arc::new(MockProvider::new(mock_scenario_from_env()))
    };
    let mut routed = SeatRoutedProvider::new(default_provider);
    let mut seat_models = HashMap::new();

    for seat in SeatKind::ALL {
        let suffix = seat_env_key(seat);
        let base_url =
            env::var(format!("WENYUAN_LLM_BASE_URL_{suffix}")).unwrap_or(default_base_url.clone());
        let api_key =
            env::var(format!("WENYUAN_LLM_API_KEY_{suffix}")).unwrap_or(default_api_key.clone());
        let model = env::var(format!("WENYUAN_LLM_MODEL_{suffix}"))
            .unwrap_or_else(|_| default_model.clone());
        let display_model = if model.is_empty() {
            "mock-model".to_string()
        } else {
            model.clone()
        };
        seat_models.insert(suffix.to_string(), display_model);

        if !base_url.is_empty()
            && !api_key.is_empty()
            && (!has_default_provider
                || base_url != default_base_url
                || api_key != default_api_key
                || model != default_model)
        {
            routed = routed.with_seat_provider(
                seat,
                Arc::new(OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
                    base_url,
                    api_key,
                    model: if model.is_empty() {
                        default_model.clone()
                    } else {
                        model
                    },
                    reasoning_effort: env::var(format!("WENYUAN_LLM_REASONING_EFFORT_{suffix}"))
                        .ok()
                        .or_else(|| env::var("WENYUAN_LLM_REASONING_EFFORT").ok()),
                    max_tokens: env_u32(&format!("WENYUAN_LLM_MAX_TOKENS_{suffix}"))
                        .or_else(|| env_u32("WENYUAN_LLM_MAX_TOKENS")),
                })),
            );
        }
    }

    (Arc::new(routed), seat_models)
}

pub fn env_u32(key: &str) -> Option<u32> {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
}

pub fn seat_env_key(seat: SeatKind) -> &'static str {
    match seat {
        SeatKind::Mouyuan => "MOUYUAN",
        SeatKind::Jingshi => "JINGSHI",
        SeatKind::Chizheng => "CHIZHENG",
    }
}

pub fn mock_scenario_from_env() -> MockScenario {
    match env::var("WENYUAN_MOCK_SCENARIO")
        .unwrap_or_default()
        .as_str()
    {
        "timeout" => MockScenario::Timeout,
        "single_seat_timeout" => MockScenario::SingleSeatTimeout,
        "malformed_then_repair" => MockScenario::MalformedThenRepair,
        "always_malformed" => MockScenario::AlwaysMalformed,
        "single_seat_failure" => MockScenario::SingleSeatFailure,
        "split_then_convergence" => MockScenario::SplitThenConvergence,
        _ => MockScenario::SuccessMajority,
    }
}

const TEST_TOPICS: &[(&str, &str, &str)] = &[
    (
        "产品方向",
        "我们的产品是一个面向开发者的API管理工具，目前月活5万。团队正在争论应该深耕现有用户做企业版，还是横向扩展做一个面向非技术用户的低代码版本。现有用户反馈企业版需求强烈，但新市场可能更大。团队只有10人，资源有限。",
        "使用三年数据：API调用量年增40%，但免费用户流失率60%。企业用户续费率95%，平均客单价$2000/月。低代码市场预计年增25%，但需要全新的UI和文档体系。",
    ),
    (
        "系统架构",
        "一个日活200万的社交阅读应用，后端是单体Ruby on Rails架构，数据库PostgreSQL单实例。最近频繁出现性能瓶颈：首页加载>3秒，热门内容时段CPU到90%，数据库连接池经常打满。需要决定是继续优化单体还是开始拆微服务。",
        "当前服务器配置：4台8核32G。MySQL只读副本已启用但效果有限。团队5人，Rails经验丰富但无人有Kubernetes经验。2024年Q2目标是将首页加载降到1秒以内。",
    ),
    (
        "技术选型",
        "团队需要为新的实时协作编辑器选择技术方案。候选：A) 自研基于OT/CRDT的方案；B) 基于Yjs + WebSocket；C) 直接嵌入成熟编辑器（如Liveblocks、Cocalc）。需要支持100人同时编辑同一文档，离线编辑，版本历史和冲突解决。",
        "团队有3名全栈工程师，2月内需要交付MVP。已有Node.js和PostgreSQL基础设施。用户预期类比Google Docs的协作体验。没有WebSocket运维经验。",
    ),
    (
        "隐私与安全",
        "一个面向学校的学生数据分析平台，需要决定数据留存策略。法规要求数据保留至少3年，但家长组织要求最小化留存。同时需要支持AI驱动的学习建议，这需要足够的历史数据训练模型。",
        "平台现有10万学生数据。计划明年扩展到50万。安全审计显示当前日志系统将原始请求记录到文本文件。没有专门的安全工程师。",
    ),
    (
        "功能优先级",
        "开发团队产能只够在下一季度完成3个重要功能之一：A) 实时协作白板（预期提升用户粘性30%）；B) AI驱动的智能搜索（预期提升内容发现率20%）；C) 第三方集成市场（预期提升付费转化率15%）。请给出优先级排序和理由。",
        "当前用户日均使用时长12分钟，搜索使用率35%，付费转化率2.1%。竞品已经上线了白板和AI搜索功能。",
    ),
    (
        "商业模式",
        "一个开源的开发者工具项目，GitHub 15k stars，月下载量50万。目前靠捐赠和个人维护。需要决定是否商业化以及如何商业化。可选：A) 提供托管云服务；B) 开源核心+企业功能；C) 保持纯开源，通过咨询和培训盈利。",
        "项目已持续4年，核心贡献者3人。企业用户占比20%但贡献了80%的使用量。竞品SaaS定价$29/月起。社区对商业化反应敏感。",
    ),
    (
        "开源项目路线",
        "一个Python数据可视化库，面临核心API设计过时、性能瓶颈和新兴竞争（如Observable Plot、Vega-Lite）。需要制定下一年的路线图：是进行大规模API重构（可能破坏向后兼容），还是增量改进性能，还是转向新的核心技术方向。",
        "项目有200+贡献者，依赖者超过1000个库。每年新增issue 500+。核心维护者只有2人有时间进行大规模重构。社区在GitHub Discussion中47%支持重构，32%担心兼容性。",
    ),
    (
        "故障根因分析",
        "线上服务在过去两周出现了3次间歇性的500错误，每次持续5-15分钟，然后自动恢复。监控显示错误期间CPU和内存正常，但数据库连接数异常增加。最近唯一的生产变更是将ORM从ActiveRecord切换到了Sequel。",
        "服务部署在AWS ECS Fargate，自动扩缩容。数据库是Aurora PostgreSQL。慢查询日志没有异常。错误日志中出现了connection pool timeout和occasional deadlocks。回滚ORM变更后问题消失。",
    ),
];

pub async fn test_topics() -> Json<Vec<TestTopic>> {
    Json(
        TEST_TOPICS
            .iter()
            .map(|(category, topic, context)| TestTopic {
                category: category.to_string(),
                topic: topic.to_string(),
                context: context.to_string(),
            })
            .collect(),
    )
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

pub async fn parse_document(
    State(state): State<AppState>,
    Json(input): Json<ParseDocumentRequest>,
) -> Result<Json<ParseDocumentResponse>, ApiError> {
    let started = Instant::now();
    if input.filename.trim().is_empty() {
        return Err(ApiError::bad_request("filename is required"));
    }
    let bytes = BASE64
        .decode(input.content_base64.trim())
        .map_err(|err| ApiError::bad_request(format!("invalid base64 content: {err}")))?;
    let preferences = load_preferences(&state.preferences_path)?;
    let max_bytes = preferences
        .tools
        .max_file_size_mb
        .saturating_mul(1024 * 1024);
    if max_bytes > 0 && bytes.len() as u64 > max_bytes {
        return Err(ApiError::bad_request(format!(
            "document is too large: {} bytes",
            bytes.len()
        )));
    }
    let document = parse_document_bytes(&input.filename, input.mime_type.as_deref(), &bytes)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let evidence = document_to_evidence(&document, SeatKind::Mouyuan);
    let tool_run = make_tool_run(
        "document_parse",
        format!("filename={}", document.filename),
        "completed",
        started.elapsed(),
        evidence.iter().map(|item| item.id).collect(),
        None,
    );
    Ok(Json(ParseDocumentResponse {
        document,
        evidence,
        tool_run,
    }))
}

pub async fn search_code_tool(
    State(state): State<AppState>,
    Json(input): Json<CodeSearchRequest>,
) -> Result<Json<CodeSearchResponse>, ApiError> {
    let started = Instant::now();
    let preferences = load_preferences(&state.preferences_path)?;
    let root = env::var("WENYUAN_CODE_SEARCH_ROOT")
        .unwrap_or_else(|_| preferences.tools.code_search_root.clone());
    let root = resolve_code_search_root(&root)?;
    let result =
        search_code(root, &input.query).map_err(|err| ApiError::bad_request(err.to_string()))?;
    let evidence = code_search_to_evidence(&result, SeatKind::Mouyuan);
    let tool_run = make_tool_run(
        "code_search",
        format!("query={}", input.query.trim()),
        "completed",
        started.elapsed(),
        evidence.iter().map(|item| item.id).collect(),
        None,
    );
    Ok(Json(CodeSearchResponse {
        result,
        evidence,
        tool_run,
    }))
}

pub fn resolve_code_search_root(input: &str) -> Result<PathBuf, ApiError> {
    let root = FsPath::new(input)
        .canonicalize()
        .map_err(|err| ApiError::bad_request(format!("invalid code search root: {err}")))?;
    let allowed_roots = allowed_code_search_roots();
    if allowed_roots
        .iter()
        .any(|allowed| root.starts_with(allowed))
    {
        Ok(root)
    } else {
        Err(ApiError::bad_request(
            "code search root is outside allowed roots",
        ))
    }
}

pub fn allowed_code_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(cwd) = env::current_dir().and_then(|path| path.canonicalize()) {
        roots.push(cwd);
    }
    if let Ok(extra) = env::var("WENYUAN_CODE_SEARCH_ALLOW_ROOTS") {
        for entry in extra.split(';').flat_map(|part| part.split(',')) {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if let Ok(path) = FsPath::new(entry).canonicalize() {
                roots.push(path);
            }
        }
    }
    roots
}

pub async fn config_status(State(state): State<AppState>) -> Json<ConfigStatus> {
    Json(state.config)
}

pub async fn get_preferences(State(state): State<AppState>) -> Result<Json<UserPreferences>, ApiError> {
    Ok(Json(load_preferences(&state.preferences_path)?))
}

pub async fn update_preferences(
    State(state): State<AppState>,
    Json(input): Json<UserPreferences>,
) -> Result<Json<UserPreferences>, ApiError> {
    let preferences = normalize_preferences(input);
    save_preferences(&state.preferences_path, &preferences)?;
    Ok(Json(preferences))
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(input): Json<CreateSessionRequest>,
) -> Result<Json<Session>, ApiError> {
    validate_create_session_request(&input)?;
    let mut session = Session::new_with_mode(
        input.title,
        input.topic,
        input.context.unwrap_or_default(),
        input.mode.unwrap_or_default(),
    );
    session.model_config = input.model_config;
    session.vote_policy = input.vote_policy;
    if let Some(enabled) = input.scribe_enabled {
        session.scribe_enabled = enabled;
    }
    if let Some(enabled) = input.search_enabled {
        session.search_enabled = enabled;
    }
    session.external_evidence = input.external_evidence.unwrap_or_default();
    session.external_tool_runs = input.external_tool_runs.unwrap_or_default();
    state
        .store
        .create_session_with_provider_refs(&session, &seat_provider_refs(&state.config))
        .await?;
    Ok(Json(session))
}

pub fn validate_create_session_request(input: &CreateSessionRequest) -> Result<(), ApiError> {
    ensure_char_limit("title", &input.title, MAX_TITLE_CHARS)?;
    ensure_char_limit("topic", &input.topic, MAX_TOPIC_CHARS)?;
    if let Some(context) = &input.context {
        ensure_char_limit("context", context, MAX_CONTEXT_CHARS)?;
    }
    if let Some(evidence) = &input.external_evidence {
        if evidence.len() > MAX_EXTERNAL_EVIDENCE_ITEMS {
            return Err(ApiError::bad_request(format!(
                "too many external evidence items: max {MAX_EXTERNAL_EVIDENCE_ITEMS}"
            )));
        }
        for item in evidence {
            ensure_char_limit(
                "external evidence content",
                &item.content,
                MAX_EXTERNAL_EVIDENCE_CHARS,
            )?;
            ensure_char_limit("external evidence source", &item.source, 1_000)?;
        }
    }
    if let Some(runs) = &input.external_tool_runs
        && runs.len() > MAX_EXTERNAL_TOOL_RUNS
    {
        return Err(ApiError::bad_request(format!(
            "too many external tool runs: max {MAX_EXTERNAL_TOOL_RUNS}"
        )));
    }
    Ok(())
}

pub fn ensure_char_limit(field: &str, value: &str, max: usize) -> Result<(), ApiError> {
    if value.chars().count() > max {
        Err(ApiError::bad_request(format!(
            "{field} is too long: max {max} chars"
        )))
    } else {
        Ok(())
    }
}

pub fn seat_provider_refs(config: &ConfigStatus) -> HashMap<SeatKind, String> {
    SeatKind::ALL
        .into_iter()
        .filter_map(|seat| {
            config
                .seat_models
                .get(seat_env_key(seat))
                .map(|model| (seat, format!("{}:{model}", config.provider_kind)))
        })
        .collect()
}

pub async fn list_sessions(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.store.list_sessions().await?))
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    Ok(Json(state.store.get_session(id).await?))
}

pub async fn start_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    let details = state.store.get_session(id).await?;
    if matches!(
        details.session.phase,
        SessionPhase::Completed | SessionPhase::Cancelled
    ) {
        return Ok(Json(details));
    }

    let Some(execution_token) = state.store.try_acquire_execution(id, 900).await? else {
        return Err(ApiError::conflict("session is already running"));
    };

    let mut running = state.running.lock().await;
    if running.contains_key(&id) {
        state
            .store
            .complete_execution(id, execution_token)
            .await
            .map_err(ApiError::from)?;
        return Err(ApiError::conflict("session is already running"));
    }
    let cancel = CancellationFlag::default();
    running.insert(id, cancel.clone());
    drop(running);

    let store = state.store.clone();
    let running_map = state.running.clone();
    let event_tx = state.event_tx.clone();
    let session = details.session;
    let mut runner = state.runner.clone();
    if session.search_enabled {
        if let Some(ref search) = state.search_backend {
            runner = runner.with_search(search.clone());
        }
    }
    tokio::spawn(async move {
        let progress = Arc::new(StoreProgressSink {
            session_id: id,
            store: store.clone(),
            event_tx: event_tx.clone(),
        });
        let result = runner
            .run_session_with_progress(session, None, cancel, Some(progress))
            .await;
        match result {
            Ok(artifacts) => {
                let active = store
                    .is_execution_active(id, execution_token)
                    .await
                    .unwrap_or(false);
                if !active {
                    running_map.lock().await.remove(&id);
                    return;
                }
                if let Some(final_session) = &artifacts.session {
                    if let Err(err) = store.update_session(final_session).await {
                        error!("failed to update session: {err}");
                        let _ = store
                            .fail_session(
                                id,
                                Some(execution_token),
                                &format!("failed to persist final session: {err}"),
                            )
                            .await;
                        let _ = event_tx.send(id);
                        running_map.lock().await.remove(&id);
                        return;
                    }
                    if let Err(err) = store.save_artifacts(id, &artifacts).await {
                        error!("failed to save artifacts: {err}");
                        let _ = store
                            .fail_session(
                                id,
                                Some(execution_token),
                                &format!("failed to persist artifacts: {err}"),
                            )
                            .await;
                        let _ = event_tx.send(id);
                        running_map.lock().await.remove(&id);
                        return;
                    }
                    if let Err(err) = store.complete_execution(id, execution_token).await {
                        error!("failed to complete session execution: {err}");
                    }
                }
            }
            Err(err) => {
                if matches!(err, AgentError::Cancelled) {
                    let active = store
                        .is_execution_active(id, execution_token)
                        .await
                        .unwrap_or(false);
                    if !active {
                        let _ = event_tx.send(id);
                        running_map.lock().await.remove(&id);
                        return;
                    }
                }
                error!("session failed: {err}");
                if let AgentError::PhaseFailed { traces, .. } = &err
                    && let Err(save_err) = store.save_seat_runs(id, traces).await
                {
                    error!("failed to save failed seat runs: {save_err}");
                }
                let _ = store
                    .fail_session(id, Some(execution_token), &err.to_string())
                    .await;
            }
        }
        let _ = event_tx.send(id);
        running_map.lock().await.remove(&id);
    });

    Ok(Json(state.store.get_session(id).await?))
}

pub async fn retry_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    if state.running.lock().await.contains_key(&id) {
        return Err(ApiError::conflict("session is already running"));
    }
    state.store.prepare_retry(id).await?;
    start_session(State(state), Path(id)).await
}

pub async fn cancel_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    if let Some(cancel) = state.running.lock().await.remove(&id) {
        cancel.cancel();
    }
    state.store.cancel_session(id).await?;
    let _ = state.event_tx.send(id);
    Ok(Json(state.store.get_session(id).await?))
}

pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    if let Some(cancel) = state.running.lock().await.remove(&id) {
        cancel.cancel();
    }
    state.store.delete_session(id).await?;
    let _ = state.event_tx.send(id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn pause_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    if let Some(cancel) = state.running.lock().await.remove(&id) {
        cancel.cancel();
    }
    state.store.pause_session(id).await?;
    let _ = state.event_tx.send(id);
    Ok(Json(state.store.get_session(id).await?))
}

pub async fn resume_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    state.store.resume_session(id).await?;
    let _ = state.event_tx.send(id);
    start_session(State(state), Path(id)).await
}

#[derive(Deserialize)]
pub struct ContextBody {
    pub context: String,
}

pub async fn update_context(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ContextBody>,
) -> Result<Json<SessionDetails>, ApiError> {
    state
        .store
        .update_session_context(id, &body.context)
        .await?;
    Ok(Json(state.store.get_session(id).await?))
}

pub async fn retry_seat(
    State(state): State<AppState>,
    Path((id, seat)): Path<(Uuid, String)>,
) -> Result<Json<SessionDetails>, ApiError> {
    let seat_kind: SeatKind = serde_json::from_value(serde_json::json!(seat))
        .map_err(|_| ApiError::bad_request(format!("invalid seat: {seat}")))?;
    state.store.retry_seat(id, seat_kind).await?;
    Ok(Json(state.store.get_session(id).await?))
}

pub async fn retry_phase(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    if state.running.lock().await.contains_key(&id) {
        return Err(ApiError::conflict("session is running, cancel first"));
    }
    let artifacts = state.store.retry_phase(id).await?;
    let details = state.store.get_session(id).await?;
    let Some(execution_token) = state.store.try_acquire_execution(id, 900).await? else {
        return Err(ApiError::conflict("session is already running"));
    };
    let mut running = state.running.lock().await;
    if running.contains_key(&id) {
        state.store.complete_execution(id, execution_token).await.map_err(ApiError::from)?;
        return Err(ApiError::conflict("session is already running"));
    }
    let cancel = CancellationFlag::default();
    running.insert(id, cancel.clone());
    drop(running);

    let store = state.store.clone();
    let running_map = state.running.clone();
    let event_tx = state.event_tx.clone();
    let session = details.session.clone();
    let mut runner = state.runner.clone();
    if session.search_enabled {
        if let Some(ref search) = state.search_backend {
            runner = runner.with_search(search.clone());
        }
    }
    tokio::spawn(async move {
        let progress = Arc::new(StoreProgressSink {
            session_id: id,
            store: store.clone(),
            event_tx: event_tx.clone(),
        });
        let result = runner
            .run_session_with_progress(session, Some(artifacts), cancel, Some(progress))
            .await;
        let _ = match result {
            Ok(result_artifacts) => {
                let active = store.is_execution_active(id, execution_token).await.unwrap_or(false);
                if !active { running_map.lock().await.remove(&id); return; }
                if let Some(final_session) = &result_artifacts.session {
                    if let Err(err) = store.update_session(final_session).await {
                        error!("failed to update session: {err}");
                        let _ = store.fail_session(id, Some(execution_token), &format!("failed to persist final session: {err}")).await;
                        let _ = event_tx.send(id);
                        running_map.lock().await.remove(&id);
                        return;
                    }
                    if let Err(err) = store.save_artifacts(id, &result_artifacts).await {
                        error!("failed to save artifacts: {err}");
                        let _ = store.fail_session(id, Some(execution_token), &format!("failed to persist artifacts: {err}")).await;
                        let _ = event_tx.send(id);
                        running_map.lock().await.remove(&id);
                        return;
                    }
                    if let Err(err) = store.complete_execution(id, execution_token).await {
                        error!("failed to complete session execution: {err}");
                    }
                }
            }
            Err(AgentError::Cancelled) => {
                running_map.lock().await.remove(&id);
            }
            Err(err) => {
                let _ = store.fail_session(id, Some(execution_token), &err.to_string()).await;
                let _ = event_tx.send(id);
                running_map.lock().await.remove(&id);
            }
        };
        let _ = event_tx.send(id);
    });
    Ok(Json(details))
}

pub async fn manual_revision(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    state.store.manual_revision_trigger(id).await?;
    let _ = state.event_tx.send(id);
    Ok(Json(state.store.get_session(id).await?))
}

pub async fn phase_trajectory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<wenyuan_store::SessionEvent>>, ApiError> {
    Ok(Json(state.store.phase_trajectory(id).await?))
}

pub async fn events_sse(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError> {
    let initial = state.store.events(id).await?;
    let initial_stream = tokio_stream::iter(initial.into_iter().map(|event| {
        Ok(Event::default().data(
            serde_json::json!({
                "type": event.event_type,
                "payload": event.payload,
                "created_at": event.created_at,
            })
            .to_string(),
        ))
    }));
    let store = state.store.clone();
    let mut rx = state.event_tx.subscribe();
    let live_stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(session_id) if session_id == id => {
                    let payload = match store.events(id).await {
                        Ok(events) => serde_json::json!({ "type": "refresh", "events": events }),
                        Err(err) => serde_json::json!({ "type": "error", "error": err.to_string() }),
                    };
                    yield Ok(Event::default().data(payload.to_string()));
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().data(serde_json::json!({ "type": "refresh" }).to_string()));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    let stream = initial_stream.chain(live_stream);
    Ok(Sse::new(stream))
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<wenyuan_store::StoreError> for ApiError {
    fn from(value: wenyuan_store::StoreError) -> Self {
        let status = match value {
            wenyuan_store::StoreError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: value.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.message
            })),
        )
            .into_response()
    }
}
