use crate::SessionStorage;
use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;

use super::PostgresStorage;

#[async_trait]
impl SessionStorage for PostgresStorage {
    async fn create_session(&self, input: &CreateSession) -> Result<Session, StorageError> {
        let session = sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (id, user_id, token, ip_address, user_agent, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(&input.token)
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
}
