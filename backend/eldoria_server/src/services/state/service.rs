use std::sync::Arc;

use eldoria_storage::models::{Character, Lorebook, Snowflake};

use crate::{errors::{AppError, StateError}, services::{character::CharacterService, lorebook::LorebookService}};

pub struct StateService {
    users: Arc<CharacterService>,
    lorebooks: Arc<LorebookService>,
    characters: Arc<CharacterService>,
}

impl StateService {
    pub fn new(
        users: Arc<CharacterService>,
        lorebooks: Arc<LorebookService>,
        characters: Arc<CharacterService>,
    ) -> Self {
        Self { users, lorebooks, characters }
    }

    /// Retrieves the active character
    pub async fn get_active_character(&self, user_id: Snowflake) -> Result<Option<Character>, StateError> {
        Ok(None)
    }

    /// Retrieves an ordered list of active lorebooks
    pub async fn get_active_lorebooks(&self, user_id: Snowflake) -> Result<Option<Vec<Lorebook>>, StateError> {
        todo!()
    }
}
