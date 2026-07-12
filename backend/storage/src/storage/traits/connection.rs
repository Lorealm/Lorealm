use crate::error::StorageError;
use crate::models::{ConnectionProfile, Secret, SecretMetadata, Snowflake};
use async_trait::async_trait;

#[async_trait]
pub trait SecretStorage: Send + Sync {
    /// Load a secret including its encrypted payload. Only call this on the
    /// decryption path (inference); listing for display goes through
    /// `list_secrets_by_owner`, which never returns ciphertext.
    async fn get_secret(&self, id: Snowflake) -> Result<Option<Secret>, StorageError>;

    /// List secret metadata owned by a user. Deliberately excludes the
    /// encrypted payload: secrets are write-only from the client's view.
    async fn list_secrets_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<SecretMetadata>, StorageError>;

    /// Create or update a secret. Updates occur on label edits and on
    /// re-encryption after a master key rotation.
    async fn save_secret(&self, secret: &Secret) -> Result<(), StorageError>;

    /// Hard-delete a secret. Profiles referencing it are detached
    /// (secret_id set to NULL by the foreign key) and keep existing.
    async fn delete_secret(&self, id: Snowflake) -> Result<bool, StorageError>;
}

#[async_trait]
pub trait ConnectionStorage: Send + Sync {
    /// Load a connection profile.
    async fn get_profile(&self, id: Snowflake) -> Result<Option<ConnectionProfile>, StorageError>;

    /// List connection profiles owned by a user.
    async fn list_profiles_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConnectionProfile>, StorageError>;

    /// Create or update a connection profile.
    async fn save_profile(&self, profile: &ConnectionProfile) -> Result<(), StorageError>;

    /// Hard-delete a connection profile.
    async fn delete_profile(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// List the profiles referencing a secret. Supports the deletion UX:
    /// confirm dialogs ("these profiles will be detached") and post-delete
    /// reporting.
    async fn profiles_using_secret(
        &self,
        secret_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError>;

    async fn get_active_profile(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError>;
    async fn set_active_profile(
        &self,
        user_id: Snowflake,
        profile_id: Option<Snowflake>,
    ) -> Result<(), StorageError>;
}
