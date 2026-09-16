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
    let app = Router::new()
        .route("/health", get(health))
        .route("/mcp", post(mcp))
        .with_state(McpState { hub });

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
        {
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
            "serverInfo": { "name": "wenyuan-mcp", "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => dispatch_tool_call(&state.hub, &request.params).await,
        method => Err(McpError::MethodNotFound(method.to_string())),
    };

    match result {
        Ok(result) => Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response(),
        Err(err) => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": err.rpc_code(), "message": err.to_string() }
        }))
        .into_response(),
    }
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

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "wenyuan_create_meeting",
            "description": "Create a shared 2-3 agent meeting around one user goal. Participants are peers; no fixed viewpoint is assigned.",
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
            "name": "wenyuan_start_meeting",
            "description": "Host-only: start a 3-seat room early with the 2 agents already joined. At least two participants are required.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id", "owner_token"],
                "properties": {
                    "meeting_id": { "type": "string" },
                    "owner_token": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "wenyuan_join_meeting",
            "description": "Join or resume a meeting participant. Reuse resume_token after disconnect so the same agent is not duplicated.",
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
            "description": "Get this participant's current meeting task. Repeated calls are idempotent until the turn is submitted.",
            "inputSchema": participant_auth_schema()
        }),
        json!({
            "name": "wenyuan_submit_turn",
            "description": "Submit one assigned turn. Re-submitting an already submitted turn is idempotent.",
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
            "description": "Ask only for essential missing information. Equivalent questions are merged and only one distinct question is active at a time.",
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
            "description": "Host-only: answer the current user question once. The answer is shared with all participants and is not asked again.",
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
            "description": "Refresh participant liveness while waiting for peers or user input.",
            "inputSchema": participant_auth_schema()
        }),
        json!({
            "name": "wenyuan_meeting_status",
            "description": "Read meeting stage, participant liveness, active user question and progress without exposing participant secrets.",
            "inputSchema": {
                "type": "object",
                "required": ["meeting_id"],
                "properties": { "meeting_id": { "type": "string" } }
            }
        }),
    ]
}

async fn dispatch_tool_call(hub: &MeetingHub, params: &Value) -> Result<Value, McpError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidParams("missing tool name".into()))?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let result: Result<Value, HubError> = match name {
        "wenyuan_create_meeting" => hub.create_meeting(from_args(args)?).await,
        "wenyuan_start_meeting" => hub.start_meeting(from_args(args)?).await,
        "wenyuan_join_meeting" => hub.join_meeting(from_args(args)?).await,
        "wenyuan_next_task" => hub.next_task(from_args(args)?).await,
        "wenyuan_submit_turn" => hub.submit_turn(from_args(args)?).await,
        "wenyuan_ask_user" => hub.ask_user(from_args(args)?).await,
        "wenyuan_answer_user" => hub.answer_user(from_args(args)?).await,
        "wenyuan_heartbeat" => hub.heartbeat(from_args(args)?).await,
        "wenyuan_meeting_status" => {
            let req: MeetingStatusRequest = from_args(args)?;
            hub.status(req.meeting_id).await
        }
        other => return Err(McpError::UnknownTool(other.to_string())),
    };

    Ok(match result {
        Ok(payload) => tool_result(payload, false),
        Err(err) => tool_result(json!({ "error": err.to_string(), "kind": err.kind() }), true),
    })
}

fn from_args<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, HubError> {
    serde_json::from_value(value).map_err(|err| HubError::InvalidInput(err.to_string()))
}

fn tool_result(payload: Value, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()) }],
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
            Self::UnknownTool(_) | Self::InvalidParams(_) => -32602,
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
    #[error("at least two participants are required")]
    NotEnoughParticipants,
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
            Self::NotEnoughParticipants => "not_enough_participants",
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

fn default_max_rounds() -> u8 { 2 }
fn default_heartbeat_timeout() -> u64 { 45 }
fn default_offline_grace() -> u64 { 60 }

