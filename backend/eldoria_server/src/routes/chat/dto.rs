use eldoria_storage::models::Snowflake;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct StartChatRequest {
    pub character_id: Snowflake,
    pub version_id: Option<Snowflake>,
    pub greeting_id: Snowflake,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ChatEdit {
    /// Edit a message in place (no new branch).
    Edit { id: Snowflake, content: String },
    /// Swipe/branch a user message: new sibling, becomes active leaf.
    /// Client then calls /completions to generate the reply.
    BranchUser { id: Snowflake, content: String },
    /// Move the active path to run through this message.
    Activate { id: Snowflake },
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub content: String, // TODO: enrich with multimodal
}

#[derive(Debug, Deserialize)]
pub struct CompletionsRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub target: Option<Snowflake>,
}
