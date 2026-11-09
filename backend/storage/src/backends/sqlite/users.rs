use crate::UserStorage;
use crate::error::StorageError;
use crate::models::{CreateInvite, CreateUser, Invite, Snowflake, UpdateUser, User};
use async_trait::async_trait;
use chrono::Utc;

use super::{NOW, SqliteStorage};

#[async_trait]
impl UserStorage for SqliteStorage {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError> {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, email, password_hash)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(input.id)
        .bind(&input.username)
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

        let user = sqlx::query_as::<_, User>(&format!(
            "UPDATE users
                         SET email = $2,
                             display_name = $3,
                             avatar_uri = $4,
                             password_hash = $5,
                             updated_at = {NOW}
                         WHERE id = $1
                         RETURNING *"
        ))
        .bind(id)
        .bind(input.email.as_deref().or(current.email.as_deref()))
        .bind(
            input
                .display_name
                .as_ref()
                .map_or(current.display_name.as_deref(), |name| name.as_deref()),
        )
        .bind(
            input
                .avatar_uri
                .as_deref()
                .or(current.avatar_uri.as_deref()),
        )
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

    async fn list_users(&self) -> Result<Vec<User>, StorageError> {
        let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at, id")
            .fetch_all(&self.pool)
            .await?;

        Ok(users)
    }

    async fn create_invite(&self, input: &CreateInvite) -> Result<Invite, StorageError> {
        let invite = sqlx::query_as::<_, Invite>(
            "INSERT INTO invites (id, code_hash, role_id, created_by, max_uses, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(input.id)
        .bind(&input.code_hash)
        .bind(input.role_id)
        .bind(input.created_by)
        .bind(input.max_uses)
        .bind(input.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(invite)
    }

    async fn list_invites(&self) -> Result<Vec<Invite>, StorageError> {
        let invites =
            sqlx::query_as::<_, Invite>("SELECT * FROM invites ORDER BY created_at DESC, id DESC")
                .fetch_all(&self.pool)
                .await?;

        Ok(invites)
    }

    async fn delete_invite(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM invites WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn redeem_invite(&self, code_hash: &str) -> Result<Option<Invite>, StorageError> {
        let invite = sqlx::query_as::<_, Invite>(
            "UPDATE invites SET uses = uses + 1
             WHERE code_hash = $1
               AND (max_uses IS NULL OR uses < max_uses)
               AND (expires_at IS NULL OR expires_at > $2)
             RETURNING *",
        )
        .bind(code_hash)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;

        Ok(invite)
    }

    async fn release_invite(&self, id: Snowflake) -> Result<(), StorageError> {
        sqlx::query("UPDATE invites SET uses = uses - 1 WHERE id = $1 AND uses > 0")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
