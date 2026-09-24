use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use lorealm_storage::{
    PersonaStorage, PluginStorage,
    models::{PluginLog, PluginLogLevel, PluginLogSource, Snowflake},
};
use tokio::sync::OnceCell;

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

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../plugins/tests/fixtures")
        .join(name)
}

/// An empty plugins directory of its own.
fn fresh_dir() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lorealm-plugins-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A plugin service over the harness's storage, reading plugins from `dir`.
fn service(h: &Harness, dir: PathBuf, config: PluginConfig) -> Arc<PluginService> {
    let storage = Arc::clone(&h.store) as Arc<dyn lorealm_storage::Storage>;
    Arc::new(PluginService::new(
        Arc::clone(&storage),
        Arc::clone(&h.snow),
        Arc::new(StateService::new(storage)),
        Arc::clone(&h.connection),
        config,
        dir,
    ))
}

/// The fixtures, installed from source once per test run.
async fn installed_dir() -> PathBuf {
    static DIR: OnceCell<PathBuf> = OnceCell::const_new();
    DIR.get_or_init(|| async {
        let h = Harness::new().await;
        let dir = fresh_dir();
        let plugins = service(&h, dir.clone(), PluginConfig::default());
        // `hello` first: the others reuse its build output.
        for name in [
            "hello",
            "hello_addon",
            "hello_rival",
            "no_data",
            "unpermitted",
        ] {
            plugins
                .install_from_source(Snowflake(0), fixture(name))
                .await
                .unwrap_or_else(|e| panic!("installing {name}: {e}"));
        }
        dir
    })
    .await
    .clone()
}

/// A harness with the fixtures installed, whose user has an active persona
/// called Ada.
async fn harness() -> Harness {
    let mut h = Harness::new().await;
    h.plugins = service(&h, installed_dir().await, PluginConfig::default());
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

    let plugins = h.plugins.list(h.user, false).await.unwrap();
    let enabled: Vec<_> = plugins
        .iter()
        .map(|p| (p.info.handle.as_str(), p.enabled))
        .collect();

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
    h.store
        .enable_user_plugin(h.user, "missing", "0.1.0")
        .await
        .unwrap();

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
    let plugins = service(&h, installed_dir().await, PluginConfig { log_cap: 3 });
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
    h.store
        .enable_user_plugin(h.user, "hello", "0.1.0")
        .await
        .unwrap();
    h.store
        .enable_user_plugin(h.user, "hello_rival", "0.1.0")
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

    let stored: Vec<String> = h
        .store
        .list_user_plugins(h.user)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.handle)
        .collect();
    assert_eq!(stored, order);
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

#[tokio::test]
async fn a_declared_frontend_entry_is_served_to_users_who_enabled_it() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let html = h
        .plugins
        .frontend_entry(h.user, "hello", "inspector.html", false)
        .await
        .unwrap();
    let theirs = h
        .plugins
        .frontend_entry(h.stranger, "hello", "inspector.html", false)
        .await;
    let undeclared = h
        .plugins
        .frontend_entry(h.user, "hello", "../manifest.toml", false)
        .await;

    assert!(html.contains("lorealm.call('greet'"));
    assert!(matches!(theirs, Err(PluginError::NotFound(_))));
    assert!(matches!(undeclared, Err(PluginError::NotFound(_))));
}

#[tokio::test]
async fn the_frontend_reaches_its_own_backend() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let greeting = h
        .plugins
        .call_frontend(
            h.user,
            "hello",
            "greet".into(),
            serde_json::json!({"name": "Ada"}),
        )
        .await
        .unwrap();
    let unknown = h
        .plugins
        .call_frontend(h.user, "hello", "nope".into(), serde_json::json!({}))
        .await;
    let not_enabled = h
        .plugins
        .call_frontend(h.stranger, "hello", "greet".into(), serde_json::json!({}))
        .await;

    assert_eq!(greeting, serde_json::json!("Hello, Ada!"));
    assert!(matches!(unknown, Err(PluginError::CallFailed(_, _))));
    assert!(matches!(not_enabled, Err(PluginError::NotFound(_))));
}

