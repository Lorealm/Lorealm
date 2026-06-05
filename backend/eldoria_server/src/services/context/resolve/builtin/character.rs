use std::collections::HashMap;

use async_trait::async_trait;
use eldoria_storage::models::{Character, ExampleDialogue};

use crate::{errors::ContextError, services::context::{CharacterPart, ContentRef}};

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
            "definition" => text_block(
                PromptRole::System,
                &character.definition.description,
                ContentRef::Character(CharacterPart::Definition),
            ),
            "personality" => text_block(
                PromptRole::System,
                &character.definition.personality,
                ContentRef::Character(CharacterPart::Personality),
            ),
            "scenario" => text_block(
                PromptRole::System,
                &character.definition.scenario,
                ContentRef::Character(CharacterPart::Scenario),
            ),

            "first_message" => {
                match character
                    .definition
                    .greetings
                    .iter()
                    .min_by_key(|g| g.order)
                {
                    Some(g) => text_block(
                        PromptRole::Assistant,
                        &g.text,
                        ContentRef::Character(CharacterPart::Greeting { order: g.order as i64 }),
                    ),
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

fn text_block(role: PromptRole, content: &str, content_ref: ContentRef) -> Vec<ResolvedBlock> {
    if content.is_empty() {
        Vec::new()
    } else {
        vec![ResolvedBlock {
            role,
            content: content.to_string(),
            content_ref,
        }]
    }
}

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
            role: PromptRole::System,
            content: e.content.clone(),
            content_ref: ContentRef::Character(CharacterPart::Example { order: e.order as i64 }),
        })
        .collect()
}
