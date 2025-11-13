use std::sync::Arc;

use storage_system::{Storage, StorageError};
use uuid::Uuid;

use crate::{models::StorageService, services::character::EldoriaCharacter};

pub struct CharacterService<S: Storage + ?Sized> {
    storage: Arc<S>,
}

impl<S: Storage + ?Sized> StorageService<S> for CharacterService<S> {
    fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
}

impl<S: Storage + ?Sized> CharacterService<S> {
    pub async fn load_character(&self, character_id: Uuid) -> Result<EldoriaCharacter, StorageError> {
        if let Ok(Some(character)) = self.storage.get_character(character_id).await {
            let mut convert = EldoriaCharacter::from(character); // Populates core fields
            
            if let Ok(greetings) = self.storage.get_character_greetings(character_id).await {
                convert.definition.greetings = greetings.into_iter().map(|x| x.into()).collect();
            }
            if let Ok(tags) = self.storage.get_character_tags(character_id).await {
                convert.metadata.tags = tags.into_iter().map(|x| x.tag).collect();
            }
            
            return Ok(convert);
        }
        
        Err(StorageError::NotFound(format!("CharacterService: Character with id '{character_id}' not found.")))
    }
    
    pub async fn save_character(&self, character: &EldoriaCharacter) -> Result<Uuid, StorageError> {
        let greetings = character.definition.greetings
            .iter()
            .map(|x| storage_system::CharacterGreeting {
                id: Uuid::new_v4(),
                character_id: character.id,
                greeting: x.text_content.clone(),
                display_order: x.order,
            });
        
        let tags = character.metadata.tags
            .iter()
            .map(|x| storage_system::CharacterTag {
                character_id: character.id,
                tag: x.clone(),
            });
        
        let return_id = self.storage.save_character(&character.clone().into()).await?;
        
        for greeting in greetings {
            self.storage.save_character_greeting(&greeting).await?;
        }
        
        for tag in tags {
            self.storage.save_character_tag(&tag).await?;
        }
        
        Ok(return_id)
    }
}