#[tokio::test]
async fn the_plugin_list_carries_frontend_slots() {
    let h = harness().await;

    let plugins = h.plugins.list(h.user, false).await.unwrap();
    let hello = plugins.iter().find(|p| p.info.handle == "hello").unwrap();

    assert_eq!(hello.slots[0].slot, "inspector");
    assert_eq!(hello.slots[0].entry, "inspector.html");
}

/// The entries of `handle`'s slots in the user's plugin list.
async fn listed_entries(h: &Harness, handle: &str, allow_unsafe: bool) -> Vec<String> {
    let plugins = h.plugins.list(h.user, allow_unsafe).await.unwrap();
    let plugin = plugins.iter().find(|p| p.info.handle == handle).unwrap();
    plugin.slots.iter().map(|s| s.entry.clone()).collect()
}

#[tokio::test]
async fn unsafe_slots_need_a_user_allowed_them() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    let refused = h
        .plugins
        .frontend_entry(h.user, "hello", "panel.js", false)
        .await;
    let module = h
        .plugins
        .frontend_entry(h.user, "hello", "panel.js", true)
        .await
        .unwrap();

    assert!(
        !listed_entries(&h, "hello", false)
            .await
            .contains(&"panel.js".into())
    );
    assert!(
        listed_entries(&h, "hello", true)
            .await
            .contains(&"panel.js".into())
    );
    assert!(matches!(refused, Err(PluginError::NotFound(_))));
    assert!(module.contains("export function mount"));
}

#[tokio::test]
async fn unsafe_slots_need_the_plugins_permission() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    let refused = h
        .plugins
        .frontend_entry(h.user, "hello_addon", "panel.js", true)
        .await;

    assert!(listed_entries(&h, "hello_addon", true).await.is_empty());
    assert!(matches!(refused, Err(PluginError::NotFound(_))));
}

/// A source for `hello` at `version`, reusing the fixture's build output.
fn hello_source(version: &str) -> PathBuf {
    let dir = fresh_dir();
    let wasm = fixture("hello/target/wasm32-wasip2/release/hello.wasm");
    let manifest = format!(
        r#"
        [plugin]
        name = "Hello"
        handle = "hello"
        description = ""
        author = "Lorealm"
        version = "{version}"
        permissions = ["prompt:write", "chat:read", "chat:write", "tools:write", "persona:read", "data:persist"]

        [structure]
        backend = "{}"
        "#,
        wasm.display()
    );
    std::fs::write(dir.join("manifest.toml"), manifest).unwrap();
    dir
}

#[tokio::test]
async fn installing_builds_and_stages_the_plugin() {
    let h = harness().await;
    let dir = fresh_dir();
    let plugins = service(&h, dir.clone(), PluginConfig::default());

    let report = plugins
        .install_from_source(h.user, fixture("hello"))
        .await
        .unwrap();
    let again = plugins.install_from_source(h.user, fixture("hello")).await;

    let installed = dir.join("hello@0.1.0");
    assert_eq!(
        (report.handle.as_str(), report.version.as_str()),
        ("hello", "0.1.0")
    );
    assert!(report.log.contains("$ cargo build"), "{}", report.log);
    assert!(installed.join("backend.wasm").is_file());
    assert!(installed.join("frontend/inspector.html").is_file());
    assert!(!installed.join("src").exists(), "sources aren't installed");
    assert!(matches!(again, Err(PluginError::AlreadyInstalled(_))));
}

