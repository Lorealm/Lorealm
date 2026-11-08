use crate::error::StorageError;
use crate::models::{CreateRole, Role, Snowflake, UpdateRole, User, UserRole};
use async_trait::async_trait;

#[async_trait]
pub trait PermissionStorage: Send + Sync {
    // Role CRUD
    async fn create_role(&self, input: &CreateRole) -> Result<Role, StorageError>;
    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, StorageError>;
    async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>, StorageError>;
    async fn get_all_roles(&self) -> Result<Vec<Role>, StorageError>;
    async fn update_role(&self, id: Snowflake, input: &UpdateRole) -> Result<Role, StorageError>;
    async fn delete_role(&self, id: Snowflake) -> Result<bool, StorageError>;

    // User ↔ Role assignment
    async fn assign_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<UserRole, StorageError>;
    async fn revoke_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<bool, StorageError>;
    async fn get_user_roles(&self, user_id: Snowflake) -> Result<Vec<Role>, StorageError>;
    /// How many users hold the role directly.
    async fn count_role_members(&self, role_id: Snowflake) -> Result<i64, StorageError>;
    /// The users who hold the role directly, oldest account first.
    async fn get_role_members(&self, role_id: Snowflake) -> Result<Vec<User>, StorageError>;
}
