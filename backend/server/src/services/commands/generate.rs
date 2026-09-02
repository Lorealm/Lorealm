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
///
/// Narrower than [`ChatService`] on purpose — a command has no use for turn
/// resolution, persistence or streaming, and naming only what it uses is what
/// lets the bridge be exercised without a provider.
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
    pub cancel: CancellationToken,
}

/// `$[generate_raw(prompt)]`: send `prompt` to the model on its own and
/// return what comes back.
pub struct GenerateRawCommand {
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

impl WeaverCommand for GenerateRawCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let prompt = match args.into_iter().next() {
            Some(Value::String(s)) => s,
            Some(Value::None) | None => {
                return Err(EvalError::host_error("generate_raw: prompt is required"));
            }
            Some(other) => other.to_output_string(),
        };
        if prompt.trim().is_empty() {
            return Err(EvalError::host_error("generate_raw: prompt is empty"));
        }

        let Some(bridge) = &self.bridge else {
            return Err(EvalError::host_error(
                "generate_raw is only available inside a command",
            ));
        };

        let text = bridge.handle.block_on(async {
            let profile = bridge
                .state
                .get_active_profile(bridge.owner)
                .await
                .map_err(|e| EvalError::host_error(format!("generate_raw: {e}")))?;

            bridge
                .chat
                .complete(
                    bridge.owner,
                    profile.id,
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
            description: "Send a prompt to the model on its own — no character, \
                          history or lorebook — and return its reply"
                .into(),
            returns: ValueType::String,
            params: vec![ParamDef {
                name: "prompt".into(),
                description: "What to ask the model".into(),
                expected_type: Some(ValueType::String),
                required: true,
            }],
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
        assert!(
            registry
                .command_signatures()
                .iter()
                .any(|s| s.name == "generate_raw")
        );

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
