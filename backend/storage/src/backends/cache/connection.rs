use async_trait::async_trait;

use crate::{
    ConnectionStorage, SecretStorage, StorageError,
    backends::CachedStorage,
    models::{ConnectionProfile, Secret, SecretMetadata, Snowflake},
};

/// Secrets are deliberately NOT cached. The `Secret` model has no
/// `Serialize` derive precisely so encrypted key material can't end up
/// serialized into Redis; every method passes straight through.
#[async_trait]
impl SecretStorage for CachedStorage {
    async fn get_secret(&self, id: Snowflake) -> Result<Option<Secret>, StorageError> {
        self.backend.get_secret(id).await
    }

    async fn list_secrets_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<SecretMetadata>, StorageError> {
        self.backend.list_secrets_by_owner(user_id).await
    }

    async fn save_secret(&self, secret: &Secret) -> Result<(), StorageError> {
        self.backend.save_secret(secret).await
    }

    async fn delete_secret(&self, id: Snowflake) -> Result<bool, StorageError> {
        self.backend.delete_secret(id).await
    }
}

#[async_trait]
impl ConnectionStorage for CachedStorage {
    async fn get_profile(&self, id: Snowflake) -> Result<Option<ConnectionProfile>, StorageError> {
        let key = self.key("connection_profile", &id.to_string());
        self.get_through(&key, self.backend.get_profile(id)).await
    }

    async fn list_profiles_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConnectionProfile>, StorageError> {
        self.backend.list_profiles_by_owner(user_id).await
    }

    async fn save_profile(&self, profile: &ConnectionProfile) -> Result<(), StorageError> {
        self.backend.save_profile(profile).await?;
        self.cache_del(&self.key("connection_profile", &profile.id.to_string()))
            .await;
        Ok(())
    }

    async fn delete_profile(&self, id: Snowflake) -> Result<bool, StorageError> {
        let deleted = self.backend.delete_profile(id).await?;
        self.cache_del(&self.key("connection_profile", &id.to_string()))
            .await;
        Ok(deleted)
    }

    async fn profiles_using_secret(
        &self,
        secret_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        self.backend.profiles_using_secret(secret_id).await
    }

    async fn get_active_profile(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        self.backend.get_active_profile(user_id).await
    }
    async fn set_active_profile(
        &self,
        user_id: Snowflake,
        profile_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.backend.set_active_profile(user_id, profile_id).await
    }
}
