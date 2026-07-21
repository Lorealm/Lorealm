use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use lorealm_storage::{
    BlobStore, Storage, StorageError,
    models::{
        BlobId, Character, CharacterDefinition, CharacterMetadata, CharacterSummary,
        CharacterVersionInfo, ExampleDialogue, Greeting, Snowflake, VersionedSaveMode,
    },
};
use tokio::{sync::RwLock, time::Instant};

use crate::{
    config::CharacterConfig,
    core::CleanableService,
    errors::{CharacterError, TargetType},
    services::{
        character::{
            EditDefinition, EditMetadata, EditableCharacter,
            card_parser::{CARD_IDENTIFIERS, DetectedSpec, parse_character_card_png},
            compat::{CharaV2, ConvertedCard, ImportedLorebook, convert_from_ccv2},
        },
        snowflake::SnowflakeService,
    },
};

/// The outcome of one successful card import.
pub struct ImportedCard {
    pub id: Snowflake,
    /// Set when the card carried a usable `character_book`.
    pub lorebook: Option<ImportedLorebook>,
}

struct EditingSession {
    version_id: Snowflake,
    last_touched: Instant,
}

pub struct CharacterService {
    storage: Arc<dyn Storage>,
    blob: Arc<dyn BlobStore>,
    snowflake: Arc<SnowflakeService>,
    config: CharacterConfig,
    /// The purpose of this is to map any edits on this character to a certain version number
    /// if it exists in the map. If nothing exists, a new version should be created.
    editing_sessions: RwLock<HashMap<Snowflake, EditingSession>>,
}

