use std::collections::HashMap;

use async_trait::async_trait;
use eldoria_storage::models::{Character, ExampleDialogue};

use crate::errors::ContextError;

use super::super::{PromptRole, ResolveContext, ResolvedBlock, SourceResolver};

pub struct CharacterResolver;

#[async_trait]
impl SourceResolver for CharacterResolver {
    fn source_id(&self) -> &str {
        "character"
    }

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let character = ctx.character;

        let blocks = match node_id {
            "definition" => text_block(PromptRole::System, &character.definition.description),
            "personality" => text_block(PromptRole::System, &character.definition.personality),
            "scenario" => text_block(PromptRole::System, &character.definition.scenario),

            "first_message" => {
                // Greetings carry an explicit `order`; the Vec isn't guaranteed sorted,
                // so pick the lowest-order one as the primary. (See note on active greeting.)
                match character
                    .definition
                    .greetings
                    .iter()
                    .min_by_key(|g| g.order)
                {
                    Some(g) => text_block(PromptRole::Assistant, &g.text),
                    None => Vec::new(),
                }
            }

            "example_messages" => {
                let max = settings
                    .get("max_examples")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                resolve_examples(&character, max)
            }

            other => {
                return Err(ContextError::UnknownSourceNode {
                    source_id: "character".into(),
                    node_id: other.into(),
                }
                .into());
            }
        };

        Ok(blocks)
    }
}

fn text_block(role: PromptRole, content: &str) -> Vec<ResolvedBlock> {
    if content.is_empty() {
        Vec::new()
    } else {
        vec![ResolvedBlock {
            role,
            content: content.to_string(),
        }]
    }
}

/// Examples sorted by `order`, capped by `max_examples`, empties dropped.
fn resolve_examples(character: &Character, max: Option<usize>) -> Vec<ResolvedBlock> {
    let mut examples: Vec<&ExampleDialogue> =
        character.definition.example_messages.iter().collect();
    examples.sort_by_key(|e| e.order);
    if let Some(n) = max {
        examples.truncate(n);
    }
    examples
        .into_iter()
        .filter(|e| !e.content.is_empty())
        .map(|e| ResolvedBlock {
            role: PromptRole::System, // FLAG: role choice — see below
            content: e.content.clone(),
        })
        .collect()
}
