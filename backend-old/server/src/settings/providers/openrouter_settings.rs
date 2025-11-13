use std::collections::HashMap;

use crate::settings::provider_settings::CoreSettings;

pub enum OpenRouterVerbosity {
    Low,
    Medium,
    High
}

impl OpenRouterVerbosity {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenRouterVerbosity::Low => "low",
            OpenRouterVerbosity::Medium => "medium",
            OpenRouterVerbosity::High => "high",
        }
    }
}

pub struct OpenRouterSettings {
    core: CoreSettings,

    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    repetition_penalty: Option<f32>,
    min_p: Option<f32>,
    top_a: Option<u32>,
    seed: Option<i32>,
    /// WARNING: Incompletely implemented
    logit_bias: Option<HashMap<i32, i32>>,
    logprobs: Option<bool>,
    top_logprobs: Option<i32>,
    /// Whether models should conduct reasoning
    reasoning: bool,
    /// Whether reasoning is included in the response
    include_reasoning: bool,
    /// Whether to stream the response
    stream: bool,

    tools: Option<Vec<String>>,
    tool_choice: Option<Vec<String>>,

    verbosity: Option<OpenRouterVerbosity>
}