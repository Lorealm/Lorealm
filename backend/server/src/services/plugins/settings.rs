use std::collections::BTreeMap;

use weaver_lang::{
    EvalError, InputDecl, Registry, SimpleContext, ValueDoc, evaluate_value_doc, parse_value_doc,
};

use crate::util::weaver::{describe_parse_errors, json_to_weaver, weaver_to_json};

/// The declarations in `source`, which holds only an `#inputs` block.
pub fn declarations(source: &str) -> Result<Vec<InputDecl>, String> {
    // A document needs a value after its inputs; settings have none.
    Ok(parse(&format!("{source}\n{{}}"))?.inputs)
}

/// Checks `values` against the declarations in `source`, and returns every
/// setting: the value given, or else its default.
pub fn resolve(
    source: &str,
    values: &serde_json::Map<String, serde_json::Value>,
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
    // No entity kinds yet, so any `Ref<...>` is refused.
    let settings = evaluate_value_doc(&doc, &bound, &mut SimpleContext::new(), &Registry::new())
        .map_err(|e| describe(&doc, e))?;
    weaver_to_json(&settings).map_err(|e| e.to_string())
}

fn parse(source: &str) -> Result<ValueDoc, String> {
    parse_value_doc(source).map_err(|errors| describe_parse_errors(source, &errors))
}

/// Names the setting an error is about, where Weaver's message doesn't.
fn describe(doc: &ValueDoc, e: EvalError) -> String {
    let decl = doc.inputs.iter().find(|d| Some(d.span) == e.span);
    match decl {
        Some(decl) if !e.message.starts_with("input ") => {
            format!("input {}: {}", decl.name, e.message)
        }
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
        let settings = resolve(SOURCE, &values(json!({ "autoplay": true }))).unwrap();
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
            let err = resolve(SOURCE, &values(given)).unwrap_err();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn refuses_references_for_now() {
        let err = resolve(
            "#inputs\nkey: Ref<elevenlabs>\n",
            &values(json!({ "key": "1" })),
        );
        assert!(err.unwrap_err().contains("unknown entity kind"));
    }

    #[test]
    fn rejects_a_value_after_the_inputs() {
        assert!(declarations("#inputs\nx: bool\n\n{ a: 1 }").is_err());
    }
}
