use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::{
    routes::characters::types::ImportResult,
    services::character::{EditDefinition, EditMetadata},
};

#[derive(Deserialize)]
pub struct CreateCharacterRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct ImportCharResponse {
    pub results: Vec<ImportResult>,
}

/// A single edit applied against the current editing checkpoint.
///
/// The endpoint accepts a `Vec<CharacterEdit>`. All ops in one request land in
/// the same debounced version. Order is preserved and ops apply in sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CharacterEdit {
    Definition(EditDefinition),
    Metadata(EditMetadata),
    AddGreeting { text: String },
    UpdateGreeting { id: Snowflake, text: String },
    RemoveGreeting { id: Snowflake },
    ReorderGreetings { ids: Vec<Snowflake> },
    AddExample { content: String },
    UpdateExample { id: Snowflake, content: String },
    RemoveExample { id: Snowflake },
    ReorderExamples { ids: Vec<Snowflake> },
    SetTags { tags: Vec<String> },
    SetAltNames { alt_names: Vec<String> },
    SetWidgets { widget_ids: Vec<Snowflake> },
    SetLorebooks { lorebook_ids: Vec<Snowflake> },
    SetTools { tool_ids: Vec<Snowflake> },
}

#[derive(Deserialize)]
pub struct DuplicateAttachmentRequest {
    pub kind: crate::services::character::AttachmentKind,
    pub id: Snowflake,
}

#[derive(Serialize)]
pub struct DuplicateAttachmentResponse {
    pub character: lorealm_storage::models::Character,
    pub new_id: Snowflake,
}

#[derive(Serialize)]
pub struct AvatarUpdateResponse {
    pub avatar_uri: String,
}
