use std::process::Command;

use lorealm_storage::PersonaStorage;

use crate::{
    errors::PluginError,
    services::{chat::PromptMessage, context::PromptRole},
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
    vec![PromptMessage {
        role: PromptRole::User,
        content: "Hi".into(),
    }]
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
