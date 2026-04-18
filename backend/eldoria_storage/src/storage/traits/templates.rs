use async_trait::async_trait;

use crate::{
    StorageError,
    models::{Snowflake, Template, TemplateMetadata},
};

#[async_trait]
pub trait TemplateStorage: Send + Sync {
    async fn save_template(&self, template: &Template) -> Result<Snowflake, StorageError>;

    /// Get a template by ID
    async fn get_template(&self, id: Snowflake) -> Result<Option<Template>, StorageError>;

    /// Get a template by ID, but only if owned by the specified user
    async fn get_template_owned(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<Option<Template>, StorageError>;

    /// List all templates for a user (metadata only)
    async fn list_templates(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<TemplateMetadata>, StorageError>;

    /// Delete a template
    async fn delete_template(&self, id: Snowflake, owner_id: Snowflake)
    -> Result<(), StorageError>;

    /// Check if a template exists
    async fn template_exists(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// Check if a template name is already used by this owner
    async fn template_name_exists(
        &self,
        owner_id: Snowflake,
        name: &str,
        exclude_id: Option<Snowflake>,
    ) -> Result<bool, StorageError>;
}
