use async_trait::async_trait;

use crate::{
    PersonaStorage, StorageError,
    backends::CachedStorage,
    models::{PersonaOverride, Snowflake, UserPersona},
};

#[async_trait]
impl PersonaStorage for CachedStorage {
    async fn list_personas(&self, user_id: Snowflake) -> Result<Vec<UserPersona>, StorageError> {
        self.backend.list_personas(user_id).await
    }
    async fn save_persona(&self, create: UserPersona) -> Result<(), StorageError> {
        self.backend.save_persona(create.clone()).await?;
        self.cache_del(&self.key("persona", &create.id.to_string()))
            .await;
        Ok(())
    }

    async fn get_persona(&self, id: Snowflake) -> Result<Option<UserPersona>, StorageError> {
        let key = self.key("persona", &id.to_string());
        self.get_through(&key, self.backend.get_persona(id)).await
    }

    async fn delete_persona(&self, id: Snowflake) -> Result<bool, StorageError> {
        let stale_overrides = match self.backend.get_persona(id).await? {
            Some(persona) => self
                .backend
                .list_overrides(persona.user_id)
                .await?
                .into_iter()
                .filter(|o| o.persona_id == id)
                .map(|o| o.character_id)
                .collect(),
            None => Vec::new(),
        };

        let removed = self.backend.delete_persona(id).await?;

        self.cache_del(&self.key("persona", &id.to_string())).await;
        for character_id in stale_overrides {
            self.cache_del(&self.key("persona_override", &character_id.to_string()))
                .await;
        }
        Ok(removed)
    }

    async fn get_active_persona(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        self.backend.get_active_persona(user_id).await
    }

    async fn set_active_persona(
        &self,
        user_id: Snowflake,
        persona_id: Snowflake,
    ) -> Result<(), StorageError> {
        self.backend.set_active_persona(user_id, persona_id).await
    }

    async fn save_override(&self, r#override: PersonaOverride) -> Result<(), StorageError> {
        let key = self.key("persona_override", &r#override.character_id.to_string());
        self.backend.save_override(r#override).await?;
        self.cache_del(&key).await;
        Ok(())
    }

    async fn get_override(
        &self,
        character_id: Snowflake,
    ) -> Result<Option<PersonaOverride>, StorageError> {
        let key = self.key("persona_override", &character_id.to_string());
        self.get_through(&key, self.backend.get_override(character_id))
            .await
    }

    async fn list_overrides(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<PersonaOverride>, StorageError> {
        self.backend.list_overrides(user_id).await
    }

    async fn delete_override(&self, character_id: Snowflake) -> Result<(), StorageError> {
        let key = self.key("persona_override", &character_id.to_string());
        self.backend.delete_override(character_id).await?;
        self.cache_del(&key).await;
        Ok(())
    }
}
