use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::{oneshot, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
pub const DEFAULT_MCP_PORT: u16 = 3847;

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub addr: SocketAddr,
    pub state_path: Option<PathBuf>,
}

impl McpServerConfig {
    pub fn from_env(state_path: PathBuf) -> Self {
        let port = std::env::var("WENYUAN_MCP_PORT")
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .unwrap_or(DEFAULT_MCP_PORT);
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            state_path: Some(state_path),
        }
    }
}

pub struct McpServerHandle {
    pub addr: SocketAddr,
    pub shutdown_tx: oneshot::Sender<()>,
}

#[derive(Clone)]
struct McpState {
    hub: MeetingHub,
}

pub async fn start_mcp_server(config: McpServerConfig) -> anyhow::Result<McpServerHandle> {
    let hub = MeetingHub::load(config.state_path).await;
    let state = McpState { hub };
    let app = Router::new()
        .route("/health", get(health))
        .route("/mcp", post(mcp))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
        if let Err(err) = result {
            warn!("MCP meeting server stopped with error: {err}");
        }
    });

    info!("Wenyuan MCP meeting server listening on http://{addr}/mcp");
    Ok(McpServerHandle { addr, shutdown_tx })
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "service": "wenyuan-mcp" }))
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

async fn mcp(State(state): State<McpState>, Json(request): Json<JsonRpcRequest>) -> Response {
    if request.id.is_none() && request.method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }

    let id = request.id.unwrap_or(Value::Null);
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "wenyuan-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => dispatch_tool_call(&state.hub, &request.params).await,
        _ => Err(McpError::MethodNotFound(request.method)),
    };

    match result {
        Ok(result) => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }))
        .into_response(),
        Err(err) => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": err.rpc_code(),
                "message": err.to_string()
            }
        }))
        .into_response(),
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "wenyuan_create_meeting",
            "description": "Create a 2-3 agent collaborative meeting around one user goal. No participant is assigned a fixed viewpoint.",
            "inputSchema": {
                "type": "object",
                "required": ["title", "goal", "participant_limit"],
                "properties": {
                    "title": { "type": "string" },
                    "goal": { "type": "string" },
                    "context": { "type": "string" },
                    "participant_limit": { "type": "integer", "minimum": 2, "maximum": 3 },
                    "max_rounds": { "type": "integer", "minimum": 1, "maximum": 2 },
                    "heartbeat_timeout_secs": { "type": "integer", "minimum": 10 },
                    "offline_grace_secs": { "type": "integer", "minimum": 10 }
                }
            }
        }),
        json!({
            "name": "wenyuan_join_meeting",
            "description": "Join or resume a meeting participant. Use resume_token to recover after disconnect without creating a duplicate participant.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id", "join_code", "display_name"],
                "properties": {
                    "meeting_id": { "type": "string" },
                    "join_code": { "type": "string" },
                    "display_name": { "type": "string" },
                    "client_name": { "type": "string" },
                    "resume_token": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "wenyuan_next_task",
            "description": "Get the participant's current task. Repeated calls are idempotent and return the same claimed turn until it is submitted.",
            "inputSchema": participant_auth_schema()
        }),
        json!({
            "name": "wenyuan_submit_turn",
            "description": "Submit one assigned meeting turn. Duplicate submission of the same turn is accepted idempotently.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id", "participant_id", "resume_token", "turn_id", "response"],
                "properties": {
                    "meeting_id": { "type": "string" },
                    "participant_id": { "type": "string" },
                    "resume_token": { "type": "string" },
                    "turn_id": { "type": "string" },
                    "response": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "wenyuan_ask_user",
            "description": "Request essential missing user information. Similar questions are merged; only one distinct user question is active at a time and later questions are queued.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id", "participant_id", "resume_token", "question", "reason"],
                "properties": {
                    "meeting_id": { "type": "string" },
                    "participant_id": { "type": "string" },
                    "resume_token": { "type": "string" },
                    "question": { "type": "string" },
                    "reason": { "type": "string" },
                    "missing_key": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "wenyuan_answer_user",
            "description": "Host-only: answer the currently active user question. Requires the meeting owner token returned by create_meeting.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id", "owner_token", "question_id", "answer"],
                "properties": {
                    "meeting_id": { "type": "string" },
                    "owner_token": { "type": "string" },
                    "question_id": { "type": "string" },
                    "answer": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "wenyuan_heartbeat",
            "description": "Refresh participant liveness. Agents waiting on peers should heartbeat periodically.",
            "inputSchema": participant_auth_schema()
        }),
        json!({
            "name": "wenyuan_meeting_status",
            "description": "Get meeting stage, participant liveness, active question, and progress without exposing participant secrets.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id"],
                "properties": { "meeting_id": { "type": "string" } }
            }
        }),
    ]
}

fn participant_auth_schema() -> Value {
    json!({
        "type": "object",
        "required": ["meeting_id", "participant_id", "resume_token"],
        "properties": {
            "meeting_id": { "type": "string" },
            "participant_id": { "type": "string" },
            "resume_token": { "type": "string" }
        }
    })
}

async fn dispatch_tool_call(hub: &MeetingHub, params: &Value) -> Result<Value, McpError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidParams("missing tool name".into()))?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let result: Result<Value, HubError> = match name {
        "wenyuan_create_meeting" => {
            let req: CreateMeetingRequest = from_args(args)?;
            hub.create_meeting(req).await
        }
        "wenyuan_join_meeting" => {
            let req: JoinMeetingRequest = from_args(args)?;
            hub.join_meeting(req).await
        }
        "wenyuan_next_task" => {
            let req: ParticipantAuth = from_args(args)?;
            hub.next_task(req).await
        }
        "wenyuan_submit_turn" => {
            let req: SubmitTurnRequest = from_args(args)?;
            hub.submit_turn(req).await
        }
        "wenyuan_ask_user" => {
            let req: AskUserRequest = from_args(args)?;
            hub.ask_user(req).await
        }
        "wenyuan_answer_user" => {
            let req: AnswerUserRequest = from_args(args)?;
            hub.answer_user(req).await
        }
        "wenyuan_heartbeat" => {
            let req: ParticipantAuth = from_args(args)?;
            hub.heartbeat(req).await
        }
        "wenyuan_meeting_status" => {
            let req: MeetingStatusRequest = from_args(args)?;
            hub.status(req.meeting_id).await
        }
        other => return Err(McpError::UnknownTool(other.to_string())),
    };

    match result {
        Ok(payload) => Ok(tool_result(payload, false)),
        Err(err) => Ok(tool_result(
            json!({ "error": err.to_string(), "kind": err.kind() }),
            true,
        )),
    }
}

