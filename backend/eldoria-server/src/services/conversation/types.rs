use std::{collections::{HashMap, HashSet}, default};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::models::MessageService;

#[derive(Debug, Error)]
pub enum ConversationError {
    #[error("No active root found in conversation")]
    NoActiveRoot,
    #[error("Active path contains invalid message ID: {0}")]
    InvalidActivePath(Uuid),
    #[error("Conversation has no messages")]
    EmptyConversation,
    #[error("Could not branch from ID: {0}")]
    BranchError(Uuid),
    #[error("Could not find message with ID: {0}")]
    MessageNotFound(Uuid),
    #[error("Entity not found: {0}")]
    EntityNotFound(Uuid),
    #[error("Storage error: {0}")]
    StorageError(String),
}

pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub metadata: ConversationMetadata,
    pub root: MessageNode,
    pub active_path: Vec<Uuid>,
    pub context: ConversationContext
}

pub struct ConversationMetadata {
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub last_message_at: DateTime<Utc>,
}

pub struct MessageNode {
    pub message: ChatMessage,
    pub children: Vec<MessageNode>,
    pub is_active: bool,
}

#[derive(Debug)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: content.to_string(),
            created_at: Utc::now()
        }
    }
    
    pub fn assistant(content: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            created_at: Utc::now()
        }
    }
    
    pub fn system(content: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: content.to_string(),
            created_at: Utc::now()
        }
    }
}

#[derive(Debug)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A policy that states how outdated components in a world should be handled
///
/// Explanation:
/// * `Locked`: A locked policy means that once a character/instance is first met, it will remain in the state it was met in for the rest of the conversation.
/// Note that non-instantiated entries will still use the newest version.
/// * `AlwaysLatest`: AlwaysLatest uses the latest world state. If a user edits an entity that was already instantiated, it's changes will apply immediately.
/// * `PromptOnDrift`: Prompts the user whether to keep or update modified entities.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SyncPolicy {
    Locked,
    AlwaysLatest,
    PromptOnDrift,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        SyncPolicy::Locked
    }
}

pub struct ConversationContext {
    pub source: ContextSource,
    pub instantiated_entities: InstantiatedEntities,
    pub sync_policy: SyncPolicy,
}

#[derive(Default)]
pub struct InstantiatedEntities {
    pub characters: HashMap<Uuid, CharacterInstance>,
    pub locations: HashMap<Uuid, LocationInstance>,
    pub items: HashMap<Uuid, ItemInstance>,
}

pub struct CharacterInstance {
    pub character_id: Uuid,
    pub name: String,
    pub persona: String,
    pub instantiated_at: DateTime<Utc>
}

pub struct LocationInstance {
    pub location_id: Uuid,
    pub name: String,
    pub description: String
}

pub struct ItemInstance {
    pub item_id: Uuid,
    pub name: String,
    pub description: String
}

#[derive(Clone, Debug)]
pub enum ContextSource {
    Empty,
    Character {
        character_id: Uuid,
        version: u64,
    },
    Scenario {
        scenario_id: Uuid,
        version: u64,
    },
    World {
        world_id: Uuid,
        version: u64,
    },
    Custom {
        context_type: String,
        context_id: Uuid,
        metadata: HashMap<String, serde_json::Value>,
    },
}

impl Conversation {
    pub fn from_storage(
        conv: storage_system::Conversation,
        meta: storage_system::ConversationMetadata,
        messages: Vec<storage_system::Message>,
        active_path: storage_system::ActivePath,
        instantiated_entities: Vec<storage_system::InstantiatedEntity>,
    ) -> Result<Self, ConversationError> {
        if messages.is_empty() {
            return Err(ConversationError::EmptyConversation);
        }

        let mut tree = build_tree(messages);
        let active_set: HashSet<Uuid> = active_path.message_ids.iter().copied().collect();

        for root in &mut tree {
            mark_active_path(root, &active_set);
        }

        let root = tree
            .into_iter()
            .find(|x| x.is_active)
            .ok_or(ConversationError::NoActiveRoot)?;

        let convo_meta = ConversationMetadata {
            title: meta.title,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            message_count: meta.message_count,
            last_message_at: meta.last_message_at,
        };

        let mut context = ConversationContext::from_storage(&conv)?;

        // Load instantiated entities into context
        for entity in instantiated_entities {
            match entity.entity_type.as_str() {
                "character" => {
                    let instance: CharacterInstance =
                        serde_json::from_str(&entity.snapshot_data)
                            .map_err(|e| ConversationError::StorageError(e.to_string()))?;
                    context.instantiated_entities.characters.insert(entity.entity_id, instance);
                }
                "location" => {
                    let instance: LocationInstance =
                        serde_json::from_str(&entity.snapshot_data)
                            .map_err(|e| ConversationError::StorageError(e.to_string()))?;
                    context.instantiated_entities.locations.insert(entity.entity_id, instance);
                }
                "item" => {
                    let instance: ItemInstance =
                        serde_json::from_str(&entity.snapshot_data)
                            .map_err(|e| ConversationError::StorageError(e.to_string()))?;
                    context.instantiated_entities.items.insert(entity.entity_id, instance);
                }
                _ => {}
            }
        }

        Ok(Self {
            id: conv.id,
            user_id: conv.user_id,
            metadata: convo_meta,
            root,
            active_path: active_path.message_ids,
            context,
        })
    }

    // ========================================================================
    // Context Management (Function Sketches for Future Implementation)
    // ========================================================================

    /// Get a character for use in this conversation. Instantiates if not already done.
    pub fn get_or_instantiate_character<S>(
        &mut self,
        character_id: Uuid,
        storage: &S,
    ) -> Result<&CharacterInstance, ConversationError>
    where
        S: WorldStorage,
    {
        todo!("Implement character instantiation with sync policy")
        // Should:
        // 1. Check if already instantiated
        // 2. Apply sync_policy logic
        // 3. Load from storage if needed
        // 4. Create snapshot and store in instantiated_entities
    }

    /// Get a location for use in this conversation
    pub fn get_or_instantiate_location<S>(
        &mut self,
        location_id: Uuid,
        storage: &S,
    ) -> Result<&LocationInstance, ConversationError>
    where
        S: WorldStorage,
    {
        todo!("Implement location instantiation")
    }

