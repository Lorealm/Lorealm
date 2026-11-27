use std::{collections::HashMap, sync::Arc, sync::Mutex as StdMutex};

use lorealm_storage::{
    Storage,
    models::{ChatRole, Param, ParamType, Snowflake, UserTool},
};
use serde_json::json;
use weaver_lang::Value as WeaverValue;

use crate::{
    errors::CommandError,
    services::{
        commands::{CommandArgs, CommandEffect, CommandRun, GenerateBridge, Generation},
        conversation::ConversationService,
        plugins::PluginService,
        state::{CapturedState, StateService},
        tools::{UserToolService, builtin_tool},
        weaver::{WeaverHost, WeaverProfile, compile_template},
    },
    util::weaver::json_to_weaver,
};

pub struct CommandService {
    storage: Arc<dyn Storage>,
    conversations: Arc<ConversationService>,
    state: Arc<StateService>,
    tools: Arc<UserToolService>,
    chat: Arc<dyn Generation>,
    plugins: Arc<PluginService>,
}

impl CommandService {
    pub fn new(
        storage: Arc<dyn Storage>,
        conversations: Arc<ConversationService>,
        state: Arc<StateService>,
        tools: Arc<UserToolService>,
        chat: Arc<dyn Generation>,
        plugins: Arc<PluginService>,
    ) -> Self {
        Self {
            storage,
            conversations,
            state,
            tools,
            chat,
            plugins,
        }
    }

    /// Every command this user can invoke as `/name`.
    pub async fn list(&self, user_id: Snowflake) -> Result<Vec<UserTool>, CommandError> {
        Ok(self.tools.list_commands(user_id).await?)
    }

    /// Evaluate one command body against a conversation.
    pub async fn run(
        &self,
        user_id: Snowflake,
        conversation_id: Snowflake,
        command_id: Snowflake,
        args: CommandArgs,
    ) -> Result<CommandRun, CommandError> {
        let conversation = self
            .storage
            .get_conversation(conversation_id)
            .await?
            .ok_or(CommandError::NotFound(conversation_id))?;
        if conversation.user_id != user_id {
            return Err(CommandError::NotOwned(conversation_id));
        }

        // Shipped rows belong to no user, so the ownership gate in `get_tool`
        // would reject them; `infer` resolves the model's builtins the same way.
        let command = match builtin_tool(command_id) {
            Some(shipped) => shipped,
            None => self.tools.get_tool(command_id, user_id).await?,
        };
        if !command.surface.callable_by_user() {
            return Err(CommandError::NotACommand(command.name));
        }

        let thread = self.conversations.active_thread(conversation_id).await?;
        let ids: Vec<Snowflake> = thread.iter().map(|m| m.id).collect();
        let reconstructed = self.state.reconstruct(&ids).await?;
        let baseline = reconstructed.persistent;

        let (cancel, _cancel_guard) = self.chat.begin_cancel(conversation_id);

        let failed = |reason: String| CommandError::Failed {
            name: command.name.clone(),
            reason,
        };

        let host = WeaverHost::builder()
            .profile(WeaverProfile::Command)
            .args(bind_args(&command.name, &command.params, &args)?)
            .pool(Arc::new(StdMutex::new(baseline.clone())))
            .generation(GenerateBridge {
                handle: tokio::runtime::Handle::current(),
                chat: Arc::clone(&self.chat),
                state: Arc::clone(&self.state),
                owner: user_id,
                conversation: conversation_id,
                cancel: cancel.clone(),
            })
            .cancel(cancel.clone())
            .plugins(self.plugins.vocabulary(user_id).await)
            .build();

        let body = compile_template(&command.body).map_err(|e| failed(e.to_string()))?;
        let evaluated = host.evaluate(body).await;

        if cancel.is_cancelled() {
            return Err(CommandError::Cancelled(command.name.clone()));
        }

        let outcome = evaluated.map_err(|e| failed(e.to_string()))?;
        let output = outcome
            .to_json()
            .map_err(|e| failed(format!("returned {e}")))?;
        let wrote_state = outcome.wrote_state();
        let effects = outcome.effects;
        let delta = outcome.delta;

        // Apply all sys effects
        let sys: Vec<&str> = effects
            .iter()
            .filter_map(|e| match e {
                CommandEffect::Sys { content } => Some(content.as_str()),
                _ => None,
            })
            .collect();
        let message = if sys.is_empty() && !wrote_state {
            None
        } else {
            let message = self
                .conversations
                .append_message(
                    conversation_id,
                    ChatRole::System,
                    sys.join("\n"),
                    json!({ "command": command.name }),
                )
                .await?;
            self.state
                .persist(
                    message.id,
                    CapturedState {
                        activation: reconstructed.activation,
                        persistent: delta,
                    },
                )
                .await?;
            Some(message)
        };

        Ok(CommandRun {
            output,
            effects: effects
                .into_iter()
                .filter(|e| !matches!(e, CommandEffect::Sys { .. }))
                .collect(),
            message,
        })
    }
}

