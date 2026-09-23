use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lorealm_plugins::{
    HostServices, Message, Persona, PluginError, PluginHost, PluginPackage, Role,
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
}

fn ask(content: &str) -> Vec<Message> {
    vec![Message {
        role: Role::User,
        content: content.into(),
    }]
}

fn compile(host: &PluginHost, name: &str) -> PluginPackage {
    build_hello();
    host.compile(&fixture(name)).unwrap()
}

#[tokio::test]
async fn prompt_build_hook_modifies_prompt() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    let prompt = vec![Message {
        role: Role::User,
        content: "Hi".into(),
    }];
    let prompt = plugin.prompt_build(prompt).await.unwrap();

    assert_eq!(prompt.len(), 2);
    assert_eq!(prompt[1].content, "Hello, Ada! #1");
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
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    let prompt = vec![Message {
        role: Role::User,
        content: "loop forever".into(),
    }];
    let result = plugin.prompt_build(prompt).await;

    assert!(
        matches!(result, Err(PluginError::Timeout)),
        "{:?}",
        result.err()
    );

    // A trapped instance can't be entered again.
    let result = plugin.prompt_build(Vec::new()).await;
    assert!(
        matches!(result, Err(PluginError::Wasm(_))),
        "{:?}",
        result.err()
    );
}

#[tokio::test]
async fn persisted_data_survives_between_calls() {
    let host = host();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    plugin.prompt_build(ask("Hi")).await.unwrap();
    let second = plugin.prompt_build(ask("Hi")).await.unwrap();
    let forgotten = plugin.prompt_build(ask("forget")).await.unwrap();

    assert_eq!(second[1].content, "Hello, Ada! #2");
    assert_eq!(forgotten[1].content, "Hello, Ada! #1");
}

#[tokio::test]
async fn persisting_without_permission_fails_the_hook() {
    let host = host();
    let package = compile(&host, "no_data");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices::default()))
        .await
        .unwrap();

    let result = plugin.prompt_build(ask("Hi")).await;

    assert!(
        matches!(&result, Err(PluginError::Hook(msg)) if msg.contains("data:persist")),
        "{:?}",
        result.err()
    );
}
