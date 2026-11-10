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
    /// Seeds the new conversation's author's note. Empty for none.
    #[serde(default)]
    pub author_note: String,
}

/// Start a playthrough of a realm.
#[derive(Deserialize)]
pub struct StartRealmChatRequest {
    pub realm_id: Snowflake,
    /// The scenario to open with. `None` uses the realm's only scenario, or
    /// emergent start when the realm defines none.
    #[serde(default)]
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ChatEdit {
    /// Edit a message in place
    Edit { id: Snowflake, content: String },
    /// Move the active path to run through this message
    Activate { id: Snowflake },
}

#[derive(Deserialize)]
pub struct SetAuthorNoteRequest {
    pub author_note: String,
}

#[derive(Deserialize)]
pub struct AgentStateQuery {
    /// Reconstruct state up to this message
    pub at: Option<Snowflake>,
}
