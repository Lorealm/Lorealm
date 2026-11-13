use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    errors::ContextError,
    services::context_template::{
        ContentRef, ContextSource, PromptRole, ResolveContext, ResolvedBlock, SourceDefinition,
        SourceNodeDefinition, SourceType, builtin::text_block,
    },
};

pub struct PersonaResolver;

#[async_trait]
impl ContextSource for PersonaResolver {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: "persona".into(),
            name: "Persona".into(),
            description: "Nodes for the user/agent's persona".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![SourceNodeDefinition {
                node_id: "description".into(),
                title: "Persona Description".into(),
                description: "The persona's description".into(),
                icon: None,
                settings: vec![],
            }],
        }
    }

    async fn resolve(
        &self,
        node_id: &str,
        _settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let Some(persona) = ctx.persona else {
            return Ok(Vec::new());
        };

        let blocks = match node_id {
            "description" => {
                if let Some(desc) = &persona.description {
                    text_block(
                        PromptRole::System,
                        desc,
                        ContentRef::Persona(persona.clone()),
                    )
                } else {
                    Vec::new()
                }
            }
            other => {
                return Err(ContextError::UnknownSourceNode {
                    source_id: "persona".into(),
                    node_id: other.into(),
                });
            }
        };

        Ok(blocks)
    }
}
