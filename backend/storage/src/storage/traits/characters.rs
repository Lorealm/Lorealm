use crate::error::StorageError;
use crate::models::{
    Character, CharacterSummary, CharacterVersionInfo, Snowflake, VersionedSaveMode,
};
use async_trait::async_trait;

#[async_trait]
pub trait CharacterStorage: Send + Sync {
    async fn list_characters(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<CharacterSummary>, StorageError>;
    /// Load the current version of a character
    async fn get_character(&self, id: Snowflake) -> Result<Option<Character>, StorageError>;

    /// Load a specific pinned version of a character by reconstructing
    /// from its snapshot.
    async fn get_character_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Character>, StorageError>;

    /// List character heads owned by the given user
    async fn list_characters_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Character>, StorageError>;

    /// Create or update a character. Always creates a new version row and
    /// atomically updates the head + child tables to match.
    async fn save_character(
        &self,
        character: &Character,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError>;

    /// Hard-delete a character and all its versions/child rows.
    /// Returns `true` if a row was deleted.
    async fn delete_character(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// List version metadata for a character
    async fn list_character_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<CharacterVersionInfo>, StorageError>;

    /// Roll the head back to a prior version. Equivalent to loading the
    /// snapshot, creating a fresh version row from it, and updating head.
    /// Returns the new `version_id`
    async fn restore_character_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError>;

    async fn get_latest_version(&self, id: Snowflake) -> Result<Snowflake, StorageError>;
}
