#[cfg(test)] use std::collections::HashMap;
#[cfg(test)] use std::sync::Arc;

#[cfg(test)] use async_trait::async_trait;
#[cfg(test)] use tokio::sync::RwLock;
#[cfg(test)] use uuid::Uuid;

#[cfg(test)]
use storage_system::{
    CharacterStorage, ConversationStorage, MessageStorage, StorageCore, StorageError, Template, TemplateMetadata, TemplateStorage, UserStorage
};

#[cfg(test)] use crate::models::TemplateService;
#[cfg(test)]
use crate::services::context::{
    ContextNode, ContextTemplate, NodeContent, PromptRole, SourceRegistry,
};

// ==================== Mock Storage Implementation ====================

/// Mock storage backend for testing
#[cfg(test)]
#[derive(Debug, Default)]
pub struct MockTemplateStorage {
    templates: RwLock<HashMap<Uuid, Template>>,
}

#[cfg(test)]
impl MockTemplateStorage {
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl StorageCore for MockTemplateStorage {
    async fn is_available(&self) -> bool {
        true
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        Ok(())
    }

    fn backend_type(&self) -> &str {
        "mock"
    }
}

#[cfg(test)] impl ConversationStorage for MockTemplateStorage {}
#[cfg(test)] impl MessageStorage for MockTemplateStorage {}
#[cfg(test)] impl UserStorage for MockTemplateStorage {}
#[cfg(test)] impl CharacterStorage for MockTemplateStorage {}

#[cfg(test)]
#[async_trait]
impl TemplateStorage for MockTemplateStorage {
    async fn save_template(&self, template: &Template) -> Result<Uuid, StorageError> {
        let mut templates = self.templates.write().await;
        templates.insert(template.id, template.clone());
        Ok(template.id)
    }

    async fn get_template(&self, id: Uuid) -> Result<Option<Template>, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.get(&id).cloned())
    }

    async fn get_template_owned(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Template>, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates
            .get(&id)
            .filter(|t| t.owner_id == owner_id)
            .cloned())
    }

    async fn list_templates(&self, owner_id: Uuid) -> Result<Vec<TemplateMetadata>, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates
            .values()
            .filter(|t| t.owner_id == owner_id)
            .map(|t| {
                // Count nodes from JSON (simplified)
                let node_count = t.nodes_json.matches("\"id\"").count();
                TemplateMetadata {
                    id: t.id,
                    owner_id: t.owner_id,
                    name: t.name.clone(),
                    description: t.description.clone(),
                    node_count,
                    version: t.version,
                    created_at: t.created_at,
                    updated_at: t.updated_at,
                }
            })
            .collect())
    }

    async fn delete_template(&self, id: Uuid, owner_id: Uuid) -> Result<(), StorageError> {
        let mut templates = self.templates.write().await;
        if let Some(template) = templates.get(&id) {
            if template.owner_id != owner_id {
                return Err(StorageError::InvalidOperation(
                    "Access denied".to_string(),
                ));
            }
            templates.remove(&id);
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("Template {} not found", id)))
        }
    }

    async fn template_exists(&self, id: Uuid) -> Result<bool, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.contains_key(&id))
    }

    async fn template_name_exists(
        &self,
        owner_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.values().any(|t| {
            t.owner_id == owner_id
                && t.name == name
                && exclude_id.map_or(true, |ex| t.id != ex)
        }))
    }
}

// ==================== Test Helpers ====================
#[cfg(test)]
fn create_test_service() -> TemplateService<MockTemplateStorage> {
    TemplateService::new(Arc::new(MockTemplateStorage::new()))
}

#[cfg(test)]
fn create_test_user_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
}

#[cfg(test)]
fn create_other_user_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
}

// ==================== TemplateService Tests ====================

#[cfg(test)]
mod template_service_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_template() {
        let service = create_test_service();
        let user_id = create_test_user_id();

        let template = ContextTemplate::new("Test Template", user_id);
        let template_id = template.id;

