use std::sync::Arc;

use chrono::Utc;
use lorealm_storage::{
    Storage,
    models::{Snowflake, UserTool},
};
use weaver_lang::CompiledTemplate;

use crate::{
    errors::{TargetType, UserToolError},
    services::{
        snowflake::SnowflakeService,
        tools::{CreateUserTool, UpdateUserTool},
    },
};

pub struct UserToolService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
}

impl UserToolService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    pub async fn get_tool(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<UserTool, UserToolError> {
        let tool = self
            .storage
            .get_tool(id)
            .await?
            .ok_or(UserToolError::NotFound(TargetType::UserTool, id))?;

        if tool.user_id != user_id {
            return Err(UserToolError::NotOwned);
        }

        Ok(tool)
    }

    pub async fn list_tools(&self, user_id: Snowflake) -> Result<Vec<UserTool>, UserToolError> {
        Ok(self.storage.list_tools(user_id).await?)
    }

    pub async fn list_enabled(&self, user_id: Snowflake) -> Result<Vec<UserTool>, UserToolError> {
        Ok(self.storage.list_enabled_tools(user_id).await?)
    }

    pub async fn create_tool(
        &self,
        user_id: Snowflake,
        create: CreateUserTool,
    ) -> Result<Snowflake, UserToolError> {
        Self::validate_name(&create.name)?;
        Self::validate_body(&create.body)?;

        let id = self.snowflake.generate();
        let row = UserTool {
            user_id,
            name: create.name,
            id,
            display_name: create.display_name,
            description: create.description,
            params: create.params,
            body: create.body,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.storage.save_tool(row).await?;
        Ok(id)
    }

    pub async fn update_tool(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        update: UpdateUserTool,
    ) -> Result<UserTool, UserToolError> {
        let mut tool = self.get_tool(id, user_id).await?;

        if let Some(name) = update.name {
            Self::validate_name(&name)?;
            tool.name = name;
        }
        if let Some(display_name) = update.display_name {
            tool.display_name = display_name;
        }
        if let Some(description) = update.description {
            tool.description = description;
        }
        if let Some(params) = update.params {
            tool.params = params;
        }
        if let Some(body) = update.body {
            Self::validate_body(&body)?;
            tool.body = body;
        }

        self.storage.save_tool(tool.clone()).await?;
        Ok(tool)
    }

    pub async fn set_enabled(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        enabled: bool,
    ) -> Result<(), UserToolError> {
        self.get_tool(id, user_id).await?;
        self.storage.set_tool_enabled(id, enabled).await?;
        Ok(())
    }

    pub async fn delete_tool(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), UserToolError> {
        self.get_tool(id, user_id).await?; // ownership gate
        self.storage.delete_tool(id).await?;
        Ok(())
    }

    fn validate_name(name: &str) -> Result<(), UserToolError> {
        let ok = !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        ok.then_some(())
            .ok_or_else(|| UserToolError::InvalidName(name.to_string()))
    }

    fn validate_body(body: &str) -> Result<(), UserToolError> {
        CompiledTemplate::compile(body)
            .map(drop)
            .map_err(|errs| UserToolError::InvalidBody(format!("{errs:?}")))
    }
}
