use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use context_weaver::{
    AssembledBlock, ChatMessage as CwMessage, ContextWeaver, NamespaceAccess, Slot,
};
use eldoria_storage::models::{ChatRole, ConversationMessage, Snowflake};

use crate::{
    errors::ContextError,
    services::{context::to_weaver_lorebook, state::StateService},
};

use super::super::{PromptRole, ResolveContext, ResolvedBlock, SourceResolver};

pub struct WorldInfoResolver {
    state: Arc<StateService>,
}

impl WorldInfoResolver {
    pub fn new(state: Arc<StateService>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SourceResolver for WorldInfoResolver {
    fn source_id(&self) -> &str {
        "world_info"
    }

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let slot = slot_for_node(node_id)?;

        // Single pipeline run, shared across all six world_info nodes this turn.
        let blocks = self.run_pipeline(ctx.user_id, ctx.messages).await?;

        let separator = settings
            .get("separator")
            .and_then(|v| v.as_str())
            .unwrap_or("\n---\n");
        let max_entries = settings
            .get("max_entries")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let mut entries: Vec<&str> = blocks
            .iter()
            .filter(|b| b.slot == slot)
            .map(|b| b.content.as_str())
            .collect();

        if let Some(n) = max_entries {
            if n > 0 {
                entries.truncate(n); // 0 = unlimited, per the SettingDefinition
            }
        }

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![ResolvedBlock {
            role: PromptRole::System, // lorebooks have no role concept
            content: entries.join(separator),
        }])
    }
}

impl WorldInfoResolver {
    async fn run_pipeline(
        &self,
        user_id: Snowflake,
        messages: &[ConversationMessage],
    ) -> Result<Vec<AssembledBlock>, ContextError> {
        let lorebooks = self
            .state
            .get_active_lorebooks(user_id)
            .await?
            .unwrap_or_default();

        // Init with the first book, stack the rest. No active books -> every slot empty.
        let mut iter = lorebooks.into_iter();
        let Some(first) = iter.next() else {
            return Ok(Vec::new());
        };
        let mut weaver = ContextWeaver::new(to_weaver_lorebook(first)?);
        for book in iter {
            weaver.add_lorebook(to_weaver_lorebook(book)?);
        }

        // TODO(namespace-service): namespace reservation + seeding is hardcoded.
        // This needs a dedicated service to feed `char`/`user` (and likely active
        // state) from the real active character + user, replacing these literals.
        weaver.reserve_namespace("char", NamespaceAccess::ReadOnly);
        weaver.reserve_namespace("user", NamespaceAccess::ReadOnly);
        weaver.set_variable("char", "name", "Aria");
        weaver.set_variable("user", "name", "Player");

        let cw_messages = convert_messages(messages);
        let blocks = weaver.assemble(&cw_messages)?;
        Ok(blocks)
    }
}

fn slot_for_node(node_id: &str) -> Result<Slot, ContextError> {
    let slot = match node_id {
        "prelude" => Slot::Prelude,
        "preamble" => Slot::Preamble,
        "backdrop" => Slot::Backdrop,
        "setting" => Slot::Setting,
        "foreground" => Slot::Foreground,
        "coda" => Slot::Coda,
        other => {
            return Err(ContextError::UnknownSourceNode {
                source_id: "world_info".into(),
                node_id: other.into(),
            });
        }
    };
    Ok(slot)
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<CwMessage> {
    messages
        .iter()
        .map(|m| match m.role {
            ChatRole::User => CwMessage::user(&m.content),
            ChatRole::Assistant => CwMessage::assistant(&m.content),
            ChatRole::System => CwMessage::system(&m.content),
        })
        .collect()
}
