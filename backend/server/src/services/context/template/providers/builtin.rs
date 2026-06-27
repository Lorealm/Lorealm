use crate::services::context::template::{
    SelectOption, SettingDefinition, SettingType, SourceDefinition, SourceNodeDefinition,
    SourceType,
};

fn chat_history_source() -> SourceDefinition {
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
                    description: "Truncate history beyond this token count (0 = unlimited)".into(),
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

fn world_info_node(id: &str, title: &str, description: &str) -> SourceNodeDefinition {
    SourceNodeDefinition {
        node_id: id.into(),
        title: title.into(),
        description: description.into(),
        icon: Some("book".into()),
        settings: vec![
            SettingDefinition {
                key: "separator".into(),
                label: "Entry Separator".into(),
                description: "Text to insert between entries".into(),
                setting_type: SettingType::String {
                    max_length: Some(100),
                },
                default_value: serde_json::json!("\n---\n"),
                required: false,
            },
            SettingDefinition {
                key: "max_entries".into(),
                label: "Maximum Entries".into(),
                description: "Maximum number of entries to include (0 = unlimited)".into(),
                setting_type: SettingType::Integer {
                    min: Some(0),
                    max: Some(100),
                },
                default_value: serde_json::json!(0),
                required: false,
            },
        ],
    }
}

fn world_info_source() -> SourceDefinition {
    SourceDefinition {
        source_id: "world_info".into(),
        name: "World Info / Lorebook".into(),
        description: "Dynamic world information entries that activate based on keywords".into(),
        source_type: SourceType::BuiltIn,
        nodes: vec![
            world_info_node(
                "prelude",
                "World Info (Prelude)",
                "World info entries placed at the deepest layer of context",
            ),
            world_info_node(
                "preamble",
                "World Info (Preamble)",
                "World info entries placed near the base of context, above the prelude",
            ),
            world_info_node(
                "backdrop",
                "World Info (Backdrop)",
                "World info entries providing background context",
            ),
            world_info_node(
                "setting",
                "World Info (Setting)",
                "World info entries placed in the middle of context",
            ),
            world_info_node(
                "foreground",
                "World Info (Foreground)",
                "World info entries placed near the front of context, closer to recent content",
            ),
            world_info_node(
                "coda",
                "World Info (Coda)",
                "World info entries placed at the most recent layer of context",
            ),
        ],
    }
}

fn character_source() -> SourceDefinition {
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
pub fn register_builtins(mut registry: super::SourceRegistry) -> super::SourceRegistry {
    registry.register(character_source());
    registry.register(chat_history_source());
    registry.register(world_info_source());
    registry
}
