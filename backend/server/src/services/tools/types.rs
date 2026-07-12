use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lorealm_providers::Error as ProviderError;
use lorealm_providers::{Tool, ToolDef};
use lorealm_storage::models::{Param, ScopePattern, Snowflake};
use serde::{Deserialize, Serialize};
use serde_json::json;
use weaver_lang::{CompiledTemplate, Registry, Value as WeaverValue};

use crate::services::tools::{ToolContext, ToolStatePool};
use crate::util::weaver::json_to_weaver;

/// Constraints on what a turn's tools may see and do. The plain chat flow is
/// unrestricted; realm agents get their grant-derived write scopes and their
/// bound tool set.
#[derive(Default)]
pub struct ToolPolicy {
    pub write_scopes: Option<Arc<Vec<ScopePattern>>>,
    /// `None` = the owner's enabled tools; `Some` = exactly these tool ids.
    pub tool_ids: Option<Vec<Snowflake>>,
}

#[derive(Deserialize)]
pub struct CreateUserTool {
    pub name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub params: Vec<Param>,
    pub body: String,
}

#[derive(Default, Deserialize)]
pub struct UpdateUserTool {
    pub name: Option<String>,
    pub display_name: Option<Option<String>>,
    pub description: Option<String>,
    pub params: Option<Vec<Param>>,
    pub body: Option<String>,
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
    body: CompiledTemplate,
    registry: Arc<Registry>,
    state: Arc<Mutex<ToolStatePool>>,
    write_scopes: Option<Arc<Vec<ScopePattern>>>,
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
        write_scopes: Option<Arc<Vec<ScopePattern>>>,
    ) -> Result<Self, ProviderError> {
        let body = CompiledTemplate::compile(&sig.body).map_err(|e| {
            ProviderError::Tool(format!("'{}' body failed to compile: {e:?}", sig.name))
        })?;
        Ok(Self {
            sig,
            body,
            registry,
            state,
            write_scopes,
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
                    cmd.insert(p.name.clone(), json_to_weaver(v));
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

        let mut ctx = ToolContext::new(cmd, Arc::clone(&self.state))
            .with_write_scopes(self.write_scopes.clone());
        let out = self
            .body
            .evaluate(&mut ctx, &self.registry)
            .map_err(|e| ProviderError::Tool(format!("tool '{}' failed: {e}", self.sig.name)))?;
        Ok(serde_json::Value::String(out))
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
