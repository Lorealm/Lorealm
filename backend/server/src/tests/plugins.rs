use std::{process::Command, sync::Arc};

use lorealm_storage::{
    PersonaStorage, PluginStorage,
    models::{PluginLog, PluginLogLevel, PluginLogSource},
};

use crate::{
    config::PluginConfig,
    errors::PluginError,
    services::{
        chat::PromptMessage,
        context::PromptRole,
        plugins::{PluginService, PluginVocabulary},
        state::StateService,
        weaver::{WeaverHost, WeaverProfile, compile_template},
    },
    tests::Harness,
};

/// Builds the hello component the plugin fixtures share.
fn build_hello() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugins/tests/fixtures/hello");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(dir)
        .status()
        .expect("failed to run cargo");
    assert!(status.success(), "failed to build the hello fixture");
}

/// A harness whose user has an active persona called Ada.
async fn harness() -> Harness {
    build_hello();
    let h = Harness::new().await;
    let persona = h.persona(h.user, "Ada").await;
    h.store.set_active_persona(h.user, persona).await.unwrap();
    h
}

fn prompt() -> Vec<PromptMessage> {
    ask("Hi")
}

fn ask(content: &str) -> Vec<PromptMessage> {
    vec![PromptMessage {
        role: PromptRole::User,
        content: content.into(),
    }]
}

/// Whether an entry from `source` at `level` contains `text`.
fn logged(logs: &[PluginLog], source: PluginLogSource, level: PluginLogLevel, text: &str) -> bool {
    logs.iter()
        .any(|l| l.source == source && l.level == level && l.message.contains(text))
}

#[tokio::test]
async fn enabled_plugin_edits_the_prompt() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let out = h.plugins.prompt_build(h.user, prompt()).await;

    assert_eq!(out.len(), 2);
    assert_eq!(out[1].role, PromptRole::System);
    assert_eq!(out[1].content, "Hello, Ada! #1");
}

#[tokio::test]
async fn disabling_unmounts_the_plugin() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.prompt_build(h.user, prompt()).await;

    h.plugins.disable(h.user, "hello").await.unwrap();
    let out = h.plugins.prompt_build(h.user, prompt()).await;

    assert_eq!(out.len(), 1);
}

#[tokio::test]
async fn plugins_are_per_user() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let out = h.plugins.prompt_build(h.stranger, prompt()).await;

    assert_eq!(out.len(), 1);
}

#[tokio::test]
async fn plugin_that_fails_init_is_skipped() {
    let h = harness().await;
    h.plugins.enable(h.user, "unpermitted").await.unwrap();

    let out = h.plugins.prompt_build(h.user, prompt()).await;

    assert_eq!(out.len(), 1);
}

#[tokio::test]
async fn only_installed_plugins_can_be_enabled() {
    let h = harness().await;

    for handle in ["missing", "../fixtures/hello", ""] {
        let result = h.plugins.enable(h.user, handle).await;
        assert!(
            matches!(result, Err(PluginError::NotFound(_))),
            "{handle:?}"
        );
    }
}

#[tokio::test]
async fn list_marks_enabled_plugins() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let plugins = h.plugins.list(h.user).await.unwrap();
    let enabled: Vec<_> = plugins
        .iter()
        .map(|p| (p.info.handle.as_str(), p.enabled))
        .collect();

    // Other fixtures may or may not be built, so only check these two.
    assert!(enabled.contains(&("hello", true)), "{enabled:?}");
    assert!(enabled.contains(&("unpermitted", false)), "{enabled:?}");
}

#[tokio::test]
async fn persisted_data_survives_remounting() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.prompt_build(h.user, prompt()).await;

    h.plugins.disable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello").await.unwrap();
    let out = h.plugins.prompt_build(h.user, prompt()).await;

    assert_eq!(out[1].content, "Hello, Ada! #2");
}

#[tokio::test]
async fn persisted_data_is_per_user() {
    let h = harness().await;
    let persona = h.persona(h.stranger, "Bo").await;
    h.store
        .set_active_persona(h.stranger, persona)
        .await
        .unwrap();
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.stranger, "hello").await.unwrap();
    h.plugins.prompt_build(h.user, prompt()).await;

    let out = h.plugins.prompt_build(h.stranger, prompt()).await;

    assert_eq!(out[1].content, "Hello, Bo! #1");
}

