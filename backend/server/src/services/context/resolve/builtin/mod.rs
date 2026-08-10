use std::sync::Arc;

use crate::services::{
    context::{ContentRef, PromptRole, ResolvedBlock, builtin::persona::PersonaResolver},
    state::StateService,
};

use super::SourceRegistry;

mod character;
mod chat_history;
mod persona;
mod widgets;
mod world_info;

pub use character::CharacterResolver;
pub use chat_history::ChatHistoryResolver;
pub use widgets::WidgetPromptsResolver;
pub use world_info::WorldInfoResolver;

pub fn register_builtin_sources(registry: &mut SourceRegistry, state: Arc<StateService>) {
    registry.register(Arc::new(ChatHistoryResolver));
    registry.register(Arc::new(CharacterResolver));
    registry.register(Arc::new(PersonaResolver));
    registry.register(Arc::new(WorldInfoResolver::new(Arc::clone(&state))));
    registry.register(Arc::new(WidgetPromptsResolver));
}

pub(super) fn text_block(
    role: PromptRole,
    content: &str,
    content_ref: ContentRef,
) -> Vec<ResolvedBlock> {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use lorealm_storage::backends::MemoryStorage;

    use crate::{
        errors::ContextError,
        services::context::{
            ContextSource, ResolveContext, ResolvedBlock, SourceDefinition, SourceNodeDefinition,
            SourceType,
        },
    };

    use super::*;

    fn builtin_registry() -> SourceRegistry {
        let state = Arc::new(StateService::new(Arc::new(MemoryStorage::new())));
        let mut registry = SourceRegistry::new();
        register_builtin_sources(&mut registry, state);
        registry
    }

    fn has_node(registry: &SourceRegistry, source_id: &str, node_id: &str) -> bool {
        registry
            .get(source_id)
            .is_some_and(|s| s.definition().nodes.iter().any(|n| n.node_id == node_id))
    }

    #[test]
    fn builtins_are_registered() {
        let registry = builtin_registry();

        assert!(registry.get("chat_history").is_some());
        assert!(registry.get("character").is_some());
        assert!(registry.get("world_info").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn builtin_nodes_are_addressable() {
        let registry = builtin_registry();

        assert!(has_node(&registry, "chat_history", "main"));
        assert!(has_node(&registry, "character", "definition"));
        assert!(has_node(&registry, "widgets", "main"));

        assert!(has_node(&registry, "world_info", "prelude"));
        assert!(has_node(&registry, "world_info", "preamble"));
        assert!(has_node(&registry, "world_info", "backdrop"));
        assert!(has_node(&registry, "world_info", "setting"));
        assert!(has_node(&registry, "world_info", "foreground"));
        assert!(has_node(&registry, "world_info", "coda"));

        assert!(!has_node(&registry, "chat_history", "nonexistent"));
        assert!(!has_node(&registry, "nonexistent", "main"));
    }

    /// The drift the single registry exists to prevent: anything the editor
    /// can list must also be resolvable at assembly time.
    #[test]
    fn every_listed_source_is_resolvable() {
        let registry = builtin_registry();
        let ids: Vec<String> = registry
            .definitions()
            .into_iter()
            .map(|d| d.source_id)
            .collect();

        assert_eq!(ids.len(), 4);
        for id in ["chat_history", "character", "world_info", "widgets"] {
            assert!(ids.contains(&id.to_string()), "missing {id}");
            assert!(registry.get(id).is_some(), "{id} listed but not resolvable");
        }
    }

    struct StubSource;

    #[async_trait]
    impl ContextSource for StubSource {
        fn definition(&self) -> SourceDefinition {
            SourceDefinition {
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
            }
        }

        async fn resolve(
            &self,
            _node_id: &str,
            _settings: &HashMap<String, serde_json::Value>,
            _ctx: &ResolveContext<'_>,
        ) -> Result<Vec<ResolvedBlock>, ContextError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn extension_source_registers_both_faces() {
        let mut registry = builtin_registry();
        registry.register(Arc::new(StubSource));

        assert!(has_node(&registry, "custom_extension", "main"));
        assert!(
            registry
                .definitions()
                .iter()
                .any(|d| d.source_id == "custom_extension")
        );
    }
}
