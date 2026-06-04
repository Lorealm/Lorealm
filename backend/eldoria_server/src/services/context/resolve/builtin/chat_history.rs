use std::collections::HashMap;

use async_trait::async_trait;
use eldoria_storage::models::ChatRole;

use crate::errors::ContextError;

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

        let blocks = ctx
            .messages
            .iter()
            .filter(|m| include_system || m.role != ChatRole::System)
            .map(|m| ResolvedBlock {
                role: m.role.clone().into(),
                content: m.content.clone(),
            })
            .collect();

        // TODO(max_tokens / truncation_strategy): needs a tokenizer
        Ok(blocks)
    }
}
