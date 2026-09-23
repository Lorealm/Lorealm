use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use lorealm_plugins::{
    CommandContext, HostServices, Level, LogEntry, LogSource, Message, Persona, PluginError,
    PluginHost, PluginPackage, Role,
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
    assert!(package.is_ok(), "{:?}", package.err());
    std::fs::remove_dir_all(&into).unwrap();
}
