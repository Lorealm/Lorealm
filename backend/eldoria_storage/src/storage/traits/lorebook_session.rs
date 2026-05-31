//! Per-conversation weaver session state.
//!
//! NOT a versioned aggregate. High-frequency, last-write-wins, keyed by
//! conversation. Persisted once per turn after `advance_turn()` +
//! `assemble()`, ideally in the same transaction as the message write.
//!
//! `LorebookSession` model holds:
//!   conversation_id:    Snowflake  (PK)
//!   lorebook_id:        Snowflake  (+ optional version pin for reproducibility)
//!   activation_state:   serde_json::Value  (serialized `ActivationState`)
//!   persistent_state:   serde_json::Value  (serialized `HashMap<String, Value>`)
//!   updated_at:         timestamptz

use crate::error::StorageError;
use crate::models::{LorebookSession, Snowflake};
use async_trait::async_trait;

#[async_trait]
pub trait LorebookSessionStorage: Send + Sync {
    /// Load the saved session for a conversation. `None` if the conversation
    /// has no persisted weaver state yet (treat as a fresh engine).
    async fn get_session(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<LorebookSession>, StorageError>;

    /// Upsert the session. Last-write-wins on `conversation_id`.
    async fn save_lorebook_session(&self, session: &LorebookSession) -> Result<(), StorageError>;

    /// Drop a conversation's saved state. Returns `true` if a row existed.
    async fn delete_lorebook_session(&self, conversation_id: Snowflake) -> Result<bool, StorageError>;
}
