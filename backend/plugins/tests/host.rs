use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use lorealm_plugins::{
    CommandContext, Finalizing, Generated, HostServices, HttpRequest, HttpResponse, Level,
    LogEntry, LogSource, LorebookActivation, LorebookEntry, Message, Persona, PluginError,
    PluginHost, PluginPackage, Role, ToolInvocation, UserInput, Verdict,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Builds the hello component with the wasm32-wasip2 target.
fn build_hello() {
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(fixture("hello"))
        .status()
        .expect("failed to run cargo");
    assert!(status.success(), "failed to build the hello fixture");
}

fn host() -> PluginHost {
    PluginHost::new(Duration::from_secs(5)).unwrap()
}

#[derive(Default)]
struct TestServices {
    data: Mutex<HashMap<String, Vec<u8>>>,
    logs: Mutex<Vec<LogEntry>>,
    /// How long `http_fetch` takes.
    latency: Duration,
}

impl TestServices {
    /// Whether an entry from `source` at `level` contains `text`.
    fn logged(&self, level: Level, source: LogSource, text: &str) -> bool {
        self.logs
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.level == level && e.source == source && e.message.contains(text))
    }
}

#[async_trait::async_trait]
impl HostServices for TestServices {
    async fn active_persona(&self) -> Result<Persona, String> {
        Ok(Persona {
            name: "Ada".into(),
            description: String::new(),
        })
    }

    async fn persist_data(&self, key: String, value: Vec<u8>) -> Result<(), String> {
        self.data.lock().unwrap().insert(key, value);
        Ok(())
    }

    async fn load_data(&self, key: String) -> Result<Option<Vec<u8>>, String> {
        Ok(self.data.lock().unwrap().get(&key).cloned())
    }

    async fn delete_data(&self, key: String) -> Result<(), String> {
        self.data.lock().unwrap().remove(&key);
        Ok(())
    }

    async fn settings(&self) -> Result<String, String> {
        Ok(r#"{"greeting":"Hi"}"#.into())
    }

    /// Answers with the URL it was sent to.
    async fn http_fetch(&self, request: HttpRequest) -> Result<HttpResponse, String> {
        tokio::time::sleep(self.latency).await;
        Ok(HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: request.url.into_bytes(),
        })
    }

    async fn log(&self, entries: Vec<LogEntry>) {
        self.logs.lock().unwrap().extend(entries);
    }
}

/// Variables keyed by (scope, name), as JSON.
#[derive(Default)]
struct TestContext(HashMap<(String, String), String>);

impl CommandContext for TestContext {
    fn get(&self, scope: &str, name: &str) -> Result<Option<String>, String> {
        Ok(self.0.get(&(scope.into(), name.into())).cloned())
    }

    fn set(&mut self, scope: &str, name: &str, value: String) -> Result<(), String> {
        self.0.insert((scope.into(), name.into()), value);
        Ok(())
    }
}

fn ask(content: &str) -> Vec<Message> {
    vec![Message {
        role: Role::User,
        content: content.into(),
    }]
}

/// Builds the Python hello component with componentize-py, once per test run:
/// unlike cargo, it takes no lock on its output.
fn build_hello_py() {
    static BUILD: Once = Once::new();
    BUILD.call_once(|| {
        let dir = fixture("hello_py");
        std::fs::create_dir_all(dir.join("target")).unwrap();
        let status = Command::new("uvx")
            .args([
                "componentize-py@0.25.1",
                "-d",
                "../../../wit",
                "-w",
                "plugin",
            ])
            .args(["componentize", "app", "-o", "target/hello.wasm"])
            .current_dir(dir)
            .status()
            .expect("failed to run uvx; is uv installed?");
        assert!(status.success(), "failed to build the hello_py fixture");
    });
}

/// The same plugin in each guest language.
const HELLOS: [&str; 2] = ["hello", "hello_py"];

fn compile(host: &PluginHost, name: &str) -> PluginPackage {
    match name {
        "hello_py" => build_hello_py(),
        _ => build_hello(),
    }
    host.compile(&fixture(name)).unwrap()
}

