use crate::TokenizerStorage;
use crate::error::StorageError;
use crate::models::{Snowflake, Tokenizer, TokenizerShare, UserSettings};
use async_trait::async_trait;
use std::collections::HashSet;

use super::MemoryStorage;

#[async_trait]
impl TokenizerStorage for MemoryStorage {
    async fn get_tokenizer(&self, id: Snowflake) -> Result<Option<Tokenizer>, StorageError> {
        Ok(self.tokenizers.read().await.get(&id).cloned())
    }

    async fn list_tokenizers_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError> {
        let mut out: Vec<Tokenizer> = self
            .tokenizers
            .read()
            .await
            .values()
            .filter(|t| t.owner_id == user_id)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    async fn list_accessible_tokenizers(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError> {
        let shared: HashSet<Snowflake> = self
            .tokenizer_shares
            .read()
            .await
            .iter()
            .filter(|s| s.user_id == user_id)
            .map(|s| s.tok_id)
            .collect();

        let mut out: Vec<Tokenizer> = self
            .tokenizers
            .read()
            .await
            .values()
            .filter(|t| t.owner_id == user_id || t.is_public || shared.contains(&t.id))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    async fn save_tokenizer(&self, tokenizer: &Tokenizer) -> Result<(), StorageError> {
        self.tokenizers
            .write()
            .await
            .insert(tokenizer.id, tokenizer.clone());
        Ok(())
    }

    async fn delete_tokenizer(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = self.tokenizers.write().await.remove(&id).is_some();
        if removed {
            // ON DELETE CASCADE for share rows.
            self.tokenizer_shares
                .write()
                .await
                .retain(|s| s.tok_id != id);
        }
        Ok(removed)
    }

    async fn can_access_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let (owner_or_public, known) = {
            let tokenizers = self.tokenizers.read().await;
            match tokenizers.get(&tok_id) {
                Some(t) => (t.owner_id == user_id || t.is_public, true),
                None => (false, false),
            }
        };
        if !known {
            return Ok(false);
        }
        if owner_or_public {
            return Ok(true);
        }

        let shared = self
            .tokenizer_shares
            .read()
            .await
            .iter()
            .any(|s| s.tok_id == tok_id && s.user_id == user_id);
        Ok(shared)
    }

    async fn share_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), StorageError> {
        let mut shares = self.tokenizer_shares.write().await;
        let exists = shares
            .iter()
            .any(|s| s.tok_id == tok_id && s.user_id == user_id);
        if !exists {
            shares.push(TokenizerShare { tok_id, user_id });
        }
        Ok(())
    }

    async fn unshare_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let mut shares = self.tokenizer_shares.write().await;
        let before = shares.len();
        shares.retain(|s| !(s.tok_id == tok_id && s.user_id == user_id));
        Ok(shares.len() != before)
    }

    async fn list_tokenizer_shares(
        &self,
        tok_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let mut out: Vec<Snowflake> = self
            .tokenizer_shares
            .read()
            .await
            .iter()
            .filter(|s| s.tok_id == tok_id)
            .map(|s| s.user_id)
            .collect();
        out.sort();
        Ok(out)
    }

    async fn get_active_tokenizer(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let profile = self
            .user_state
            .read()
            .await
            .get(&user_id)
            .ok_or(StorageError::NotFound(format!(
                "no state for user: {user_id}"
            )))?
            .active_tokenizer_id
            .to_owned();

        Ok(profile)
    }
    async fn set_active_tokenizer(
        &self,
        user_id: Snowflake,
        tokenizer_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.user_state
            .write()
            .await
            .entry(user_id)
            .and_modify(|e| e.active_tokenizer_id = tokenizer_id)
            .or_insert(UserSettings {
                active_template_id: None,
                active_persona_id: None,
                active_connection_profile_id: None,
                active_tokenizer_id: tokenizer_id,
            });

        Ok(())
    }
}