fn from_args<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, HubError> {
    serde_json::from_value(value).map_err(|err| HubError::InvalidInput(err.to_string()))
}

fn tool_result(payload: Value, is_error: bool) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
        }],
        "structuredContent": payload,
        "isError": is_error
    })
}

#[derive(Debug, Error)]
enum McpError {
    #[error("method not found: {0}")]
    MethodNotFound(String),
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("invalid params: {0}")]
    InvalidParams(String),
}

impl McpError {
    fn rpc_code(&self) -> i32 {
        match self {
            Self::MethodNotFound(_) => -32601,
            Self::UnknownTool(_) => -32602,
            Self::InvalidParams(_) => -32602,
        }
    }
}

impl From<HubError> for McpError {
    fn from(value: HubError) -> Self {
        Self::InvalidParams(value.to_string())
    }
}

#[derive(Debug, Error)]
pub enum HubError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("meeting not found")]
    MeetingNotFound,
    #[error("participant not found")]
    ParticipantNotFound,
    #[error("invalid join code")]
    InvalidJoinCode,
    #[error("invalid participant token")]
    InvalidParticipantToken,
    #[error("invalid owner token")]
    InvalidOwnerToken,
    #[error("meeting already started")]
    MeetingStarted,
    #[error("meeting is full")]
    MeetingFull,
    #[error("meeting is paused: {0}")]
    MeetingPaused(String),
    #[error("turn not found")]
    TurnNotFound,
    #[error("turn is assigned to another participant")]
    WrongAssignee,
    #[error("question not found")]
    QuestionNotFound,
}

