use std::sync::Arc;

use chrono::Utc;
use eldoria_storage::{Storage, models::{Snowflake, UserPersona}};

use crate::{errors::PersonaError, services::snowflake::SnowflakeService};

pub struct PersonaService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>
}

impl PersonaService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    pub async fn create_default(&self, user_id: Snowflake, name: String) -> Result<Snowflake, PersonaError> {
        let id = self.snowflake.generate();
        let persona = UserPersona {
            id,
            user_id,
            display_name: name,
            pseudonym: None,
            avatar_uri: None,
            description: None,
            updated_at: Utc::now(),
        };

        self.storage.save_persona(persona).await?;

        Ok(id)
    }
}
