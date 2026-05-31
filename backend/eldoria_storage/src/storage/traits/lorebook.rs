//! Lorebook storage trait.
//!
//! Coarse-grained: writes take whole `Lorebook` aggregates (config + entries)
//! so the backend owns atomicity. Versioning is implicit — every
//! `save_lorebook` creates a new `lorebook_versions` row and atomically
//! updates the head plus the entry child rows.
//!
//! Note: the storage `Lorebook` model is distinct from the engine's
//! `context_weaver::Lorebook`. Each entry is held as its raw `.weaver`
//! source (frontmatter + body); rehydrate to the engine type at load time
//! via `Entry::parse` + `add_entry`.

use crate::error::StorageError;
use crate::models::{Lorebook, LorebookMetadata, LorebookVersionInfo, Snowflake};
use async_trait::async_trait;

#[async_trait]
pub trait LorebookStorage: Send + Sync {
    /// Load the current version of a lorebook, including child rows
    /// (entries as raw `.weaver` source). Returns `None` if absent.
    async fn get_lorebook(&self, id: Snowflake) -> Result<Option<Lorebook>, StorageError>;

    /// Load a specific pinned version of a lorebook by reconstructing
    /// from its snapshot. The returned `Lorebook`'s `current_version_id`
    /// will equal `version_id`, not the entity's actual head.
    async fn get_lorebook_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Lorebook>, StorageError>;

    /// List lorebook heads owned by a user. Returns current state only —
    /// no versions, no snapshots.
    async fn list_lorebooks_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, StorageError>;

    /// Create or update a lorebook. Always creates a new version row and
    /// atomically updates the head + entry child tables to match. Returns
    /// the new `version_id`.
    ///
    /// Caller is responsible for generating `lorebook.id` and
    /// `lorebook.current_version_id` (via snowflake gen) before calling.
    /// The storage layer treats those IDs as authoritative.
    async fn save_lorebook(&self, lorebook: &Lorebook) -> Result<Snowflake, StorageError>;

    /// Hard-delete a lorebook and all its versions/entry rows. Cascades to
    /// session state referencing it. Returns `true` if a row was deleted.
    async fn delete_lorebook(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// List version metadata for a lorebook (no snapshots). Ordered newest
    /// first. Used by the rollback/history UI.
    async fn list_lorebook_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<LorebookVersionInfo>, StorageError>;

    /// Roll the head back to a prior version. Equivalent to loading the
    /// snapshot, creating a fresh version row from it, and updating head.
    /// Returns the new `version_id` (which is NOT the same as `version_id`
    /// — restoring creates a new version that mirrors the old one).
    async fn restore_lorebook_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError>;
}
