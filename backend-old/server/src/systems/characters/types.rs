use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub static ELDORIA_CHARACTER_SPEC: &str = "eldoria_chara_card_v1";
pub static ELDORIA_CHARACTER_VER: f32 = 1.0;

/// A wrapper trait for defining multiple definitions for the character data type
/// in case we need to extend it at a later point
pub trait CharacterDefinition<'a>: Serialize + Deserialize<'a> + 'a {
    fn load_plain(serialized_string: &'a str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(serialized_string)
    }

    fn load_from_image(path: &'a str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
/// The main character definition
pub struct EldoriaCharacter {
    pub spec: String,
    pub version: f32,
    pub metadata: EldoriaCharacterMetadata,
    pub definition: EldoriaCharacterDefinition,
    // pub lorebook: todo!()
}

#[derive(Serialize, Deserialize)]
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
    pub image_path: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct EldoriaCharacterDefinition {
    /// The actual in-chat name for this character
    pub name: String,
    /// The main substance of the character
    pub description: String,
    pub personality: String,
    pub example_messages: Vec<String>,
    pub greetings: Vec<String>
}

impl CharacterDefinition<'_> for EldoriaCharacterDefinition {}

#[derive(Serialize, Deserialize)]
/// The default Character V2 Specification (see https://github.com/bradennapier/character-cards-v2)
/// This is here for compatibility reasons only. 
/// 
/// See the `EldoriaCharacter` struct for the actual datatype we use throughout the application.
pub struct CharaV2 {
    spec: String,
    spec_version: String,
    data: CharaV2Data,
    #[serde(skip)]
    image_path: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct CharaV2Data {
    name: String,
    description: String,
    personality: String,
    scenario: String,
    first_mes: String,
    mes_example: String,
    creator_notes: String,
    system_prompt: String,
    post_history_instructions: String,
    alternate_greetings: Vec<String>,
    tags: Vec<String>,
    creator: String,
    character_version: String,
    extensions: HashMap<String, serde_json::Value>,
    character_book: CharaV2WorldInfo
}

#[derive(Serialize, Deserialize)]
pub struct CharaV2WorldInfo {
    name: String,
    description: String,
    scan_depth: usize,
    token_budget: usize,
    recursive_scanning: bool,
    extensions: HashMap<String, serde_json::Value>,
    entries: Vec<CharaV2WorldInfoEntry>
}

#[derive(Serialize, Deserialize)]
pub struct CharaV2WorldInfoEntry {
    keys: Vec<String>,
    content: String,
    extensions: HashMap<String, serde_json::Value>,
    enabled: bool,
    insertion_order: usize,
    case_sensitive: bool
}

impl Into<EldoriaCharacter> for CharaV2 {
    fn into(self) -> EldoriaCharacter {
        let example_messages = self.data.mes_example.split("<START>")
            .map(|x| x.to_string())
            .collect();

        let mut greetings = vec![self.data.first_mes];
        greetings.extend(self.data.alternate_greetings);

        EldoriaCharacter {
            spec: ELDORIA_CHARACTER_SPEC.to_string(),
            version: ELDORIA_CHARACTER_VER,
            metadata: EldoriaCharacterMetadata {
                display_name: None,
                alt_names: Vec::new(),
                tags: self.data.tags,
                creator: self.data.creator,
                creator_notes: self.data.creator_notes,
                version: self.spec_version,
                image_path: self.image_path
            }, 
            definition: EldoriaCharacterDefinition {
                name: self.data.name,
                description: self.data.description,
                personality: self.data.personality,
                example_messages,
                greetings
            }
        }
    }
}