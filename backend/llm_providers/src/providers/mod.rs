use crate::{error::Result, types::*};
use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    type ProviderParams: Default + Send + Sync + Serialize;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Response>;

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>>;

    /// List available models for this provider (if supported)
    /// Returns None if the provider doesn't support listing models
    async fn list_models(&self) -> Result<Option<Vec<ModelInfo>>> {
        Ok(None)
    }
}

/// Attaches `request` as the JSON body with the profile's custom fields
/// applied: `extra_body` merged in, then `exclude_body` keys dropped, and
/// `extra_headers` replacing any header of the same name. Apply it last.
fn with_custom_fields(
    builder: reqwest::RequestBuilder,
    request: &impl Serialize,
    params: &CommonParams,
) -> reqwest::RequestBuilder {
    // Through text rather than `to_value`, which widens f32s: 0.7 would go out as 0.699999988.
    let text = serde_json::to_vec(request).expect("request serialization cannot fail");
    let mut body: serde_json::Value = serde_json::from_slice(&text).expect("just serialized");
    if let Some(object) = body.as_object_mut() {
        if let Some(extra) = &params.extra_body {
            merge(object, extra);
        }
        for key in &params.exclude_body {
            object.remove(key);
        }
    }

    // Invalid names and values are rejected when a profile is saved.
    let headers: reqwest::header::HeaderMap = params
        .extra_headers
        .iter()
        .filter_map(|(name, value)| {
            Some((
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).ok()?,
                reqwest::header::HeaderValue::from_str(value).ok()?,
            ))
        })
        .collect();

    builder.json(&body).headers(headers)
}

fn merge(
    into: &mut serde_json::Map<String, serde_json::Value>,
    from: &serde_json::Map<String, serde_json::Value>,
) {
    for (key, value) in from {
        match (into.get_mut(key), value) {
            (Some(serde_json::Value::Object(a)), serde_json::Value::Object(b)) => merge(a, b),
            _ => {
                into.insert(key.clone(), value.clone());
            }
        }
    }
}

mod anthropic;
mod anthropic_wire;
mod gemini_wire;
mod openai;
mod openai_wire;
mod openrouter;
mod vertex;

pub use anthropic::{
    ANTHROPIC_DEFAULT_BASE_URL, AnthropicOptions, AnthropicProvider, AnthropicThinking,
};
pub use openai::{OPENAI_DEFAULT_BASE_URL, OpenAiOptions, OpenAiProvider};
pub use openrouter::{
    OpenRouterOptions, OpenRouterProvider, OpenRouterProviderPreference, OpenRouterReasoning,
};
use serde::Serialize;
pub use vertex::{
    VERTEX_DEFAULT_LOCATION, VertexAuth, VertexOptions, VertexProvider, VertexSafetySetting,
    VertexThinking,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn custom_fields_merge_exclude_and_override_headers() {
        let params = CommonParams {
            extra_body: json!({"temperature": 0.5, "provider": {"sort": "price"}})
                .as_object()
                .cloned(),
            exclude_body: vec!["stop".into()],
            extra_headers: vec![("authorization".into(), "Bearer mine".into())],
            ..CommonParams::default()
        };
        let request = json!({
            "temperature": 0.7,
            "top_p": 0.9,
            "stop": ["\n"],
            "provider": {"order": ["a"]}
        });
        let builder = reqwest::Client::new()
            .post("http://localhost/")
            .header("Authorization", "Bearer provider");

        let built = with_custom_fields(builder, &request, &params)
            .build()
            .unwrap();

        let body: serde_json::Value =
            serde_json::from_slice(built.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(
            body,
            json!({
                "temperature": 0.5,
                "top_p": 0.9,
                "provider": {"order": ["a"], "sort": "price"}
            })
        );
        let auth: Vec<_> = built.headers().get_all("authorization").iter().collect();
        assert_eq!(auth, ["Bearer mine"]);
    }
}
