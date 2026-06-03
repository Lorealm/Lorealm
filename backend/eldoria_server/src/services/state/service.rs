use std::sync::Arc;

use eldoria_storage::models::{Character, Lorebook, Snowflake};

use crate::{
    errors::{AppError, StateError},
    services::{character::CharacterService, lorebook::LorebookService},
};

pub struct StateService {
    lorebooks: Arc<LorebookService>,
    characters: Arc<CharacterService>,
}

impl StateService {
    pub fn new(lorebooks: Arc<LorebookService>, characters: Arc<CharacterService>) -> Self {
        Self {
            lorebooks,
            characters,
        }
    }

    /// Retrieves the active character
    pub async fn get_active_character(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Character>, StateError> {
        Ok(None)
    }

    /// Retrieves an ordered list of active lorebooks
    pub async fn get_active_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Vec<Lorebook>>, StateError> {
        todo!()
    }
}
