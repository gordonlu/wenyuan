use chrono::Utc;
use sqlx::Row;
use crate::{Store, StoreError};

const MIGRATIONS: &str = r#"
create table if not exists sessions (
    id text primary key,
    title text not null,
    topic text not null,
    context text not null,
    mode text not null default 'three_seat',
    phase text not null,
    created_at text not null,
    updated_at text not null,
    result_json text,
    failure_reason text,
    convergence_used integer not null default 0,
    artifacts_json text,
    execution_token text,
    lease_expires_at text,
    recovery_state text not null default 'idle',
    external_evidence_json text not null default '[]',
    external_tool_runs_json text not null default '[]'
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
create table if not exists claims (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);
create table if not exists evidence (
    id text primary key,
    session_id text not null,
    proposed_by text not null,
    data_json text not null
);
create table if not exists assessments (
    id text primary key,
    session_id text not null,
    assessor text not null,
    data_json text not null
);
create table if not exists claim_evidence_links (
    claim_id text not null,
    evidence_id text not null,
    session_id text not null,
    link_type text not null,
    primary key (claim_id, evidence_id)
);
"#;

impl Store {
    pub async fn migrate(&self) -> Result<(), StoreError> {
        // Track applied migrations so schema changes are explicit and auditable.
        sqlx::query(
            "create table if not exists _schema_version (
                version integer primary key,
                name text not null,
                applied_at text not null
            )",
        )
        .execute(&self.pool)
        .await?;

        let applied: i32 = sqlx::query_scalar("select count(*) from _schema_version")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        if applied == 0 {
            for statement in MIGRATIONS
                .split(";")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                sqlx::query(statement).execute(&self.pool).await?;
            }
            let now = Utc::now().to_rfc3339();
            sqlx::query("insert into _schema_version (version, name, applied_at) values (1, '001_initial', ?1)")
                .bind(&now)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_session_execution_columns(&self) -> Result<(), StoreError> {
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

    pub(crate) async fn ensure_seat_run_trace_columns(&self) -> Result<(), StoreError> {
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

    pub(crate) async fn ensure_seat_conversation_columns(&self) -> Result<(), StoreError> {
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

    pub(crate) async fn ensure_model_config_column(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        if !columns.contains("model_config") {
            sqlx::query("alter table sessions add column model_config text")
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_vote_policy_column(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        if !columns.contains("vote_policy") {
            sqlx::query("alter table sessions add column vote_policy text")
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_scribe_enabled_column(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        if !columns.contains("scribe_enabled") {
            sqlx::query(
                "alter table sessions add column scribe_enabled integer not null default 0",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_search_enabled_column(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        if !columns.contains("search_enabled") {
            sqlx::query(
                "alter table sessions add column search_enabled integer not null default 0",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_external_evidence_column(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        if !columns.contains("external_evidence_json") {
            sqlx::query(
                "alter table sessions add column external_evidence_json text not null default '[]'",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_external_tool_runs_column(&self) -> Result<(), StoreError> {
        let rows = sqlx::query("pragma table_info(sessions)")
            .fetch_all(&self.pool)
            .await?;
        let columns: std::collections::HashSet<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();
        if !columns.contains("external_tool_runs_json") {
            sqlx::query(
                "alter table sessions add column external_tool_runs_json text not null default '[]'",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}