        let result = service.create_template(template).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, template_id);
        assert_eq!(created.name, "Test Template");
        assert_eq!(created.owner_id, user_id);
    }

    #[tokio::test]
    async fn test_get_template_owned() {
        let service = create_test_service();
        let user_id = create_test_user_id();

        let template = ContextTemplate::new("Test Template", user_id);
        let template_id = template.id;

        service.create_template(template).await.unwrap();

        let result = service.get_template_owned(template_id, user_id).await;
        assert!(result.is_ok());

        let retrieved = result.unwrap();
        assert_eq!(retrieved.id, template_id);
        assert_eq!(retrieved.name, "Test Template");
    }

    #[tokio::test]
    async fn test_get_template_wrong_owner() {
        let service = create_test_service();
        let user_id = create_test_user_id();
        let other_user = create_other_user_id();

        let template = ContextTemplate::new("Test Template", user_id);
        let template_id = template.id;

        service.create_template(template).await.unwrap();

        let result = service.get_template_owned(template_id, other_user).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_template_not_found() {
        let service = create_test_service();
        let user_id = create_test_user_id();
        let fake_id = Uuid::new_v4();

        let result = service.get_template_owned(fake_id, user_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_templates() {
        let service = create_test_service();
        let user_id = create_test_user_id();
        let other_user = create_other_user_id();

        // Create templates for user
        service
            .create_template(ContextTemplate::new("Template 1", user_id))
            .await
            .unwrap();
        service
            .create_template(ContextTemplate::new("Template 2", user_id))
            .await
            .unwrap();

        // Create template for other user
        service
            .create_template(ContextTemplate::new("Other Template", other_user))
            .await
            .unwrap();

        let user_templates = service.list_templates(user_id).await.unwrap();
        assert_eq!(user_templates.len(), 2);

        let other_templates = service.list_templates(other_user).await.unwrap();
        assert_eq!(other_templates.len(), 1);
    }

    #[tokio::test]
    async fn test_update_template() {
        let service = create_test_service();
        let user_id = create_test_user_id();

        let mut template = ContextTemplate::new("Original Name", user_id);
        let template_id = template.id;

        service.create_template(template.clone()).await.unwrap();

        template.name = "Updated Name".to_string();
        template.description = "New description".to_string();

        let updated = service.update_template(template, user_id).await.unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, "New description");

        // Verify persistence
        let retrieved = service.get_template_owned(template_id, user_id).await.unwrap();
        assert_eq!(retrieved.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_delete_template() {
        let service = create_test_service();
        let user_id = create_test_user_id();

        let template = ContextTemplate::new("To Delete", user_id);
        let template_id = template.id;

        service.create_template(template).await.unwrap();

        // Verify it exists
        assert!(service.template_exists(template_id).await.unwrap());

        // Delete it
        let result = service.delete_template(template_id, user_id).await;
        assert!(result.is_ok());

        // Verify it's gone
        assert!(!service.template_exists(template_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_template_wrong_owner() {
        let service = create_test_service();
        let user_id = create_test_user_id();
        let other_user = create_other_user_id();

        let template = ContextTemplate::new("Protected", user_id);
        let template_id = template.id;

        service.create_template(template).await.unwrap();

        let result = service.delete_template(template_id, other_user).await;
        assert!(result.is_err());

        // Verify it still exists
        assert!(service.template_exists(template_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_duplicate_name_rejected() {
        let service = create_test_service();
        let user_id = create_test_user_id();

        service
            .create_template(ContextTemplate::new("Same Name", user_id))
            .await
            .unwrap();

        let result = service
            .create_template(ContextTemplate::new("Same Name", user_id))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_same_name_different_owners_allowed() {
        let service = create_test_service();
        let user_id = create_test_user_id();
        let other_user = create_other_user_id();

        service
            .create_template(ContextTemplate::new("Same Name", user_id))
            .await
            .unwrap();

        let result = service
            .create_template(ContextTemplate::new("Same Name", other_user))
            .await;

        assert!(result.is_ok());
    }
}

// ==================== ContextTemplate Node Operation Tests ====================
#[cfg(test)]
mod template_node_tests {
    use super::*;

    #[test]
    fn test_add_node_to_root() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let node = ContextNode::new_text("System", "Hello", Some(PromptRole::System));
        let node_id = node.id;

        let path = template.add_node(node, None, None).unwrap();
        assert_eq!(path, vec![0]);
        assert_eq!(template.nodes.len(), 1);
        assert_eq!(template.nodes[0].id, node_id);
    }

    #[test]
    fn test_add_node_at_specific_index() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        // Add two nodes
        template
            .add_node(ContextNode::new_text("First", "1", None), None, None)
            .unwrap();
        template
            .add_node(ContextNode::new_text("Third", "3", None), None, None)
            .unwrap();

        // Insert at index 1
        let middle = ContextNode::new_text("Second", "2", None);
        let middle_id = middle.id;
        let path = template.add_node(middle, None, Some(1)).unwrap();

        assert_eq!(path, vec![1]);
        assert_eq!(template.nodes[1].id, middle_id);
        assert_eq!(template.nodes[1].title, "Second");
    }

    #[test]
    fn test_add_node_to_group() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        // Add a group
        let group = ContextNode::new_group("My Group");
        template.add_node(group, None, None).unwrap();

        // Add a node to the group
        let child = ContextNode::new_text("Child", "Content", None);
        let child_id = child.id;
        let path = template.add_node(child, Some(&vec![0]), None).unwrap();

        assert_eq!(path, vec![0, 0]);

        if let NodeContent::Group { children, .. } = &template.nodes[0].content {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].id, child_id);
        } else {
            panic!("Expected group node");
        }
    }

    #[test]
    fn test_add_node_to_nested_group() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        // Create nested structure: root -> group1 -> group2
        let group1 = ContextNode::new_group("Group 1");
        template.add_node(group1, None, None).unwrap();

        let group2 = ContextNode::new_group("Group 2");
        template.add_node(group2, Some(&vec![0]), None).unwrap();

        // Add node to group2
        let deep_node = ContextNode::new_text("Deep", "Content", None);
        let deep_id = deep_node.id;
        let path = template.add_node(deep_node, Some(&vec![0, 0]), None).unwrap();

        assert_eq!(path, vec![0, 0, 0]);

        // Verify structure
        if let NodeContent::Group { children: g1_children, .. } = &template.nodes[0].content {
            if let NodeContent::Group { children: g2_children, .. } = &g1_children[0].content {
                assert_eq!(g2_children.len(), 1);
                assert_eq!(g2_children[0].id, deep_id);
            } else {
                panic!("Expected nested group");
            }
        } else {
            panic!("Expected group");
        }
    }

    #[test]
    fn test_remove_node() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let node = ContextNode::new_text("To Remove", "Content", None);
        let node_id = node.id;

        template.add_node(node, None, None).unwrap();
        assert_eq!(template.nodes.len(), 1);

        let removed = template.remove_node(node_id).unwrap();
        assert_eq!(removed.id, node_id);
        assert!(template.nodes.is_empty());
    }

    #[test]
    fn test_remove_node_from_group() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let group = ContextNode::new_group("Group");
        template.add_node(group, None, None).unwrap();

        let child = ContextNode::new_text("Child", "Content", None);
        let child_id = child.id;
        template.add_node(child, Some(&vec![0]), None).unwrap();

        template.remove_node(child_id).unwrap();

        if let NodeContent::Group { children, .. } = &template.nodes[0].content {
            assert!(children.is_empty());
        } else {
            panic!("Expected group");
        }
    }

    #[test]
    fn test_remove_node_not_found() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let result = template.remove_node(Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn test_move_node_reorder() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let node1 = ContextNode::new_text("First", "1", None);
        let node1_id = node1.id;
        let node2 = ContextNode::new_text("Second", "2", None);
        let node2_id = node2.id;

        template.add_node(node1, None, None).unwrap();
        template.add_node(node2, None, None).unwrap();

        // Move first node to end
        template.move_node(node1_id, None, 1).unwrap();

        assert_eq!(template.nodes[0].id, node2_id);
        assert_eq!(template.nodes[1].id, node1_id);
    }

    #[test]
    fn test_move_node_into_group() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        // Add group first at index 0
        let group = ContextNode::new_group("Group");
        let group_id = group.id;
        template.add_node(group, None, None).unwrap();

        // Verify group is at index 0
        assert_eq!(template.nodes.len(), 1);
        assert_eq!(template.nodes[0].id, group_id);

        // Add text at index 1
        let text = ContextNode::new_text("Text", "Content", None);
        let text_id = text.id;
        template.add_node(text, None, None).unwrap();

        // Verify we have 2 nodes: group at 0, text at 1
        assert_eq!(template.nodes.len(), 2);
        assert_eq!(template.nodes[0].id, group_id);
        assert_eq!(template.nodes[1].id, text_id);

        // Move text (at index 1) into group (at index 0)
        // After text is removed from root, group is still at index 0
        let result = template.move_node(text_id, Some(&vec![0]), 0);
        assert!(result.is_ok(), "move_node failed: {:?}", result.err());

        // Root should only have the group now
        assert_eq!(template.nodes.len(), 1, "Expected 1 node at root, got {}", template.nodes.len());
        assert_eq!(template.nodes[0].id, group_id);

        if let NodeContent::Group { children, .. } = &template.nodes[0].content {
            assert_eq!(children.len(), 1, "Expected 1 child in group, got {}", children.len());
            assert_eq!(children[0].id, text_id);
        } else {
            panic!("Expected group at index 0");
        }
    }

    #[test]
    fn test_move_node_out_of_group() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let mut group = ContextNode::new_group("Group");
        if let NodeContent::Group { children, .. } = &mut group.content {
            let child = ContextNode::new_text("Child", "Content", None);
            children.push(child);
        }
        let child_id = if let NodeContent::Group { children, .. } = &group.content {
            children[0].id
        } else {
            panic!()
        };

        template.add_node(group, None, None).unwrap();

        // Move child to root
        template.move_node(child_id, None, 1).unwrap();

        assert_eq!(template.nodes.len(), 2);
        assert_eq!(template.nodes[1].id, child_id);

        if let NodeContent::Group { children, .. } = &template.nodes[0].content {
            assert!(children.is_empty());
        }
    }

    #[test]
    fn test_toggle_node() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let node = ContextNode::new_text("Test", "Content", None);
        let node_id = node.id;
        template.add_node(node, None, None).unwrap();

        assert!(template.nodes[0].enabled);

        let new_state = template.toggle_node(node_id).unwrap();
        assert!(!new_state);
        assert!(!template.nodes[0].enabled);

        let new_state = template.toggle_node(node_id).unwrap();
        assert!(new_state);
        assert!(template.nodes[0].enabled);
    }

    #[test]
    fn test_update_node() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let node = ContextNode::new_text("Original", "Content", None);
        let node_id = node.id;
        template.add_node(node, None, None).unwrap();

        template
            .update_node(
                node_id,
                Some("Updated Title".to_string()),
                Some("New description".to_string()),
                Some(false),
                None,
            )
            .unwrap();

        assert_eq!(template.nodes[0].title, "Updated Title");
        assert_eq!(template.nodes[0].description, "New description");
        assert!(!template.nodes[0].enabled);
    }

    #[test]
    fn test_find_node_path() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let node1 = ContextNode::new_text("Root", "Content", None);
        let node1_id = node1.id;
        template.add_node(node1, None, None).unwrap();

        let group = ContextNode::new_group("Group");
        template.add_node(group, None, None).unwrap();

        let nested = ContextNode::new_text("Nested", "Content", None);
        let nested_id = nested.id;
        template.add_node(nested, Some(&vec![1]), None).unwrap();

        assert_eq!(template.find_node_path(node1_id), Some(vec![0]));
        assert_eq!(template.find_node_path(nested_id), Some(vec![1, 0]));
        assert_eq!(template.find_node_path(Uuid::new_v4()), None);
    }

    #[test]
    fn test_node_count() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        assert_eq!(template.node_count(), 0);

        template
            .add_node(ContextNode::new_text("1", "", None), None, None)
            .unwrap();
        assert_eq!(template.node_count(), 1);

        let group = ContextNode::new_group("Group");
        template.add_node(group, None, None).unwrap();
        assert_eq!(template.node_count(), 2);

        template
            .add_node(ContextNode::new_text("Nested", "", None), Some(&vec![1]), None)
            .unwrap();
        assert_eq!(template.node_count(), 3);
    }
}

