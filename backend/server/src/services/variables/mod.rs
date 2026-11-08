use std::{collections::HashMap, sync::Arc};

use lorealm_storage::models::{Character, Realm, UserPersona};
use weaver_lang::Value;

pub mod builtin;

/// What a provider may read.
pub struct VariableContext<'a> {
    pub character: Option<&'a Character>,
    pub persona: Option<&'a UserPersona>,
    pub realm: Option<&'a Realm>,
}

pub trait VariableProvider: Send + Sync {
    fn scope(&self) -> &str;

    fn provide(&self, ctx: &VariableContext) -> HashMap<String, Value>;
}

/// One turn's variables, keyed scope -> name -> value.
#[derive(Debug, Default, Clone)]
pub struct Variables(HashMap<String, HashMap<String, Value>>);

impl Variables {
    #[cfg(test)]
    pub fn from_map(map: HashMap<String, HashMap<String, Value>>) -> Self {
        Self(map)
    }

    pub fn get(&self, scope: &str, name: &str) -> Option<&Value> {
        self.0.get(scope)?.get(name)
    }

    /// Scope-by-scope, for hosts that must be seeded up front rather than
    /// queried (`ContextWeaver` takes values through `set_variable`).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &HashMap<String, Value>)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Replace every `{{scope:name}}` that `vars` knows
pub fn substitute(content: &str, vars: &Variables) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        let Some(end) = after.find("}}") else {
            // Unterminated: nothing left to substitute.
            out.push_str(&rest[start..]);
            return out;
        };

        let token = &after[..end];
        match token
            .split_once(':')
            .and_then(|(scope, name)| vars.get(scope.trim(), name.trim()))
        {
            Some(value) => out.push_str(&value.to_output_string()),
            None => out.push_str(&rest[start..start + 2 + end + 2]),
        }

        rest = &after[end + 2..];
    }

    out.push_str(rest);
    out
}

/// All known variable providers, keyed by [`VariableProvider::scope`].
#[derive(Clone, Default)]
pub struct VariableRegistry {
    providers: HashMap<String, Arc<dyn VariableProvider>>,
}

impl VariableRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn VariableProvider>) {
        self.providers
            .insert(provider.scope().to_string(), provider);
    }

    /// A snapshot over the current variables. This function resolves eagerly.
    ///
    /// `ctx` serves a filter, meaning only variables belonging to an entity
    /// within `ctx` is served.
    pub fn snapshot(&self, ctx: &VariableContext) -> Variables {
        Variables(
            self.providers
                .iter()
                .map(|(scope, p)| (scope.clone(), p.provide(ctx)))
                .filter(|(_, vars)| !vars.is_empty())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Variables {
        Variables(HashMap::from([(
            "char".to_string(),
            HashMap::from([
                ("name".to_string(), Value::String("Aria".into())),
                ("level".to_string(), Value::Number(7.0)),
            ]),
        )]))
    }

    #[test]
    fn substitutes_known_tokens() {
        assert_eq!(substitute("Hi {{char:name}}!", &vars()), "Hi Aria!");
    }

    #[test]
    fn renders_through_weavers_own_output_string() {
        assert_eq!(substitute("lvl {{char:level}}", &vars()), "lvl 7");
    }

    #[test]
    fn leaves_unknown_tokens_verbatim() {
        assert_eq!(
            substitute("{{nation:gold}} {{char:missing}} {{bare}}", &vars()),
            "{{nation:gold}} {{char:missing}} {{bare}}"
        );
    }

    #[test]
    fn substituted_values_are_not_rescanned() {
        let v = Variables(HashMap::from([(
            "char".to_string(),
            HashMap::from([
                ("name".to_string(), Value::String("{{char:level}}".into())),
                ("level".to_string(), Value::String("99".into())),
            ]),
        )]));

        assert_eq!(substitute("{{char:name}}", &v), "{{char:level}}");
    }

    #[test]
    fn handles_unterminated_and_empty() {
        assert_eq!(substitute("a {{char:name", &vars()), "a {{char:name");
        assert_eq!(substitute("", &vars()), "");
        assert_eq!(substitute("no tokens", &vars()), "no tokens");
    }

    #[test]
    fn empty_provider_output_is_dropped_from_the_snapshot() {
        struct Empty;
        impl VariableProvider for Empty {
            fn scope(&self) -> &str {
                "ghost"
            }
            fn provide(&self, _: &VariableContext) -> HashMap<String, Value> {
                HashMap::new()
            }
        }

        let mut reg = VariableRegistry::new();
        reg.register(Arc::new(Empty));
        let snapshot = reg.snapshot(&VariableContext {
            character: None,
            persona: None,
            realm: None,
        });

        assert!(snapshot.iter().next().is_none());
    }
}
