use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    errors::ContextError,
    services::context_template::{
        ContentRef, ContextSource, PromptRole, ResolveContext, ResolvedBlock, SourceDefinition,
        SourceNodeDefinition, SourceType, builtin::text_block,
    },
};

pub struct AuthorNoteResolver;

#[async_trait]
impl ContextSource for AuthorNoteResolver {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: "author_note".into(),
            name: "Author's Note".into(),
            description: "A note kept alongside the conversation".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![SourceNodeDefinition {
                node_id: "main".into(),
                title: "Author's Note".into(),
                description: "Insert the conversation's author's note at this position".into(),
                icon: Some("notebook-pen".into()),
                settings: Vec::new(),
            }],
        }
    }

    async fn resolve(
        &self,
        node_id: &str,
        _settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        debug_assert_eq!(node_id, "main");

        Ok(text_block(
            PromptRole::System,
            ctx.author_note.trim(),
            ContentRef::Verbatim,
        ))
    }
}
