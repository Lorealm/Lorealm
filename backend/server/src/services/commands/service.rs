use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use lorealm_storage::{
    Storage,
    models::{ChatRole, Param, ParamType, Snowflake, UserTool},
};
use serde_json::json;
use weaver_lang::{CompiledTemplate, Value as WeaverValue};

use crate::{
    errors::CommandError,
    services::{
        commands::{CommandArgs, CommandEffect, CommandRun, register_effect_commands},
        conversation::ConversationService,
        state::{CapturedState, StateService},
        tools::{ToolContext, UserToolService, build_tool_registry},
    },
    util::weaver::{describe_parse_errors, json_to_weaver, weaver_to_json},
};

pub struct CommandService {
    storage: Arc<dyn Storage>,
    conversations: Arc<ConversationService>,
    state: Arc<StateService>,
    tools: Arc<UserToolService>,
}

impl CommandService {
    pub fn new(
        storage: Arc<dyn Storage>,
        conversations: Arc<ConversationService>,
        state: Arc<StateService>,
        tools: Arc<UserToolService>,
    ) -> Self {
        Self {
            storage,
            conversations,
            state,
            tools,
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
        if conversation.realm_id.is_some() {
            return Err(CommandError::RealmConversation(conversation_id));
        }

        let command = self.tools.get_tool(command_id, user_id).await?;
        if !command.surface.callable_by_user() {
            return Err(CommandError::NotACommand(command.name));
        }

        let body =
            CompiledTemplate::compile(&command.body).map_err(|errors| CommandError::Failed {
                name: command.name.clone(),
                reason: describe_parse_errors(&command.body, &errors),
            })?;

        let bound = bind_args(&command.name, &command.params, &args)?;

        let thread = self.conversations.active_thread(conversation_id).await?;
        let ids: Vec<Snowflake> = thread.iter().map(|m| m.id).collect();
        let reconstructed = self.state.reconstruct(&ids).await?;
        let baseline = reconstructed.persistent;
        let pool = Arc::new(StdMutex::new(baseline.clone()));

        let mut registry = build_tool_registry();
        let sink = register_effect_commands(&mut registry);

        let mut ctx = ToolContext::new(bound, Arc::clone(&pool));
        let output =
            body.evaluate_value(&mut ctx, &registry)
                .map_err(|e| CommandError::Failed {
                    name: command.name.clone(),
                    reason: e.to_string(),
                })?;
        let output = weaver_to_json(&output).map_err(|e| CommandError::Failed {
            name: command.name.clone(),
            reason: format!("returned {e}"),
        })?;

        let effects = std::mem::take(&mut *sink.lock().unwrap());
        let delta = {
            let after = pool.lock().unwrap();
            self.state.diff(&baseline, &after)
        };

        // Apply all sys effects
        let sys: Vec<&str> = effects
            .iter()
            .filter_map(|e| match e {
                CommandEffect::Sys { content } => Some(content.as_str()),
                _ => None,
            })
            .collect();
        let wrote_state = !delta.writes.is_empty() || !delta.deletes.is_empty();

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
        ConversationStorage, UserToolStorage,
        backends::MemoryStorage,
        models::{Conversation, ToolSurface},
    };

    struct Harness {
        service: CommandService,
        store: Arc<MemoryStorage>,
        snow: Arc<SnowflakeService>,
        user: Snowflake,
        conversation: Snowflake,
    }

    async fn harness() -> Harness {
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
            created_at: now,
            updated_at: now,
        };
        let conversation_id = conversation.id;
        store.create_conversation(&conversation).await.unwrap();

        let conversations = Arc::new(ConversationService::new(store.clone(), snow.clone()));
        let state = Arc::new(StateService::new(store.clone()));
        let tools = Arc::new(UserToolService::new(store.clone(), snow.clone()));

        Harness {
            service: CommandService::new(store.clone(), conversations, state, tools),
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
    async fn a_realm_playthrough_refuses_commands() {
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
            created_at: now,
            updated_at: now,
        };
        let realm_id = realm_convo.id;
        h.store.create_conversation(&realm_convo).await.unwrap();
        let id = h.define("noop", "hi", ToolSurface::Command).await;

        assert!(matches!(
            h.service
                .run(h.user, realm_id, id, CommandArgs::default())
                .await,
            Err(CommandError::RealmConversation(_))
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
