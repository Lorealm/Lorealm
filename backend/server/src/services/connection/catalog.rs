use lorealm_providers::{OPENAI_DEFAULT_BASE_URL, OpenAiOptions, OpenAiProvider};
use std::sync::Arc;

pub struct OpenAiPreset {
    pub provider: &'static str,
    pub base_url: &'static str,
    /// Send the output limit as `max_completion_tokens` instead of `max_tokens`.
    pub max_completion_tokens: bool,
}

pub const OPENAI_PRESETS: &[OpenAiPreset] = &[
    OpenAiPreset {
        provider: "openai_platform",
        base_url: OPENAI_DEFAULT_BASE_URL,
        max_completion_tokens: true,
    },
    OpenAiPreset {
        provider: "nanogpt",
        base_url: "https://nano-gpt.com/api/v1",
        max_completion_tokens: false,
    },
    OpenAiPreset {
        provider: "chutes",
        base_url: "https://llm.chutes.ai/v1",
        max_completion_tokens: false,
    },
    OpenAiPreset {
        provider: "deepseek",
        base_url: "https://api.deepseek.com",
        max_completion_tokens: false,
    },
];

pub fn openai_preset(provider: &str) -> Option<&'static OpenAiPreset> {
    OPENAI_PRESETS.iter().find(|p| p.provider == provider)
}

impl OpenAiPreset {
    /// A profile's own `base_url`, e.g. a proxy, wins over the preset's.
    pub fn provider(
        &self,
        http: Arc<reqwest::Client>,
        model: impl Into<String>,
        options: &OpenAiOptions,
        api_key: Option<String>,
    ) -> OpenAiProvider {
        let base_url = options
            .base_url
            .clone()
            .unwrap_or_else(|| self.base_url.to_string());
        let provider = OpenAiProvider::new(http, model, Some(base_url), api_key);
        if self.max_completion_tokens {
            provider.with_max_completion_tokens()
        } else {
            provider
        }
    }
}
