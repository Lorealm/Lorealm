use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

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

struct TestServices;

#[async_trait::async_trait]
impl HostServices for TestServices {
    async fn active_persona(&self) -> Result<Persona, String> {
        Ok(Persona {
            name: "Ada".into(),
            description: String::new(),
        })
    }
}

fn compile(host: &PluginHost, name: &str) -> PluginPackage {
    build_hello();
    host.compile(&fixture(name)).unwrap()
}

#[tokio::test]
async fn prompt_build_hook_modifies_prompt() {
    let host = PluginHost::new().unwrap();
    let package = compile(&host, "hello");
    let mut plugin = host
        .instantiate(&package, Arc::new(TestServices))
        .await
        .unwrap();

    let prompt = vec![Message {
        role: Role::User,
        content: "Hi".into(),
    }];
    let prompt = plugin.prompt_build(prompt).await.unwrap();

    assert_eq!(prompt.len(), 2);
    assert_eq!(prompt[1].content, "Hello, Ada!");
}

#[tokio::test]
async fn registering_hook_without_permission_fails_init() {
    let host = PluginHost::new().unwrap();
    let package = compile(&host, "unpermitted");

    let result = host.instantiate(&package, Arc::new(TestServices)).await;

    assert!(
        matches!(&result, Err(PluginError::Init(msg)) if msg.contains("prompt:write")),
        "{:?}",
        result.err()
    );
}

#[tokio::test]
async fn missing_manifest_is_an_error() {
    let host = PluginHost::new().unwrap();

    let result = host.compile(&fixture("does-not-exist"));

    assert!(matches!(result, Err(PluginError::Manifest(_))));
}
