use std::sync::Arc;

use async_trait::async_trait;
use lorealm_storage::models::Snowflake;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use weaver_lang::{
    EvalContext, EvalError, Registry, Value, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

use crate::{
    errors::ChatError,
    services::{
        chat::{CancelGuard, ChatService, PromptMessage},
        context::PromptRole,
        state::StateService,
    },
};

/// What a command run needs from the chat stack: run one completion, and
/// register a cancellation token where the existing cancel endpoint finds it.
#[async_trait]
pub trait Generation: Send + Sync {
    fn begin_cancel(&self, conversation_id: Snowflake) -> (CancellationToken, CancelGuard);

    async fn complete(
        &self,
        owner: Snowflake,
        profile_id: Snowflake,
        prompt: Vec<PromptMessage>,
        cancel: CancellationToken,
    ) -> Result<String, ChatError>;

    async fn complete_with_context(
        &self,
        owner: Snowflake,
        conversation_id: Snowflake,
        ask: String,
        profile_id: Snowflake,
        cancel: CancellationToken,
    ) -> Result<String, ChatError>;
}

#[async_trait]
impl Generation for ChatService {
    fn begin_cancel(&self, conversation_id: Snowflake) -> (CancellationToken, CancelGuard) {
        ChatService::begin_cancel(self, conversation_id)
    }

    async fn complete(
        &self,
        owner: Snowflake,
        profile_id: Snowflake,
        prompt: Vec<PromptMessage>,
        cancel: CancellationToken,
    ) -> Result<String, ChatError> {
        ChatService::complete(self, owner, profile_id, prompt, cancel).await
    }

    async fn complete_with_context(
        &self,
        owner: Snowflake,
        conversation_id: Snowflake,
        ask: String,
        profile_id: Snowflake,
        cancel: CancellationToken,
    ) -> Result<String, ChatError> {
        ChatService::complete_with_context(self, owner, conversation_id, ask, profile_id, cancel)
            .await
    }
}

/// What `$[generate_raw]` needs to reach the model from a synchronous body.
///
/// Weaver evaluates synchronously, so the body runs on a blocking thread
/// and blocks *that* thread on the inference future. Constructing this off
/// a runtime worker thread and calling it there would panic, so only
/// [`CommandService::run`] builds one, and only ever inside `spawn_blocking`.
///
/// [`CommandService::run`]: crate::services::commands::CommandService::run
pub struct GenerateBridge {
    pub handle: Handle,
    pub chat: Arc<dyn Generation>,
    /// Resolves the active connection profile at the moment of use
    pub state: Arc<StateService>,
    pub owner: Snowflake,
    /// The conversation `$[generate]` assembles context from.
    pub conversation: Snowflake,
    pub cancel: CancellationToken,
}

impl GenerateBridge {
    /// The profile to answer with: the one the body named, else whichever is
    /// active at this moment.
    async fn profile_id(&self, named: Option<Snowflake>) -> Result<Snowflake, EvalError> {
        match named {
            Some(id) => Ok(id),
            None => Ok(self
                .state
                .get_active_profile(self.owner)
                .await
                .map_err(|e| EvalError::host_error(format!("no connection profile: {e}")))?
                .id),
        }
    }
}

/// The `(prompt, profile?)` both generate commands take.
fn generate_args(
    command: &str,
    args: Vec<Value>,
) -> Result<(String, Option<Snowflake>), EvalError> {
    let mut args = args.into_iter();

    let prompt = match args.next() {
        Some(Value::String(s)) => s,
        Some(Value::None) | None => {
            return Err(EvalError::host_error(format!(
                "{command}: prompt is required"
            )));
        }
        Some(other) => other.to_output_string(),
    };
    if prompt.trim().is_empty() {
        return Err(EvalError::host_error(format!("{command}: prompt is empty")));
    }

    let profile = match args.next() {
        None | Some(Value::None) => None,
        Some(Value::String(id)) => Some(id.parse::<Snowflake>().map_err(|_| {
            EvalError::host_error(format!("{command}: '{id}' is not a profile id"))
        })?),
        // A snowflake does not survive an f64, so an unquoted one is already
        // the wrong number by the time it arrives here.
        Some(Value::Number(_)) => {
            return Err(EvalError::host_error(format!(
                "{command}: quote the profile id — an unquoted one loses precision"
            )));
        }
        Some(other) => {
            return Err(EvalError::host_error(format!(
                "{command}: profile must be a profile id, got {other:?}"
            )));
        }
    };

    Ok((prompt, profile))
}

/// The `profile` parameter, identical on both commands.
fn profile_param() -> ParamDef {
    ParamDef {
        name: "profile".into(),
        description: "Connection profile id to answer with, quoted. Defaults to the \
                      active profile"
            .into(),
        expected_type: Some(ValueType::String),
        required: false,
    }
}

/// `$[generate_raw(prompt)]`: send `prompt` to the model on its own and
/// return what comes back.
pub struct GenerateRawCommand {
    bridge: Option<GenerateBridge>,
}

/// `$[generate(prompt)]`: ask the model with the conversation's context
pub struct GenerateCommand {
    bridge: Option<GenerateBridge>,
}

impl GenerateRawCommand {
    pub fn new(bridge: GenerateBridge) -> Self {
        Self {
            bridge: Some(bridge),
        }
    }

    /// The signature without the machinery, for the editor's name resolution
    /// and the vocabulary endpoint. Calling it is an error.
    pub fn declaration() -> Self {
        Self { bridge: None }
    }
}

impl GenerateCommand {
    pub fn new(bridge: GenerateBridge) -> Self {
        Self {
            bridge: Some(bridge),
        }
    }

    /// See [`GenerateRawCommand::declaration`].
    pub fn declaration() -> Self {
        Self { bridge: None }
    }
}

/// The bridge, or the error a body gets for calling this outside a command.
fn bridge_of<'a>(
    bridge: &'a Option<GenerateBridge>,
    command: &str,
) -> Result<&'a GenerateBridge, EvalError> {
    bridge.as_ref().ok_or_else(|| {
        EvalError::host_error(format!("{command} is only available inside a command"))
    })
}

