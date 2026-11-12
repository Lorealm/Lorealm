use std::{collections::BTreeMap, sync::Arc};

use chrono::Utc;
use lorealm_storage::{
    Storage,
    models::{Blueprint, BlueprintSummary, Realm, RealmOrigin, Snowflake},
};
use weaver_lang::Value as WeaverValue;

use crate::{
    errors::{RealmError, TargetType},
    services::{
        blueprint::{BuildForm, InputSpec, OutlineResult, outline},
        character::CharacterService,
        context_template::{
            TemplateService,
            weaver::check::{Dialect, Severity, check},
        },
        module::ModuleService,
        persona::PersonaService,
        realm::{RealmService, wtn},
        snowflake::SnowflakeService,
        weaver::WeaverProfile,
    },
    util::weaver::json_to_weaver,
};

/// Blueprints: the constructor half of a realm.
pub struct BlueprintService {
    pub(super) storage: Arc<dyn Storage>,
    pub(super) snowflake: Arc<SnowflakeService>,
    realms: Arc<RealmService>,
    pub(super) characters: Arc<CharacterService>,
    pub(super) templates: Arc<TemplateService>,
    pub(super) personas: Arc<PersonaService>,
    pub(super) modules: Arc<ModuleService>,
}

impl BlueprintService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        realms: Arc<RealmService>,
        characters: Arc<CharacterService>,
        templates: Arc<TemplateService>,
        personas: Arc<PersonaService>,
        modules: Arc<ModuleService>,
    ) -> Self {
        Self {
            storage,
            snowflake,
            realms,
            characters,
            templates,
            personas,
            modules,
        }
    }

    pub async fn list(&self, user_id: Snowflake) -> Result<Vec<BlueprintSummary>, RealmError> {
        Ok(self.storage.list_blueprints(user_id).await?)
    }

    pub async fn get(&self, id: Snowflake, user_id: Snowflake) -> Result<Blueprint, RealmError> {
        let blueprint = self
            .storage
            .get_blueprint(id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::Blueprint, id))?;
        if blueprint.owner_id != user_id {
            return Err(RealmError::NotOwned(TargetType::Blueprint, id));
        }
        Ok(blueprint)
    }

    pub async fn create(
        &self,
        user_id: Snowflake,
        name: String,
        source: String,
    ) -> Result<Blueprint, RealmError> {
        Self::check(&source)?;

        let now = Utc::now();
        let blueprint = Blueprint {
            id: self.snowflake.generate(),
            owner_id: user_id,
            name,
            description: String::new(),
            source,
            created_at: now,
            updated_at: now,
        };
        self.storage.save_blueprint(&blueprint).await?;
        Ok(blueprint)
    }

    pub async fn update(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        name: Option<String>,
        description: Option<String>,
        source: Option<String>,
    ) -> Result<Blueprint, RealmError> {
        let mut blueprint = self.get(id, user_id).await?;

        if let Some(source) = source {
            Self::check(&source)?;
            blueprint.source = source;
        }
        if let Some(name) = name {
            blueprint.name = name;
        }
        if let Some(description) = description {
            blueprint.description = description;
        }
        blueprint.updated_at = Utc::now();

        self.storage.save_blueprint(&blueprint).await?;
        Ok(blueprint)
    }

    pub async fn delete(&self, id: Snowflake, user_id: Snowflake) -> Result<(), RealmError> {
        self.get(id, user_id).await?;
        self.storage.delete_blueprint(id).await?;
        Ok(())
    }

    /// What the blueprint asks for. Useful when rendering frontend components.
    pub async fn inputs(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<InputSpec>, RealmError> {
        let blueprint = self.get(id, user_id).await?;
        let doc = wtn::compile(&blueprint.source)?;
        Ok(doc.declarations().iter().map(InputSpec::of).collect())
    }

    pub async fn form(&self, id: Snowflake, user_id: Snowflake) -> Result<BuildForm, RealmError> {
        let blueprint = self.get(id, user_id).await?;
        let doc = wtn::compile(&blueprint.source)?;
        Ok(BuildForm {
            introduction: outline::introduction(doc.document()),
            inputs: doc.declarations().iter().map(InputSpec::of).collect(),
        })
    }

    /// The blueprint's structure, as the editor's block model.
    pub async fn outline(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<OutlineResult, RealmError> {
        let blueprint = self.get(id, user_id).await?;
        let doc = wtn::compile(&blueprint.source)?;
        Ok(outline::outline(doc.document(), &blueprint.source).into())
    }

    /// Expand/convert the blueprint into a realm
    pub async fn expand(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        inputs: serde_json::Map<String, serde_json::Value>,
    ) -> Result<Realm, RealmError> {
        let blueprint = self.get(id, user_id).await?;

        let bound: BTreeMap<String, WeaverValue> = inputs
            .iter()
            .map(|(name, value)| {
                json_to_weaver(value)
                    .map(|v| (name.clone(), v))
                    .map_err(|e| RealmError::InvalidTemplate(format!("input '{name}': {e}")))
            })
            .collect::<Result<_, _>>()?;

        let origin = RealmOrigin {
            blueprint_id: blueprint.id,
            inputs: inputs.into_iter().collect(),
        };
        self.realms
            .import(user_id, &blueprint.source, bound, Some(origin))
            .await
    }

    /// A blueprint's source must parse, and must declare inputs this server
    /// can actually satisfy.
    pub(super) fn check(source: &str) -> Result<(), RealmError> {
        let refused = check(
            source,
            Dialect::Document,
            Some(WeaverProfile::RealmTemplate),
        )
        .into_iter()
        .find(|d| d.severity == Severity::Error);

        match refused {
            Some(d) => Err(RealmError::InvalidTemplate(d.message)),
            None => Ok(()),
        }
    }
}
