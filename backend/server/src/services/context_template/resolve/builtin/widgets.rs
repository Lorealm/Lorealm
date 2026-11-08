use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    errors::ContextError,
    services::context_template::{
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
            nodes: vec![
                SourceNodeDefinition {
                    node_id: "main".into(),
                    title: "Widget Prompts".into(),
                    description: "Insert the active widgets' prompts at this position".into(),
                    icon: Some("layout-dashboard".into()),
                    settings: Vec::new(),
                },
                SourceNodeDefinition {
                    node_id: "state".into(),
                    title: "Widget State".into(),
                    description: "Insert what the active widgets currently show, as text".into(),
                    icon: Some("gauge".into()),
                    settings: Vec::new(),
                },
            ],
        }
    }

    async fn resolve(
        &self,
        node_id: &str,
        _settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let texts = match node_id {
            "state" => ctx.widget_renders,
            _ => ctx.widget_prompts,
        };

        Ok(texts
            .iter()
            .filter(|p| !p.trim().is_empty())
            .map(|p| ResolvedBlock::atomic(PromptRole::System, p.clone(), ContentRef::Verbatim))
            .collect())
    }
}