/// Bind `key=value` arguments onto the command's declared params, coercing
/// each to the type it declared. The trailing text is always `{{cmd:rest}}`.
fn bind_args(
    command: &str,
    params: &[Param],
    args: &CommandArgs,
) -> Result<HashMap<String, WeaverValue>, CommandError> {
    let mut bound = HashMap::new();

    for param in params {
        match args.named.get(&param.name) {
            Some(raw) => {
                bound.insert(param.name.clone(), coerce(&param.name, raw, &param.ty)?);
            }
            None if param.required => {
                return Err(CommandError::BadArgument {
                    name: param.name.clone(),
                    reason: format!("required by /{command}"),
                });
            }
            None => {
                bound.insert(param.name.clone(), WeaverValue::None);
            }
        }
    }

    bound.insert("rest".into(), WeaverValue::String(args.rest.clone()));
    Ok(bound)
}

fn coerce(name: &str, raw: &str, ty: &ParamType) -> Result<WeaverValue, CommandError> {
    let bad = |reason: String| CommandError::BadArgument {
        name: name.to_string(),
        reason,
    };

    Ok(match ty {
        ParamType::String => WeaverValue::String(raw.to_string()),
        ParamType::Number => WeaverValue::Number(
            raw.parse::<f64>()
                .map_err(|_| bad(format!("'{raw}' is not a number")))?,
        ),
        ParamType::Bool => match raw {
            "true" | "yes" | "1" => WeaverValue::Bool(true),
            "false" | "no" | "0" => WeaverValue::Bool(false),
            other => return Err(bad(format!("'{other}' is not true or false"))),
        },
        // No shorthand for these: a composer argument spelling an array or an
        // object writes it as JSON.
        ParamType::Array(_) | ParamType::Object => {
            let parsed: serde_json::Value =
                serde_json::from_str(raw).map_err(|e| bad(format!("expected JSON, but {e}")))?;
            json_to_weaver(&parsed).map_err(|e| bad(e.to_string()))?
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::snowflake::SnowflakeService;
    use chrono::Utc;
    use lorealm_storage::{
        ConnectionStorage, ConversationStorage, UserToolStorage,
        backends::MemoryStorage,
        models::{ConnectionProfile, Conversation, ToolSurface},
    };

    /// Answers every completion with a fixed string, recording what it was
    /// asked.
    struct StubGenerator {
        reply: String,
        seen: StdMutex<Vec<String>>,
        /// Which profile each completion was asked to answer with.
        profiles: StdMutex<Vec<Snowflake>>,
        in_context: StdMutex<Vec<String>>,
        cancellations: dashmap::DashMap<Snowflake, tokio_util::sync::CancellationToken>,
        /// Wait for cancellation instead of answering
        hang: bool,
    }

    impl StubGenerator {
        fn new(reply: &str) -> Arc<Self> {
            Arc::new(Self {
                reply: reply.into(),
                seen: StdMutex::new(Vec::new()),
                profiles: StdMutex::new(Vec::new()),
                in_context: StdMutex::new(Vec::new()),
                cancellations: dashmap::DashMap::new(),
                hang: false,
            })
        }

        fn hanging() -> Arc<Self> {
            Arc::new(Self {
                reply: String::new(),
                seen: StdMutex::new(Vec::new()),
                profiles: StdMutex::new(Vec::new()),
                in_context: StdMutex::new(Vec::new()),
                cancellations: dashmap::DashMap::new(),
                hang: true,
            })
        }

        /// Cancel whatever run is in flight for `conversation_id` the way the
        /// `DELETE /chat/{id}/completions` handler does.
        fn cancel(&self, conversation_id: Snowflake) -> bool {
            match self.cancellations.get(&conversation_id) {
                Some(token) => {
                    token.cancel();
                    true
                }
                None => false,
            }
        }
    }

    #[async_trait::async_trait]
    impl Generation for StubGenerator {
        fn begin_cancel(
            &self,
            conversation_id: Snowflake,
        ) -> (
            tokio_util::sync::CancellationToken,
            crate::services::chat::CancelGuard,
        ) {
            let token = tokio_util::sync::CancellationToken::new();
            self.cancellations.insert(conversation_id, token.clone());
            (
                token,
                crate::services::chat::CancelGuard {
                    map: Arc::new(dashmap::DashMap::new()),
                    key: conversation_id,
                },
            )
        }

        fn try_begin_cancel(
            &self,
            conversation_id: Snowflake,
        ) -> Option<(
            tokio_util::sync::CancellationToken,
            crate::services::chat::CancelGuard,
        )> {
            Some(self.begin_cancel(conversation_id))
        }

        async fn complete(
            &self,
            _owner: Snowflake,
            profile_id: Snowflake,
            prompt: Vec<crate::services::chat::PromptMessage>,
            cancel: tokio_util::sync::CancellationToken,
        ) -> Result<String, crate::errors::ChatError> {
            self.seen
                .lock()
                .unwrap()
                .extend(prompt.into_iter().map(|m| m.content));
            self.profiles.lock().unwrap().push(profile_id);
            if self.hang {
                cancel.cancelled().await;
            }
            Ok(self.reply.clone())
        }

        async fn complete_with_context(
            &self,
            owner: Snowflake,
            _conversation_id: Snowflake,
            ask: String,
            profile_id: Snowflake,
            cancel: tokio_util::sync::CancellationToken,
        ) -> Result<String, crate::errors::ChatError> {
            self.in_context.lock().unwrap().push(ask.clone());
            self.complete(
                owner,
                profile_id,
                vec![crate::services::chat::PromptMessage {
                    role: crate::services::context_template::PromptRole::User,
                    content: ask,
                }],
                cancel,
            )
            .await
        }
    }

    struct Harness {
        service: CommandService,
        generator: Arc<StubGenerator>,
        active_profile: Snowflake,
        store: Arc<MemoryStorage>,
        snow: Arc<SnowflakeService>,
        user: Snowflake,
        conversation: Snowflake,
    }

    async fn harness() -> Harness {
        harness_replying("").await
    }

    async fn harness_replying(reply: &str) -> Harness {
        build(StubGenerator::new(reply)).await
    }

    async fn hanging_harness() -> Harness {
        build(StubGenerator::hanging()).await
    }

    async fn build(generator: Arc<StubGenerator>) -> Harness {
        let snow = Arc::new(SnowflakeService::new(0));
        let store = Arc::new(MemoryStorage::new());
        let user = snow.generate();
        let now = Utc::now();
        let conversation = Conversation {
            id: snow.generate(),
            user_id: user,
            character_id: Some(snow.generate()),
            char_version_id: Some(snow.generate()),
            realm_id: None,
            active_leaf_id: None,
            title: None,
            author_note: String::new(),
            created_at: now,
            updated_at: now,
        };
        let conversation_id = conversation.id;
        store.create_conversation(&conversation).await.unwrap();

        let conversations = Arc::new(ConversationService::new(store.clone(), snow.clone(), 64));
        let state = Arc::new(StateService::new(store.clone()));
        let tools = Arc::new(UserToolService::new(store.clone(), snow.clone()));
        // `$[generate_raw]` resolves the active profile at the moment of use,
        // so only the generation tests need this to exist.
        let profile = ConnectionProfile {
            id: snow.generate(),
            user_id: user,
            secret_id: None,
            name: "test".into(),
            provider: "openai".into(),
            model: Some("stub".into()),
            token_budget: 4096,
            common_params: serde_json::json!({}),
            provider_params: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        };
        store.save_profile(&profile).await.unwrap();
        store
            .set_active_profile(user, Some(profile.id))
            .await
            .unwrap();

        let connection = Arc::new(crate::services::connection::ConnectionService::new(
            store.clone(),
            Arc::clone(&snow),
            Arc::new(lorealm_encryption::LocalKeyManager::new_random()),
            reqwest::Client::new(),
        ));
        let plugins = Arc::new(PluginService::new(
            store.clone(),
            Arc::clone(&snow),
            Arc::clone(&state),
            connection,
            Default::default(),
            // No plugins installed.
            std::path::PathBuf::new(),
        ));
        Harness {
            service: CommandService::new(
                store.clone(),
                conversations,
                state,
                tools,
                Arc::clone(&generator) as Arc<dyn Generation>,
                plugins,
            ),
            generator,
            active_profile: profile.id,
            store,
            snow,
            user,
            conversation: conversation_id,
        }
    }

    impl Harness {
        async fn define(&self, name: &str, body: &str, surface: ToolSurface) -> Snowflake {
            let id = self.snow.generate();
            let now = Utc::now();
            self.store
                .save_tool(UserTool {
                    user_id: self.user,
                    name: name.into(),
                    id,
                    display_name: None,
                    description: String::new(),
                    params: Vec::new(),
                    body: body.into(),
                    surface,
                    enabled: true,
                    created_at: now,
                    updated_at: now,
                })
                .await
                .unwrap();
            id
        }

        async fn add_profile(&self, name: &str) -> Snowflake {
            let now = Utc::now();
            let profile = ConnectionProfile {
                id: self.snow.generate(),
                user_id: self.user,
                secret_id: None,
                name: name.into(),
                provider: "openai".into(),
                model: Some(name.into()),
                token_budget: 4096,
                common_params: serde_json::json!({}),
                provider_params: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            };
            let id = profile.id;
            self.store.save_profile(&profile).await.unwrap();
            id
        }

        async fn run(&self, id: Snowflake, rest: &str) -> Result<CommandRun, CommandError> {
            self.service
                .run(
                    self.user,
                    self.conversation,
                    id,
                    CommandArgs {
                        named: HashMap::new(),
                        rest: rest.into(),
                    },
                )
                .await
        }
    }

    #[tokio::test]
    async fn a_body_that_only_computes_leaves_no_trace_in_the_conversation() {
        let h = harness().await;
        let id = h
            .define("double", "{# return 21 * 2 #}", ToolSurface::Command)
            .await;

        let run = h.run(id, "").await.unwrap();
        assert_eq!(run.output, json!(42));
        assert!(
            run.message.is_none(),
            "nothing was written, so nothing anchors"
        );
        assert!(run.effects.is_empty());
    }

    #[tokio::test]
    async fn sys_output_becomes_the_anchor_message_and_never_a_client_effect() {
        let h = harness().await;
        let id = h
            .define("greet", r#"$[sys("Hello there.")]"#, ToolSurface::Command)
            .await;

        let run = h.run(id, "").await.unwrap();
        let message = run.message.expect("sys text anchors a message");
        assert_eq!(message.content, "Hello there.");
        assert_eq!(message.role, ChatRole::System);
        assert_eq!(message.metadata, json!({ "command": "greet" }));
        assert!(run.effects.is_empty(), "sys is realized server-side");
    }

    #[tokio::test]
    async fn a_toast_travels_back_without_anchoring_anything() {
        let h = harness().await;
        let id = h
            .define(
                "notify",
                r#"$[toast("Saved.", "success")]"#,
                ToolSurface::Command,
            )
            .await;

        let run = h.run(id, "").await.unwrap();
        assert!(run.message.is_none());
        assert!(matches!(
            run.effects.as_slice(),
            [CommandEffect::Toast { variant, .. }] if variant == "success"
        ));
    }

    #[tokio::test]
    async fn a_state_write_anchors_a_message_and_the_next_run_reads_it_back() {
        let h = harness().await;
        let write = h
            .define(
                "setgold",
                r#"$[set_var("global:gold", 10)]"#,
                ToolSurface::Command,
            )
            .await;
        let read = h
            .define(
                "getgold",
                "{# return {{global:gold}} #}",
                ToolSurface::Command,
            )
            .await;

        // Nothing rendered, but the write still needs somewhere to live.
        let first = h.run(write, "").await.unwrap();
        let anchor = first.message.expect("a state write anchors a message");
        assert_eq!(anchor.content, "", "no sys text, so the anchor is empty");

        // Reconstructed from the active path, which now runs through the anchor.
        assert_eq!(h.run(read, "").await.unwrap().output, json!(10));
    }

    #[tokio::test]
    async fn trailing_text_reaches_the_body_as_cmd_rest() {
        let h = harness().await;
        let id = h
            .define("echo", "{# return {{cmd:rest}} #}", ToolSurface::Command)
            .await;

        assert_eq!(
            h.run(id, "for initiative").await.unwrap().output,
            json!("for initiative")
        );
    }

    #[tokio::test]
    async fn a_tool_surface_row_is_not_invocable_as_a_command() {
        let h = harness().await;
        let id = h.define("modelonly", "hi", ToolSurface::Tool).await;

        assert!(matches!(
            h.run(id, "").await,
            Err(CommandError::NotACommand(ref name)) if name == "modelonly"
        ));
    }

    #[tokio::test]
    async fn both_is_reachable_from_either_side() {
        let h = harness().await;
        let id = h
            .define("shared", "{# return 1 #}", ToolSurface::Both)
            .await;
        assert!(h.run(id, "").await.is_ok());
    }

    #[tokio::test]
    async fn a_body_that_fails_reports_the_command_rather_than_a_bare_eval_error() {
        let h = harness().await;
        let id = h
            .define(
                "broken",
                r#"$[toast("x", "catastrophe")]"#,
                ToolSurface::Command,
            )
            .await;

        assert!(matches!(
            h.run(id, "").await,
            Err(CommandError::Failed { ref name, .. }) if name == "broken"
        ));
    }

    #[tokio::test]
    async fn a_realm_playthrough_runs_commands() {
        let h = harness().await;
        let now = Utc::now();
        let realm_convo = Conversation {
            id: h.snow.generate(),
            user_id: h.user,
            character_id: None,
            char_version_id: None,
            realm_id: Some(h.snow.generate()),
            active_leaf_id: None,
            title: None,
            author_note: String::new(),
            created_at: now,
            updated_at: now,
        };
        let realm_id = realm_convo.id;
        h.store.create_conversation(&realm_convo).await.unwrap();
        let id = h.define("noop", "hi", ToolSurface::Command).await;

        let run = h
            .service
            .run(h.user, realm_id, id, CommandArgs::default())
            .await
            .unwrap();
        assert_eq!(run.output, json!("hi"));
    }

    #[tokio::test]
    async fn a_body_blocks_on_generation_and_gets_the_text_back() {
        let h = harness_replying("The Rusty Tankard").await;
        let id = h
            .define(
                "tavern",
                r#"{# return $[generate_raw("Name a tavern")] #}"#,
                ToolSurface::Command,
            )
            .await;

        assert_eq!(
            h.run(id, "").await.unwrap().output,
            json!("The Rusty Tankard")
        );
        assert_eq!(
            h.generator.seen.lock().unwrap().as_slice(),
            ["Name a tavern"]
        );
    }

    #[tokio::test]
    async fn generated_text_can_be_stored_and_read_back_by_the_next_run() {
        let h = harness_replying("a sudden hush").await;
        let write = h
            .define(
                "note",
                r#"$[set_var("global:mood", $[generate_raw("Describe the mood")])]"#,
                ToolSurface::Command,
            )
            .await;
        let read = h
            .define("mood", "{# return {{global:mood}} #}", ToolSurface::Command)
            .await;

        h.run(write, "").await.unwrap();
        assert_eq!(
            h.run(read, "").await.unwrap().output,
            json!("a sudden hush")
        );
    }

    #[tokio::test]
    async fn generation_composes_with_the_output_commands() {
        let h = harness_replying("three coins").await;
        let id = h
            .define(
                "loot",
                r#"$[sys($[generate_raw("What is in the chest?")])]"#,
                ToolSurface::Command,
            )
            .await;

        let message = h.run(id, "").await.unwrap().message.expect("sys anchors");
        assert_eq!(message.content, "three coins");
    }

    #[tokio::test]
    async fn generation_uses_the_active_profile_unless_the_body_names_one() {
        let h = harness_replying("ok").await;
        let id = h
            .define(
                "ask",
                r#"$[generate_raw("anything")]"#,
                ToolSurface::Command,
            )
            .await;

        h.run(id, "").await.unwrap();
        assert_eq!(
            h.generator.profiles.lock().unwrap().as_slice(),
            [h.active_profile]
        );
    }

    #[tokio::test]
    async fn a_named_profile_answers_instead_of_the_active_one() {
        let h = harness_replying("ok").await;
        // A second profile, so "cheap model for summaries" is a real choice.
        let cheap = h.add_profile("cheap").await;
        let id = h
            .define(
                "summarize",
                &format!(r#"$[generate_raw("recap", "{cheap}")]"#),
                ToolSurface::Command,
            )
            .await;

        h.run(id, "").await.unwrap();
        assert_eq!(h.generator.profiles.lock().unwrap().as_slice(), [cheap]);
    }

    #[tokio::test]
    async fn generate_goes_through_the_context_assembled_path() {
        let h = harness_replying("They argued, then left.").await;
        let id = h
            .define(
                "recap",
                r#"{# return $[generate("Summarize the scene")] #}"#,
                ToolSurface::Command,
            )
            .await;

        assert_eq!(
            h.run(id, "").await.unwrap().output,
            json!("They argued, then left.")
        );
        assert_eq!(
            h.generator.in_context.lock().unwrap().as_slice(),
            ["Summarize the scene"]
        );
    }

    #[tokio::test]
    async fn generate_raw_does_not_take_the_context_path() {
        let h = harness_replying("ok").await;
        let id = h
            .define("ask", r#"$[generate_raw("hello")]"#, ToolSurface::Command)
            .await;

        h.run(id, "").await.unwrap();
        assert!(
            h.generator.in_context.lock().unwrap().is_empty(),
            "generate_raw must not assemble the conversation"
        );
    }

    #[tokio::test]
    async fn both_generate_commands_honour_a_named_profile() {
        let h = harness_replying("ok").await;
        let cheap = h.add_profile("cheap").await;
        let id = h
            .define(
                "both",
                &format!(r#"$[generate_raw("a", "{cheap}")]$[generate("b", "{cheap}")]"#),
                ToolSurface::Command,
            )
            .await;

        h.run(id, "").await.unwrap();
        assert_eq!(
            h.generator.profiles.lock().unwrap().as_slice(),
            [cheap, cheap]
        );
    }

    #[tokio::test]
    async fn generate_rejects_a_bad_profile_the_same_way_generate_raw_does() {
        let h = harness_replying("ok").await;
        let id = h
            .define(
                "bad",
                r#"$[generate("x", "not-an-id")]"#,
                ToolSurface::Command,
            )
            .await;

        assert!(matches!(
            h.run(id, "").await,
            Err(CommandError::Failed { ref reason, .. }) if reason.contains("not a profile id")
        ));
    }

    #[tokio::test]
    async fn a_profile_that_is_not_an_id_is_refused_before_generating() {
        let h = harness_replying("ok").await;
        let id = h
            .define(
                "bad",
                r#"$[generate_raw("x", "haiku-cheap")]"#,
                ToolSurface::Command,
            )
            .await;

        assert!(matches!(
            h.run(id, "").await,
            Err(CommandError::Failed { ref reason, .. }) if reason.contains("not a profile id")
        ));
        assert!(h.generator.profiles.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_unquoted_profile_id_is_refused_rather_than_silently_rounded() {
        let h = harness_replying("ok").await;
        let id = h
            .define(
                "rounded",
                r#"$[generate_raw("x", 353622362832240640)]"#,
                ToolSurface::Command,
            )
            .await;

        assert!(matches!(
            h.run(id, "").await,
            Err(CommandError::Failed { ref reason, .. }) if reason.contains("loses precision")
        ));
    }

    #[tokio::test]
    async fn impersonate_drafts_into_the_composer_without_touching_the_conversation() {
        let h = harness_replying("I draw my sword.").await;
        let id = h
            .define(
                "impersonate",
                r#"$[compose($[generate_raw("Write my next message")])]"#,
                ToolSurface::Command,
            )
            .await;

        let run = h.run(id, "").await.unwrap();
        assert!(
            run.message.is_none(),
            "a draft is not committed to the conversation"
        );
        // Asserted as JSON because that is what the client actually receives;
        // this pins the wire shape the frontend's CommandEffect union expects.
        assert_eq!(
            serde_json::to_value(&run.effects).unwrap(),
            json!([{
                "type": "compose",
                "content": "I draw my sword.",
                "mode": "replace",
            }])
        );
    }

    #[tokio::test]
    async fn cancelling_the_conversation_interrupts_a_body_blocked_on_generation() {
        let h = hanging_harness().await;
        let id = h
            .define(
                "slow",
                r#"{# return $[generate_raw("takes forever")] #}"#,
                ToolSurface::Command,
            )
            .await;

        let conversation = h.conversation;
        let generator = Arc::clone(&h.generator);
        let canceller = tokio::spawn(async move {
            // Wait for the body to reach the hanging completion.
            while !generator.cancel(conversation) {
                tokio::task::yield_now().await;
            }
        });

        // Completes rather than hanging: the run is interruptible even while
        // the blocking thread is parked on a network call
        let run = h.run(id, "").await;
        canceller.await.unwrap();
        assert!(
            matches!(run, Err(CommandError::Cancelled(ref name)) if name == "slow"),
            "expected a cancellation, got {run:?}"
        );
    }

    #[tokio::test]
    async fn a_runaway_loop_stops_at_the_iteration_ceiling() {
        let h = harness().await;
        // 10_001 iterations, one past MAX_ITERATIONS.
        let id = h
            .define(
                "spin",
                "{# foreach n in @[array.range(start: 0, end: 10001)] #}.{# endforeach #}",
                ToolSurface::Command,
            )
            .await;

        assert!(matches!(
            h.run(id, "").await,
            Err(CommandError::Failed { ref name, .. }) if name == "spin"
        ));
    }

    #[tokio::test]
    async fn a_shipped_command_runs_although_it_belongs_to_no_user() {
        let h = harness_replying("I draw my blade.").await;
        // Owned by BUILTIN_OWNER, so the ownership gate would reject it.
        let run = h
            .run(crate::services::tools::IMPERSONATE_ID, "")
            .await
            .unwrap();

        assert_eq!(
            serde_json::to_value(&run.effects).unwrap(),
            json!([{ "type": "compose", "content": "I draw my blade.", "mode": "replace" }])
        );
        assert!(run.message.is_none(), "a draft is not committed");
    }

    #[tokio::test]
    async fn trailing_text_steers_a_shipped_command() {
        let h = harness_replying("ok").await;
        h.run(crate::services::tools::IMPERSONATE_ID, "be furious")
            .await
            .unwrap();

        let asked = h.generator.in_context.lock().unwrap().clone();
        assert!(
            asked.iter().any(|p| p.ends_with("be furious")),
            "the steer reaches the prompt: {asked:?}"
        );
    }

    #[tokio::test]
    async fn shipped_commands_are_listed_before_a_users_own() {
        let h = harness().await;
        h.define("zzz_mine", "x", ToolSurface::Command).await;

        let listed = h.service.list(h.user).await.unwrap();
        let shipped = listed
            .iter()
            .position(|t| t.name == "impersonate")
            .expect("shipped commands are listed");
        let mine = listed.iter().position(|t| t.name == "zzz_mine").unwrap();

        assert!(shipped < mine, "shipped rows must come first");
    }

    #[tokio::test]
    async fn a_users_row_of_the_same_name_is_listed_after_the_shipped_one() {
        let h = harness().await;
        h.define("impersonate", "mine", ToolSurface::Command).await;

        let listed = h.service.list(h.user).await.unwrap();
        let both: Vec<&UserTool> = listed.iter().filter(|t| t.name == "impersonate").collect();
        assert_eq!(
            both.len(),
            2,
            "both are listed; the client resolves which wins"
        );
        assert_eq!(
            both[1].body, "mine",
            "the user's own comes last, so it shadows"
        );
    }

    #[tokio::test]
    async fn a_shipped_tool_is_still_not_invocable_as_a_command() {
        let h = harness().await;
        assert!(matches!(
            h.run(crate::services::tools::SPEAK_TO_ID, "").await,
            Err(CommandError::NotACommand(ref name)) if name == "speak_to"
        ));
    }

    #[tokio::test]
    async fn another_users_conversation_is_not_reachable() {
        let h = harness().await;
        let id = h.define("noop", "hi", ToolSurface::Command).await;
        let stranger = h.snow.generate();

        assert!(matches!(
            h.service
                .run(stranger, h.conversation, id, CommandArgs::default())
                .await,
            Err(CommandError::NotOwned(_))
        ));
    }

    fn param(name: &str, ty: ParamType, required: bool) -> Param {
        Param {
            name: name.into(),
            ty,
            description: None,
            required,
            resource: None,
        }
    }

    fn args(named: &[(&str, &str)], rest: &str) -> CommandArgs {
        CommandArgs {
            named: named
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            rest: rest.into(),
        }
    }

    #[test]
    fn trailing_text_is_always_bound_as_rest() {
        let bound = bind_args("echo", &[], &args(&[], "for initiative")).unwrap();
        assert_eq!(
            bound.get("rest"),
            Some(&WeaverValue::String("for initiative".into()))
        );
    }

    #[test]
    fn named_arguments_coerce_to_their_declared_type() {
        let params = [
            param("sides", ParamType::Number, true),
            param("loud", ParamType::Bool, false),
            param("tags", ParamType::Array(Box::new(ParamType::String)), false),
        ];
        let bound = bind_args(
            "roll",
            &params,
            &args(
                &[("sides", "20"), ("loud", "yes"), ("tags", r#"["a","b"]"#)],
                "",
            ),
        )
        .unwrap();

        assert_eq!(bound.get("sides"), Some(&WeaverValue::Number(20.0)));
        assert_eq!(bound.get("loud"), Some(&WeaverValue::Bool(true)));
        assert_eq!(
            bound.get("tags"),
            Some(&WeaverValue::Array(vec!["a".into(), "b".into()]))
        );
    }

    #[test]
    fn an_omitted_optional_param_is_none_and_a_required_one_is_an_error() {
        let params = [
            param("sides", ParamType::Number, true),
            param("loud", ParamType::Bool, false),
        ];
        let bound = bind_args("roll", &params, &args(&[("sides", "6")], "")).unwrap();
        assert_eq!(bound.get("loud"), Some(&WeaverValue::None));

        let missing = bind_args("roll", &params, &args(&[], ""));
        assert!(matches!(
            missing,
            Err(CommandError::BadArgument { ref name, .. }) if name == "sides"
        ));
    }

    #[test]
    fn a_value_that_does_not_fit_its_type_is_rejected_by_name() {
        let params = [param("sides", ParamType::Number, true)];
        let err = bind_args("roll", &params, &args(&[("sides", "twenty")], "")).unwrap_err();
        assert!(matches!(
            err,
            CommandError::BadArgument { ref name, ref reason }
                if name == "sides" && reason.contains("not a number")
        ));
    }
}