#[tokio::test]
async fn a_failed_build_is_reported_and_logged() {
    let h = harness().await;
    let plugins = service(&h, fresh_dir(), PluginConfig::default());
    let source = fresh_dir();
    std::fs::write(
        source.join("manifest.toml"),
        r#"
        [plugin]
        name = "Broken"
        handle = "broken"
        description = ""
        author = "Lorealm"
        version = "0.1.0"

        [build]
        backend = ["sh", "-c", "echo compiling; exit 3"]
        "#,
    )
    .unwrap();

    let result = plugins.install_from_source(h.user, source).await;
    let logs = plugins.logs(h.user, "broken").await.unwrap();

    assert!(matches!(result, Err(PluginError::BuildFailed(_))));
    assert!(logs[0].message.starts_with("Build failed"), "{logs:#?}");
    assert!(logs[0].message.contains("compiling"), "{logs:#?}");
}

#[tokio::test]
async fn users_stay_on_their_version_until_they_update() {
    let h = harness().await;
    let dir = fresh_dir();
    let plugins = service(&h, dir.clone(), PluginConfig::default());
    plugins
        .install_from_source(h.user, hello_source("0.1.0"))
        .await
        .unwrap();
    plugins.enable(h.user, "hello").await.unwrap();

    plugins
        .install_from_source(h.user, hello_source("0.2.0"))
        .await
        .unwrap();
    plugins.enable(h.stranger, "hello").await.unwrap();
    let mine = plugins.list(h.user, false).await.unwrap();

    assert_eq!(mine[0].info.version.to_string(), "0.1.0");
    assert_eq!(mine[0].latest.as_ref().unwrap().to_string(), "0.2.0");
    assert_eq!(
        h.store.list_user_plugins(h.stranger).await.unwrap()[0].version,
        "0.2.0"
    );

    plugins.update(h.user, "hello", None).await.unwrap();

    assert_eq!(
        h.store.list_user_plugins(h.user).await.unwrap()[0].version,
        "0.2.0"
    );
    assert!(!dir.join("hello@0.1.0").exists(), "no one is on it anymore");
}

#[tokio::test]
async fn an_update_that_breaks_a_dependent_is_refused() {
    let h = harness().await;
    let plugins = service(&h, fresh_dir(), PluginConfig::default());
    plugins
        .install_from_source(h.user, fixture("hello"))
        .await
        .unwrap();
    plugins
        .install_from_source(h.user, fixture("hello_addon"))
        .await
        .unwrap();
    plugins.enable(h.user, "hello").await.unwrap();
    plugins.enable(h.user, "hello_addon").await.unwrap();
    plugins
        .install_from_source(h.user, hello_source("0.2.0"))
        .await
        .unwrap();

    let result = plugins.update(h.user, "hello", None).await;

    assert!(
        matches!(&result, Err(PluginError::CannotUpdate(_, reason)) if reason.starts_with("`hello_addon` requires `hello` ^0.1")),
        "{result:?}"
    );
}

#[tokio::test]
async fn uninstalling_keeps_versions_users_are_on() {
    let h = harness().await;
    let dir = fresh_dir();
    let plugins = service(&h, dir.clone(), PluginConfig::default());
    plugins
        .install_from_source(h.user, hello_source("0.1.0"))
        .await
        .unwrap();
    plugins.enable(h.user, "hello").await.unwrap();

    plugins.uninstall("hello").await.unwrap();
    let kept = dir.join("hello@0.1.0").exists();
    let theirs = plugins.enable(h.stranger, "hello").await;
    let out = plugins.prompt_build(h.user, prompt()).await;
    plugins.disable(h.user, "hello").await.unwrap();

    assert!(kept);
    assert!(matches!(theirs, Err(PluginError::NotFound(_))));
    assert_eq!(out.len(), 2, "still runs for the user on it");
    assert!(!dir.join("hello@0.1.0").exists());
}

#[tokio::test]
async fn an_installed_plugin_serves_its_readme() {
    let h = harness().await;

    let readme = h.plugins.readme(h.user, "hello").await.unwrap();
    let none = h.plugins.readme(h.user, "hello_addon").await;

    assert!(readme.starts_with("# Hello"), "{readme}");
    assert!(matches!(none, Err(PluginError::NotFound(_))));
}

