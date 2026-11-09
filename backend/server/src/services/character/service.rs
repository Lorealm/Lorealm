use crate::services::{
    cards::import::{AttachmentReport, ImportOptions, import_attachments},
    character::AttachmentKind,
};
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
    /// What happened to the card's Lorealm attachments.
    pub attachments: AttachmentReport,
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
            lorebook_ids: Vec::new(),
            tool_ids: Vec::new(),
            widget_ids: Vec::new(),
            extensions: serde_json::Value::Null,
        };

        self.storage
            .save_character(&char, VersionedSaveMode::NewVersion)
            .await?;

        // A new character will likely be edited directly after creation
        self.editing_sessions.write().await.insert(
            char.id,
            EditingSession {
                version_id: char.current_version_id,
                last_touched: Instant::now(),
            },
        );

        Ok(char)
    }

    /// Delete a character together with every chat that uses it.
    pub async fn delete_character(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), CharacterError> {
        let latest = self.get_latest_version_id(character_id).await?;
        self.load_version(character_id, latest, user_id).await?;

        let chats = self.storage.list_conversations(user_id).await?;
        for chat in chats
            .iter()
            .filter(|c| c.character_id == Some(character_id))
        {
            self.storage.delete_conversation(chat.id).await?;
        }

        self.editing_sessions.write().await.remove(&character_id);
        self.storage.delete_character(character_id).await?;
        Ok(())
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
        if let Some(injection) = edit.template_injection {
            char.character.definition.template_injection = injection;
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

    /// Replace one kind of attachment on the character. Ids are checked against
    /// the caller's own resources, de-duplicated, and stored in the order given.
    pub async fn set_attachments(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        kind: AttachmentKind,
        ids: Vec<Snowflake>,
    ) -> Result<Character, CharacterError> {
        let mut resolved: Vec<Snowflake> = Vec::with_capacity(ids.len());
        for id in ids {
            if resolved.contains(&id) {
                continue;
            }
            self.check_attachable(user_id, kind, id).await?;
            resolved.push(id);
        }

        let mut char = self.get_character_for_edit(character_id, user_id).await?;
        *kind.ids_mut(&mut char.character) = resolved;

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    /// Replaces an attachement with another. Used when duplicating character
    /// attachements.
    pub async fn replace_attachment(
        &self,
        character_id: Snowflake,
        user_id: Snowflake,
        kind: AttachmentKind,
        old: Snowflake,
        new: Snowflake,
    ) -> Result<Character, CharacterError> {
        self.check_attachable(user_id, kind, new).await?;

        let mut char = self.get_character_for_edit(character_id, user_id).await?;
        let ids = kind.ids_mut(&mut char.character);
        let Some(slot) = ids.iter().position(|id| *id == old) else {
            return Err(CharacterError::NotFound(kind.target(), old));
        };
        ids[slot] = new;
        ids.dedup();

        self.storage
            .save_character(&char.character, char.save_mode)
            .await?;

        Ok(char.character)
    }

    /// The resource exists and belongs to `user_id`.
    async fn check_attachable(
        &self,
        user_id: Snowflake,
        kind: AttachmentKind,
        id: Snowflake,
    ) -> Result<(), CharacterError> {
        let owner = match kind {
            AttachmentKind::Lorebook => self.storage.get_lorebook(id).await?.map(|l| l.owner_id),
            AttachmentKind::Tool => self.storage.get_tool(id).await?.map(|t| t.user_id),
            AttachmentKind::Widget => self.storage.get_widget(id).await?.map(|w| w.user_id),
        };
        match owner {
            Some(owner) if owner == user_id => Ok(()),
            _ => Err(CharacterError::NotFound(kind.target(), id)),
        }
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
    pub async fn import_card(
        &self,
        bytes: &[u8],
        user_id: Snowflake,
        options: ImportOptions,
    ) -> Result<ImportedCard, CharacterError> {
        let is_json = bytes
            .iter()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|b| *b == b'{');
        let (json, image) = if is_json {
            (bytes.to_vec(), None)
        } else {
            let parsed = parse_character_card_png(bytes, CARD_IDENTIFIERS)?;
            if let DetectedSpec::Unknown(spec) = parsed.detected_spec {
                return Err(CharacterError::UnsupportedFormat(spec));
            }
            (parsed.json_bytes, Some(parsed.stripped_image))
        };

        // V3 is a superset of the CCv2 fields we read, so one converter serves
        // both. Fields unique to V3 ride along in `extensions`.
        let card: CharaV2 = serde_json::from_slice(&json)?;
        if !matches!(card.spec.as_str(), "chara_card_v2" | "chara_card_v3") {
            return Err(CharacterError::UnsupportedFormat(card.spec));
        }
        let ConvertedCard {
            mut character,
            lorebook,
            extension,
        } = convert_from_ccv2(card, user_id, &self.snowflake);
        let character_id = character.id;

        let imported_lorebook = match lorebook {
            Some((book, stats)) => {
                self.storage.save_lorebook(&book).await?;
                Some(stats)
            }
            None => None,
        };

        let attachments = import_attachments(
            self.storage.as_ref(),
            &self.snowflake,
            user_id,
            &extension,
            options,
        )
        .await?;
        if !attachments.lorebook_ids.is_empty() {
            character.lorebook_ids = attachments.lorebook_ids;
        }
        character.tool_ids = attachments.tool_ids;
        character.widget_ids = attachments.widget_ids;

        if let Some(image) = image {
            let blob_id = self.blob.put(&image).await?;
            character.metadata.avatar_uri = Some(blob_id.to_uri());
        }
        self.storage
            .save_character(&character, VersionedSaveMode::NewVersion)
            .await?;

        // An imported character is already complete, and should
        // therefore not be marked as editable immediately.
        // A new edit should create a new version on top of the imported card.

        Ok(ImportedCard {
            id: character_id,
            lorebook: imported_lorebook,
            attachments: attachments.report,
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
