//! Gemini models through the Gemini API, keyed from Google AI Studio.
//!
//! The Gemini API speaks the same `generateContent` format as Vertex AI, so
//! requests go through [`VertexProvider`] pointed at a different root; only
//! the options and the model listing differ.

use super::vertex::{VertexAuth, VertexOptions, VertexProvider};
use super::{VertexSafetySetting, VertexThinking};
use crate::{error::*, types::*};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const GEMINI_DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Clone)]
pub struct GeminiProvider {
    inner: VertexProvider,
    client: Arc<HttpClient>,
    api_key: String,
    base_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct GeminiOptions {
    /// Root of the API, including the version segment. Defaults to
    /// [`GEMINI_DEFAULT_BASE_URL`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<VertexThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<VertexSafetySetting>>,
}

impl GeminiOptions {
    fn to_vertex(&self) -> VertexOptions {
        VertexOptions {
            project_id: None,
            location: None,
            thinking: self.thinking.clone(),
            safety_settings: self.safety_settings.clone(),
        }
    }
}

impl GeminiProvider {
    pub fn new(
        client: Arc<HttpClient>,
        model: impl Into<String>,
        base_url: Option<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let base_url = base_url
            .unwrap_or_else(|| GEMINI_DEFAULT_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        let inner = VertexProvider::new(
            Arc::clone(&client),
            model,
            VertexAuth::ApiKey(api_key.clone()),
            None,
            None,
        )
        .with_base_url(base_url.clone());
        Self {
            inner,
            client,
            api_key,
            base_url,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelList {
    #[serde(default)]
    models: Vec<WireModel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireModel {
    name: String,
    display_name: Option<String>,
    description: Option<String>,
    input_token_limit: Option<u32>,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
}

/// Keep only models that can chat, under the id `generateContent` expects
/// ("gemini-2.5-pro", not "models/gemini-2.5-pro").
fn to_model_infos(list: ModelList) -> Vec<ModelInfo> {
    list.models
        .into_iter()
        .filter(|m| {
            m.supported_generation_methods
                .iter()
                .any(|g| g == "generateContent")
        })
        .map(|m| ModelInfo {
            id: m
                .name
                .strip_prefix("models/")
                .unwrap_or(&m.name)
                .to_string(),
            name: m.display_name,
            description: m.description,
            context_length: m.input_token_limit,
            pricing: None,
        })
        .collect()
}

#[async_trait]
impl super::LlmProvider for GeminiProvider {
    type ProviderParams = GeminiOptions;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Response> {
        self.inner
            .complete(messages, params, &provider_params.to_vertex())
            .await
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        self.inner
            .stream(messages, params, &provider_params.to_vertex())
            .await
    }

    async fn list_models(&self) -> Result<Option<Vec<ModelInfo>>> {
        // One page of the maximum size covers the whole catalogue.
        let response = self
            .client
            .get(format!("{}/models?pageSize=1000", self.base_url))
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        Ok(Some(to_model_infos(response.json().await?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_keeps_chat_models_without_prefix() {
        let list: ModelList = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "name": "models/gemini-2.5-pro",
                    "displayName": "Gemini 2.5 Pro",
                    "inputTokenLimit": 1048576,
                    "supportedGenerationMethods": ["generateContent", "countTokens"]
                },
                {
                    "name": "models/text-embedding-004",
                    "supportedGenerationMethods": ["embedContent"]
                }
            ],
            "nextPageToken": "abc"
        }))
        .unwrap();

        let models = to_model_infos(list);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-2.5-pro");
        assert_eq!(models[0].name.as_deref(), Some("Gemini 2.5 Pro"));
        assert_eq!(models[0].context_length, Some(1048576));
    }

    #[test]
    fn options_reject_vertex_only_fields() {
        let result: std::result::Result<GeminiOptions, _> =
            serde_json::from_value(serde_json::json!({"project_id": "p"}));
        assert!(result.is_err());
    }
}
