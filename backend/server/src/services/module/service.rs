use std::sync::Arc;

use chrono::Utc;
use lorealm_storage::{
    Storage,
    models::{CodeModule, Snowflake},
};

use super::LoadedModule;
use crate::{
    errors::ModuleError,
    services::{
        context_template::weaver::check::{Dialect, Severity, check},
        snowflake::SnowflakeService,
        weaver::WeaverHost,
    },
};

pub struct ModuleService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
}

impl ModuleService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    pub async fn list(&self, user_id: Snowflake) -> Result<Vec<CodeModule>, ModuleError> {
        Ok(self.storage.list_modules(user_id).await?)
    }

    pub async fn get(&self, id: Snowflake, user_id: Snowflake) -> Result<CodeModule, ModuleError> {
        match self.storage.get_module(id).await? {
            Some(module) if module.owner_id == user_id => Ok(module),
            _ => Err(ModuleError::NotFound(id)),
        }
    }

    pub async fn create(
        &self,
        user_id: Snowflake,
        name: String,
        description: String,
        source: String,
    ) -> Result<CodeModule, ModuleError> {
        self.check_name(user_id, None, &name).await?;
        Self::check_source(&name, &source)?;

        let now = Utc::now();
        let module = CodeModule {
            id: self.snowflake.generate(),
            owner_id: user_id,
            name,
            description,
            source,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        self.storage.save_module(&module).await?;
        Ok(module)
    }

    pub async fn update(
        &self,
        id: Snowflake,
        user_id: Snowflake,
        name: Option<String>,
        description: Option<String>,
        source: Option<String>,
        enabled: Option<bool>,
    ) -> Result<CodeModule, ModuleError> {
        let mut module = self.get(id, user_id).await?;

        if let Some(name) = name {
            self.check_name(user_id, Some(id), &name).await?;
            module.name = name;
        }
        if let Some(source) = source {
            module.source = source;
        }
        Self::check_source(&module.name, &module.source)?;
        if let Some(description) = description {
            module.description = description;
        }
        if let Some(enabled) = enabled {
            module.enabled = enabled;
        }
        module.updated_at = Utc::now();

        self.storage.save_module(&module).await?;
        Ok(module)
    }

    pub async fn delete(&self, id: Snowflake, user_id: Snowflake) -> Result<(), ModuleError> {
        self.get(id, user_id).await?;
        self.storage.delete_module(id).await?;
        Ok(())
    }

    /// A name is how callers reach the module, so it must be an identifier
    /// and must not collide with the stdlib, a plugin, or another module.
    async fn check_name(
        &self,
        user_id: Snowflake,
        id: Option<Snowflake>,
        name: &str,
    ) -> Result<(), ModuleError> {
        let mut chars = name.chars();
        let identifier = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !identifier {
            return Err(ModuleError::Invalid(
                "a name may only use letters, digits and underscores, and may not start with a digit"
                    .to_string(),
            ));
        }

        let builtin = super::RESERVED_SCOPES.contains(&name)
            || WeaverHost::builder()
                .build()
                .registry()
                .processor_signatures()
                .into_iter()
                .any(|s| s.namespace == name);
        let plugin = self
            .storage
            .list_user_plugins(user_id)
            .await?
            .iter()
            .any(|p| p.handle == name);
        let module = self
            .storage
            .list_modules(user_id)
            .await?
            .iter()
            .any(|m| m.name == name && Some(m.id) != id);

        if builtin || plugin || module {
            return Err(ModuleError::NameTaken(name.to_string()));
        }
        Ok(())
    }

    /// The source must parse, and its values must evaluate.
    fn check_source(name: &str, source: &str) -> Result<(), ModuleError> {
        if let Some(d) = check(source, Dialect::Module, None)
            .into_iter()
            .find(|d| d.severity == Severity::Error)
        {
            return Err(ModuleError::Invalid(d.message));
        }
        LoadedModule::load(name, source)
            .map(|_| ())
            .map_err(ModuleError::Invalid)
    }
}