#[tokio::test]
async fn settings_are_checked_and_saved() {
    let h = harness().await;
    let values = |v: serde_json::Value| v.as_object().unwrap().clone();
    let save = |v: serde_json::Value| h.plugins.save_settings(h.user, "hello", values(v));

    // Key types come from enabled plugins.
    let early = h
        .connection
        .create_secret(h.user, "elevenlabs".into(), "Mine".into(), "sk".into(), &[])
        .await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    let key_types = h.plugins.key_types(h.user).await.unwrap();
    let key = h
        .connection
        .create_secret(
            h.user,
            "elevenlabs".into(),
            "Mine".into(),
            "sk".into(),
            &key_types,
        )
        .await
        .unwrap();
    let theirs = h
        .connection
        .create_secret(
            h.stranger,
            "elevenlabs".into(),
            "Theirs".into(),
            "sk".into(),
            &key_types,
        )
        .await
        .unwrap();

    let blank = h.plugins.settings(h.user, "hello").await.unwrap();
    let wrong_type = save(serde_json::json!({ "greeting": 1, "key": key.id })).await;
    let wrong_key = save(serde_json::json!({ "greeting": "hi", "key": theirs.id })).await;
    save(serde_json::json!({ "greeting": "hi", "key": key.id }))
        .await
        .unwrap();
    let saved = h.plugins.settings(h.user, "hello").await.unwrap();
    let none = h.plugins.settings(h.user, "hello_addon").await.unwrap();

    assert!(early.is_err());
    assert_eq!(key_types, ["elevenlabs"]);
    assert_eq!(blank.inputs.len(), 3);
    assert!(matches!(
        wrong_type,
        Err(PluginError::InvalidSettings(_, _))
    ));
    assert!(
        matches!(&wrong_key, Err(PluginError::InvalidSettings(_, r)) if r.contains("pick one of your elevenlabs keys")),
        "{wrong_key:?}"
    );
    assert_eq!(
        serde_json::Value::Object(saved.values),
        serde_json::json!({ "greeting": "hi", "excited": false, "key": key.id })
    );
    assert!(none.inputs.is_empty());
}

#[tokio::test]
async fn a_plugin_reads_its_settings() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    let key_types = h.plugins.key_types(h.user).await.unwrap();
    let key = h
        .connection
        .create_secret(
            h.user,
            "elevenlabs".into(),
            "Mine".into(),
            "sk".into(),
            &key_types,
        )
        .await
        .unwrap();
    let read = || {
        h.plugins
            .call_frontend(h.user, "hello", "settings".into(), serde_json::json!({}))
    };

    let unset = read().await;
    let logs = h.plugins.logs(h.user, "hello").await.unwrap();
    h.plugins
        .save_settings(
            h.user,
            "hello",
            serde_json::json!({ "greeting": "hi", "key": key.id })
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    let set = read().await.unwrap();

    assert!(matches!(unset, Err(PluginError::CallFailed(_, _))));
    assert!(
        logged(
            &logs,
            PluginLogSource::Host,
            PluginLogLevel::Warn,
            "Couldn't read your settings: missing required input: greeting"
        ),
        "{logs:#?}"
    );
    assert_eq!(
        set,
        serde_json::json!({ "greeting": "hi", "excited": false, "key": key.id })
    );
}

