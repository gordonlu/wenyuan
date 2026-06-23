use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: i64,
    pub session_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
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

impl Store {
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

    pub async fn phase_trajectory(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<SessionEvent>, StoreError> {
        let rows = sqlx::query(
            "select id, session_id, event_type, payload_json, created_at
             from session_events
             where session_id = ?1
               and event_type like 'phase_%'
             order by id asc",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(event_from_row).collect()
    }
}
