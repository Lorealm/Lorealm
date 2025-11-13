use crate::{
    error::StorageError,
    impl_storage_capabilities,
    models::*,
    storage::{CharacterStorage, ConversationStorage, MessageStorage, StorageCore, UserStorage},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Debug)]
pub struct FileStorage {
    base_path: PathBuf,
    pretty: bool,
}

impl FileStorage {
    pub fn new(base_path: impl Into<PathBuf>, pretty: bool) -> Result<Self, StorageError> {
        let base_path = base_path.into();

        // Convert to absolute path without requiring it to exist
        let base_path = if base_path.is_absolute() {
            base_path
        } else {
            std::env::current_dir()
                .map_err(|e| {
                    StorageError::Configuration(format!("Failed to get current directory: {e}"))
                })?
                .join(base_path)
        };

        Ok(Self { base_path, pretty })
    }

    /// Safely construct a path within the base directory
    /// This prevents path traversal attacks
    fn safe_path(&self, components: &[&str]) -> Result<PathBuf, StorageError> {
        let mut path = self.base_path.clone();

        for component in components {
            // Validate each component
            Self::validate_path_component(component)?;
            path.push(component);
        }

        // Normalize the path to resolve any .. or . components
        let normalized = self.normalize_path(&path)?;

        // Ensure the final path is still within base_path
        if !normalized.starts_with(&self.base_path) {
            return Err(StorageError::InvalidOperation(
                "Path traversal attempt detected".to_string(),
            ));
        }

        Ok(normalized)
    }

