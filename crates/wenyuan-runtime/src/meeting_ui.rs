use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Clone)]
pub struct MeetingUiState {
    state_path: Arc<PathBuf>,
    mcp_addr: Option<SocketAddr>,
    local_token: Arc<String>,
    client: reqwest::Client,
}

pub fn router(data_dir: PathBuf, mcp_addr: Option<SocketAddr>, local_token: String) -> Router {
    let state = MeetingUiState {
        state_path: Arc::new(data_dir.join("mcp-meetings.json")),
        mcp_addr,
        local_token: Arc::new(local_token),
        client: reqwest::Client::new(),
    };

    Router::new()
        .route("/api/meetings", get(list_meetings).post(create_meeting))
        .route("/api/meetings/{id}", get(get_meeting))
        .route("/api/meetings/{id}/start", post(start_meeting))
        .route(
            "/api/meetings/{id}/questions/{question_id}/answer",
            post(answer_question),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct CreateMeetingInput {
    title: String,
    goal: String,
    #[serde(default)]
    context: String,
    participant_limit: u8,
}

#[derive(Debug, Deserialize)]
struct OwnerInput {
    owner_token: String,
}

#[derive(Debug, Deserialize)]
struct AnswerInput {
    owner_token: String,
    answer: String,
}

async fn list_meetings(State(state): State<MeetingUiState>) -> Response {
    match read_meetings(&state.state_path).await {
        Ok(mut meetings) => {
            meetings.sort_by(|a, b| {
                b.get("updated_at")
                    .and_then(Value::as_str)
                    .cmp(&a.get("updated_at").and_then(Value::as_str))
            });
            let rows: Vec<Value> = meetings.iter().map(meeting_summary).collect();
            Json(json!(rows)).into_response()
        }
        Err(response) => response,
    }
}

async fn get_meeting(
    State(state): State<MeetingUiState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Let the coordinator refresh liveness/stage before we read the persisted snapshot.
    if state.mcp_addr.is_some() {
        let _ = mcp_call(
            &state,
            "wenyuan_meeting_status",
            json!({ "meeting_id": id }),
        )
        .await;
    }

    let meetings = match read_meetings(&state.state_path).await {
        Ok(items) => items,
        Err(response) => return response,
    };
    let Some(meeting) = meetings
        .iter()
        .find(|meeting| meeting.get("id").and_then(Value::as_str) == Some(id.as_str()))
    else {
        return api_error(StatusCode::NOT_FOUND, "meeting not found");
    };

    let owner_token = headers
        .get("x-wenyuan-meeting-owner")
        .and_then(|value| value.to_str().ok());
    let owner_access = owner_token.is_some_and(|token| {
        meeting.get("owner_token").and_then(Value::as_str) == Some(token)
    });
    Json(meeting_detail(meeting, owner_access, mcp_endpoint(&state))).into_response()
}

async fn create_meeting(
    State(state): State<MeetingUiState>,
    headers: HeaderMap,
    Json(input): Json<CreateMeetingInput>,
) -> Response {
    if let Err(response) = require_local_write(&state, &headers) {
        return response;
    }
    if !(2..=3).contains(&input.participant_limit) {
        return api_error(StatusCode::BAD_REQUEST, "participant_limit must be 2 or 3");
    }
    if input.title.trim().is_empty() || input.goal.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "title and goal must not be empty");
    }

    match mcp_call(
        &state,
        "wenyuan_create_meeting",
        json!({
            "title": input.title.trim(),
            "goal": input.goal.trim(),
            "context": input.context.trim(),
            "participant_limit": input.participant_limit,
            "max_rounds": 2
        }),
    )
    .await
    {
        Ok(mut payload) => {
            if let Some(object) = payload.as_object_mut() {
                object.insert("mcp_endpoint".into(), json!(mcp_endpoint(&state)));
            }
            Json(payload).into_response()
        }
        Err(response) => response,
    }
}

async fn start_meeting(
    State(state): State<MeetingUiState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<OwnerInput>,
) -> Response {
    if let Err(response) = require_local_write(&state, &headers) {
        return response;
    }
    match mcp_call(
        &state,
        "wenyuan_start_meeting",
        json!({ "meeting_id": id, "owner_token": input.owner_token }),
    )
    .await
    {
        Ok(payload) => Json(payload).into_response(),
        Err(response) => response,
    }
}

