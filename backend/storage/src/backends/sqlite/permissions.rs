use crate::PermissionStorage;
use crate::error::StorageError;
use crate::models::{CreateRole, Role, Snowflake, UpdateRole, User, UserRole};
use async_trait::async_trait;

use super::SqliteStorage;

#[async_trait]
impl PermissionStorage for SqliteStorage {
    async fn create_role(&self, input: &CreateRole) -> Result<Role, StorageError> {
        let role = sqlx::query_as::<_, Role>(
            "INSERT INTO roles (id, name, permissions, position)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(input.id)
        .bind(&input.name)
        .bind(input.permissions)
        .bind(input.position)
        .fetch_one(&self.pool)
        .await?;

        Ok(role)
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, StorageError> {
        let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(role)
    }

    async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>, StorageError> {
        let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(role)
    }

    async fn get_all_roles(&self) -> Result<Vec<Role>, StorageError> {
        let roles = sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?;

        Ok(roles)
    }

    async fn update_role(&self, id: Snowflake, input: &UpdateRole) -> Result<Role, StorageError> {
        let current = self
            .get_role(id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Role {id} not found")))?;

        let role = sqlx::query_as::<_, Role>(
            "UPDATE roles SET name = $2, permissions = $3, position = $4 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(input.name.as_deref().unwrap_or(&current.name))
        .bind(input.permissions.unwrap_or(current.permissions))
        .bind(input.position.unwrap_or(current.position))
        .fetch_one(&self.pool)
        .await?;

        Ok(role)
    }

    async fn delete_role(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM roles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn assign_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<UserRole, StorageError> {
        let user_role = sqlx::query_as::<_, UserRole>(
            "INSERT INTO user_roles (user_id, role_id)
             VALUES ($1, $2)
             ON CONFLICT (user_id, role_id) DO NOTHING
             RETURNING *",
        )
        .bind(user_id)
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            StorageError::Conflict(format!("User {user_id} already has role {role_id}"))
        })?;

        Ok(user_role)
    }

    async fn revoke_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2")
            .bind(user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_user_roles(&self, user_id: Snowflake) -> Result<Vec<Role>, StorageError> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT r.* FROM roles r
             INNER JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(roles)
    }

    async fn count_role_members(&self, role_id: Snowflake) -> Result<i64, StorageError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_roles WHERE role_id = $1")
            .bind(role_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    async fn get_role_members(&self, role_id: Snowflake) -> Result<Vec<User>, StorageError> {
        let users = sqlx::query_as::<_, User>(
            "SELECT u.* FROM users u
             INNER JOIN user_roles ur ON ur.user_id = u.id
             WHERE ur.role_id = $1
             ORDER BY u.created_at, u.id",
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}
