use serde::Serialize;

use crate::services::{
    cards::import::AttachmentReport,
    character::{ImportedCard, ImportedLorebook},
};

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImportResult {
    Ok {
        id: lorealm_storage::models::Snowflake,
        #[serde(skip_serializing_if = "Option::is_none")]
        lorebook: Option<ImportedLorebook>,
        #[serde(skip_serializing_if = "AttachmentReport::is_empty")]
        attachments: AttachmentReport,
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
            attachments: card.attachments,
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
        }
    }
}
