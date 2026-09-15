//! Blueprints: the constructor stage in front of a realm.

use lorealm_storage::models::Snowflake;
use serde_json::{Map, json};

use crate::{
    errors::RealmError,
    services::blueprint::{InputSpec, InputSpecType},
    tests::Harness,
};

const SOURCE: &str = r#"
#inputs
regents: [Ref<Character>]
template: Ref<Template>
profile: Ref<ConnectionProfile>
difficulty: enum("gentle", "brutal") = "gentle"
rounds: number = 3

{
  name: "Sengoku"
  description: "A {{input:difficulty}} campaign over {{input:rounds}} rounds."
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
    }
    {# endforeach #}
  ]
}"#;

fn spec<'a>(specs: &'a [InputSpec], name: &str) -> &'a InputSpec {
    specs.iter().find(|s| s.name == name).expect(name)
}

#[tokio::test]
async fn a_blueprint_describes_the_form_that_instantiates_it() {
    let h = Harness::new().await;
    let blueprint = h
        .blueprints
        .create(h.user, "Sengoku".into(), SOURCE.into())
        .await
        .expect("saves");

    let specs = h
        .blueprints
        .inputs(blueprint.id, h.user)
        .await
        .expect("describes itself");

    let regents = spec(&specs, "regents");
    assert!(regents.required);
    assert_eq!(regents.declared, "[Ref<Character>]");
    assert!(
        matches!(
            &regents.ty,
            InputSpecType::List { of } if matches!(
                **of,
                InputSpecType::Reference {
                    kind: Some(lorealm_storage::models::RealmResourceKind::Character)
                }
            )
        ),
        "{:?}",
        regents.ty
    );

    let difficulty = spec(&specs, "difficulty");
    assert!(!difficulty.required);
    assert!(
        matches!(&difficulty.ty, InputSpecType::Enum { variants } if variants == &["gentle", "brutal"]),
        "{:?}",
        difficulty.ty
    );

    assert!(matches!(spec(&specs, "rounds").ty, InputSpecType::Number));
}

#[tokio::test]
async fn expanding_a_blueprint_produces_a_realm_that_remembers_where_it_came_from() {
    let h = Harness::new().await;
    let ito = h.character(h.user, "Ito Kaede").await;
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;

    let blueprint = h
        .blueprints
        .create(h.user, "Sengoku".into(), SOURCE.into())
        .await
        .unwrap();

    let mut inputs = Map::new();
    inputs.insert("regents".into(), json!([ito.to_string()]));
    inputs.insert("template".into(), json!(template.to_string()));
    inputs.insert("profile".into(), json!(profile.to_string()));
    inputs.insert("difficulty".into(), json!("brutal"));

    let realm = h
        .blueprints
        .expand(blueprint.id, h.user, inputs)
        .await
        .expect("expands");

    assert_eq!(realm.definition.agents.len(), 1);
    assert_eq!(realm.definition.agents[0].handle, "regent_ito_kaede");
    // A declared default filled in where no value was supplied.
    assert_eq!(
        realm.definition.description,
        "A brutal campaign over 3 rounds."
    );

    let origin = realm.origin.as_ref().expect("records its origin");
    assert_eq!(origin.blueprint_id, blueprint.id);
    // Ids, not the projections they resolved to — re-expandable later.
    assert_eq!(origin.inputs["regents"], json!([ito.to_string()]));
    assert_eq!(origin.inputs["difficulty"], json!("brutal"));

    // And it survives the round trip through storage.
    let loaded = h.realms.get_realm(realm.id, h.user).await.unwrap();
    assert_eq!(
        loaded.origin.expect("origin persists").blueprint_id,
        blueprint.id
    );
}

#[tokio::test]
async fn a_realm_that_was_never_expanded_has_no_origin() {
    let h = Harness::new().await;
    let realm = h.realms.create_realm("By hand", h.user).await.unwrap();
    assert!(realm.origin.is_none());
}

#[tokio::test]
async fn a_document_that_does_not_parse_is_refused_on_save() {
    let h = Harness::new().await;
    let err = h
        .blueprints
        .create(h.user, "Broken".into(), "{ name: ".into())
        .await
        .unwrap_err();
    assert!(matches!(err, RealmError::InvalidTemplate(_)), "{err:?}");
    assert!(h.blueprints.list(h.user).await.unwrap().is_empty());
}

