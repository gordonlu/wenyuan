use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;
use wenyuan_agent::{DiscussionArtifacts, SeatRunStatus, SeatRunTrace, system_prompt};
use wenyuan_core::{ChatMessage, SeatKind, Session, SessionPhase};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("time parse error: {0}")]
    Time(#[from] chrono::ParseError),
    #[error("session not found")]
    NotFound,
    #[error("session is already running")]
    AlreadyRunning,
}

#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        store.ensure_session_execution_columns().await?;
        store.ensure_seat_conversation_columns().await?;
        store.ensure_seat_run_trace_columns().await?;
        Ok(store)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        for statement in MIGRATIONS
            .split(";")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        Ok(())
    }

    async fn ensure_session_execution_columns(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();

        if !columns.contains("execution_token") {
            sqlx::query("alter table sessions add column execution_token text")
                .execute(&self.pool)
                .await?;
        }
        if !columns.contains("lease_expires_at") {
            sqlx::query("alter table sessions add column lease_expires_at text")
                .execute(&self.pool)
                .await?;
        }
        if !columns.contains("recovery_state") {
            sqlx::query(
                "alter table sessions add column recovery_state text not null default 'idle'",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn ensure_seat_run_trace_columns(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(seat_runs)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        let additions = [
            (
                "prompt_version",
                "alter table seat_runs add column prompt_version text",
            ),
            (
                "repair_attempted",
                "alter table seat_runs add column repair_attempted integer not null default 0",
            ),
            (
                "duration_ms",
                "alter table seat_runs add column duration_ms text",
            ),
            (
                "prompt_tokens",
                "alter table seat_runs add column prompt_tokens integer",
            ),
            (
                "completion_tokens",
                "alter table seat_runs add column completion_tokens integer",
            ),
            (
                "total_tokens",
                "alter table seat_runs add column total_tokens integer",
            ),
            (
                "upstream_status",
                "alter table seat_runs add column upstream_status integer",
            ),
        ];
        for (column, statement) in additions {
            if !columns.contains(column) {
                sqlx::query(statement).execute(&self.pool).await?;
            }
        }
        Ok(())
    }

    async fn ensure_seat_conversation_columns(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(seats)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        let additions = [
            (
                "system_prompt",
                "alter table seats add column system_prompt text not null default ''",
            ),
            (
                "conversation_json",
                "alter table seats add column conversation_json text not null default '[]'",
            ),
            (
                "provider_ref",
                "alter table seats add column provider_ref text not null default 'default'",
            ),
        ];
        for (column, statement) in additions {
            if !columns.contains(column) {
                sqlx::query(statement).execute(&self.pool).await?;
            }
        }
        Ok(())
    }

    pub async fn create_session(&self, session: &Session) -> Result<(), StoreError> {
        self.create_session_with_provider_refs(session, &HashMap::new())
            .await
    }

    pub async fn create_session_with_provider_refs(
        &self,
        session: &Session,
        provider_refs: &HashMap<SeatKind, String>,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "insert into sessions
            (id, title, topic, context, phase, created_at, updated_at, result_json, failure_reason, convergence_used, artifacts_json, recovery_state)
            values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(session.id.to_string())
        .bind(&session.title)
        .bind(&session.topic)
        .bind(&session.context)
        .bind(phase_to_string(session.phase))
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(optional_json(&session.result)?)
        .bind(&session.failure_reason)
        .bind(session.convergence_used)
        .bind(serde_json::to_string(&DiscussionArtifacts::default())?)
        .bind("idle")
        .execute(&mut *tx)
        .await?;
        for seat in SeatKind::ALL {
            let conversation = vec![ChatMessage {
                role: "system".into(),
                content: system_prompt(seat).to_string(),
            }];
            sqlx::query(
                "insert into seats
                 (session_id, seat_kind, status, last_error, system_prompt, conversation_json, provider_ref)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(session.id.to_string())
            .bind(seat_to_string(seat))
            .bind("pending")
            .bind(Option::<String>::None)
            .bind(system_prompt(seat))
            .bind(serde_json::to_string(&conversation)?)
            .bind(
                provider_refs
                    .get(&seat)
                    .map(String::as_str)
                    .unwrap_or("default"),
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.append_event(
            session.id,
            "session_created",
            serde_json::json!({ "title": session.title }),
        )
        .await?;
        Ok(())
    }

    pub async fn update_session(&self, session: &Session) -> Result<(), StoreError> {
        sqlx::query(
            "update sessions set phase = ?2, updated_at = ?3, result_json = ?4, failure_reason = ?5, convergence_used = ?6 where id = ?1",
        )
        .bind(session.id.to_string())
        .bind(phase_to_string(session.phase))
        .bind(session.updated_at.to_rfc3339())
        .bind(optional_json(&session.result)?)
        .bind(&session.failure_reason)
        .bind(session.convergence_used)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_artifacts(
        &self,
        session_id: Uuid,
        artifacts: &DiscussionArtifacts,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("delete from ideas where session_id = ?1")
            .bind(session_id.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("delete from critiques where session_id = ?1")
            .bind(session_id.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("delete from proposals where session_id = ?1")
            .bind(session_id.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("delete from votes where session_id = ?1")
            .bind(session_id.to_string())
            .execute(&mut *tx)
            .await?;

        for idea in &artifacts.ideas {
            sqlx::query("insert into ideas (id, session_id, proposed_by, data_json) values (?1, ?2, ?3, ?4)")
                .bind(idea.id.to_string())
                .bind(session_id.to_string())
                .bind(format!("{:?}", idea.proposed_by))
                .bind(serde_json::to_string(idea)?)
                .execute(&mut *tx)
                .await?;
        }
        for critique in &artifacts.critiques {
            sqlx::query("insert into critiques (id, session_id, reviewer, data_json) values (?1, ?2, ?3, ?4)")
                .bind(Uuid::new_v4().to_string())
                .bind(session_id.to_string())
                .bind(format!("{:?}", critique.reviewer))
                .bind(serde_json::to_string(critique)?)
                .execute(&mut *tx)
                .await?;
        }
        for proposal in &artifacts.proposals {
            sqlx::query("insert into proposals (id, session_id, proposed_by, data_json) values (?1, ?2, ?3, ?4)")
                .bind(proposal.id.to_string())
                .bind(session_id.to_string())
                .bind(format!("{:?}", proposal.proposed_by))
                .bind(serde_json::to_string(proposal)?)
                .execute(&mut *tx)
                .await?;
        }
        for vote in &artifacts.votes {
            sqlx::query("insert into votes (id, session_id, voter, proposal_id, data_json) values (?1, ?2, ?3, ?4, ?5)")
                .bind(Uuid::new_v4().to_string())
                .bind(session_id.to_string())
                .bind(format!("{:?}", vote.voter))
                .bind(vote.proposal_id.to_string())
                .bind(serde_json::to_string(vote)?)
                .execute(&mut *tx)
                .await?;
        }
        insert_seat_runs(&mut tx, session_id, &artifacts.seat_runs).await?;
        update_seat_conversations(&mut tx, session_id, &artifacts.seat_runs).await?;
        let result_json = optional_json(&artifacts.decision)?;
        sqlx::query("update sessions set artifacts_json = ?2, result_json = ?3 where id = ?1")
            .bind(session_id.to_string())
            .bind(serde_json::to_string(artifacts)?)
            .bind(result_json)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.append_event(
            session_id,
            "session_completed",
            serde_json::json!({ "ok": true }),
        )
        .await?;
        Ok(())
    }

    pub async fn save_seat_runs(
        &self,
        session_id: Uuid,
        seat_runs: &[SeatRunTrace],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        insert_seat_runs(&mut tx, session_id, seat_runs).await?;
        update_seat_conversations(&mut tx, session_id, seat_runs).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn count_seat_runs(&self, session_id: Uuid) -> Result<i64, StoreError> {
        Ok(
            sqlx::query_scalar("select count(*) from seat_runs where session_id = ?1")
                .bind(session_id.to_string())
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn failed_seat_run_raw_outputs(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query(
            "select raw_output from seat_runs where session_id = ?1 and status = 'failed'",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                row.try_get::<Option<String>, _>("raw_output")
                    .ok()
                    .flatten()
            })
            .collect())
    }

    pub async fn prepare_retry(&self, session_id: Uuid) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        let empty_artifacts = serde_json::to_string(&DiscussionArtifacts::default())?;
        sqlx::query(
            "update sessions
             set phase = 'draft',
                 result_json = null,
                 failure_reason = null,
                 convergence_used = 0,
                 artifacts_json = ?2,
                 execution_token = null,
                 lease_expires_at = null,
                 recovery_state = 'idle',
                 updated_at = ?3
             where id = ?1",
        )
        .bind(session_id.to_string())
        .bind(empty_artifacts)
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await?;
        for table in ["ideas", "critiques", "proposals", "votes"] {
            let statement = format!("delete from {table} where session_id = ?1");
            sqlx::query(&statement)
                .bind(session_id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        self.append_event(session_id, "session_retry_prepared", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn try_acquire_execution(
        &self,
        session_id: Uuid,
        lease_seconds: i64,
    ) -> Result<Option<Uuid>, StoreError> {
        let token = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::seconds(lease_seconds);
        let result = sqlx::query(
            "update sessions
             set execution_token = ?2, lease_expires_at = ?3, recovery_state = 'running', updated_at = ?4
             where id = ?1
               and phase not in ('completed', 'failed', 'cancelled')
               and (execution_token is null or lease_expires_at is null or lease_expires_at < ?4)",
        )
        .bind(session_id.to_string())
        .bind(token.to_string())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.append_event(
            session_id,
            "execution_acquired",
            serde_json::json!({ "token": token, "lease_expires_at": expires_at }),
        )
        .await?;
        Ok(Some(token))
    }

    pub async fn is_execution_active(
        &self,
        session_id: Uuid,
        token: Uuid,
    ) -> Result<bool, StoreError> {
        let count: i64 = sqlx::query_scalar(
            "select count(*) from sessions where id = ?1 and execution_token = ?2",
        )
        .bind(session_id.to_string())
        .bind(token.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(count == 1)
    }

    pub async fn complete_execution(
        &self,
        session_id: Uuid,
        token: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "update sessions
             set execution_token = null, lease_expires_at = null, recovery_state = 'completed'
             where id = ?1 and execution_token = ?2",
        )
        .bind(session_id.to_string())
        .bind(token.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn fail_session(
        &self,
        session_id: Uuid,
        token: Option<Uuid>,
        reason: &str,
    ) -> Result<(), StoreError> {
        let now = Utc::now();
        let mut query = sqlx::query(
            "update sessions
             set phase = 'failed',
                 failure_reason = ?2,
                 updated_at = ?3,
                 execution_token = null,
                 lease_expires_at = null,
                 recovery_state = 'failed'
             where id = ?1",
        )
        .bind(session_id.to_string())
        .bind(reason)
        .bind(now.to_rfc3339());

        if let Some(token) = token {
            query = sqlx::query(
                "update sessions
                 set phase = 'failed',
                     failure_reason = ?3,
                     updated_at = ?4,
                     execution_token = null,
                     lease_expires_at = null,
                     recovery_state = 'failed'
                 where id = ?1 and execution_token = ?2",
            )
            .bind(session_id.to_string())
            .bind(token.to_string())
            .bind(reason)
            .bind(now.to_rfc3339());
        }

        query.execute(&self.pool).await?;
        self.append_event(
            session_id,
            "session_failed",
            serde_json::json!({ "error": reason }),
        )
        .await?;
        Ok(())
    }

    pub async fn recover_stale_executions(&self) -> Result<usize, StoreError> {
        let now = Utc::now().to_rfc3339();
        let stale_rows = sqlx::query(
            "select id from sessions
             where execution_token is not null
               and lease_expires_at is not null
               and lease_expires_at < ?1
               and phase not in ('completed', 'failed', 'cancelled')",
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;
        let ids: Vec<Uuid> = stale_rows
            .into_iter()
            .filter_map(|row| {
                row.try_get::<String, _>("id")
                    .ok()
                    .and_then(|id| Uuid::parse_str(&id).ok())
            })
            .collect();

        if ids.is_empty() {
            return Ok(0);
        }

        sqlx::query(
            "update sessions
             set execution_token = null, lease_expires_at = null, recovery_state = 'retry_required'
             where execution_token is not null
               and lease_expires_at is not null
               and lease_expires_at < ?1
               and phase not in ('completed', 'failed', 'cancelled')",
        )
        .bind(&now)
        .execute(&self.pool)
        .await?;

        for id in &ids {
            self.append_event(
                *id,
                "execution_recovery_required",
                serde_json::json!({ "reason": "lease_expired" }),
            )
            .await?;
        }

        Ok(ids.len())
    }

    pub async fn execution_info(&self, session_id: Uuid) -> Result<ExecutionInfo, StoreError> {
        let row = sqlx::query(
            "select execution_token, lease_expires_at, recovery_state from sessions where id = ?1",
        )
        .bind(session_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        Ok(ExecutionInfo {
            running: row
                .try_get::<Option<String>, _>("execution_token")?
                .is_some(),
            lease_expires_at: row.try_get("lease_expires_at")?,
            recovery_state: row.try_get("recovery_state")?,
        })
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StoreError> {
        let rows = sqlx::query(
            "select id, title, phase, created_at, updated_at, result_json from sessions order by created_at desc",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(summary_from_row).collect()
    }

    pub async fn get_session(&self, id: Uuid) -> Result<SessionDetails, StoreError> {
        let row = sqlx::query("select * from sessions where id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::NotFound)?;
        let artifacts_json: Option<String> = row.try_get("artifacts_json")?;
        Ok(SessionDetails {
            session: session_from_row(&row)?,
            artifacts: artifacts_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default(),
            seats: self.seats(id).await?,
            execution: self.execution_info(id).await?,
            events: self.events(id).await?,
        })
    }

    pub async fn seats(&self, session_id: Uuid) -> Result<Vec<SeatRecord>, StoreError> {
        let rows = sqlx::query(
            "select session_id, seat_kind, status, last_error, system_prompt, conversation_json, provider_ref
             from seats where session_id = ?1 order by seat_kind asc",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(seat_from_row).collect()
    }

    pub async fn cancel_session(&self, id: Uuid) -> Result<(), StoreError> {
        let now = Utc::now();
        sqlx::query(
            "update sessions
             set phase = ?2,
                 updated_at = ?3,
                 execution_token = null,
                 lease_expires_at = null,
                 recovery_state = 'cancelled'
             where id = ?1",
        )
        .bind(id.to_string())
        .bind(phase_to_string(SessionPhase::Cancelled))
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        self.append_event(id, "session_cancelled", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn append_event(
        &self,
        session_id: Uuid,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "insert into session_events (session_id, event_type, payload_json, created_at) values (?1, ?2, ?3, ?4)",
        )
        .bind(session_id.to_string())
        .bind(event_type)
        .bind(payload.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn events(&self, session_id: Uuid) -> Result<Vec<SessionEvent>, StoreError> {
        let rows = sqlx::query(
            "select id, session_id, event_type, payload_json, created_at from session_events where session_id = ?1 order by id asc",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(event_from_row).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub title: String,
    pub phase: SessionPhase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub has_majority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetails {
    pub session: Session,
    pub artifacts: DiscussionArtifacts,
    pub seats: Vec<SeatRecord>,
    pub execution: ExecutionInfo,
    pub events: Vec<SessionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatRecord {
    pub session_id: Uuid,
    pub seat: SeatKind,
    pub status: String,
    pub last_error: Option<String>,
    pub system_prompt: String,
    pub conversation: Vec<ChatMessage>,
    pub provider_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub running: bool,
    pub lease_expires_at: Option<String>,
    pub recovery_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: i64,
    pub session_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

const MIGRATIONS: &str = r#"
create table if not exists sessions (
    id text primary key,
    title text not null,
    topic text not null,
    context text not null,
    phase text not null,
    created_at text not null,
    updated_at text not null,
    result_json text,
    failure_reason text,
    convergence_used integer not null default 0,
    artifacts_json text,
    execution_token text,
    lease_expires_at text,
    recovery_state text not null default 'idle'
);
create table if not exists seats (
    session_id text not null,
    seat_kind text not null,
    status text not null,
    last_error text,
    system_prompt text not null default '',
    conversation_json text not null default '[]',
    provider_ref text not null default 'default',
    primary key (session_id, seat_kind)
);
create table if not exists seat_runs (
    id text primary key,
    session_id text not null,
    seat_kind text not null,
    phase text not null,
    status text not null,
    raw_output text,
    error text,
    prompt_version text,
    repair_attempted integer not null default 0,
    duration_ms text,
    prompt_tokens integer,
    completion_tokens integer,
    total_tokens integer,
    upstream_status integer
);
create table if not exists rounds (
    id text primary key,
    session_id text not null,
    phase text not null,
    round_index integer not null
);
create table if not exists ideas (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);
create table if not exists critiques (
    id text primary key,
    session_id text not null,
    reviewer text not null,
    data_json text not null
);
create table if not exists proposals (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);
create table if not exists votes (
    id text primary key,
    session_id text not null,
    voter text not null,
    proposal_id text not null,
    data_json text not null
);
create table if not exists session_events (
    id integer primary key autoincrement,
    session_id text not null,
    event_type text not null,
    payload_json text not null,
    created_at text not null
);
"#;

fn optional_json<T: Serialize>(value: &Option<T>) -> Result<Option<String>, serde_json::Error> {
    value.as_ref().map(serde_json::to_string).transpose()
}

fn phase_to_string(phase: SessionPhase) -> &'static str {
    match phase {
        SessionPhase::Draft => "draft",
        SessionPhase::IndependentDeliberation => "independent_deliberation",
        SessionPhase::CrossCritique => "cross_critique",
        SessionPhase::Revision => "revision",
        SessionPhase::Voting => "voting",
        SessionPhase::Convergence => "convergence",
        SessionPhase::Completed => "completed",
        SessionPhase::Failed => "failed",
        SessionPhase::Cancelled => "cancelled",
    }
}

fn parse_phase(value: &str) -> SessionPhase {
    match value {
        "independent_deliberation" => SessionPhase::IndependentDeliberation,
        "cross_critique" => SessionPhase::CrossCritique,
        "revision" => SessionPhase::Revision,
        "voting" => SessionPhase::Voting,
        "convergence" => SessionPhase::Convergence,
        "completed" => SessionPhase::Completed,
        "failed" => SessionPhase::Failed,
        "cancelled" => SessionPhase::Cancelled,
        _ => SessionPhase::Draft,
    }
}

fn seat_to_string(seat: SeatKind) -> &'static str {
    match seat {
        SeatKind::Mouyuan => "mouyuan",
        SeatKind::Jingshi => "jingshi",
        SeatKind::Chizheng => "chizheng",
    }
}

fn parse_seat(value: &str) -> SeatKind {
    match value {
        "jingshi" | "Jingshi" => SeatKind::Jingshi,
        "chizheng" | "Chizheng" => SeatKind::Chizheng,
        _ => SeatKind::Mouyuan,
    }
}

fn parse_time(value: String) -> Result<DateTime<Utc>, StoreError> {
    Ok(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc))
}

fn session_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Session, StoreError> {
    let result_json: Option<String> = row.try_get("result_json")?;
    Ok(Session {
        id: Uuid::parse_str(&row.try_get::<String, _>("id")?)
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
        title: row.try_get("title")?,
        topic: row.try_get("topic")?,
        context: row.try_get("context")?,
        phase: parse_phase(&row.try_get::<String, _>("phase")?),
        created_at: parse_time(row.try_get("created_at")?)?,
        updated_at: parse_time(row.try_get("updated_at")?)?,
        result: result_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?,
        failure_reason: row.try_get("failure_reason")?,
        convergence_used: row.try_get("convergence_used")?,
    })
}

fn summary_from_row(row: sqlx::sqlite::SqliteRow) -> Result<SessionSummary, StoreError> {
    let result_json: Option<String> = row.try_get("result_json")?;
    Ok(SessionSummary {
        id: Uuid::parse_str(&row.try_get::<String, _>("id")?)
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
        title: row.try_get("title")?,
        phase: parse_phase(&row.try_get::<String, _>("phase")?),
        created_at: parse_time(row.try_get("created_at")?)?,
        updated_at: parse_time(row.try_get("updated_at")?)?,
        has_majority: result_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.get("status").cloned())
            .is_some_and(|status| status == "majority_reached"),
    })
}

fn event_from_row(row: sqlx::sqlite::SqliteRow) -> Result<SessionEvent, StoreError> {
    let payload_json: String = row.try_get("payload_json")?;
    Ok(SessionEvent {
        id: row.try_get("id")?,
        session_id: Uuid::parse_str(&row.try_get::<String, _>("session_id")?)
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
        event_type: row.try_get("event_type")?,
        payload: serde_json::from_str(&payload_json)?,
        created_at: parse_time(row.try_get("created_at")?)?,
    })
}

fn seat_from_row(row: sqlx::sqlite::SqliteRow) -> Result<SeatRecord, StoreError> {
    let conversation_json: String = row.try_get("conversation_json")?;
    Ok(SeatRecord {
        session_id: Uuid::parse_str(&row.try_get::<String, _>("session_id")?)
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
        seat: parse_seat(&row.try_get::<String, _>("seat_kind")?),
        status: row.try_get("status")?,
        last_error: row.try_get("last_error")?,
        system_prompt: row.try_get("system_prompt")?,
        conversation: serde_json::from_str(&conversation_json)?,
        provider_ref: row.try_get("provider_ref")?,
    })
}

async fn insert_seat_runs(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    session_id: Uuid,
    seat_runs: &[SeatRunTrace],
) -> Result<(), StoreError> {
    for run in seat_runs {
        sqlx::query(
            "insert into seat_runs
             (id, session_id, seat_kind, phase, status, raw_output, error, prompt_version, repair_attempted, duration_ms, prompt_tokens, completion_tokens, total_tokens, upstream_status)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(run.id.to_string())
        .bind(session_id.to_string())
        .bind(format!("{:?}", run.seat))
        .bind(phase_to_string(run.phase))
        .bind(match run.status {
            SeatRunStatus::Completed => "completed",
            SeatRunStatus::Failed => "failed",
        })
        .bind(&run.raw_output)
        .bind(&run.error)
        .bind(&run.prompt_version)
        .bind(run.repair_attempted)
        .bind(run.duration_ms.to_string())
        .bind(run.prompt_tokens.map(i64::from))
        .bind(run.completion_tokens.map(i64::from))
        .bind(run.total_tokens.map(i64::from))
        .bind(run.upstream_status.map(i64::from))
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn update_seat_conversations(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    session_id: Uuid,
    seat_runs: &[SeatRunTrace],
) -> Result<(), StoreError> {
    for seat in SeatKind::ALL {
        let scoped: Vec<_> = seat_runs.iter().filter(|run| run.seat == seat).collect();
        if scoped.is_empty() {
            continue;
        }

        let mut conversation = vec![ChatMessage {
            role: "system".into(),
            content: system_prompt(seat).to_string(),
        }];
        let mut status = "completed";
        let mut last_error = None;

        for run in scoped {
            conversation.push(ChatMessage {
                role: "user".into(),
                content: format!(
                    "请执行 {:?} 阶段并只返回 JSON。{}",
                    run.phase,
                    if run.repair_attempted {
                        "这是格式修复请求。"
                    } else {
                        ""
                    }
                ),
            });
            if let Some(raw_output) = &run.raw_output {
                conversation.push(ChatMessage {
                    role: "assistant".into(),
                    content: raw_output.clone(),
                });
            }
            if run.status == SeatRunStatus::Failed {
                status = "failed";
                last_error = run.error.clone();
            }
        }

        sqlx::query(
            "update seats
             set status = ?3, last_error = ?4, conversation_json = ?5
             where session_id = ?1 and seat_kind = ?2",
        )
        .bind(session_id.to_string())
        .bind(seat_to_string(seat))
        .bind(status)
        .bind(last_error)
        .bind(serde_json::to_string(&conversation)?)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wenyuan_agent::{AgentRunner, CancellationFlag};
    use wenyuan_core::Session;
    use wenyuan_provider::{MockProvider, MockScenario};

    #[tokio::test]
    async fn sqlite_persists_and_reloads_session() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "是否加入多 Agent 讨论", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        let runner = AgentRunner::new(Arc::new(MockProvider::new(MockScenario::SuccessMajority)));
        let artifacts = runner
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap();
        store.save_artifacts(id, &artifacts).await.unwrap();
        let details = store.get_session(id).await.unwrap();
        assert_eq!(details.artifacts.ideas.len(), 6);
        assert!(details.artifacts.seat_runs.len() >= 12);
        assert_eq!(
            store.count_seat_runs(id).await.unwrap(),
            details.artifacts.seat_runs.len() as i64
        );
        assert!(details.session.result.is_some());
        assert!(
            details
                .events
                .iter()
                .any(|e| e.event_type == "session_completed")
        );
    }

    #[tokio::test]
    async fn execution_lease_blocks_duplicate_start() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "检查重复启动", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();

        let first = store.try_acquire_execution(id, 60).await.unwrap();
        let second = store.try_acquire_execution(id, 60).await.unwrap();

        assert!(first.is_some());
        assert!(second.is_none());
        assert!(store.execution_info(id).await.unwrap().running);
    }

    #[tokio::test]
    async fn create_session_initializes_independent_seat_conversations() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "三席独立会话", "");
        let id = session.id;

        store.create_session(&session).await.unwrap();
        let details = store.get_session(id).await.unwrap();

        assert_eq!(details.seats.len(), 3);
        for seat in SeatKind::ALL {
            let record = details
                .seats
                .iter()
                .find(|record| record.seat == seat)
                .unwrap();
            assert_eq!(record.status, "pending");
            assert_eq!(record.provider_ref, "default");
            assert_eq!(record.conversation.len(), 1);
            assert_eq!(record.conversation[0].role, "system");
            assert_eq!(record.conversation[0].content, system_prompt(seat));
        }
    }

    #[tokio::test]
    async fn create_session_persists_seat_provider_refs() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "每席模型配置", "");
        let mut refs = HashMap::new();
        refs.insert(SeatKind::Mouyuan, "openai-compatible:model-a".to_string());
        refs.insert(SeatKind::Jingshi, "openai-compatible:model-b".to_string());

        store
            .create_session_with_provider_refs(&session, &refs)
            .await
            .unwrap();
        let details = store.get_session(session.id).await.unwrap();

        let mouyuan = details
            .seats
            .iter()
            .find(|seat| seat.seat == SeatKind::Mouyuan)
            .unwrap();
        let jingshi = details
            .seats
            .iter()
            .find(|seat| seat.seat == SeatKind::Jingshi)
            .unwrap();
        let chizheng = details
            .seats
            .iter()
            .find(|seat| seat.seat == SeatKind::Chizheng)
            .unwrap();

        assert_eq!(mouyuan.provider_ref, "openai-compatible:model-a");
        assert_eq!(jingshi.provider_ref, "openai-compatible:model-b");
        assert_eq!(chizheng.provider_ref, "default");
    }

    #[tokio::test]
    async fn save_artifacts_updates_independent_seat_conversation_history() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "记录上下文历史", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        let runner = AgentRunner::new(Arc::new(MockProvider::new(MockScenario::SuccessMajority)));
        let artifacts = runner
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap();

        store.save_artifacts(id, &artifacts).await.unwrap();
        let details = store.get_session(id).await.unwrap();

        for seat in SeatKind::ALL {
            let record = details
                .seats
                .iter()
                .find(|record| record.seat == seat)
                .unwrap();
            assert!(record.conversation.len() > 2);
            assert!(
                record
                    .conversation
                    .iter()
                    .any(|message| message.role == "assistant")
            );
        }
    }

    #[tokio::test]
    async fn concurrent_execution_acquire_allows_only_one_winner() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "并发推进同一 Session", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();

        let left = {
            let store = store.clone();
            tokio::spawn(async move { store.try_acquire_execution(id, 60).await.unwrap() })
        };
        let right = {
            let store = store.clone();
            tokio::spawn(async move { store.try_acquire_execution(id, 60).await.unwrap() })
        };
        let results = vec![left.await.unwrap(), right.await.unwrap()];

        assert_eq!(results.into_iter().filter(Option::is_some).count(), 1);
    }

    #[tokio::test]
    async fn stale_execution_is_marked_retry_required() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "服务重启恢复", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        assert!(store.try_acquire_execution(id, -1).await.unwrap().is_some());

        let recovered = store.recover_stale_executions().await.unwrap();
        let info = store.execution_info(id).await.unwrap();

        assert_eq!(recovered, 1);
        assert!(!info.running);
        assert_eq!(info.recovery_state, "retry_required");
        assert!(
            store
                .events(id)
                .await
                .unwrap()
                .iter()
                .any(|event| event.event_type == "execution_recovery_required")
        );
    }

    #[tokio::test]
    async fn failure_marks_session_failed_and_clears_lease() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "失败落库", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        let token = store.try_acquire_execution(id, 60).await.unwrap().unwrap();

        store
            .fail_session(id, Some(token), "mock failure")
            .await
            .unwrap();
        let details = store.get_session(id).await.unwrap();

        assert_eq!(details.session.phase, SessionPhase::Failed);
        assert_eq!(
            details.session.failure_reason.as_deref(),
            Some("mock failure")
        );
        assert!(!details.execution.running);
    }

    #[tokio::test]
    async fn cancel_clears_active_execution() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "取消清理 lease", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        assert!(store.try_acquire_execution(id, 60).await.unwrap().is_some());

        store.cancel_session(id).await.unwrap();
        let details = store.get_session(id).await.unwrap();

        assert_eq!(details.session.phase, SessionPhase::Cancelled);
        assert!(!details.execution.running);
        assert_eq!(details.execution.recovery_state, "cancelled");
    }

    #[tokio::test]
    async fn failed_raw_outputs_can_be_persisted_for_diagnostics() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "保存失败原始输出", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        let runner = AgentRunner::new(Arc::new(MockProvider::new(MockScenario::AlwaysMalformed)));
        let err = runner
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap_err();
        let wenyuan_agent::AgentError::PhaseFailed { traces, .. } = err else {
            panic!("expected phase failure");
        };

        store.save_seat_runs(id, &traces).await.unwrap();
        let raw_outputs = store.failed_seat_run_raw_outputs(id).await.unwrap();

        assert!(raw_outputs.iter().any(|raw| raw == "{ broken json"));
    }

    #[tokio::test]
    async fn retry_preparation_resets_failed_session_but_keeps_seat_run_history() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let session = Session::new("议题", "重试失败 Session", "");
        let id = session.id;
        store.create_session(&session).await.unwrap();
        let token = store.try_acquire_execution(id, 60).await.unwrap().unwrap();
        store
            .fail_session(id, Some(token), "first run failed")
            .await
            .unwrap();

        let runner = AgentRunner::new(Arc::new(MockProvider::new(MockScenario::AlwaysMalformed)));
        let err = runner
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap_err();
        let wenyuan_agent::AgentError::PhaseFailed { traces, .. } = err else {
            panic!("expected phase failure");
        };
        store.save_seat_runs(id, &traces).await.unwrap();
        let previous_runs = store.count_seat_runs(id).await.unwrap();

        store.prepare_retry(id).await.unwrap();
        let details = store.get_session(id).await.unwrap();

        assert_eq!(details.session.phase, SessionPhase::Draft);
        assert!(details.session.failure_reason.is_none());
        assert_eq!(store.count_seat_runs(id).await.unwrap(), previous_runs);
        assert!(store.try_acquire_execution(id, 60).await.unwrap().is_some());
    }
}
