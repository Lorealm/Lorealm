use std::collections::HashSet;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::log::{LogBuffer, LogEntry, LogSource};
use crate::manifest::{Manifest, ManifestError};

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    imports: { default: async },
    exports: { default: async },
    with: { "lorealm:plugin/types.eval-context": EvalContextHandle },
});

/// What the plugin's `eval-context` resource stands for. The context itself
/// is [`PluginState::context`]; the resource only lets the guest name it.
pub struct EvalContextHandle;

use lorealm::plugin::types::{
    Block, CommandCall, CommandSignature, FrontendCall, Hook, Level, Message, Persona,
    ProcessorCall, ProcessorSignature, SourceCall, SourceNode,
};

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
    /// The plugin answered a call with an error (or the wrong response).
    #[error("plugin call failed: {0}")]
    Call(String),
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

/// The variables of the evaluation a command runs in, as JSON.
pub trait CommandContext {
    fn get(&self, scope: &str, name: &str) -> Result<Option<String>, String>;
    fn set(&mut self, scope: &str, name: &str, value: String) -> Result<(), String>;
}

/// The context of the command call in progress. The lifetime is erased so it
/// fits in the `'static` store; see [`LoadedPlugin::call_command`] for why
/// that holds.
struct ContextPtr(NonNull<dyn CommandContext>);

// SAFETY: only dereferenced on the thread that set it, during the call.
unsafe impl Send for ContextPtr {}

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

/// What a plugin registered during `init`, for the embedder to expose.
#[derive(Debug, Clone, Default)]
pub struct Registrations {
    pub commands: Vec<CommandSignature>,
    pub processors: Vec<ProcessorSignature>,
    pub source_nodes: Vec<SourceNode>,
}

/// Per-instance store data.
pub struct PluginState {
    wasi: WasiCtx,
    table: ResourceTable,
    permissions: HashSet<String>,
    hooks: Vec<Hook>,
    registrations: Registrations,
    /// Registering is only allowed while `init` runs.
    initializing: bool,
    services: Arc<dyn HostServices>,
    logs: LogBuffer,
    /// Set only while a command runs, with whether it declared
    /// `mutates-context`.
    context: Option<(ContextPtr, bool)>,
}

impl PluginState {
    fn context(&mut self) -> Result<(&mut dyn CommandContext, bool), String> {
        match &mut self.context {
            // SAFETY: set by `call_command` for the length of the call, and
            // the guest can only reach this through the borrowed resource that
            // call hands it.
            Some((ptr, mutates)) => Ok((unsafe { ptr.0.as_mut() }, *mutates)),
            None => Err("the context is only available while a command runs".into()),
        }
    }

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