#[tokio::test]
async fn an_entity_input_with_a_default_is_refused_on_save() {
    // Caught here rather than at expansion, where the value would silently
    // bind as a bare id.
    let h = Harness::new().await;
    let err = h
        .blueprints
        .create(
            h.user,
            "Defaulted".into(),
            r#"
#inputs
lead: Ref<Character> = "12345"

{ name: r"x" }"#
                .into(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(&err, RealmError::InvalidTemplate(hint) if hint.contains("cannot have a default")),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_blueprint_belongs_to_its_owner() {
    let h = Harness::new().await;
    let blueprint = h
        .blueprints
        .create(h.user, "Mine".into(), SOURCE.into())
        .await
        .unwrap();

    for err in [
        h.blueprints
            .get(blueprint.id, h.stranger)
            .await
            .unwrap_err(),
        h.blueprints
            .inputs(blueprint.id, h.stranger)
            .await
            .unwrap_err(),
        h.blueprints
            .expand(blueprint.id, h.stranger, Map::new())
            .await
            .unwrap_err(),
    ] {
        assert!(matches!(err, RealmError::NotOwned(_, _)), "{err:?}");
    }

    assert!(h.blueprints.list(h.stranger).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_missing_required_input_is_reported_rather_than_guessed() {
    let h = Harness::new().await;
    let blueprint = h
        .blueprints
        .create(h.user, "Sengoku".into(), SOURCE.into())
        .await
        .unwrap();

    let err = h
        .blueprints
        .expand(blueprint.id, h.user, Map::new())
        .await
        .unwrap_err();
    assert!(
        matches!(&err, RealmError::InvalidTemplate(hint) if hint.contains("regents")),
        "{err:?}"
    );
}

#[tokio::test]
async fn updating_the_source_keeps_the_blueprint_usable() {
    let h = Harness::new().await;
    let blueprint = h
        .blueprints
        .create(h.user, "Sengoku".into(), SOURCE.into())
        .await
        .unwrap();

    // A bad edit is rejected and leaves the stored source alone.
    let err = h
        .blueprints
        .update(blueprint.id, h.user, None, None, Some("{ name: ".into()))
        .await
        .unwrap_err();
    assert!(matches!(err, RealmError::InvalidTemplate(_)), "{err:?}");
    assert_eq!(
        h.blueprints.get(blueprint.id, h.user).await.unwrap().source,
        SOURCE
    );

    let updated = h
        .blueprints
        .update(
            blueprint.id,
            h.user,
            Some("Renamed".into()),
            None,
            Some(r#"{ name: r"Minimal" }"#.into()),
        )
        .await
        .expect("accepts a good edit");
    assert_eq!(updated.name, "Renamed");
    assert!(
        h.blueprints
            .inputs(blueprint.id, h.user)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn deleting_a_blueprint_leaves_the_realms_it_made() {
    let h = Harness::new().await;
    let ito = h.character(h.user, "Ito").await;
    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;
    let blueprint = h
        .blueprints
        .create(h.user, "Sengoku".into(), SOURCE.into())
        .await
        .unwrap();

    let mut inputs = Map::new();
    inputs.insert("regents".into(), json!([ito.to_string()]));
    inputs.insert("template".into(), json!(template.to_string()));
    inputs.insert("profile".into(), json!(profile.to_string()));
    let realm = h
        .blueprints
        .expand(blueprint.id, h.user, inputs)
        .await
        .unwrap();

    h.blueprints.delete(blueprint.id, h.user).await.unwrap();

    // The realm is a materialization, not a view: it keeps working, and it
    // still names the blueprint it no longer has.
    let loaded = h
        .realms
        .get_realm(realm.id, h.user)
        .await
        .expect("still there");
    assert_eq!(loaded.definition.agents.len(), 1);
    assert_eq!(loaded.origin.unwrap().blueprint_id, blueprint.id);
}

#[tokio::test]
async fn ids_stay_strings_through_the_input_bridge() {
    // A Snowflake past 2^53 would come back a different number if it ever
    // travelled as one.
    let h = Harness::new().await;
    let big: Snowflake = h.character(h.user, "Late Arrival").await;
    assert!(big.0 > (1i64 << 53), "test id is not large enough: {big}");

    let template = h.template(h.user, "Roleplay").await;
    let profile = h.profile(h.user, "Local").await;
    let blueprint = h
        .blueprints
        .create(h.user, "Sengoku".into(), SOURCE.into())
        .await
        .unwrap();

    let mut inputs = Map::new();
    inputs.insert("regents".into(), json!([big.to_string()]));
    inputs.insert("template".into(), json!(template.to_string()));
    inputs.insert("profile".into(), json!(profile.to_string()));

    let realm = h
        .blueprints
        .expand(blueprint.id, h.user, inputs)
        .await
        .unwrap();
    assert_eq!(realm.definition.agents[0].character_id, Some(big));
}
