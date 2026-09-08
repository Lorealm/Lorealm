use std::sync::Arc;

use lorealm_storage::models::{
    AgentObservation, Character, ConversationMessage, Snowflake, UserPersona,
};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::services::{
    chat::{WidgetInstance, sanitize_widget_markers},
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
    Persona(UserPersona),
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

#[derive(Debug, Clone, Serialize)]
pub struct BlockPart {
    pub content: String,
    /// Higher survives longer under budget pressure.
    pub priority: i32,
}

/// How far a block can be shrunk before it has to be dropped outright.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(tag = "trim", rename_all = "snake_case")]
pub enum Trim {
    /// All or nothing.
    #[default]
    Atomic,
    /// Stays valid when rebuilt from any subset of `parts`.
    Joined {
        parts: Vec<BlockPart>,
        separator: String,
    },
}

/// One resolved piece of context, tagged with the role it occupies.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedBlock {
    pub role: PromptRole,
    pub content: String,
    pub content_ref: ContentRef,
    #[serde(flatten)]
    pub trim: Trim,
}

impl ResolvedBlock {
    pub fn atomic(role: PromptRole, content: impl Into<String>, content_ref: ContentRef) -> Self {
        Self {
            role,
            content: content.into(),
            content_ref,
            trim: Trim::Atomic,
        }
    }

    pub fn joined(
        role: PromptRole,
        parts: Vec<BlockPart>,
        separator: impl Into<String>,
        content_ref: ContentRef,
    ) -> Self {
        let separator = separator.into();
        let content = parts
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join(&separator);
        Self {
            role,
            content,
            content_ref,
            trim: Trim::Joined { parts, separator },
        }
    }
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
    pub persona: Option<&'a UserPersona>,
    pub user_id: Snowflake,
    pub state: &'a ReconstructedState,
    pub lorebook_state: Mutex<Option<CapturedState>>,
    /// `None` = the user's active lorebooks; `Some` = exactly these books.
    pub lorebook_ids: Option<&'a [Snowflake]>,
    pub tokenizer: &'a Arc<kitoken::Kitoken>,
    /// Host variables for this turn
    pub variables: Variables,
    pub widget_prompts: &'a [String],
}

/// A single prompt-visible turn, source-agnostic
pub struct PromptTurn {
    pub role: PromptRole,
    pub content: String,
    /// How this turn's content can be recovered when the trace is persisted.
    pub content_ref: ContentRef,
}

impl PromptTurn {
    pub fn from_message(m: &ConversationMessage) -> Self {
        // Widget markers live in stored content but must never reach a prompt.
        let content = if m.content.contains('\u{E000}') {
            let widgets: Vec<WidgetInstance> = m
                .metadata
                .get("widgets")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            sanitize_widget_markers(&m.content, &widgets)
        } else {
            m.content.clone()
        };
        Self {
            role: chat_role_to_prompt_role(&m.role),
            content,
            content_ref: ContentRef::Message(m.id),
        }
    }

    pub fn from_observation(o: &AgentObservation) -> Self {
        Self {
            role: chat_role_to_prompt_role(&o.role),
            content: sanitize_widget_markers(&o.content, &[]),
            content_ref: ContentRef::Verbatim,
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

#[cfg(test)]
mod tests {
    use lorealm_storage::models::ChatRole;
    use serde_json::json;

    use super::*;

    #[test]
    fn message_markers_are_sanitized_for_prompts() {
        let m = ConversationMessage {
            id: Snowflake(1),
            conversation_id: Snowflake(2),
            parent: None,
            role: ChatRole::Assistant,
            content: "here you go: \u{E000}widget:0\u{E001}".into(),
            order: 0,
            metadata: json!({ "widgets": [{ "widget_id": "note", "props": {} }] }),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(
            PromptTurn::from_message(&m).content,
            "here you go: [widget: note]"
        );
    }

    #[test]
    fn observation_markers_are_sanitized_for_prompts() {
        let o = AgentObservation {
            id: Snowflake(1),
            conversation_id: Snowflake(2),
            message_id: Snowflake(3),
            agent_id: Snowflake(4),
            event_id: None,
            role: ChatRole::User,
            content: "Alice: look \u{E000}widget:0\u{E001}".into(),
        };
        assert_eq!(
            PromptTurn::from_observation(&o).content,
            "Alice: look [widget]"
        );
    }
}