#[tokio::test]
async fn a_plugin_fetches_with_the_users_key() {
    // Answers with the header named in the path.
    let echo = axum::Router::new().route(
        "/{header}",
        axum::routing::get(
            |axum::extract::Path(header): axum::extract::Path<String>,
             headers: axum::http::HeaderMap| async move {
                headers
                    .get(header)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_owned()
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:38417")
        .await
        .unwrap();
    tokio::spawn(async move { axum::serve(listener, echo).await.unwrap() });

    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    let key_types = h.plugins.key_types(h.user).await.unwrap();
    let key = h
        .connection
        .create_secret(
            h.user,
            "elevenlabs".into(),
            "Mine".into(),
            "sk-test".into(),
            &key_types,
        )
        .await
        .unwrap();
    h.plugins
        .save_settings(
            h.user,
            "hello",
            serde_json::json!({ "greeting": "hi", "key": key.id })
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap();
    let fetch = |authorization: &str| {
        let args = serde_json::json!({
            "url": "http://127.0.0.1:38417/authorization",
            "headers": { "authorization": authorization },
        });
        h.plugins
            .call_frontend(h.user, "hello", "fetch".into(), args)
    };

    let with_key = fetch("Bearer {secret:key}").await.unwrap();
    let not_a_key = fetch("Bearer {secret:greeting}").await;
    let plain = fetch("Bearer nothing").await.unwrap();
    let agent = h
        .plugins
        .call_frontend(
            h.user,
            "hello",
            "fetch".into(),
            serde_json::json!({ "url": "http://127.0.0.1:38417/user-agent" }),
        )
        .await
        .unwrap();

    assert_eq!(with_key, serde_json::json!("Bearer sk-test"));
    assert!(matches!(not_a_key, Err(PluginError::CallFailed(_, _))));
    assert_eq!(plain, serde_json::json!("Bearer nothing"));
    assert_eq!(
        agent,
        serde_json::json!(format!(
            "Lorealm/{} (plugin hello@0.1.0)",
            env!("CARGO_PKG_VERSION")
        ))
    );
}

/// The example in `plugins/tts`, short of calling a provider.
#[tokio::test]
async fn the_tts_example_installs_and_asks_for_a_key() {
    let h = Harness::new().await;
    let plugins = service(&h, fresh_dir(), PluginConfig::default());
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/tts");
    plugins
        .install_from_source(h.user, source)
        .await
        .unwrap_or_else(|e| panic!("installing: {e}"));
    plugins.enable(h.user, "tts").await.unwrap();
    let call = |name: &str, args: serde_json::Value| {
        plugins.call_frontend(h.user, "tts", name.into(), args)
    };

    let speak = call("speak", serde_json::json!({ "text": "*Hello* there." }))
        .await
        .unwrap();
    let first = call("autoplay", serde_json::json!({ "id": "1" }))
        .await
        .unwrap();
    let again = call("autoplay", serde_json::json!({ "id": "1" }))
        .await
        .unwrap();

    assert_eq!(
        speak,
        serde_json::json!({ "error": "Pick your OpenRouter key under Settings → You → Plugins." })
    );
    assert_eq!(first, serde_json::json!(true));
    assert_eq!(again, serde_json::json!(false));
    assert_eq!(
        plugins.key_types(h.user).await.unwrap(),
        ["elevenlabs", "openrouter"]
    );
}

/// Has `hello_addon` emit `hello_addon.ping` with `payload`.
async fn ping(h: &Harness, payload: serde_json::Value) {
    h.plugins
        .call_frontend(
            h.user,
            "hello_addon",
            "emit".into(),
            serde_json::json!({"topic": "hello_addon.ping", "payload": payload}),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn an_emitted_event_reaches_other_plugins() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    ping(&h, serde_json::json!("hi")).await;
    let received = h
        .plugins
        .call_frontend(h.user, "hello", "event".into(), serde_json::json!({}))
        .await
        .unwrap();

    assert_eq!(received, serde_json::json!(r#"hello_addon.ping "hi""#));
}

#[tokio::test]
async fn a_chain_of_events_stops() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    ping(&h, serde_json::json!("echo")).await;
    let logs = h.plugins.logs(h.user, "hello_addon").await.unwrap();

    use PluginLogLevel::Warn;
    use PluginLogSource::Host;
    assert!(
        logged(&logs, Host, Warn, "a chain of events stops"),
        "{logs:#?}"
    );
}

#[tokio::test]
async fn after_generate_reaches_enabled_plugins() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();

    h.plugins
        .after_generate(h.user, Snowflake(1), Snowflake(2), "Hi there".into())
        .await;
    let observed = h
        .plugins
        .call_frontend(h.user, "hello", "event".into(), serde_json::json!({}))
        .await
        .unwrap();

    assert_eq!(observed, serde_json::json!("after-generate Hi there"));
}

#[tokio::test]
async fn message_finalize_runs_in_load_order() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    let content = h
        .plugins
        .message_finalize(h.user, Snowflake(1), "Hi there".into())
        .await;

    assert_eq!(content, "Hi there!!");
}

#[tokio::test]
async fn user_message_runs_in_load_order_until_refused() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();

    let changed = h
        .plugins
        .user_message(h.user, Snowflake(1), "Hi".into())
        .await;
    let denied = h
        .plugins
        .user_message(h.user, Snowflake(1), "deny me".into())
        .await;

    assert_eq!(changed.unwrap(), "Hi??");
    let refusal = denied.unwrap_err();
    assert_eq!(refusal.handle, "hello");
    assert_eq!(refusal.reason, "denied on purpose");
}

#[tokio::test]
async fn lorebook_activate_runs_in_load_order() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();
    let entry = |content: &str| lorealm_plugins::LorebookEntry {
        id: content.into(),
        content: content.into(),
        priority: 0,
    };

    let vocabulary = h.plugins.vocabulary(h.user).await.unwrap();
    let entries = vocabulary
        .lorebook_activate("setting", vec![entry("dragons"), entry("drop me")])
        .await;

    let contents: Vec<_> = entries.iter().map(|e| e.content.as_str()).collect();
    assert_eq!(contents, ["dragons!!"]);
}

#[tokio::test]
async fn tool_call_runs_in_load_order_until_refused() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();
    let arguments = serde_json::json!({ "city": "Oslo" });

    let changed = h
        .plugins
        .tool_call(h.user, "weather", arguments.clone())
        .await;
    let denied = h.plugins.tool_call(h.user, "forbidden", arguments).await;

    assert_eq!(
        changed.unwrap(),
        serde_json::json!({ "city": "Oslo", "seen": 2 })
    );
    let refusal = denied.unwrap_err();
    assert_eq!(refusal.handle, "hello");
    assert_eq!(refusal.reason, "denied on purpose");
}

#[tokio::test]
async fn tool_result_runs_in_load_order() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    h.plugins.enable(h.user, "hello_addon").await.unwrap();
    let arguments = serde_json::json!({ "city": "Oslo" });

    let value = h
        .plugins
        .tool_result(
            h.user,
            "weather",
            &arguments,
            Ok(serde_json::json!({ "temp": 3 })),
        )
        .await;
    let error = h
        .plugins
        .tool_result(h.user, "weather", &arguments, Err("boom".into()))
        .await;

    assert_eq!(value, Ok(serde_json::json!({ "temp": 3, "checked": 2 })));
    assert_eq!(error, Err("boom!!".into()));
}

#[tokio::test]
async fn turn_hooks_reach_enabled_plugins() {
    let h = harness().await;
    h.plugins.enable(h.user, "hello").await.unwrap();
    let last_event = async || {
        h.plugins
            .call_frontend(h.user, "hello", "event".into(), serde_json::json!({}))
            .await
            .unwrap()
    };

    h.plugins
        .turn_start(h.user, Snowflake(1), "narrator".into())
        .await;
    let started = last_event().await;
    h.plugins
        .turn_end(
            h.user,
            Snowflake(1),
            "narrator".into(),
            Snowflake(2),
            "Night falls.".into(),
        )
        .await;
    let ended = last_event().await;

    assert_eq!(started, serde_json::json!("turn-start narrator"));
    assert_eq!(ended, serde_json::json!("turn-end narrator Night falls."));
}
