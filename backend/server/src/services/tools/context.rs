use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use weaver_lang::{EvalContext, EvalError, Registry, Value as WeaverValue};

use crate::{
    services::context::weaver::stdlib,
    util::weaver::{json_to_weaver, weaver_to_json},
};

pub type ToolStatePool =
    std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>;

pub struct ToolContext {
    cmd: HashMap<String, WeaverValue>, // this call's args; read-only to the body
    local: HashMap<String, WeaverValue>, // per-call scratch; discarded
    state: Arc<Mutex<ToolStatePool>>,  // shared across the turn; persisted after the stream
}

impl ToolContext {
    pub fn new(cmd: HashMap<String, WeaverValue>, state: Arc<Mutex<ToolStatePool>>) -> Self {
        Self {
            cmd,
            local: HashMap::new(),
            state,
        }
    }
}

impl EvalContext for ToolContext {
    fn resolve_variable(&self, scope: &str, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        Ok(match scope {
            "cmd" => self.cmd.get(name).cloned(),
            "local" => self.local.get(name).cloned(),
            // "state" and any other persistent namespace read from the pool.
            other => self
                .state
                .lock()
                .unwrap()
                .get(other)
                .and_then(|ns| ns.get(name))
                .map(json_to_weaver),
        })
    }

    fn set_variable(
        &mut self,
        scope: &str,
        name: &str,
        value: WeaverValue,
    ) -> Result<(), EvalError> {
        match scope {
            "local" => {
                self.local.insert(name.into(), value);
                Ok(())
            }
            "user" | "char" => Err(EvalError::host_error(format!(
                "namespace '{scope}' is a reserved namespace"
            ))),
            other => {
                self.state
                    .lock()
                    .unwrap()
                    .entry(other.into())
                    .or_default()
                    .insert(name.into(), weaver_to_json(&value));
                Ok(())
            }
        }
    }

    fn fire_trigger(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        Err(EvalError::host_error(
            "triggers are not available in tool bodies",
        ))
    }
    fn resolve_document(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        Err(EvalError::host_error(
            "documents are not available in tool bodies",
        ))
    }
}

pub fn build_tool_registry() -> Registry {
    let mut r = Registry::new();
    stdlib::register_all(&mut r);
    r
}