// ==================== Source Node Tests ====================
#[cfg(test)]
mod source_node_tests {
    use super::*;

    #[test]
    fn test_duplicate_source_node_rejected() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let source1 = ContextNode::new_source("Chat History", "chat_history", "main");
        template.add_node(source1, None, None).unwrap();

        let source2 = ContextNode::new_source("Chat History 2", "chat_history", "main");
        let result = template.add_node(source2, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_different_source_nodes_allowed() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        let source1 = ContextNode::new_source("Chat", "chat_history", "main");
        let source2 = ContextNode::new_source("World Info", "world_info", "before_chat");

        template.add_node(source1, None, None).unwrap();
        let result = template.add_node(source2, None, None);

        assert!(result.is_ok());
        assert_eq!(template.nodes.len(), 2);
    }

    #[test]
    fn test_get_used_source_nodes() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        template
            .add_node(
                ContextNode::new_source("Chat", "chat_history", "main"),
                None,
                None,
            )
            .unwrap();

        let group = ContextNode::new_group("Group");
        template.add_node(group, None, None).unwrap();

        template
            .add_node(
                ContextNode::new_source("World Info", "world_info", "before_chat"),
                Some(&vec![1]),
                None,
            )
            .unwrap();

        let used = template.get_used_source_nodes();
        assert_eq!(used.len(), 2);
        assert!(used.contains(&("chat_history".to_string(), "main".to_string())));
        assert!(used.contains(&("world_info".to_string(), "before_chat".to_string())));
    }

    #[test]
    fn test_is_source_node_used() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);

        template
            .add_node(
                ContextNode::new_source("Chat", "chat_history", "main"),
                None,
                None,
            )
            .unwrap();

        assert!(template.is_source_node_used("chat_history", "main"));
        assert!(!template.is_source_node_used("chat_history", "other"));
        assert!(!template.is_source_node_used("world_info", "main"));
    }
}

