use std::sync::Arc;

use lorealm_storage::{Storage, models::Snowflake};

use crate::{
    errors::PermissionError,
    services::{
        permission::{ADMINISTRATOR_ROLE_ID, EVERYONE_ROLE_ID, Permissions, Role},
        users::UserService,
    },
};

pub struct PermissionService {
    storage: Arc<dyn Storage>,
    users: Arc<UserService>,
}

impl PermissionService {
    pub fn new(storage: Arc<dyn Storage>, users: Arc<UserService>) -> Self {
        Self { storage, users }
    }

    pub async fn get_user_roles(&self, user_id: Snowflake) -> Result<Vec<Role>, PermissionError> {
        Ok(self
            .storage
            .get_user_roles(user_id)
            .await?
            .iter()
            .map(|r| Role::from(r))
            .collect())
    }

    /// Everything the user may do: `@everyone` united with each role they hold.
    pub async fn resolve(&self, user_id: Snowflake) -> Result<Permissions, PermissionError> {
        let everyone = self
            .storage
            .get_role(EVERYONE_ROLE_ID)
            .await?
            .map(|r| Permissions::from_db(r.permissions))
            .unwrap_or_else(Permissions::empty);

        Ok(self
            .get_user_roles(user_id)
            .await?
            .iter()
            .fold(everyone, |acc, role| acc | role.permissions))
    }

    /// Makes the user an administrator if nobody is one yet, so the first
    /// account on a server can run it.
    pub async fn claim_administrator_if_unclaimed(
        &self,
        user_id: Snowflake,
    ) -> Result<bool, PermissionError> {
        if self.storage.count_role_members(ADMINISTRATOR_ROLE_ID).await? > 0 {
            return Ok(false);
        }
        self.storage.assign_role(user_id, ADMINISTRATOR_ROLE_ID).await?;
        Ok(true)
    }
}
