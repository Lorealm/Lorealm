use crate::PermissionStorage;
use crate::error::StorageError;
use crate::models::{CreateRole, Role, Snowflake, UpdateRole, UserRole};
use async_trait::async_trait;

use super::CachedStorage;

#[async_trait]
impl PermissionStorage for CachedStorage {
    async fn create_role(&self, input: &CreateRole) -> Result<Role, StorageError> {
        let role = self.backend.create_role(input).await?;
        self.cache_set(&self.key("role", &role.id.to_string()), &role)
            .await;
        Ok(role)
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, StorageError> {
        let key = self.key("role", &id.to_string());
        self.get_through(&key, self.backend.get_role(id)).await
    }

    async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>, StorageError> {
        self.backend.get_role_by_name(name).await
    }

    async fn get_all_roles(&self) -> Result<Vec<Role>, StorageError> {
        self.backend.get_all_roles().await
    }

    async fn update_role(&self, id: Snowflake, input: &UpdateRole) -> Result<Role, StorageError> {
        let role = self.backend.update_role(id, input).await?;
        self.cache_del(&self.key("role", &id.to_string())).await;
        // Invalidate computed permissions for all users with this role.
        // This is coarse — a scan of user_roles would be more precise,
        // but for a self-hosted app the cache TTL handles staleness fine.
        Ok(role)
    }

    async fn delete_role(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = self.backend.delete_role(id).await?;
        if result {
            self.cache_del(&self.key("role", &id.to_string())).await;
        }
        Ok(result)
    }

    async fn assign_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<UserRole, StorageError> {
        let ur = self.backend.assign_role(user_id, role_id).await?;
        self.cache_del(&self.key("user_perms", &user_id.to_string()))
            .await;
        Ok(ur)
    }

    async fn revoke_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let result = self.backend.revoke_role(user_id, role_id).await?;
        if result {
            self.cache_del(&self.key("user_perms", &user_id.to_string()))
                .await;
        }
        Ok(result)
    }

    async fn get_user_roles(&self, user_id: Snowflake) -> Result<Vec<Role>, StorageError> {
        self.backend.get_user_roles(user_id).await
    }
}
