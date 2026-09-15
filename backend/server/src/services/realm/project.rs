use std::collections::BTreeMap;

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

/// What a declaration asks the host to resolve before expansion.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum Needs {
    One(RealmResourceKind),
    Many(RealmResourceKind),
    /// A shape the language checks itself: string, number, bool, enum, or a
    /// list of those.
    Nothing,
}

/// Classify one declaration, naming the input if its kind is unknown.
pub(super) fn needs(input: &str, ty: &InputType) -> Result<Needs, RealmError> {
    let named = |kind: &str| {
        RealmResourceKind::from_ref_name(kind).ok_or_else(|| {
            RealmError::InvalidTemplate(format!("input '{input}': no such entity kind '{kind}'"))
        })
    };
    Ok(match ty {
        InputType::Ref(kind) => Needs::One(named(kind)?),
        InputType::List(inner) => match &**inner {
            InputType::Ref(kind) => Needs::Many(named(kind)?),
            _ => Needs::Nothing,
        },
        _ => Needs::Nothing,
    })
}

/// An entity input cannot carry a default.
pub fn reject_defaulted_entities(
    declarations: &[weaver_lang::InputDecl],
) -> Result<(), RealmError> {
    for declaration in declarations {
        if !declaration.is_required()
            && needs(&declaration.name, &declaration.ty)? != Needs::Nothing
        {
            return Err(RealmError::InvalidTemplate(format!(
                "input '{}' names an entity, so it cannot have a default — make it required",
                declaration.name
            )));
        }
    }
    Ok(())
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

impl RealmService {
    /// Bind every declared input, resolving the ones that name an entity.
    pub(super) async fn resolve_inputs(
        &self,
        user_id: Snowflake,
        declarations: &[weaver_lang::InputDecl],
        supplied: BTreeMap<String, WeaverValue>,
    ) -> Result<BTreeMap<String, WeaverValue>, RealmError> {
        reject_defaulted_entities(declarations)?;

        let mut bound = BTreeMap::new();
        for (name, value) in supplied {
            let Some(declaration) = declarations.iter().find(|d| d.name == name) else {
                bound.insert(name, value);
                continue;
            };

            let resolved = match needs(&name, &declaration.ty)? {
                Needs::Nothing => value,
                Needs::One(kind) => {
                    let id = as_id(&name, kind, &value)?;
                    self.project(user_id, kind, id).await?
                }
                Needs::Many(kind) => {
                    let items = value.into_array().ok_or_else(|| {
                        RealmError::InvalidTemplate(format!(
                            "input '{name}': expected a list of {} ids",
                            kind.as_str()
                        ))
                    })?;
                    let mut out = Vec::with_capacity(items.len());
                    for item in &items {
                        let id = as_id(&name, kind, item)?;
                        out.push(self.project(user_id, kind, id).await?);
                    }
                    WeaverValue::Array(out)
                }
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
plain: string
numbers: [number]
choice: enum("a", "b") = "a"

{}"#,
        );
        let got: Vec<Needs> = decls
            .iter()
            .map(|d| needs(&d.name, &d.ty).unwrap())
            .collect();

        assert_eq!(
            got,
            vec![
                Needs::One(RealmResourceKind::Character),
                Needs::Many(RealmResourceKind::Lorebook),
                Needs::Nothing,
                Needs::Nothing,
                Needs::Nothing,
            ]
        );
    }

    #[test]
    fn every_resource_kind_is_nameable_from_a_document() {
        for kind in RealmResourceKind::ALL {
            assert_eq!(
                needs("x", &InputType::Ref(kind.ref_name().into())).unwrap(),
                Needs::One(kind)
            );
        }
    }

    #[test]
    fn an_unknown_kind_is_reported_against_its_input() {
        let err = needs("regents", &InputType::Ref("Wizard".into())).unwrap_err();
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
