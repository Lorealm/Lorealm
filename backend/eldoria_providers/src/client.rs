use crate::{error::Result, providers::LlmProvider, types::*};
use futures::Stream;

pub struct Client<P: LlmProvider> {
    provider: P,
}

impl<P: LlmProvider> Client<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub fn request(&self) -> RequestBuilder<'_, P> {
        RequestBuilder {
            provider: &self.provider,
            messages: Vec::new(),
            common_params: CommonParams::default(),
            provider_params: P::ProviderParams::default(),
        }
    }

    /// List available models for this provider (if supported)
    pub async fn list_models(&self) -> Result<Option<Vec<ModelInfo>>> {
        self.provider.list_models().await
    }
}

pub struct RequestBuilder<'a, P: LlmProvider> {
    provider: &'a P,
    messages: Vec<Message>,
    common_params: CommonParams,
    provider_params: P::ProviderParams,
}

impl<'a, P: LlmProvider> RequestBuilder<'a, P> {
    pub fn message(mut self, role: Role, content: impl Into<String>) -> Self {
        self.messages.push(Message {
            role,
            content: content.into(),
        });
        self
    }

    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn system(self, content: impl Into<String>) -> Self {
        self.message(Role::System, content)
    }

    pub fn user(self, content: impl Into<String>) -> Self {
        self.message(Role::User, content)
    }

    pub fn assistant(self, content: impl Into<String>) -> Self {
        self.message(Role::Assistant, content)
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.common_params.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.common_params.max_tokens = Some(max_tokens);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.common_params.top_p = Some(top_p);
        self
    }

    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.common_params.stop = Some(stop);
        self
    }

    pub fn configure<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut P::ProviderParams),
    {
        f(&mut self.provider_params);
        self
    }

    pub async fn send(self) -> Result<Response> {
        self.provider
            .complete(self.messages, &self.common_params, &self.provider_params)
            .await
    }

    pub async fn stream(
        self,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        self.provider
            .stream(self.messages, &self.common_params, &self.provider_params)
            .await
    }
}