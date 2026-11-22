mod characters;
mod hosting;
mod plugins;
mod rate_limit;
mod session;
mod storage;
mod task;
mod tokenizers;
mod user;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::RwLock,
};

pub use characters::*;
pub use hosting::{HostingConfig, OpenFileLimit};
use indexmap::IndexMap;
pub use plugins::PluginConfig;
pub use rate_limit::*;
pub use session::*;
pub use task::*;
pub use tokenizers::TokenizerConfig;
pub use user::*;

use serde::{Serialize, de::DeserializeOwned};

use crate::errors::ConfigError;

pub trait Config: Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static {
    /// A stable key used for storage/lookup (e.g. "hosting", "database")
    fn key() -> &'static str;

    /// Load from raw JSON, falling back to defaults for missing fields.
    fn from_value(value: serde_json::Value) -> Self {
        // Merge the incoming value on top of the serialized default,
        let mut base = serde_json::to_value(Self::default()).unwrap();
        merge_json(&mut base, value);
        serde_json::from_value(base).unwrap_or_default()
    }

    /// Patch with a partial update
    fn patch(&mut self, partial: serde_json::Value) -> Result<(), serde_json::Error> {
        let mut current = serde_json::to_value(&*self)?;
        merge_json(&mut current, partial);
        *self = serde_json::from_value(current)?;
        Ok(())
    }
}

fn merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base), serde_json::Value::Object(overlay)) => {
            for (k, v) in overlay {
                merge_json(base.entry(k).or_insert(serde_json::Value::Null), v);
            }
        }
        (base, overlay) => *base = overlay,
    }
}

type Patcher = Box<
    dyn Fn(&serde_json::Value, serde_json::Value) -> Result<serde_json::Value, serde_json::Error>
        + Send
        + Sync,
>;

pub struct ConfigProvider {
    path: PathBuf,
    store: RwLock<IndexMap<String, serde_json::Value>>, // BTree to presserve serialization order
    patchers: HashMap<String, Patcher>,
}

impl ConfigProvider {
    pub fn load(path: PathBuf) -> std::io::Result<Self> {
        let mut store = IndexMap::new();
        let mut existed = false;

        match std::fs::read(&path) {
            Ok(bytes) => {
                existed = true;
                match serde_yaml::from_slice(&bytes) {
                    Ok(stored) => store = stored,
                    Err(e) => eprintln!("config: ignoring unreadable {path:?}: {e}"),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }

        let mut provider = Self {
            path,
            store: RwLock::new(store),
            patchers: HashMap::new(),
        };
        provider.register::<HostingConfig>();
        provider.register::<lorealm_storage::StorageConfig>();
        provider.register::<SessionConfig>();
        provider.register::<UserConfig>();
        provider.register::<CharacterConfig>();
        provider.register::<TokenizerConfig>();
        provider.register::<PluginConfig>();
        provider.register::<TaskConfig>();
        provider.register::<RateLimitConfig>();

        if !existed {
            let store = provider.store.read().unwrap();
            persist(&provider.path, &store)?;
        }

        Ok(provider)
    }

    fn register<C: Config>(&mut self) {
        self.store
            .get_mut()
            .unwrap()
            .entry(C::key().to_owned())
            .or_insert_with(|| serde_json::to_value(C::default()).unwrap());

        self.patchers.insert(
            C::key().to_owned(),
            Box::new(|current, partial| {
                let mut config = C::from_value(current.clone());
                config.patch(partial)?;
                serde_json::to_value(&config)
            }),
        );
    }

    pub fn get<C: Config>(&self) -> C {
        self.store
            .read()
            .unwrap()
            .get(C::key())
            .cloned()
            .map(C::from_value)
            .unwrap_or_default()
    }

    pub fn get_raw(&self, key: &str) -> Option<serde_json::Value> {
        self.store.read().unwrap().get(key).cloned()
    }

    pub fn get_all(&self) -> serde_json::Value {
        serde_json::Value::Object(
            self.store
                .read()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    }

    pub fn patch_by_key(
        &self,
        key: &str,
        partial: serde_json::Value,
    ) -> Result<serde_json::Value, ConfigError> {
        let patcher = self.patchers.get(key).ok_or(ConfigError::UnknownKey)?;

        let mut store = self.store.write().unwrap();
        let base = store.get(key).cloned().unwrap_or(serde_json::Value::Null);
        let new_value = patcher(&base, partial).map_err(ConfigError::Invalid)?;

        let previous = store.insert(key.to_owned(), new_value.clone());

        if let Err(e) = persist(&self.path, &store) {
            match previous {
                // roll memory back to match disk
                Some(prev) => store.insert(key.to_owned(), prev),
                None => store.swap_remove(key),
            };
            return Err(ConfigError::Io(e));
        }
        Ok(new_value)
    }
}

fn persist(path: &Path, store: &IndexMap<String, serde_json::Value>) -> std::io::Result<()> {
    let yaml = serde_yaml::to_string(store).expect("config values are serializable");
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, yaml.as_bytes())?;
    std::fs::rename(&tmp, path)
}