async fn answer_question(
    State(state): State<MeetingUiState>,
    Path((id, question_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(input): Json<AnswerInput>,
) -> Response {
    if let Err(response) = require_local_write(&state, &headers) {
        return response;
    }
    if input.answer.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "answer must not be empty");
    }
    match mcp_call(
        &state,
        "wenyuan_answer_user",
        json!({
            "meeting_id": id,
            "owner_token": input.owner_token,
            "question_id": question_id,
            "answer": input.answer.trim()
        }),
    )
    .await
    {
        Ok(payload) => Json(payload).into_response(),
        Err(response) => response,
    }
}

fn require_local_write(state: &MeetingUiState, headers: &HeaderMap) -> Result<(), Response> {
    let token = headers
        .get("x-wenyuan-token")
        .and_then(|value| value.to_str().ok());
    if token == Some(state.local_token.as_str()) {
        Ok(())
    } else {
        Err(api_error(StatusCode::FORBIDDEN, "invalid local write token"))
    }
}

fn mcp_endpoint(state: &MeetingUiState) -> Option<String> {
    state
        .mcp_addr
        .map(|addr| format!("http://{addr}/mcp"))
}

async fn mcp_call(state: &MeetingUiState, tool: &str, arguments: Value) -> Result<Value, Response> {
    let Some(addr) = state.mcp_addr else {
        return Err(api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "MCP meeting server is not available",
        ));
    };
    let response = state
        .client
        .post(format!("http://{addr}/mcp"))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        }))
        .send()
        .await
        .map_err(|err| {
            api_error(
                StatusCode::BAD_GATEWAY,
                format!("failed to reach MCP meeting server: {err}"),
            )
        })?;

    let status = response.status();
    let payload: Value = response.json().await.map_err(|err| {
        api_error(
            StatusCode::BAD_GATEWAY,
            format!("invalid MCP response: {err}"),
        )
    })?;
    if !status.is_success() {
        return Err(api_error(
            StatusCode::BAD_GATEWAY,
            format!("MCP meeting server returned {status}"),
        ));
    }
    if let Some(message) = payload
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
    {
        return Err(api_error(StatusCode::BAD_GATEWAY, message));
    }

    let result = payload.get("result").cloned().unwrap_or(Value::Null);
    let structured = result
        .get("structuredContent")
        .cloned()
        .unwrap_or(Value::Null);
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        let message = structured
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("MCP meeting operation failed");
        let kind = structured.get("kind").and_then(Value::as_str).unwrap_or("");
        let http_status = match kind {
            "invalid_owner_token" | "invalid_participant_token" => StatusCode::FORBIDDEN,
            "meeting_not_found" | "question_not_found" => StatusCode::NOT_FOUND,
            "meeting_paused" | "meeting_started" | "not_enough_participants" => {
                StatusCode::CONFLICT
            }
            _ => StatusCode::BAD_REQUEST,
        };
        return Err(api_error(http_status, message));
    }
    Ok(structured)
}

async fn read_meetings(path: &PathBuf) -> Result<Vec<Value>, Response> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut last_error = None;
    for attempt in 0..3 {
        match tokio::fs::read(path.as_path()).await {
            Ok(bytes) => match serde_json::from_slice::<Vec<Value>>(&bytes) {
                Ok(items) => return Ok(items),
                Err(err) => last_error = Some(format!("invalid meeting state: {err}")),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => last_error = Some(format!("failed to read meeting state: {err}")),
        }
        if attempt < 2 {
            sleep(Duration::from_millis(15)).await;
        }
    }
    Err(api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        last_error.unwrap_or_else(|| "failed to read meeting state".into()),
    ))
}

fn meeting_summary(meeting: &Value) -> Value {
    let participants = meeting
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let online = participants
        .iter()
        .filter(|participant| participant.get("state").and_then(Value::as_str) == Some("online"))
        .count();
    json!({
        "meeting_id": meeting.get("id").cloned().unwrap_or(Value::Null),
        "title": meeting.pointer("/config/title").cloned().unwrap_or(Value::Null),
        "goal": meeting.pointer("/config/goal").cloned().unwrap_or(Value::Null),
        "stage": meeting.get("stage").cloned().unwrap_or(Value::Null),
        "participant_limit": meeting.pointer("/config/participant_limit").cloned().unwrap_or(Value::Null),
        "participant_count": participants.len(),
        "participants_online": online,
        "created_at": meeting.get("created_at").cloned().unwrap_or(Value::Null),
        "updated_at": meeting.get("updated_at").cloned().unwrap_or(Value::Null),
        "paused_reason": meeting.get("paused_reason").cloned().unwrap_or(Value::Null),
        "has_final": final_response(meeting).is_some()
    })
}

