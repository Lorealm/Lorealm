use async_trait::async_trait;

use crate::{
    StorageError,
    models::{PersonaOverride, Snowflake, UserPersona},
};

#[async_trait]
pub trait PersonaStorage: Send + Sync {
    async fn list_personas(&self, user_id: Snowflake) -> Result<Vec<UserPersona>, StorageError>;
    async fn save_persona(&self, create: UserPersona) -> Result<(), StorageError>;
    async fn get_persona(&self, id: Snowflake) -> Result<Option<UserPersona>, StorageError>;
    async fn set_active_persona(
        &self,
        user_id: Snowflake,
        persona_id: Snowflake,
    ) -> Result<(), StorageError>;
    async fn get_active_persona(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError>;
    async fn save_override(&self, r#override: PersonaOverride) -> Result<(), StorageError>;
    async fn get_override(
        &self,
        character_id: Snowflake,
    ) -> Result<Option<PersonaOverride>, StorageError>;
    async fn list_overrides(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<PersonaOverride>, StorageError>;
    async fn delete_override(&self, character_id: Snowflake) -> Result<(), StorageError>;
}
