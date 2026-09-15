use std::collections::BTreeMap;

use lorealm_storage::models::{RealmDefinition, is_identifier};
use serde_json::Value as Json;
use weaver_lang::Value as WeaverValue;

use crate::{
    errors::RealmError,
    services::weaver::{CompiledDoc, WeaverHost, WeaverProfile, compile_doc},
};

/// Two spaces per level
const INDENT: &str = "  ";

/// Render a realm definition as a WTN document.
pub fn to_wtn(definition: &RealmDefinition) -> Result<String, RealmError> {
    let mut out = String::new();
    write_value(&serde_json::to_value(definition)?, 0, &mut out)?;
    out.push('\n');
    Ok(out)
}

/// Expand a WTN document into a realm definition.
pub fn compile(source: &str) -> Result<CompiledDoc, RealmError> {
    compile_doc(source).map_err(bad_template)
}

/// Expand a parsed document against inputs that are already resolved.
pub fn expand(
    doc: CompiledDoc,
    inputs: BTreeMap<String, WeaverValue>,
) -> Result<RealmDefinition, RealmError> {
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmTemplate)
        .build();

    let outcome = host
        .evaluate_inline(doc.with_inputs(inputs))
        .map_err(bad_template)?;

    Ok(serde_json::from_value(
        outcome.to_json().map_err(|e| bad_template(e.to_string()))?,
    )?)
}

/// Parse and expand in one step, for a document with nothing to resolve.
#[cfg(test)]
pub fn from_wtn(
    source: &str,
    inputs: BTreeMap<String, WeaverValue>,
) -> Result<RealmDefinition, RealmError> {
    expand(compile(source)?, inputs)
}

fn bad_template(e: impl ToString) -> RealmError {
    RealmError::InvalidTemplate(e.to_string())
}

fn write_value(value: &Json, depth: usize, out: &mut String) -> Result<(), RealmError> {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => write_number(n, out)?,
        Json::String(s) => out.push_str(&raw_string(s)),
        Json::Array(items) if items.is_empty() => out.push_str("[]"),
        Json::Array(items) => {
            out.push_str("[\n");
            for item in items {
                indent(depth + 1, out);
                write_value(item, depth + 1, out)?;
                out.push('\n');
            }
            indent(depth, out);
            out.push(']');
        }
        Json::Object(fields) if fields.is_empty() => out.push_str("{}"),
        Json::Object(fields) => {
            let mut names: Vec<&String> = fields.keys().collect();
            names.sort();

            out.push_str("{\n");
            for name in names {
                let field = &fields[name];
                indent(depth + 1, out);
                out.push_str(&key(name)?);
                out.push_str(": ");
                write_value(field, depth + 1, out)?;
                out.push('\n');
            }
            indent(depth, out);
            out.push('}');
        }
    }
    Ok(())
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
}

/// A string as a raw WTN literal.
fn raw_string(s: &str) -> String {
    let bytes = s.as_bytes();
    let hashes = bytes
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'"')
        .map(|(i, _)| bytes[i + 1..].iter().take_while(|c| **c == b'#').count() + 1)
        .max()
        .unwrap_or(0);

    let fence = "#".repeat(hashes);
    format!("r{fence}\"{s}\"{fence}")
}

/// An object key.
fn key(name: &str) -> Result<String, RealmError> {
    if is_identifier(name) {
        return Ok(name.to_string());
    }
    for opener in ["{{", "{#", "@[", "$[", "[[", "<trigger"] {
        if name.contains(opener) {
            return Err(RealmError::InvalidTemplate(format!(
                "key '{name}' contains weaver syntax ('{opener}'), and a key cannot be written raw"
            )));
        }
    }
    Ok(serde_json::to_string(name)?)
}

