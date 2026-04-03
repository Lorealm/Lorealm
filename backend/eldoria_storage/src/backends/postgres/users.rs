use crate::error::StorageError;
use crate::models::{CreateUser, Snowflake, UpdateUser, User};
use crate::UserStorage;
use async_trait::async_trait;

use super::PostgresStorage;

#[async_trait]
impl UserStorage for PostgresStorage {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError> {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, display_name, email, password_hash)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(input.id)
        .bind(&input.username)
        .bind(&input.display_name)
        .bind(&input.email)
        .bind(&input.password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_id(&self, id: Snowflake) -> Result<Option<User>, StorageError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, StorageError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    async fn update_user(&self, id: Snowflake, input: &UpdateUser) -> Result<User, StorageError> {
        let current = self
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("User {id} not found")))?;

        let user = sqlx::query_as::<_, User>(
            "UPDATE users
             SET display_name = $2, email = $3, password_hash = $4, updated_at = now()
             WHERE id = $1
             RETURNING *",
        )
        .bind(id)
        .bind(
            input
                .display_name
                .as_deref()
                .unwrap_or(&current.display_name),
        )
        .bind(input.email.as_deref().unwrap_or(&current.email))
        .bind(
            input
                .password_hash
                .as_deref()
                .unwrap_or(&current.password_hash),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn delete_user(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
