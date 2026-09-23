use std::path::Path;

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::manifest::{Manifest, ManifestError};

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    exports: { default: async },
});

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("wasm error: {0:#}")]
    Wasm(#[from] wasmtime::Error),
    #[error("plugin init failed: {0}")]
    Init(String),
}

/// Per-instance store data.
pub struct PluginState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

pub struct PluginHost {
    engine: Engine,
    linker: Linker<PluginState>,
}

pub struct LoadedPlugin {
    pub manifest: Manifest,
    #[allow(dead_code)]
    store: Store<PluginState>,
    #[allow(dead_code)]
    instance: Plugin,
}

impl PluginHost {
    pub fn new() -> Result<Self, PluginError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        Ok(Self { engine, linker })
    }

    /// Reads `manifest.toml` from `dir`, mounts the backend component and calls its `init`.
    pub async fn load(&self, dir: &Path) -> Result<LoadedPlugin, PluginError> {
        let manifest = Manifest::from_dir(dir)?;
        let component = Component::from_file(&self.engine, dir.join(&manifest.structure.backend))?;

        let state = PluginState {
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
        };
        let mut store = Store::new(&self.engine, state);
        let instance = Plugin::instantiate_async(&mut store, &component, &self.linker).await?;
        instance.call_init(&mut store).await?.map_err(PluginError::Init)?;

        Ok(LoadedPlugin {
            manifest,
            store,
            instance,
        })
    }
}
