use eldoria_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::{routes::characters::types::ImportResult, services::character::{EditDefinition, EditMetadata}};

#[derive(Serialize)]
pub struct ImportCharResponse {
    pub results: Vec<ImportResult>,
}

/// A single edit applied against the current editing checkpoint.
///
/// The endpoint accepts a `Vec<CharacterEdit>`; all ops in one request land in
/// the same debounced version. Order is preserved and ops apply in sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CharacterEdit {
    // --- scalar field groups (merge-patch: None = leave unchanged) ---
    Definition(EditDefinition),
    Metadata(EditMetadata),

    // --- greetings (item-level, identity via Snowflake) ---
    AddGreeting {
        text: String,
    },
    UpdateGreeting {
        id: Snowflake,
        text: String,
    },
    RemoveGreeting {
        id: Snowflake,
    },
    /// Full ordered id list. `order` stays server-authoritative.
    ReorderGreetings {
        ids: Vec<Snowflake>,
    },

    // --- example dialogues (item-level, identity via Snowflake) ---
    AddExample {
        content: String,
    },
    UpdateExample {
        id: Snowflake,
        content: String,
    },
    RemoveExample {
        id: Snowflake,
    },
    ReorderExamples {
        ids: Vec<Snowflake>,
    },

    // --- whole-list replace (no per-item identity needed) ---
    SetTags {
        tags: Vec<String>,
    },
    SetAltNames {
        alt_names: Vec<String>,
    },
}
