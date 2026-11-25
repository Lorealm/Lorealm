//! Profile validation and model listing for the native providers.

use crate::{errors::ConnectionError, services::connection::ProfileInput, tests::Harness};
use lorealm_storage::models::Snowflake;

fn input(provider: &str, secret_id: Option<Snowflake>, params: serde_json::Value) -> ProfileInput {
    ProfileInput {
        secret_id,
        name: "profile".into(),
        model: Some("model".into()),
        token_budget: 4096,
        provider: provider.into(),
        common_params: serde_json::json!({}),
        provider_params: params,
    }
}

async fn rejects(h: &Harness, provider: &str, params: serde_json::Value) {
    let result = h
        .connection
        .create_profile(h.user, input(provider, None, params.clone()))
        .await;
    assert!(
        matches!(result, Err(ConnectionError::Validation(_))),
        "{provider} accepted {params}"
    );
}

#[tokio::test]
async fn native_provider_params_are_stored_normalized() {
    let h = Harness::new().await;

    let profile = h
        .connection
        .create_profile(
            h.user,
            input(
                "vertex",
                None,
                serde_json::json!({
                    "project_id": "example.com:my-project",
                    "location": "europe-west4",
                    "thinking": {"thinking_level": "low"}
                }),
            ),
        )
        .await
        .unwrap();
    assert_eq!(profile.provider_params["location"], "europe-west4");

    h.connection
        .create_profile(
            h.user,
            input(
                "anthropic",
                None,
                serde_json::json!({
                    "thinking": {"type": "adaptive", "display": "summarized"},
                    "effort": "high"
                }),
            ),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn bad_native_provider_params_are_rejected() {
    let h = Harness::new().await;

    rejects(
        &h,
        "anthropic",
        serde_json::json!({ "base_url": "api.anthropic.com" }),
    )
    .await;
    rejects(&h, "anthropic", serde_json::json!({ "thinkng": {} })).await;
    rejects(&h, "vertex", serde_json::json!({ "region": "us-central1" })).await;
    rejects(&h, "vertex", serde_json::json!({ "project_id": "a/../b" })).await;
    // The location is part of the hostname the key is sent to.
    rejects(
        &h,
        "vertex",
        serde_json::json!({ "location": "evil.example/" }),
    )
    .await;
    rejects(&h, "vertex", serde_json::json!({ "location": "" })).await;
    // AI Studio has no project or region.
    rejects(&h, "gemini", serde_json::json!({ "project_id": "p" })).await;
    rejects(
        &h,
        "openai_platform",
        serde_json::json!({ "template": {
            "system_prefix": "", "system_suffix": "", "user_prefix": "",
            "user_suffix": "", "assistant_prefix": "", "assistant_suffix": ""
        }}),
    )
    .await;
}

#[tokio::test]
async fn profiles_that_cannot_list_models_get_an_empty_list() {
    let h = Harness::new().await;

    for provider in ["anthropic", "vertex", "gemini", "openai_platform"] {
        let profile = h
            .connection
            .create_profile(h.user, input(provider, None, serde_json::json!({})))
            .await
            .unwrap();
        let models = h.connection.list_models(h.user, profile.id).await.unwrap();
        assert!(models.is_empty(), "{provider}");
    }
}

#[tokio::test]
async fn anthropic_models_are_listed_with_the_stored_key() {
    let endpoint = axum::Router::new().route(
        "/v1/models",
        axum::routing::get(|headers: axum::http::HeaderMap| async move {
            assert_eq!(headers["x-api-key"], "sk-test");
            axum::Json(serde_json::json!({
                "data": [{"id": "claude-opus-5", "display_name": "Claude Opus 5",
                          "max_input_tokens": 1000000}],
                "has_more": false
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, endpoint).await.unwrap() });

    let h = Harness::new().await;
    let secret = h
        .connection
        .create_secret(
            h.user,
            "anthropic".into(),
            "main".into(),
            "sk-test".into(),
            &[],
        )
        .await
        .unwrap();
    let profile = h
        .connection
        .create_profile(
            h.user,
            input(
                "anthropic",
                Some(secret.id),
                serde_json::json!({ "base_url": base_url }),
            ),
        )
        .await
        .unwrap();

    let models = h.connection.list_models(h.user, profile.id).await.unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].name.as_deref(), Some("Claude Opus 5"));
    assert_eq!(models[0].context_length, Some(1_000_000));
}

/// Serves one listing route on a local port and returns the API root.
async fn serve_listing(
    path: &'static str,
    check: fn(&axum::http::HeaderMap),
    body: serde_json::Value,
) -> String {
    let endpoint = axum::Router::new().route(
        path,
        axum::routing::get(move |headers: axum::http::HeaderMap| async move {
            check(&headers);
            axum::Json(body)
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, endpoint).await.unwrap() });
    base_url
}

async fn profile_with_key(h: &Harness, provider: &str, base_url: String) -> Snowflake {
    let secret = h
        .connection
        .create_secret(
            h.user,
            provider.into(),
            "main".into(),
            "sk-test".into(),
            &[],
        )
        .await
        .unwrap();
    h.connection
        .create_profile(
            h.user,
            input(
                provider,
                Some(secret.id),
                serde_json::json!({ "base_url": base_url }),
            ),
        )
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn gemini_models_are_listed_with_the_stored_key() {
    let base_url = serve_listing(
        "/v1/models",
        |headers| assert_eq!(headers["x-goog-api-key"], "sk-test"),
        serde_json::json!({ "models": [
            {"name": "models/gemini-3-pro", "displayName": "Gemini 3 Pro",
             "supportedGenerationMethods": ["generateContent"]},
            {"name": "models/embedding-001", "supportedGenerationMethods": ["embedContent"]}
        ]}),
    )
    .await;

    let h = Harness::new().await;
    let profile = profile_with_key(&h, "gemini", base_url).await;

    let models = h.connection.list_models(h.user, profile).await.unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "gemini-3-pro");
}

#[tokio::test]
async fn openai_platform_models_are_listed_with_the_stored_key() {
    let base_url = serve_listing(
        "/v1/models",
        |headers| assert_eq!(headers["authorization"], "Bearer sk-test"),
        serde_json::json!({ "object": "list", "data": [{"id": "gpt-5", "object": "model"}] }),
    )
    .await;

    let h = Harness::new().await;
    let profile = profile_with_key(&h, "openai_platform", base_url).await;

    let models = h.connection.list_models(h.user, profile).await.unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "gpt-5");
}

#[tokio::test]
async fn keys_stay_with_their_own_provider() {
    let h = Harness::new().await;
    let secret = h
        .connection
        .create_secret(
            h.user,
            "openai".into(),
            "local".into(),
            "sk-test".into(),
            &[],
        )
        .await
        .unwrap();
    let result = h
        .connection
        .create_profile(
            h.user,
            input("openai_platform", Some(secret.id), serde_json::json!({})),
        )
        .await;
    assert!(matches!(result, Err(ConnectionError::Validation(_))));
}
