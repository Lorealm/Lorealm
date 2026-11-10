use std::{collections::BTreeMap, sync::Arc};

use lorealm_storage::{
    Storage,
    models::{
        Realm, RealmOrigin, RealmResourceRef, RealmSummary, RealmVersionInfo, Snowflake,
        VersionedSaveMode,
    },
};
use weaver_lang::Value as WeaverValue;

use crate::{
    errors::{RealmError, TargetType},
    services::{
        character::CharacterService, connection::ConnectionService,
        context_template::TemplateService, persona::PersonaService, plugins::PluginService,
        snowflake::SnowflakeService,
    },
};

use super::{project::resolve_scenario_slugs, validate::realm_resources, wtn};

/// Realms as definitions: built from a blueprint or imported, then read,
/// exported and deleted. A realm is never edited in place, and playing one
/// is the chat service's business.
pub struct RealmService {
    pub(super) storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    pub(super) characters: Arc<CharacterService>,
    pub(super) templates: Arc<TemplateService>,
    pub(super) connection: Arc<ConnectionService>,
    pub(super) personas: Arc<PersonaService>,
    plugins: Arc<PluginService>,
}

impl RealmService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        characters: Arc<CharacterService>,
        templates: Arc<TemplateService>,
        connection: Arc<ConnectionService>,
        personas: Arc<PersonaService>,
        plugins: Arc<PluginService>,
    ) -> Self {
        Self {
            storage,
            snowflake,
            characters,
            templates,
            connection,
            personas,
            plugins,
        }
    }

    pub async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, RealmError> {
        let realms = self.storage.list_realms(user_id).await?;

        Ok(realms)
    }

    pub async fn get_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;

        Ok(realm)
    }

    pub async fn delete_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        self.storage.delete_realm(realm_id).await?;

        Ok(())
    }

    pub async fn list_versions(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<RealmVersionInfo>, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        Ok(self.storage.list_realm_versions(realm_id).await?)
    }

    pub async fn get_version(
        &self,
        realm_id: Snowflake,
        version_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        self.storage
            .get_realm_at_version(realm_id, version_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::RealmVersion, version_id))
    }

    /// Create a realm by expanding a WTN document.
    pub async fn import(
        &self,
        user_id: Snowflake,
        source: &str,
        inputs: BTreeMap<String, WeaverValue>,
        origin: Option<RealmOrigin>,
    ) -> Result<Realm, RealmError> {
        let doc = wtn::compile(source)?;
        let inputs = self
            .resolve_inputs(user_id, doc.declarations(), inputs)
            .await?;

        let plugins = self.plugins.vocabulary(user_id).await;
        let mut definition = wtn::expand(doc, inputs, plugins.as_ref())?;
        resolve_scenario_slugs(&mut definition.scenarios);
        self.bind_cast(user_id, &mut definition).await?;
        self.validate_definition(user_id, &definition).await?;

        let realm = Realm {
            id: self.snowflake.generate(),
            owner_id: user_id,
            current_version_id: self.snowflake.generate(),
            tags: definition.tags.clone(),
            definition,
            origin,
        };
        self.storage
            .save_realm(&realm, VersionedSaveMode::NewVersion)
            .await?;

        Ok(realm)
    }

    /// The realm as a WTN document.
    pub async fn export(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<String, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        wtn::to_wtn(&realm.definition)
    }

    /// Everything the realm's cast references, derived from the cast itself.
    pub async fn list_resources(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<RealmResourceRef>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        Ok(realm_resources(&realm.definition))
    }

    async fn owned_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        let realm = self
            .storage
            .get_realm(realm_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::Realm, realm_id))?;

        if realm.owner_id != user_id {
            return Err(RealmError::NotOwned(TargetType::Realm, realm_id));
        }

        Ok(realm)
    }
}