fn meeting_detail(meeting: &Value, owner_access: bool, endpoint: Option<String>) -> Value {
    let participants = meeting
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let public_participants: Vec<Value> = participants
        .iter()
        .map(|participant| {
            json!({
                "id": participant.get("id").cloned().unwrap_or(Value::Null),
                "display_name": participant.get("display_name").cloned().unwrap_or(Value::Null),
                "client_name": participant.get("client_name").cloned().unwrap_or(Value::Null),
                "joined_at": participant.get("joined_at").cloned().unwrap_or(Value::Null),
                "last_seen": participant.get("last_seen").cloned().unwrap_or(Value::Null),
                "state": participant.get("state").cloned().unwrap_or(Value::Null)
            })
        })
        .collect();

    let questions = meeting
        .get("questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let active_question = questions
        .iter()
        .find(|question| question.get("status").and_then(Value::as_str) == Some("open"))
        .map(public_question)
        .unwrap_or(Value::Null);

    let mut detail = Map::new();
    detail.insert("meeting_id".into(), meeting.get("id").cloned().unwrap_or(Value::Null));
    detail.insert("title".into(), meeting.pointer("/config/title").cloned().unwrap_or(Value::Null));
    detail.insert("goal".into(), meeting.pointer("/config/goal").cloned().unwrap_or(Value::Null));
    detail.insert("stage".into(), meeting.get("stage").cloned().unwrap_or(Value::Null));
    detail.insert(
        "participant_limit".into(),
        meeting
            .pointer("/config/participant_limit")
            .cloned()
            .unwrap_or(Value::Null),
    );
    detail.insert("participants".into(), json!(public_participants));
    detail.insert("active_question".into(), active_question);
    detail.insert("progress".into(), progress_summary(meeting));
    detail.insert(
        "paused_reason".into(),
        meeting.get("paused_reason").cloned().unwrap_or(Value::Null),
    );
    detail.insert(
        "created_at".into(),
        meeting.get("created_at").cloned().unwrap_or(Value::Null),
    );
    detail.insert(
        "updated_at".into(),
        meeting.get("updated_at").cloned().unwrap_or(Value::Null),
    );
    detail.insert("final".into(), final_response(meeting).unwrap_or(Value::Null));
    detail.insert("owner_access".into(), json!(owner_access));
    detail.insert("mcp_endpoint".into(), json!(endpoint));

    if owner_access {
        detail.insert(
            "context".into(),
            meeting.pointer("/config/context").cloned().unwrap_or(Value::Null),
        );
        detail.insert(
            "join_code".into(),
            meeting.get("join_code").cloned().unwrap_or(Value::Null),
        );
        detail.insert(
            "questions".into(),
            json!(questions.iter().map(owner_question).collect::<Vec<_>>()),
        );
        detail.insert("transcript".into(), json!(owner_transcript(meeting)));
    } else {
        detail.insert("context".into(), Value::Null);
        detail.insert("join_code".into(), Value::Null);
        detail.insert("questions".into(), json!([]));
        detail.insert("transcript".into(), json!([]));
    }

    Value::Object(detail)
}

fn owner_transcript(meeting: &Value) -> Vec<Value> {
    let participants = meeting
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meeting
        .get("turns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|turn| {
            let assignee = turn.get("assignee").and_then(Value::as_str).unwrap_or("");
            let display_name = participants
                .iter()
                .find(|participant| participant.get("id").and_then(Value::as_str) == Some(assignee))
                .and_then(|participant| participant.get("display_name"))
                .cloned()
                .unwrap_or_else(|| json!("Unknown agent"));
            json!({
                "id": turn.get("id").cloned().unwrap_or(Value::Null),
                "stage": turn.get("stage").cloned().unwrap_or(Value::Null),
                "assignee": turn.get("assignee").cloned().unwrap_or(Value::Null),
                "assignee_name": display_name,
                "status": turn.get("status").cloned().unwrap_or(Value::Null),
                "response": turn.get("response").cloned().unwrap_or(Value::Null),
                "skip_reason": turn.get("skip_reason").cloned().unwrap_or(Value::Null),
                "created_at": turn.get("created_at").cloned().unwrap_or(Value::Null),
                "updated_at": turn.get("updated_at").cloned().unwrap_or(Value::Null)
            })
        })
        .collect()
}

fn public_question(question: &Value) -> Value {
    json!({
        "id": question.get("id").cloned().unwrap_or(Value::Null),
        "question": question.get("question").cloned().unwrap_or(Value::Null),
        "reason": question.get("reason").cloned().unwrap_or(Value::Null),
        "missing_key": question.get("missing_key").cloned().unwrap_or(Value::Null),
        "asked_by_count": question
            .get("asked_by")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        "status": question.get("status").cloned().unwrap_or(Value::Null)
    })
}

