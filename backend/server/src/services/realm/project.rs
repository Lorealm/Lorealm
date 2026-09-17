use std::collections::{BTreeMap, HashMap, hash_map::Entry};

use lorealm_storage::models::{RealmResourceKind, Snowflake};
use weaver_lang::{InputType, Value as WeaverValue};

use crate::{
    errors::{RealmError, TargetType},
    services::realm::RealmService,
};

/// The fields a resolved reference exposes.
fn projection(id: Snowflake, name: &str, description: &str) -> WeaverValue {
    WeaverValue::object([
        ("id", WeaverValue::String(id.to_string())),
        ("name", WeaverValue::String(name.to_string())),
        ("description", WeaverValue::String(description.to_string())),
        ("slug", WeaverValue::String(slug(name))),
    ])
}

/// `name` reduced to a weaver identifier, for minting handles in a loop.
pub(super) fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    match trimmed.chars().next() {
        // A leading digit is legal in a name and not in an identifier.
        Some(c) if c.is_ascii_digit() => format!("_{trimmed}"),
        Some(_) => trimmed.to_string(),
        None => "unnamed".to_string(),
    }
}

/// Whether resolving this type reaches an entity anywhere inside it.
pub(super) fn contains_entity(ty: &InputType) -> bool {
    match ty {
        InputType::Ref(_) => true,
        InputType::List(inner) => contains_entity(inner),
        InputType::Object(fields) => fields.iter().any(|(_, ty)| contains_entity(ty)),
        _ => false,
    }
}

pub(super) fn reject_defaulted_entities(
    declarations: &[weaver_lang::InputDecl],
) -> Result<(), RealmError> {
    for declaration in declarations {
        if !declaration.is_required() && contains_entity(&declaration.ty) {
            return Err(RealmError::InvalidTemplate(format!(
                "input '{}' names an entity, so it cannot have a default — make it required",
                declaration.name
            )));
        }
    }
    Ok(())
}

/// The kind a `Ref<..>` names, or an error against the input that wrote it.
fn named(input: &str, kind: &str) -> Result<RealmResourceKind, RealmError> {
    RealmResourceKind::from_ref_name(kind).ok_or_else(|| {
        RealmError::InvalidTemplate(format!("input '{input}': no such entity kind '{kind}'"))
    })
}

/// A supplied value read as the id it is meant to be.
fn as_id(
    input: &str,
    kind: RealmResourceKind,
    value: &WeaverValue,
) -> Result<Snowflake, RealmError> {
    value
        .as_string()
        .and_then(|s| s.parse::<i64>().ok())
        .map(Snowflake)
        .ok_or_else(|| {
            RealmError::InvalidTemplate(format!(
                "input '{input}': expected the id of a {}, got {}",
                kind.as_str(),
                value.type_name()
            ))
        })
}

/// Every entity a supplied value refers to
fn gather(
    input: &str,
    ty: &InputType,
    value: &WeaverValue,
    out: &mut Vec<(RealmResourceKind, Snowflake)>,
) -> Result<(), RealmError> {
    match ty {
        InputType::Ref(kind) => {
            let kind = named(input, kind)?;
            out.push((kind, as_id(input, kind, value)?));
        }
        InputType::List(inner) => {
            let items = value.as_array().ok_or_else(|| {
                RealmError::InvalidTemplate(format!("input '{input}': expected a list"))
            })?;
            for item in items {
                gather(input, inner, item, out)?;
            }
        }
        InputType::Object(fields) => {
            let map = value.as_object().ok_or_else(|| {
                RealmError::InvalidTemplate(format!("input '{input}': expected an object"))
            })?;
            for (name, ty) in fields {
                let field = map.get(name).ok_or_else(|| {
                    RealmError::InvalidTemplate(format!("input '{input}': missing field '{name}'"))
                })?;
                gather(&format!("{input}.{name}"), ty, field, out)?;
            }
        }
        // Everything else is a shape the language checks for itself.
        _ => {}
    }
    Ok(())
}