#[tokio::test]
async fn prompt_build_hook_modifies_prompt() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();

        let prompt = plugin.prompt_build(ask("Hi")).await.unwrap();

        assert_eq!(prompt.len(), 2, "{name}");
        assert_eq!(prompt[1].content, "Hello, Ada! #1", "{name}");
    }
}

#[tokio::test]
async fn registering_hook_without_permission_fails_init() {
    let host = host();
    let package = compile(&host, "unpermitted");

    let result = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await;

    assert!(
        matches!(&result, Err(PluginError::Init(msg)) if msg.contains("prompt:write")),
        "{:?}",
        result.err()
    );
}

#[tokio::test]
async fn missing_manifest_is_an_error() {
    let host = host();

    let result = host.compile(&fixture("does-not-exist"));

    assert!(matches!(result, Err(PluginError::Manifest(_))));
}

#[tokio::test]
async fn runaway_hook_times_out() {
    let host = PluginHost::new(Duration::from_millis(100)).unwrap();
    for name in HELLOS {
        let package = compile(&host, name);
        let services = Arc::new(TestServices::default());
        let mut plugin = host.instantiate(&package, services.clone()).await.unwrap();

        let result = plugin.prompt_build(ask("loop forever")).await;
        assert!(
            matches!(result, Err(PluginError::Timeout)),
            "{name}: {:?}",
            result.err()
        );
        assert!(
            services.logged(Level::Error, LogSource::Host, "timed out"),
            "{name}"
        );

        // A trapped instance can't be entered again.
        let result = plugin.prompt_build(Vec::new()).await;
        assert!(
            matches!(result, Err(PluginError::Wasm(_))),
            "{name}: {:?}",
            result.err()
        );
    }
}

#[tokio::test]
async fn persisted_data_survives_between_calls() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();

        plugin.prompt_build(ask("Hi")).await.unwrap();
        let second = plugin.prompt_build(ask("Hi")).await.unwrap();
        let forgotten = plugin.prompt_build(ask("forget")).await.unwrap();

        assert_eq!(second[1].content, "Hello, Ada! #2", "{name}");
        assert_eq!(forgotten[1].content, "Hello, Ada! #1", "{name}");
    }
}

#[tokio::test]
async fn persisting_without_permission_fails_the_hook() {
    let host = host();
    let package = compile(&host, "no_data");
    let services = Arc::new(TestServices::default());
    let mut plugin = host.instantiate(&package, services.clone()).await.unwrap();

    let result = plugin.prompt_build(ask("Hi")).await;

    assert!(
        matches!(&result, Err(PluginError::Call(msg)) if msg.contains("data:persist")),
        "{:?}",
        result.err()
    );
    assert!(services.logged(
        Level::Warn,
        LogSource::Host,
        "missing permission `data:persist`"
    ));
    assert!(services.logged(Level::Error, LogSource::Host, "call failed"));
}

#[tokio::test]
async fn guest_logs_and_output_reach_the_host() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let services = Arc::new(TestServices::default());
        let mut plugin = host.instantiate(&package, services.clone()).await.unwrap();

        plugin.prompt_build(ask("Hi")).await.unwrap();

        assert!(
            services.logged(Level::Info, LogSource::Guest, "greeting #1"),
            "{name}: log import"
        );
        assert!(
            services.logged(Level::Info, LogSource::Guest, "printed greeting #1"),
            "{name}: stdout"
        );
    }
}

#[tokio::test]
async fn crash_output_is_logged() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let services = Arc::new(TestServices::default());
        let mut plugin = host.instantiate(&package, services.clone()).await.unwrap();

        let result = plugin.prompt_build(ask("crash")).await;

        assert!(result.is_err(), "{name}");
        assert!(
            services.logged(Level::Warn, LogSource::Guest, "crashed on purpose"),
            "{name}: {:#?}",
            services.logs.lock().unwrap()
        );
        assert!(services.logged(Level::Error, LogSource::Host, ""), "{name}");
    }
}

