use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    errors::{ContextError, TokenizerError},
    services::context::{
        SelectOption, SettingDefinition, SettingType, SourceDefinition, SourceNodeDefinition,
        SourceType,
    },
};

use super::super::{ContextSource, ResolveContext, ResolvedBlock};

pub struct ChatHistoryResolver;

#[async_trait]
impl ContextSource for ChatHistoryResolver {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: "chat_history".into(),
            name: "Chat History".into(),
            description: "The conversation history between user and assistant".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![SourceNodeDefinition {
                node_id: "main".into(),
                title: "Chat History".into(),
                description: "Insert the conversation history at this position".into(),
                icon: Some("message-square".into()),
                settings: vec![
                    SettingDefinition {
                        key: "max_tokens".into(),
                        label: "Maximum Tokens".into(),
                        description: "Truncate history beyond this token count (0 = unlimited)"
                            .into(),
                        setting_type: SettingType::TokenCount,
                        default_value: serde_json::json!(0),
                        required: false,
                    },
                    SettingDefinition {
                        key: "truncation_strategy".into(),
                        label: "Truncation Strategy".into(),
                        description: "How to handle truncation when limit is reached".into(),
                        setting_type: SettingType::Select {
                            options: vec![
                                SelectOption {
                                    value: "oldest_first".into(),
                                    label: "Remove oldest messages".into(),
                                },
                                SelectOption {
                                    value: "middle_out".into(),
                                    label: "Remove messages from the middle".into(),
                                },
                            ],
                        },
                        default_value: serde_json::json!("oldest_first"),
                        required: false,
                    },
                ],
            }],
        }
    }

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        debug_assert_eq!(node_id, "main");

        let max_tokens = settings
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let strategy = settings
            .get("truncation_strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("oldest_first");

        let kept: Vec<_> = ctx.messages.iter().collect();

        // Decide which of `kept` to retain. max_tokens == 0 means unlimited.
        let keep: Vec<usize> = if max_tokens == 0 || kept.is_empty() {
            (0..kept.len()).collect()
        } else {
            let cost: Vec<usize> = kept
                .iter()
                .map(|m| {
                    Ok(ctx
                        .tokenizer
                        .encode(m.content.as_str(), false)
                        .map_err(|e| TokenizerError::Tokenize(e.to_string()))?
                        .len())
                })
                .collect::<Result<_, ContextError>>()?;

            match strategy {
                "middle_out" => {
                    let mut remaining = max_tokens;
                    let (mut lo, mut hi) = (0usize, kept.len());
                    let mut front = Vec::new();
                    let mut back = Vec::new();
                    let mut prefer_front = true;
                    while lo < hi {
                        let front_fits = cost[lo] <= remaining;
                        let back_fits = cost[hi - 1] <= remaining;
                        if !front_fits && !back_fits {
                            break;
                        }
                        let take_front = if prefer_front { front_fits } else { !back_fits };
                        if take_front {
                            remaining -= cost[lo];
                            front.push(lo);
                            lo += 1;
                        } else {
                            remaining -= cost[hi - 1];
                            back.push(hi - 1);
                            hi -= 1;
                        }
                        prefer_front = !prefer_front;
                    }
                    back.reverse();
                    front.extend(back);
                    front
                }
                // "oldest_first" (default)
                _ => {
                    let mut remaining = max_tokens;
                    let mut rev = Vec::new();
                    for idx in (0..kept.len()).rev() {
                        if cost[idx] > remaining {
                            break;
                        }
                        remaining -= cost[idx];
                        rev.push(idx);
                    }
                    rev.reverse();
                    rev
                }
            }
        };

        let blocks = keep
            .into_iter()
            .map(|i| {
                let t = kept[i];
                ResolvedBlock {
                    role: t.role.clone(),
                    content: t.content.clone(),
                    content_ref: t.content_ref.clone(),
                }
            })
            .collect();

        Ok(blocks)
    }
}
