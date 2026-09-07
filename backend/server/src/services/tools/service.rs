use std::sync::Arc;

use chrono::Utc;
use lorealm_storage::{
    Storage,
    models::{Param, ParamType, Snowflake, ToolSurface, UserTool},
};
use weaver_lang::CompiledTemplate;

use crate::{
    errors::{TargetType, UserToolError},
    services::{
        snowflake::SnowflakeService,
        tools::{CreateUserTool, UpdateUserTool, builtin_tools},
    },
    util::weaver::describe_parse_errors,
};

/// Command names a user may not take.
const RESERVED_COMMAND_NAMES: [&str; 1] = ["send"];

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

    /// Enabled rows the model may call on a turn.
    pub async fn list_enabled(&self, user_id: Snowflake) -> Result<Vec<UserTool>, UserToolError> {
        let tools = self.storage.list_enabled_tools(user_id).await?;
        Ok(tools
            .into_iter()
            .filter(|t| t.surface.callable_by_model())
            .collect())
    }

    /// Enabled rows the user may invoke as `/name`, shipped ones first.
    pub async fn list_commands(&self, user_id: Snowflake) -> Result<Vec<UserTool>, UserToolError> {
        let mut commands: Vec<UserTool> = builtin_tools()
            .iter()
            .filter(|t| t.surface.callable_by_user())
            .cloned()
            .collect();

        let stored = self.storage.list_enabled_tools(user_id).await?;
        commands.extend(stored.into_iter().filter(|t| t.surface.callable_by_user()));
        Ok(commands)
    }

    pub async fn create_tool(
        &self,
        user_id: Snowflake,
        create: CreateUserTool,
    ) -> Result<Snowflake, UserToolError> {
        let surface = create.surface.unwrap_or_default();
        Self::validate_name(&create.name)?;
        Self::validate_command_name(&create.name, surface)?;
        Self::validate_body(&create.body)?;
        Self::validate_params(&create.params)?;

        let id = self.snowflake.generate();
        let row = UserTool {
            user_id,
            name: create.name,
            id,
            display_name: create.display_name,
            description: create.description,
            params: create.params,
            body: create.body,
            surface,
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
            Self::validate_params(&params)?;
            tool.params = params;
        }
        if let Some(body) = update.body {
            Self::validate_body(&body)?;
            tool.body = body;
        }
        if let Some(surface) = update.surface {
            tool.surface = surface;
        }

        Self::validate_command_name(&tool.name, tool.surface)?;

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

    /// Reject a reserved name
    fn validate_command_name(name: &str, surface: ToolSurface) -> Result<(), UserToolError> {
        if surface.callable_by_user() && RESERVED_COMMAND_NAMES.contains(&name) {
            return Err(UserToolError::InvalidName(format!(
                "'{name}' is reserved and cannot be used as a command"
            )));
        }
        Ok(())
    }

    fn validate_params(params: &[Param]) -> Result<(), UserToolError> {
        for param in params {
            if param.resource.is_some() && !matches!(param.ty, ParamType::String) {
                return Err(UserToolError::InvalidParam {
                    name: param.name.clone(),
                    reason: "only a string parameter can name a resource".into(),
                });
            }
        }
        Ok(())
    }

    fn validate_body(body: &str) -> Result<(), UserToolError> {
        CompiledTemplate::compile(body)
            .map(drop)
            .map_err(|errs| UserToolError::InvalidBody(describe_parse_errors(body, &errs)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lorealm_storage::models::ResourceKind;

    fn param(ty: ParamType, resource: Option<ResourceKind>) -> Param {
        Param {
            name: "profile".into(),
            ty,
            description: None,
            required: false,
            resource,
        }
    }

    #[test]
    fn a_resource_hint_belongs_on_a_string_parameter() {
        assert!(
            UserToolService::validate_params(&[param(
                ParamType::String,
                Some(ResourceKind::ConnectionProfile)
            )])
            .is_ok()
        );
    }

    #[test]
    fn a_resource_hint_on_a_non_string_parameter_is_rejected_by_name() {
        let err = UserToolService::validate_params(&[param(
            ParamType::Number,
            Some(ResourceKind::ConnectionProfile),
        )])
        .unwrap_err();
        assert!(matches!(
            err,
            UserToolError::InvalidParam { ref name, .. } if name == "profile"
        ));

        assert!(
            UserToolService::validate_params(&[param(
                ParamType::Array(Box::new(ParamType::String)),
                Some(ResourceKind::Character)
            )])
            .is_err()
        );
    }

    #[test]
    fn a_command_may_not_take_a_reserved_name() {
        assert!(
            UserToolService::validate_command_name("send", ToolSurface::Command).is_err(),
            "plain composer text dispatches through /send"
        );
        assert!(UserToolService::validate_command_name("send", ToolSurface::Both).is_err());
    }

    #[test]
    fn a_tool_may_take_a_name_that_is_reserved_for_commands() {
        assert!(UserToolService::validate_command_name("send", ToolSurface::Tool).is_ok());
    }

    #[test]
    fn every_other_name_is_shadowable() {
        for name in ["newchat", "go", "back", "conversations", "summarize"] {
            assert!(
                UserToolService::validate_command_name(name, ToolSurface::Command).is_ok(),
                "{name} should be overridable"
            );
        }
    }

    #[test]
    fn plain_parameters_are_unaffected() {
        assert!(UserToolService::validate_params(&[param(ParamType::Number, None)]).is_ok());
        assert!(UserToolService::validate_params(&[]).is_ok());
    }

    #[test]
    fn the_hint_is_absent_from_json_rather_than_null() {
        let json = serde_json::to_value(param(ParamType::String, None)).unwrap();
        assert!(
            json.get("resource").is_none(),
            "existing stored params round-trip unchanged: {json}"
        );
    }
}