impl WeaverCommand for GenerateRawCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let (prompt, profile) = generate_args("generate_raw", args)?;
        let bridge = bridge_of(&self.bridge, "generate_raw")?;

        // Already on a blocking thread, so blocking again is free of the usual
        // hazard: this is not a runtime worker.
        let text = bridge.handle.block_on(async {
            let profile_id = bridge.profile_id(profile).await?;
            bridge
                .chat
                .complete(
                    bridge.owner,
                    profile_id,
                    vec![PromptMessage {
                        role: PromptRole::User,
                        content: prompt,
                    }],
                    bridge.cancel.clone(),
                )
                .await
                .map_err(|e| EvalError::host_error(format!("generate_raw failed: {e}")))
        })?;

        Ok(Some(Value::String(text)))
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "generate_raw".into(),
            description: "Send a barebones prompt to the model."
                .into(),
            returns: ValueType::String,
            params: vec![
                ParamDef {
                    name: "prompt".into(),
                    description: "What to ask the model".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                profile_param(),
            ],
            mutates_context: false,
        }
    }
}

impl WeaverCommand for GenerateCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let (prompt, profile) = generate_args("generate", args)?;
        let bridge = bridge_of(&self.bridge, "generate")?;

        let text = bridge.handle.block_on(async {
            let profile_id = bridge.profile_id(profile).await?;
            bridge
                .chat
                .complete_with_context(
                    bridge.owner,
                    bridge.conversation,
                    prompt,
                    profile_id,
                    bridge.cancel.clone(),
                )
                .await
                .map_err(|e| EvalError::host_error(format!("generate failed: {e}")))
        })?;

        Ok(Some(Value::String(text)))
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "generate".into(),
            description: "Send a prompt to the model with the relevant context included."
                .into(),
            returns: ValueType::String,
            params: vec![
                ParamDef {
                    name: "prompt".into(),
                    description: "What to ask, appended after the history".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                profile_param(),
            ],
            mutates_context: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tools::{ToolContext, ToolStatePool, build_tool_registry};
    use std::{collections::HashMap, sync::Mutex};
    use weaver_lang::CompiledTemplate;

    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    /// The bridge moves a body onto a blocking thread, which only compiles
    /// while these hold.
    #[test]
    fn everything_the_body_needs_can_cross_onto_a_blocking_thread() {
        assert_send::<ToolContext>();
        assert_send::<CompiledTemplate>();
        assert_send_sync::<Registry>();
    }

    #[test]
    fn without_a_bridge_the_call_is_refused_but_the_name_still_resolves() {
        let mut registry = build_tool_registry();
        registry.register_command(GenerateRawCommand::declaration());
        registry.register_command(GenerateCommand::declaration());
        let names: Vec<String> = registry
            .command_signatures()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert!(names.contains(&"generate_raw".to_string()));
        assert!(names.contains(&"generate".to_string()));

        let mut ctx = ToolContext::new(
            HashMap::new(),
            Arc::new(Mutex::new(ToolStatePool::default())),
        );
        let err = CompiledTemplate::compile(r#"$[generate_raw("hi")]"#)
            .unwrap()
            .evaluate(&mut ctx, &registry)
            .unwrap_err();
        assert!(err.to_string().contains("only available inside a command"));
    }

    #[test]
    fn generate_is_refused_without_a_bridge_too() {
        let mut registry = build_tool_registry();
        registry.register_command(GenerateCommand::declaration());
        let mut ctx = ToolContext::new(
            HashMap::new(),
            Arc::new(Mutex::new(ToolStatePool::default())),
        );
        let err = CompiledTemplate::compile(r#"$[generate("hi")]"#)
            .unwrap()
            .evaluate(&mut ctx, &registry)
            .unwrap_err();
        assert!(err.to_string().contains("only available inside a command"));
    }

    #[test]
    fn an_empty_prompt_is_rejected_before_any_network_call() {
        let mut registry = build_tool_registry();
        registry.register_command(GenerateRawCommand::declaration());
        let mut ctx = ToolContext::new(
            HashMap::new(),
            Arc::new(Mutex::new(ToolStatePool::default())),
        );
        // Arguments are validated ahead of the bridge, so this reports the
        // real mistake rather than "not available here".
        let err = CompiledTemplate::compile(r#"$[generate_raw("   ")]"#)
            .unwrap()
            .evaluate(&mut ctx, &registry)
            .unwrap_err();
        assert!(err.to_string().contains("prompt is empty"));
    }
}
