use async_trait::async_trait;

use crate::{
    StorageError, UserWidgetStorage,
    backends::CachedStorage,
    models::{Snowflake, UserWidget},
};

#[async_trait]
impl UserWidgetStorage for CachedStorage {
    async fn list_widgets(&self, user_id: Snowflake) -> Result<Vec<UserWidget>, StorageError> {
        self.backend.list_widgets(user_id).await
    }

    async fn list_enabled_widgets(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserWidget>, StorageError> {
        self.backend.list_enabled_widgets(user_id).await
    }

    async fn get_widget(&self, id: Snowflake) -> Result<Option<UserWidget>, StorageError> {
        let key = self.key("user_widget", &id.to_string());
        self.get_through(&key, self.backend.get_widget(id)).await
    }

    async fn save_widget(&self, widget: UserWidget) -> Result<(), StorageError> {
        let id = widget.id;
        self.backend.save_widget(widget).await?;
        self.cache_del(&self.key("user_widget", &id.to_string()))
            .await;
        Ok(())
    }

    async fn set_widget_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        self.backend.set_widget_enabled(id, enabled).await?;
        self.cache_del(&self.key("user_widget", &id.to_string()))
            .await;
        Ok(())
    }

    async fn delete_widget(&self, id: Snowflake) -> Result<(), StorageError> {
        self.backend.delete_widget(id).await?;
        self.cache_del(&self.key("user_widget", &id.to_string()))
            .await;
        Ok(())
    }
}
