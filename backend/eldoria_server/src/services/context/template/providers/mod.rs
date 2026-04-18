use std::collections::HashMap;

mod builtin;

/// Registry of all available context sources
#[derive(Debug, Clone)]
pub struct SourceRegistry {
    sources: HashMap<String, super::SourceDefinition>,
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
        registry = builtin::register_builtins(registry);

        registry
    }

    /// Register a source definition
    pub fn register(&mut self, source: super::SourceDefinition) {
        self.sources.insert(source.source_id.clone(), source);
    }

    /// Unregister a source by ID
    pub fn unregister(&mut self, source_id: &str) -> Option<super::SourceDefinition> {
        self.sources.remove(source_id)
    }

    /// Get a source by ID
    pub fn get(&self, source_id: &str) -> Option<&super::SourceDefinition> {
        self.sources.get(source_id)
    }

    /// Get all registered sources
    pub fn all_sources(&self) -> Vec<&super::SourceDefinition> {
        self.sources.values().collect()
    }

    /// Get all sources as owned values (for serialization)
    pub fn all_sources_owned(&self) -> Vec<super::SourceDefinition> {
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
}

#[cfg(test)]
mod tests {
    use crate::services::context::template::{SourceDefinition, SourceNodeDefinition, SourceType};

    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = SourceRegistry::new();

        assert!(registry.has_source("chat_history"));
        assert!(registry.has_source("character"));
        assert!(registry.has_source("world_info"));
        assert!(!registry.has_source("nonexistent"));
    }

    #[test]
    fn test_registry_node_lookup() {
        let registry = SourceRegistry::new();

        assert!(registry.has_node("chat_history", "main"));
        assert!(registry.has_node("character", "definition"));

        assert!(registry.has_node("world_info", "prelude"));
        assert!(registry.has_node("world_info", "preamble"));
        assert!(registry.has_node("world_info", "backdrop"));
        assert!(registry.has_node("world_info", "setting"));
        assert!(registry.has_node("world_info", "foreground"));
        assert!(registry.has_node("world_info", "coda"));

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
