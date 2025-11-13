use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub static ELDORIA_CHARACTER_SPEC: &str = "eldoria_chara";
pub static ELDORIA_CHARACTER_VER: f32 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The main character definition
pub struct EldoriaCharacter {
    pub id: Uuid,
    pub user_id: Uuid,
    pub spec: String,
    pub version: f32,
    pub metadata: EldoriaCharacterMetadata,
    pub definition: EldoriaCharacterDefinition,
    // pub lorebook: todo!()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EldoriaCharacterMetadata {
    /// The name to be displayed on the character's page `(None = same as definition name)`
    pub display_name: Option<String>,
    /// Alternate names. Used for sorting
    pub alt_names: Vec<String>,
    /// A list of arbitrary tags
    pub tags: Vec<String>,
    /// The creator of the card
    pub creator: String,
    pub creator_notes: String,
    /// An arbitrary version of the card
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub image_path: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EldoriaCharacterDefinition {
    /// The actual in-chat name for this character
    pub name: String,
    /// The main substance of the character
    pub description: String,
    pub personality: String,
    pub example_messages: Vec<String>,
    pub greetings: Vec<EldoriaCharacterGreeting>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EldoriaCharacterGreeting {
    pub order: usize,
    pub text_content: String
}

impl From<EldoriaCharacter> for storage_system::Character {
    fn from(character: EldoriaCharacter) -> Self {
        storage_system::Character {
            id: character.id,
            user_id: character.user_id,
            spec: character.spec,
            version: character.version,
            name: character.definition.name,
            display_name: character.metadata.display_name,
            personality: character.definition.personality,
            creator: character.metadata.creator,
            creator_notes: character.metadata.creator_notes,
            description: character.definition.description,
            created_at: character.metadata.created_at,
            updated_at: character.metadata.updated_at,
        }
    }
}

impl From<storage_system::Character> for EldoriaCharacter {
    fn from(character: storage_system::Character) -> Self {
        EldoriaCharacter {
            id: character.id,
            user_id: character.user_id,
            spec: character.spec,
            version: character.version,
            metadata: EldoriaCharacterMetadata {
                display_name: character.display_name,
                alt_names: vec![],
                tags: vec![],
                creator: character.creator,
                creator_notes: character.creator_notes,
                version: String::new(),
                created_at: character.created_at,
                updated_at: character.updated_at,
                image_path: None,
            },
            definition: EldoriaCharacterDefinition {
                name: character.name,
                description: character.description,
                personality: character.personality,
                example_messages: vec![],
                greetings: vec![],
            },
        }
    }
}

impl From<storage_system::CharacterGreeting> for EldoriaCharacterGreeting {
    fn from(value: storage_system::CharacterGreeting) -> Self {
        Self { order: value.display_order, text_content: value.greeting }
    }
}