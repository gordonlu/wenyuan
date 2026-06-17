use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::{error, info};
use uuid::Uuid;
use wenyuan_agent::{AgentError, AgentRunner, CancellationFlag};
use wenyuan_core::{Session, SessionPhase};
use wenyuan_provider::{
    LlmProvider, MockProvider, MockScenario, OpenAiCompatibleConfig, OpenAiCompatibleProvider,
};
use wenyuan_store::{SessionDetails, Store};

#[derive(Clone)]
struct AppState {
    store: Store,
    runner: AgentRunner,
    running: Arc<Mutex<HashMap<Uuid, CancellationFlag>>>,
    config: ConfigStatus,
}

#[derive(Debug, Clone, Serialize)]
struct ConfigStatus {
    provider_configured: bool,
    provider_kind: String,
    model: String,
    database_url: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    title: String,
    topic: String,
    context: Option<String>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wenyuan_server=info,tower_http=info".into()),
        )
        .init();

    let database_url =
        env::var("WENYUAN_DATABASE_URL").unwrap_or_else(|_| "sqlite://wenyuan.db".into());
    let bind = env::var("WENYUAN_BIND").unwrap_or_else(|_| "127.0.0.1:3210".into());
    let store = Store::connect(&database_url).await?;
    let recovered = store.recover_stale_executions().await?;
    if recovered > 0 {
        info!("marked {recovered} stale session execution(s) as retry_required");
    }
    let (provider, config) = provider_from_env(&database_url);
    let state = AppState {
        store,
        runner: AgentRunner::new(provider),
        running: Arc::new(Mutex::new(HashMap::new())),
        config,
    };

    let app = app(state);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let addr: SocketAddr = listener.local_addr()?;
    info!("Wenyuan server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/config/status", get(config_status))
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/{id}", get(get_session))
        .route("/api/sessions/{id}/start", post(start_session))
        .route("/api/sessions/{id}/retry", post(retry_session))
        .route("/api/sessions/{id}/cancel", post(cancel_session))
        .route("/api/sessions/{id}/events", get(events_sse))
        .fallback_service(ServeDir::new("web/dist").fallback(ServeDir::new("web/dist")))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn provider_from_env(database_url: &str) -> (Arc<dyn LlmProvider>, ConfigStatus) {
    let base_url = env::var("WENYUAN_LLM_BASE_URL").unwrap_or_default();
    let api_key = env::var("WENYUAN_LLM_API_KEY").unwrap_or_default();
    let model = env::var("WENYUAN_LLM_MODEL").unwrap_or_else(|_| "mock-model".into());
    let provider_kind = if !base_url.is_empty() && !api_key.is_empty() {
        "openai-compatible".to_string()
    } else {
        "mock".to_string()
    };
    let provider: Arc<dyn LlmProvider> = if provider_kind == "openai-compatible" {
        Arc::new(OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
            base_url,
            api_key,
            model: model.clone(),
        }))
    } else {
        Arc::new(MockProvider::new(mock_scenario_from_env()))
    };
    (
        provider,
        ConfigStatus {
            provider_configured: provider_kind != "openai-compatible"
                || env::var("WENYUAN_LLM_API_KEY").is_ok(),
            provider_kind,
            model,
            database_url: database_url.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    )
}

fn mock_scenario_from_env() -> MockScenario {
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

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn config_status(State(state): State<AppState>) -> Json<ConfigStatus> {
    Json(state.config)
}

async fn create_session(
    State(state): State<AppState>,
    Json(input): Json<CreateSessionRequest>,
) -> Result<Json<Session>, ApiError> {
    let session = Session::new(input.title, input.topic, input.context.unwrap_or_default());
    state.store.create_session(&session).await?;
    Ok(Json(session))
}

async fn list_sessions(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.store.list_sessions().await?))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    Ok(Json(state.store.get_session(id).await?))
}

async fn start_session(
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
    let runner = state.runner.clone();
    let running_map = state.running.clone();
    let session = details.session;
    tokio::spawn(async move {
        let result = runner.run_session(session, cancel).await;
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
                    }
                    if let Err(err) = store.save_artifacts(id, &artifacts).await {
                        error!("failed to save artifacts: {err}");
                    }
                    for event in &artifacts.events {
                        let _ = store
                            .append_event(id, event, serde_json::json!({ "source": "runner" }))
                            .await;
                    }
                    if let Err(err) = store.complete_execution(id, execution_token).await {
                        error!("failed to complete session execution: {err}");
                    }
                }
            }
            Err(err) => {
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
        running_map.lock().await.remove(&id);
    });

    Ok(Json(state.store.get_session(id).await?))
}

async fn retry_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    if state.running.lock().await.contains_key(&id) {
        return Err(ApiError::conflict("session is already running"));
    }
    state.store.prepare_retry(id).await?;
    start_session(State(state), Path(id)).await
}

async fn cancel_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionDetails>, ApiError> {
    if let Some(cancel) = state.running.lock().await.remove(&id) {
        cancel.cancel();
    }
    state.store.cancel_session(id).await?;
    Ok(Json(state.store.get_session(id).await?))
}

async fn events_sse(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError> {
    let events = state.store.events(id).await?;
    let stream = tokio_stream::iter(events.into_iter().map(|event| {
        Ok(Event::default()
            .event(event.event_type)
            .data(event.payload.to_string()))
    }));
    Ok(Sse::new(stream))
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
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
