use crate::TokenizerStorage;
use crate::error::StorageError;
use crate::models::{Snowflake, Tokenizer};
use async_trait::async_trait;

use super::CachedStorage;

#[async_trait]
impl TokenizerStorage for CachedStorage {
    async fn get_tokenizer(&self, id: Snowflake) -> Result<Option<Tokenizer>, StorageError> {
        let key = self.key("tokenizer", &id.to_string());
        self.get_through(&key, self.backend.get_tokenizer(id)).await
    }

    async fn list_tokenizers_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError> {
        self.backend.list_tokenizers_by_owner(user_id).await
    }

    async fn list_accessible_tokenizers(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError> {
        self.backend.list_accessible_tokenizers(user_id).await
    }

    async fn save_tokenizer(&self, tokenizer: &Tokenizer) -> Result<(), StorageError> {
        self.backend.save_tokenizer(tokenizer).await?;
        self.cache_del(&self.key("tokenizer", &tokenizer.id.to_string()))
            .await;
        Ok(())
    }

    async fn delete_tokenizer(&self, id: Snowflake) -> Result<bool, StorageError> {
        let deleted = self.backend.delete_tokenizer(id).await?;
        self.cache_del(&self.key("tokenizer", &id.to_string()))
            .await;
        Ok(deleted)
    }

    async fn can_access_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError> {
        self.backend.can_access_tokenizer(tok_id, user_id).await
    }

    async fn share_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), StorageError> {
        self.backend.share_tokenizer(tok_id, user_id).await
    }

    async fn unshare_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError> {
        self.backend.unshare_tokenizer(tok_id, user_id).await
    }

    async fn list_tokenizer_shares(
        &self,
        tok_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        self.backend.list_tokenizer_shares(tok_id).await
    }

    async fn get_active_tokenizer(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        self.backend.get_active_tokenizer(user_id).await
    }
    async fn set_active_tokenizer(
        &self,
        user_id: Snowflake,
        tokenizer_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.set_active_tokenizer(user_id, tokenizer_id).await
    }
}
