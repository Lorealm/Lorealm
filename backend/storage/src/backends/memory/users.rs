use crate::UserStorage;
use crate::error::StorageError;
use crate::models::{CreateUser, Snowflake, UpdateUser, User};
use async_trait::async_trait;
use chrono::Utc;

use super::MemoryStorage;

#[async_trait]
impl UserStorage for MemoryStorage {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError> {
        let mut users = self.users.write().await;

        if users.contains_key(&input.id) {
            return Err(StorageError::Conflict(format!(
                "User {} already exists",
                input.id
            )));
        }

        if users.values().any(|u| u.username == input.username) {
            return Err(StorageError::Conflict("Username already taken".into()));
        }

        // Several users may have no email, as in the SQL backends' UNIQUE column.
        if input.email.is_some() && users.values().any(|u| u.email == input.email) {
            return Err(StorageError::Conflict("Email already taken".into()));
        }

        let now = Utc::now();
        let user = User {
            id: input.id,
            username: input.username.clone(),
            email: input.email.clone(),
            display_name: None,
            avatar_uri: None,
            password_hash: input.password_hash.clone(),
            created_at: now,
            updated_at: now,
        };

        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn get_user_by_id(&self, id: Snowflake) -> Result<Option<User>, StorageError> {
        Ok(self.users.read().await.get(&id).cloned())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.username == username)
            .cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, StorageError> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.email.as_deref() == Some(email))
            .cloned())
    }

    async fn update_user(&self, id: Snowflake, input: &UpdateUser) -> Result<User, StorageError> {
        let mut users = self.users.write().await;

        let user = users
            .get_mut(&id)
            .ok_or_else(|| StorageError::NotFound(format!("User {id} not found")))?;

        if let Some(ref email) = input.email {
            user.email = Some(email.clone());
        }
        if let Some(ref display_name) = input.display_name {
            user.display_name = display_name.clone();
        }
        if let Some(ref avatar_uri) = input.avatar_uri {
            user.avatar_uri = Some(avatar_uri.clone());
        }
        if let Some(ref hash) = input.password_hash {
            user.password_hash = hash.clone();
        }
        user.updated_at = Utc::now();

        Ok(user.clone())
    }

    async fn delete_user(&self, id: Snowflake) -> Result<bool, StorageError> {
        Ok(self.users.write().await.remove(&id).is_some())
    }
}
