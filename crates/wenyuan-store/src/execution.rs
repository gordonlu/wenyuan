use super::*;

impl Store {
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
}