impl CharacterService {
    pub fn new(
        storage: Arc<dyn Storage>,
        blob: Arc<dyn BlobStore>,
        snowflake: Arc<SnowflakeService>,
        config: CharacterConfig,
    ) -> Self {
        Self {
            storage,
            blob,
            snowflake,
            config,
            editing_sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_character(
        &self,
        user_id: Snowflake,
        name: String,
    ) -> Result<Character, CharacterError> {
        let char = Character {
            id: self.snowflake.generate(),
            user_id,
            current_version_id: self.snowflake.generate(),
            metadata: CharacterMetadata::default(),
            definition: CharacterDefinition {
                name,
                ..Default::default()
            },
            lorebook_id: None,
            extensions: serde_json::Value::Null,
        };

        self.storage
            .save_character(&char, VersionedSaveMode::NewVersion)
            .await?;

        Ok(char)
    }

    pub async fn list_characters(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<CharacterSummary>, CharacterError> {
        Ok(self.storage.list_characters(user_id).await?)
    }

    pub async fn load_version(
        &self,
        character_id: Snowflake,
        version_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Character, CharacterError> {
        let character = self
            .storage
            .get_character_at_version(character_id, version_id)
            .await?
            .ok_or(CharacterError::NotFound(
                TargetType::CharacterVersion,
                character_id,
            ))?;

        if character.user_id != user_id {
            return Err(CharacterError::PermissionDenied);
        }

        Ok(character)
    }

    /// Retrieves the latest version of this character's ID
    ///
    /// **WARNING:** Ownership is not checked, and needs to be checked manually at a later time!
    pub async fn get_latest_version_id(
        &self,
        character_id: Snowflake,
    ) -> Result<Snowflake, CharacterError> {
        Ok(self.storage.get_latest_version(character_id).await?)
    }

    pub async fn list_versions(
        &self,
        character_id: Snowflake,
        _user_id: Snowflake,
    ) -> Result<Vec<CharacterVersionInfo>, CharacterError> {
        Ok(self.storage.list_character_versions(character_id).await?)
    }

    // Tries retrieving an avatar from blob storage using its uri.
    //
    // Returns an error if no blob is found
    pub async fn avatar_from_uri(
        &self,
        avatar_uri: Option<String>,
    ) -> Result<Bytes, CharacterError> {
        let blob_id =
            avatar_uri
                .as_deref()
                .and_then(BlobId::from_uri)
                .ok_or(StorageError::NotFound(
                    avatar_uri.unwrap_or("<null>".into()),
                ))?;

        let bytes = self
            .blob
            .get(&blob_id)
            .await?
            .ok_or(StorageError::NotFound(blob_id.to_uri()))?;

        Ok(bytes)
    }

    async fn get_character_for_edit(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<EditableCharacter, CharacterError> {
        let mut character =
            self.storage
                .get_character(character_id)
                .await?
                .ok_or(CharacterError::NotFound(
                    TargetType::Character,
                    character_id,
                ))?;
        if character.user_id != user_id {
            return Err(CharacterError::NotOwned(character_id));
        }
        let window = Duration::from_secs(self.config.edit_session_ttl);
        let (mode, target) = {
            let mut sessions = self.editing_sessions.write().await;
            match sessions.get_mut(&character.id) {
                Some(s) if s.last_touched.elapsed() < window => {
                    s.last_touched = Instant::now();
                    (VersionedSaveMode::UpdateCurrent, s.version_id)
                }
                _ => {
                    let v = self.snowflake.generate();
                    sessions.insert(
                        character.id,
                        EditingSession {
                            version_id: v,
                            last_touched: Instant::now(),
                        },
                    );
                    (VersionedSaveMode::NewVersion, v)
                }
            }
        };

        character.current_version_id = target;
        Ok(EditableCharacter {
            character,
            save_mode: mode,
        })
    }

    pub async fn patch_definition(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        edit: EditDefinition,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        if let Some(name) = edit.name {
            char.character.definition.name = name
        }
        if let Some(desc) = edit.description {
            char.character.definition.description = desc;
        }
        if let Some(personality) = edit.personality {
            char.character.definition.personality = personality;
        }
        if let Some(scenario) = edit.scenario {
            char.character.definition.scenario = scenario;
        }
        if let Some(system_prompt) = edit.system_prompt {
            char.character.definition.system_prompt = system_prompt;
        }
        if let Some(post_history) = edit.post_history_instructions {
            char.character.definition.post_history_instructions = post_history;
        }

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn patch_metadata(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        edit: EditMetadata,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        if let Some(creator) = edit.creator {
            char.character.metadata.creator = creator;
        }
        if let Some(notes) = edit.creator_notes {
            char.character.metadata.creator_notes = notes;
        }
        if let Some(version) = edit.card_version {
            char.character.metadata.card_version = version;
        }
        if let Some(display_name) = edit.display_name {
            char.character.metadata.display_name = display_name;
        }

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn add_greeting(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        greeting: String,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        let new_greeting = Greeting {
            id: self.snowflake.generate(),
            order: (char.character.definition.greetings.len() + 1) as u32,
            text: greeting,
        };

        char.character.definition.greetings.push(new_greeting);

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn update_greeting(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        greeting_id: Snowflake,
        text: String,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        let greeting = char
            .character
            .definition
            .greetings
            .iter_mut()
            .find(|g| g.id == greeting_id)
            .ok_or(CharacterError::NotFound(TargetType::Greeting, greeting_id))?;

        greeting.text = text;

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn remove_greeting(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        greeting_id: Snowflake,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        char.character
            .definition
            .greetings
            .retain(|g| g.id != greeting_id);

        for (index, greeting) in char.character.definition.greetings.iter_mut().enumerate() {
            greeting.order = index as u32;
        }

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn reorder_greetings(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        greeting_ids: Vec<Snowflake>,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        char.character.definition.greetings.sort_by_key(|greeting| {
            greeting_ids
                .iter()
                .position(|&id| id == greeting.id)
                .unwrap_or(usize::MAX) // If not found, move to the end
        });

        for (index, greeting) in char.character.definition.greetings.iter_mut().enumerate() {
            greeting.order = index as u32;
        }

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn add_example(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        content: String,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        let new_example = ExampleDialogue {
            id: self.snowflake.generate(),
            order: (char.character.definition.example_messages.len() + 1) as u32,
            content,
        };

        char.character.definition.example_messages.push(new_example);

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn update_example(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        example_id: Snowflake,
        content: String,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        let example = char
            .character
            .definition
            .example_messages
            .iter_mut()
            .find(|g| g.id == example_id)
            .ok_or(CharacterError::NotFound(
                TargetType::ExampleMessage,
                example_id,
            ))?;

        example.content = content;

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn remove_example(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        example_id: Snowflake,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        char.character
            .definition
            .example_messages
            .retain(|g| g.id != example_id);

        for (index, example) in char
            .character
            .definition
            .example_messages
            .iter_mut()
            .enumerate()
        {
            example.order = index as u32;
        }

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn reorder_example(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        example_ids: Vec<Snowflake>,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;

        char.character
            .definition
            .example_messages
            .sort_by_key(|example| {
                example_ids
                    .iter()
                    .position(|&id| id == example.id)
                    .unwrap_or(usize::MAX) // If not found, move to the end
            });

        for (index, example) in char
            .character
            .definition
            .example_messages
            .iter_mut()
            .enumerate()
        {
            example.order = index as u32;
        }

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn set_tags(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        tags: Vec<String>,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;
        char.character.metadata.tags = tags;

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn set_alt_names(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        alt_names: Vec<String>,
    ) -> Result<Character, CharacterError> {
        let mut char = self.get_character_for_edit(character_id, user_id).await?;
        char.character.metadata.alt_names = alt_names;

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    pub async fn update_image(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        image_bytes: Bytes,
    ) -> Result<String, CharacterError> {
        let mut edit = self.get_character_for_edit(character_id, user_id).await?;
        let blob_id = self.blob.put(&image_bytes).await?;
        let image_uri = blob_id.to_uri();
        edit.character.metadata.avatar_uri = Some(image_uri.clone());
        self.storage
            .save_character(&edit.character, edit.save_mode)
            .await?;

        Ok(image_uri)
    }

    /// Import a character card from a PNG. Parses the embedded tEXt chunk,
    /// converts to the domain `Character` type, and persists it.
    ///
    /// A card carrying a `character_book` also yields a lorebook, saved first
    /// so the character never points at a book that isn't there.
    pub async fn import_from_png(
        &self,
        png_bytes: &[u8],
        user_id: Snowflake,
    ) -> Result<ImportedCard, CharacterError> {
        let parsed = parse_character_card_png(png_bytes, CARD_IDENTIFIERS)?;

        // V3 is a superset of the CCv2 fields we read, so one converter serves
        // both. Fields unique to V3 ride along in `extensions`.
        let converted = match parsed.detected_spec {
            DetectedSpec::CharaV2 | DetectedSpec::CharaV3 => {
                let card: CharaV2 = serde_json::from_slice(&parsed.json_bytes)?;
                convert_from_ccv2(card, user_id, &self.snowflake)
            }
            DetectedSpec::Unknown(spec) => {
                return Err(CharacterError::UnsupportedFormat(spec));
            }
        };

        let ConvertedCard {
            mut character,
            lorebook,
        } = converted;
        let character_id = character.id;

        let imported_lorebook = match lorebook {
            Some((book, stats)) => {
                self.storage.save_lorebook(&book).await?;
                Some(stats)
            }
            None => None,
        };

        // This always exists due to the scope of the function
        let blob_id = self.blob.put(&parsed.stripped_image).await?;
        character.metadata.avatar_uri = Some(blob_id.to_uri());
        self.storage
            .save_character(&character, VersionedSaveMode::NewVersion)
            .await?;

        Ok(ImportedCard {
            id: character_id,
            lorebook: imported_lorebook,
        })
    }
}

#[async_trait]
impl CleanableService for CharacterService {
    fn name(&self) -> &'static str {
        "CharacterService"
    }
    async fn clean(&self) -> usize {
        let mut sessions = self.editing_sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| {
            s.last_touched.elapsed() < Duration::from_secs(self.config.edit_session_ttl)
        });
        before - sessions.len()
    }
}
