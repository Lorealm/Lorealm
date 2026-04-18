use std::sync::Arc;

use eldoria_storage::{
    Storage,
    models::{Snowflake, TemplateMetadata},
};

use crate::{
    errors::ContextError,
    services::{
        context::{
            ContextNode, CreateNode, NodeWithPath, MoveNode, MovedNode, UpdateNode,
            providers::SourceRegistry, template::ContextTemplate,
        },
        snowflake::SnowflakeService,
    },
};

pub struct ContextService {
    storage: Arc<dyn Storage>,
    snowflake_service: Arc<SnowflakeService>,
    registry: Arc<SourceRegistry>,
}

impl ContextService {
    pub fn new(storage: Arc<dyn Storage>, snowflake_service: Arc<SnowflakeService>) -> Self {
        Self {
            storage,
            snowflake_service,
            registry: Arc::new(SourceRegistry::new()),
        }
    }

    pub async fn list_templates(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<TemplateMetadata>, ContextError> {
        Ok(self.storage.list_templates(user_id).await?)
    }

    pub async fn get_template(
        &self,
        template_id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<ContextTemplate, ContextError> {
        let template = self
            .storage
            .get_template_owned(template_id, owner_id)
            .await?
            .ok_or(ContextError::NotFound(template_id))?;

        Ok(ContextTemplate::from_db(template)?)
    }

    pub async fn create_template(
        &self,
        owner_id: Snowflake,
        name: Option<String>,
        desc: Option<String>,
        default: bool,
    ) -> Result<ContextTemplate, ContextError> {
        let mut template = if default {
            ContextTemplate::create_default(owner_id, &self.snowflake_service)
        } else {
            ContextTemplate::new("New Template", owner_id, &self.snowflake_service)
        };

        if let Some(n) = name {
            template.name = n;
        }
        if let Some(d) = desc {
            template.description = d;
        }

        self.storage.save_template(&template.to_db()?).await?;

        Ok(template)
    }

    pub async fn update_template(
        &self,
        template_id: Snowflake,
        owner_id: Snowflake,
        name: Option<String>,
        desc: Option<String>,
    ) -> Result<ContextTemplate, ContextError> {
        let mut template = self.get_template(template_id, owner_id).await?;

        if name.is_none() && desc.is_none() {
            return Ok(template);
        }

        if let Some(n) = name {
            template.name = n;
        }
        if let Some(d) = desc {
            template.description = d;
        }

        self.storage.save_template(&template.to_db()?).await?;

        Ok(template)
    }

    pub async fn delete_template(
        &self,
        template_id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<(), ContextError> {
        self.storage.delete_template(template_id, owner_id).await?;

        Ok(())
    }

    pub async fn add_node(
        &self,
        template_id: Snowflake,
        owner_id: Snowflake,
        request: impl Into<CreateNode>,
    ) -> Result<NodeWithPath, ContextError> {
        let mut template = self.get_template(template_id, owner_id).await?;
        let req = request.into();
        let node = ContextNode {
            id: self.snowflake_service.generate(),
            title: req.title,
            description: req.description,
            enabled: true,
            content: req.content,
        };

        let path = template.add_node(node, req.parent_path.as_ref(), req.index)?;
        self.storage.save_template(&template.to_db()?).await?;
        let node = template
            .get_node(&path)
            .cloned()
            .ok_or(ContextError::Template(
                crate::errors::TemplateError::InvalidPath(path.clone()),
            ))?;

        Ok(NodeWithPath { node, path })
    }

    pub async fn update_node(
        &self,
        template_id: Snowflake,
        node_id: Snowflake,
        owner_id: Snowflake,
        request: impl Into<UpdateNode>,
    ) -> Result<NodeWithPath, ContextError> {
        let mut template = self.get_template(template_id, owner_id).await?;
        let req = request.into();

        template.update_node(
            node_id,
            req.title,
            req.description,
            req.enabled,
            req.content,
        )?;

        self.storage.save_template(&template.to_db()?).await?;

        let path = template
            .find_node_path(node_id)
            .ok_or(ContextError::Template(
                crate::errors::TemplateError::NodeNotFound(node_id),
            ))?;
        let node = template
            .get_node(&path)
            .cloned()
            .ok_or(ContextError::Template(
                crate::errors::TemplateError::InvalidPath(path.clone()),
            ))?;

        Ok(NodeWithPath { node, path })
    }

    pub async fn delete_node(
        &self,
        template_id: Snowflake,
        node_id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<(), ContextError> {
        let mut template = self.get_template(template_id, owner_id).await?;

        template.remove_node(node_id)?;
        self.storage.save_template(&template.to_db()?).await?;

        Ok(())
    }

    pub async fn move_node(
        &self,
        template_id: Snowflake,
        node_id: Snowflake,
        owner_id: Snowflake,
        request: impl Into<MoveNode>,
    ) -> Result<MovedNode, ContextError> {
        let mut template = self.get_template(template_id, owner_id).await?;

        let req = request.into();

        let new_path = template.move_node(node_id, req.new_parent_path.as_ref(), req.new_index)?;
        self.storage.save_template(&template.to_db()?).await?;

        Ok(MovedNode { new_path })
    }

    pub async fn toggle_node(
        &self,
        template_id: Snowflake,
        node_id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<bool, ContextError> {
        let mut template = self.get_template(template_id, owner_id).await?;
        let enabled = template.toggle_node(node_id)?;
        self.storage.save_template(&template.to_db()?).await?;

        Ok(enabled)
    }

    pub fn with_registry<R>(&self, f: impl FnOnce(&SourceRegistry) -> R) -> R {
        f(&self.registry)
    }
}
