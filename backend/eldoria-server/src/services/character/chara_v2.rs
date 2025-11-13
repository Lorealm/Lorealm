use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::character::{types::{EldoriaCharacter, EldoriaCharacterDefinition, EldoriaCharacterMetadata, ELDORIA_CHARACTER_SPEC, ELDORIA_CHARACTER_VER}, EldoriaCharacterGreeting};

#[derive(Serialize, Deserialize)]
/// The default Character V2 Specification (see https://github.com/bradennapier/character-cards-v2)
/// This is here for compatibility reasons only. 
/// 
/// See the `EldoriaCharacter` struct for the actual datatype we use throughout the application.
pub struct CharaV2 {
    pub spec: String,
    pub spec_version: String,
    pub data: CharaV2Data,
    #[serde(skip)]
    pub image_path: Option<String>,
    #[serde(skip)]
    pub user_id: Uuid
}

#[derive(Serialize, Deserialize)]
pub struct CharaV2Data {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
    pub creator_notes: String,
    pub system_prompt: String,
    pub post_history_instructions: String,
    pub alternate_greetings: Vec<String>,
    pub tags: Vec<String>,
    pub creator: String,
    pub character_version: String,
    pub extensions: HashMap<String, serde_json::Value>,
    pub character_book: Option<CharaV2WorldInfo>
}

#[derive(Serialize, Deserialize)]
pub struct CharaV2WorldInfo {
    pub name: String,
    pub description: String,
    pub scan_depth: usize,
    pub token_budget: usize,
    pub recursive_scanning: bool,
    pub extensions: HashMap<String, serde_json::Value>,
    pub entries: Vec<CharaV2WorldInfoEntry>
}

#[derive(Serialize, Deserialize)]
pub struct CharaV2WorldInfoEntry {
    pub keys: Vec<String>,
    pub content: String,
    pub extensions: HashMap<String, serde_json::Value>,
    pub enabled: bool,
    pub insertion_order: usize,
    pub case_sensitive: bool
}

impl From<CharaV2> for EldoriaCharacter {
    fn from(chara: CharaV2) -> Self {
        let example_messages = chara.data.mes_example.split("<START>")
            .map(|x| x.to_string())
            .collect();

        let mut greetings = vec![chara.data.first_mes];
        greetings.extend(chara.data.alternate_greetings);
        
        let char_greetings = greetings.iter()
            .enumerate()
            .map(|(index, x)| EldoriaCharacterGreeting {
                order: index,
                text_content: x.to_owned(),
            })
            .collect();

        EldoriaCharacter {
            spec: ELDORIA_CHARACTER_SPEC.to_string(),
            version: ELDORIA_CHARACTER_VER,
            id: Uuid::new_v4(),
            user_id: chara.user_id,
            metadata: EldoriaCharacterMetadata {
                display_name: None,
                alt_names: Vec::new(),
                tags: chara.data.tags,
                creator: chara.data.creator,
                creator_notes: chara.data.creator_notes,
                version: chara.spec_version,
                image_path: chara.image_path,
                created_at: Utc::now(),
                updated_at: Utc::now()
            }, 
            definition: EldoriaCharacterDefinition {
                name: chara.data.name,
                description: chara.data.description,
                personality: chara.data.personality,
                example_messages,
                greetings: char_greetings
            }
        }
    }
}