/// Rebuild a value with each entity id replaced by its projection.
fn substitute(
    input: &str,
    ty: &InputType,
    value: WeaverValue,
    projected: &HashMap<(RealmResourceKind, Snowflake), WeaverValue>,
) -> Result<WeaverValue, RealmError> {
    Ok(match ty {
        InputType::Ref(kind) => {
            let kind = named(input, kind)?;
            let id = as_id(input, kind, &value)?;
            projected
                .get(&(kind, id))
                .cloned()
                .expect("gather visits every reference substitute does")
        }
        InputType::List(inner) => {
            let items = value.into_array().ok_or_else(|| {
                RealmError::InvalidTemplate(format!("input '{input}': expected a list"))
            })?;
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(substitute(input, inner, item, projected)?);
            }
            WeaverValue::Array(out)
        }
        InputType::Object(fields) => {
            let mut map = value.into_object().ok_or_else(|| {
                RealmError::InvalidTemplate(format!("input '{input}': expected an object"))
            })?;
            for (name, ty) in fields {
                let Some(field) = map.remove(name) else { continue };
                let nested = format!("{input}.{name}");
                map.insert(name.clone(), substitute(&nested, ty, field, projected)?);
            }
            WeaverValue::Object(map)
        }
        _ => value,
    })
}

impl RealmService {
    /// Bind every declared input, and resolve the ones that name an entity.
    pub(super) async fn resolve_inputs(
        &self,
        user_id: Snowflake,
        declarations: &[weaver_lang::InputDecl],
        supplied: BTreeMap<String, WeaverValue>,
    ) -> Result<BTreeMap<String, WeaverValue>, RealmError> {
        reject_defaulted_entities(declarations)?;

        let declared = |name: &str| declarations.iter().find(|d| d.name == name);

        let mut wanted = Vec::new();
        for (name, value) in &supplied {
            if let Some(declaration) = declared(name) {
                gather(name, &declaration.ty, value, &mut wanted)?;
            }
        }

        let mut projected = HashMap::new();
        for (kind, id) in wanted {
            if let Entry::Vacant(slot) = projected.entry((kind, id)) {
                slot.insert(self.project(user_id, kind, id).await?);
            }
        }

        let mut bound = BTreeMap::new();
        for (name, value) in supplied {
            let resolved = match declared(&name) {
                Some(declaration) => substitute(&name, &declaration.ty, value, &projected)?,
                None => value,
            };
            bound.insert(name, resolved);
        }

        Ok(bound)
    }

    async fn project(
        &self,
        user_id: Snowflake,
        kind: RealmResourceKind,
        id: Snowflake,
    ) -> Result<WeaverValue, RealmError> {
        self.validate_resource(user_id, kind, id).await?;

        Ok(match kind {
            RealmResourceKind::Character => {
                let character = self
                    .storage
                    .get_character(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::Character, id))?;
                let name = character
                    .metadata
                    .display_name
                    .as_deref()
                    .unwrap_or(&character.definition.name);
                projection(id, name, &character.definition.description)
            }
            RealmResourceKind::Lorebook => {
                let book = self
                    .storage
                    .get_lorebook(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::Lorebook, id))?;
                projection(id, &book.name, &book.description)
            }
            RealmResourceKind::Template => {
                let template = self.templates.get_template(id, user_id).await?;
                projection(id, &template.name, &template.description)
            }
            RealmResourceKind::Tool => {
                let tool = self
                    .storage
                    .get_tool(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::UserTool, id))?;
                let name = tool.display_name.as_deref().unwrap_or(&tool.name);
                projection(id, name, &tool.description)
            }
            RealmResourceKind::Persona => {
                let persona = self.personas.get_persona(id, user_id).await?;
                let name = persona.display_name.as_deref().unwrap_or(&persona.name);
                projection(id, name, persona.description.as_deref().unwrap_or_default())
            }
            RealmResourceKind::ConnectionProfile => {
                let profile = self.connection.get_profile(user_id, id).await?;
                projection(id, &profile.name, &profile.provider)
            }
            RealmResourceKind::Widget => {
                let widget = self
                    .storage
                    .get_widget(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::UserWidget, id))?;
                projection(id, &widget.name, &widget.description)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use weaver_lang::parse_value_doc;

    use super::*;

    fn declarations(source: &str) -> Vec<weaver_lang::InputDecl> {
        parse_value_doc(source).expect("parses").inputs
    }

    #[test]
    fn a_slug_is_always_a_weaver_identifier() {
        for (name, want) in [
            ("Ito Kaede", "ito_kaede"),
            ("House Mōri", "house_m_ri"),
            ("  spaced  out  ", "spaced_out"),
            ("7th Legion", "_7th_legion"),
            ("!!!", "unnamed"),
            ("", "unnamed"),
            ("ALREADY_FINE", "already_fine"),
            ("a---b", "a_b"),
        ] {
            let got = slug(name);
            assert_eq!(got, want, "{name:?}");
            assert!(
                lorealm_storage::models::is_identifier(&got),
                "{got:?} is not an identifier"
            );
        }
    }

