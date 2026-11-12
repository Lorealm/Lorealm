use std::{io::Cursor, sync::Arc};

use chrono::Utc;
use lorealm_storage::{
    PersonaStorage, Storage, UserToolStorage,
    models::{RealmResourceKind, Snowflake, UserPersona},
};
use serde_json::Map;

use crate::{
    errors::RealmError,
    services::{
        blueprint::{ResourceStatus, static_refs},
        module::ModuleService,
        realm::wtn,
    },
    tests::Harness,
};

const COURT: &str = r#"
#inputs
profile: Ref<ConnectionProfile>

{
  name: "Court"
  modules: ["MODULE"]
  agents: [
    {
      handle: "lord"
      name: "Lord"
      kind: "llm"
      character_id: "CHARACTER"
      template_id: "TEMPLATE"
      profile_id: {{input:profile.id}}
      lorebooks: ["LOREBOOK"]
      tools: [{ tool_id: "TOOL", event_channel: null }]
    }
    {
      handle: "you"
      name: "You"
      kind: "player"
      persona_id: "PERSONA"
      widgets: ["WIDGET"]
    }
  ]
}"#;

const DICE: &str = "export sides = 6\n";

fn modules(h: &Harness) -> ModuleService {
    ModuleService::new(
        Arc::clone(&h.store) as Arc<dyn Storage>,
        Arc::clone(&h.snow),
    )
}

fn pixel() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1, 1)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

struct Court {
    blueprint: Snowflake,
    /// Every id the source names.
    ids: Vec<Snowflake>,
}

