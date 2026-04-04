use crate::SessionStorage;
use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::PostgresStorage;

#[async_trait]
impl SessionStorage for PostgresStorage {
    async fn create_session(&self, input: &CreateSession) -> Result<Session, StorageError> {
        let session = sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (id, user_id, token, csrf_token, ip_address, user_agent, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(&input.token)
        .bind(&input.csrf)
        .bind(&input.ip_address)
        .bind(&input.user_agent)
        .bind(input.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        let session = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE token = $1 AND expires_at > now()",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    async fn delete_session(&self, token: &str) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError> {
        let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn cleanup_expired(&self) -> Result<u64, StorageError> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
    async fn rotate_session_token(
        &self,
        session_id: Snowflake,
        new_token_hash: &str,
        old_token_hash: &str,
        grace_expires_at: DateTime<Utc>,
        new_expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET token = $1,
                previous_token = $2,
                grace_expires_at = $3,
                last_rotated_at = now(),
                expires_at = COALESCE($4, expires_at)
            WHERE id = $5
                AND revoked_at IS NULL
            "#,
        )
        .bind(new_token_hash)
        .bind(old_token_hash)
        .bind(grace_expires_at)
        .bind(new_expires_at)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Session {session_id} not found"
            )));
        }

        Ok(())
    }

    async fn extend_session(
        &self,
        session_id: Snowflake,
        new_expires_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let result =
            sqlx::query("UPDATE sessions SET expires_at = $1 WHERE id = $2 AND revoked_at IS NULL")
                .bind(new_expires_at)
                .bind(session_id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Session {session_id} not found"
            )));
        }

        Ok(())
    }

    async fn revoke_session(&self, id: Snowflake) -> Result<(), StorageError> {
        let result = sqlx::query(
            "UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Session {id} not found")));
        }

        Ok(())
    }

    async fn revoke_all_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError> {
        let result = sqlx::query(
            "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn increment_request_count(&self, session_id: Snowflake) -> Result<i32, StorageError> {
        let row = sqlx::query_scalar::<_, i32>(
            r#"
                UPDATE sessions
                SET requests_since_csrf_rotation = requests_since_csrf_rotation + 1
                WHERE id = $1 AND revoked_at IS NULL
                RETURNING requests_since_csrf_rotation
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StorageError::NotFound(format!("Session {session_id} not found")))?;

        Ok(row)
    }

    async fn rotate_csrf_token(
        &self,
        session_id: Snowflake,
        new_csrf_hash: &str,
    ) -> Result<(), StorageError> {
        let result = sqlx::query(
            r#"
                UPDATE sessions
                SET csrf_token = $1,
                    last_csrf_rotated_at = now(),
                    requests_since_csrf_rotation = 0
                WHERE id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(new_csrf_hash)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Session {session_id} not found"
            )));
        }

        Ok(())
    }
}
