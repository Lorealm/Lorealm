use async_trait::async_trait;
use chrono::Utc;

use crate::{
    PersonaStorage, StorageError,
    backends::MemoryStorage,
    models::{PersonaOverride, Snowflake, UserPersona, UserSettings},
};

#[async_trait]
impl PersonaStorage for MemoryStorage {
    async fn list_personas(&self, user_id: Snowflake) -> Result<Vec<UserPersona>, StorageError> {
        let personas: Vec<UserPersona> = self
            .personas
            .read()
            .await
            .values()
            .filter(|b| b.user_id == user_id)
            .cloned()
            .collect();

        Ok(personas)
    }
    async fn save_persona(&self, create: UserPersona) -> Result<(), StorageError> {
        let mut personas = self.personas.write().await;
        match personas.get_mut(&create.id) {
            Some(existing) => {
                existing.display_name = create.display_name;
                existing.pseudonym = create.pseudonym;
                existing.avatar_uri = create.avatar_uri;
                existing.description = create.description;
                existing.updated_at = Utc::now();
            }
            None => {
                personas.insert(create.id, create);
            }
        }
        Ok(())
    }

    async fn get_persona(&self, id: Snowflake) -> Result<Option<UserPersona>, StorageError> {
        let personas = self.personas.read().await;
        Ok(personas.get(&id).cloned())
    }

    async fn get_active_persona(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let persona = self
            .user_state
            .read()
            .await
            .get(&user_id)
            .ok_or(StorageError::NotFound(format!(
                "no state for user: {user_id}"
            )))?
            .active_persona_id
            .to_owned();

        Ok(persona)
    }

    async fn set_active_persona(
        &self,
        user_id: Snowflake,
        persona_id: Snowflake,
    ) -> Result<(), StorageError> {
        self.user_state
            .write()
            .await
            .entry(user_id)
            .and_modify(|e| e.active_persona_id = Some(persona_id))
            .or_insert(UserSettings {
                active_template_id: None,
                active_persona_id: Some(persona_id),
            });

        Ok(())
    }

    async fn save_override(&self, r#override: PersonaOverride) -> Result<(), StorageError> {
        let mut overrides = self.persona_overrides.write().await;
        overrides.insert(r#override.character_id, r#override);
        Ok(())
    }

    async fn get_override(
        &self,
        character_id: Snowflake,
    ) -> Result<Option<PersonaOverride>, StorageError> {
        let overrides = self.persona_overrides.read().await;
        Ok(overrides.get(&character_id).cloned())
    }
}