/// WTN numbers are `f64`. An integer past the mantissa would come back a
/// different number, so refuse rather than corrupt it.
fn write_number(n: &serde_json::Number, out: &mut String) -> Result<(), RealmError> {
    const EXACT: u64 = 1 << 53;
    let magnitude = match (n.as_i64(), n.as_u64()) {
        (Some(i), _) => Some(i.unsigned_abs()),
        (_, Some(u)) => Some(u),
        _ => None,
    };
    if magnitude.is_some_and(|m| m > EXACT) {
        return Err(RealmError::InvalidTemplate(format!(
            "{n} cannot round-trip through WTN: numbers are f64"
        )));
    }
    out.push_str(&n.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use lorealm_storage::models::{
        AgentAccess, AgentGrant, AgentKind, AgentToolBinding, Director, RealmAgent, Scenario,
        Snowflake,
    };
    use serde_json::json;

    use super::*;

    const SCRIPT: &str = r#"{# if {{self:gold}} < 150 #}
    {# return "A shinobi costs 150 koku." #}
{# endif #}
$[inc_var("self:gold", -150)]
{# return "A shinobi slips toward " + {{cmd:target}} + " tonight." #}"#;

    fn agent(handle: &str) -> RealmAgent {
        RealmAgent {
            handle: handle.into(),
            name: handle.to_uppercase(),
            kind: AgentKind::Llm,
            character_id: Some(Snowflake(1934857293847562)),
            char_version_id: Some(Snowflake(1934857293847563)),
            template_id: Some(Snowflake(20)),
            profile_id: Some(Snowflake(30)),
            user_id: None,
            persona_id: None,
            properties: HashMap::from([
                ("gold".into(), json!(150)),
                ("army.size".into(), json!(12)),
                ("alive".into(), json!(true)),
            ]),
            grants: vec![AgentGrant {
                channel: "world:*".into(),
                access: AgentAccess::ReadWrite,
            }],
            tools: vec![AgentToolBinding {
                tool_id: Snowflake(40),
                event_channel: Some("main".into()),
            }],
            speech_channel: None,
            lorebooks: vec![Snowflake(50)],
            widgets: Vec::new(),
            script: Some(SCRIPT.into()),
            run_condition: Some("{{self:alive}} == true".into()),
        }
    }

    fn realm() -> RealmDefinition {
        RealmDefinition {
            name: "Sengoku".into(),
            description: "Houses scheme through the seasons.".into(),
            agents: vec![agent("regent_ito"), agent("regent_mori")],
            scenarios: vec![Scenario {
                id: Snowflake(70),
                name: "Spring".into(),
                description: String::new(),
                opening: Some("The thaw comes early this year.".into()),
            }],
            director: Some(Director::Script {
                source: r#"{{cast:handles}}"#.into(),
            }),
        }
    }

    fn round_trip(definition: &RealmDefinition) -> RealmDefinition {
        let source = to_wtn(definition).expect("emits");
        from_wtn(&source, BTreeMap::new()).unwrap_or_else(|e| panic!("{e}\n\n{source}"))
    }

    fn as_json(definition: &RealmDefinition) -> serde_json::Value {
        serde_json::to_value(definition).unwrap()
    }

    #[test]
    fn a_realm_round_trips_through_its_document() {
        let original = realm();
        assert_eq!(as_json(&round_trip(&original)), as_json(&original));
    }

    #[test]
    fn the_same_realm_always_emits_the_same_document() {
        let definition = realm();
        let first = to_wtn(&definition).unwrap();
        assert_eq!(first, to_wtn(&realm()).unwrap());

        let keys: Vec<&str> = first
            .lines()
            .skip_while(|l| !l.contains("properties:"))
            .skip(1)
            .take(3)
            .map(|l| l.trim().split(':').next().unwrap())
            .collect();
        assert_eq!(keys, ["alive", r#""army.size""#, "gold"]);
    }

    #[test]
    fn an_empty_realm_round_trips() {
        let original = RealmDefinition::default();
        assert_eq!(as_json(&round_trip(&original)), as_json(&original));
    }

    #[test]
    fn weaver_source_comes_back_as_text_rather_than_evaluated() {
        let back = round_trip(&realm());
        assert_eq!(back.agents[0].script.as_deref(), Some(SCRIPT));
        assert_eq!(
            back.agents[0].run_condition.as_deref(),
            Some("{{self:alive}} == true")
        );
        assert!(
            to_wtn(&realm())
                .unwrap()
                .contains(r#"r"{{self:alive}} == true""#)
        );
    }

    #[test]
    fn a_snowflake_stays_a_string_and_keeps_every_digit() {
        let source = to_wtn(&realm()).unwrap();
        assert!(source.contains(r#"r"1934857293847562""#), "{source}");
        assert_eq!(
            round_trip(&realm()).agents[0].character_id,
            Some(Snowflake(1934857293847562))
        );
    }

    #[test]
    fn a_quote_in_a_body_widens_the_fence() {
        let mut definition = realm();
        definition.agents[0].script = Some(r##"say "hi" and r#"nested"#"##.into());

        assert_eq!(
            round_trip(&definition).agents[0].script.as_deref(),
            Some(r##"say "hi" and r#"nested"#"##)
        );
    }

    #[test]
    fn a_template_with_inputs_expands_into_the_same_shape() {
        let source = r#"
#inputs
houses: [string]
season: enum("spring", "winter") = "spring"

{
  name: "Sengoku"
  description: r"Houses scheme through the seasons."
  agents: [
    {# foreach h in {{input:houses}} #}
    {
      handle: "regent_{{h}}"
      name: "{{h}}"
      kind: "llm"
      character_id: null
      char_version_id: null
      template_id: r"20"
      profile_id: r"30"
      user_id: null
      persona_id: null
      run_condition: r"{{self:alive}} == true"
    }
    {# endforeach #}
  ]
  scenarios: []
  director: { kind: "round_robin", over: { match: "all" } }
}"#;

        let inputs = BTreeMap::from([(
            "houses".to_string(),
            WeaverValue::Array(vec!["ito".into(), "mori".into()]),
        )]);
        let definition = from_wtn(source, inputs).expect("expands");

        let handles: Vec<&str> = definition
            .agents
            .iter()
            .map(|a| a.handle.as_str())
            .collect();
        assert_eq!(handles, ["regent_ito", "regent_mori"]);
        // A raw string stayed raw even inside a document that interpolates.
        assert_eq!(
            definition.agents[0].run_condition.as_deref(),
            Some("{{self:alive}} == true")
        );
    }

    #[test]
    fn a_number_too_large_for_f64_is_refused_rather_than_mangled() {
        let mut definition = realm();
        definition.agents[0]
            .properties
            .insert("treasury".into(), json!(9_007_199_254_740_993_i64));

        assert!(matches!(
            to_wtn(&definition),
            Err(RealmError::InvalidTemplate(_))
        ));
    }
}
