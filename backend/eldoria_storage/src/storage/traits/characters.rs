//! Character storage trait.
//!
//! Coarse-grained: writes take whole `Character` aggregates so the backend
//! owns atomicity. Versioning is implicit — every `save_character` creates
//! a new `character_versions` row and atomically updates the head.

use crate::error::StorageError;
use crate::models::{Character, CharacterSummary, CharacterVersionInfo, Snowflake, VersionedSaveMode};
use async_trait::async_trait;

#[async_trait]
pub trait CharacterStorage: Send + Sync {
    async fn list_characters(&self, user_id: Snowflake) -> Result<Vec<CharacterSummary>, StorageError>;
    /// Load the current version of a character, including child rows
    /// (tags, greetings, example messages). Returns `None` if absent.
    async fn get_character(&self, id: Snowflake) -> Result<Option<Character>, StorageError>;

    /// Load a specific pinned version of a character by reconstructing
    /// from its snapshot. The returned `Character`'s `current_version_id`
    /// will equal `version_id`, not the entity's actual head.
    async fn get_character_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Character>, StorageError>;

    /// List character heads owned by a user. Returns current state only —
    /// no versions, no snapshots.
    async fn list_characters_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Character>, StorageError>;

    /// Create or update a character. Always creates a new version row and
    /// atomically updates the head + child tables to match. Returns the
    /// new `version_id`.
    ///
    /// Caller is responsible for generating `character.id` and
    /// `character.current_version_id` (via snowflake gen) before calling.
    /// The storage layer treats those IDs as authoritative.
    async fn save_character(&self, character: &Character, mode: VersionedSaveMode) -> Result<Snowflake, StorageError>;

    /// Hard-delete a character and all its versions/child rows.
    /// Returns `true` if a row was deleted.
    async fn delete_character(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// List version metadata for a character (no snapshots). Ordered newest
    /// first. Used by the rollback/history UI.
    async fn list_character_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<CharacterVersionInfo>, StorageError>;

    /// Roll the head back to a prior version. Equivalent to loading the
    /// snapshot, creating a fresh version row from it, and updating head.
    /// Returns the new `version_id` (which is NOT the same as `version_id`
    /// — restoring creates a new version that mirrors the old one).
    async fn restore_character_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError>;

    async fn get_latest_version(
        &self,
        id: Snowflake
    ) -> Result<Snowflake, StorageError>;
}