#[tokio::test]
async fn registered_callables_are_listed_and_callable() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();

        let registered = plugin.registrations();
        assert_eq!(registered.commands[0].name, "greet", "{name}");
        assert_eq!(registered.processors[0].name, "shout", "{name}");
        assert_eq!(registered.source_nodes[0].id, "greeting", "{name}");

        let greeting = plugin
            .call_command(
                "greet".into(),
                vec![r#""Ada""#.into()],
                &mut TestContext::default(),
            )
            .await
            .unwrap();
        let shout = plugin
            .call_processor("shout".into(), vec![("text".into(), r#""hi""#.into())])
            .await
            .unwrap();
        let blocks = plugin
            .resolve_source("greeting".into(), r#"{"name": "Bo"}"#.into())
            .await
            .unwrap();

        assert_eq!(greeting.as_deref(), Some(r#""Hello, Ada!""#), "{name}");
        assert_eq!(shout, r#""HI!""#, "{name}");
        assert_eq!(blocks[0].content, "Greetings, Bo.", "{name}");
    }
}

#[tokio::test]
async fn a_failing_call_is_an_error_not_a_trap() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();

        let result = plugin
            .call_command(
                "greet".into(),
                vec!["42".into()],
                &mut TestContext::default(),
            )
            .await;
        assert!(
            matches!(&result, Err(PluginError::Call(msg)) if msg.contains("expected a string")),
            "{name}: {result:?}"
        );

        // Still callable afterwards.
        let result = plugin
            .call_command(
                "greet".into(),
                vec![r#""Ada""#.into()],
                &mut TestContext::default(),
            )
            .await;
        assert!(result.is_ok(), "{name}: {result:?}");
    }
}

#[tokio::test]
async fn a_command_reads_and_writes_through_its_context() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();
        let mut context = TestContext::default();
        context.set("tally", "n", "41".into()).unwrap();

        let count = plugin
            .call_command("tally".into(), vec![r#""n""#.into()], &mut context)
            .await
            .unwrap();

        assert_eq!(count.as_deref(), Some("42"), "{name}");
        assert_eq!(
            context.get("tally", "n").unwrap().as_deref(),
            Some("42"),
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_command_that_didnt_declare_mutation_cannot_write() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();
        let mut context = TestContext::default();

        let result = plugin
            .call_command("greet".into(), vec![r#""sneak""#.into()], &mut context)
            .await;

        assert!(
            matches!(&result, Err(PluginError::Call(msg)) if msg.contains("mutates-context")),
            "{name}: {result:?}"
        );
        assert!(context.0.is_empty(), "{name}");
    }
}

#[tokio::test]
async fn reading_the_context_needs_permission() {
    let host = host();
    let package = compile(&host, "no_data");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    let result = plugin
        .call_command(
            "tally".into(),
            vec![r#""n""#.into()],
            &mut TestContext::default(),
        )
        .await;

    assert!(
        matches!(&result, Err(PluginError::Call(msg)) if msg.contains("state:read")),
        "{result:?}"
    );
}

#[tokio::test]
async fn the_frontend_calls_its_own_backend() {
    let host = host();
    for name in HELLOS {
        let package = compile(&host, name);
        let mut plugin = host
            .instantiate(&package, Arc::new(TestServices::default()))
            .await
            .unwrap();

        let greeting = plugin
            .call_frontend("greet".into(), r#"{"name": "Ada"}"#.into())
            .await
            .unwrap();
        let unknown = plugin.call_frontend("nope".into(), "{}".into()).await;

        assert_eq!(greeting, r#""Hello, Ada!""#, "{name}");
        assert!(matches!(unknown, Err(PluginError::Call(_))), "{name}");
    }
}

#[tokio::test]
async fn staging_copies_only_the_built_plugin() {
    build_hello();
    let into = std::env::temp_dir().join(format!("lorealm-stage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&into);
    std::fs::create_dir_all(&into).unwrap();

    // Its source points outside its own directory, at hello's build output.
    let manifest = lorealm_plugins::stage(&fixture("hello_addon"), &into).unwrap();
    let package = host().compile(&into);

    assert_eq!(manifest.structure.backend, PathBuf::from("backend.wasm"));
    assert_eq!(manifest.frontend.path, PathBuf::from("frontend"));
    assert!(into.join("frontend/panel.js").is_file());
    assert!(!into.join("README.md").exists(), "hello_addon has none");
    assert!(package.is_ok(), "{:?}", package.err());
    std::fs::remove_dir_all(&into).unwrap();
}

#[tokio::test]
async fn plugin_reads_its_settings() {
    let host = host();
    let package = compile(&host, "hello");
    let services = Arc::new(TestServices::default());
    let mut plugin = host.instantiate(&package, services).await.unwrap();

    let settings = plugin
        .call_frontend("settings".into(), "{}".into())
        .await
        .unwrap();

    assert_eq!(settings, r#"{"greeting":"Hi"}"#);
}

#[tokio::test]
async fn plugin_fetches_only_from_its_hosts() {
    let host = host();
    let package = compile(&host, "hello");
    let services = Arc::new(TestServices::default());
    let mut plugin = host.instantiate(&package, services.clone()).await.unwrap();
    let mut fetch = async |url: &str| {
        let args = format!(r#"{{"url":"{url}"}}"#);
        plugin.call_frontend("fetch".into(), args).await
    };

    let allowed = fetch("https://example.com/speak").await.unwrap();
    let other_port = fetch("https://example.com:8443/speak").await;
    let other_host = fetch("https://example.org/speak").await;
    let not_http = fetch("file:///etc/passwd").await;

    assert_eq!(allowed, r#""https://example.com/speak""#);
    assert!(matches!(other_port, Err(PluginError::Call(_))));
    assert!(matches!(other_host, Err(PluginError::Call(_))));
    assert!(matches!(not_http, Err(PluginError::Call(_))));
    assert!(services.logged(
        Level::Warn,
        LogSource::Host,
        "isn't in the manifest's network hosts"
    ));
}

#[tokio::test]
async fn time_spent_fetching_does_not_count_toward_the_limit() {
    // Fetching takes longer than a whole call may.
    let host = PluginHost::new(Duration::from_millis(200)).unwrap();
    let package = compile(&host, "hello");
    let services = Arc::new(TestServices {
        latency: Duration::from_millis(500),
        ..Default::default()
    });
    let mut plugin = host.instantiate(&package, services).await.unwrap();

    let fetched = plugin
        .call_frontend("fetch".into(), r#"{"url":"https://example.com/"}"#.into())
        .await;

    assert!(fetched.is_ok(), "{fetched:?}");
}

#[tokio::test]
async fn fetching_needs_permission() {
    let host = host();
    let package = compile(&host, "no_data");
    let services = Arc::new(TestServices::default());
    let mut plugin = host.instantiate(&package, services.clone()).await.unwrap();

    let result = plugin
        .call_frontend("fetch".into(), r#"{"url":"https://example.com/"}"#.into())
        .await;

    assert!(matches!(&result, Err(PluginError::Call(msg)) if msg.contains("net:fetch")));
    assert!(services.logged(
        Level::Warn,
        LogSource::Host,
        "missing permission `net:fetch`"
    ));
}

#[tokio::test]
async fn a_plugin_emits_only_its_declared_events() {
    let host = host();
    build_hello();
    let hello = host.compile(&fixture("hello")).unwrap();
    let addon = host.compile(&fixture("hello_addon")).unwrap();
    let services = Arc::new(TestServices::default());
    let mut hello = host.instantiate(&hello, services.clone()).await.unwrap();
    let mut addon = host.instantiate(&addon, services.clone()).await.unwrap();
    let emit = |topic: &str| format!(r#"{{"topic": "{topic}", "payload": 1}}"#);

    let undeclared = hello.call_frontend("emit".into(), emit("hello.ping")).await;
    let not_its_own = addon.call_frontend("emit".into(), emit("hello.ping")).await;
    addon
        .call_frontend("emit".into(), emit("hello_addon.ping"))
        .await
        .unwrap();

    assert!(matches!(undeclared, Err(PluginError::Call(_))));
    assert!(matches!(not_its_own, Err(PluginError::Call(_))));
    assert!(services.logged(Level::Warn, LogSource::Host, "`hello.ping` isn't"));
    assert_eq!(
        addon.take_emitted(),
        [("hello_addon.ping".to_owned(), "1".to_owned())]
    );
}

#[tokio::test]
async fn an_event_reaches_only_subscribers() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    plugin
        .custom_event("hello_addon.other".into(), "1".into())
        .await
        .unwrap();
    let ignored = plugin
        .call_frontend("event".into(), "{}".into())
        .await
        .unwrap();
    plugin
        .custom_event("hello_addon.ping".into(), "2".into())
        .await
        .unwrap();
    let received = plugin
        .call_frontend("event".into(), "{}".into())
        .await
        .unwrap();

    assert_eq!(ignored, "null");
    assert_eq!(received, r#""hello_addon.ping 2""#);
}

#[tokio::test]
async fn a_plugin_subscribes_and_unsubscribes_after_init() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();
    let last_event = async |plugin: &mut lorealm_plugins::LoadedPlugin| {
        plugin
            .call_frontend("event".into(), "{}".into())
            .await
            .unwrap()
    };

    plugin
        .call_frontend("subscribe".into(), r#""hello_addon.other""#.into())
        .await
        .unwrap();
    plugin
        .custom_event("hello_addon.other".into(), "1".into())
        .await
        .unwrap();
    let subscribed = last_event(&mut plugin).await;
    plugin
        .call_frontend("unsubscribe".into(), r#""hello_addon.other""#.into())
        .await
        .unwrap();
    plugin
        .custom_event("hello_addon.other".into(), "2".into())
        .await
        .unwrap();
    let unsubscribed = last_event(&mut plugin).await;

    assert_eq!(subscribed, r#""hello_addon.other 1""#);
    assert_eq!(unsubscribed, subscribed);
}

#[tokio::test]
async fn after_generate_observes_the_reply() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    plugin
        .after_generate(Generated {
            conversation: 1,
            message: 2,
            content: "Hi there".into(),
        })
        .await
        .unwrap();
    let observed = plugin
        .call_frontend("event".into(), "{}".into())
        .await
        .unwrap();

    assert_eq!(observed, r#""after-generate Hi there""#);
}

#[tokio::test]
async fn message_finalize_changes_the_reply() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    let content = plugin
        .message_finalize(Finalizing {
            conversation: 1,
            content: "Hi there".into(),
        })
        .await
        .unwrap();

    assert_eq!(content, "Hi there!");
}

#[tokio::test]
async fn user_message_changes_or_refuses_the_input() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();
    let input = |content: &str| UserInput {
        conversation: 1,
        content: content.into(),
    };

    let changed = plugin.user_message(input("Hi")).await.unwrap();
    let denied = plugin.user_message(input("deny me")).await.unwrap();

    assert!(matches!(changed, Verdict::Change(c) if c == "Hi?"));
    assert!(matches!(denied, Verdict::Deny(r) if r == "denied on purpose"));
}

fn entry(content: &str) -> LorebookEntry {
    LorebookEntry {
        id: content.into(),
        content: content.into(),
        priority: 0,
    }
}

#[tokio::test]
async fn lorebook_activate_changes_the_entries() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    let entries = plugin
        .lorebook_activate(LorebookActivation {
            slot: "setting".into(),
            entries: vec![entry("dragons"), entry("drop me")],
        })
        .await
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "dragons");
    assert_eq!(entries[0].content, "dragons!");
}

#[tokio::test]
async fn tool_call_changes_or_refuses_the_call() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();
    let call = |name: &str| ToolInvocation {
        name: name.into(),
        arguments: r#"{"city":"Oslo"}"#.into(),
    };

    let changed = plugin.tool_call(call("weather")).await.unwrap();
    let denied = plugin.tool_call(call("forbidden")).await.unwrap();

    assert!(matches!(changed, Verdict::Change(a) if a == r#"{"city":"Oslo","seen":1}"#));
    assert!(matches!(denied, Verdict::Deny(r) if r == "denied on purpose"));
}