/// A blueprint naming one of every exportable kind, owned by `h.user`.
async fn court(h: &Harness) -> Court {
    let character = h.character(h.user, "Lord Oda").await;
    let template = h.template(h.user, "Roleplay").await;
    let lorebook = h.lorebook(h.user, "Provinces").await;
    let tool = h.tool(h.user, "decree").await;
    let widget = h.widget(h.user, "treasury").await;
    let module = modules(h)
        .create(h.user, "dice".into(), String::new(), DICE.into())
        .await
        .expect("module saves")
        .id;

    let persona = h.snow.generate();
    let avatar = h.blobs.put(&pixel()).await.unwrap();
    h.store
        .save_persona(UserPersona {
            id: persona,
            user_id: h.user,
            name: "Envoy".into(),
            display_name: None,
            pseudonym: None,
            avatar_uri: Some(avatar.to_uri()),
            description: Some("Speaks for the capital.".into()),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

    let source = COURT
        .replace("CHARACTER", &character.to_string())
        .replace("TEMPLATE", &template.to_string())
        .replace("LOREBOOK", &lorebook.to_string())
        .replace("TOOL", &tool.to_string())
        .replace("WIDGET", &widget.to_string())
        .replace("MODULE", &module.to_string())
        .replace("PERSONA", &persona.to_string());
    let blueprint = h
        .blueprints
        .create(h.user, "Court".into(), source)
        .await
        .expect("blueprint saves");

    Court {
        blueprint: blueprint.id,
        ids: vec![character, template, lorebook, tool, widget, module, persona],
    }
}

#[test]
fn written_ids_are_found_through_loops_and_conditionals() {
    let doc = wtn::compile(
        r#"
#inputs
extras: [Ref<Character>]
big: bool = false

{
  name: "Arena"
  modules: ["70"]
  agents: [
    { handle: "a", character_id: "10", properties: { character_id: "999" } }
    {# foreach c in {{input:extras}} #}
    { handle: "x_{{c.slug}}", character_id: c.id, template_id: "20" }
    {# endforeach #}
    {# if {{input:big}} #}
    { handle: "b", lorebooks: ["30", "31"], tools: [{ tool_id: "40" }] }
    {# endif #}
  ]
}"#,
    )
    .expect("compiles");

    let found: Vec<(RealmResourceKind, i64)> = static_refs(doc.document())
        .iter()
        .map(|r| (r.kind, r.id.0))
        .collect();

    use RealmResourceKind::*;
    assert_eq!(
        found,
        vec![
            (Module, 70),
            (Character, 10),
            (Template, 20),
            (Lorebook, 30),
            (Lorebook, 31),
            (Tool, 40),
        ]
    );
}

#[tokio::test]
async fn a_bundle_rebuilds_the_blueprint_in_another_library() {
    let h = Harness::new().await;
    let court = court(&h).await;

    let bundle = h
        .blueprints
        .export_bundle(court.blueprint, h.user)
        .await
        .expect("exports");
    assert_eq!(bundle.file_name, "Court.lrlm");

    let imported = h
        .blueprints
        .import_bundle(h.stranger, &bundle.bytes)
        .await
        .expect("imports");

    assert_eq!(imported.blueprint.owner_id, h.stranger);
    assert_eq!(imported.resources.len(), 7);
    assert!(imported.skipped.is_empty(), "{:?}", imported.skipped);
    for id in &court.ids {
        assert!(
            !imported.blueprint.source.contains(&id.to_string()),
            "the author's id {id} is still in the source"
        );
    }

    // Ownership is the only gate a realm has, so expanding proves every
    // reference now points at something the importer owns.
    let profile = h.profile(h.stranger, "Local").await;
    let mut inputs = Map::new();
    inputs.insert("profile".into(), profile.to_string().into());
    let realm = h
        .blueprints
        .expand(imported.blueprint.id, h.stranger, inputs)
        .await
        .expect("expands for the importer");

    let lord = &realm.definition.agents[0];
    let you = &realm.definition.agents[1];
    assert!(lord.character_id.is_some_and(|id| id != court.ids[0]));
    assert_eq!(lord.lorebooks.len(), 1);
    assert_eq!(lord.tools.len(), 1);
    assert_eq!(you.widgets.len(), 1);
    assert_eq!(realm.definition.modules.len(), 1);

    let persona = h
        .store
        .get_persona(you.persona_id.expect("a persona"))
        .await
        .unwrap()
        .expect("saved");
    assert_eq!(persona.user_id, h.stranger);
    assert_eq!(
        persona.description.as_deref(),
        Some("Speaks for the capital.")
    );
    assert!(persona.avatar_uri.is_some(), "the avatar came along");
}

#[tokio::test]
async fn a_preview_lists_what_would_arrive_and_saves_nothing() {
    let h = Harness::new().await;
    let court = court(&h).await;
    let bundle = h
        .blueprints
        .export_bundle(court.blueprint, h.user)
        .await
        .expect("exports");

    let preview = h
        .blueprints
        .preview_bundle(h.stranger, &bundle.bytes)
        .await
        .expect("previews");

    assert_eq!(preview.name, "Court");
    let mut listed: Vec<(&str, &str)> = preview
        .resources
        .iter()
        .map(|r| (r.kind.as_str(), r.name.as_str()))
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        vec![
            ("character", "Lord Oda"),
            ("lorebook", "Provinces"),
            ("module", "dice"),
            ("persona", "Envoy"),
            ("template", "Roleplay"),
            ("tool", "decree"),
            ("widget", "treasury"),
        ]
    );
    assert!(
        preview
            .resources
            .iter()
            .all(|r| r.status == ResourceStatus::New)
    );
    assert!(h.blueprints.list(h.stranger).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_module_name_already_taken_stops_the_import_before_anything_is_saved() {
    let h = Harness::new().await;
    let court = court(&h).await;
    let bundle = h
        .blueprints
        .export_bundle(court.blueprint, h.user)
        .await
        .expect("exports");
    modules(&h)
        .create(
            h.stranger,
            "dice".into(),
            String::new(),
            "export sides = 20\n".into(),
        )
        .await
        .expect("saves");

    let preview = h
        .blueprints
        .preview_bundle(h.stranger, &bundle.bytes)
        .await
        .expect("previews");
    let dice = preview
        .resources
        .iter()
        .find(|r| r.kind == RealmResourceKind::Module)
        .unwrap();
    assert_eq!(dice.status, ResourceStatus::Blocked);

    let refused = h.blueprints.import_bundle(h.stranger, &bundle.bytes).await;
    assert!(matches!(refused, Err(RealmError::InvalidBundle(_))));
    assert!(h.store.list_tools(h.stranger).await.unwrap().is_empty());
}

#[tokio::test]
async fn an_identical_module_is_reused() {
    let h = Harness::new().await;
    let court = court(&h).await;
    let bundle = h
        .blueprints
        .export_bundle(court.blueprint, h.user)
        .await
        .expect("exports");
    let mine = modules(&h)
        .create(h.stranger, "dice".into(), String::new(), DICE.into())
        .await
        .expect("saves");

    let imported = h
        .blueprints
        .import_bundle(h.stranger, &bundle.bytes)
        .await
        .expect("imports");

    let dice = imported
        .resources
        .iter()
        .find(|r| r.kind == RealmResourceKind::Module)
        .unwrap();
    assert!(dice.reused);
    assert_eq!(dice.id, mine.id);
}

#[tokio::test]
async fn a_written_connection_profile_refuses_the_export() {
    let h = Harness::new().await;
    let profile = h.profile(h.user, "Local").await;
    let source = format!(
        "{{\n  name: \"Solo\"\n  agents: [\n    {{ handle: \"a\", name: \"A\", kind: \"llm\", profile_id: \"{profile}\" }}\n  ]\n}}"
    );
    let blueprint = h
        .blueprints
        .create(h.user, "Solo".into(), source)
        .await
        .expect("saves");

    let refused = h.blueprints.export_bundle(blueprint.id, h.user).await;

    let Err(RealmError::NotExportable(why)) = refused else {
        panic!("a written profile must refuse the export");
    };
    assert!(why.starts_with("Line 4 "), "{why}");
    assert!(why.contains("API keys"), "{why}");
}

#[tokio::test]
async fn a_reference_to_something_deleted_refuses_the_export() {
    let h = Harness::new().await;
    let blueprint = h
        .blueprints
        .create(
            h.user,
            "Ghost".into(),
            "{\n  name: \"Ghost\"\n  agents: [{ handle: \"a\", name: \"A\", kind: \"llm\", character_id: \"12345\" }]\n}".into(),
        )
        .await
        .expect("saves");

    let refused = h.blueprints.export_bundle(blueprint.id, h.user).await;

    assert!(
        matches!(&refused, Err(RealmError::NotExportable(why)) if why.contains("character")),
        "{:?}",
        refused.err()
    );
}

#[tokio::test]
async fn a_file_that_is_not_a_bundle_is_refused() {
    let h = Harness::new().await;

    let refused = h.blueprints.preview_bundle(h.user, b"not a zip").await;

    assert!(matches!(refused, Err(RealmError::InvalidBundle(_))));
}
