use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lorealm_providers::Error as ProviderError;
use lorealm_providers::{Tool, ToolDef};
use lorealm_storage::models::{Param, Snowflake, ToolSurface};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use weaver_lang::{CompiledTemplate, Registry, Value as WeaverValue};

use crate::services::chat::AttachmentSpec;
use crate::services::tools::{Actor, EventSink, RealmView, ToolContext, ToolStatePool};
use crate::util::weaver::{json_to_weaver, weaver_to_json};

/// Which tools a turn exposes and on whose behalf they run.
#[derive(Default)]
pub struct ToolPolicy {
    pub tool_ids: Option<Vec<Snowflake>>,
    pub extra_tool_ids: Vec<Snowflake>,
    pub actor: Option<Actor>,
    pub realm: Option<RealmView>,
    pub event_sink: Option<EventSink>,
    pub attachments: Vec<AttachmentSpec>,
    pub conversation: Option<Snowflake>,
    pub cancel: Option<CancellationToken>,
}

#[derive(Deserialize)]
pub struct CreateUserTool {
    pub name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub params: Vec<Param>,
    pub body: String,
    #[serde(default)]
    pub surface: Option<ToolSurface>,
}

#[derive(Default, Deserialize)]
pub struct UpdateUserTool {
    pub name: Option<String>,
    pub display_name: Option<Option<String>>,
    pub description: Option<String>,
    pub params: Option<Vec<Param>>,
    pub body: Option<String>,
    pub surface: Option<ToolSurface>,
}

#[derive(Serialize, Deserialize)]
pub struct ToolSignature {
    pub name: String,
    pub description: String,
    pub params: Vec<Param>,
    pub body: String, // the weaver-lang source
}

pub struct CompiledUserTool {
    sig: ToolSignature,
    body: Arc<CompiledTemplate>,
    registry: Arc<Registry>,
    state: Arc<Mutex<ToolStatePool>>,
    actor: Option<Actor>,
    realm: Option<RealmView>,
}

impl From<lorealm_storage::models::UserTool> for ToolSignature {
    fn from(value: lorealm_storage::models::UserTool) -> Self {
        Self {
            name: value.name,
            description: value.description,
            params: value.params,
            body: value.body,
        }
    }
}

impl CompiledUserTool {
    pub fn build(
        sig: ToolSignature,
        registry: Arc<Registry>,
        state: Arc<Mutex<ToolStatePool>>,
        actor: Option<Actor>,
        realm: Option<RealmView>,
    ) -> Result<Self, ProviderError> {
        let body = CompiledTemplate::compile(&sig.body).map_err(|e| {
            ProviderError::Tool(format!("'{}' body failed to compile: {e:?}", sig.name))
        })?;
        Ok(Self {
            sig,
            body: Arc::new(body),
            registry,
            state,
            actor,
            realm,
        })
    }
}

#[async_trait]
impl Tool for CompiledUserTool {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: self.sig.name.clone(),
            description: self.sig.description.clone(),
            input_schema: self.sig.input_schema(),
        }
    }

    async fn call(&self, input: serde_json::Value) -> Result<serde_json::Value, ProviderError> {
        let args = input.as_object().cloned().unwrap_or_default();

        let mut cmd = HashMap::new();
        for p in &self.sig.params {
            match args.get(&p.name) {
                Some(v) => {
                    let value = json_to_weaver(v)
                        .map_err(|e| ProviderError::Tool(format!("parameter '{}': {e}", p.name)))?;
                    cmd.insert(p.name.clone(), value);
                }
                None if p.required => {
                    return Err(ProviderError::Tool(format!(
                        "missing required parameter '{}'",
                        p.name
                    )));
                }
                None => {
                    cmd.insert(p.name.clone(), WeaverValue::None);
                }
            }
        }

        let ctx = ToolContext::new(cmd, Arc::clone(&self.state))
            .with_actor(self.actor.clone())
            .with_realm(self.realm.clone());

        let body = Arc::clone(&self.body);
        let registry = Arc::clone(&self.registry);
        let evaluated = tokio::task::spawn_blocking(move || {
            let mut ctx = ctx;
            body.evaluate_value(&mut ctx, &registry)
        })
        .await
        .map_err(|e| {
            ProviderError::Tool(format!("tool '{}' did not finish: {e}", self.sig.name))
        })?;

        // A body without `{# return #}` yields its rendered output as a string
        let out = evaluated
            .map_err(|e| ProviderError::Tool(format!("tool '{}' failed: {e}", self.sig.name)))?;
        weaver_to_json(&out)
            .map_err(|e| ProviderError::Tool(format!("tool '{}' returned {e}", self.sig.name)))
    }
}

