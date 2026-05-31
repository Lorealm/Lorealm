use std::sync::Arc;

use chrono::Utc;
use eldoria_storage::{
    Storage,
    models::{Lorebook, LorebookEntry, LorebookMetadata, Snowflake},
};

use crate::{
    errors::{ContextError, ContextType, LorebookError},
    services::{
        lorebook::{CreateLorebook, UpdateLorebook},
        snowflake::SnowflakeService,
    },
};

const DEFAULT_ENTRY_ID: &str = "new_entry";

pub struct LorebookService {
    storage: Arc<dyn Storage>,
    snowflake_service: Arc<SnowflakeService>,
}

impl LorebookService {
    pub fn new(storage: Arc<dyn Storage>, snowflake_service: Arc<SnowflakeService>) -> Self {
        Self {
            storage,
            snowflake_service,
        }
    }

    pub async fn get_lorebook(
        &self,
        lorebook_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Lorebook, ContextError> {
        let lorebook = self
            .storage
            .get_lorebook(lorebook_id)
            .await?
            .ok_or(ContextError::NotFound(ContextType::Lorebook, lorebook_id))?;

        if lorebook.owner_id != user_id {
            return Err(ContextError::Lorebook(LorebookError::NotOwned(
                user_id,
                lorebook_id,
            )));
        }

        Ok(lorebook)
    }

    pub async fn create_lorebook(
        &self,
        owner_id: Snowflake,
        create: CreateLorebook,
    ) -> Result<Lorebook, ContextError> {
        let mut lorebook = context_weaver::Lorebook::new();
        lorebook.config = context_weaver::LorebookConfig {
            name: create.name.clone(),
            description: create.description.unwrap_or_default(),
            ..Default::default()
        };

        let id = self.snowflake_service.generate();
        let db_lorebook = weaver_to_db(lorebook, id, owner_id, &self.snowflake_service)?;
        self.storage.save_lorebook(&db_lorebook).await?;
        Ok(db_lorebook)
    }

    pub async fn list_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, ContextError> {
        Ok(self.storage.list_lorebooks_by_owner(user_id).await?)
    }

    pub async fn update_lorebook(
        &self,
        lorebook_id: Snowflake,
        user_id: Snowflake,
        update: UpdateLorebook,
    ) -> Result<Lorebook, ContextError> {
        let mut lorebook = self.get_lorebook(lorebook_id, user_id).await?;

        if let Some(name) = update.name {
            lorebook.name = name;
        }
        if let Some(desc) = update.description {
            lorebook.description = desc;
        }

        // Rebuild config from whatever the caller sent (or the existing YAML),
        // then force the canonical name/description onto it so the two never drift.
        let mut config: context_weaver::LorebookConfig = match update.config {
            Some(c) => c,
            None => serde_yaml::from_str(&lorebook.config_yaml)
                .map_err(|e| ContextError::Lorebook(LorebookError::Yaml(e)))?,
        };
        config.name = lorebook.name.clone();
        config.description = lorebook.description.clone();
        lorebook.config_yaml = serde_yaml::to_string(&config)
            .map_err(|e| ContextError::Lorebook(LorebookError::Yaml(e)))?;

        self.storage.save_lorebook(&lorebook).await?;
        Ok(lorebook)
    }

    pub async fn delete_lorebook(
        &self,
        lorebook_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, ContextError> {
        // Make sure user owns the lorebook before deleting
        let _ = self.get_lorebook(lorebook_id, user_id).await?;

        Ok(self.storage.delete_lorebook(lorebook_id).await?)
    }

    pub async fn get_entry(
        &self,
        lorebook_id: Snowflake,
        user_id: Snowflake,
        entry_id: Snowflake,
    ) -> Result<LorebookEntry, ContextError> {
        let lorebook = self.get_lorebook(lorebook_id, user_id).await?;

        let entry =
            lorebook
                .entries
                .iter()
                .find(|e| e.id == entry_id)
                .ok_or(ContextError::NotFound(
                    ContextType::LorebookEntry,
                    lorebook_id,
                ))?;

        Ok(entry.clone())
    }

    pub async fn create_entry(
        &self,
        lorebook_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<LorebookEntry, ContextError> {
        let mut lorebook = self.get_lorebook(lorebook_id, user_id).await?;

        let existing_ids: std::collections::HashSet<&str> = lorebook
            .entries
            .iter()
            .map(|e| e.entry_id.as_str())
            .collect();

        let id = std::iter::once(DEFAULT_ENTRY_ID.to_string())
            .chain((1..).map(|n| format!("{DEFAULT_ENTRY_ID}_{n}")))
            .find(|candidate| !existing_ids.contains(candidate.as_str()))
            .unwrap(); // infinite iterator, always finds a free slot

        let entry = LorebookEntry {
            id: self.snowflake_service.generate(),
            lorebook_id,
            entry_id: id.clone(),
            source: default_entry_source(id),
        };

        context_weaver::Entry::parse(&entry.source, None)
            .map_err(|e| ContextError::Lorebook(LorebookError::ContextWeaver(e)))?;

        lorebook.entries.push(entry.clone());
        self.storage.save_lorebook(&lorebook).await?;

        Ok(entry)
    }

    pub async fn update_entry(
        &self,
        lorebook_id: Snowflake,
        entry_id: Snowflake,
        user_id: Snowflake,
        source: String,
    ) -> Result<LorebookEntry, ContextError> {
        let mut lorebook = self.get_lorebook(lorebook_id, user_id).await?;

        let idx = lorebook
            .entries
            .iter()
            .position(|e| e.id == entry_id)
            .ok_or(ContextError::NotFound(ContextType::Lorebook, entry_id))?;

        // Parse the entry to ensure the content is properly formatted.
        context_weaver::Entry::parse(&source, None)
            .map_err(|e| ContextError::Lorebook(LorebookError::ContextWeaver(e)))?;

        lorebook.entries[idx].source = source;
        self.storage.save_lorebook(&lorebook).await?;

        Ok(lorebook.entries[idx].clone())
    }

    pub async fn delete_entry(
        &self,
        lorebook_id: Snowflake,
        entry_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), ContextError> {
        let mut lorebook = self.get_lorebook(lorebook_id, user_id).await?;

        let idx = lorebook
            .entries
            .iter()
            .position(|e| e.id == entry_id)
            .ok_or(ContextError::NotFound(ContextType::Lorebook, entry_id))?;

        lorebook.entries.remove(idx);
        self.storage.save_lorebook(&lorebook).await?;

        Ok(())
    }
}

fn default_entry_source(id: String) -> String {
    format!("---\nid: {id}\nname: 'New Entry'\nkeywords: []\npriority: 100\nslot: backdrop\n---\n")
}

fn entry_to_source(entry: &context_weaver::Entry) -> Result<String, ContextError> {
    // Reconstruct the .weaver file: serialized frontmatter + preserved body.
    // Stand-in for the not-yet-implemented Entry::to_source().
    let frontmatter = serde_yaml::to_string(&entry.meta)
        .map_err(|e| ContextError::Lorebook(LorebookError::Yaml(e)))?;
    Ok(format!("---\n{frontmatter}---\n{}", entry.source_body))
}

fn weaver_to_db(
    lb: context_weaver::Lorebook,
    id: Snowflake,
    owner_id: Snowflake,
    snowflake_gen: &SnowflakeService, // was by-value — can't move into a .map closure
) -> Result<Lorebook, ContextError> {
    let entries = lb
        .entries_in_order()
        .map(|e| weaver_entry_to_storage(e, id, snowflake_gen))
        .collect::<Result<Vec<_>, _>>()?; // collect Results into one Result

    Ok(Lorebook {
        id,
        owner_id,
        current_version_id: id, // placeholder while versioning is dormant
        name: lb.config.name.clone(),
        description: lb.config.description.clone(),
        config_yaml: serde_yaml::to_string(&lb.config)
            .map_err(|e| ContextError::Lorebook(LorebookError::Yaml(e)))?,
        schema_version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        entries,
    })
}

fn weaver_entry_to_storage(
    entry: &context_weaver::Entry,
    lb_id: Snowflake,
    snowflake_gen: &SnowflakeService,
) -> Result<LorebookEntry, ContextError> {
    Ok(LorebookEntry {
        id: snowflake_gen.generate(), // fresh row id; see note
        lorebook_id: lb_id,
        entry_id: entry.meta.id.clone(), // the human string id ("dark_forest")
        source: entry_to_source(entry)?,
    })
}