    #[test]
    fn only_entity_declarations_need_resolving() {
        let decls = declarations(
            r#"
#inputs
one: Ref<Character>
many: [Ref<Lorebook>]
nested: [{char: Ref<Character>, talkativeness: range(0, 1)}]
plain: string
numbers: [number]
choice: enum("a", "b") = "a"
dial: range(0, 10)
window: span(0, 10)

{}"#,
        );
        let got: Vec<bool> = decls.iter().map(|d| contains_entity(&d.ty)).collect();
        assert_eq!(got, [true, true, true, false, false, false, false, false]);
    }

    #[test]
    fn an_entity_inside_an_object_is_still_gathered() {
        let decls = declarations(
            r#"
#inputs
cast: [{char: Ref<Character>, talkativeness: range(0, 1)}]

{}"#,
        );
        let supplied = WeaverValue::Array(vec![
            WeaverValue::object([
                ("char", WeaverValue::String("11".into())),
                ("talkativeness", WeaverValue::Number(0.5)),
            ]),
            WeaverValue::object([
                ("char", WeaverValue::String("22".into())),
                ("talkativeness", WeaverValue::Number(0.1)),
            ]),
        ]);

        let mut found = Vec::new();
        gather("cast", &decls[0].ty, &supplied, &mut found).expect("gathers");

        assert_eq!(
            found,
            [
                (RealmResourceKind::Character, Snowflake(11)),
                (RealmResourceKind::Character, Snowflake(22)),
            ]
        );
    }

    #[test]
    fn substitution_replaces_the_reference_and_leaves_the_rest() {
        let decls = declarations(
            r#"
#inputs
cast: [{char: Ref<Character>, talkativeness: range(0, 1)}]

{}"#,
        );
        let projected = HashMap::from([(
            (RealmResourceKind::Character, Snowflake(11)),
            projection(Snowflake(11), "Ito Kaede", "A regent."),
        )]);
        let supplied = WeaverValue::Array(vec![WeaverValue::object([
            ("char", WeaverValue::String("11".into())),
            ("talkativeness", WeaverValue::Number(0.5)),
        ])]);

        let out = substitute("cast", &decls[0].ty, supplied, &projected).expect("substitutes");
        assert_eq!(
            out.to_json(),
            r#"[{"char":{"description":"A regent.","id":"11","name":"Ito Kaede","slug":"ito_kaede"},"talkativeness":0.5}]"#
        );
    }

    #[test]
    fn every_resource_kind_is_nameable_from_a_document() {
        for kind in RealmResourceKind::ALL {
            let ty = InputType::Ref(kind.ref_name().into());
            let mut found = Vec::new();
            gather("x", &ty, &WeaverValue::String("1".into()), &mut found).expect("gathers");
            assert_eq!(found, [(kind, Snowflake(1))]);
        }
    }

    #[test]
    fn an_unknown_kind_is_reported_against_its_input() {
        let mut found = Vec::new();
        let err = gather(
            "regents",
            &InputType::Ref("Wizard".into()),
            &WeaverValue::String("1".into()),
            &mut found,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("input 'regents'") && err.to_string().contains("Wizard"),
            "{err}"
        );
    }

    #[test]
    fn an_entity_input_may_not_carry_a_default() {
        let err = reject_defaulted_entities(&declarations(
            r#"
#inputs
lead: Ref<Character> = "12345"

{}"#,
        ))
        .unwrap_err();
        assert!(err.to_string().contains("cannot have a default"), "{err}");

        assert!(
            reject_defaulted_entities(&declarations(
                r#"
#inputs
pair: {char: Ref<Character>, weight: number} = 1

{}"#,
            ))
            .is_err()
        );

        assert!(
            reject_defaulted_entities(&declarations(
                r#"
#inputs
rounds: number = 3

{}"#,
            ))
            .is_ok()
        );
    }

    #[test]
    fn a_projection_carries_exactly_the_agreed_fields() {
        let value = projection(Snowflake(1934857293847562), "Ito Kaede", "A regent.");
        assert_eq!(
            value.to_json(),
            r#"{"description":"A regent.","id":"1934857293847562","name":"Ito Kaede","slug":"ito_kaede"}"#
        );
    }

