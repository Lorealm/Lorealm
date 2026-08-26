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

mod openai;
mod openai_wire;
mod openrouter;

pub use openai::{OPENAI_DEFAULT_BASE_URL, OpenAiOptions, OpenAiProvider};
pub use openrouter::{
    OpenRouterOptions, OpenRouterProvider, OpenRouterProviderPreference, OpenRouterReasoning,
};
use serde::Serialize;
