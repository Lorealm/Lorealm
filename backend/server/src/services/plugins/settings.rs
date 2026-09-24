use std::collections::BTreeMap;

use weaver_lang::{
    EvalContext, EvalError, EvalErrorKind, InputDecl, Registry, Value, ValueDoc,
    evaluate_value_doc, parse_value_doc,
};

use lorealm_storage::{Storage, StorageError, models::Snowflake};

use crate::util::weaver::{describe_parse_errors, json_to_weaver, weaver_to_json};

/// The declarations in `source`, which holds only an `#inputs` block.
pub fn declarations(source: &str) -> Result<Vec<InputDecl>, String> {
    // A document needs a value after its inputs; settings have none.
    Ok(parse(&format!("{source}\n{{}}"))?.inputs)
}

/// The user's API keys: the ids of their keys of each type the user's plugins
/// declare. A `Ref<type>` setting holds one of them.
pub type Keys = BTreeMap<String, Vec<String>>;

/// Each of `types` with the ids of the user's keys of it.
pub async fn keys(
    storage: &dyn Storage,
    user_id: Snowflake,
    types: impl IntoIterator<Item = String>,
) -> Result<Keys, StorageError> {
    let mut keys: Keys = types.into_iter().map(|kind| (kind, Vec::new())).collect();
    for secret in storage.list_secrets_by_owner(user_id).await? {
        if let Some(ids) = keys.get_mut(&secret.provider) {
            ids.push(secret.id.to_string());
        }
    }
    Ok(keys)
}

/// What the user saved for `handle`, unchecked.
pub async fn saved(
    storage: &dyn Storage,
    user_id: Snowflake,
    handle: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, StorageError> {
    Ok(match storage.get_plugin_settings(user_id, handle).await? {
        Some(serde_json::Value::Object(values)) => values,
        _ => serde_json::Map::new(),
    })
}

/// Checks `values` against the declarations in `source`, and returns every
/// setting: the value given, or else its default.
pub fn resolve(
    source: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    keys: &Keys,
) -> Result<serde_json::Value, String> {
    // The value collects every input, so evaluating it also applies defaults.
    let fields: Vec<String> = declarations(source)?
        .iter()
        .map(|d| format!("{0}: {{{{input:{0}}}}}", d.name))
        .collect();
    let doc = parse(&format!("{source}\n{{ {} }}", fields.join(", ")))?;

    let mut bound = BTreeMap::new();
    for (name, value) in values {
        let value = json_to_weaver(value).map_err(|e| format!("input {name}: {e}"))?;
        bound.insert(name.clone(), value);
    }
    let mut registry = Registry::new();
    for kind in keys.keys() {
        registry.register_kind(kind);
    }
    let settings = evaluate_value_doc(&doc, &bound, &mut KeyContext(keys.clone()), &registry)
        .map_err(|e| describe(&doc, e))?;
    weaver_to_json(&settings).map_err(|e| e.to_string())
}

/// Settings have no variables, triggers or documents; only their `Ref<type>`
/// values need the host.
struct KeyContext(Keys);

impl EvalContext for KeyContext {
    fn resolve_variable(&self, _: &str, _: &str) -> Result<Option<Value>, EvalError> {
        Ok(None)
    }

    fn set_variable(&mut self, _: &str, _: &str, _: Value) -> Result<(), EvalError> {
        Err(EvalError::host_error("settings can't set variables"))
    }

    fn fire_trigger(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        Err(EvalError::host_error("settings can't fire triggers"))
    }

    fn resolve_document(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        Err(EvalError::host_error("settings can't include documents"))
    }

    fn validate_input(&self, kind: &str, value: &Value) -> Result<(), EvalError> {
        let held = value.as_string().is_some_and(|id| {
            self.0
                .get(kind)
                .is_some_and(|ids| ids.iter().any(|i| i == id))
        });
        if held {
            Ok(())
        } else {
            Err(EvalError::new(
                EvalErrorKind::TypeError,
                format!("pick one of your {kind} keys"),
            ))
        }
    }
}

fn parse(source: &str) -> Result<ValueDoc, String> {
    parse_value_doc(source).map_err(|errors| describe_parse_errors(source, &errors))
}

/// Names the setting an error is about, where Weaver's message doesn't.
fn describe(doc: &ValueDoc, e: EvalError) -> String {
    let decl = doc.inputs.iter().find(|d| Some(d.span) == e.span);
    // Some of Weaver's messages already name it.
    let named = e.message.starts_with("input ") || e.message.starts_with("missing required input");
    match decl {
        Some(decl) if !named => format!("input {}: {}", decl.name, e.message),
        _ => e.message,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const SOURCE: &str = r#"#inputs
voice: enum("rachel", "adam") = "rachel"
speed: range(0.5, 2) = 1
autoplay: bool
"#;

    fn values(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn reads_declarations() {
        let names: Vec<_> = declarations(SOURCE)
            .unwrap()
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(names, ["voice", "speed", "autoplay"]);
    }

    #[test]
    fn applies_defaults() {
        let settings = resolve(SOURCE, &values(json!({ "autoplay": true })), &Keys::new()).unwrap();
        assert_eq!(
            settings,
            json!({ "voice": "rachel", "speed": 1, "autoplay": true })
        );
    }

    #[test]
    fn rejects_invalid_values() {
        let cases = [
            (json!({}), "missing required input: autoplay"),
            (
                json!({ "autoplay": "yes" }),
                "input autoplay: expected bool",
            ),
            (json!({ "autoplay": true, "voice": "bob" }), "input voice"),
            (json!({ "autoplay": true, "speed": 3 }), "input speed"),
            (
                json!({ "autoplay": true, "volume": 1 }),
                "no such input: volume",
            ),
        ];
        for (given, expected) in cases {
            let err = resolve(SOURCE, &values(given), &Keys::new()).unwrap_err();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn a_key_must_be_one_of_the_users() {
        let source = "#inputs\nkey: Ref<elevenlabs>\n";
        let keys = Keys::from([("elevenlabs".into(), vec!["1".into()])]);
        let resolve =
            |key: &str, keys: &Keys| resolve(source, &values(json!({ "key": key })), keys);

        assert_eq!(resolve("1", &keys).unwrap(), json!({ "key": "1" }));
        assert!(
            resolve("2", &keys)
                .unwrap_err()
                .contains("input key: pick one of your elevenlabs keys")
        );
        assert!(
            resolve("1", &Keys::new())
                .unwrap_err()
                .contains("unknown entity kind")
        );
    }

    #[test]
    fn rejects_a_value_after_the_inputs() {
        assert!(declarations("#inputs\nx: bool\n\n{ a: 1 }").is_err());
    }
}