    #[test]
    fn an_object_input_pairs_an_entity_with_its_own_settings() {
        let source = r#"
#inputs
cast: [{char: Ref<Character>, talkativeness: range(0, 1)}]

{
  name: r"Group Chat"
  agents: [
    {# foreach c in {{input:cast}} #}
      {
        char_version_id: null
        character_id: {{c.char.id}}
        handle: "speaker_{{c.char.slug}}"
        kind: r"llm"
        name: "{{c.char.name}}"
        persona_id: null
        profile_id: null
        properties: { talkativeness: {{c.talkativeness}} }
        template_id: null
        user_id: null
      }
    {# endforeach #}
  ]
}
"#;
        let entry = |id: i64, name: &str, talk: f64| {
            WeaverValue::object([
                ("char", projection(Snowflake(id), name, "")),
                ("talkativeness", WeaverValue::Number(talk)),
            ])
        };
        let resolved = BTreeMap::from([(
            "cast".to_string(),
            WeaverValue::Array(vec![entry(11, "Ito Kaede", 0.8), entry(22, "Mori Rin", 0.2)]),
        )]);

        let definition = super::super::wtn::from_wtn(source, resolved).expect("expands");
        let seen: Vec<(&str, &str, Option<Snowflake>, f64)> = definition
            .agents
            .iter()
            .map(|a| {
                (
                    a.handle.as_str(),
                    a.name.as_str(),
                    a.character_id,
                    a.properties["talkativeness"].as_f64().unwrap(),
                )
            })
            .collect();

        assert_eq!(
            seen,
            [
                ("speaker_ito_kaede", "Ito Kaede", Some(Snowflake(11)), 0.8),
                ("speaker_mori_rin", "Mori Rin", Some(Snowflake(22)), 0.2),
            ]
        );
    }

    #[test]
    fn a_scenario_can_be_filled_from_an_input() {
        let source = r######"
#inputs
custom: {name: string, text: string}

{
  name: r"Group Chat"
  scenarios: [
    { id: r"1", name: r"Spring", description: r"", opening: r"The snow gives way." }
    { id: r"2", name: "{{input:custom.name}}", description: r"", opening: "{{input:custom.text}}" }
  ]
}
"######;
        let resolved = BTreeMap::from([(
            "custom".to_string(),
            WeaverValue::object([
                ("name", WeaverValue::String("A tense summit".into())),
                ("text", WeaverValue::String("The envoys are seated.".into())),
            ]),
        )]);

        let definition = super::super::wtn::from_wtn(source, resolved).expect("expands");
        let seen: Vec<(&str, Option<&str>)> = definition
            .scenarios
            .iter()
            .map(|s| (s.name.as_str(), s.opening.as_deref()))
            .collect();

        assert_eq!(
            seen,
            [
                ("Spring", Some("The snow gives way.")),
                ("A tense summit", Some("The envoys are seated.")),
            ]
        );
    }

    #[test]
    fn a_document_reads_a_projection_as_an_object() {
        let source = r#"
#inputs
regents: [Ref<Character>]

{
  name: "Sengoku"
  description: r""
  scenarios: []
  director: null
  agents: [
    {# foreach c in {{input:regents}} #}
    {
      handle: "regent_{{c.slug}}"
      name: "{{c.name}}"
      kind: "llm"
      character_id: c.id
      char_version_id: null
      template_id: r"20"
      profile_id: r"30"
      user_id: null
      persona_id: null
    }
    {# endforeach #}
  ]
}"#;
        let resolved = BTreeMap::from([(
            "regents".to_string(),
            WeaverValue::Array(vec![
                projection(Snowflake(11), "Ito Kaede", "A regent."),
                projection(Snowflake(22), "7th Legion", "Not a person."),
            ]),
        )]);

        let definition = super::super::wtn::from_wtn(source, resolved).expect("expands");

        let seen: Vec<(&str, &str, Option<Snowflake>)> = definition
            .agents
            .iter()
            .map(|a| (a.handle.as_str(), a.name.as_str(), a.character_id))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("regent_ito_kaede", "Ito Kaede", Some(Snowflake(11))),
                ("regent__7th_legion", "7th Legion", Some(Snowflake(22))),
            ]
        );
    }
}
