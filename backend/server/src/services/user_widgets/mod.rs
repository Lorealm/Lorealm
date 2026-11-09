use std::sync::Arc;

use chrono::Utc;
use lorealm_storage::{
    Storage,
    models::{Snowflake, UserWidget, WidgetMode, WidgetPlacement, is_identifier},
};
use serde::Deserialize;

use crate::{
    errors::{TargetType, UserWidgetError},
    services::{snowflake::SnowflakeService, tools::builtin_tool},
};

#[derive(Deserialize)]
pub struct CreateUserWidget {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub render: String,
    #[serde(default)]
    pub tools: Vec<Snowflake>,
    #[serde(default)]
    pub placement: WidgetPlacement,
    #[serde(default)]
    pub mode: WidgetMode,
    #[serde(default)]
    pub props: Vec<String>,
}

#[derive(Default, Deserialize)]
pub struct UpdateUserWidget {
    pub name: Option<String>,
    pub description: Option<String>,
    pub source: Option<String>,
    pub prompt: Option<String>,
    pub render: Option<String>,
    pub tools: Option<Vec<Snowflake>>,
    pub placement: Option<WidgetPlacement>,
    pub mode: Option<WidgetMode>,
    pub props: Option<Vec<String>>,
}

pub struct UserWidgetService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
}

impl UserWidgetService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    pub async fn get_widget(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<UserWidget, UserWidgetError> {
        let widget = self
            .storage
            .get_widget(id)
            .await?
            .ok_or(UserWidgetError::NotFound(TargetType::UserWidget, id))?;

        if widget.user_id != user_id {
            return Err(UserWidgetError::NotOwned);
        }

        Ok(widget)
    }

    pub async fn duplicate_widget(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<UserWidget, UserWidgetError> {
        let original = self.get_widget(id, user_id).await?;
        let copy = UserWidget {
            id: self.snowflake.generate(),
            name: format!("{} (copy)", original.name),
            enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..original
        };
        self.storage.save_widget(copy.clone()).await?;
        Ok(copy)
    }

    pub async fn list_widgets(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserWidget>, UserWidgetError> {
        Ok(self.storage.list_widgets(user_id).await?)
    }

    pub async fn create_widget(
        &self,
        user_id: Snowflake,
        create: CreateUserWidget,
    ) -> Result<UserWidget, UserWidgetError> {
        Self::validate_name(&create.name)?;
        self.validate_tools(user_id, &create.tools).await?;

        let row = UserWidget {
            id: self.snowflake.generate(),
            user_id,
            name: create.name,
            description: create.description,
            source: create.source,
            prompt: create.prompt,
            render: create.render,
            tools: create.tools,
            enabled: false,
            placement: create.placement,
            mode: create.mode,
            props: create.props,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.storage.save_widget(row.clone()).await?;
        Ok(row)
    }

    pub async fn update_widget(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        update: UpdateUserWidget,
    ) -> Result<UserWidget, UserWidgetError> {
        let mut widget = self.get_widget(id, user_id).await?;

        if let Some(name) = update.name {
            Self::validate_name(&name)?;
            widget.name = name;
        }
        if let Some(description) = update.description {
            widget.description = description;
        }
        if let Some(source) = update.source {
            widget.source = source;
        }
        if let Some(prompt) = update.prompt {
            widget.prompt = prompt;
        }
        if let Some(render) = update.render {
            widget.render = render;
        }
        if let Some(tools) = update.tools {
            self.validate_tools(user_id, &tools).await?;
            widget.tools = tools;
        }
        if let Some(placement) = update.placement {
            widget.placement = placement;
        }
        if let Some(mode) = update.mode {
            widget.mode = mode;
        }
        if let Some(props) = update.props {
            Self::validate_props(&props)?;
            widget.props = props;
        }

        self.storage.save_widget(widget.clone()).await?;
        Ok(widget)
    }

    pub async fn set_enabled(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        enabled: bool,
    ) -> Result<(), UserWidgetError> {
        self.get_widget(id, user_id).await?;
        self.storage.set_widget_enabled(id, enabled).await?;
        Ok(())
    }

    pub async fn delete_widget(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), UserWidgetError> {
        self.get_widget(id, user_id).await?; // ownership gate
        self.storage.delete_widget(id).await?;
        Ok(())
    }

    fn validate_props(props: &[String]) -> Result<(), UserWidgetError> {
        for p in props {
            if p.trim().is_empty() || !is_identifier(p) {
                return Err(UserWidgetError::Invalid(format!(
                    "prop name '{p}' must be a bare identifier"
                )));
            }
        }
        Ok(())
    }

    fn validate_name(name: &str) -> Result<(), UserWidgetError> {
        if name.trim().is_empty() {
            return Err(UserWidgetError::Invalid("a widget needs a name".into()));
        }
        Ok(())
    }

    async fn validate_tools(
        &self,
        user_id: Snowflake,
        tools: &[Snowflake],
    ) -> Result<(), UserWidgetError> {
        for &id in tools {
            if builtin_tool(id).is_some() {
                continue;
            }
            let tool = self
                .storage
                .get_tool(id)
                .await?
                .ok_or(UserWidgetError::NotFound(TargetType::UserTool, id))?;
            if tool.user_id != user_id {
                return Err(UserWidgetError::Invalid(format!(
                    "tool {id} is not yours to bundle"
                )));
            }
        }
        Ok(())
    }
}