impl MeetingConfig {
    fn validate(&self) -> Result<(), HubError> {
        if !(2..=3).contains(&self.participant_limit) {
            return Err(HubError::InvalidInput("participant_limit must be 2 or 3".into()));
        }
        if !(1..=2).contains(&self.max_rounds) {
            return Err(HubError::InvalidInput("max_rounds must be 1 or 2".into()));
        }
        if self.title.trim().is_empty() || self.goal.trim().is_empty() {
            return Err(HubError::InvalidInput("title and goal must not be empty".into()));
        }
        if self.heartbeat_timeout_secs < 10 || self.offline_grace_secs < 10 {
            return Err(HubError::InvalidInput("timeouts must be at least 10 seconds".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStage { Lobby, Initial, CrossReview, Synthesis, Completed, Paused }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantState { Online, Stale, Left }

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TurnStage { Initial, CrossReview, Synthesis }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus { Pending, Claimed, Submitted, Skipped }

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
pub enum QuestionStatus { Open, Deferred, Answered }

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
        Self { meetings: Arc::new(RwLock::new(HashMap::new())), state_path: None }
    }

    pub async fn load(state_path: Option<PathBuf>) -> Self {
        let mut meetings = HashMap::new();
        if let Some(path) = state_path.as_ref() {
            if let Ok(bytes) = tokio::fs::read(path).await {
                match serde_json::from_slice::<Vec<Meeting>>(&bytes) {
                    Ok(items) => meetings.extend(items.into_iter().map(|meeting| (meeting.id, meeting))),
                    Err(err) => warn!("failed to restore MCP meetings: {err}"),
                }
            }
        }
        Self { meetings: Arc::new(RwLock::new(meetings)), state_path: state_path.map(Arc::new) }
    }

    async fn persist(&self) {
        let Some(path) = self.state_path.as_ref() else { return; };
        let snapshot: Vec<Meeting> = self.meetings.read().await.values().cloned().collect();
        let Ok(bytes) = serde_json::to_vec_pretty(&snapshot) else { return; };
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
            heartbeat_timeout_secs: request.heartbeat_timeout_secs.unwrap_or_else(default_heartbeat_timeout),
            offline_grace_secs: request.offline_grace_secs.unwrap_or_else(default_offline_grace),
        };
        config.validate()?;
        let id = Uuid::new_v4();
        let meeting = Meeting {
            id,
            config,
            stage: MeetingStage::Lobby,
            join_code: short_secret(),
            owner_token: Uuid::new_v4().to_string(),
            participants: vec![],
            turns: vec![],
            questions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            paused_reason: None,
        };
        let result = json!({
            "meeting_id": id,
            "join_code": meeting.join_code,
            "owner_token": meeting.owner_token,
            "participant_limit": meeting.config.participant_limit,
            "stage": "lobby",
            "policy": {
                "roles": "peer agents share the same goal; no forced viewpoint",
                "cross_review": "only substantive disagreement or omissions; do not repeat consensus",
                "questions": "essential questions only; deduplicated and serialized",
                "offline": "3-agent meetings may degrade to 2 after grace; 2-agent meetings pause rather than degrade to 1"
            }
        });
        self.meetings.write().await.insert(id, meeting);
        self.persist().await;
        Ok(result)
    }

    pub async fn start_meeting(&self, request: StartMeetingRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        if meeting.owner_token != request.owner_token { return Err(HubError::InvalidOwnerToken); }
        if meeting.stage != MeetingStage::Lobby { return Err(HubError::MeetingStarted); }
        if meeting.participants.len() < 2 { return Err(HubError::NotEnoughParticipants); }
        start_initial_round(meeting);
        let result = meeting_snapshot(meeting);
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn join_meeting(&self, request: JoinMeetingRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        if request.join_code != meeting.join_code { return Err(HubError::InvalidJoinCode); }

        if let Some(token) = request.resume_token.as_deref() {
            let Some(index) = meeting.participants.iter().position(|p| p.resume_token == token) else {
                return Err(HubError::InvalidParticipantToken);
            };
            {
                let participant = &mut meeting.participants[index];
                participant.last_seen = Utc::now();
                participant.state = ParticipantState::Online;
                if !request.display_name.trim().is_empty() {
                    participant.display_name = request.display_name.trim().to_string();
                }
            }
            meeting.updated_at = Utc::now();
            if meeting.stage == MeetingStage::Paused && online_count(meeting) >= 2 {
                meeting.paused_reason = None;
                meeting.stage = infer_stage_from_turns(meeting);
            }
            let participant = meeting.participants[index].clone();
            let result = join_payload(meeting, &participant);
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        if meeting.stage != MeetingStage::Lobby { return Err(HubError::MeetingStarted); }
        if meeting.participants.len() >= usize::from(meeting.config.participant_limit) {
            return Err(HubError::MeetingFull);
        }
        let display_name = request.display_name.trim();
        if display_name.is_empty() { return Err(HubError::InvalidInput("display_name must not be empty".into())); }
        if meeting.participants.iter().any(|p| p.display_name.eq_ignore_ascii_case(display_name)) {
            return Err(HubError::InvalidInput("display_name already exists; reconnect with resume_token".into()));
        }

        let participant = Participant {
            id: Uuid::new_v4(),
            display_name: display_name.to_string(),
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
        let result = join_payload(meeting, &participant);
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn next_task(&self, auth: ParticipantAuth) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&auth.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&auth.participant_id, "participant_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &auth.resume_token)?;
        refresh_offline_and_progress(meeting);

        if let Some(question) = active_question(meeting) {
            let result = json!({ "status": "waiting_for_user", "question": public_question(question), "stage": meeting.stage });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }
        if meeting.stage == MeetingStage::Paused {
            let reason = meeting.paused_reason.clone().unwrap_or_else(|| "meeting paused".into());
            drop(meetings);
            self.persist().await;
            return Err(HubError::MeetingPaused(reason));
        }
        if meeting.stage == MeetingStage::Completed {
            let result = json!({ "status": "completed", "stage": meeting.stage, "final": final_response(meeting) });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }
        if meeting.stage == MeetingStage::Lobby {
            let result = json!({ "status": "waiting_for_peers", "stage": meeting.stage, "progress": progress_summary(meeting) });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        if let Some(index) = meeting.turns.iter().position(|turn| {
            turn.assignee == participant_id && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed)
        }) {
            if meeting.turns[index].status == TurnStatus::Pending {
                meeting.turns[index].status = TurnStatus::Claimed;
                meeting.turns[index].updated_at = Utc::now();
            }
            let turn = meeting.turns[index].clone();
            let answers = answered_questions(meeting);
            let result = json!({
                "status": "task",
                "meeting_id": meeting.id,
                "stage": meeting.stage,
                "turn_id": turn.id,
                "turn_stage": turn.stage,
                "payload": turn.payload,
                "user_answers": answers,
                "instruction": task_instruction(turn.stage)
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        let result = json!({ "status": "waiting_for_peers", "stage": meeting.stage, "progress": progress_summary(meeting) });
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn submit_turn(&self, request: SubmitTurnRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&request.participant_id, "participant_id")?;
        let turn_id = parse_uuid(&request.turn_id, "turn_id")?;
        if request.response.trim().is_empty() { return Err(HubError::InvalidInput("response must not be empty".into())); }
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &request.resume_token)?;
        let Some(index) = meeting.turns.iter().position(|turn| turn.id == turn_id) else { return Err(HubError::TurnNotFound); };
        if meeting.turns[index].assignee != participant_id { return Err(HubError::WrongAssignee); }
        if meeting.turns[index].status == TurnStatus::Submitted {
            let result = json!({ "ok": true, "idempotent": true, "turn_id": turn_id, "stage": meeting.stage });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }
        meeting.turns[index].response = Some(request.response.trim().to_string());
        meeting.turns[index].status = TurnStatus::Submitted;
        meeting.turns[index].updated_at = Utc::now();
        meeting.updated_at = Utc::now();
        refresh_offline_and_progress(meeting);
        let result = json!({ "ok": true, "idempotent": false, "turn_id": turn_id, "stage": meeting.stage, "progress": progress_summary(meeting) });
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn ask_user(&self, request: AskUserRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let participant_id = parse_uuid(&request.participant_id, "participant_id")?;
        if request.question.trim().is_empty() || request.reason.trim().is_empty() {
            return Err(HubError::InvalidInput("question and reason must not be empty".into()));
        }
        let normalized = normalize_question(&request.question);
        let missing_key = request.missing_key.unwrap_or_default().trim().to_string();
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &request.resume_token)?;

        if let Some(index) = meeting.questions.iter().position(|question| {
            matches!(question.status, QuestionStatus::Open | QuestionStatus::Deferred)
                && questions_match(question, &normalized, &missing_key)
        }) {
            if !meeting.questions[index].asked_by.contains(&participant_id) {
                meeting.questions[index].asked_by.push(participant_id);
            }
            let existing = meeting.questions[index].clone();
            let active = active_question(meeting).map(public_question);
            let result = json!({
                "deduplicated": true,
                "queued": existing.status == QuestionStatus::Deferred,
                "question": public_question(&existing),
                "active_question": active
            });
            drop(meetings);
            self.persist().await;
            return Ok(result);
        }

        let queued = active_question(meeting).is_some();
        let question = UserQuestion {
            id: Uuid::new_v4(),
            question: request.question.trim().to_string(),
            reason: request.reason.trim().to_string(),
            missing_key,
            asked_by: vec![participant_id],
            status: if queued { QuestionStatus::Deferred } else { QuestionStatus::Open },
            answer: None,
            created_at: Utc::now(),
            answered_at: None,
        };
        let public = public_question(&question);
        meeting.questions.push(question);
        meeting.updated_at = Utc::now();
        let active = active_question(meeting).map(public_question);
        drop(meetings);
        self.persist().await;
        Ok(json!({ "deduplicated": false, "queued": queued, "question": public, "active_question": active }))
    }

    pub async fn answer_user(&self, request: AnswerUserRequest) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&request.meeting_id, "meeting_id")?;
        let question_id = parse_uuid(&request.question_id, "question_id")?;
        if request.answer.trim().is_empty() { return Err(HubError::InvalidInput("answer must not be empty".into())); }
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        if meeting.owner_token != request.owner_token { return Err(HubError::InvalidOwnerToken); }
        let Some(index) = meeting.questions.iter().position(|question| question.id == question_id) else { return Err(HubError::QuestionNotFound); };
        meeting.questions[index].answer = Some(request.answer.trim().to_string());
        meeting.questions[index].status = QuestionStatus::Answered;
        meeting.questions[index].answered_at = Some(Utc::now());
        if let Some(next) = meeting.questions.iter_mut().find(|q| q.status == QuestionStatus::Deferred) {
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
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
        touch_participant(meeting, participant_id, &auth.resume_token)?;
        refresh_offline_and_progress(meeting);
        let result = json!({ "ok": true, "stage": meeting.stage, "progress": progress_summary(meeting) });
        drop(meetings);
        self.persist().await;
        Ok(result)
    }

    pub async fn status(&self, meeting_id: String) -> Result<Value, HubError> {
        let meeting_id = parse_uuid(&meeting_id, "meeting_id")?;
        let mut meetings = self.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).ok_or(HubError::MeetingNotFound)?;
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
pub struct StartMeetingRequest { pub meeting_id: String, pub owner_token: String }
#[derive(Debug, Deserialize)]
pub struct JoinMeetingRequest {
    pub meeting_id: String,
    pub join_code: String,
    pub display_name: String,
    pub client_name: Option<String>,
    pub resume_token: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct ParticipantAuth { pub meeting_id: String, pub participant_id: String, pub resume_token: String }
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
pub struct AnswerUserRequest { pub meeting_id: String, pub owner_token: String, pub question_id: String, pub answer: String }
#[derive(Debug, Deserialize)]
pub struct MeetingStatusRequest { pub meeting_id: String }

fn short_secret() -> String {
    Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>().to_uppercase()
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, HubError> {
    Uuid::parse_str(value).map_err(|_| HubError::InvalidInput(format!("{field} must be a UUID")))
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
        "instruction": "Keep participant_id and resume_token. Call next_task; heartbeat while waiting. Solve the user's goal directly and do not invent a role or viewpoint."
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
                    "Solve the user's actual goal; no preassigned viewpoint.",
                    "Give your own best recommendation or implementation independently.",
                    "State assumptions and decisive uncertainty, not generic caveats.",
                    "Ask the user only when missing information can materially change the answer and cannot be handled with an explicit assumption."
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
        if participant.state != ParticipantState::Left && now.signed_duration_since(participant.last_seen) > timeout {
            participant.state = ParticipantState::Stale;
        }
    }
    if matches!(meeting.stage, MeetingStage::Completed | MeetingStage::Lobby) { return; }

    let healthy: HashSet<Uuid> = meeting.participants.iter()
        .filter(|p| p.state != ParticipantState::Left && now.signed_duration_since(p.last_seen) <= timeout + grace)
        .map(|p| p.id)
        .collect();
    let current = match meeting.stage {
        MeetingStage::Initial => Some(TurnStage::Initial),
        MeetingStage::CrossReview => Some(TurnStage::CrossReview),
        MeetingStage::Synthesis => Some(TurnStage::Synthesis),
        _ => None,
    };

    if let Some(stage) = current {
        let lost: Vec<Uuid> = meeting.turns.iter()
            .filter(|turn| turn.stage == stage && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed) && !healthy.contains(&turn.assignee))
            .map(|turn| turn.assignee)
            .collect();
        if !lost.is_empty() {
            if healthy.len() < 2 {
                meeting.stage = MeetingStage::Paused;
                meeting.paused_reason = Some("fewer than two live agents remain; Wenyuan will not silently turn the meeting into a single-agent answer".into());
                meeting.updated_at = now;
                return;
            }
            if stage == TurnStage::Synthesis {
                if let Some(index) = meeting.turns.iter().position(|turn| turn.stage == TurnStage::Synthesis && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed) && lost.contains(&turn.assignee)) {
                    let old = meeting.turns[index].assignee;
                    if let Some(replacement) = choose_synthesizer(meeting, Some(old)) {
                        meeting.turns[index].assignee = replacement;
                        meeting.turns[index].status = TurnStatus::Pending;
                        meeting.turns[index].skip_reason = Some("synthesizer_reassigned_after_disconnect".into());
                        meeting.turns[index].updated_at = now;
                    }
                }
            } else {
                for turn in &mut meeting.turns {
                    if turn.stage == stage && matches!(turn.status, TurnStatus::Pending | TurnStatus::Claimed) && lost.contains(&turn.assignee) {
                        turn.status = TurnStatus::Skipped;
                        turn.skip_reason = Some("participant_offline_after_grace".into());
                        turn.updated_at = now;
                    }
                }
            }
        }
    }

    if meeting.stage == MeetingStage::Paused || active_question(meeting).is_some() { return; }
    match meeting.stage {
        MeetingStage::Initial if stage_is_terminal(meeting, TurnStage::Initial) => {
            if submitted_count(meeting, TurnStage::Initial) < 2 {
                meeting.stage = MeetingStage::Paused;
                meeting.paused_reason = Some("fewer than two initial opinions are available".into());
            } else if meeting.config.max_rounds >= 2 {
                create_cross_review_round(meeting);
            } else {
                create_synthesis_round(meeting);
            }
        }
        MeetingStage::CrossReview if stage_is_terminal(meeting, TurnStage::CrossReview) => create_synthesis_round(meeting),
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
    let outputs = submitted_outputs(meeting, TurnStage::Initial);
    let participants: Vec<Uuid> = meeting.participants.iter().filter(|p| p.state == ParticipantState::Online).map(|p| p.id).collect();
    for participant_id in participants {
        let peer_outputs: Vec<Value> = outputs.iter().filter(|(author, _)| *author != participant_id)
            .map(|(author, response)| json!({ "participant_id": author, "response": response })).collect();
        meeting.turns.push(MeetingTurn {
            id: Uuid::new_v4(), stage: TurnStage::CrossReview, assignee: participant_id, status: TurnStatus::Pending,
            payload: json!({
                "title": meeting.config.title,
                "goal": meeting.config.goal,
                "context": meeting.config.context,
                "peer_outputs": peer_outputs,
                "rules": [
                    "Do not repeat points everyone already agrees on.",
                    "React only to material disagreement, missing constraints, or a clearly better implementation.",
                    "Agreement is valid; disagreement is never required for its own sake.",
                    "If user input is truly decisive, ask the smallest non-duplicate question via wenyuan_ask_user."
                ]
            }),
            response: None, skip_reason: None, created_at: now, updated_at: now,
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
    let contributions: Vec<Value> = meeting.turns.iter().filter(|turn| turn.status == TurnStatus::Submitted)
        .filter_map(|turn| turn.response.as_ref().map(|response| json!({ "participant_id": turn.assignee, "stage": turn.stage, "response": response }))).collect();
    meeting.turns.push(MeetingTurn {
        id: Uuid::new_v4(), stage: TurnStage::Synthesis, assignee, status: TurnStatus::Pending,
        payload: json!({
            "title": meeting.config.title,
            "goal": meeting.config.goal,
            "context": meeting.config.context,
            "contributions": contributions,
            "user_answers": answered_questions(meeting),
            "rules": [
                "You are only a temporary editor, not a privileged viewpoint.",
                "Choose the strongest implementation for the user's goal rather than averaging incompatible answers.",
                "Preserve meaningful dissent and uncertainty; never fabricate consensus.",
                "Do not re-ask answered questions.",
                "Return a concise recommendation, implementation steps, trade-offs, and unresolved items."
            ]
        }),
        response: None, skip_reason: None, created_at: now, updated_at: now,
    });
    meeting.updated_at = now;
}

fn choose_synthesizer(meeting: &Meeting, exclude: Option<Uuid>) -> Option<Uuid> {
    meeting.participants.iter()
        .filter(|p| p.state == ParticipantState::Online && Some(p.id) != exclude)
        .min_by_key(|p| meeting.turns.iter().filter(|turn| turn.assignee == p.id && turn.status == TurnStatus::Submitted).count())
        .map(|p| p.id)
}

fn stage_is_terminal(meeting: &Meeting, stage: TurnStage) -> bool {
    let turns: Vec<&MeetingTurn> = meeting.turns.iter().filter(|turn| turn.stage == stage).collect();
    !turns.is_empty() && turns.iter().all(|turn| matches!(turn.status, TurnStatus::Submitted | TurnStatus::Skipped))
}

fn submitted_count(meeting: &Meeting, stage: TurnStage) -> usize {
    meeting.turns.iter().filter(|turn| turn.stage == stage && turn.status == TurnStatus::Submitted).count()
}

fn submitted_outputs(meeting: &Meeting, stage: TurnStage) -> Vec<(Uuid, String)> {
    meeting.turns.iter().filter(|turn| turn.stage == stage && turn.status == TurnStatus::Submitted)
        .filter_map(|turn| turn.response.clone().map(|response| (turn.assignee, response))).collect()
}

fn online_count(meeting: &Meeting) -> usize {
    meeting.participants.iter().filter(|p| p.state == ParticipantState::Online).count()
}

fn touch_participant(meeting: &mut Meeting, participant_id: Uuid, resume_token: &str) -> Result<(), HubError> {
    let Some(index) = meeting.participants.iter().position(|p| p.id == participant_id) else { return Err(HubError::ParticipantNotFound); };
    if meeting.participants[index].resume_token != resume_token { return Err(HubError::InvalidParticipantToken); }
    meeting.participants[index].last_seen = Utc::now();
    meeting.participants[index].state = ParticipantState::Online;
    meeting.updated_at = Utc::now();
    if meeting.stage == MeetingStage::Paused && online_count(meeting) >= 2 {
        meeting.paused_reason = None;
        meeting.stage = infer_stage_from_turns(meeting);
    }
    Ok(())
}

fn infer_stage_from_turns(meeting: &Meeting) -> MeetingStage {
    if meeting.turns.iter().any(|t| t.stage == TurnStage::Synthesis && t.status != TurnStatus::Submitted) { MeetingStage::Synthesis }
    else if meeting.turns.iter().any(|t| t.stage == TurnStage::CrossReview && !matches!(t.status, TurnStatus::Submitted | TurnStatus::Skipped)) { MeetingStage::CrossReview }
    else if meeting.turns.iter().any(|t| t.stage == TurnStage::Initial && !matches!(t.status, TurnStatus::Submitted | TurnStatus::Skipped)) { MeetingStage::Initial }
    else if meeting.turns.iter().any(|t| t.stage == TurnStage::Synthesis && t.status == TurnStatus::Submitted) { MeetingStage::Completed }
    else { MeetingStage::Initial }
}

fn normalize_question(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation()).flat_map(char::to_lowercase).collect()
}

fn questions_match(existing: &UserQuestion, normalized: &str, missing_key: &str) -> bool {
    if !missing_key.is_empty() && existing.missing_key == missing_key { return true; }
    let other = normalize_question(&existing.question);
    if other == normalized { return true; }
    let min_len = other.chars().count().min(normalized.chars().count());
    min_len >= 8 && (other.contains(normalized) || normalized.contains(&other))
}

fn active_question(meeting: &Meeting) -> Option<&UserQuestion> {
    meeting.questions.iter().find(|q| q.status == QuestionStatus::Open)
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
    meeting.questions.iter().filter(|q| q.status == QuestionStatus::Answered)
        .filter_map(|q| q.answer.as_ref().map(|answer| json!({ "question": q.question, "answer": answer, "missing_key": q.missing_key }))).collect()
}

fn task_instruction(stage: TurnStage) -> &'static str {
    match stage {
        TurnStage::Initial => "Give your own best answer to the same user goal. Do not role-play a fixed perspective.",
        TurnStage::CrossReview => "Review only substantive disagreement or omissions. Do not repeat consensus just to create interaction.",
        TurnStage::Synthesis => "Act as temporary synthesizer: choose the strongest implementation, preserve real dissent, and avoid fake consensus.",
    }
}

fn final_response(meeting: &Meeting) -> Option<Value> {
    meeting.turns.iter().rev().find(|turn| turn.stage == TurnStage::Synthesis && turn.status == TurnStatus::Submitted)
        .and_then(|turn| turn.response.as_ref().map(|response| json!({
            "response": response,
            "synthesizer": turn.assignee,
            "degraded": meeting.turns.iter().any(|turn| turn.status == TurnStatus::Skipped),
            "skipped_turns": meeting.turns.iter().filter(|turn| turn.status == TurnStatus::Skipped).count()
        })))
}

fn progress_summary(meeting: &Meeting) -> Value {
    let mut stages = HashMap::new();
    for stage in [TurnStage::Initial, TurnStage::CrossReview, TurnStage::Synthesis] {
        let total = meeting.turns.iter().filter(|turn| turn.stage == stage).count();
        let submitted = meeting.turns.iter().filter(|turn| turn.stage == stage && turn.status == TurnStatus::Submitted).count();
        let skipped = meeting.turns.iter().filter(|turn| turn.stage == stage && turn.status == TurnStatus::Skipped).count();
        stages.insert(format!("{stage:?}").to_lowercase(), json!({ "total": total, "submitted": submitted, "skipped": skipped }));
    }
    json!({
        "turns": stages,
        "participants_online": online_count(meeting),
        "questions_open": meeting.questions.iter().filter(|q| q.status == QuestionStatus::Open).count(),
        "questions_deferred": meeting.questions.iter().filter(|q| q.status == QuestionStatus::Deferred).count()
    })
}

fn meeting_snapshot(meeting: &Meeting) -> Value {
    json!({
        "meeting_id": meeting.id,
        "title": meeting.config.title,
        "goal": meeting.config.goal,
        "stage": meeting.stage,
        "participant_limit": meeting.config.participant_limit,
        "participants": meeting.participants.iter().map(|p| json!({
            "id": p.id, "display_name": p.display_name, "client_name": p.client_name, "state": p.state, "last_seen": p.last_seen
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
        let created = hub.create_meeting(CreateMeetingRequest {
            title: "test".into(), goal: "find the best implementation".into(), context: None,
            participant_limit: 2, max_rounds: Some(2), heartbeat_timeout_secs: Some(10), offline_grace_secs: Some(10),
        }).await.unwrap();
        let meeting_id = created["meeting_id"].as_str().unwrap().to_string();
        let join_code = created["join_code"].as_str().unwrap().to_string();
        let a = hub.join_meeting(JoinMeetingRequest {
            meeting_id: meeting_id.clone(), join_code: join_code.clone(), display_name: "A".into(), client_name: Some("client-a".into()), resume_token: None,
        }).await.unwrap();
        let b = hub.join_meeting(JoinMeetingRequest {
            meeting_id, join_code, display_name: "B".into(), client_name: Some("client-b".into()), resume_token: None,
        }).await.unwrap();
        (created, a, b)
    }

    fn auth(join: &Value) -> ParticipantAuth {
        ParticipantAuth {
            meeting_id: join["meeting_id"].as_str().unwrap().into(),
            participant_id: join["participant_id"].as_str().unwrap().into(),
            resume_token: join["resume_token"].as_str().unwrap().into(),
        }
    }

    #[tokio::test]
    async fn two_agents_receive_the_same_goal_without_roles() {
        let hub = MeetingHub::new_ephemeral();
        let (_, a, b) = create_two(&hub).await;
        let ta = hub.next_task(auth(&a)).await.unwrap();
        let tb = hub.next_task(auth(&b)).await.unwrap();
        assert_eq!(ta["payload"]["goal"], tb["payload"]["goal"]);
        assert_eq!(ta["turn_stage"], "initial");
    }

    #[tokio::test]
    async fn next_task_is_idempotent_until_submit() {
        let hub = MeetingHub::new_ephemeral();
        let (_, a, _) = create_two(&hub).await;
        let first = hub.next_task(auth(&a)).await.unwrap();
        let second = hub.next_task(auth(&a)).await.unwrap();
        assert_eq!(first["turn_id"], second["turn_id"]);
    }

    #[tokio::test]
    async fn duplicate_user_questions_are_merged() {
        let hub = MeetingHub::new_ephemeral();
        let (_, a, b) = create_two(&hub).await;
        let first = hub.ask_user(AskUserRequest {
            meeting_id: a["meeting_id"].as_str().unwrap().into(), participant_id: a["participant_id"].as_str().unwrap().into(),
            resume_token: a["resume_token"].as_str().unwrap().into(), question: "用户的预算上限是多少？".into(), reason: "会改变方案".into(), missing_key: Some("budget".into()),
        }).await.unwrap();
        let second = hub.ask_user(AskUserRequest {
            meeting_id: b["meeting_id"].as_str().unwrap().into(), participant_id: b["participant_id"].as_str().unwrap().into(),
            resume_token: b["resume_token"].as_str().unwrap().into(), question: "预算上限是多少".into(), reason: "影响推荐".into(), missing_key: Some("budget".into()),
        }).await.unwrap();
        assert_eq!(first["deduplicated"], false);
        assert_eq!(second["deduplicated"], true);
        assert_eq!(second["question"]["asked_by_count"], 2);
    }

    #[tokio::test]
    async fn two_agent_meeting_pauses_after_disconnect_grace() {
        let hub = MeetingHub::new_ephemeral();
        let (_, a, b) = create_two(&hub).await;
        let meeting_id = Uuid::parse_str(a["meeting_id"].as_str().unwrap()).unwrap();
        let b_id = Uuid::parse_str(b["participant_id"].as_str().unwrap()).unwrap();
        let mut meetings = hub.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_id).unwrap();
        meeting.participants.iter_mut().find(|p| p.id == b_id).unwrap().last_seen = Utc::now() - ChronoDuration::seconds(25);
        refresh_offline_and_progress(meeting);
        assert_eq!(meeting.stage, MeetingStage::Paused);
    }

    #[tokio::test]
    async fn three_agent_meeting_degrades_to_two_after_grace() {
        let hub = MeetingHub::new_ephemeral();
        let created = hub.create_meeting(CreateMeetingRequest {
            title: "test".into(), goal: "best implementation".into(), context: None,
            participant_limit: 3, max_rounds: Some(1), heartbeat_timeout_secs: Some(10), offline_grace_secs: Some(10),
        }).await.unwrap();
        let meeting_id = created["meeting_id"].as_str().unwrap().to_string();
        let join_code = created["join_code"].as_str().unwrap().to_string();
        let mut joined = vec![];
        for name in ["A", "B", "C"] {
            joined.push(hub.join_meeting(JoinMeetingRequest {
                meeting_id: meeting_id.clone(), join_code: join_code.clone(), display_name: name.into(), client_name: None, resume_token: None,
            }).await.unwrap());
        }
        let meeting_uuid = Uuid::parse_str(&meeting_id).unwrap();
        let c_id = Uuid::parse_str(joined[2]["participant_id"].as_str().unwrap()).unwrap();
        let mut meetings = hub.meetings.write().await;
        let meeting = meetings.get_mut(&meeting_uuid).unwrap();
        meeting.participants.iter_mut().find(|p| p.id == c_id).unwrap().last_seen = Utc::now() - ChronoDuration::seconds(25);
        refresh_offline_and_progress(meeting);
        assert_ne!(meeting.stage, MeetingStage::Paused);
        assert!(meeting.turns.iter().any(|turn| turn.assignee == c_id && turn.status == TurnStatus::Skipped));
    }

    #[tokio::test]
    async fn resume_token_reconnects_same_participant() {
        let hub = MeetingHub::new_ephemeral();
        let (created, a, _) = create_two(&hub).await;
        let resumed = hub.join_meeting(JoinMeetingRequest {
            meeting_id: a["meeting_id"].as_str().unwrap().into(),
            join_code: created["join_code"].as_str().unwrap().into(),
            display_name: "A".into(), client_name: Some("client-a-2".into()),
            resume_token: Some(a["resume_token"].as_str().unwrap().into()),
        }).await.unwrap();
        assert_eq!(resumed["participant_id"], a["participant_id"]);
    }
}
