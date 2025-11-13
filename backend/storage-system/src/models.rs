use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Context source info
    pub context_type: String, // "empty", "character", "scenario", "world", "custom"
    pub context_id: Option<Uuid>,
    pub context_version: Option<u64>,
    pub context_metadata: Option<String>, // JSON for custom contexts

    // Sync policy
    pub sync_policy: String, // Serialized SyncPolicy enum
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub title: Option<String>,
    pub message_count: usize,
    pub last_message_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub parent_message_id: Option<Uuid>, // Which message this follows
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Track which path through the tree is currently "active"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePath {
    pub conversation_id: Uuid,
    pub message_ids: Vec<Uuid>, // Ordered list of message IDs in the current path
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiatedEntity {
    pub id: Uuid, // Entity instance ID (for updates)
    pub conversation_id: Uuid,
    pub entity_id: Uuid,       // Original entity ID in world/character db
    pub entity_type: String,   // "character", "location", "item", etc.
    pub snapshot_data: String, // JSON snapshot of the entity
    pub instantiated_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct User {
    pub id: Uuid,
    pub permissions: u64,
    pub email: Option<String>,
    pub display_name: Option<String>,
    /// Should only be retrieved when logging in
    pub password: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub csrf_token: String,
    pub csrf_rotation_count: u32,
    pub csrf_last_rotated_at: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub added_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Character {
    pub id: Uuid,
    pub user_id: Uuid,
    pub spec: String,
    pub version: f32,
    pub name: String,
    pub display_name: Option<String>,
    pub personality: String,
    pub creator: String,
    pub creator_notes: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterTag {
    pub character_id: Uuid,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterGreeting {
    pub id: Uuid,
    pub character_id: Uuid,
    pub greeting: String,
    pub display_order: usize,
}
