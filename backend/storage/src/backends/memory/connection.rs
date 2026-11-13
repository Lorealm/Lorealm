use async_trait::async_trait;
use chrono::Utc;

use crate::{
    ConnectionStorage, SecretStorage, StorageError,
    backends::MemoryStorage,
    models::{ConnectionProfile, Secret, SecretMetadata, Snowflake, UserSettings},
};

#[async_trait]
impl SecretStorage for MemoryStorage {
    async fn get_secret(&self, id: Snowflake) -> Result<Option<Secret>, StorageError> {
        Ok(self.secrets.read().await.get(&id).cloned())
    }

    async fn list_secrets_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<SecretMetadata>, StorageError> {
        let mut secrets: Vec<SecretMetadata> = self
            .secrets
            .read()
            .await
            .values()
            .filter(|s| s.user_id == user_id)
            .map(|s| SecretMetadata {
                id: s.id,
                user_id: s.user_id,
                provider: s.provider.clone(),
                label: s.label.clone(),
                key_hint: s.key_hint.clone(),
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect();
        secrets.sort_by_key(|s| s.created_at);
        Ok(secrets)
    }

    async fn save_secret(&self, secret: &Secret) -> Result<(), StorageError> {
        let mut secrets = self.secrets.write().await;
        let now = Utc::now();
        let mut stored = secret.clone();
        match secrets.get(&secret.id) {
            // Mirror the Postgres upsert: user_id and provider are immutable,
            // created_at is preserved.
            Some(existing) => {
                stored.user_id = existing.user_id;
                stored.provider = existing.provider.clone();
                stored.created_at = existing.created_at;
                stored.updated_at = now;
            }
            None => {
                stored.created_at = now;
                stored.updated_at = now;
            }
        }
        secrets.insert(stored.id, stored);
        Ok(())
    }

    async fn delete_secret(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = self.secrets.write().await.remove(&id).is_some();
        if removed {
            // Mirror the foreign key's ON DELETE SET NULL.
            let mut profiles = self.connection_profiles.write().await;
            for profile in profiles.values_mut() {
                if profile.secret_id == Some(id) {
                    profile.secret_id = None;
                }
            }
        }
        Ok(removed)
    }
}

#[async_trait]
impl ConnectionStorage for MemoryStorage {
    async fn get_profile(&self, id: Snowflake) -> Result<Option<ConnectionProfile>, StorageError> {
        Ok(self.connection_profiles.read().await.get(&id).cloned())
    }

    async fn list_profiles_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConnectionProfile>, StorageError> {
        let mut profiles: Vec<ConnectionProfile> = self
            .connection_profiles
            .read()
            .await
            .values()
            .filter(|p| p.user_id == user_id)
            .cloned()
            .collect();
        profiles.sort_by_key(|x| std::cmp::Reverse(x.updated_at));
        Ok(profiles)
    }

    async fn save_profile(&self, profile: &ConnectionProfile) -> Result<(), StorageError> {
        // Mirror the foreign key: a referenced secret must exist.
        if let Some(secret_id) = profile.secret_id
            && !self.secrets.read().await.contains_key(&secret_id)
        {
            return Err(StorageError::InvalidInput(format!(
                "secret {secret_id} does not exist"
            )));
        }

        let mut profiles = self.connection_profiles.write().await;
        let now = Utc::now();
        let mut stored = profile.clone();
        match profiles.get(&profile.id) {
            // user_id immutable, created_at preserved, like the SQL upsert.
            Some(existing) => {
                stored.user_id = existing.user_id;
                stored.created_at = existing.created_at;
                stored.updated_at = now;
            }
            None => {
                stored.created_at = now;
                stored.updated_at = now;
            }
        }
        profiles.insert(stored.id, stored);
        Ok(())
    }

    async fn delete_profile(&self, id: Snowflake) -> Result<bool, StorageError> {
        Ok(self.connection_profiles.write().await.remove(&id).is_some())
    }

    async fn profiles_using_secret(
        &self,
        secret_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let mut refs: Vec<(Snowflake, chrono::DateTime<Utc>)> = self
            .connection_profiles
            .read()
            .await
            .values()
            .filter(|p| p.secret_id == Some(secret_id))
            .map(|p| (p.id, p.updated_at))
            .collect();
        refs.sort_by_key(|x| std::cmp::Reverse(x.1));
        Ok(refs.into_iter().map(|(id, _)| id).collect())
    }

    async fn get_active_profile(
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
            .active_connection_profile_id
            .to_owned();

        Ok(profile)
    }
    async fn set_active_profile(
        &self,
        user_id: Snowflake,
        profile_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.user_state
            .write()
            .await
            .entry(user_id)
            .and_modify(|e| e.active_connection_profile_id = profile_id)
            .or_insert(UserSettings {
                active_template_id: None,
                active_persona_id: None,
                active_connection_profile_id: profile_id,
                active_tokenizer_id: None,
            });

        Ok(())
    }
}