#[tokio::test]
async fn plugin_logs_are_stored_per_user() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    h.plugins.prompt_build(h.user, prompt()).await;
    let logs = h.plugins.logs(h.user, "hello").await.unwrap();
    let theirs = h.plugins.logs(h.stranger, "hello").await.unwrap();

    use PluginLogLevel::Info;
    use PluginLogSource::Guest;
    assert!(logged(&logs, Guest, Info, "greeting #1"), "{logs:#?}");
    assert!(
        logged(&logs, Guest, Info, "printed greeting #1"),
        "{logs:#?}"
    );
    assert!(theirs.is_empty());
}

#[tokio::test]
async fn crashing_plugin_is_logged_and_stopped() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    h.plugins.prompt_build(h.user, ask("crash")).await;
    let logs = h.plugins.logs(h.user, "hello").await.unwrap();

    use PluginLogSource::{Guest, Host};
    assert!(logged(
        &logs,
        Guest,
        PluginLogLevel::Warn,
        "crashed on purpose"
    ));
    assert!(logged(&logs, Host, PluginLogLevel::Error, ""));
    assert!(logs[0].message.starts_with("Plugin stopped"), "{logs:#?}");
}

#[tokio::test]
async fn enabled_plugin_that_is_not_installed_is_logged() {
    let h = harness().await;
    h.store.enable_user_plugin(h.user, "missing").await.unwrap();

    h.plugins.prompt_build(h.user, prompt()).await;
    let logs = h.plugins.logs(h.user, "missing").await.unwrap();

    assert!(
        logged(
            &logs,
            PluginLogSource::Host,
            PluginLogLevel::Error,
            "not installed"
        ),
        "{logs:#?}"
    );
}

#[tokio::test]
async fn log_keeps_only_the_newest_entries() {
    let h = harness().await;
    let storage = Arc::clone(&h.store) as Arc<dyn lorealm_storage::Storage>;
    let plugins = PluginService::new(
        Arc::clone(&storage),
        Arc::clone(&h.snow),
        Arc::new(StateService::new(storage)),
        PluginConfig { log_cap: 3 },
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugins/tests/fixtures"),
    );
    plugins.enable(h.user, "hello").await.unwrap();

    for _ in 0..3 {
        plugins.prompt_build(h.user, prompt()).await;
    }
    let logs = plugins.logs(h.user, "hello").await.unwrap();
    let messages: Vec<_> = logs.iter().map(|l| l.message.as_str()).collect();

    assert_eq!(
        messages,
        ["printed greeting #3", "greeting #3", "printed greeting #2"]
    );
}

#[tokio::test]
async fn a_plugin_cannot_be_enabled_before_its_requirement() {
    let h = harness().await;

    let result = h.plugins.enable(h.user, "hello_addon").await;

    assert!(
        matches!(&result, Err(PluginError::CannotEnable(_, reason)) if reason.contains("requires `hello`")),
        "{result:?}"
    );
}

#[tokio::test]
async fn a_plugin_loads_after_its_requirement() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    let out = h.plugins.prompt_build(h.user, prompt()).await;

    assert_eq!(out.len(), 3);
}

#[tokio::test]
async fn a_required_plugin_cannot_be_disabled() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    let result = h.plugins.disable(h.user, "hello").await;

    assert!(
        matches!(&result, Err(PluginError::Required(_, by)) if by == &["hello_addon"]),
        "{result:?}"
    );
}

#[tokio::test]
async fn a_conflicting_plugin_cannot_be_enabled() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let result = h.plugins.enable(h.user, "hello_rival").await;

    assert!(
        matches!(&result, Err(PluginError::CannotEnable(_, reason)) if reason == "conflicts with `hello`"),
        "{result:?}"
    );
}

#[tokio::test]
async fn a_conflict_that_got_past_enabling_is_skipped_and_logged() {
    let h = harness().await;
    h.store.enable_user_plugin(h.user, "hello").await.unwrap();
    h.store
        .enable_user_plugin(h.user, "hello_rival")
        .await
        .unwrap();

    let out = h.plugins.prompt_build(h.user, prompt()).await;
    let logs = h.plugins.logs(h.user, "hello_rival").await.unwrap();

    assert_eq!(out.len(), 2);
    assert!(
        logged(
            &logs,
            PluginLogSource::Host,
            PluginLogLevel::Error,
            "Not loaded: it conflicts with `hello`."
        ),
        "{logs:#?}"
    );
}

