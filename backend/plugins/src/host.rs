use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::manifest::{Manifest, ManifestError};

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    imports: { default: async },
    exports: { default: async },
    additional_derives: [PartialEq, Eq, Hash],
});

use lorealm::plugin::types::{Hook, Message, Persona};

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("wasm error: {0:#}")]
    Wasm(wasmtime::Error),
    #[error("plugin call timed out")]
    Timeout,
    #[error("plugin init failed: {0}")]
    Init(String),
    #[error("plugin hook failed: {0}")]
    Hook(String),
}

impl From<wasmtime::Error> for PluginError {
    fn from(err: wasmtime::Error) -> Self {
        if err.downcast_ref::<Trap>() == Some(&Trap::Interrupt) {
            Self::Timeout
        } else {
            Self::Wasm(err)
        }
    }
}

/// How often the engine's epoch advances; the granularity of call timeouts.
const EPOCH_TICK: Duration = Duration::from_millis(10);

const MAX_DATA_KEY: usize = 256;
const MAX_DATA_VALUE: usize = 1024 * 1024;

/// What the embedder provides to plugins.
#[async_trait::async_trait]
pub trait HostServices: Send + Sync {
    async fn active_persona(&self) -> Result<Persona, String>;
    async fn persist_data(&self, key: String, value: Vec<u8>) -> Result<(), String>;
    async fn load_data(&self, key: String) -> Result<Option<Vec<u8>>, String>;
    async fn delete_data(&self, key: String) -> Result<(), String>;
}

/// Per-instance store data.
pub struct PluginState {
    wasi: WasiCtx,
    table: ResourceTable,
    permissions: HashSet<String>,
    hooks: HashSet<Hook>,
    services: Arc<dyn HostServices>,
}

impl PluginState {
    fn require(&self, permission: &str) -> Result<(), String> {
        if self.permissions.contains(permission) {
            Ok(())
        } else {
            Err(format!("missing permission `{permission}`"))
        }
    }

    fn require_data_key(&self, key: &str) -> Result<(), String> {
        self.require("data:persist")?;
        if key.is_empty() || key.len() > MAX_DATA_KEY {
            return Err(format!("data keys must be 1 to {MAX_DATA_KEY} bytes"));
        }
        Ok(())
    }
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl lorealm::plugin::types::Host for PluginState {}

impl lorealm::plugin::host::Host for PluginState {
    async fn register_hook(&mut self, hook: Hook) -> Result<(), String> {
        let permission = match hook {
            Hook::PromptBuild => "prompt:write",
        };
        self.require(permission)?;
        self.hooks.insert(hook);
        Ok(())
    }

    async fn active_persona(&mut self) -> Result<Persona, String> {
        self.require("persona:read")?;
        self.services.active_persona().await
    }

    async fn persist_data(&mut self, key: String, value: Vec<u8>) -> Result<(), String> {
        self.require_data_key(&key)?;
        if value.len() > MAX_DATA_VALUE {
            return Err(format!("data values must be at most {MAX_DATA_VALUE} bytes"));
        }
        self.services.persist_data(key, value).await
    }

    async fn load_data(&mut self, key: String) -> Result<Option<Vec<u8>>, String> {
        self.require_data_key(&key)?;
        self.services.load_data(key).await
    }

    async fn delete_data(&mut self, key: String) -> Result<(), String> {
        self.require_data_key(&key)?;
        self.services.delete_data(key).await
    }
}

pub struct PluginHost {
    engine: Engine,
    linker: Linker<PluginState>,
    /// Epoch ticks a single call into a plugin may take.
    deadline: u64,
}

/// A validated manifest and its compiled component. Independent of who runs
/// it, so it can be compiled once and instantiated many times.
pub struct PluginPackage {
    pub manifest: Manifest,
    component: Component,
}

pub struct LoadedPlugin {
    store: Store<PluginState>,
    instance: Plugin,
    deadline: u64,
}

impl PluginHost {
    pub fn new(call_timeout: Duration) -> Result<Self, PluginError> {
        let engine = Engine::new(Config::new().epoch_interruption(true))?;

        let ticker = engine.weak();
        std::thread::spawn(move || {
            while let Some(engine) = ticker.upgrade() {
                engine.increment_epoch();
                drop(engine);
                std::thread::sleep(EPOCH_TICK);
            }
        });

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        Plugin::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;
        Ok(Self {
            engine,
            linker,
            deadline: (call_timeout.as_millis() / EPOCH_TICK.as_millis()).max(1) as u64,
        })
    }

    /// Reads `manifest.toml` from `dir` and compiles the backend component.
    pub fn compile(&self, dir: &Path) -> Result<PluginPackage, PluginError> {
        let manifest = Manifest::from_dir(dir)?;
        let component = Component::from_file(&self.engine, dir.join(&manifest.structure.backend))?;
        Ok(PluginPackage {
            manifest,
            component,
        })
    }

    /// Mounts `package` and calls its `init`. The plugin is granted the
    /// permissions its manifest declares.
    pub async fn instantiate(
        &self,
        package: &PluginPackage,
        services: Arc<dyn HostServices>,
    ) -> Result<LoadedPlugin, PluginError> {
        let state = PluginState {
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
            permissions: package
                .manifest
                .plugin
                .permissions
                .iter()
                .cloned()
                .collect(),
            hooks: HashSet::new(),
            services,
        };
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(self.deadline);
        let instance =
            Plugin::instantiate_async(&mut store, &package.component, &self.linker).await?;
        instance
            .call_init(&mut store)
            .await?
            .map_err(PluginError::Init)?;

        Ok(LoadedPlugin {
            store,
            instance,
            deadline: self.deadline,
        })
    }
}

impl LoadedPlugin {
    /// Runs the `prompt-build` hook. Returns `prompt` unchanged if the plugin
    /// did not register it.
    pub async fn prompt_build(
        &mut self,
        prompt: Vec<Message>,
    ) -> Result<Vec<Message>, PluginError> {
        if !self.store.data().hooks.contains(&Hook::PromptBuild) {
            return Ok(prompt);
        }
        match self.handle(Event::PromptBuild(prompt)).await? {
            Event::PromptBuild(prompt) => Ok(prompt),
        }
    }

    async fn handle(&mut self, event: Event) -> Result<Event, PluginError> {
        self.store.set_epoch_deadline(self.deadline);
        self.instance
            .call_handle(&mut self.store, &event)
            .await?
            .map_err(PluginError::Hook)
    }
}
