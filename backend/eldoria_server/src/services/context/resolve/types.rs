use eldoria_storage::models::{Character, ConversationMessage, Snowflake, UserPersona};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::services::{context::PromptRole, state::CapturedState};

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
    pub character: &'a Character,
    pub messages: &'a [ConversationMessage],
    pub user_id: Snowflake,
    pub lorebook_state: Mutex<Option<CapturedState>>,
    pub persona: &'a UserPersona
}