impl HubError {
    fn kind(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::MeetingNotFound => "meeting_not_found",
            Self::ParticipantNotFound => "participant_not_found",
            Self::InvalidJoinCode => "invalid_join_code",
            Self::InvalidParticipantToken => "invalid_participant_token",
            Self::InvalidOwnerToken => "invalid_owner_token",
            Self::MeetingStarted => "meeting_started",
            Self::MeetingFull => "meeting_full",
            Self::MeetingPaused(_) => "meeting_paused",
            Self::TurnNotFound => "turn_not_found",
            Self::WrongAssignee => "wrong_assignee",
            Self::QuestionNotFound => "question_not_found",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingConfig {
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub context: String,
    pub participant_limit: u8,
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u8,
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_secs: u64,
    #[serde(default = "default_offline_grace")]
    pub offline_grace_secs: u64,
}

fn default_max_rounds() -> u8 {
    2
}
fn default_heartbeat_timeout() -> u64 {
    45
}
fn default_offline_grace() -> u64 {
    60
}

impl MeetingConfig {
    fn validate(&self) -> Result<(), HubError> {
        if !(2..=3).contains(&self.participant_limit) {
            return Err(HubError::InvalidInput(
                "participant_limit must be 2 or 3".into(),
            ));
        }
        if !(1..=2).contains(&self.max_rounds) {
            return Err(HubError::InvalidInput("max_rounds must be 1 or 2".into()));
        }
        if self.title.trim().is_empty() || self.goal.trim().is_empty() {
            return Err(HubError::InvalidInput(
                "title and goal must not be empty".into(),
            ));
        }
        if self.heartbeat_timeout_secs < 10 || self.offline_grace_secs < 10 {
            return Err(HubError::InvalidInput(
                "heartbeat timeout and offline grace must be at least 10 seconds".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStage {
    Lobby,
    Initial,
    CrossReview,
    Synthesis,
    Completed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantState {
    Online,
    Stale,
    Left,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: Uuid,
    pub display_name: String,
    pub client_name: String,
    pub joined_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub state: ParticipantState,
    resume_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnStage {
    Initial,
    CrossReview,
    Synthesis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Pending,
    Claimed,
    Submitted,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingTurn {
    pub id: Uuid,
    pub stage: TurnStage,
    pub assignee: Uuid,
    pub status: TurnStatus,
    pub payload: Value,
    pub response: Option<String>,
    pub skip_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Open,
    Deferred,
    Answered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuestion {
    pub id: Uuid,
    pub question: String,
    pub reason: String,
    pub missing_key: String,
    pub asked_by: Vec<Uuid>,
    pub status: QuestionStatus,
    pub answer: Option<String>,
    pub created_at: DateTime<Utc>,
    pub answered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    pub id: Uuid,
    pub config: MeetingConfig,
    pub stage: MeetingStage,
    join_code: String,
    owner_token: String,
    pub participants: Vec<Participant>,
    pub turns: Vec<MeetingTurn>,
    pub questions: Vec<UserQuestion>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub paused_reason: Option<String>,
}

#[derive(Clone)]
pub struct MeetingHub {
    meetings: Arc<RwLock<HashMap<Uuid, Meeting>>>,
    state_path: Option<Arc<PathBuf>>,
}

impl MeetingHub {
    pub fn new_ephemeral() -> Self {
        Self {
            meetings: Arc::new(RwLock::new(HashMap::new())),
            state_path: None,
        }
    }

    pub async fn load(state_path: Option<PathBuf>) -> Self {
        let mut meetings = HashMap::new();
        if let Some(path) = state_path.as_ref() {
            if let Ok(bytes) = tokio::fs::read(path).await {
                match serde_json::from_slice::<Vec<Meeting>>(&bytes) {
                    Ok(restored) => {
                        meetings.extend(restored.into_iter().map(|meeting| (meeting.id, meeting)));
                    }
                    Err(err) => warn!("failed to restore MCP meetings: {err}"),
                }
            }
        }
        Self {
            meetings: Arc::new(RwLock::new(meetings)),
            state_path: state_path.map(Arc::new),
        }
    }

    async fn persist(&self) {
        let Some(path) = self.state_path.as_ref() else {
            return;
        };
        let snapshot: Vec<Meeting> = self.meetings.read().await.values().cloned().collect();
        let Ok(bytes) = serde_json::to_vec_pretty(&snapshot) else {
            return;
        };
        if let Some(parent) = path.parent() {
            if let Err(err) = tokio::fs::create_dir_all(parent).await {
                warn!("failed to create MCP state directory: {err}");
                return;
            }
        }
        if let Err(err) = tokio::fs::write(path.as_ref(), bytes).await {
            warn!("failed to persist MCP meetings: {err}");
        }
    }

    pub async fn create_meeting(&self, request: CreateMeetingRequest) -> Result<Value, HubError> {
        let config = MeetingConfig {
            title: request.title,
            goal: request.goal,
            context: request.context.unwrap_or_default(),
            participant_limit: request.participant_limit,
            max_rounds: request.max_rounds.unwrap_or_else(default_max_rounds),
            heartbeat_timeout_secs: request
                .heartbeat_timeout_secs
                .unwrap_or_else(default_heartbeat_timeout),
            offline_grace_secs: request
                .offline_grace_secs
                .unwrap_or_else(default_offline_grace),
        };
        config.validate()?;

        let id = Uuid::new_v4();
        let meeting = Meeting {
            id,
            config,
            stage: MeetingStage::Lobby,
            join_code: short_secret(),
            owner_token: Uuid::new_v4().to_string(),
            participants: Vec::new(),
            turns: Vec::new(),
            questions: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            paused_reason: None,
        };
        let join_code = meeting.join_code.clone();
        let owner_token = meeting.owner_token.clone();
        let participant_limit = meeting.config.participant_limit;
        self.meetings.write().await.insert(id, meeting);
        self.persist().await;

        Ok(json!({
            "meeting_id": id,
            "join_code": join_code,
            "owner_token": owner_token,
            "participant_limit": participant_limit,
            "stage": "lobby",
            "interaction_policy": {
                "initial": "all participants answer the same user goal independently",
                "cross_review": "react only to material disagreements or omissions; do not repeat agreed points",
                "synthesis": "one temporary synthesizer combines the best implementation and preserves unresolved dissent",
                "questions": "only essential missing information; similar questions are deduplicated and only one is active at a time",
                "offline": "3-agent meetings may degrade to 2 after grace; 2-agent meetings pause if one agent is lost"
            }
        }))
    }

    pub async fn join_meeting(&self, request: JoinMeetingRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        if request.join_code != meeting.join_code {
            return Err(HubError::InvalidJoinCode);
        }

        if let Some(token) = request.resume_token.as_deref() {
            if let Some(participant) = meeting
                .participants
                .iter_mut()
                .find(|participant| participant.resume_token == token)
            {
                participant.last_seen = Utc::now();
                participant.state = ParticipantState::Online;
                if !request.display_name.trim().is_empty() {
                    participant.display_name = request.display_name.trim().to_string();
                }
                meeting.updated_at = Utc::now();
                let payload = join_payload(meeting, participant);
                drop(meetings);
                self.persist().await;
                return Ok(payload);
            }
            return Err(HubError::InvalidParticipantToken);
        }

        if meeting.stage != MeetingStage::Lobby {
            return Err(HubError::MeetingStarted);
        }
        if meeting.participants.len() >= usize::from(meeting.config.participant_limit) {
            return Err(HubError::MeetingFull);
        }
        if request.display_name.trim().is_empty() {
            return Err(HubError::InvalidInput("display_name must not be empty".into()));
        }
        if meeting
            .participants
            .iter()
            .any(|p| p.display_name.eq_ignore_ascii_case(request.display_name.trim()))
        {
            return Err(HubError::InvalidInput(
                "display_name already exists; use resume_token to reconnect".into(),
            ));
        }

        let participant = Participant {
            id: Uuid::new_v4(),
            display_name: request.display_name.trim().to_string(),
            client_name: request.client_name.unwrap_or_default(),
            joined_at: Utc::now(),
            last_seen: Utc::now(),
            state: ParticipantState::Online,
            resume_token: Uuid::new_v4().to_string(),
        };
        meeting.participants.push(participant.clone());
        meeting.updated_at = Utc::now();
        if meeting.participants.len() == usize::from(meeting.config.participant_limit) {
            start_initial_round(meeting);
        }
        let payload = join_payload(meeting, &participant);
        drop(meetings);
        self.persist().await;
        Ok(payload)
    }

    pub async fn next_task(&self, auth: ParticipantAuth) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&auth.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&auth.participant_id, "participant_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &auth.resume_token)?;
        refresh_offline_and_progress(meeting);

        if let Some(question) = active_question(meeting) {
            let result = json!({
                "status": "waiting_for_user",
                "question": public_question(question),
                "stage": meeting.stage
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }
        if meeting.stage == MeetingStage::Paused {
            let reason = meeting
                .paused_reason
                .clone()
                .unwrap_or_else(|| "meeting paused".into());
            drop(meetings);
            self.persist().await;
            return Err(HubError::MeetingPaused(reason));
        }
        if meeting.stage == MeetingStage::Completed {
            let result = json!({
                "status": "completed",
                "stage": meeting.stage,
                "final": final_response(meeting)
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        if let Some(turn) = meeting.turns.iter_mut().find(|turn| {
            turn.assignee == participant_id
                && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed)
        }) {
            if turn.status == TurnStatus::Pending {
                turn.status = TurnStatus::Claimed;
                turn.updated_at = Utc::now();
            }
            let result = json!({
                "status": "task",
                "meeting_id": meeting.id,
                "stage": meeting.stage,
                "turn_id": turn.id,
                "turn_stage": turn.stage,
                "payload": turn.payload,
                "user_answers": answered_questions(meeting),
                "instruction": task_instruction(&turn.stage)
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        let result = json!({
            "status": "waiting_for_peers",
            "stage": meeting.stage,
            "progress": progress_summary(meeting)
        });
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn submit_turn(&self, request: SubmitTurnRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&request.participant_id, "participant_id")?;
        let turn_id = parse_uuid(&request.turn_id, "turn_id")?;
        if request.response.trim().is_empty() {
            return Err(HubError::InvalidInput("response must not be empty".into()));
        }

        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &request.resume_token)?;
        let turn = meeting
            .turns
            .iter_mut()
            .find(|turn| turn.id == turn_id)
            .ok_or(HubError::TurnNotFound)?;
        if turn.assignee != participant_id {
            return Err(HubError::WrongAssignee);
        }
        if turn.status == TurnStatus::Submitted {
            let result = json!({
                "ok": true,
                "idempotent": true,
                "turn_id": turn.id,
                "stage": meeting.stage
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }
        turn.response = Some(request.response.trim().to_string());
        turn.status = TurnStatus::Submitted;
        turn.updated_at = Utc::now();
        meeting.updated_at = Utc::now();
        refresh_offline_and_progress(meeting);
        let result = json!({
            "ok": true,
            "idempotent": false,
            "turn_id": turn_id,
            "stage": meeting.stage,
            "progress": progress_summary(meeting)
        });
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn ask_user(&self, request: AskUserRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&request.participant_id, "participant_id")?;
        if request.question.trim().is_empty() || request.reason.trim().is_empty() {
            return Err(HubError::InvalidInput(
                "question and reason must not be empty".into(),
            ));
        }
        let normalized = normalize_question(&request.question);
        let missing_key = request.missing_key.unwrap_or_default().trim().to_string();

        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &request.resume_token)?;

        if let Some(existing) = meeting.questions.iter_mut().find(|question| {
            matches!(question.status, QuestionStatus::Open | QuestionStatus::Deferred)
                && questions_match(question, &normalized, &missing_key)
        }) {
            if !existing.asked_by.contains(&participant_id) {
                existing.asked_by.push(participant_id);
            }
            let result = json!({
                "deduplicated": true,
                "queued": existing.status == QuestionStatus::Deferred,
                "question": public_question(existing),
                "active_question": active_question(meeting).map(public_question)
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        let has_open = meeting
            .questions
            .iter()
            .any(|question| question.status == QuestionStatus::Open);
        let question = UserQuestion {
            id: Uuid::new_v4(),
            question: request.question.trim().to_string(),
            reason: request.reason.trim().to_string(),
            missing_key,
            asked_by: vec![participant_id],
            status: if has_open {
                QuestionStatus::Deferred
            } else {
                QuestionStatus::Open
            },
            answer: None,
            created_at: Utc::now(),
            answered_at: None,
        };
        let queued = question.status == QuestionStatus::Deferred;
        let question_public = public_question(&question);
        meeting.questions.push(question);
        meeting.updated_at = Utc::now();
        let active = active_question(meeting).map(public_question);
        drop(meetings);
        self.persist().await;
        Ok(json!({
            "deduplicated": false,
            "queued": queued,
            "question": question_public,
            "active_question": active
        }))
    }

    pub async fn answer_user(&self, request: AnswerUserRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let question_id = parse_uuid(&request.question_id, "question_id")?;
        if request.answer.trim().is_empty() {
            return Err(HubError::InvalidInput("answer must not be empty".into()));
        }
        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        if meeting.owner_token != request.owner_token {
            return Err(HubError::InvalidOwnerToken);
        }
        let question = meeting
            .questions
            .iter_mut()
            .find(|question| question.id == question_id)
            .ok_or(HubError::QuestionNotFound)?;
        question.answer = Some(request.answer.trim().to_string());
        question.status = QuestionStatus::Answered;
        question.answered_at = Some(Utc::now());
        if let Some(next) = meeting
            .questions
            .iter_mut()
            .find(|question| question.status == QuestionStatus::Deferred)
        {
            next.status = QuestionStatus::Open;
        }
        meeting.updated_at = Utc::now();
        refresh_offline_and_progress(meeting);
        let next_active = active_question(meeting).map(public_question);
        drop(meetings);
        self.persist().await;
        Ok(json!({ "ok": true, "next_active_question": next_active }))
    }

    pub async fn heartbeat(&self, auth: ParticipantAuth) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&auth.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&auth.participant_id, "participant_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &auth.resume_token)?;
        refresh_offline_and_progress(meeting);
        let result = json!({
            "ok": true,
            "stage": meeting.stage,
            "progress": progress_summary(meeting)
        });
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn status(&self, meeting_id: String) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&meeting_id, "meeting_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings
            .get_mut(&meeting_id)
            .ok_or(HubError::MeetingNotFound)?;
        refresh_offline_and_progress(meeting);
        let result = meeting_snapshot(meeting);
        drop(meetings);
        self.persist().await;
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMeetingRequest {
    pub title: String,
    pub goal: String,
    pub context: Option<String>,
    pub participant_limit: u8,
    pub max_rounds: Option<u8>,
    pub heartbeat_timeout_secs: Option<u64>,
    pub offline_grace_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct JoinMeetingRequest {
    pub meeting_id: String,
    pub join_code: String,
    pub display_name: String,
    pub client_name: Option<String>,
    pub resume_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ParticipantAuth {
    pub meeting_id: String,
    pub participant_id: String,
    pub resume_token: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitTurnRequest {
    pub meeting_id: String,
    pub participant_id: String,
    pub resume_token: String,
    pub turn_id: String,
    pub response: String,
}

#[derive(Debug, Deserialize)]
pub struct AskUserRequest {
    pub meeting_id: String,
    pub participant_id: String,
    pub resume_token: String,
    pub question: String,
    pub reason: String,
    pub missing_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnswerUserRequest {
    pub meeting_id: String,
    pub owner_token: String,
    pub question_id: String,
    pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct MeetingStatusRequest {
    pub meeting_id: String,
}

fn short_secret() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>()
        .to_uppercase()
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, HubError> {
    Uuid::parse_str(value)
        .map_err(|_| HubError::InvalidInput(format!("{field} must be a UUID")))
}

fn join_payload(meeting: &Meeting, participant: &Participant) -> Value {
    json!({
        "meeting_id": meeting.id,
        "participant_id": participant.id,
        "resume_token": participant.resume_token,
        "display_name": participant.display_name,
        "stage": meeting.stage,
        "joined": meeting.participants.len(),
        "participant_limit": meeting.config.participant_limit,
        "heartbeat_interval_secs": (meeting.config.heartbeat_timeout_secs / 2).max(5),
        "instruction": "Keep participant_id and resume_token. Call next_task; while waiting, send heartbeat. Do not invent a role or viewpoint: optimize for the user's stated goal."
    })
}

fn start_initial_round(meeting: &mut Meeting) {
    meeting.stage = MeetingStage::Initial;
    meeting.paused_reason = None;
    let now = Utc::now();
    for participant in &meeting.participants {
        meeting.turns.push(MeetingTurn {
            id: Uuid::new_v4(),
            stage: TurnStage::Initial,
            assignee: participant.id,
            status: TurnStatus::Pending,
            payload: json!({
                "title": meeting.config.title,
                "goal": meeting.config.goal,
                "context": meeting.config.context,
                "peer_outputs": [],
                "rules": [
                    "Solve the user's actual goal; do not adopt a preassigned viewpoint.",
                    "Give your best implementation/recommendation independently.",
                    "State assumptions and only essential missing information.",
                    "Do not ask the user a question if a reasonable answer can be given with explicit assumptions."
                ]
            }),
            response: None,
            skip_reason: None,
            created_at: now,
            updated_at: now,
        });
    }
    meeting.updated_at = now;
}

fn refresh_offline_and_progress(meeting: &mut Meeting) {
    let now = Utc::now();
    let timeout = ChronoDuration::seconds(meeting.config.heartbeat_timeout_secs as i64);
    let grace = ChronoDuration::seconds(meeting.config.offline_grace_secs as i64);

    for participant in &mut meeting.participants {
        if participant.state != ParticipantState::Left
            && now.signed_duration_since(participant.last_seen) > timeout
        {
            participant.state = ParticipantState::Stale;
        }
    }

    if matches!(meeting.stage, MeetingStage::Completed | MeetingStage::Lobby) {
        return;
    }

    let healthy: HashSet<Uuid> = meeting
        .participants
        .iter()
        .filter(|participant| {
            participant.state != ParticipantState::Left
                && now.signed_duration_since(participant.last_seen) <= timeout + grace
        })
        .map(|participant| participant.id)
        .collect();

    let current_turn_stage = match meeting.stage {
        MeetingStage::Initial => Some(TurnStage::Initial),
        MeetingStage::CrossReview => Some(TurnStage::CrossReview),
        MeetingStage::Synthesis => Some(TurnStage::Synthesis),
        _ => None,
    };

    if let Some(current_turn_stage) = current_turn_stage {
        let stale_assignees: Vec<Uuid> = meeting
            .turns
            .iter()
            .filter(|turn| {
                turn.stage == current_turn_stage
                    && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed)
                    && !healthy.contains(&turn.assignee)
            })
            .map(|turn| turn.assignee)
            .collect();

        if !stale_assignees.is_empty() {
            if meeting.config.participant_limit == 2 && healthy.len() < 2 {
                meeting.stage = MeetingStage::Paused;
                meeting.paused_reason = Some(
                    "2-agent meeting lost one participant; waiting for reconnection instead of silently becoming a single-agent answer"
                        .into(),
                );
                meeting.updated_at = now;
                return;
            }

            if healthy.len() >= 2 {
                if current_turn_stage == TurnStage::Synthesis {
                    if let Some(turn) = meeting.turns.iter_mut().find(|turn| {
                        turn.stage == TurnStage::Synthesis
                            && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed)
                    }) {
                        if let Some(replacement) = choose_synthesizer(meeting, Some(turn.assignee)) {
                            turn.assignee = replacement;
                            turn.status = TurnStatus::Pending;
                            turn.updated_at = now;
                            turn.skip_reason = Some("synthesizer_reassigned_after_disconnect".into());
                        }
                    }
                } else {
                    for turn in &mut meeting.turns {
                        if turn.stage == current_turn_stage
                            && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed)
                            && stale_assignees.contains(&turn.assignee)
                        {
                            turn.status = TurnStatus::Skipped;
                            turn.skip_reason = Some("participant_offline_after_grace".into());
                            turn.updated_at = now;
                        }
                    }
                }
            }
        }
    }

    if meeting.stage == MeetingStage::Paused {
        return;
    }

    match meeting.stage {
        MeetingStage::Initial if stage_is_terminal(meeting, TurnStage::Initial) => {
            let submitted = submitted_count(meeting, TurnStage::Initial);
            if submitted < 2 {
                meeting.stage = MeetingStage::Paused;
                meeting.paused_reason = Some("fewer than two initial opinions are available".into());
            } else if meeting.config.max_rounds >= 2 {
                create_cross_review_round(meeting);
            } else {
                create_synthesis_round(meeting);
            }
        }
        MeetingStage::CrossReview if stage_is_terminal(meeting, TurnStage::CrossReview) => {
            create_synthesis_round(meeting);
        }
        MeetingStage::Synthesis if stage_is_terminal(meeting, TurnStage::Synthesis) => {
            meeting.stage = MeetingStage::Completed;
            meeting.paused_reason = None;
            meeting.updated_at = now;
        }
        _ => {}
    }
}

fn create_cross_review_round(meeting: &mut Meeting) {
    meeting.stage = MeetingStage::CrossReview;
    let now = Utc::now();
    let initial_outputs = submitted_outputs(meeting, TurnStage::Initial);
    let eligible: Vec<Uuid> = meeting
        .participants
        .iter()
        .filter(|participant| participant.state == ParticipantState::Online)
        .map(|participant| participant.id)
        .collect();
    for participant_id in eligible {
        let peer_outputs: Vec<Value> = initial_outputs
            .iter()
            .filter(|(author, _)| *author != participant_id)
            .map(|(author, response)| json!({ "participant_id": author, "response": response }))
            .collect();
        meeting.turns.push(MeetingTurn {
            id: Uuid::new_v4(),
            stage: TurnStage::CrossReview,
            assignee: participant_id,
            status: TurnStatus::Pending,
            payload: json!({
                "title": meeting.config.title,
                "goal": meeting.config.goal,
                "context": meeting.config.context,
                "peer_outputs": peer_outputs,
                "rules": [
                    "Do not restate points everyone already agrees on.",
                    "React only to material disagreement, missing constraints, or a clearly better implementation.",
                    "You may explicitly agree when another answer is better; disagreement is not required.",
                    "If user input is truly required, call wenyuan_ask_user once with the smallest decisive question."
                ]
            }),
            response: None,
            skip_reason: None,
            created_at: now,
            updated_at: now,
        });
    }
    meeting.updated_at = now;
}

fn create_synthesis_round(meeting: &mut Meeting) {
    let Some(assignee) = choose_synthesizer(meeting, None) else {
        meeting.stage = MeetingStage::Paused;
        meeting.paused_reason = Some("no online participant is available for synthesis".into());
        return;
    };
    meeting.stage = MeetingStage::Synthesis;
    meeting.paused_reason = None;
    let now = Utc::now();
    let contributions: Vec<Value> = meeting
        .turns
        .iter()
        .filter(|turn| turn.status == TurnStatus::Submitted)
        .filter_map(|turn| {
            turn.response.as_ref().map(|response| {
                json!({
                    "participant_id": turn.assignee,
                    "stage": turn.stage,
                    "response": response
                })
            })
        })
        .collect();
    meeting.turns.push(MeetingTurn {
        id: Uuid::new_v4(),
        stage: TurnStage::Synthesis,
        assignee,
        status: TurnStatus::Pending,
        payload: json!({
            "title": meeting.config.title,
            "goal": meeting.config.goal,
            "context": meeting.config.context,
            "contributions": contributions,
            "user_answers": answered_questions(meeting),
            "rules": [
                "Synthesize the best answer for the user's goal; you are a temporary editor, not a privileged viewpoint.",
                "Prefer concrete implementation over averaging incompatible answers.",
                "Preserve meaningful dissent and uncertainty instead of fabricating consensus.",
                "Do not re-ask already answered questions.",
                "Return a concise final recommendation, implementation steps, trade-offs, and unresolved items."
            ]
        }),
        response: None,
        skip_reason: None,
        created_at: now,
        updated_at: now,
    });
    meeting.updated_at = now;
}

fn choose_synthesizer(meeting: &Meeting, exclude: Option<Uuid>) -> Option<Uuid> {
    meeting
        .participants
        .iter()
        .filter(|participant| {
            participant.state == ParticipantState::Online && Some(participant.id) != exclude
        })
        .min_by_key(|participant| {
            meeting
                .turns
                .iter()
                .filter(|turn| {
                    turn.assignee == participant.id && turn.status == TurnStatus::Submitted
                })
                .count()
        })
        .map(|participant| participant.id)
}

fn stage_is_terminal(meeting: &Meeting, stage: TurnStage) -> bool {
    let turns: Vec<&MeetingTurn> = meeting.turns.iter().filter(|turn| turn.stage == stage).collect();
    !turns.is_empty()
        && turns.iter().all(|turn| {
            matches!(turn.status, TurnStatus::Submitted | TurnStatus::Skipped)
        })
}

fn submitted_count(meeting: &Meeting, stage: TurnStage) -> usize {
    meeting
        .turns
        .iter()
        .filter(|turn| turn.stage == stage && turn.status == TurnStatus::Submitted)
        .count()
}

fn submitted_outputs(meeting: &Meeting, stage: TurnStage) -> Vec<(Uuid, String)> {
    meeting
        .turns
        .iter()
        .filter(|turn| turn.stage == stage && turn.status == TurnStatus::Submitted)
        .filter_map(|turn| turn.response.clone().map(|response| (turn.assignee, response)))
        .collect()
}

fn touch_participant(
    meeting: &mut Meeting,
    participant_id: Uuid,
    resume_token: &str,
) -> Result<(), HubError> {
    let participant = meeting
        .participants
        .iter_mut()
        .find(|participant| participant.id == participant_id)
        .ok_or(HubError::ParticipantNotFound)?;
    if participant.resume_token != resume_token {
        return Err(HubError::InvalidParticipantToken);
    }
    participant.last_seen = Utc::now();
    participant.state = ParticipantState::Online;
    meeting.updated_at = Utc::now();
    if meeting.stage == MeetingStage::Paused {
        let online = meeting
            .participants
            .iter()
            .filter(|participant| participant.state == ParticipantState::Online)
            .count();
        if online >= 2 {
            meeting.paused_reason = None;
            meeting.stage = infer_stage_from_turns(meeting);
        }
    }
    Ok(())
}

fn infer_stage_from_turns(meeting: &Meeting) -> MeetingStage {
    if meeting
        .turns
        .iter()
        .any(|turn| turn.stage == TurnStage::Synthesis && turn.status != TurnStatus::Submitted)
    {
        MeetingStage::Synthesis
    } else if meeting
        .turns
        .iter()
        .any(|turn| turn.stage == TurnStage::CrossReview && !matches!(turn.status, TurnStatus::Submitted | TurnStatus::Skipped))
    {
        MeetingStage::CrossReview
    } else if meeting
        .turns
        .iter()
        .any(|turn| turn.stage == TurnStage::Initial && !matches!(turn.status, TurnStatus::Submitted | TurnStatus::Skipped))
    {
        MeetingStage::Initial
    } else if meeting
        .turns
        .iter()
        .any(|turn| turn.stage == TurnStage::Synthesis && turn.status == TurnStatus::Submitted)
    {
        MeetingStage::Completed
    } else {
        MeetingStage::Initial
    }
}

fn normalize_question(input: &str) -> String {
    input
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn questions_match(existing: &UserQuestion, normalized: &str, missing_key: &str) -> bool {
    if !missing_key.is_empty() && existing.missing_key == missing_key {
        return true;
    }
    let other = normalize_question(&existing.question);
    if other == normalized {
        return true;
    }
    let min_len = other.chars().count().min(normalized.chars().count());
    min_len >= 8 && (other.contains(normalized) || normalized.contains(&other))
}

fn active_question(meeting: &Meeting) -> Option<&UserQuestion> {
    meeting
        .questions
        .iter()
        .find(|question| question.status == QuestionStatus::Open)
}

fn public_question(question: &UserQuestion) -> Value {
    json!({
        "id": question.id,
        "question": question.question,
        "reason": question.reason,
        "missing_key": question.missing_key,
        "asked_by_count": question.asked_by.len(),
        "status": question.status
    })
}

fn answered_questions(meeting: &Meeting) -> Vec<Value> {
    meeting
        .questions
        .iter()
        .filter(|question| question.status == QuestionStatus::Answered)
        .filter_map(|question| {
            question.answer.as_ref().map(|answer| {
                json!({
                    "question": question.question,
                    "answer": answer,
                    "missing_key": question.missing_key
                })
            })
        })
        .collect()
}

fn task_instruction(stage: &TurnStage) -> &'static str {
    match stage {
        TurnStage::Initial => {
            "Give your own best answer to the same user goal. Do not role-play a fixed perspective."
        }
        TurnStage::CrossReview => {
            "Review only substantive disagreement or omissions. Do not repeat consensus for the sake of interaction."
        }
        TurnStage::Synthesis => {
            "Act as temporary synthesizer: choose the strongest implementation, preserve real dissent, and avoid fake consensus."
        }
    }
}

fn final_response(meeting: &Meeting) -> Option<Value> {
    meeting
        .turns
        .iter()
        .rev()
        .find(|turn| turn.stage == TurnStage::Synthesis && turn.status == TurnStatus::Submitted)
        .and_then(|turn| {
            turn.response.as_ref().map(|response| {
                json!({
                    "response": response,
                    "synthesizer": turn.assignee,
                    "degraded": meeting
                        .turns
                        .iter()
                        .any(|turn| turn.status == TurnStatus::Skipped),
                    "skipped_turns": meeting
                        .turns
                        .iter()
                        .filter(|turn| turn.status == TurnStatus::Skipped)
                        .count()
                })
            })
        })
}

fn progress_summary(meeting: &Meeting) -> Value {
    let mut by_stage = HashMap::new();
    for stage in [TurnStage::Initial, TurnStage::CrossReview, TurnStage::Synthesis] {
        let total = meeting.turns.iter().filter(|turn| turn.stage == stage).count();
        let submitted = meeting
            .turns
            .iter()
            .filter(|turn| turn.stage == stage && turn.status == TurnStatus::Submitted)
            .count();
        let skipped = meeting
            .turns
            .iter()
            .filter(|turn| turn.stage == stage && turn.status == TurnStatus::Skipped)
            .count();
        by_stage.insert(
            format!("{stage:?}").to_lowercase(),
            json!({ "total": total, "submitted": submitted, "skipped": skipped }),
        );
    }
    json!({
        "turns": by_stage,
        "participants_online": meeting
            .participants
            .iter()
            .filter(|participant| participant.state == ParticipantState::Online)
            .count(),
        "questions_open": meeting
            .questions
            .iter()
            .filter(|question| question.status == QuestionStatus::Open)
            .count(),
        "questions_deferred": meeting
            .questions
            .iter()
            .filter(|question| question.status == QuestionStatus::Deferred)
            .count()
    })
}

fn meeting_snapshot(meeting: &Meeting) -> Value {
    json!({
        "meeting_id": meeting.id,
        "title": meeting.config.title,
        "goal": meeting.config.goal,
        "stage": meeting.stage,
        "participant_limit": meeting.config.participant_limit,
        "participants": meeting.participants.iter().map(|participant| json!({
            "id": participant.id,
            "display_name": participant.display_name,
            "client_name": participant.client_name,
            "state": participant.state,
            "last_seen": participant.last_seen
        })).collect::<Vec<_>>(),
        "active_question": active_question(meeting).map(public_question),
        "progress": progress_summary(meeting),
        "paused_reason": meeting.paused_reason,
        "final": final_response(meeting)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_two(hub: &MeetingHub) -> (Value, Value, Value) {
        let created = hub
            .create_meeting(CreateMeetingRequest {
                title: "test".into(),
                goal: "find the best implementation".into(),
                context: None,
                participant_limit: 2,
                max_rounds: Some(2),
                heartbeat_timeout_secs: Some(10),
                offline_grace_secs: Some(10),
            })
            .await
            .unwrap();
        let meeting_id = created["meeting_id"].as_str().unwrap().to_string();
        let join_code = created["join_code"].as_str().unwrap().to_string();
        let a = hub
            .join_meeting(JoinMeetingRequest {
                meeting_id: meeting_id.clone(),
                join_code: join_code.clone(),
                display_name: "A".into(),
                client_name: Some("client-a".into()),
                resume_token: None,
            })
            .await
            .unwrap();
        let b = hub
            .join_meeting(JoinMeetingRequest {
                meeting_id,
                join_code,
                display_name: "B".into(),
                client_name: Some("client-b".into()),
                resume_token: None,
            })
            .await
            .unwrap();
        (created, a, b)
    }

    fn auth(join: &Value) -> ParticipantAuth {
        ParticipantAuth {
            meeting_id: join["meeting_id"].as_str().unwrap().to_string(),
            participant_id: join["participant_id"].as_str().unwrap().to_string(),
            resume_token: join["resume_token"].as_str().unwrap().to_string(),
        }
    }

    #[tokio::test]
    async fn two_participants_start_same_goal_without_fixed_roles() {
        let hub = MeetingHub::new_ephemeral();
        let (_created, a, b) = create_two(&hub).await;
        let task_a = hub.next_task(auth(&a)).await.unwrap();
        let task_b = hub.next_task(auth(&b)).await.unwrap();
        assert_eq!(task_a["status"], "task");
        assert_eq!(task_b["status"], "task");
        assert_eq!(task_a["payload"]["goal"], task_b["payload"]["goal"]);
    }

    #[tokio::test]
    async fn repeated_next_task_is_idempotent() {
        let hub = MeetingHub::new_ephemeral();
        let (_created, a, _b) = create_two(&hub).await;
        let first = hub.next_task(auth(&a)).await.unwrap();
        let second = hub.next_task(auth(&a)).await.unwrap();
        assert_eq!(first["turn_id"], second["turn_id"]);
    }

    #[tokio::test]
    async fn duplicate_user_questions_are_merged() {
        let hub = MeetingHub::new_ephemeral();
        let (_created, a, b) = create_two(&hub).await;
        let req_a = AskUserRequest {
            meeting_id: a["meeting_id"].as_str().unwrap().into(),
            participant_id: a["participant_id"].as_str().unwrap().into(),
            resume_token: a["resume_token"].as_str().unwrap().into(),
            question: "用户的预算上限是多少？".into(),
            reason: "会改变实现方案".into(),
            missing_key: Some("budget".into()),
        };
        let req_b = AskUserRequest {
            meeting_id: b["meeting_id"].as_str().unwrap().into(),
            participant_id: b["participant_id"].as_str().unwrap().into(),
            resume_token: b["resume_token"].as_str().unwrap().into(),
            question: "预算上限是多少".into(),
            reason: "影响推荐".into(),
            missing_key: Some("budget".into()),
        };
        let first = hub.ask_user(req_a).await.unwrap();
        let second = hub.ask_user(req_b).await.unwrap();
        assert_eq!(first["deduplicated"], false);
        assert_eq!(second["deduplicated"], true);
        assert_eq!(second["question"]["asked_by_count"], 2);
    }

    #[tokio::test]
    async fn two_agent_meeting_pauses_when_one_is_lost() {
        let hub = MeetingHub::new_ephemeral();
        let (_created, a, b) = create_two(&hub).await;
        let meeting_id = Uuid::parse_str(a["meeting_id"].as_str().unwrap()).unwrap();
        let b_id = Uuid::parse_str(b["participant_id"].as_str().unwrap()).unwrap();
        {
            let mut meetings = hub.meetings.write().await;
            let meeting = meetings.get_mut(&meeting_id).unwrap();
            let participant = meeting
                .participants
                .iter_mut()
                .find(|participant| participant.id == b_id)
                .unwrap();
            participant.last_seen = Utc::now() - ChronoDuration::seconds(25);
            refresh_offline_and_progress(meeting);
            assert_eq!(meeting.stage, MeetingStage::Paused);
        }
    }

    #[tokio::test]
    async fn three_agent_meeting_can_degrade_to_two() {
        let hub = MeetingHub::new_ephemeral();
        let created = hub
            .create_meeting(CreateMeetingRequest {
                title: "test".into(),
                goal: "best implementation".into(),
                context: None,
                participant_limit: 3,
                max_rounds: Some(1),
                heartbeat_timeout_secs: Some(10),
                offline_grace_secs: Some(10),
            })
            .await
            .unwrap();
        let meeting_id = created["meeting_id"].as_str().unwrap().to_string();
        let join_code = created["join_code"].as_str().unwrap().to_string();
        let mut joined = Vec::new();
        for name in ["A", "B", "C"] {
            joined.push(
                hub.join_meeting(JoinMeetingRequest {
                    meeting_id: meeting_id.clone(),
                    join_code: join_code.clone(),
                    display_name: name.into(),
                    client_name: None,
                    resume_token: None,
                })
                .await
                .unwrap(),
            );
        }
        let meeting_uuid = Uuid::parse_str(&meeting_id).unwrap();
        let c_id = Uuid::parse_str(joined[2]["participant_id"].as_str().unwrap()).unwrap();
        {
            let mut meetings = hub.meetings.write().await;
            let meeting = meetings.get_mut(&meeting_uuid).unwrap();
            let participant = meeting
                .participants
                .iter_mut()
                .find(|participant| participant.id == c_id)
                .unwrap();
            participant.last_seen = Utc::now() - ChronoDuration::seconds(25);
            refresh_offline_and_progress(meeting);
            assert_ne!(meeting.stage, MeetingStage::Paused);
            assert!(meeting.turns.iter().any(|turn| {
                turn.assignee == c_id && turn.status == TurnStatus::Skipped
            }));
        }
    }
}
