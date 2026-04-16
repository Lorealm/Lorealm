use std::sync::Arc;

use eldoria_storage::{Storage, models::Snowflake};

use crate::services::{
    character::{
        CharacterServiceError,
        card_parser::{DetectedSpec, parse_character_card_png},
        compat::{CharaV2, convert_from_ccv2},
    },
    snowflake::SnowflakeService,
};

pub struct CharacterService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
}

impl CharacterService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    /// Import a character card from a PNG. Parses the embedded tEXt chunk,
    /// converts to the domain `Character` type, and persists it.
    ///
    /// Returns the new character's id on success.
    pub async fn import_from_png(
        &self,
        png_bytes: &[u8],
        user_id: Snowflake,
    ) -> Result<Snowflake, CharacterServiceError> {
        let parsed = parse_character_card_png(&png_bytes, "chara")?;

        let character = match parsed.detected_spec {
            DetectedSpec::CharaV2 => {
                let card: CharaV2 = serde_json::from_slice(&parsed.json_bytes)?;
                convert_from_ccv2(card, user_id, &self.snowflake)
            }
            DetectedSpec::Unknown(spec) => {
                return Err(CharacterServiceError::UnsupportedFormat(spec));
            }
        };

        let character_id = character.id;
        self.storage.save_character(&character).await?;

        // TODO: persist parsed.stripped_image as the avatar, then update
        // character.metadata.avatar_uri via a follow-up save. Deferred
        // until blob storage is decided.

        Ok(character_id)
    }
}
