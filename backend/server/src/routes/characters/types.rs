use serde::Serialize;

use crate::services::character::{ImportedCard, ImportedLorebook};

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImportResult {
    Ok {
        id: lorealm_storage::models::Snowflake,
        /// Present when the card shipped a `character_book` that converted.
        /// Purely informational — the client uses it for feedback, not to
        /// decide anything.
        #[serde(skip_serializing_if = "Option::is_none")]
        lorebook: Option<ImportedLorebook>,
    },
    Failed {
        reason: String,
    },
}

impl ImportResult {
    pub fn ok(card: ImportedCard) -> Self {
        Self::Ok {
            id: card.id,
            lorebook: card.lorebook,
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
        }
    }
}
