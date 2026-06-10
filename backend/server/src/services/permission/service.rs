use std::sync::Arc;

use lorealm_storage::{Storage, models::Snowflake};

use crate::{
    errors::PermissionError,
    services::{permission::Role, users::UserService},
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
}
