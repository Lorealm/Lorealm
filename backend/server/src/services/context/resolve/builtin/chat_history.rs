use std::collections::HashMap;

use async_trait::async_trait;
use lorealm_storage::models::ChatRole;

use crate::{errors::ContextError, services::context::ContentRef};

use super::super::{ResolveContext, ResolvedBlock, SourceResolver};

pub struct ChatHistoryResolver;

#[async_trait]
impl SourceResolver for ChatHistoryResolver {
    fn source_id(&self) -> &str {
        "chat_history"
    }

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        debug_assert_eq!(node_id, "main");

        let include_system = settings
            .get("include_system")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Temporary: cap history by message count until token-based truncation lands.
        const MAX_MESSAGES: usize = 20;

        let kept: Vec<_> = ctx
            .messages
            .iter()
            .filter(|m| include_system || m.role != ChatRole::System)
            .collect();

        let start = kept.len().saturating_sub(MAX_MESSAGES);

        let blocks = kept[start..]
            .iter()
            .map(|m| ResolvedBlock {
                role: m.role.clone().into(),
                content: m.content.clone(),
                content_ref: ContentRef::Message(m.id),
            })
            .collect();

        // TODO(max_tokens / truncation_strategy): needs a tokenizer
        Ok(blocks)
    }
}
