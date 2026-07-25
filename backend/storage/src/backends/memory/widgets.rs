use async_trait::async_trait;

use crate::{
    StorageError, UserWidgetStorage,
    backends::MemoryStorage,
    models::{Snowflake, UserWidget},
};

#[async_trait]
impl UserWidgetStorage for MemoryStorage {
    async fn list_widgets(&self, user_id: Snowflake) -> Result<Vec<UserWidget>, StorageError> {
        let mut out: Vec<UserWidget> = self
            .user_widgets
            .read()
            .await
            .values()
            .filter(|w| w.user_id == user_id)
            .cloned()
            .collect();
        out.sort_by_key(|w| w.created_at);
        Ok(out)
    }

    async fn list_enabled_widgets(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserWidget>, StorageError> {
        let mut out: Vec<UserWidget> = self
            .user_widgets
            .read()
            .await
            .values()
            .filter(|w| w.user_id == user_id && w.enabled)
            .cloned()
            .collect();
        out.sort_by_key(|w| w.created_at);
        Ok(out)
    }

    async fn get_widget(&self, id: Snowflake) -> Result<Option<UserWidget>, StorageError> {
        Ok(self.user_widgets.read().await.get(&id).cloned())
    }

    async fn save_widget(&self, widget: UserWidget) -> Result<(), StorageError> {
        self.user_widgets.write().await.insert(widget.id, widget);
        Ok(())
    }

    async fn set_widget_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        if let Some(widget) = self.user_widgets.write().await.get_mut(&id) {
            widget.enabled = enabled;
        }
        Ok(())
    }

    async fn delete_widget(&self, id: Snowflake) -> Result<(), StorageError> {
        self.user_widgets.write().await.remove(&id);
        Ok(())
    }
}
