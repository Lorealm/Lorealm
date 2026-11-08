use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use lorealm_storage::{
    BlobStore, Storage, StorageError,
    models::{BlobId, PersonaOverride, Snowflake, UserPersona},
};

use crate::{
    errors::{PersonaError, TargetType},
    services::{persona::UpdatePersona, snowflake::SnowflakeService},
};

pub struct PersonaService {
    storage: Arc<dyn Storage>,
    blob: Arc<dyn BlobStore>,
    snowflake: Arc<SnowflakeService>,
}

impl PersonaService {
    pub fn new(
        storage: Arc<dyn Storage>,
        blob: Arc<dyn BlobStore>,
        snowflake: Arc<SnowflakeService>,
    ) -> Self {
        Self {
            storage,
            blob,
            snowflake,
        }
    }

    pub async fn get_persona(
        &self,
        persona_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<UserPersona, PersonaError> {
        let persona = self
            .storage
            .get_persona(persona_id)
            .await?
            .ok_or(PersonaError::NotFound(TargetType::Persona, persona_id))?;

        if persona.user_id != user_id {
            return Err(PersonaError::NotOwned);
        }

        Ok(persona)
    }

    pub async fn create_default(
        &self,
        user_id: Snowflake,
        name: String,
    ) -> Result<Snowflake, PersonaError> {
        let id = self.snowflake.generate();
        let persona = UserPersona {
            id,
            user_id,
            name,
            display_name: None,
            pseudonym: None,
            avatar_uri: None,
            description: None,
            updated_at: Utc::now(),
        };

        self.storage.save_persona(persona).await?;

        Ok(id)
    }

    pub async fn list_personas(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserPersona>, PersonaError> {
        let personas = self.storage.list_personas(user_id).await?;

        Ok(personas)
    }

    pub async fn update_persona(
        &self,
        persona_id: Snowflake,
        user_id: Snowflake,
        update: UpdatePersona,
    ) -> Result<UserPersona, PersonaError> {
        let mut persona = self.get_persona(persona_id, user_id).await?;

        if let Some(name) = update.name {
            persona.name = name;
        }
        if let Some(display_name) = update.display_name {
            persona.display_name = display_name;
        }
        if let Some(pseudonym) = update.pseudonym {
            persona.pseudonym = pseudonym;
        }
        if let Some(description) = update.description {
            persona.description = description
        }

        self.storage.save_persona(persona.clone()).await?;

        Ok(persona)
    }

    /// Delete a persona. The last remaining persona cannot be deleted, since
    /// chats need one. If it was the active persona, another one takes over.
    pub async fn delete_persona(
        &self,
        persona_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), PersonaError> {
        self.get_persona(persona_id, user_id).await?;

        let personas = self.storage.list_personas(user_id).await?;
        let Some(successor) = personas.iter().find(|p| p.id != persona_id) else {
            return Err(PersonaError::LastPersona);
        };

        let active = match self.storage.get_active_persona(user_id).await {
            Ok(active) => active,
            // The memory backend reports a user without settings as not found.
            Err(StorageError::NotFound(_)) => None,
            Err(e) => return Err(e.into()),
        };
        if active == Some(persona_id) {
            self.storage
                .set_active_persona(user_id, successor.id)
                .await?;
        }

        self.storage.delete_persona(persona_id).await?;
        Ok(())
    }

    pub async fn avatar_from_uri(&self, avatar_uri: Option<String>) -> Result<Bytes, PersonaError> {
        let blob_id =
            avatar_uri
                .as_deref()
                .and_then(BlobId::from_uri)
                .ok_or(StorageError::NotFound(
                    avatar_uri.unwrap_or("<null>".into()),
                ))?;

        let bytes = self
            .blob
            .get(&blob_id)
            .await?
            .ok_or(StorageError::NotFound(blob_id.to_uri()))?;

        Ok(bytes)
    }

    pub async fn get_avatar(
        &self,
        persona_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Bytes, PersonaError> {
        let persona = self.get_persona(persona_id, user_id).await?;
        let bytes = self.avatar_from_uri(persona.avatar_uri).await?;

        Ok(bytes)
    }

    pub async fn update_image(
        &self,
        persona_id: Snowflake,
        user_id: Snowflake,
        image_bytes: Bytes,
    ) -> Result<String, PersonaError> {
        let mut persona = self.get_persona(persona_id, user_id).await?;
        let blob_id = self.blob.put(&image_bytes).await?;
        let image_uri = blob_id.to_uri();
        persona.avatar_uri = Some(image_uri.clone());
        self.storage.save_persona(persona).await?;

        Ok(image_uri)
    }

    /// Gets the persona for the given character
    ///
    /// First tries finding an explicit override, if not, falls back to `default_persona`
    pub async fn get_persona_for_character(
        &self,
        user_id: Snowflake,
        character_id: Snowflake,
        default_persona: Snowflake,
    ) -> Result<UserPersona, PersonaError> {
        let r#override = self.storage.get_override(character_id).await?;

        let persona = match r#override {
            Some(p) => self.get_persona(p.persona_id, user_id).await?,
            None => self.get_persona(default_persona, user_id).await?,
        };

        Ok(persona)
    }

    pub async fn list_overrides(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<PersonaOverride>, PersonaError> {
        Ok(self.storage.list_overrides(user_id).await?)
    }

    pub async fn set_override(
        &self,
        user_id: Snowflake,
        character_id: Snowflake,
        persona_id: Snowflake,
    ) -> Result<PersonaOverride, PersonaError> {
        self.get_persona(persona_id, user_id).await?;

        let r#override = PersonaOverride {
            character_id,
            persona_id,
            user_id,
        };
        self.storage.save_override(r#override.clone()).await?;

        Ok(r#override)
    }

    pub async fn clear_override(
        &self,
        user_id: Snowflake,
        character_id: Snowflake,
    ) -> Result<(), PersonaError> {
        match self.storage.get_override(character_id).await? {
            Some(existing) if existing.user_id != user_id => return Err(PersonaError::NotOwned),
            Some(_) => self.storage.delete_override(character_id).await?,
            None => return Ok(()),
        }

        Ok(())
    }
}
