use async_trait::async_trait;

use crate::{
    StorageError, TemplateStorage,
    backends::CachedStorage,
    models::{Snowflake, Template, TemplateMetadata},
};

#[async_trait]
impl TemplateStorage for CachedStorage {
    async fn save_template(&self, template: &Template) -> Result<Snowflake, StorageError> {
        let id = self.backend.save_template(template).await?;
        self.cache_del(&self.key("template", &id.0.to_string()))
            .await;
        Ok(id)
    }

    async fn get_template(&self, id: Snowflake) -> Result<Option<Template>, StorageError> {
        let key = self.key("template", &id.0.to_string());
        self.get_through(&key, self.backend.get_template(id)).await
    }

    async fn get_template_owned(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<Option<Template>, StorageError> {
        Ok(self
            .get_template(id)
            .await?
            .filter(|t| t.owner_id == owner_id))
    }

    async fn list_templates(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<TemplateMetadata>, StorageError> {
        self.backend.list_templates(owner_id).await
    }

    async fn delete_template(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<(), StorageError> {
        self.backend.delete_template(id, owner_id).await?;
        self.cache_del(&self.key("template", &id.0.to_string()))
            .await;
        Ok(())
    }

    async fn template_exists(&self, id: Snowflake) -> Result<bool, StorageError> {
        self.backend.template_exists(id).await
    }

    async fn template_name_exists(
        &self,
        owner_id: Snowflake,
        name: &str,
        exclude_id: Option<Snowflake>,
    ) -> Result<bool, StorageError> {
        self.backend
            .template_name_exists(owner_id, name, exclude_id)
            .await
    }

    async fn get_active_template(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let key = self.key("active_template", &user_id.to_string());
        self.get_through(&key, self.backend.get_active_template(user_id))
            .await
    }

    async fn set_active_template(
        &self,
        user_id: Snowflake,
        template_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.backend
            .set_active_template(user_id, template_id)
            .await?;
        self.cache_del(&self.key("active_template", &user_id.to_string()))
            .await;
        Ok(())
    }
}
