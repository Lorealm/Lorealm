use std::sync::Arc;

use lorealm_storage::Storage;

use crate::{
    errors::ModuleError,
    services::{
        context_template::weaver::check::{Dialect, Severity, check_with},
        module::ModuleService,
        weaver::{WeaverHost, WeaverProfile, compile_expr, compile_template},
    },
    tests::Harness,
};

const COMBAT: &str = r#"
// Where the fight happens.
export title = "Arena"
export hit_table = [1, 2, 4, 8]

// How hard a blow lands.
export proc damage(base: number, mult: number = 1) -> number
  {# return {{input:base}} * {{input:mult}} #}
{# endproc #}

// Promises a number, renders text.
export proc liar() -> number
  twelve
{# endproc #}

export proc toast_hit(who: string)
  $[toast({{input:who}} + " was hit")]
{# endproc #}
"#;

fn modules(h: &Harness) -> ModuleService {
    ModuleService::new(
        Arc::clone(&h.store) as Arc<dyn Storage>,
        Arc::clone(&h.snow),
    )
}

async fn with_combat() -> (Harness, ModuleService) {
    let h = Harness::new().await;
    let service = modules(&h);
    service
        .create(h.user, "combat".into(), String::new(), COMBAT.into())
        .await
        .expect("saves");
    (h, service)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_processor_and_value_reach_a_host() {
    let (h, _) = with_combat().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmScript)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();

    let body = compile_template(
        "{{combat:title}}: @[combat.damage(3, mult: 2)] / {{combat:hit_table[2]}}",
    )
    .unwrap();
    let outcome = host.evaluate(body).await.unwrap();

    assert_eq!(outcome.to_output_string(), "Arena: 6 / 4");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_processor_runs_inline_with_its_defaults() {
    let (h, _) = with_combat().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmCondition)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();

    let outcome = host
        .evaluate_inline(compile_expr("@[combat.damage(base: 5)] == 5").unwrap())
        .unwrap();

    assert!(outcome.is_truthy());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_processor_has_the_callers_capabilities() {
    let (h, _) = with_combat().await;
    let body = r#"@[combat.toast_hit("Ada")]"#;

    let command = WeaverHost::builder()
        .profile(WeaverProfile::Command)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();
    let outcome = command
        .evaluate(compile_template(body).unwrap())
        .await
        .unwrap();
    assert_eq!(outcome.effects.len(), 1);

    let condition = WeaverHost::builder()
        .profile(WeaverProfile::RealmCondition)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();
    assert!(
        condition
            .evaluate_inline(compile_template(body).unwrap())
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_scope_is_read_only() {
    let (h, _) = with_combat().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmScript)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();

    let err = host
        .evaluate(compile_template(r#"$[set_var("combat:title", 1)]"#).unwrap())
        .await
        .unwrap_err();

    assert!(err.to_string().contains("read-only"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_sharing_a_state_namespace_is_an_error() {
    let (h, _) = with_combat().await;
    let pool = std::sync::Arc::new(std::sync::Mutex::new(
        [(
            "combat".to_string(),
            [("title".to_string(), serde_json::json!("Old"))].into(),
        )]
        .into(),
    ));
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmScript)
        .pool(pool)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();

    let err = host
        .evaluate(compile_template("{{combat:title}}").unwrap())
        .await
        .unwrap_err();

    assert!(err.to_string().contains("both a module"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_disabled_module_is_not_registered() {
    let (h, service) = with_combat().await;
    let id = service.list(h.user).await.unwrap()[0].id;
    service
        .update(id, h.user, None, None, None, Some(false))
        .await
        .unwrap();

    assert!(h.plugins.vocabulary(h.user).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_editor_knows_module_processors() {
    let (h, _) = with_combat().await;
    let vocabulary = h.plugins.vocabulary(h.user).await;

    let found = check_with(
        "@[combat.damage(2)] @[combat.nope()]",
        Dialect::Template,
        None,
        vocabulary.as_ref(),
    );

    let messages: Vec<_> = found.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["unknown processor `combat.nope`"]);
}

#[tokio::test]
async fn a_name_must_be_free_and_an_identifier() {
    let (h, service) = with_combat().await;
    let create =
        |name: &str| service.create(h.user, name.into(), String::new(), "export x = 1".into());

    assert!(matches!(
        create("math").await,
        Err(ModuleError::NameTaken(_))
    ));
    assert!(matches!(
        create("cast").await,
        Err(ModuleError::NameTaken(_))
    ));
    assert!(matches!(
        create("local").await,
        Err(ModuleError::NameTaken(_))
    ));
    assert!(matches!(
        create("combat").await,
        Err(ModuleError::NameTaken(_))
    ));
    assert!(matches!(
        create("2fast").await,
        Err(ModuleError::Invalid(_))
    ));
    assert!(matches!(
        create("has-dash").await,
        Err(ModuleError::Invalid(_))
    ));
    // Another user's module does not take the name.
    assert!(
        service
            .create(
                h.stranger,
                "combat".into(),
                String::new(),
                "export x = 1".into()
            )
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn source_is_checked_on_save() {
    let h = Harness::new().await;
    let service = modules(&h);
    let create = |source: &str| service.create(h.user, "m".into(), String::new(), source.into());

    // Does not parse.
    assert!(matches!(
        create("export = 1").await,
        Err(ModuleError::Invalid(_))
    ));
    // Parses, but a value may only be a constant.
    assert!(matches!(
        create("export x = {{state:gold}}").await,
        Err(ModuleError::Invalid(_))
    ));
}

#[test]
fn the_module_dialect_reports_unknown_processors_in_bodies() {
    let found = check_with(
        "export proc f()\n  @[math.nope()]\n{# endproc #}",
        Dialect::Module,
        None,
        None,
    );

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Warning);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_editor_can_complete_module_values() {
    let (h, _) = with_combat().await;
    let vocabulary = h.plugins.vocabulary(h.user).await.unwrap();

    let namespaces = vocabulary.module_namespaces();

    let [combat] = namespaces.as_slice() else {
        panic!("expected one module namespace, got {namespaces:?}");
    };
    assert_eq!(combat.scope, "combat");
    let members: Vec<(&str, String, &str)> = combat
        .members
        .iter()
        .map(|m| {
            (
                m.name.as_str(),
                format!("{:?}", m.returns),
                m.description.as_str(),
            )
        })
        .collect();
    assert_eq!(
        members,
        [
            ("title", "String".to_string(), "Where the fight happens."),
            ("hit_table", "Array".to_string(), ""),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proc_signature_carries_its_docs_and_return_type() {
    let (h, _) = with_combat().await;
    let host = WeaverHost::builder()
        .declarations_only()
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();

    let damage = host
        .registry()
        .processor_signature("combat", "damage")
        .unwrap();

    assert_eq!(damage.description, "How hard a blow lands.");
    assert_eq!(format!("{:?}", damage.returns), "Number");
    assert_eq!(
        format!("{:?}", damage.properties[0].expected_type),
        "Some(Number)"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proc_returning_the_wrong_type_fails_at_the_call() {
    let (h, _) = with_combat().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmScript)
        .plugins(h.plugins.vocabulary(h.user).await)
        .build();

    let err = host
        .evaluate(compile_template("@[combat.liar()]").unwrap())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("expected number"), "{err}");
    let ok = host
        .evaluate(compile_template("@[combat.damage(2)]").unwrap())
        .await
        .unwrap();
    assert_eq!(ok.to_output_string(), "2");
}

/// Values evaluate under the same ceilings as every host.
#[tokio::test]
async fn an_unbounded_value_is_refused_on_save() {
    let h = Harness::new().await;
    let refused = modules(&h)
        .create(
            h.user,
            "big".into(),
            String::new(),
            "export big = [{# foreach i in @[array.range(start: 0, end: 20000)] #} i {# endforeach #}]"
                .into(),
        )
        .await;

    assert!(
        matches!(&refused, Err(ModuleError::Invalid(m)) if m.contains("iteration")),
        "{refused:?}"
    );
}
