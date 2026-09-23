use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::log::{LogBuffer, LogEntry, LogSource};
use crate::manifest::{Manifest, ManifestError};

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    imports: { default: async },
    exports: { default: async },
    additional_derives: [PartialEq, Eq, Hash],
});

use lorealm::plugin::types::{Hook, Level, Message, Persona};

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
    /// Receives what the plugin and the host logged during one call.
    async fn log(&self, entries: Vec<LogEntry>);
}

/// Per-instance store data.
pub struct PluginState {
    wasi: WasiCtx,
    table: ResourceTable,
    permissions: HashSet<String>,
    hooks: HashSet<Hook>,
    services: Arc<dyn HostServices>,
    logs: LogBuffer,
}

impl PluginState {
    fn require(&mut self, permission: &str) -> Result<(), String> {
        if self.permissions.contains(permission) {
            Ok(())
        } else {
            let message = format!("missing permission `{permission}`");
            self.logs
                .record(Level::Warn, LogSource::Host, message.clone());
            Err(message)
        }
    }

    fn require_data_key(&mut self, key: &str) -> Result<(), String> {
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
    async fn log(&mut self, level: Level, message: String) {
        self.logs.record(level, LogSource::Guest, message);
    }

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
            return Err(format!(
                "data values must be at most {MAX_DATA_VALUE} bytes"
            ));
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

/// A validated manifest and its compiled component.
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
        let logs = LogBuffer::default();
        let state = PluginState {
            wasi: WasiCtx::builder()
                .stdout(logs.stdout.clone())
                .stderr(logs.stderr.clone())
                .build(),
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
            logs,
        };
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(self.deadline);
        let result: Result<Plugin, PluginError> = async {
            let instance =
                Plugin::instantiate_async(&mut store, &package.component, &self.linker).await?;
            instance
                .call_init(&mut store)
                .await?
                .map_err(PluginError::Init)?;
            Ok(instance)
        }
        .await;
        flush_logs(&mut store, result.as_ref().err()).await;
        let instance = result?;

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
        let result = match self.instance.call_handle(&mut self.store, &event).await {
            Ok(result) => result.map_err(PluginError::Hook),
            Err(err) => Err(err.into()),
        };
        flush_logs(&mut self.store, result.as_ref().err()).await;
        result
    }
}

/// Hands the call's log entries to the embedder, ending with `error` (the
/// call's outcome) as a host entry.
async fn flush_logs(store: &mut Store<PluginState>, error: Option<&PluginError>) {
    let state = store.data_mut();
    let entries = state.logs.drain(error.map(ToString::to_string));
    if !entries.is_empty() {
        state.services.log(entries).await;
    }
}
