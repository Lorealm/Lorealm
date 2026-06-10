use crate::{error::Result, types::*};
use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    type ProviderParams: Default + Send + Sync;

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
mod anthropic;
mod openrouter;

pub use openai::{OpenAIProvider, OpenAIOptions};
pub use anthropic::{AnthropicProvider, AnthropicOptions};
pub use openrouter::{OpenRouterProvider, OpenRouterOptions};