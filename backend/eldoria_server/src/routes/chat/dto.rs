use eldoria_storage::models::Snowflake;
use serde::Deserialize;

#[derive(serde::Deserialize)]
pub struct GetConversationQuery {
    #[serde(default)]
    pub full: bool,
}

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
    /// Edit a message in place
    Edit { id: Snowflake, content: String },
    /// Swipe/branch a user message: new sibling, becomes active leaf.
    /// Does not trigger inference
    BranchUser { id: Snowflake, content: String },
    /// Move the active path to run through this message.
    Activate { id: Snowflake },
}

#[derive(Debug, Deserialize)]
pub struct CompletionsRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub target: Option<Snowflake>,
}
