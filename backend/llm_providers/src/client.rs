use crate::{error::*, providers::LlmProvider, tools::ToolRegistry, types::*};
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
    /// Append a plain-text message.
    pub fn message(mut self, role: Role, content: impl Into<String>) -> Self {
        self.messages.push(Message::text(role, content));
        self
    }

    /// Append a message with arbitrary content blocks (images, tool results, ...).
    pub fn message_blocks(mut self, role: Role, content: Vec<ContentBlock>) -> Self {
        self.messages.push(Message { role, content });
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

    pub fn common_params(mut self, params: CommonParams) -> Self {
        self.common_params = params;
        self
    }

    /// Advertise tool definitions to the model without executing them
    /// automatically. Use this when you want to handle `ToolUse` blocks
    /// yourself; use `send_with_tools` for the automatic loop.
    pub fn tools(mut self, tools: Vec<ToolDef>) -> Self {
        self.common_params.tools = Some(tools);
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

    /// Send the request and automatically execute any tool calls the model
    /// makes, feeding results back until the model produces a response with
    /// no tool calls (or `max_rounds` is exceeded).
    ///
    /// Tool definitions are taken from the registry unless already set via
    /// `tools()`. Tool execution errors are reported back to the model as
    /// error results rather than aborting the loop.
    pub async fn send_with_tools(
        mut self,
        registry: &ToolRegistry,
        max_rounds: usize,
    ) -> Result<Response> {
        if self.common_params.tools.is_none() {
            self.common_params.tools = Some(registry.defs());
        }

        for _ in 0..max_rounds {
            let response = self
                .provider
                .complete(
                    self.messages.clone(),
                    &self.common_params,
                    &self.provider_params,
                )
                .await?;

            let tool_uses: Vec<(String, String, serde_json::Value)> = response
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect();

            if tool_uses.is_empty() {
                return Ok(response);
            }

            self.messages.push(Message {
                role: Role::Assistant,
                content: response.content,
            });

            let mut results = Vec::with_capacity(tool_uses.len());
            for (id, name, input) in tool_uses {
                let block = match registry.call(&name, input).await {
                    Ok(value) => ContentBlock::ToolResult {
                        tool_use_id: id,
                        content: value.to_string(),
                        is_error: false,
                    },
                    Err(e) => ContentBlock::ToolResult {
                        tool_use_id: id,
                        content: e.to_string(),
                        is_error: true,
                    },
                };
                results.push(block);
            }
            self.messages.push(Message {
                role: Role::User,
                content: results,
            });
        }

        Err(Error::Tool(format!(
            "tool loop did not finish within {} rounds",
            max_rounds
        )))
    }

    pub async fn stream(self) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        self.provider
            .stream(self.messages, &self.common_params, &self.provider_params)
            .await
    }
}
