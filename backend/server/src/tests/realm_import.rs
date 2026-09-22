//! Importing a WTN document: the path untrusted content takes into a realm.

use std::collections::BTreeMap;

use lorealm_storage::{CharacterStorage, models::AgentKind};
use weaver_lang::Value as WeaverValue;

use crate::{errors::RealmError, tests::Harness};

/// The strategy-game shape from the design doc: a cast built from whatever
/// characters the importer picked.
const REGENTS: &str = r#"
#inputs
regents: [Ref<Character>]
template: Ref<Template>
profile: Ref<ConnectionProfile>

{
  name: "Sengoku"
  description: r"Houses scheme through the seasons."
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
      template_id: {{input:template.id}}
      profile_id: {{input:profile.id}}
      user_id: null
      persona_id: null
      run_condition: r"{{self:alive}} == true"
    }
    {# endforeach #}
  ]
}"#;

fn ids(values: &[&str]) -> WeaverValue {
    WeaverValue::Array(values.iter().map(|v| WeaverValue::from(*v)).collect())
}

#[tokio::test]
async fn a_document_becomes_a_realm_built_from_the_characters_it_was_given() {
    let h = Harness::new().await;
    let ito = h.character(h.user, "Ito Kaede").await;
    let mori = h.character(h.user, "House Mōri").await;
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;

    let inputs = BTreeMap::from([
        (
            "regents".to_string(),
            ids(&[&ito.to_string(), &mori.to_string()]),
        ),
        ("template".to_string(), template.to_string().into()),
        ("profile".to_string(), profile.to_string().into()),
    ]);

    let realm = h
        .realms
        .import(h.user, REGENTS, inputs, None)
        .await
        .expect("imports");

    assert_eq!(realm.owner_id, h.user);
    assert_eq!(realm.definition.name, "Sengoku");

    let cast: Vec<(&str, &str)> = realm
        .definition
        .agents
        .iter()
        .map(|a| (a.handle.as_str(), a.name.as_str()))
        .collect();
    assert_eq!(
        cast,
        vec![
            ("regent_ito_kaede", "Ito Kaede"),
            ("regent_house_m_ri", "House Mōri"),
        ]
    );

    assert_eq!(realm.definition.agents[0].character_id, Some(ito));
    assert_eq!(realm.definition.agents[1].character_id, Some(mori));
    assert_eq!(realm.definition.agents[0].template_id, Some(template));
    assert_eq!(realm.definition.agents[0].kind, AgentKind::Llm);

    let loaded = h.realms.get_realm(realm.id, h.user).await.expect("loads");
    assert_eq!(loaded.definition.agents.len(), 2);
}

const GROUP_CHAT: &str = r#"
#inputs
lead: Ref<Character>
who: Ref<Persona>
template: Ref<Template>
profile: Ref<ConnectionProfile>

{
  name: "Group Chat"
  description: r"You and one other."
  scenarios: []
  director: null
  agents: [
    {
      handle: "{{input:who.slug}}"
      name: "{{input:who.name}}"
      kind: "player"
      character_id: null
      char_version_id: null
      template_id: {{input:template.id}}
      profile_id: {{input:profile.id}}
      persona_id: {{input:who.id}}
      user_id: null
    }
    {
      handle: "{{input:lead.slug}}"
      name: "{{input:lead.name}}"
      kind: "llm"
      character_id: {{input:lead.id}}
      char_version_id: null
      template_id: {{input:template.id}}
      profile_id: {{input:profile.id}}
      persona_id: null
      user_id: null
    }
  ]
}"#;

#[tokio::test]
async fn expansion_binds_the_player_to_the_caller_and_pins_the_character_version() {
    let h = Harness::new().await;
    let lead = h.character(h.user, "Sayuri").await;
    let who = h.persona(h.user, "Me").await;
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;

    let inputs = BTreeMap::from([
        ("lead".to_string(), lead.to_string().into()),
        ("who".to_string(), who.to_string().into()),
        ("template".to_string(), template.to_string().into()),
        ("profile".to_string(), profile.to_string().into()),
    ]);

    let realm = h
        .realms
        .import(h.user, GROUP_CHAT, inputs, None)
        .await
        .expect("imports");

    let player = &realm.definition.agents[0];
    assert_eq!(player.kind, AgentKind::Player);
    assert_eq!(player.user_id, Some(h.user));

    let actor = &realm.definition.agents[1];
    assert_eq!(actor.kind, AgentKind::Llm);
    assert_eq!(actor.character_id, Some(lead));
    assert_eq!(
        actor.char_version_id,
        Some(h.store.get_latest_version(lead).await.unwrap()),
        "an agent with a character but no version cannot take a turn"
    );
}

#[tokio::test]
async fn a_template_cannot_reach_a_character_the_importer_does_not_own() {
    let h = Harness::new().await;
    let theirs = h.character(h.stranger, "Someone Else").await;
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;

    let inputs = BTreeMap::from([
        ("regents".to_string(), ids(&[&theirs.to_string()])),
        ("template".to_string(), template.to_string().into()),
        ("profile".to_string(), profile.to_string().into()),
    ]);

    let err = h
        .realms
        .import(h.user, REGENTS, inputs, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, RealmError::NotOwned(_, id) if id == theirs),
        "{err:?}"
    );
}

