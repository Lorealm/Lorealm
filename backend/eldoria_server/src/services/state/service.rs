use std::sync::Arc;

use eldoria_storage::{
    Storage,
    models::{Lorebook, Snowflake},
};

use crate::{
    errors::{StateError, TargetType},
    services::context::ContextTemplate,
};

pub struct StateService {
    storage: Arc<dyn Storage>,
}

impl StateService {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub async fn set_active_lorebooks(
        &self,
        user_id: Snowflake,
        lorebooks: Vec<Snowflake>,
    ) -> Result<(), StateError> {
        self.storage
            .set_enabled_lorebooks(user_id, &lorebooks)
            .await?;
        Ok(())
    }

    pub async fn get_active_lorebook_ids(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Vec<Snowflake>>, StateError> {
        let lorebook_ids = match self.storage.list_enabled_lorebooks(user_id).await {
            Ok(ids) => ids,
            Err(e) => match e {
                eldoria_storage::StorageError::NotFound(_) => return Ok(None),
                _ => return Err(e.into()),
            },
        };

        Ok(Some(lorebook_ids))
    }

    /// Retrieves an ordered list of active lorebooks
    pub async fn get_active_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Vec<Lorebook>>, StateError> {
        let lorebook_ids = self.get_active_lorebook_ids(user_id).await?;

        if lorebook_ids.is_none() {
            return Ok(None);
        }

        let mut lorebooks: Vec<Lorebook> = Vec::new();
        // unwrap: invariant check above
        for lorebook_id in lorebook_ids.unwrap() {
            let maybe_lorebook = self.storage.get_lorebook(lorebook_id).await?;
            if let Some(lorebook) = maybe_lorebook {
                if lorebook.owner_id != user_id {
                    tracing::warn!(
                        "User ({user_id}) tried requesting a lorebook they did not own. Dropping silently."
                    );
                    continue;
                }
                lorebooks.push(lorebook);
            }
        }

        Ok(Some(lorebooks))
    }

    pub async fn set_active_template(
        &self,
        user_id: Snowflake,
        template_id: Option<Snowflake>,
    ) -> Result<(), StateError> {
        self.storage
            .set_active_template(user_id, template_id)
            .await?;
        Ok(())
    }

    pub async fn get_active_template_id(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StateError> {
        let template_id = self.storage.get_active_template(user_id).await?;

        Ok(template_id)
    }

    pub async fn get_active_template(
        &self,
        user_id: Snowflake,
    ) -> Result<ContextTemplate, StateError> {
        let template_id = self
            .get_active_template_id(user_id)
            .await?
            .ok_or(StateError::NoActiveTemplate)?;

        let template = self
            .storage
            .get_template_owned(template_id, user_id)
            .await?
            .ok_or(StateError::NotFound(
                TargetType::ContextTemplate,
                template_id,
            ))?;

        let ctx_template = ContextTemplate::from_db(template)
            .map_err(|e| StateError::TemplateConversion(e.to_string()))?;

        Ok(ctx_template)
    }
}
