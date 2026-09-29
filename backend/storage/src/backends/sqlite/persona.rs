use async_trait::async_trait;

use super::NOW;
use crate::{
    PersonaStorage, StorageError,
    backends::SqliteStorage,
    models::{PersonaOverride, Snowflake, UserPersona},
};

#[async_trait]
impl PersonaStorage for SqliteStorage {
    async fn list_personas(&self, user_id: Snowflake) -> Result<Vec<UserPersona>, StorageError> {
        let personas =
            sqlx::query_as::<_, UserPersona>(r#"SELECT * FROM user_personas WHERE user_id = $1"#)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(personas)
    }
    async fn save_persona(&self, create: UserPersona) -> Result<(), StorageError> {
        sqlx::query(&format!(
            r#"INSERT INTO user_personas
                (id, user_id, name, display_name, pseudonym, avatar_uri, description, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, {NOW})
            ON CONFLICT (id) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                pseudonym = EXCLUDED.pseudonym,
                avatar_uri = EXCLUDED.avatar_uri,
                description = EXCLUDED.description,
                updated_at = {NOW}"#
        ))
        .bind(create.id)
        .bind(create.user_id)
        .bind(create.name)
        .bind(create.display_name)
        .bind(create.pseudonym)
        .bind(create.avatar_uri)
        .bind(create.description)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_persona(&self, id: Snowflake) -> Result<Option<UserPersona>, StorageError> {
        let persona =
            sqlx::query_as::<_, UserPersona>(r#"SELECT * FROM user_personas WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(persona)
    }

    async fn get_active_persona(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let active: Option<Option<Snowflake>> =
            sqlx::query_scalar(r#"SELECT active_persona_id FROM user_settings WHERE user_id = $1"#)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(active.flatten())
    }

    async fn set_active_persona(
        &self,
        user_id: Snowflake,
        persona_id: Snowflake,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO user_settings (user_id, active_persona_id) VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET active_persona_id = EXCLUDED.active_persona_id"#,
        )
        .bind(user_id)
        .bind(persona_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn save_override(&self, r#override: PersonaOverride) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO character_persona_overrides
                (character_id, persona_id, user_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (character_id) DO UPDATE SET
                persona_id = EXCLUDED.persona_id,
                user_id = EXCLUDED.user_id"#,
        )
        .bind(r#override.character_id)
        .bind(r#override.persona_id)
        .bind(r#override.user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_override(
        &self,
        character_id: Snowflake,
    ) -> Result<Option<PersonaOverride>, StorageError> {
        let result = sqlx::query_as::<_, PersonaOverride>(
            r#"SELECT character_id, persona_id, user_id
               FROM character_persona_overrides WHERE character_id = $1"#,
        )
        .bind(character_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn list_overrides(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<PersonaOverride>, StorageError> {
        let overrides = sqlx::query_as::<_, PersonaOverride>(
            r#"SELECT character_id, persona_id, user_id
               FROM character_persona_overrides WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(overrides)
    }

    async fn delete_override(&self, character_id: Snowflake) -> Result<(), StorageError> {
        sqlx::query(r#"DELETE FROM character_persona_overrides WHERE character_id = $1"#)
            .bind(character_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
