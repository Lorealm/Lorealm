mod hosting;
mod session;
mod user;
mod storage;

use std::collections::HashMap;

pub use hosting::HostingConfig;
pub use session::*;
pub use user::*;
pub use storage::*;

use serde::{Serialize, de::DeserializeOwned};

pub trait Config: Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static {
    /// A stable key used for storage/lookup (e.g. "hosting", "database")
    fn key() -> &'static str;

    /// Load from raw JSON, falling back to defaults for missing fields.
    fn from_value(value: serde_json::Value) -> Self {
        // Merge the incoming value on top of the serialized default,
        // so any new fields that aren't in the stored config get filled in.
        let mut base = serde_json::to_value(Self::default()).unwrap();
        merge_json(&mut base, value);
        serde_json::from_value(base).unwrap_or_default()
    }

    /// Patch with a partial update
    fn patch(&mut self, partial: serde_json::Value) {
        let mut current = serde_json::to_value(&*self).unwrap();
        merge_json(&mut current, partial);
        if let Ok(patched) = serde_json::from_value(current) {
            *self = patched;
        }
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

#[derive(Clone)]
pub struct ConfigProvider {
    store: HashMap<String, serde_json::Value>,
}

impl ConfigProvider {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn load() -> Self {
        let mut provider = Self::new();
        provider.register::<HostingConfig>();
        provider.register::<StorageConfig>();
        provider.register::<SessionConfig>();
        provider.register::<UserConfig>();
        provider
    }

    fn register<C: Config>(&mut self) {
        self.store
            .entry(C::key().to_owned())
            .or_insert_with(|| serde_json::to_value(C::default()).unwrap());
    }

    pub fn get<C: Config>(&self) -> C {
        self.store
            .get(C::key())
            .cloned()
            .map(C::from_value)
            .unwrap_or_default()
    }

    pub fn patch<C: Config>(&mut self, partial: serde_json::Value) {
        let mut config = self.get::<C>();
        config.patch(partial);
        self.store
            .insert(C::key().to_owned(), serde_json::to_value(&config).unwrap());
    }
}
