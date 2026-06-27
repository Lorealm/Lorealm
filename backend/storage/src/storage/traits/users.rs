use crate::error::StorageError;
use crate::models::{CreateUser, Snowflake, UpdateUser, User};
use async_trait::async_trait;

#[async_trait]
pub trait UserStorage: Send + Sync {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError>;
    async fn get_user_by_id(&self, id: Snowflake) -> Result<Option<User>, StorageError>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, StorageError>;
    async fn update_user(&self, id: Snowflake, input: &UpdateUser) -> Result<User, StorageError>;
    async fn delete_user(&self, id: Snowflake) -> Result<bool, StorageError>;
}
