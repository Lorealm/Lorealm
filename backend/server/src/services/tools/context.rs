use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use lorealm_storage::models::ScopePattern;
use weaver_lang::{EvalContext, EvalError, Registry, Value as WeaverValue};

use crate::{
    services::context::weaver::stdlib,
    util::weaver::{json_to_weaver, weaver_to_json},
};

pub type ToolStatePool =
    std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>;

pub struct ToolContext {
    cmd: HashMap<String, WeaverValue>,
    local: HashMap<String, WeaverValue>,
    state: Arc<Mutex<ToolStatePool>>,
    write_scopes: Option<Arc<Vec<ScopePattern>>>,
}

impl ToolContext {
    pub fn new(cmd: HashMap<String, WeaverValue>, state: Arc<Mutex<ToolStatePool>>) -> Self {
        Self {
            cmd,
            local: HashMap::new(),
            state,
            write_scopes: None,
        }
    }

    pub fn with_write_scopes(mut self, scopes: Option<Arc<Vec<ScopePattern>>>) -> Self {
        self.write_scopes = scopes;
        self
    }
}

impl EvalContext for ToolContext {
    fn resolve_variable(&self, scope: &str, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        Ok(match scope {
            "cmd" => self.cmd.get(name).cloned(),
            "local" => self.local.get(name).cloned(),
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
                if let Some(allowed) = &self.write_scopes {
                    if !allowed.iter().any(|p| p.matches(other)) {
                        return Err(EvalError::host_error(format!(
                            "no write access to namespace '{other}'"
                        )));
                    }
                }
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
        // TODO: Eventually we want to tie this into lorebooks
        Err(EvalError::host_error(
            "triggers are currently not supported in tool bodies",
        ))
    }
    fn resolve_document(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        // TODO: Eventually we want to tie this into lorebooks
        Err(EvalError::host_error(
            "documents are currently not supported in tool bodies",
        ))
    }
}

pub fn build_tool_registry() -> Registry {
    let mut r = Registry::new();
    stdlib::register_all(&mut r);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(scopes: Option<Vec<&str>>) -> ToolContext {
        let state = Arc::new(Mutex::new(ToolStatePool::default()));
        ToolContext::new(HashMap::new(), state).with_write_scopes(
            scopes.map(|s| Arc::new(s.into_iter().map(ScopePattern::parse).collect())),
        )
    }

    #[test]
    fn unrestricted_writes_anywhere() {
        let mut c = ctx(None);
        assert!(c.set_variable("nation.a", "gold", WeaverValue::None).is_ok());
    }

    #[test]
    fn write_scope_enforced() {
        let mut c = ctx(Some(vec!["nation.a", "market.*"]));
        assert!(c.set_variable("nation.a", "gold", WeaverValue::None).is_ok());
        assert!(c.set_variable("market.ore", "price", WeaverValue::None).is_ok());
        assert!(c.set_variable("nation.b", "gold", WeaverValue::None).is_err());
    }

    #[test]
    fn local_scope_bypasses_grants() {
        let mut c = ctx(Some(vec![]));
        assert!(c.set_variable("local", "scratch", WeaverValue::None).is_ok());
        assert!(c.set_variable("anything", "x", WeaverValue::None).is_err());
    }
}
