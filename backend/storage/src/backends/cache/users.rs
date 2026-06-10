use crate::UserStorage;
use crate::error::StorageError;
use crate::models::{CreateUser, Snowflake, UpdateUser, User};
use async_trait::async_trait;

use super::CachedStorage;

#[async_trait]
impl UserStorage for CachedStorage {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError> {
        let user = self.backend.create_user(input).await?;
        self.cache_set(&self.key("user", &user.id.to_string()), &user)
            .await;
        Ok(user)
    }

    async fn get_user_by_id(&self, id: Snowflake) -> Result<Option<User>, StorageError> {
        let key = self.key("user", &id.to_string());
        self.get_through(&key, self.backend.get_user_by_id(id))
            .await
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError> {
        self.backend.get_user_by_username(username).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, StorageError> {
        self.backend.get_user_by_email(email).await
    }

    async fn update_user(&self, id: Snowflake, input: &UpdateUser) -> Result<User, StorageError> {
        let user = self.backend.update_user(id, input).await?;
        self.cache_del(&self.key("user", &id.to_string())).await;
        Ok(user)
    }

    async fn delete_user(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = self.backend.delete_user(id).await?;
        if result {
            self.cache_del(&self.key("user", &id.to_string())).await;
        }
        Ok(result)
    }
}