    /// Get an item for use in this conversation
    pub fn get_or_instantiate_item<S>(
        &mut self,
        item_id: Uuid,
        storage: &S,
    ) -> Result<&ItemInstance, ConversationError>
    where
        S: WorldStorage,
    {
        todo!("Implement item instantiation")
    }

    /// Check if any instantiated entities have drifted from world state
    pub fn check_entity_drift<S>(
        &self,
        storage: &S,
    ) -> Result<Vec<EntityDrift>, ConversationError>
    where
        S: WorldStorage,
    {
        todo!("Implement drift detection")
        // Should compare instantiated snapshots with current world state
    }

    /// Sync a specific entity to current world state
    pub fn sync_entity_to_world<S>(
        &mut self,
        entity_id: Uuid,
        entity_type: EntityType,
        storage: &S,
    ) -> Result<(), ConversationError>
    where
        S: WorldStorage,
    {
        todo!("Implement entity sync")
    }

    /// Build context for LLM generation
    pub fn build_generation_context(&self) -> GenerationContext {
        todo!("Build prompt context from instantiated entities")
        // Should create a structured context with:
        // - System prompts based on context source
        // - Active character personas
        // - Location descriptions
        // - Relevant items
    }

    // ========================================================================
    // Message Management
    // ========================================================================

    pub fn get_active_thread(&self) -> Vec<&ChatMessage> {
        self.active_path.iter()
            .filter_map(|id| self.get_node(*id).ok())
            .map(|node| &node.message)
            .collect()
    }

    pub fn get_swipes(&self, message_id: Uuid) -> Result<Vec<&ChatMessage>, ConversationError> {
        let parent_id = self.find_parent_id(message_id)
            .ok_or(ConversationError::MessageNotFound(message_id))?;

        let parent = self.get_node(parent_id)?;
        Ok(parent.children.iter().map(|child| &child.message).collect())
    }

    pub fn count_swipes(&self, message_id: Uuid) -> Result<usize, ConversationError> {
        Ok(self.get_swipes(message_id)?.len())
    }

    pub fn to_storage(&self) -> ConversationStorageBundle {
        let messages = flatten_tree(&self.root, None, self.id);

        let active_path = storage_system::ActivePath {
            conversation_id: self.id,
            message_ids: self.active_path.clone(),
            updated_at: Utc::now(),
        };

        let conversation = self.context.to_storage(self.id, self.user_id);

        let metadata = storage_system::ConversationMetadata {
            id: Uuid::new_v4(),
            conversation_id: self.id,
            title: self.metadata.title.clone(),
            message_count: self.metadata.message_count,
            last_message_at: self.metadata.last_message_at,
            created_at: self.metadata.created_at,
            updated_at: Utc::now(),
        };

        let instantiated_entities = self.context.instantiated_entities_to_storage(self.id);

        ConversationStorageBundle {
            conversation,
            metadata,
            messages,
            active_path,
            instantiated_entities,
        }
    }

    pub fn get_tree(&self) -> &MessageNode {
        &self.root
    }

    pub fn get_node(&self, id: Uuid) -> Result<&MessageNode, ConversationError> {
        find_node_recursive(&self.root, id).ok_or(ConversationError::MessageNotFound(id))
    }

    pub fn get_node_mut(&mut self, id: Uuid) -> Result<&mut MessageNode, ConversationError> {
        find_node_recursive_mut(&mut self.root, id).ok_or(ConversationError::MessageNotFound(id))
    }

    fn get_path_until(&self, message_id: Uuid) -> Result<Vec<Uuid>, ConversationError> {
        let mut new_active_path = Vec::new();
        let mut parent_found = false;

        for id in &self.active_path {
            new_active_path.push(id.to_owned());
            if id == &message_id {
                parent_found = true;
                break;
            }
        }

        if !parent_found {
            return Err(ConversationError::InvalidActivePath(message_id));
        }

        Ok(new_active_path)
    }

    pub fn select_swipe(&mut self, message_id: Uuid) -> Result<(), ConversationError> {
        let _ = self.get_node(message_id)?;
        let parent_id = self.find_parent_id(message_id);

        let new_active_path = if let Some(parent_id) = parent_id {
            let mut path = self.get_path_until(parent_id)?;
            path.push(message_id);
            path
        } else {
            vec![message_id]
        };

        clear_active_flags(&mut self.root);
        let active_set: HashSet<Uuid> = new_active_path.iter().copied().collect();
        mark_active_path(&mut self.root, &active_set);

        let mut complete_path = new_active_path;
        let mut current_id = message_id;

        loop {
            let current_node = self.get_node(current_id)?;

            if let Some(child) = current_node.children.first() {
                let child_id = child.message.id;
                complete_path.push(child_id);

                let child_node = self.get_node_mut(child_id)?;
                child_node.is_active = true;

                current_id = child_id;
            } else {
                break;
            }
        }

        self.active_path = complete_path;
        self.metadata.updated_at = Utc::now();
        Ok(())
    }

    pub fn branch_from(
        &mut self,
        parent_id: Uuid,
        new_message: ChatMessage,
    ) -> Result<(), ConversationError> {
        let mut new_active_path = self.get_path_until(parent_id)?;

        new_active_path.push(new_message.id);
        let parent_node = self.get_node_mut(parent_id)?;

        let new_node = MessageNode {
            message: new_message,
            children: Vec::new(),
            is_active: false,
        };

        parent_node.add_child(new_node, true);
        self.active_path = new_active_path;
        self.metadata.message_count += 1;
        self.metadata.last_message_at = Utc::now();
        self.metadata.updated_at = Utc::now();
        Ok(())
    }

    fn find_parent_id(&self, child_id: Uuid) -> Option<Uuid> {
        find_parent_recursive(&self.root, child_id)
    }

    fn update_active_path_after_deletion(
        &mut self,
        deleted_id: Uuid,
        parent_id: Uuid,
        reindex: bool
    ) -> Result<(), ConversationError> {
        let deleted_pos = self.active_path.iter()
            .position(|id| *id == deleted_id)
            .ok_or(ConversationError::InvalidActivePath(deleted_id))?;

        if reindex {
            self.active_path.remove(deleted_pos);
        } else {
            self.active_path.truncate(deleted_pos);
        }

        clear_active_flags(&mut self.root);
        let active_set: HashSet<Uuid> = self.active_path.iter().copied().collect();
        mark_active_path(&mut self.root, &active_set);

        Ok(())
    }