    /// Normalize a path by resolving . and .. components without requiring it to exist
    fn normalize_path(&self, path: &Path) -> Result<PathBuf, StorageError> {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                std::path::Component::RootDir => normalized.push("/"),
                std::path::Component::CurDir => {} // Skip "."
                std::path::Component::ParentDir => {
                    // Go up one level
                    if !normalized.pop() {
                        return Err(StorageError::InvalidOperation(
                            "Path traversal above root".to_string(),
                        ));
                    }
                }
                std::path::Component::Normal(name) => normalized.push(name),
            }
        }

        Ok(normalized)
    }

    /// Validate a single path component
    fn validate_path_component(component: &str) -> Result<(), StorageError> {
        let component = component.to_string();
        // Check for empty or suspicious components
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.contains('/')
            || component.contains('\\')
            || component.contains('\0')
        {
            return Err(StorageError::InvalidOperation(format!(
                "Invalid path component: {component}"
            )));
        }

        // Ensure it's a valid filename (basic check)
        if component.len() > 255 {
            return Err(StorageError::InvalidOperation(
                "Path component too long".to_string(),
            ));
        }

        Ok(())
    }

    /// Sanitize an ID to ensure it's safe for filesystem use
    fn sanitize_id(id: &str) -> Result<String, StorageError> {
        // Validate UUID format or create hash-based name
        if Uuid::parse_str(id).is_ok()
            || (id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                && id.len() <= 100)
        {
            Ok(id.to_string())
        } else {
            // For non-standard IDs, create a hash-based safe name
            let mut hasher = Sha256::new();
            hasher.update(id.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            Ok(hash[..16].to_string()) // Use first 16 chars of hash
        }
    }

    /// Create sharded directory structure for better filesystem performance
    /// For example: conversations/ab/cd/abcd1234-5678-...
    fn sharded_path(&self, base: &str, id: &str) -> Result<PathBuf, StorageError> {
        let safe_id = Self::sanitize_id(id)?;

        // Create shard directories based on first 4 chars
        let (shard1, shard2) = if safe_id.len() >= 4 {
            (&safe_id[0..2], &safe_id[2..4])
        } else {
            ("00", "00")
        };

        self.safe_path(&[base, shard1, shard2, &safe_id])
    }

    /// Get conversation directory path
    fn conversation_path(&self, conversation_id: &str) -> Result<PathBuf, StorageError> {
        self.sharded_path("conversations", conversation_id)
    }

    /// Get messages directory for a conversation
    fn messages_path(&self, conversation_id: &str) -> Result<PathBuf, StorageError> {
        let conv_path = self.conversation_path(conversation_id)?;
        Ok(conv_path.join("messages"))
    }

    /// Get user directory path
    fn user_path(&self, user_id: &str) -> Result<PathBuf, StorageError> {
        self.sharded_path("users", user_id)
    }

    /// Get session directory path
    fn session_path(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        self.sharded_path("sessions", session_id)
    }

    /// Atomically write file with temp + rename pattern
    async fn atomic_write(&self, path: &Path, content: &str) -> Result<(), StorageError> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content).await?;

        // Atomic rename
        fs::rename(&temp_path, path).await?;

        Ok(())
    }

    /// Read and validate JSON file
    async fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<Option<T>, StorageError> {
        match fs::read_to_string(path).await {
            Ok(content) => {
                // Validate JSON before deserializing
                let value: serde_json::Value = serde_json::from_str(&content)?;
                let typed: T = serde_json::from_value(value)?;
                Ok(Some(typed))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[async_trait]
impl StorageCore for FileStorage {
    async fn is_available(&self) -> bool {
        // Try to create the base path if it doesn't exist
        if (tokio::fs::create_dir_all(&self.base_path).await).is_err() {
            return false;
        }

        // Check write permissions
        let test_file = self.base_path.join(".write_test");
        let can_write = tokio::fs::write(&test_file, b"test").await.is_ok();
        let can_delete = tokio::fs::remove_file(&test_file).await.is_ok();

        can_write && can_delete
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        // Create base directory
        fs::create_dir_all(&self.base_path).await?;

        // Create base directory structure
        fs::create_dir_all(self.safe_path(&["conversations"])?).await?;
        fs::create_dir_all(self.safe_path(&["users"])?).await?;
        fs::create_dir_all(self.safe_path(&["sessions"])?).await?;
        fs::create_dir_all(self.safe_path(&["temp"])?).await?;

        // Create a metadata file to track storage version
        let metadata = serde_json::json!({
            "version": "1.0",
            "created_at": chrono::Utc::now(),
            "storage_type": "file"
        });

        let metadata_path = self.safe_path(&["storage.meta"])?;
        self.atomic_write(&metadata_path, &serde_json::to_string_pretty(&metadata)?)
            .await?;

        Ok(())
    }

    fn backend_type(&self) -> &str {
        "file"
    }
}

#[async_trait]
impl ConversationStorage for FileStorage {
    async fn save_conversation(&self, conversation: &Conversation) -> Result<Uuid, StorageError> {
        // Validate conversation ID
        Self::validate_path_component(&conversation.id.to_string())?;

        let conv_dir = self.conversation_path(&conversation.id.to_string())?;
        let meta_path = conv_dir.join("conversation.json");

        let content = if self.pretty {
            serde_json::to_string_pretty(conversation)?
        } else {
            serde_json::to_string(conversation)?
        };

        self.atomic_write(&meta_path, &content).await?;
        Ok(conversation.id)
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, StorageError> {
        Self::validate_path_component(&id.to_string())?;

        let conv_dir = self.conversation_path(&id.to_string())?;
        let meta_path = conv_dir.join("conversation.json");

        self.read_json(&meta_path).await
    }

    async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        Self::validate_path_component(&user_id.to_string())?;

        let conversations_base = self.safe_path(&["conversations"])?;

        // Check if conversations directory exists
        if !conversations_base.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        // Walk through sharded directories
        let mut stack = vec![conversations_base];

        while let Some(dir) = stack.pop() {
            let mut entries = match fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() {
                    // Check if this is a conversation directory
                    let meta_path = path.join("conversation.json");
                    if meta_path.exists() {
                        if let Ok(Some(conv)) = self.read_json::<Conversation>(&meta_path).await {
                            if conv.user_id == user_id {
                                // Count messages and find last message timestamp
                                let messages_dir = path.join("messages");
                                let (message_count, last_message_at) = if messages_dir.exists() {
                                    let mut count = 0;
                                    let mut last_message_at = conv.created_at;

                                    if let Ok(mut entries) = fs::read_dir(&messages_dir).await {
                                        while let Ok(Some(entry)) = entries.next_entry().await {
                                            count += 1;
                                            let msg_path = entry.path();
                                            if let Ok(Some(msg)) =
                                                self.read_json::<Message>(&msg_path).await
                                            {
                                                if msg.created_at > last_message_at {
                                                    last_message_at = msg.created_at;
                                                }
                                            }
                                        }
                                    }
                                    (count, last_message_at)
                                } else {
                                    (0, conv.created_at)
                                };

                                // Check for metadata file to get title
                                let metadata_path = path.join("metadata.json");
                                let title = if metadata_path.exists() {
                                    if let Ok(Some(stored_meta)) =
                                        self.read_json::<ConversationMetadata>(&metadata_path).await
                                    {
                                        stored_meta.title
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                results.push(ConversationMetadata {
                                    id: Uuid::new_v4(), // Generate new ID for metadata entry
                                    conversation_id: conv.id,
                                    title,
                                    created_at: conv.created_at,
                                    updated_at: conv.updated_at,
                                    message_count,
                                    last_message_at,
                                });
                            }
                        }
                    } else {
                        // It's a shard directory, continue traversing
                        stack.push(path);
                    }
                }
            }
        }

        // Sort by updated_at descending
        results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(results)
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), StorageError> {
        Self::validate_path_component(&id.to_string())?;

        let conv_dir = self.conversation_path(&id.to_string())?;

        if conv_dir.exists() {
            fs::remove_dir_all(&conv_dir).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl MessageStorage for FileStorage {
    async fn save_message(&self, message: &Message) -> Result<Uuid, StorageError> {
        Self::validate_path_component(&message.id.to_string())?;
        Self::validate_path_component(&message.conversation_id.to_string())?;

        let messages_dir = self.messages_path(&message.conversation_id.to_string())?;
        let message_path = messages_dir.join(format!("{}.json", message.id));

        let content = if self.pretty {
            serde_json::to_string_pretty(message)?
        } else {
            serde_json::to_string(message)?
        };

        self.atomic_write(&message_path, &content).await?;
        Ok(message.id)
    }

    async fn get_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, StorageError> {
        Self::validate_path_component(&conversation_id.to_string())?;

        let messages_dir = self.messages_path(&conversation_id.to_string())?;

        if !messages_dir.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        let mut entries = fs::read_dir(&messages_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(Some(message)) = self.read_json::<Message>(&path).await {
                    messages.push(message);
                }
            }
        }

        // Sort by created_at
        messages.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(messages)
    }
}

#[async_trait]
impl UserStorage for FileStorage {
    async fn save_user(&self, user: &User) -> Result<Uuid, StorageError> {
        Self::validate_path_component(&user.id.to_string())?;

        let user_dir = self.user_path(&user.id.to_string())?;
        let user_path = user_dir.join("user.json");

        let content = if self.pretty {
            serde_json::to_string_pretty(user)?
        } else {
            serde_json::to_string(user)?
        };

        self.atomic_write(&user_path, &content).await?;
        Ok(user.id)
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, StorageError> {
        Self::validate_path_component(&id.to_string())?;

        let user_dir = self.user_path(&id.to_string())?;
        let user_path = user_dir.join("user.json");

        self.read_json(&user_path).await
    }

    async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StorageError> {
        // Scan through all user directories to find matching username
        // Note: This is inefficient for file-based storage, but necessary without indexing
        let users_base = self.safe_path(&["users"])?;

        if !users_base.exists() {
            return Ok(None);
        }

        // Walk through sharded directories
        let mut stack = vec![users_base];

        while let Some(dir) = stack.pop() {
            let mut entries = match fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() {
                    // Check if this is a user directory
                    let user_path = path.join("user.json");
                    if user_path.exists() {
                        if let Ok(Some(user)) = self.read_json::<User>(&user_path).await {
                            if let Some(display_name) = &user.display_name {
                                if display_name == name {
                                    return Ok(Some(user));
                                }
                            }
                        }
                    } else {
                        // It's a shard directory, continue traversing
                        stack.push(path);
                    }
                }
            }
        }

        Ok(None)
    }

    async fn save_session(&self, session: &Session) -> Result<Uuid, StorageError> {
        Self::validate_path_component(&session.id.to_string())?;

        let session_dir = self.session_path(&session.id.to_string())?;
        let session_path = session_dir.join("session.json");

        let content = if self.pretty {
            serde_json::to_string_pretty(session)?
        } else {
            serde_json::to_string(session)?
        };

        self.atomic_write(&session_path, &content).await?;
        Ok(session.id)
    }

    async fn get_session_by_id(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        Self::validate_path_component(&id.to_string())?;

        let session_dir = self.session_path(&id.to_string())?;
        let session_path = session_dir.join("session.json");

        self.read_json(&session_path).await
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        // Scan through all session directories to find matching token
        // Note: This is inefficient for file-based storage, but necessary without indexing
        let sessions_base = self.safe_path(&["sessions"])?;

        if !sessions_base.exists() {
            return Ok(None);
        }

        // Walk through sharded directories
        let mut stack = vec![sessions_base];

        while let Some(dir) = stack.pop() {
            let mut entries = match fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() {
                    // Check if this is a session directory
                    let session_path = path.join("session.json");
                    if session_path.exists() {
                        if let Ok(Some(session)) = self.read_json::<Session>(&session_path).await {
                            if session.token == token {
                                return Ok(Some(session));
                            }
                        }
                    } else {
                        // It's a shard directory, continue traversing
                        stack.push(path);
                    }
                }
            }
        }

        Ok(None)
    }

    async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        Self::validate_path_component(&session_id.to_string())?;

        let session_dir = self.session_path(&session_id.to_string())?;
        let session_path = session_dir.join("session.json");

        // Read existing session
        let mut session: Session = match self.read_json(&session_path).await? {
            Some(session) => session,
            None => {
                return Err(StorageError::NotFound(format!(
                    "Session {session_id} not found"
                )));
            }
        };

        // Update fields
        session.last_active_at = last_active_at;
        if let Some(expires_at) = new_expiration {
            session.expires_at = expires_at;
        }

        // Write back
        let content = if self.pretty {
            serde_json::to_string_pretty(&session)?
        } else {
            serde_json::to_string(&session)?
        };

        self.atomic_write(&session_path, &content).await?;
        Ok(())
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        Self::validate_path_component(&session_id.to_string())?;

        let session_dir = self.session_path(&session_id.to_string())?;

        if session_dir.exists() {
            fs::remove_dir_all(&session_dir).await?;
        }

        Ok(())
    }

    async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        Self::validate_path_component(&session_id.to_string())?;

        let session_dir = self.session_path(&session_id.to_string())?;
        let session_path = session_dir.join("session.json");

        let mut session: Session = match self.read_json(&session_path).await? {
            Some(session) => session,
            None => {
                return Err(StorageError::NotFound(format!(
                    "Session {session_id} not found"
                )));
            }
        };

        session.csrf_token = new_csrf_token;
        session.csrf_rotation_count = rotation_count;
        session.csrf_last_rotated_at = rotated_at;

        let content = if self.pretty {
            serde_json::to_string_pretty(&session)?
        } else {
            serde_json::to_string(&session)?
        };

        self.atomic_write(&session_path, &content).await?;
        Ok(())
    }
}

impl CharacterStorage for FileStorage {}

impl_storage_capabilities!(FileStorage, {
    supports_conversations: true,
    supports_messages: true,
    supports_users: true,
    supports_sessions: true,
    supports_characters: false
});