// ==================== Validation Tests ====================
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validate_empty_template() {
        let user_id = create_test_user_id();
        let template = ContextTemplate::new("Test", user_id);
        let registry = SourceRegistry::new();

        let result = template.validate(&registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_valid_sources() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);
        let registry = SourceRegistry::new();

        template
            .add_node(
                ContextNode::new_source("Chat", "chat_history", "main"),
                None,
                None,
            )
            .unwrap();

        let result = template.validate(&registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_unknown_source() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);
        let registry = SourceRegistry::new();

        template
            .add_node(
                ContextNode::new_source("Unknown", "nonexistent_source", "main"),
                None,
                None,
            )
            .unwrap();

        let result = template.validate(&registry);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_with_unknown_node_in_source() {
        let user_id = create_test_user_id();
        let mut template = ContextTemplate::new("Test", user_id);
        let registry = SourceRegistry::new();

        template
            .add_node(
                ContextNode::new_source("Chat", "chat_history", "nonexistent_node"),
                None,
                None,
            )
            .unwrap();

        let result = template.validate(&registry);
        assert!(result.is_err());
    }
}

// ==================== Serialization Tests ====================
#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn test_node_content_roundtrip() {
        let contents = vec![
            NodeContent::Text {
                content: "Hello".to_string(),
                role: Some(PromptRole::System),
            },
            NodeContent::Source {
                source_id: "chat_history".to_string(),
                node_id: "main".to_string(),
                settings: {
                    let mut map = HashMap::new();
                    map.insert("max_tokens".to_string(), serde_json::json!(2000));
                    map
                },
            },
            NodeContent::Group {
                children: vec![],
                prefix: Some("[start]".to_string()),
                suffix: Some("[end]".to_string()),
                always_include_wrappers: true,
            },
        ];

        for content in contents {
            let json = serde_json::to_string(&content).unwrap();
            let deserialized: NodeContent = serde_json::from_str(&json).unwrap();
            assert_eq!(content, deserialized);
        }
    }

    #[test]
    fn test_context_node_roundtrip() {
        let node = ContextNode {
            id: Uuid::new_v4(),
            title: "Test Node".to_string(),
            description: "A test".to_string(),
            enabled: true,
            content: NodeContent::Text {
                content: "Hello".to_string(),
                role: Some(PromptRole::User),
            },
        };

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: ContextNode = serde_json::from_str(&json).unwrap();

        assert_eq!(node.id, deserialized.id);
        assert_eq!(node.title, deserialized.title);
        assert_eq!(node.content, deserialized.content);
    }

    #[test]
    fn test_nested_nodes_roundtrip() {
        let mut group = ContextNode::new_group("Parent");
        if let NodeContent::Group { children, prefix, suffix, .. } = &mut group.content {
            *prefix = Some("<group>".to_string());
            *suffix = Some("</group>".to_string());
            children.push(ContextNode::new_text("Child 1", "Content 1", None));
            children.push(ContextNode::new_source("Child 2", "chat_history", "main"));
        }

        let nodes = vec![group];
        let json = serde_json::to_string(&nodes).unwrap();
        let deserialized: Vec<ContextNode> = serde_json::from_str(&json).unwrap();

        assert_eq!(nodes.len(), deserialized.len());

        if let NodeContent::Group { children, .. } = &deserialized[0].content {
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected group");
        }
    }
}

// ==================== Source Registry Tests ====================
#[cfg(test)]
mod source_registry_tests {
    use super::*;

    #[test]
    fn test_registry_has_builtin_sources() {
        let registry = SourceRegistry::new();

        assert!(registry.has_source("chat_history"));
        assert!(registry.has_source("world_info"));
        assert!(registry.has_source("character"));
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
        use crate::services::context::{SourceDefinition, SourceNodeDefinition, SourceType};

        let mut registry = SourceRegistry::new();

        let custom = SourceDefinition {
            source_id: "custom".to_string(),
            name: "Custom Source".to_string(),
            description: "A custom source".to_string(),
            source_type: SourceType::Extension {
                extension_id: "ext-1".to_string(),
            },
            nodes: vec![SourceNodeDefinition {
                node_id: "output".to_string(),
                title: "Custom Output".to_string(),
                description: "Custom output".to_string(),
                icon: None,
                settings: vec![],
            }],
        };

        registry.register(custom);

        assert!(registry.has_source("custom"));
        assert!(registry.has_node("custom", "output"));
    }

    #[test]
    fn test_all_sources() {
        let registry = SourceRegistry::new();
        let sources = registry.all_sources();

        assert!(sources.len() >= 3);
    }
}
