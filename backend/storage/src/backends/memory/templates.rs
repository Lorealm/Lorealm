use async_trait::async_trait;

use crate::{
    StorageError, TemplateStorage,
    backends::MemoryStorage,
    models::{Snowflake, Template, TemplateMetadata, UserSettings},
};

fn template_to_metadata(t: &Template) -> TemplateMetadata {
    TemplateMetadata {
        id: t.id,
        owner_id: t.owner_id,
        name: t.name.clone(),
        description: t.description.clone(),
        node_count: t.nodes_json.as_array().map(|a| a.len()).unwrap_or(0),
        version: t.version,
        created_at: t.created_at,
        updated_at: t.updated_at,
    }
}

#[async_trait]
impl TemplateStorage for MemoryStorage {
    async fn save_template(&self, template: &Template) -> Result<Snowflake, StorageError> {
        let mut templates = self.templates.write().await;
        templates.insert(template.id, template.clone());
        Ok(template.id)
    }

    async fn get_template(&self, id: Snowflake) -> Result<Option<Template>, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.get(&id).cloned())
    }

    async fn get_template_owned(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<Option<Template>, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates
            .get(&id)
            .filter(|t| t.owner_id == owner_id)
            .cloned())
    }

    async fn list_templates(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<TemplateMetadata>, StorageError> {
        let templates = self.templates.read().await;
        let mut out: Vec<TemplateMetadata> = templates
            .values()
            .filter(|t| t.owner_id == owner_id)
            .map(template_to_metadata)
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(out)
    }

    async fn delete_template(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<(), StorageError> {
        let mut templates = self.templates.write().await;
        if templates.get(&id).map(|t| t.owner_id) == Some(owner_id) {
            templates.remove(&id);
        }
        Ok(())
    }

    async fn template_exists(&self, id: Snowflake) -> Result<bool, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.contains_key(&id))
    }

    async fn template_name_exists(
        &self,
        owner_id: Snowflake,
        name: &str,
        exclude_id: Option<Snowflake>,
    ) -> Result<bool, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.values().any(|t| {
            t.owner_id == owner_id && t.name == name && exclude_id.is_none_or(|eid| t.id != eid)
        }))
    }

    async fn get_active_template(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let template = self
            .user_state
            .read()
            .await
            .get(&user_id)
            .ok_or(StorageError::NotFound(format!(
                "no state for user: {user_id}"
            )))?
            .active_template_id
            .to_owned();

        Ok(template)
    }

    async fn set_active_template(
        &self,
        user_id: Snowflake,
        template_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.user_state
            .write()
            .await
            .entry(user_id)
            .and_modify(|e| e.active_template_id = template_id)
            .or_insert(UserSettings {
                active_template_id: template_id,
                active_persona_id: None,
                active_connection_profile_id: None
            });

        Ok(())
    }
}