    /// Checks that `name` can be registered as a `kind`, given the names
    /// already taken.
    fn check_registration<'a>(
        &self,
        kind: &str,
        name: &str,
        mut taken: impl Iterator<Item = &'a String>,
    ) -> Result<(), String> {
        if !self.initializing {
            return Err(format!("a {kind} can only be registered during `init`"));
        }
        if !is_identifier(name) {
            return Err(format!(
                "{kind} name `{name}` must be an identifier ([A-Za-z_][A-Za-z0-9_]*)"
            ));
        }
        if taken.any(|t| t == name) {
            return Err(format!("{kind} `{name}` is already registered"));
        }
        Ok(())
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

impl lorealm::plugin::types::HostEvalContext for PluginState {
    async fn get(
        &mut self,
        _: Resource<EvalContextHandle>,
        scope: String,
        name: String,
    ) -> Result<Option<String>, String> {
        self.require("state:read")?;
        self.context()?.0.get(&scope, &name)
    }

    async fn set(
        &mut self,
        _: Resource<EvalContextHandle>,
        scope: String,
        name: String,
        value: String,
    ) -> Result<(), String> {
        self.require("state:write")?;
        let (context, mutates) = self.context()?;
        if !mutates {
            return Err("this command didn't declare `mutates-context`".into());
        }
        context.set(&scope, &name, value)
    }

    async fn drop(&mut self, handle: Resource<EvalContextHandle>) -> wasmtime::Result<()> {
        self.table.delete(handle)?;
        Ok(())
    }
}

impl lorealm::plugin::host::Host for PluginState {
    async fn log(&mut self, level: Level, message: String) {
        self.logs.record(level, LogSource::Guest, message);
    }

    async fn register_hook(&mut self, hook: Hook) -> Result<(), String> {
        let permission = match hook {
            Hook::PromptBuild => "prompt:write",
        };
        self.require(permission)?;
        if !self.hooks.contains(&hook) {
            self.hooks.push(hook);
        }
        Ok(())
    }

    async fn register_command(&mut self, command: CommandSignature) -> Result<(), String> {
        let taken = self.registrations.commands.iter().map(|c| &c.name);
        self.check_registration("command", &command.name, taken)?;
        self.registrations.commands.push(command);
        Ok(())
    }

    async fn register_processor(&mut self, processor: ProcessorSignature) -> Result<(), String> {
        let taken = self.registrations.processors.iter().map(|p| &p.name);
        self.check_registration("processor", &processor.name, taken)?;
        self.registrations.processors.push(processor);
        Ok(())
    }

    async fn register_source_node(&mut self, node: SourceNode) -> Result<(), String> {
        let taken = self.registrations.source_nodes.iter().map(|n| &n.id);
        self.check_registration("source node", &node.id, taken)?;
        self.registrations.source_nodes.push(node);
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
            hooks: Vec::new(),
            registrations: Registrations::default(),
            initializing: true,
            services,
            logs,
            context: None,
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
        store.data_mut().initializing = false;

        Ok(LoadedPlugin {
            store,
            instance,
            deadline: self.deadline,
        })
    }
}

impl LoadedPlugin {
    pub fn registrations(&self) -> &Registrations {
        &self.store.data().registrations
    }

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
            Response::PromptBuild(prompt) => Ok(prompt),
            _ => Err(mismatch("prompt-build")),
        }
    }

    /// Calls a registered command with JSON arguments. The plugin reaches
    /// `context` through the `eval-context` it is lent for the call.
    pub async fn call_command(
        &mut self,
        name: String,
        args: Vec<String>,
        context: &mut dyn CommandContext,
    ) -> Result<Option<String>, PluginError> {
        let state = self.store.data_mut();
        let mutates = state
            .registrations
            .commands
            .iter()
            .any(|c| c.name == name && c.mutates_context);
        let handle = state
            .table
            .push(EvalContextHandle)
            .map_err(wasmtime::Error::from)?;
        // SAFETY: erases the borrow's lifetime. The pointer is cleared below,
        // with nothing in between that can return early, so it never outlives
        // `context`. The guest can only reach it through `handle`, which the
        // component model invalidates when the call returns.
        let ptr: NonNull<dyn CommandContext + 'static> =
            unsafe { std::mem::transmute(NonNull::from(context)) };
        state.context = Some((ContextPtr(ptr), mutates));
        let borrowed = Resource::new_borrow(handle.rep());

        let result = self
            .handle(Event::Command(CommandCall {
                name,
                args,
                context: borrowed,
            }))
            .await;

        let state = self.store.data_mut();
        state.context = None;
        state.table.delete(handle).map_err(wasmtime::Error::from)?;
        match result? {
            Response::Command(value) => Ok(value),
            _ => Err(mismatch("command")),
        }
    }

    /// Calls a registered processor with JSON properties.
    pub async fn call_processor(
        &mut self,
        name: String,
        properties: Vec<(String, String)>,
    ) -> Result<String, PluginError> {
        match self
            .handle(Event::Processor(ProcessorCall { name, properties }))
            .await?
        {
            Response::Processor(value) => Ok(value),
            _ => Err(mismatch("processor")),
        }
    }

    /// Resolves a registered source node, given its settings as a JSON object.
    pub async fn resolve_source(
        &mut self,
        node: String,
        settings: String,
    ) -> Result<Vec<Block>, PluginError> {
        match self
            .handle(Event::Source(SourceCall { node, settings }))
            .await?
        {
            Response::Source(blocks) => Ok(blocks),
            _ => Err(mismatch("source")),
        }
    }

    /// Calls the plugin on behalf of its frontend, with JSON arguments.
    pub async fn call_frontend(
        &mut self,
        name: String,
        args: String,
    ) -> Result<String, PluginError> {
        match self
            .handle(Event::Frontend(FrontendCall { name, args }))
            .await?
        {
            Response::Frontend(value) => Ok(value),
            _ => Err(mismatch("frontend")),
        }
    }

    async fn handle(&mut self, event: Event) -> Result<Response, PluginError> {
        self.store.set_epoch_deadline(self.deadline);
        let result = match self.instance.call_handle(&mut self.store, &event).await {
            Ok(result) => result.map_err(PluginError::Call),
            Err(err) => Err(err.into()),
        };
        flush_logs(&mut self.store, result.as_ref().err()).await;
        result
    }
}

fn mismatch(event: &str) -> PluginError {
    PluginError::Call(format!(
        "answered a `{event}` event with a different response"
    ))
}

/// `[A-Za-z_][A-Za-z0-9_]*`, the shape Weaver accepts as a name.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
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
