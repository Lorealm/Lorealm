use std::sync::Arc;

use lorealm_storage::models::{AgentObservation, Character, ConversationMessage, Snowflake};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::services::{
    context::PromptRole,
    state::{CapturedState, ReconstructedState},
    variables::Variables,
};

#[derive(Debug, Clone, Serialize)]
pub enum ContentRef {
    /// The text must be stored verbatim. Used for unpredictable/dynamic sources
    /// that can't be reconstructed reliably
    Verbatim,
    Message(Snowflake),
    Character(CharacterPart),
    /// Recoverable reference defined by a plugin source.
    /// The frontend dispatches on source_id to interpret `data`.
    Plugin {
        source_id: String,
        data: serde_json::Value,
    },
}

impl ContentRef {
    /// Whether the live content can be dropped before persisting.
    pub fn is_recoverable(&self) -> bool {
        !matches!(self, ContentRef::Verbatim)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum CharacterPart {
    Definition,
    Personality,
    Scenario,
    Greeting { order: i64 },
    Example { order: i64 },
}

/// One resolved piece of context, tagged with the role it occupies.
/// Tree order is preserved by the walk.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedBlock {
    pub role: PromptRole,
    pub content: String,
    pub content_ref: ContentRef,
}

#[derive(Debug, Clone, Serialize)]
pub struct TracedBlock {
    #[serde(flatten)]
    pub block: ResolvedBlock,
    pub origin: BlockOrigin,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockOrigin {
    /// The ContextNode that emitted this block.
    pub node_id: Snowflake,
    pub node_title: String,
    /// Enclosing group ids, outermost first. Empty = root-level.
    pub path: Vec<Snowflake>,
    pub kind: OriginKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OriginKind {
    Text,
    Source { source_id: String, node_id: String },
    GroupWrapper, // a prefix/suffix block, belongs to the group node
}

pub struct ResolveContext<'a> {
    pub character: Option<&'a Character>,
    pub messages: &'a [PromptTurn],
    pub user_id: Snowflake,
    pub state: &'a ReconstructedState,
    pub lorebook_state: Mutex<Option<CapturedState>>,
    /// `None` = the user's active lorebooks; `Some` = exactly these books
    /// (realm turns feed the agent's bound imports here).
    pub lorebook_ids: Option<&'a [Snowflake]>,
    pub tokenizer: &'a Arc<kitoken::Kitoken>,
    /// Host variables for this turn, materialized once so the lorebook host and
    /// block substitution cannot disagree. See [`crate::services::variables`].
    pub variables: Variables,
}

/// A single prompt-visible turn, source-agnostic. Both conversation
/// messages and agent observations map into this before assembly.
pub struct PromptTurn {
    pub role: PromptRole,
    pub content: String,
    /// How this turn's content can be recovered when the trace is persisted.
    pub content_ref: ContentRef,
}

impl PromptTurn {
    pub fn from_message(m: &ConversationMessage) -> Self {
        Self {
            role: chat_role_to_prompt_role(&m.role), // reuse whatever the chat resolver uses today
            content: m.content.clone(),
            content_ref: ContentRef::Message(m.id),
        }
    }

    pub fn from_observation(o: &AgentObservation) -> Self {
        Self {
            role: chat_role_to_prompt_role(&o.role),
            content: o.content.clone(),
            content_ref: ContentRef::Verbatim, // see note below
        }
    }
}

fn chat_role_to_prompt_role(role: &lorealm_storage::prelude::ChatRole) -> PromptRole {
    match role {
        lorealm_storage::models::ChatRole::System => PromptRole::System,
        lorealm_storage::models::ChatRole::User => PromptRole::User,
        lorealm_storage::models::ChatRole::Assistant => PromptRole::Assistant,
    }
}
