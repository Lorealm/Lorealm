use lorealm_storage::models::{ConversationMessage, Snowflake};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::commands::CommandEffect;

/// One `/name` invocation as the composer parsed it.
#[derive(Debug, Default, Deserialize)]
pub struct CommandArgs {
    /// `key=value` pairs
    #[serde(default)]
    pub named: HashMap<String, String>,
    /// Whatever text followed the named parameters, always bound to `{{cmd:rest}}`.
    #[serde(default)]
    pub rest: String,
}

#[derive(Debug, Deserialize)]
pub struct RunCommandRequest {
    pub conversation_id: Snowflake,
    #[serde(flatten)]
    pub args: CommandArgs,
}

/// What one command run produced.
#[derive(Debug, Serialize)]
pub struct CommandRun {
    /// The body's return value, typed. `null` when it returned nothing.
    pub output: serde_json::Value,
    /// Effects for the client to carry out.
    pub effects: Vec<CommandEffect>,
    /// The system message this run anchored its state writes to, if any.
    pub message: Option<ConversationMessage>,
}
