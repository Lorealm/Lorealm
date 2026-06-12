use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Convenience constructor for a plain-text message.
    pub fn text(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentBlock::Text {
                text: content.into(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A single piece of message content. Providers translate these into their own
/// wire formats; tool results in particular are represented very differently
/// across APIs (separate `role: "tool"` messages vs. user-role blocks).
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        /// e.g. "image/png"
        media_type: String,
        source: ImageSource,
    },
    /// Reasoning/thinking produced by the model before its answer. Never
    /// sent back over the wire; providers skip it when serializing.
    Thinking {
        text: String,
    },
    /// A tool invocation requested by the model.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// The result of executing a tool, sent back to the model.
    /// By convention this lives in a `Role::User` message.
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone)]
pub enum ImageSource {
    Base64(String),
    Url(String),
}

#[derive(Debug, Clone)]
pub struct Response {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}

impl Response {
    /// Concatenated text of all text blocks.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct Delta {
    pub content: String,
    /// Reasoning/thinking tokens, when the model emits them. Empty otherwise.
    pub reasoning: String,
    pub tool_calls: Vec<ToolCallDelta>,
    pub finish_reason: Option<String>,
}

/// A fragment of a streamed tool call. Fragments sharing the same `index`
/// belong to the same call: `id` and `name` typically arrive on the first
/// fragment, while `arguments` is a partial JSON string that must be
/// concatenated across fragments (see `streaming::collect_blocks`).
#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}

/// A tool definition advertised to the model. `input_schema` is a JSON Schema
/// object describing the tool's arguments; all major providers accept it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct CommonParams {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub tools: Option<Vec<ToolDef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub context_length: Option<u32>,
    pub pricing: Option<ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub prompt: Option<f64>,     // Price per million tokens
    pub completion: Option<f64>, // Price per million tokens
}
