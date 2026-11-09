use crate::error::StorageError;
use crate::models::{CreateInvite, CreateUser, Invite, Snowflake, UpdateUser, User};
use async_trait::async_trait;

#[async_trait]
pub trait UserStorage: Send + Sync {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError>;
    async fn get_user_by_id(&self, id: Snowflake) -> Result<Option<User>, StorageError>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, StorageError>;
    async fn update_user(&self, id: Snowflake, input: &UpdateUser) -> Result<User, StorageError>;
    async fn delete_user(&self, id: Snowflake) -> Result<bool, StorageError>;
    /// Every user, oldest account first.
    async fn list_users(&self) -> Result<Vec<User>, StorageError>;
    async fn create_invite(&self, input: &CreateInvite) -> Result<Invite, StorageError>;
    async fn list_invites(&self) -> Result<Vec<Invite>, StorageError>;
    async fn delete_invite(&self, id: Snowflake) -> Result<bool, StorageError>;
    /// Uses up one of the invite's uses. Returns `None` if the code can't be used.
    async fn redeem_invite(&self, code_hash: &str) -> Result<Option<Invite>, StorageError>;
    /// Gives back a use taken by `redeem_invite`.
    async fn release_invite(&self, id: Snowflake) -> Result<(), StorageError>;
}
