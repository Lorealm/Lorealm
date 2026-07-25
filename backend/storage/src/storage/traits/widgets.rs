use async_trait::async_trait;

use crate::{
    StorageError,
    models::{Snowflake, UserWidget},
};

#[async_trait]
pub trait UserWidgetStorage: Send + Sync {
    async fn list_widgets(&self, user_id: Snowflake) -> Result<Vec<UserWidget>, StorageError>;
    async fn list_enabled_widgets(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserWidget>, StorageError>;
    async fn get_widget(&self, id: Snowflake) -> Result<Option<UserWidget>, StorageError>;
    async fn save_widget(&self, widget: UserWidget) -> Result<(), StorageError>;
    async fn set_widget_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError>;
    async fn delete_widget(&self, id: Snowflake) -> Result<(), StorageError>;
}
