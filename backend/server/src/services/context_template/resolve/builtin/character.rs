use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use lorealm_storage::models::{Character, ExampleDialogue};
use weaver_lang::CompiledTemplate;

use crate::{
    errors::ContextError,
    services::{
        context_template::{
            CharacterPart, ContentRef, SelectOption, SettingDefinition, SettingType,
            SourceDefinition, SourceNodeDefinition, SourceType, builtin::text_block,
        },
        plugins::UserVocabulary,
        state::{StatePool, write_var},
        variables::Variables,
        weaver::{WeaverHost, WeaverProfile},
    },
    util::weaver::weaver_to_json,
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
                SourceNodeDefinition {
                    node_id: "injection".into(),
                    title: "Character Injection".into(),
                    description: "Your text here, combined with whatever the character's card \
                                  adds to it"
                        .into(),
                    icon: Some("syringe".into()),
                    settings: vec![
                        SettingDefinition {
                            key: "base".into(),
                            label: "Text".into(),
                            description: "Used as-is. A card's injection can include it by \
                                          writing {{template}}."
                                .into(),
                            setting_type: SettingType::Text,
                            default_value: serde_json::json!(""),
                            required: false,
                        },
                        SettingDefinition {
                            key: "mode".into(),
                            label: "Card injection".into(),
                            description: "How the character's injection combines with your text"
                                .into(),
                            setting_type: SettingType::Select {
                                options: [
                                    ("replace", "Replace"),
                                    ("ignore", "Ignore"),
                                    ("prepend", "Prepend"),
                                    ("append", "Append"),
                                ]
                                .into_iter()
                                .map(|(value, label)| SelectOption {
                                    value: value.into(),
                                    label: label.into(),
                                })
                                .collect(),
                            },
                            default_value: serde_json::json!("replace"),
                            required: false,
                        },
                    ],
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

            "injection" => {
                let base = settings.get("base").and_then(|v| v.as_str()).unwrap_or("");
                let mode = settings
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("replace");
                let content = inject(
                    &character.definition.template_injection,
                    base,
                    mode,
                    &ctx.state.persistent,
                    &ctx.variables,
                    ctx.plugins.as_ref(),
                );
                if content.trim().is_empty() {
                    Vec::new()
                } else {
                    text_block(PromptRole::System, &content, ContentRef::Verbatim)
                }
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

const TEMPLATE_MACRO: &str = "{{template}}";
/// Stands in for `{{template}}` while the injection is evaluated
const TEMPLATE_MARK: &str = "\u{E002}";

/// Combine a card's `injection` with the node's `base` text under `mode`.
fn inject(
    injection: &str,
    base: &str,
    mode: &str,
    pool: &StatePool,
    vars: &Variables,
    plugins: Option<&UserVocabulary>,
) -> String {
    if injection.trim().is_empty() {
        return base.to_string();
    }
    let source = match mode {
        "ignore" => return base.to_string(),
        "prepend" => format!("{injection}\n\n{TEMPLATE_MACRO}"),
        "append" => format!("{TEMPLATE_MACRO}\n\n{injection}"),
        _ => injection.to_string(),
    };
    let source = source.replace(TEMPLATE_MACRO, TEMPLATE_MARK);

    // Seed host variables (`{{char:name}}`, ...) the way lorebooks see them.
    let mut pool = pool.clone();
    for (scope, values) in vars.iter() {
        let ns = pool.entry(scope.to_string()).or_default();
        for (name, value) in values {
            if let Ok(json) = weaver_to_json(value) {
                write_var(ns, name, &json);
            }
        }
    }

    let rendered = CompiledTemplate::compile(&source)
        .map_err(|e| format!("{e:?}"))
        .and_then(|body| {
            WeaverHost::builder()
                .profile(WeaverProfile::WidgetRender)
                .pool(Arc::new(Mutex::new(pool)))
                .plugins(plugins.cloned())
                .build()
                .evaluate_inline(body)
                .map_err(|e| e.to_string())
        });
    match rendered {
        Ok(outcome) => outcome.to_output_string().replace(TEMPLATE_MARK, base),
        Err(e) => {
            tracing::warn!("character injection failed to evaluate: {e}");
            base.to_string()
        }
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
        .map(|e| {
            ResolvedBlock::atomic(
                PromptRole::System,
                e.content.clone(),
                ContentRef::Character(CharacterPart::Example {
                    order: e.order as i64,
                }),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn pool() -> StatePool {
        StatePool::from_iter([(
            "story".to_string(),
            HashMap::from([("traits".to_string(), json!("brave"))]),
        )])
    }

    fn run(injection: &str, mode: &str) -> String {
        inject(
            injection,
            "BASE",
            mode,
            &pool(),
            &Variables::default(),
            None,
        )
    }

    #[test]
    fn replace_substitutes_the_base_and_reads_state() {
        assert_eq!(
            run("{{template}} | Traits: {{story:traits}}", "replace"),
            "BASE | Traits: brave"
        );
    }

    #[test]
    fn replace_without_the_macro_drops_the_base() {
        assert_eq!(run("Traits: {{story:traits}}", "replace"), "Traits: brave");
    }

    #[test]
    fn prepend_and_append_place_the_injection_around_the_base() {
        assert_eq!(run("T: {{story:traits}}", "prepend"), "T: brave\n\nBASE");
        assert_eq!(run("T: {{story:traits}}", "append"), "BASE\n\nT: brave");
    }

    #[test]
    fn ignore_keeps_only_the_base() {
        assert_eq!(run("T: {{story:traits}}", "ignore"), "BASE");
    }

    #[test]
    fn an_unknown_mode_behaves_as_replace() {
        assert_eq!(run("x {{template}}", "nonsense"), "x BASE");
    }

    #[test]
    fn no_injection_leaves_the_base_in_every_mode() {
        for mode in ["replace", "ignore", "prepend", "append"] {
            assert_eq!(run("  ", mode), "BASE");
        }
    }

    #[test]
    fn the_base_is_never_evaluated() {
        let out = inject(
            "{{template}}",
            "keep {{story:traits}} $[set_var(\"a:b\", 1)]",
            "replace",
            &pool(),
            &Variables::default(),
            None,
        );
        assert_eq!(out, "keep {{story:traits}} $[set_var(\"a:b\", 1)]");
    }

    #[test]
    fn control_flow_works_in_the_injection() {
        let pool = StatePool::from_iter([(
            "story".to_string(),
            HashMap::from([
                ("traits".to_string(), json!(["brave", "curious"])),
                ("cursed".to_string(), json!(true)),
            ]),
        )]);
        let injection = "{{template}}\n\
            {# foreach t in {{story:traits}} #}- {{t}}\n{# endforeach #}\
            {# if {{story:cursed}} #}Cursed.{# endif #}\
            {# if $[var_exists(\"story:missing\")] #}never{# endif #}";
        let out = inject(
            injection,
            "BASE",
            "replace",
            &pool,
            &Variables::default(),
            None,
        );
        assert_eq!(out.matches("- brave").count(), 1, "{out:?}");
        assert!(out.contains("- curious"), "{out:?}");
        assert!(out.contains("Cursed."), "{out:?}");
        assert!(!out.contains("never"), "{out:?}");
        assert!(out.starts_with("BASE"), "{out:?}");
    }

    /// Reading a variable that was never written is an evaluation error, so
    /// the whole injection falls back. Guard optional reads with `var_exists`.
    #[test]
    fn an_unset_variable_falls_back_unless_guarded() {
        assert_eq!(run("{{story:missing}} hi", "replace"), "BASE");
        assert_eq!(
            run(
                "{# if $[var_exists(\"story:missing\")] #}{{story:missing}}{# endif #}hi",
                "replace"
            ),
            "hi"
        );
    }

    #[test]
    fn local_scratch_is_writable_but_stored_state_is_not() {
        assert_eq!(run("$[set_var(\"local:n\", 2)]{{local:n}}", "replace"), "2");
        // A write to stored state is refused, so the injection falls back.
        assert_eq!(run("$[set_var(\"story:x\", 1)]hi", "replace"), "BASE");
    }

    #[test]
    fn a_broken_injection_falls_back_to_the_base() {
        assert_eq!(run("$[unclosed(", "replace"), "BASE");
    }

    #[test]
    fn host_variables_resolve_inside_the_injection() {
        let vars = Variables::from_map(HashMap::from([(
            "char".to_string(),
            HashMap::from([(
                "name".to_string(),
                weaver_lang::Value::String("Aria".into()),
            )]),
        )]));
        assert_eq!(
            inject(
                "{{template}} with {{char:name}}",
                "BASE",
                "replace",
                &pool(),
                &vars,
                None
            ),
            "BASE with Aria"
        );
    }
}
