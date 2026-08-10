use std::collections::HashMap;

use async_trait::async_trait;
use lorealm_storage::models::{Character, ExampleDialogue};

use crate::{
    errors::ContextError,
    services::context::{
        CharacterPart, ContentRef, SettingDefinition, SettingType, SourceDefinition,
        SourceNodeDefinition, SourceType, builtin::text_block,
    },
};

use super::super::{ContextSource, PromptRole, ResolveContext, ResolvedBlock};

pub struct CharacterResolver;

#[async_trait]
impl ContextSource for CharacterResolver {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: "character".into(),
            name: "Character".into(),
            description: "Character definition and personality data".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![
                SourceNodeDefinition {
                    node_id: "definition".into(),
                    title: "Character Definition".into(),
                    description: "The main character definition/description".into(),
                    icon: Some("user".into()),
                    settings: vec![],
                },
                SourceNodeDefinition {
                    node_id: "personality".into(),
                    title: "Character Personality".into(),
                    description: "Character personality traits and behavior".into(),
                    icon: Some("heart".into()),
                    settings: vec![],
                },
                SourceNodeDefinition {
                    node_id: "scenario".into(),
                    title: "Scenario".into(),
                    description: "The current roleplay scenario".into(),
                    icon: Some("map".into()),
                    settings: vec![],
                },
                SourceNodeDefinition {
                    node_id: "first_message".into(),
                    title: "First Message".into(),
                    description: "Character's opening message".into(),
                    icon: Some("message-circle".into()),
                    settings: vec![],
                },
                SourceNodeDefinition {
                    node_id: "example_messages".into(),
                    title: "Example Messages".into(),
                    description: "Example conversation messages for style reference".into(),
                    icon: Some("list".into()),
                    settings: vec![SettingDefinition {
                        key: "max_examples".into(),
                        label: "Maximum Examples".into(),
                        description: "Maximum number of example exchanges to include".into(),
                        setting_type: SettingType::Integer {
                            min: Some(0),
                            max: Some(20),
                        },
                        default_value: serde_json::json!(5),
                        required: false,
                    }],
                },
            ],
        }
    }

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let Some(character) = ctx.character else {
            return Ok(Vec::new());
        };

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
                        ContentRef::Character(CharacterPart::Greeting {
                            order: g.order as i64,
                        }),
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
            content_ref: ContentRef::Character(CharacterPart::Example {
                order: e.order as i64,
            }),
        })
        .collect()
}