impl ToolSignature {
    fn input_schema(&self) -> serde_json::Value {
        let mut props = serde_json::Map::new();
        let mut required = Vec::new();
        for p in &self.params {
            let mut s = p.ty.json_schema();
            if let (Some(d), Some(o)) = (&p.description, s.as_object_mut()) {
                o.insert("description".into(), json!(d));
            }
            props.insert(p.name.clone(), s);
            if p.required {
                required.push(json!(p.name));
            }
        }
        json!({ "type": "object", "properties": props, "required": required })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::weaver::WeaverHost;
    use weaver_lang::Registry;

    fn turn_registry() -> Registry {
        let mut registry = WeaverHost::base().into_registry();
        crate::services::commands::register_effect_commands(&mut registry);
        registry.register_command(crate::services::commands::GenerateRawCommand::declaration());
        registry.register_command(crate::services::commands::GenerateCommand::declaration());
        registry
    }

    async fn call_err(body: &str) -> String {
        let tool = CompiledUserTool::build(
            ToolSignature {
                name: "t".into(),
                description: String::new(),
                params: Vec::new(),
                body: body.into(),
            },
            Arc::new(turn_registry()),
            Arc::new(Mutex::new(ToolStatePool::default())),
            None,
            None,
        )
        .expect("body compiles");
        match tool.call(json!({})).await {
            Ok(v) => format!("unexpectedly succeeded: {v}"),
            Err(e) => e.to_string(),
        }
    }

    async fn call(body: &str) -> serde_json::Value {
        let tool = CompiledUserTool::build(
            ToolSignature {
                name: "t".into(),
                description: String::new(),
                params: Vec::new(),
                body: body.into(),
            },
            WeaverHost::base().into_shared().0,
            Arc::new(Mutex::new(ToolStatePool::default())),
            None,
            None,
        )
        .expect("body compiles");
        tool.call(json!({})).await.expect("body evaluates")
    }

    #[tokio::test]
    async fn a_body_without_a_return_still_hands_back_its_rendered_text() {
        assert_eq!(call("Hello").await, json!("Hello"));
    }

    #[tokio::test]
    async fn a_returned_array_reaches_the_model_as_an_array() {
        assert_eq!(
            call(r#"ignored{# return ["sword", "cursed"] #}"#).await,
            json!(["sword", "cursed"])
        );
    }

    #[tokio::test]
    async fn returned_numbers_objects_and_none_keep_their_types() {
        assert_eq!(call("{# return 42 #}").await, json!(42));
        assert_eq!(call("{# return true #}").await, json!(true));
        assert_eq!(call("{# return #}").await, json!(null));
    }

    #[tokio::test]
    async fn a_tool_body_resolves_the_generate_and_effect_commands() {
        for source in [
            r#"$[generate_raw("write a poem")]"#,
            r#"$[generate("write a poem")]"#,
            r#"$[compose("here is your poem")]"#,
            r#"$[toast("done")]"#,
            r#"$[sys("noted")]"#,
        ] {
            let err = call_err(source).await;
            assert!(
                !err.contains("undefined command"),
                "{source} should resolve, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn generation_without_a_bridge_refuses_by_reason() {
        let err = call_err(r#"$[generate_raw("x")]"#).await;
        assert!(err.contains("only available inside a command"), "{err}");
    }

    #[tokio::test]
    async fn a_body_evaluating_off_the_runtime_thread_still_returns_normally() {
        assert_eq!(call("{# return [1, 2] #}").await, json!([1, 2]));
    }

    #[tokio::test]
    async fn stop_keeps_the_output_rendered_before_it() {
        assert_eq!(call("kept{# stop #}dropped").await, json!("kept"));
    }
}
