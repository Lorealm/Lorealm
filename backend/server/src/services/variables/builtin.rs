use std::{collections::HashMap, sync::Arc};

use weaver_lang::Value;

use super::{VariableContext, VariableProvider, VariableRegistry};

/// `{{char:*}}` — the character being spoken as.
pub struct CharProvider;

impl VariableProvider for CharProvider {
    fn scope(&self) -> &str {
        "char"
    }

    fn provide(&self, ctx: &VariableContext) -> HashMap<String, Value> {
        let Some(ch) = ctx.character else {
            return HashMap::new();
        };

        let name = ch
            .metadata
            .display_name
            .clone()
            .unwrap_or_else(|| ch.definition.name.clone());

        HashMap::from([("name".to_string(), Value::String(name))])
    }
}

/// `{{user:*}}` — the persona the user is speaking as.
pub struct UserProvider;

impl VariableProvider for UserProvider {
    fn scope(&self) -> &str {
        "user"
    }

    fn provide(&self, ctx: &VariableContext) -> HashMap<String, Value> {
        let Some(p) = ctx.persona else {
            return HashMap::new();
        };

        let name = p.display_name.clone().unwrap_or_else(|| p.name.clone());

        HashMap::from([("name".to_string(), Value::String(name))])
    }
}

pub struct RealmProvider;

impl VariableProvider for RealmProvider {
    fn scope(&self) -> &str {
        "realm"
    }

    fn provide(&self, ctx: &VariableContext) -> HashMap<String, Value> {
        let Some(cast) = ctx.cast else {
            return HashMap::new();
        };

        let agents = cast
            .agents
            .iter()
            .map(|a| Value::String(a.name.clone()))
            .collect::<Vec<_>>();

        HashMap::from([("agents".to_string(), Value::Array(agents))])
    }
}

pub fn register_builtin_providers(registry: &mut VariableRegistry) {
    registry.register(Arc::new(CharProvider));
    registry.register(Arc::new(UserProvider));
    registry.register(Arc::new(RealmProvider));
}
