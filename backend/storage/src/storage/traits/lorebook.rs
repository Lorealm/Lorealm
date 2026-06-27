use crate::error::StorageError;
use crate::models::{Lorebook, LorebookMetadata, LorebookVersionInfo, Snowflake};
use async_trait::async_trait;

#[async_trait]
pub trait LorebookStorage: Send + Sync {
    /// Load the current version of a lorebook
    async fn get_lorebook(&self, id: Snowflake) -> Result<Option<Lorebook>, StorageError>;

    /// Load a specific pinned version of a lorebook by reconstructing
    /// from its snapshot.
    async fn get_lorebook_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Lorebook>, StorageError>;

    /// List lorebook heads owned by a user.
    async fn list_lorebooks_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, StorageError>;

    /// Create or update a lorebook. Always creates a new version row and
    /// atomically updates the head + entry child tables to match.
    async fn save_lorebook(&self, lorebook: &Lorebook) -> Result<Snowflake, StorageError>;

    /// Hard-delete a lorebook
    async fn delete_lorebook(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// List version metadata for a lorebook
    async fn list_lorebook_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<LorebookVersionInfo>, StorageError>;

    /// Roll the head back to a prior version.
    async fn restore_lorebook_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError>;

    /// List the global active lorebooks for a given user
    async fn list_enabled_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError>;
    /// Set the global active lorebooks for a given user
    async fn set_enabled_lorebooks(
        &self,
        user_id: Snowflake,
        ordered: &[Snowflake],
    ) -> Result<(), StorageError>;
}