    pub fn delete_message(
        &mut self,
        message_id: Uuid,
        reindex: bool
    ) -> Result<(), ConversationError> {
        if self.root.message.id == message_id {
            return Err(ConversationError::BranchError(message_id));
        }

        let parent_id = self.find_parent_id(message_id)
            .ok_or(ConversationError::MessageNotFound(message_id))?;

        let parent_node = self.get_node_mut(parent_id)?;

        let child_index = parent_node.children.iter()
            .position(|child| child.message.id == message_id)
            .ok_or(ConversationError::MessageNotFound(message_id))?;

        if reindex {
            let deleted_node = parent_node.children.remove(child_index);
            let children_to_promote = deleted_node.children;
            parent_node.children.splice(child_index..child_index, children_to_promote);
        } else {
            parent_node.children.remove(child_index);
        }

        if self.active_path.contains(&message_id) {
            self.update_active_path_after_deletion(message_id, parent_id, reindex)?;
        }

        self.metadata.message_count = count_messages(&self.root);
        self.metadata.updated_at = Utc::now();

        Ok(())
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

pub struct ConversationStorageBundle {
    pub conversation: storage_system::Conversation,
    pub metadata: storage_system::ConversationMetadata,
    pub messages: Vec<storage_system::Message>,
    pub active_path: storage_system::ActivePath,
    pub instantiated_entities: Vec<storage_system::InstantiatedEntity>,
}

pub struct EntityDrift {
    pub entity_id: Uuid,
    pub entity_type: EntityType,
    pub detected_at: DateTime<Utc>,
    pub changes: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum EntityType {
    Character,
    Location,
    Item,
}

pub struct GenerationContext {
    pub system_prompt: String,
    pub character_contexts: Vec<String>,
    pub location_context: Option<String>,
    pub relevant_items: Vec<String>,
}

// Trait that storage implementations must provide
pub trait WorldStorage {
    fn load_character(&self, id: Uuid) -> Result<CharacterData, ConversationError>;
    fn load_location(&self, id: Uuid) -> Result<LocationData, ConversationError>;
    fn load_item(&self, id: Uuid) -> Result<ItemData, ConversationError>;
}

#[derive(Clone)]
pub struct CharacterData {
    pub id: Uuid,
    pub name: String,
    pub persona: String,
    pub version: u64,
}

#[derive(Clone)]
pub struct LocationData {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: u64,
}

#[derive(Clone)]
pub struct ItemData {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: u64,
}

// ============================================================================
// Context Conversion Implementations
// ============================================================================

impl ConversationContext {
    pub fn from_storage(stored: &storage_system::Conversation) -> Result<Self, ConversationError> {
        let source = match stored.context_type.as_str() {
            "empty" => ContextSource::Empty,
            "character" => ContextSource::Character {
                character_id: stored.context_id
                    .ok_or_else(|| ConversationError::StorageError("Missing context_id for character".into()))?,
                version: stored.context_version
                    .ok_or_else(|| ConversationError::StorageError("Missing context_version for character".into()))?,
            },
            "scenario" => ContextSource::Scenario {
                scenario_id: stored.context_id
                    .ok_or_else(|| ConversationError::StorageError("Missing context_id for scenario".into()))?,
                version: stored.context_version
                    .ok_or_else(|| ConversationError::StorageError("Missing context_version for scenario".into()))?,
            },
            "world" => ContextSource::World {
                world_id: stored.context_id
                    .ok_or_else(|| ConversationError::StorageError("Missing context_id for world".into()))?,
                version: stored.context_version
                    .ok_or_else(|| ConversationError::StorageError("Missing context_version for world".into()))?,
            },
            "custom" => ContextSource::Custom {
                context_type: "custom".to_string(),
                context_id: stored.context_id
                    .ok_or_else(|| ConversationError::StorageError("Missing context_id for custom".into()))?,
                metadata: serde_json::from_str(
                    stored.context_metadata.as_ref()
                        .ok_or_else(|| ConversationError::StorageError("Missing context_metadata for custom".into()))?
                ).map_err(|e| ConversationError::StorageError(e.to_string()))?,
            },
            _ => ContextSource::Empty,
        };

        let sync_policy: SyncPolicy = serde_json::from_str(&stored.sync_policy)
            .map_err(|e| ConversationError::StorageError(e.to_string()))?;

        Ok(ConversationContext {
            source,
            instantiated_entities: InstantiatedEntities::default(),
            sync_policy,
        })
    }

    pub fn to_storage(&self, conversation_id: Uuid, user_id: Uuid) -> storage_system::Conversation {
        let (context_type, context_id, context_version, context_metadata) = match &self.source {
            ContextSource::Empty => ("empty".to_string(), None, None, None),
            ContextSource::Character { character_id, version } =>
                ("character".to_string(), Some(*character_id), Some(*version), None),
            ContextSource::Scenario { scenario_id, version } =>
                ("scenario".to_string(), Some(*scenario_id), Some(*version), None),
            ContextSource::World { world_id, version } =>
                ("world".to_string(), Some(*world_id), Some(*version), None),
            ContextSource::Custom { context_type: _, context_id, metadata } =>
                ("custom".to_string(), Some(*context_id), None,
                 Some(serde_json::to_string(&metadata).unwrap())),
        };

        storage_system::Conversation {
            id: conversation_id,
            user_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context_type,
            context_id,
            context_version,
            context_metadata,
            sync_policy: serde_json::to_string(&self.sync_policy).unwrap(),
        }
    }

    pub fn instantiated_entities_to_storage(&self, conversation_id: Uuid) -> Vec<storage_system::InstantiatedEntity> {
        self.instantiated_entities.to_storage(conversation_id)
    }
}

impl InstantiatedEntities {
    pub fn to_storage(&self, conversation_id: Uuid) -> Vec<storage_system::InstantiatedEntity> {
        let mut entities = Vec::new();

        for (id, character) in &self.characters {
            entities.push(storage_system::InstantiatedEntity {
                id: Uuid::new_v4(),
                conversation_id,
                entity_id: *id,
                entity_type: "character".to_string(),
                snapshot_data: serde_json::to_string(character).unwrap(),
                instantiated_at: character.instantiated_at,
                modified_at: Utc::now(),
            });
        }

        for (id, location) in &self.locations {
            entities.push(storage_system::InstantiatedEntity {
                id: Uuid::new_v4(),
                conversation_id,
                entity_id: *id,
                entity_type: "location".to_string(),
                snapshot_data: serde_json::to_string(location).unwrap(),
                instantiated_at: Utc::now(),
                modified_at: Utc::now(),
            });
        }

        for (id, item) in &self.items {
            entities.push(storage_system::InstantiatedEntity {
                id: Uuid::new_v4(),
                conversation_id,
                entity_id: *id,
                entity_type: "item".to_string(),
                snapshot_data: serde_json::to_string(item).unwrap(),
                instantiated_at: Utc::now(),
                modified_at: Utc::now(),
            });
        }

        entities
    }
}

// Need Serialize/Deserialize for snapshots
impl Serialize for CharacterInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CharacterInstance", 4)?;
        state.serialize_field("character_id", &self.character_id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("persona", &self.persona)?;
        state.serialize_field("instantiated_at", &self.instantiated_at)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CharacterInstance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            character_id: Uuid,
            name: String,
            persona: String,
            instantiated_at: DateTime<Utc>,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(CharacterInstance {
            character_id: helper.character_id,
            name: helper.name,
            persona: helper.persona,
            instantiated_at: helper.instantiated_at,
        })
    }
}

impl Serialize for LocationInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LocationInstance", 3)?;
        state.serialize_field("location_id", &self.location_id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("description", &self.description)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LocationInstance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            location_id: Uuid,
            name: String,
            description: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(LocationInstance {
            location_id: helper.location_id,
            name: helper.name,
            description: helper.description,
        })
    }
}

impl Serialize for ItemInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ItemInstance", 3)?;
        state.serialize_field("item_id", &self.item_id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("description", &self.description)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ItemInstance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            item_id: Uuid,
            name: String,
            description: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(ItemInstance {
            item_id: helper.item_id,
            name: helper.name,
            description: helper.description,
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

impl MessageNode {
    pub fn add_child(&mut self, node: MessageNode, set_active: bool) {
        if set_active {
            self.children
                .iter_mut()
                .for_each(|child| child.is_active = false);
        }
        self.children.push(node);
    }
}

fn mark_active_path(root: &mut MessageNode, active_ids: &HashSet<Uuid>) {
    if active_ids.contains(&root.message.id) {
        root.is_active = true;
    }

    for child in &mut root.children {
        mark_active_path(child, active_ids);
    }
}

fn build_tree(messages: Vec<storage_system::Message>) -> Vec<MessageNode> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut children_map: HashMap<Option<Uuid>, Vec<storage_system::Message>> = HashMap::new();
    for message in messages {
        children_map
            .entry(message.parent_message_id)
            .or_insert_with(Vec::new)
            .push(message);
    }

    let roots = children_map.remove(&None).unwrap_or_default();
    roots
        .into_iter()
        .map(|msg| build_subtree(msg, &children_map))
        .collect()
}

fn build_subtree(
    message: storage_system::Message,
    children_map: &HashMap<Option<Uuid>, Vec<storage_system::Message>>,
) -> MessageNode {
    let message_id = message.id;
    let node_message = ChatMessage {
        id: message.id,
        role: message.role.into(),
        content: message.content,
        created_at: message.created_at,
    };

    let children = children_map
        .get(&Some(message_id))
        .map(|msgs| {
            let mut nodes: Vec<_> = msgs
                .iter()
                .map(|child| build_subtree(child.clone(), children_map))
                .collect();
            nodes.sort_by_key(|n| n.message.created_at);
            nodes
        })
        .unwrap_or_default();

    MessageNode {
        message: node_message,
        children,
        is_active: false,
    }
}

fn find_node_recursive(node: &MessageNode, id: Uuid) -> Option<&MessageNode> {
    if node.message.id == id {
        return Some(node);
    }

    for child in &node.children {
        if let Some(found) = find_node_recursive(child, id) {
            return Some(found);
        }
    }

    None
}

fn find_node_recursive_mut(node: &mut MessageNode, id: Uuid) -> Option<&mut MessageNode> {
    if node.message.id == id {
        return Some(node);
    }

    for child in &mut node.children {
        if let Some(found) = find_node_recursive_mut(child, id) {
            return Some(found);
        }
    }

    None
}

fn clear_active_flags(node: &mut MessageNode) {
    node.is_active = false;
    for child in &mut node.children {
        clear_active_flags(child);
    }
}

fn find_parent_recursive(node: &MessageNode, child_id: Uuid) -> Option<Uuid> {
    for child in &node.children {
        if child.message.id == child_id {
            return Some(node.message.id);
        }
    }

    for child in &node.children {
        if let Some(parent_id) = find_parent_recursive(child, child_id) {
            return Some(parent_id);
        }
    }

    None
}

fn count_messages(node: &MessageNode) -> usize {
    1 + node.children.iter().map(|child| count_messages(child)).sum::<usize>()
}

fn flatten_tree(node: &MessageNode, parent_id: Option<Uuid>, conversation_id: Uuid) -> Vec<storage_system::Message> {
    let mut messages = vec![storage_system::Message {
        id: node.message.id,
        conversation_id,
        parent_message_id: parent_id,
        role: (&node.message.role).into(),
        content: node.message.content.clone(),
        created_at: node.message.created_at,
        updated_at: Utc::now(),
    }];

    for child in &node.children {
        messages.extend(flatten_tree(child, Some(node.message.id), conversation_id));
    }

    messages
}

impl Into<storage_system::MessageRole> for &MessageRole {
    fn into(self) -> storage_system::MessageRole {
        match &self {
            MessageRole::User => storage_system::MessageRole::User,
            MessageRole::Assistant => storage_system::MessageRole::Assistant,
            MessageRole::System => storage_system::MessageRole::System,
        }
    }
}

impl From<storage_system::MessageRole> for MessageRole {
    fn from(value: storage_system::MessageRole) -> Self {
        match value {
            storage_system::MessageRole::User => MessageRole::User,
            storage_system::MessageRole::Assistant => MessageRole::Assistant,
            storage_system::MessageRole::System => MessageRole::System,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_message(id: u128, role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage {
            id: Uuid::from_u128(id),
            role,
            content: content.to_string(),
            created_at: Utc::now(),
        }
    }

    fn create_storage_message(
        id: u128,
        conversation_id: u128,
        parent_id: Option<u128>,
        role: storage_system::MessageRole,
        content: &str,
    ) -> storage_system::Message {
        storage_system::Message {
            id: Uuid::from_u128(id),
            conversation_id: Uuid::from_u128(conversation_id),
            parent_message_id: parent_id.map(Uuid::from_u128),
            role,
            content: content.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_simple_conversation() -> Conversation {
        let conv_id = 100u128;
        let messages = vec![
            create_storage_message(1, conv_id, None, storage_system::MessageRole::User, "Hello"),
            create_storage_message(2, conv_id, Some(1), storage_system::MessageRole::Assistant, "Hi there"),
            create_storage_message(3, conv_id, Some(2), storage_system::MessageRole::User, "How are you?"),
        ];

        let conv = storage_system::Conversation {
            id: Uuid::from_u128(conv_id),
            user_id: Uuid::from_u128(200),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context_type: "empty".to_string(),
            context_id: None,
            context_version: None,
            context_metadata: None,
            sync_policy: serde_json::to_string(&SyncPolicy::Locked).unwrap(),
        };

        let meta = storage_system::ConversationMetadata {
            id: Uuid::from_u128(101),
            conversation_id: Uuid::from_u128(conv_id),
            title: Some("Test".to_string()),
            message_count: 3,
            last_message_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let active_path = storage_system::ActivePath {
            conversation_id: Uuid::from_u128(conv_id),
            message_ids: vec![
                Uuid::from_u128(1),
                Uuid::from_u128(2),
                Uuid::from_u128(3),
            ],
            updated_at: Utc::now(),
        };

        Conversation::from_storage(conv, meta, messages, active_path, vec![]).unwrap()
    }

    fn create_conversation_with_swipes() -> Conversation {
        let conv_id = 100u128;
        let messages = vec![
            create_storage_message(1, conv_id, None, storage_system::MessageRole::User, "Hello"),
            create_storage_message(2, conv_id, Some(1), storage_system::MessageRole::Assistant, "Hi there"),
            create_storage_message(3, conv_id, Some(1), storage_system::MessageRole::Assistant, "Hello!"),
            create_storage_message(4, conv_id, Some(1), storage_system::MessageRole::Assistant, "Hey"),
            create_storage_message(5, conv_id, Some(2), storage_system::MessageRole::User, "How are you?"),
        ];

        let conv = storage_system::Conversation {
            id: Uuid::from_u128(conv_id),
            user_id: Uuid::from_u128(200),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context_type: "empty".to_string(),
            context_id: None,
            context_version: None,
            context_metadata: None,
            sync_policy: serde_json::to_string(&SyncPolicy::Locked).unwrap(),
        };

        let meta = storage_system::ConversationMetadata {
            id: Uuid::from_u128(101),
            conversation_id: Uuid::from_u128(conv_id),
            title: Some("Test with Swipes".to_string()),
            message_count: 5,
            last_message_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let active_path = storage_system::ActivePath {
            conversation_id: Uuid::from_u128(conv_id),
            message_ids: vec![
                Uuid::from_u128(1),
                Uuid::from_u128(2),
                Uuid::from_u128(5),
            ],
            updated_at: Utc::now(),
        };

        Conversation::from_storage(conv, meta, messages, active_path, vec![]).unwrap()
    }

    #[test]
    fn test_from_storage_simple() {
        let conv = create_simple_conversation();

        assert_eq!(conv.id, Uuid::from_u128(100));
        assert_eq!(conv.user_id, Uuid::from_u128(200));
        assert_eq!(conv.metadata.message_count, 3);
        assert_eq!(conv.active_path.len(), 3);
        assert_eq!(conv.root.message.id, Uuid::from_u128(1));
    }

    #[test]
    fn test_from_storage_empty_messages() {
        let conv_id = 100u128;
        let conv = storage_system::Conversation {
            id: Uuid::from_u128(conv_id),
            user_id: Uuid::from_u128(200),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context_type: "empty".to_string(),
            context_id: None,
            context_version: None,
            context_metadata: None,
            sync_policy: serde_json::to_string(&SyncPolicy::Locked).unwrap(),
        };

        let meta = storage_system::ConversationMetadata {
            id: Uuid::from_u128(101),
            conversation_id: Uuid::from_u128(conv_id),
            title: None,
            message_count: 0,
            last_message_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let active_path = storage_system::ActivePath {
            conversation_id: Uuid::from_u128(conv_id),
            message_ids: vec![],
            updated_at: Utc::now(),
        };

        let result = Conversation::from_storage(conv, meta, vec![], active_path, vec![]);

        assert!(matches!(result, Err(ConversationError::EmptyConversation)));
    }

    #[test]
    fn test_context_source_character() {
        let conv_id = 100u128;
        let char_id = 500u128;

        let conv = storage_system::Conversation {
            id: Uuid::from_u128(conv_id),
            user_id: Uuid::from_u128(200),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context_type: "character".to_string(),
            context_id: Some(Uuid::from_u128(char_id)),
            context_version: Some(1),
            context_metadata: None,
            sync_policy: serde_json::to_string(&SyncPolicy::Locked).unwrap(),
        };

        let messages = vec![
            create_storage_message(1, conv_id, None, storage_system::MessageRole::System, "Character intro"),
        ];

        let meta = storage_system::ConversationMetadata {
            id: Uuid::from_u128(101),
            conversation_id: Uuid::from_u128(conv_id),
            title: Some("Character Chat".to_string()),
            message_count: 1,
            last_message_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let active_path = storage_system::ActivePath {
            conversation_id: Uuid::from_u128(conv_id),
            message_ids: vec![Uuid::from_u128(1)],
            updated_at: Utc::now(),
        };

        let conversation = Conversation::from_storage(conv, meta, messages, active_path, vec![]).unwrap();

        match conversation.context.source {
            ContextSource::Character { character_id, version } => {
                assert_eq!(character_id, Uuid::from_u128(char_id));
                assert_eq!(version, 1);
            }
            _ => panic!("Expected Character context source"),
        }
    }

    #[test]
    fn test_get_node() {
        let conv = create_simple_conversation();

        let node = conv.get_node(Uuid::from_u128(2)).unwrap();
        assert_eq!(node.message.content, "Hi there");

        let result = conv.get_node(Uuid::from_u128(999));
        assert!(matches!(result, Err(ConversationError::MessageNotFound(_))));
    }

    #[test]
    fn test_active_path_marking() {
        let conv = create_simple_conversation();

        let node1 = conv.get_node(Uuid::from_u128(1)).unwrap();
        assert!(node1.is_active);

        let node2 = conv.get_node(Uuid::from_u128(2)).unwrap();
        assert!(node2.is_active);

        let node3 = conv.get_node(Uuid::from_u128(3)).unwrap();
        assert!(node3.is_active);
    }

    #[test]
    fn test_select_swipe() {
        let mut conv = create_conversation_with_swipes();

        assert_eq!(conv.active_path, vec![
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Uuid::from_u128(5),
        ]);

        conv.select_swipe(Uuid::from_u128(3)).unwrap();

        assert_eq!(conv.active_path, vec![
            Uuid::from_u128(1),
            Uuid::from_u128(3),
        ]);

        assert!(conv.get_node(Uuid::from_u128(1)).unwrap().is_active);
        assert!(!conv.get_node(Uuid::from_u128(2)).unwrap().is_active);
        assert!(conv.get_node(Uuid::from_u128(3)).unwrap().is_active);
        assert!(!conv.get_node(Uuid::from_u128(5)).unwrap().is_active);
    }

    #[test]
    fn test_branch_from() {
        let mut conv = create_simple_conversation();
        let initial_count = conv.metadata.message_count;

        let new_message = create_message(10, MessageRole::Assistant, "New branch");
        conv.branch_from(Uuid::from_u128(2), new_message).unwrap();

        assert_eq!(conv.active_path, vec![
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Uuid::from_u128(10),
        ]);

        assert_eq!(conv.metadata.message_count, initial_count + 1);

        let node3 = conv.get_node(Uuid::from_u128(3)).unwrap();
        assert!(!node3.is_active);

        let node10 = conv.get_node(Uuid::from_u128(10)).unwrap();
        assert_eq!(node10.message.content, "New branch");
    }

    #[test]
    fn test_delete_message_without_reindex() {
        let mut conv = create_simple_conversation();

        conv.delete_message(Uuid::from_u128(2), false).unwrap();

        assert_eq!(conv.active_path, vec![Uuid::from_u128(1)]);
        assert!(conv.get_node(Uuid::from_u128(2)).is_err());
        assert!(conv.get_node(Uuid::from_u128(3)).is_err());
        assert_eq!(conv.metadata.message_count, 1);
    }

    #[test]
    fn test_delete_message_with_reindex() {
        let mut conv = create_simple_conversation();

        conv.delete_message(Uuid::from_u128(2), true).unwrap();

        assert_eq!(conv.active_path, vec![
            Uuid::from_u128(1),
            Uuid::from_u128(3),
        ]);

        assert!(conv.get_node(Uuid::from_u128(2)).is_err());

        let node3 = conv.get_node(Uuid::from_u128(3)).unwrap();
        assert_eq!(node3.message.content, "How are you?");

        let parent_id = conv.find_parent_id(Uuid::from_u128(3)).unwrap();
        assert_eq!(parent_id, Uuid::from_u128(1));
    }

    #[test]
    fn test_delete_root_fails() {
        let mut conv = create_simple_conversation();
        let result = conv.delete_message(Uuid::from_u128(1), false);
        assert!(matches!(result, Err(ConversationError::BranchError(_))));
    }

    #[test]
    fn test_to_storage() {
        let conv = create_simple_conversation();
        let bundle = conv.to_storage();

        assert_eq!(bundle.conversation.id, conv.id);
        assert_eq!(bundle.conversation.user_id, conv.user_id);
        assert_eq!(bundle.messages.len(), 3);
        assert_eq!(bundle.active_path.message_ids.len(), 3);
        assert_eq!(bundle.metadata.message_count, 3);
    }

    #[test]
    fn test_get_active_thread() {
        let conv = create_simple_conversation();
        let thread = conv.get_active_thread();

        assert_eq!(thread.len(), 3);
        assert_eq!(thread[0].content, "Hello");
        assert_eq!(thread[1].content, "Hi there");
        assert_eq!(thread[2].content, "How are you?");
    }

    #[test]
    fn test_get_swipes() {
        let conv = create_conversation_with_swipes();

        // Get swipes for message 2 (should return 2, 3, 4)
        let swipes = conv.get_swipes(Uuid::from_u128(2)).unwrap();
        assert_eq!(swipes.len(), 3);

        let contents: Vec<&str> = swipes.iter().map(|m| m.content.as_str()).collect();
        assert!(contents.contains(&"Hi there"));
        assert!(contents.contains(&"Hello!"));
        assert!(contents.contains(&"Hey"));
    }

    #[test]
    fn test_count_swipes() {
        let conv = create_conversation_with_swipes();
        let count = conv.count_swipes(Uuid::from_u128(2)).unwrap();
        assert_eq!(count, 3);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;

    // Mock WorldStorage for testing
    struct MockWorldStorage {
        characters: HashMap<Uuid, CharacterData>,
        locations: HashMap<Uuid, LocationData>,
        items: HashMap<Uuid, ItemData>,
    }

    impl MockWorldStorage {
        fn new() -> Self {
            let mut characters = HashMap::new();
            let mut locations = HashMap::new();
            let mut items = HashMap::new();

            // Add some test data
            characters.insert(
                Uuid::from_u128(1000),
                CharacterData {
                    id: Uuid::from_u128(1000),
                    name: "Elara the Wise".to_string(),
                    persona: "An ancient elven sage who speaks in riddles and offers cryptic wisdom.".to_string(),
                    version: 1,
                }
            );

            characters.insert(
                Uuid::from_u128(1001),
                CharacterData {
                    id: Uuid::from_u128(1001),
                    name: "Grimjaw the Orc".to_string(),
                    persona: "A battle-hardened orc warrior with a surprisingly gentle heart.".to_string(),
                    version: 1,
                }
            );

            locations.insert(
                Uuid::from_u128(2000),
                LocationData {
                    id: Uuid::from_u128(2000),
                    name: "The Ancient Library".to_string(),
                    description: "A vast repository of knowledge, its shelves stretching into the shadows.".to_string(),
                    version: 1,
                }
            );

            items.insert(
                Uuid::from_u128(3000),
                ItemData {
                    id: Uuid::from_u128(3000),
                    name: "Tome of Forgotten Lore".to_string(),
                    description: "A dusty book with strange symbols on its cover.".to_string(),
                    version: 1,
                }
            );

            Self { characters, locations, items }
        }
    }

    impl WorldStorage for MockWorldStorage {
        fn load_character(&self, id: Uuid) -> Result<CharacterData, ConversationError> {
            self.characters.get(&id)
                .cloned()
                .ok_or(ConversationError::EntityNotFound(id))
        }

        fn load_location(&self, id: Uuid) -> Result<LocationData, ConversationError> {
            self.locations.get(&id)
                .cloned()
                .ok_or(ConversationError::EntityNotFound(id))
        }

        fn load_item(&self, id: Uuid) -> Result<ItemData, ConversationError> {
            self.items.get(&id)
                .cloned()
                .ok_or(ConversationError::EntityNotFound(id))
        }
    }

    #[test]
    fn test_world_conversation_full_lifecycle() {
        // Setup: Create a world and mock storage
        let world_id = Uuid::from_u128(5000);
        let conv_id = 6000;
        let user_id = 7000;
        let storage = MockWorldStorage::new();

        // Step 1: Create a world-based conversation
        println!("=== Step 1: Create world conversation ===");

        let messages = vec![
            create_storage_message(
                1,
                conv_id,
                None,
                storage_system::MessageRole::System,
                "You enter the Ancient Library, a place of vast knowledge and mystery."
            ),
        ];

        let conv = storage_system::Conversation {
            id: Uuid::from_u128(conv_id),
            user_id: Uuid::from_u128(user_id),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context_type: "world".to_string(),
            context_id: Some(world_id),
            context_version: Some(1),
            context_metadata: None,
            sync_policy: serde_json::to_string(&SyncPolicy::Locked).unwrap(),
        };

        let meta = storage_system::ConversationMetadata {
            id: Uuid::new_v4(),
            conversation_id: Uuid::from_u128(conv_id),
            title: Some("Adventure in the Ancient Library".to_string()),
            message_count: 1,
            last_message_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let active_path = storage_system::ActivePath {
            conversation_id: Uuid::from_u128(conv_id),
            message_ids: vec![Uuid::from_u128(1)],
            updated_at: Utc::now(),
        };

        let mut conversation = Conversation::from_storage(
            conv,
            meta,
            messages,
            active_path,
            vec![]
        ).unwrap();

        // Verify world context
        match &conversation.context.source {
            ContextSource::World { world_id: wid, version } => {
                assert_eq!(*wid, world_id);
                assert_eq!(*version, 1);
                println!("✓ World context created: world_id={}, version={}", wid, version);
            }
            _ => panic!("Expected World context"),
        }

        // Step 2: User encounters Elara - instantiate character
        println!("\n=== Step 2: Instantiate character (Elara) ===");

        let elara_id = Uuid::from_u128(1000);
        let elara = storage.load_character(elara_id).unwrap();

        // Manually instantiate (since get_or_instantiate_character is not implemented yet)
        let elara_instance = CharacterInstance {
            character_id: elara.id,
            name: elara.name.clone(),
            persona: elara.persona.clone(),
            instantiated_at: Utc::now(),
        };
        conversation.context.instantiated_entities.characters.insert(elara_id, elara_instance);
        println!("✓ Elara instantiated: {}", elara.name);

        // Add user message
        let user_msg = create_message(2, MessageRole::User, "I approach the wise sage.");
        conversation.branch_from(Uuid::from_u128(1), user_msg).unwrap();
        println!("✓ User message added");

        // Step 3: Create multiple AI responses (swipes)
        println!("\n=== Step 3: Generate AI response swipes ===");

        let response1 = create_message(
            3,
            MessageRole::Assistant,
            "Elara looks up from her ancient tome. 'Ah, a seeker of knowledge. Tell me, young one, what brings you to this dusty sanctuary?'"
        );
        conversation.branch_from(Uuid::from_u128(2), response1).unwrap();
        println!("✓ Response 1 (swipe 1) generated");

        // Create alternative responses (swipes)
        let response2 = create_message(
            4,
            MessageRole::Assistant,
            "The sage's eyes glow with an ethereal light. 'The threads of fate have brought you here. Speak quickly, for time is a river that flows ever onward.'"
        );

        let response3 = create_message(
            5,
            MessageRole::Assistant,
            "Elara smiles knowingly. 'Another adventurer seeking answers in the written word. Very well, I shall aid you... for a price.'"
        );

        // Add swipes by branching from the same parent
        let parent_node = conversation.get_node_mut(Uuid::from_u128(2)).unwrap();
        parent_node.add_child(
            MessageNode {
                message: response2,
                children: Vec::new(),
                is_active: false,
            },
            false
        );
        parent_node.add_child(
            MessageNode {
                message: response3,
                children: Vec::new(),
                is_active: false,
            },
            false
        );

        // Update message count to reflect manually added swipes
        conversation.metadata.message_count += 2;

        println!("✓ Response 2 (swipe 2) added");
        println!("✓ Response 3 (swipe 3) added");

        // Verify swipes exist
        let swipes = conversation.get_swipes(Uuid::from_u128(3)).unwrap();
        assert_eq!(swipes.len(), 3);
        println!("✓ Verified 3 swipes exist at this position");

        // Step 4: User selects different swipe
        println!("\n=== Step 4: Switch to different swipe ===");

        assert_eq!(conversation.active_path.last(), Some(&Uuid::from_u128(3)));
        conversation.select_swipe(Uuid::from_u128(4)).unwrap();
        assert_eq!(conversation.active_path.last(), Some(&Uuid::from_u128(4)));
        println!("✓ Switched from swipe 1 to swipe 2");

        // Verify only swipe 2 is active
        assert!(!conversation.get_node(Uuid::from_u128(3)).unwrap().is_active);
        assert!(conversation.get_node(Uuid::from_u128(4)).unwrap().is_active);
        assert!(!conversation.get_node(Uuid::from_u128(5)).unwrap().is_active);

        // Step 5: Continue conversation from selected swipe
        println!("\n=== Step 5: Continue conversation ===");

        let user_msg2 = create_message(
            6,
            MessageRole::User,
            "What wisdom can you share about the Tome of Forgotten Lore?"
        );
        conversation.branch_from(Uuid::from_u128(4), user_msg2).unwrap();

        // Instantiate the item being discussed
        let tome_id = Uuid::from_u128(3000);
        let tome = storage.load_item(tome_id).unwrap();
        let tome_instance = ItemInstance {
            item_id: tome.id,
            name: tome.name.clone(),
            description: tome.description.clone(),
        };
        conversation.context.instantiated_entities.items.insert(tome_id, tome_instance);
        println!("✓ Tome of Forgotten Lore instantiated");

        let ai_response = create_message(
            7,
            MessageRole::Assistant,
            "Elara's expression grows solemn. 'That tome... it contains knowledge that mortals were never meant to possess. Tread carefully.'"
        );
        conversation.branch_from(Uuid::from_u128(6), ai_response).unwrap();
        println!("✓ Conversation continued");

        // Step 6: Instantiate location
        println!("\n=== Step 6: Instantiate location ===");

        let library_id = Uuid::from_u128(2000);
        let library = storage.load_location(library_id).unwrap();
        let library_instance = LocationInstance {
            location_id: library.id,
            name: library.name.clone(),
            description: library.description.clone(),
        };
        conversation.context.instantiated_entities.locations.insert(library_id, library_instance);
        println!("✓ The Ancient Library location instantiated");

        // Step 7: Delete a message (with reindex to keep later messages)
        println!("\n=== Step 7: Delete message with reindex ===");

        let count_before = count_messages(&conversation.root); // Get actual count from tree
        conversation.delete_message(Uuid::from_u128(6), true).unwrap();
        println!("✓ Deleted user message 6, promoted message 7 to child of 4");
        assert_eq!(conversation.metadata.message_count, count_before - 1);

        // Verify message 6 is gone but 7 still exists
        assert!(conversation.get_node(Uuid::from_u128(6)).is_err());
        assert!(conversation.get_node(Uuid::from_u128(7)).is_ok());

        // Verify active path updated correctly
        assert_eq!(
            conversation.active_path,
            vec![
                Uuid::from_u128(1),
                Uuid::from_u128(2),
                Uuid::from_u128(4),
                Uuid::from_u128(7),
            ]
        );

        // Step 8: Verify instantiated entities
        println!("\n=== Step 8: Verify instantiated entities ===");

        assert_eq!(conversation.context.instantiated_entities.characters.len(), 1);
        assert_eq!(conversation.context.instantiated_entities.locations.len(), 1);
        assert_eq!(conversation.context.instantiated_entities.items.len(), 1);

        assert!(conversation.context.instantiated_entities.characters.contains_key(&elara_id));
        assert!(conversation.context.instantiated_entities.locations.contains_key(&library_id));
        assert!(conversation.context.instantiated_entities.items.contains_key(&tome_id));

        println!("✓ 1 character instantiated: Elara the Wise");
        println!("✓ 1 location instantiated: The Ancient Library");
        println!("✓ 1 item instantiated: Tome of Forgotten Lore");

        // Step 9: Convert to storage and verify
        println!("\n=== Step 9: Save to storage ===");

        let bundle = conversation.to_storage();

        assert_eq!(bundle.conversation.id, Uuid::from_u128(conv_id));
        assert_eq!(bundle.conversation.context_type, "world");
        assert_eq!(bundle.conversation.context_id, Some(world_id));

        assert_eq!(bundle.messages.len(), 6); // 7 original - 1 deleted = 6
        assert_eq!(bundle.active_path.message_ids.len(), 4);
        assert_eq!(bundle.instantiated_entities.len(), 3); // 1 char + 1 loc + 1 item

        println!("✓ Conversation saved: {} messages", bundle.messages.len());
        println!("✓ Active path length: {}", bundle.active_path.message_ids.len());
        println!("✓ Instantiated entities: {}", bundle.instantiated_entities.len());

        // Step 10: Reload from storage and verify integrity
        println!("\n=== Step 10: Reload from storage ===");

        let reloaded = Conversation::from_storage(
            bundle.conversation,
            bundle.metadata,
            bundle.messages,
            bundle.active_path,
            bundle.instantiated_entities,
        ).unwrap();

        assert_eq!(reloaded.id, conversation.id);
        assert_eq!(reloaded.active_path.len(), conversation.active_path.len());
        assert_eq!(
            reloaded.context.instantiated_entities.characters.len(),
            conversation.context.instantiated_entities.characters.len()
        );
        assert_eq!(
            reloaded.context.instantiated_entities.locations.len(),
            conversation.context.instantiated_entities.locations.len()
        );
        assert_eq!(
            reloaded.context.instantiated_entities.items.len(),
            conversation.context.instantiated_entities.items.len()
        );

        // Verify character data preserved
        let reloaded_elara = reloaded.context.instantiated_entities.characters.get(&elara_id).unwrap();
        assert_eq!(reloaded_elara.name, "Elara the Wise");

        // Simulate context insertion
        assert!(reloaded_elara.persona.contains("ancient elven sage"));
        println!("✓ Context insertion simulated");

        println!("✓ Conversation reloaded successfully");
        println!("✓ All entity data preserved");

        // Step 11: Verify conversation thread
        println!("\n=== Step 11: Verify active thread ===");

        let thread = reloaded.get_active_thread();
        assert_eq!(thread.len(), 4);

        println!("Active conversation thread:");
        for (i, msg) in thread.iter().enumerate() {
            println!("  {}. [{:?}] {}", i + 1, msg.role,
                if msg.content.len() > 50 {
                    format!("{}...", &msg.content[..50])
                } else {
                    msg.content.clone()
                }
            );
        }

        println!("\n=== Test Complete ===");
        println!("✓ World-based conversation created");
        println!("✓ Entities instantiated (1 character, 1 location, 1 item)");
        println!("✓ Multiple swipes generated and selected");
        println!("✓ Message deletion with reindexing");
        println!("✓ Save/load cycle preserves all data");
        println!("✓ Active conversation thread intact");
    }

    // Helper for creating storage messages (using the one from main tests)
    fn create_storage_message(
        id: u128,
        conversation_id: u128,
        parent_id: Option<u128>,
        role: storage_system::MessageRole,
        content: &str,
    ) -> storage_system::Message {
        storage_system::Message {
            id: Uuid::from_u128(id),
            conversation_id: Uuid::from_u128(conversation_id),
            parent_message_id: parent_id.map(Uuid::from_u128),
            role,
            content: content.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_message(id: u128, role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage {
            id: Uuid::from_u128(id),
            role,
            content: content.to_string(),
            created_at: Utc::now(),
        }
    }
}
