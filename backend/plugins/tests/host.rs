use std::path::{Path, PathBuf};
use std::process::Command;

use lorealm_plugins::{PluginError, PluginHost};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Builds a fixture component with the wasm32-wasip2 target.
fn build_fixture(name: &str) -> PathBuf {
    let dir = fixture(name);
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(&dir)
        .status()
        .expect("failed to run cargo");
    assert!(status.success(), "failed to build fixture {name}");
    dir
}

#[tokio::test]
async fn loads_and_inits_component() {
    let dir = build_fixture("hello");
    let host = PluginHost::new().unwrap();

    let plugin = host.load(&dir).await.unwrap();

    assert_eq!(plugin.manifest.plugin.handle, "hello");
}

#[tokio::test]
async fn missing_manifest_is_an_error() {
    let host = PluginHost::new().unwrap();

    let result = host.load(&fixture("does-not-exist")).await;

    assert!(matches!(result, Err(PluginError::Manifest(_))));
}