fn owner_question(question: &Value) -> Value {
    let mut value = public_question(question);
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "answer".into(),
            question.get("answer").cloned().unwrap_or(Value::Null),
        );
        object.insert(
            "answered_at".into(),
            question.get("answered_at").cloned().unwrap_or(Value::Null),
        );
    }
    value
}

fn progress_summary(meeting: &Value) -> Value {
    let turns = meeting
        .get("turns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let questions = meeting
        .get("questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let participants = meeting
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut stages = Map::new();
    for stage in ["initial", "cross_review", "synthesis"] {
        let stage_turns: Vec<&Value> = turns
            .iter()
            .filter(|turn| turn.get("stage").and_then(Value::as_str) == Some(stage))
            .collect();
        let submitted = stage_turns
            .iter()
            .filter(|turn| turn.get("status").and_then(Value::as_str) == Some("submitted"))
            .count();
        let skipped = stage_turns
            .iter()
            .filter(|turn| turn.get("status").and_then(Value::as_str) == Some("skipped"))
            .count();
        stages.insert(
            stage.into(),
            json!({ "total": stage_turns.len(), "submitted": submitted, "skipped": skipped }),
        );
    }

    json!({
        "turns": stages,
        "participants_online": participants
            .iter()
            .filter(|participant| participant.get("state").and_then(Value::as_str) == Some("online"))
            .count(),
        "questions_open": questions
            .iter()
            .filter(|question| question.get("status").and_then(Value::as_str) == Some("open"))
            .count(),
        "questions_deferred": questions
            .iter()
            .filter(|question| question.get("status").and_then(Value::as_str) == Some("deferred"))
            .count()
    })
}

fn final_response(meeting: &Value) -> Option<Value> {
    let turns = meeting.get("turns")?.as_array()?;
    let final_turn = turns.iter().rev().find(|turn| {
        turn.get("stage").and_then(Value::as_str) == Some("synthesis")
            && turn.get("status").and_then(Value::as_str) == Some("submitted")
    })?;
    let response = final_turn.get("response")?.as_str()?;
    let skipped = turns
        .iter()
        .filter(|turn| turn.get("status").and_then(Value::as_str) == Some("skipped"))
        .count();
    Some(json!({
        "response": response,
        "synthesizer": final_turn.get("assignee").cloned().unwrap_or(Value::Null),
        "degraded": skipped > 0,
        "skipped_turns": skipped
    }))
}

fn api_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meeting() -> Value {
        json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "config": {
                "title": "Reef Expert",
                "goal": "choose implementation",
                "context": "private context",
                "participant_limit": 2
            },
            "stage": "completed",
            "join_code": "JOIN1234",
            "owner_token": "owner-secret",
            "participants": [{
                "id": "00000000-0000-0000-0000-000000000010",
                "display_name": "Agent A",
                "client_name": "Codex",
                "joined_at": "2026-09-16T00:00:00Z",
                "last_seen": "2026-09-16T00:01:00Z",
                "state": "online"
            }],
            "turns": [{
                "id": "00000000-0000-0000-0000-000000000020",
                "stage": "synthesis",
                "assignee": "00000000-0000-0000-0000-000000000010",
                "status": "submitted",
                "response": "final answer",
                "skip_reason": null,
                "created_at": "2026-09-16T00:00:30Z",
                "updated_at": "2026-09-16T00:01:00Z"
            }],
            "questions": [],
            "created_at": "2026-09-16T00:00:00Z",
            "updated_at": "2026-09-16T00:01:00Z",
            "paused_reason": null
        })
    }

    #[test]
    fn public_detail_hides_owner_secrets_and_transcript() {
        let detail = meeting_detail(&sample_meeting(), false, Some("http://127.0.0.1:3847/mcp".into()));
        assert_eq!(detail["owner_access"], false);
        assert!(detail["join_code"].is_null());
        assert!(detail["context"].is_null());
        assert_eq!(detail["transcript"].as_array().unwrap().len(), 0);
        assert_eq!(detail["final"]["response"], "final answer");
    }

    #[test]
    fn owner_detail_exposes_join_code_and_transcript_without_owner_token() {
        let detail = meeting_detail(&sample_meeting(), true, Some("http://127.0.0.1:3847/mcp".into()));
        assert_eq!(detail["join_code"], "JOIN1234");
        assert_eq!(detail["context"], "private context");
        assert_eq!(detail["transcript"].as_array().unwrap().len(), 1);
        assert!(detail.get("owner_token").is_none());
    }
}
