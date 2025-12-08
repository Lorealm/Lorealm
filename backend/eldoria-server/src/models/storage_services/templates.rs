use std::sync::Arc;

use uuid::Uuid;

// Adjust this import path to match your storage_system crate
use storage_system::{Storage, StorageError, Template, TemplateMetadata};

use crate::services::context::{ContextNode, ContextTemplate, NodeContent};

/// Errors specific to template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateServiceError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Template not found: {0}")]
    NotFound(Uuid),

    #[error("Access denied: user {user_id} cannot access template {template_id}")]
    AccessDenied { user_id: Uuid, template_id: Uuid },

    #[error("Template with name '{0}' already exists")]
    DuplicateName(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

/// Service for template operations, bridging domain logic and storage
pub struct TemplateService<S: Storage + ?Sized> {
    storage: Arc<S>,
}

impl<S: Storage + ?Sized> TemplateService<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Get a template by ID
    pub async fn get_template(&self, id: Uuid) -> Result<ContextTemplate, TemplateServiceError> {
        let storage_template = self
            .storage
            .get_template(id)
            .await?
            .ok_or(TemplateServiceError::NotFound(id))?;

        self.storage_to_domain(storage_template)
    }

    /// Get a template by ID, verifying ownership
    pub async fn get_template_owned(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<ContextTemplate, TemplateServiceError> {
        let storage_template = self
            .storage
            .get_template_owned(id, owner_id)
            .await?
            .ok_or(TemplateServiceError::NotFound(id))?;

        // Double-check ownership (storage might not enforce it)
        if storage_template.owner_id != owner_id {
            return Err(TemplateServiceError::AccessDenied {
                user_id: owner_id,
                template_id: id,
            });
        }

        self.storage_to_domain(storage_template)
    }

    /// List all templates for a user
    pub async fn list_templates(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<TemplateMetadata>, TemplateServiceError> {
        Ok(self.storage.list_templates(owner_id).await?)
    }

    /// Create a new template
    pub async fn create_template(
        &self,
        template: ContextTemplate,
    ) -> Result<ContextTemplate, TemplateServiceError> {
        // Check for duplicate name
        if self
            .storage
            .template_name_exists(template.owner_id, &template.name, None)
            .await?
        {
            return Err(TemplateServiceError::DuplicateName(template.name.clone()));
        }

        let storage_template = self.domain_to_storage(&template)?;
        self.storage.save_template(&storage_template).await?;

        Ok(template)
    }

    /// Update an existing template
    pub async fn update_template(
        &self,
        template: ContextTemplate,
        owner_id: Uuid,
    ) -> Result<ContextTemplate, TemplateServiceError> {
        // Verify ownership
        let existing = self
            .storage
            .get_template_owned(template.id, owner_id)
            .await?
            .ok_or(TemplateServiceError::NotFound(template.id))?;

        if existing.owner_id != owner_id {
            return Err(TemplateServiceError::AccessDenied {
                user_id: owner_id,
                template_id: template.id,
            });
        }

        // Check for duplicate name (excluding this template)
        if self
            .storage
            .template_name_exists(owner_id, &template.name, Some(template.id))
            .await?
        {
            return Err(TemplateServiceError::DuplicateName(template.name.clone()));
        }

        let storage_template = self.domain_to_storage(&template)?;
        self.storage.save_template(&storage_template).await?;

        Ok(template)
    }

    /// Delete a template
    pub async fn delete_template(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<(), TemplateServiceError> {
        // Verify ownership first
        let existing = self
            .storage
            .get_template_owned(id, owner_id)
            .await?
            .ok_or(TemplateServiceError::NotFound(id))?;

        if existing.owner_id != owner_id {
            return Err(TemplateServiceError::AccessDenied {
                user_id: owner_id,
                template_id: id,
            });
        }

        self.storage.delete_template(id, owner_id).await?;
        Ok(())
    }

    /// Check if a template exists
    pub async fn template_exists(&self, id: Uuid) -> Result<bool, TemplateServiceError> {
        Ok(self.storage.template_exists(id).await?)
    }

    // ==================== Conversion Helpers ====================

    /// Convert storage entity to domain model
    fn storage_to_domain(
        &self,
        storage: Template,
    ) -> Result<ContextTemplate, TemplateServiceError> {
        let nodes: Vec<ContextNode> = serde_json::from_str(&storage.nodes_json).map_err(|e| {
            TemplateServiceError::Deserialization(format!(
                "Failed to deserialize nodes: {}",
                e
            ))
        })?;

        Ok(ContextTemplate {
            id: storage.id,
            owner_id: storage.owner_id,
            name: storage.name,
            description: storage.description,
            nodes,
            version: storage.version,
            created_at: storage.created_at,
            updated_at: storage.updated_at,
        })
    }

    /// Convert domain model to storage entity
    fn domain_to_storage(
        &self,
        domain: &ContextTemplate,
    ) -> Result<Template, TemplateServiceError> {
        let nodes_json = serde_json::to_string(&domain.nodes).map_err(|e| {
            TemplateServiceError::Serialization(format!("Failed to serialize nodes: {}", e))
        })?;

        Ok(Template {
            id: domain.id,
            owner_id: domain.owner_id,
            name: domain.name.clone(),
            description: domain.description.clone(),
            nodes_json,
            version: domain.version,
            created_at: domain.created_at,
            updated_at: domain.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context::PromptRole;

    // Mock storage for testing would go here
    // You can use your existing test infrastructure

    #[test]
    fn test_node_serialization_roundtrip() {
        let nodes = vec![
            ContextNode::new_text("System", "Hello", Some(PromptRole::System)),
            ContextNode::new_source("History", "chat_history", "main"),
            ContextNode::new_group("Group"),
        ];

        let json = serde_json::to_string(&nodes).unwrap();
        let deserialized: Vec<ContextNode> = serde_json::from_str(&json).unwrap();

        assert_eq!(nodes.len(), deserialized.len());
        assert_eq!(nodes[0].title, deserialized[0].title);
    }
}