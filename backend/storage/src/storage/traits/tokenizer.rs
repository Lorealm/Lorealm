use crate::error::StorageError;
use crate::models::{Snowflake, Tokenizer};
use async_trait::async_trait;

#[async_trait]
pub trait TokenizerStorage: Send + Sync {
    /// Load a tokenizer by id.
    async fn get_tokenizer(&self, id: Snowflake) -> Result<Option<Tokenizer>, StorageError>;

    /// List tokenizers owned by a user.
    async fn list_tokenizers_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError>;

    /// List every tokenizer a user can use: owned, public, or explicitly shared.
    async fn list_accessible_tokenizers(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError>;

    /// Create or update a tokenizer by id.
    async fn save_tokenizer(&self, tokenizer: &Tokenizer) -> Result<(), StorageError>;

    /// Hard-delete a tokenizer. Share rows are removed via ON DELETE CASCADE.
    async fn delete_tokenizer(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// Whether a user may use a tokenizer (owner, public, or shared).
    async fn can_access_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError>;

    /// Grant a user explicit access to a tokenizer. Idempotent.
    async fn share_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), StorageError>;

    /// Revoke a user's explicit access to a tokenizer.
    async fn unshare_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError>;

    /// List the users a tokenizer is explicitly shared with.
    async fn list_tokenizer_shares(
        &self,
        tok_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError>;

    async fn get_active_tokenizer(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError>;
    async fn set_active_tokenizer(
        &self,
        user_id: Snowflake,
        tokenizer_id: Option<Snowflake>,
    ) -> Result<(), StorageError>;
}
