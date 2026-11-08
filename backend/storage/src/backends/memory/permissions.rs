use crate::PermissionStorage;
use crate::error::StorageError;
use crate::models::{CreateRole, Role, Snowflake, UpdateRole, User, UserRole};
use async_trait::async_trait;
use chrono::Utc;

use super::MemoryStorage;

#[async_trait]
impl PermissionStorage for MemoryStorage {
    async fn create_role(&self, input: &CreateRole) -> Result<Role, StorageError> {
        let mut roles = self.roles.write().await;

        if roles.contains_key(&input.id) {
            return Err(StorageError::Conflict(format!(
                "Role {} already exists",
                input.id
            )));
        }

        if roles.values().any(|r| r.name == input.name) {
            return Err(StorageError::Conflict("Role name already taken".into()));
        }

        let role = Role {
            id: input.id,
            name: input.name.clone(),
            permissions: input.permissions,
            position: input.position,
            created_at: Utc::now(),
        };

        roles.insert(role.id, role.clone());
        Ok(role)
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, StorageError> {
        Ok(self.roles.read().await.get(&id).cloned())
    }

    async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>, StorageError> {
        Ok(self
            .roles
            .read()
            .await
            .values()
            .find(|r| r.name == name)
            .cloned())
    }

    async fn get_all_roles(&self) -> Result<Vec<Role>, StorageError> {
        let mut roles: Vec<Role> = self.roles.read().await.values().cloned().collect();
        roles.sort_by_key(|r| r.created_at);
        Ok(roles)
    }

    async fn update_role(&self, id: Snowflake, input: &UpdateRole) -> Result<Role, StorageError> {
        let mut roles = self.roles.write().await;

        let role = roles
            .get_mut(&id)
            .ok_or_else(|| StorageError::NotFound(format!("Role {id} not found")))?;

        if let Some(ref name) = input.name {
            role.name = name.clone();
        }
        if let Some(perms) = input.permissions {
            role.permissions = perms;
        }
        if let Some(position) = input.position {
            role.position = position;
        }

        Ok(role.clone())
    }

    async fn delete_role(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = self.roles.write().await.remove(&id).is_some();
        if removed {
            self.user_roles.write().await.retain(|ur| ur.role_id != id);
        }
        Ok(removed)
    }

    async fn assign_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<UserRole, StorageError> {
        let mut user_roles = self.user_roles.write().await;

        if user_roles
            .iter()
            .any(|ur| ur.user_id == user_id && ur.role_id == role_id)
        {
            return Err(StorageError::Conflict(format!(
                "User {user_id} already has role {role_id}"
            )));
        }

        let ur = UserRole {
            user_id,
            role_id,
            assigned_at: Utc::now(),
        };

        user_roles.push(ur.clone());
        Ok(ur)
    }

    async fn revoke_role(
        &self,
        user_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let mut user_roles = self.user_roles.write().await;
        let before = user_roles.len();
        user_roles.retain(|ur| !(ur.user_id == user_id && ur.role_id == role_id));
        Ok(user_roles.len() < before)
    }

    async fn get_user_roles(&self, user_id: Snowflake) -> Result<Vec<Role>, StorageError> {
        let user_roles = self.user_roles.read().await;
        let roles = self.roles.read().await;

        let result = user_roles
            .iter()
            .filter(|ur| ur.user_id == user_id)
            .filter_map(|ur| roles.get(&ur.role_id).cloned())
            .collect();

        Ok(result)
    }

    async fn count_role_members(&self, role_id: Snowflake) -> Result<i64, StorageError> {
        let user_roles = self.user_roles.read().await;
        Ok(user_roles.iter().filter(|ur| ur.role_id == role_id).count() as i64)
    }

    async fn get_role_members(&self, role_id: Snowflake) -> Result<Vec<User>, StorageError> {
        let user_roles = self.user_roles.read().await;
        let users = self.users.read().await;

        let mut members: Vec<User> = user_roles
            .iter()
            .filter(|ur| ur.role_id == role_id)
            .filter_map(|ur| users.get(&ur.user_id).cloned())
            .collect();
        members.sort_by_key(|u| (u.created_at, u.id));
        Ok(members)
    }
}
