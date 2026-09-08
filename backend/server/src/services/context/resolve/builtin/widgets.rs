use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    errors::ContextError,
    services::context::{
        ContentRef, PromptRole, SourceDefinition, SourceNodeDefinition, SourceType,
    },
};

use super::super::{ContextSource, ResolveContext, ResolvedBlock};

pub struct WidgetPromptsResolver;

#[async_trait]
impl ContextSource for WidgetPromptsResolver {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: "widgets".into(),
            name: "Widgets".into(),
            description: "Prompts injected by widgets".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![SourceNodeDefinition {
                node_id: "main".into(),
                title: "Widget Prompts".into(),
                description: "Insert the active widgets' prompts at this position".into(),
                icon: Some("layout-dashboard".into()),
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

        Ok(ctx
            .widget_prompts
            .iter()
            .filter(|p| !p.trim().is_empty())
            .map(|p| ResolvedBlock::atomic(PromptRole::System, p.clone(), ContentRef::Verbatim))
            .collect())
    }
}