#[tokio::test]
async fn an_id_that_resolves_to_nothing_is_refused() {
    let h = Harness::new().await;
    let missing = h.snow.generate();
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;

    let inputs = BTreeMap::from([
        ("regents".to_string(), ids(&[&missing.to_string()])),
        ("template".to_string(), template.to_string().into()),
        ("profile".to_string(), profile.to_string().into()),
    ]);

    let err = h
        .realms
        .import(h.user, REGENTS, inputs, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, RealmError::NotFound(_, id) if id == missing),
        "{err:?}"
    );
}

#[tokio::test]
async fn every_resource_kind_projects_its_name_and_description() {
    let h = Harness::new().await;
    let source = r#"
#inputs
character: Ref<Character>
lorebook: Ref<Lorebook>
template: Ref<Template>
tool: Ref<Tool>
persona: Ref<Persona>
profile: Ref<ConnectionProfile>
widget: Ref<Widget>

{
  name: "{{input:character.name}}"
  description: r""
  agents: []
  scenarios: [
    { id: r"1", name: "{{input:lorebook.name}}", description: r"", opening: null }
    { id: r"2", name: "{{input:template.name}}", description: r"", opening: null }
    { id: r"3", name: "{{input:tool.name}}", description: r"", opening: null }
    { id: r"4", name: "{{input:persona.name}}", description: r"", opening: null }
    { id: r"5", name: "{{input:profile.name}}", description: r"", opening: null }
    { id: r"6", name: "{{input:widget.name}}", description: r"", opening: null }
  ]
  director: null
}"#;

    let inputs = BTreeMap::from([
        (
            "character".to_string(),
            h.character(h.user, "A Character").await.to_string().into(),
        ),
        (
            "lorebook".to_string(),
            h.lorebook(h.user, "A Lorebook").await.to_string().into(),
        ),
        (
            "template".to_string(),
            h.template(h.user, "A Template").await.to_string().into(),
        ),
        (
            "tool".to_string(),
            h.tool(h.user, "A Tool").await.to_string().into(),
        ),
        (
            "persona".to_string(),
            h.persona(h.user, "A Persona").await.to_string().into(),
        ),
        (
            "profile".to_string(),
            h.profile(h.user, "A Profile").await.to_string().into(),
        ),
        (
            "widget".to_string(),
            h.widget(h.user, "A Widget").await.to_string().into(),
        ),
    ]);

    let realm = h
        .realms
        .import(h.user, source, inputs, None)
        .await
        .expect("imports");

    assert_eq!(realm.definition.name, "A Character");
    let names: Vec<&str> = realm
        .definition
        .scenarios
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "A Lorebook",
            "A Template",
            "A Tool",
            "A Persona",
            "A Profile",
            "A Widget",
        ]
    );
}

#[tokio::test]
async fn an_expanded_definition_is_validated_before_it_is_saved() {
    let h = Harness::new().await;
    let character = h.character(h.user, "Ito").await;

    // Two loop iterations over the same name collide on the handle.
    let source = r#"
#inputs
regents: [Ref<Character>]

{
  name: "Collides"
  description: r""
  scenarios: []
  director: null
  agents: [
    {# foreach c in {{input:regents}} #}
    {
      handle: "regent"
      name: "{{c.name}}"
      kind: "player"
      character_id: null
      char_version_id: null
      template_id: null
      profile_id: null
      user_id: null
      persona_id: null
    }
    {# endforeach #}
  ]
}"#;

    let inputs = BTreeMap::from([(
        "regents".to_string(),
        ids(&[&character.to_string(), &character.to_string()]),
    )]);

    let err = h
        .realms
        .import(h.user, source, inputs, None)
        .await
        .unwrap_err();
    assert!(
        matches!(&err, RealmError::InvalidAgent(hint) if hint.contains("already taken")),
        "{err:?}"
    );
    assert!(h.realms.list_realms(h.user).await.unwrap().is_empty());
}

#[tokio::test]
async fn an_llm_agent_without_a_template_is_caught_at_import() {
    let h = Harness::new().await;
    let source = r#"
{
  name: "Half a realm"
  description: r""
  scenarios: []
  director: null
  agents: [
    {
      handle: "narrator"
      name: "Narrator"
      kind: "llm"
      character_id: null
      char_version_id: null
      template_id: null
      profile_id: null
      user_id: null
      persona_id: null
    }
  ]
}"#;

    let err = h
        .realms
        .import(h.user, source, BTreeMap::new(), None)
        .await
        .unwrap_err();
    assert!(
        matches!(&err, RealmError::InvalidAgent(hint) if hint.contains("needs a template")),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_realm_exports_to_a_document_that_imports_back() {
    let h = Harness::new().await;
    let ito = h.character(h.user, "Ito Kaede").await;
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;

    let inputs = BTreeMap::from([
        ("regents".to_string(), ids(&[&ito.to_string()])),
        ("template".to_string(), template.to_string().into()),
        ("profile".to_string(), profile.to_string().into()),
    ]);
    let original = h
        .realms
        .import(h.user, REGENTS, inputs, None)
        .await
        .unwrap();

    // The export has no inputs left as it's now a concrete realm, no longer a blueprint.
    let document = h.realms.export(original.id, h.user).await.unwrap();
    let copy = h
        .realms
        .import(h.user, &document, BTreeMap::new(), None)
        .await
        .expect("re-imports");

    assert_eq!(
        serde_json::to_value(&copy.definition).unwrap(),
        serde_json::to_value(&original.definition).unwrap()
    );
    assert_ne!(copy.id, original.id);
}