#[tokio::test]
async fn load_order_must_list_exactly_the_enabled_plugins() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "unpermitted").await.unwrap();

    for order in [
        vec!["hello"],
        vec!["hello", "unpermitted", "no_data"],
        vec!["hello", "hello"],
    ] {
        let order = order.into_iter().map(String::from).collect();
        let result = h.plugins.set_order(h.user, order).await;
        assert!(
            matches!(result, Err(PluginError::InvalidOrder)),
            "{result:?}"
        );
    }

    let order = vec!["unpermitted".to_owned(), "hello".to_owned()];
    h.plugins.set_order(h.user, order.clone()).await.unwrap();

    assert_eq!(h.store.list_user_plugins(h.user).await.unwrap(), order);
}

async fn with_hello() -> (Harness, PluginVocabulary) {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    let plugins = h.plugins.vocabulary(h.user).await.unwrap();
    (h, plugins)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_processor_runs_in_a_template() {
    let (_h, plugins) = with_hello().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::ChatTurn)
        .plugins(Some(plugins))
        .build();

    let body = compile_template(r#"@[hello.shout(text: "hi")]"#).unwrap();
    let outcome = host.evaluate(body).await.unwrap();

    assert_eq!(outcome.to_output_string(), "HI!");
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_callables_run_in_inline_hosts() {
    let (_h, plugins) = with_hello().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmCondition)
        .plugins(Some(plugins))
        .build();

    let body = compile_template(r#"@[hello.shout(text: "hi")]"#).unwrap();
    let outcome = host.evaluate_inline(body).unwrap();

    assert_eq!(outcome.to_output_string(), "HI!");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_command_runs_under_its_handle() {
    let (_h, plugins) = with_hello().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::ChatTurn)
        .plugins(Some(plugins))
        .build();

    let body = compile_template(r#"$[hello.greet("Ada")]"#).unwrap();
    let outcome = host.evaluate(body).await.unwrap();

    let signature = host.registry().command_signature("hello.greet").unwrap();
    assert_eq!(signature.params[0].name, "name");
    assert_eq!(outcome.to_output_string(), "Hello, Ada!");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_plugin_command_is_an_evaluation_error() {
    let (_h, plugins) = with_hello().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::Command)
        .plugins(Some(plugins))
        .build();

    let body = compile_template("$[hello.greet(42)]").unwrap();
    let err = host.evaluate(body).await.unwrap_err();

    assert!(err.to_string().contains("expected a string"), "{err}");
}

#[tokio::test]
async fn plugin_sources_are_listed_under_their_handle() {
    let (_h, plugins) = with_hello().await;

    let sources: Vec<_> = plugins.sources().map(|s| s.definition()).collect();

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source_id, "plugin:hello");
    assert_eq!(sources[0].name, "Hello");
    assert_eq!(sources[0].nodes[0].node_id, "greeting");
}

#[tokio::test]
async fn a_user_without_plugins_has_no_vocabulary() {
    let h = harness().await;

    assert!(h.plugins.vocabulary(h.user).await.is_none());
}

/// A pool with `tally:n` set to 41.
fn tally_pool() -> Arc<std::sync::Mutex<crate::services::state::StatePool>> {
    let mut pool = crate::services::state::StatePool::new();
    pool.entry("tally".into())
        .or_default()
        .insert("n".into(), serde_json::json!(41));
    Arc::new(std::sync::Mutex::new(pool))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_command_reads_and_writes_state() {
    let (_h, plugins) = with_hello().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::ChatTurn)
        .pool(tally_pool())
        .plugins(Some(plugins))
        .build();

    let body = compile_template(r#"$[hello.tally("n")]"#).unwrap();
    let outcome = host.evaluate(body).await.unwrap();

    let signature = host.registry().command_signature("hello.tally").unwrap();
    assert!(signature.mutates_context);
    assert_eq!(outcome.to_output_string(), "42");
    assert_eq!(outcome.pool["tally"]["n"], serde_json::json!(42));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_read_only_host_refuses_a_plugin_write() {
    let (_h, plugins) = with_hello().await;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmCondition)
        .pool(tally_pool())
        .plugins(Some(plugins))
        .build();

    let body = compile_template(r#"$[hello.tally("n")]"#).unwrap();
    let err = host.evaluate_inline(body).unwrap_err();

    assert!(err.to_string().contains("read-only"), "{err}");
}
