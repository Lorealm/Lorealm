use std::collections::HashMap;

use super::types::{
    SelectOption, SettingDefinition, SettingType, SourceDefinition, SourceNodeDefinition,
    SourceType,
};

/// Registry of all available context sources
#[derive(Debug, Clone)]
pub struct SourceRegistry {
    sources: HashMap<String, SourceDefinition>,
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceRegistry {
    /// Create a new registry with built-in sources
    pub fn new() -> Self {
        let mut registry = Self {
            sources: HashMap::new(),
        };

        // Register built-in sources
        registry.register(Self::chat_history_source());
        registry.register(Self::world_info_source());
        registry.register(Self::character_source());

        registry
    }

    /// Register a source definition
    pub fn register(&mut self, source: SourceDefinition) {
        self.sources.insert(source.source_id.clone(), source);
    }

    /// Unregister a source by ID
    pub fn unregister(&mut self, source_id: &str) -> Option<SourceDefinition> {
        self.sources.remove(source_id)
    }

    /// Get a source by ID
    pub fn get(&self, source_id: &str) -> Option<&SourceDefinition> {
        self.sources.get(source_id)
    }

    /// Get all registered sources
    pub fn all_sources(&self) -> Vec<&SourceDefinition> {
        self.sources.values().collect()
    }

    /// Get all sources as owned values (for serialization)
    pub fn all_sources_owned(&self) -> Vec<SourceDefinition> {
        self.sources.values().cloned().collect()
    }

    /// Check if a source exists
    pub fn has_source(&self, source_id: &str) -> bool {
        self.sources.contains_key(source_id)
    }

    /// Check if a specific node exists within a source
    pub fn has_node(&self, source_id: &str, node_id: &str) -> bool {
        self.sources
            .get(source_id)
            .map(|s| s.nodes.iter().any(|n| n.node_id == node_id))
            .unwrap_or(false)
    }

    // ==================== Built-in Source Definitions ====================

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
                                    value: "summarize".into(),
                                    label: "Summarize old messages".into(),
                                },
                            ],
                        },
                        default_value: serde_json::json!("oldest_first"),
                        required: false,
                    },
                    SettingDefinition {
                        key: "include_system".into(),
                        label: "Include System Messages".into(),
                        description: "Whether to include system messages in history".into(),
                        setting_type: SettingType::Boolean,
                        default_value: serde_json::json!(true),
                        required: false,
                    },
                ],
            }],
        }
    }

    fn world_info_source() -> SourceDefinition {
        SourceDefinition {
            source_id: "world_info".into(),
            name: "World Info / Lorebook".into(),
            description: "Dynamic world information entries that activate based on keywords".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![
                SourceNodeDefinition {
                    node_id: "before_chat".into(),
                    title: "World Info (Before Chat)".into(),
                    description: "World info entries configured to appear before chat history"
                        .into(),
                    icon: Some("book".into()),
                    settings: vec![
                        SettingDefinition {
                            key: "separator".into(),
                            label: "Entry Separator".into(),
                            description: "Text to insert between entries".into(),
                            setting_type: SettingType::String { max_length: Some(100) },
                            default_value: serde_json::json!("\n---\n"),
                            required: false,
                        },
                        SettingDefinition {
                            key: "max_entries".into(),
                            label: "Maximum Entries".into(),
                            description: "Maximum number of entries to include (0 = unlimited)"
                                .into(),
                            setting_type: SettingType::Integer {
                                min: Some(0),
                                max: Some(100),
                            },
                            default_value: serde_json::json!(0),
                            required: false,
                        },
                    ],
                },
                SourceNodeDefinition {
                    node_id: "after_chat".into(),
                    title: "World Info (After Chat)".into(),
                    description: "World info entries configured to appear after chat history".into(),
                    icon: Some("book".into()),
                    settings: vec![
                        SettingDefinition {
                            key: "separator".into(),
                            label: "Entry Separator".into(),
                            description: "Text to insert between entries".into(),
                            setting_type: SettingType::String { max_length: Some(100) },
                            default_value: serde_json::json!("\n---\n"),
                            required: false,
                        },
                        SettingDefinition {
                            key: "max_entries".into(),
                            label: "Maximum Entries".into(),
                            description: "Maximum number of entries to include (0 = unlimited)"
                                .into(),
                            setting_type: SettingType::Integer {
                                min: Some(0),
                                max: Some(100),
                            },
                            default_value: serde_json::json!(0),
                            required: false,
                        },
                    ],
                },
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
                    settings: vec![
                        SettingDefinition {
                            key: "max_examples".into(),
                            label: "Maximum Examples".into(),
                            description: "Maximum number of example exchanges to include".into(),
                            setting_type: SettingType::Integer {
                                min: Some(0),
                                max: Some(20),
                            },
                            default_value: serde_json::json!(5),
                            required: false,
                        },
                    ],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = SourceRegistry::new();

        assert!(registry.has_source("chat_history"));
        assert!(registry.has_source("world_info"));
        assert!(registry.has_source("character"));
        assert!(!registry.has_source("nonexistent"));
    }

    #[test]
    fn test_registry_node_lookup() {
        let registry = SourceRegistry::new();

        assert!(registry.has_node("chat_history", "main"));
        assert!(registry.has_node("world_info", "before_chat"));
        assert!(registry.has_node("world_info", "after_chat"));
        assert!(registry.has_node("character", "definition"));

        assert!(!registry.has_node("chat_history", "nonexistent"));
        assert!(!registry.has_node("nonexistent", "main"));
    }

    #[test]
    fn test_register_custom_source() {
        let mut registry = SourceRegistry::new();

        let custom_source = SourceDefinition {
            source_id: "custom_extension".into(),
            name: "Custom Extension".into(),
            description: "A custom source from an extension".into(),
            source_type: SourceType::Extension {
                extension_id: "ext-123".into(),
            },
            nodes: vec![SourceNodeDefinition {
                node_id: "main".into(),
                title: "Custom Output".into(),
                description: "Output from custom extension".into(),
                icon: Some("puzzle".into()),
                settings: vec![],
            }],
        };

        registry.register(custom_source);

        assert!(registry.has_source("custom_extension"));
        assert!(registry.has_node("custom_extension", "main"));
    }

    #[test]
    fn test_unregister_source() {
        let mut registry = SourceRegistry::new();

        let removed = registry.unregister("chat_history");
        assert!(removed.is_some());
        assert!(!registry.has_source("chat_history"));
    }

    #[test]
    fn test_all_sources() {
        let registry = SourceRegistry::new();
        let sources = registry.all_sources();

        assert!(sources.len() >= 3); // At least the built-in ones
    }
}