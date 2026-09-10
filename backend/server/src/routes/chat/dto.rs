use lorealm_storage::models::Snowflake;
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
    /// Seeds the new conversation's author's note. Empty for none.
    #[serde(default)]
    pub author_note: String,
}

/// Instantiate a realm preset into a fresh playthrough conversation.
#[derive(Deserialize)]
pub struct StartRealmChatRequest {
    pub realm_id: Snowflake,
    /// The scenario to open with. `None` uses the realm's only scenario, or
    /// emergent start when the realm defines none.
    #[serde(default)]
    pub scenario_id: Option<Snowflake>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ChatEdit {
    /// Edit a message in place
    Edit { id: Snowflake, content: String },
    /// Swipe/branch a user message
    BranchUser { id: Snowflake, content: String },
    /// Move the active path to run through this message
    Activate { id: Snowflake },
}

#[derive(Debug, Deserialize)]
pub struct CompletionsRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub target: Option<Snowflake>,
    /// Write on from an existing assistant reply instead of starting a new one.
    #[serde(default, rename = "continue")]
    pub continued: bool,
}

#[derive(Deserialize)]
pub struct SetAuthorNoteRequest {
    pub author_note: String,
}
