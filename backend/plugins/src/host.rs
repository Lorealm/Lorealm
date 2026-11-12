use std::collections::HashSet;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::{Duration, Instant};

use semver::Version;

use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap, UpdateDeadline};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::log::{LogBuffer, LogEntry, LogSource};
use lorealm_plugin_manifest::{Manifest, ManifestError};

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    imports: { default: async },
    exports: { default: async },
    with: { "lorealm:plugin/types.eval-context": EvalContextHandle },
});

/// What the plugin's `eval-context` resource stands for.
pub struct EvalContextHandle;

use lorealm::plugin::types::{
    Block, ChatMessage, CommandCall, CommandSignature, CustomEvent, Finalizing, FrontendCall,
    Generated, Hook, HttpRequest, HttpResponse, Level, LorebookActivation, LorebookEntry, Message,
    MessageRange, Persona, ProcessorCall, ProcessorSignature, PromptAssembly, SourceCall,
    SourceNode, ToolInvocation, ToolOutput, TurnEnded, TurnStarted, UserInput, Verdict,
};

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(
        "built for plugin API {plugin}, and this server runs {API_VERSION}; it needs a release built for {API_VERSION}"
    )]
    Api { plugin: Version },
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

/// The plugin API this server runs: the version of the `lorealm:plugin` WIT
/// package. Changes whenever `wit/plugin.wit` does.
pub const API_VERSION: Version = Version::new(0, 1, 0);

/// Refuses a plugin built for another plugin API. The match is exact, since
/// any change to the WIT can stop a built component from loading.
pub fn check_api(manifest: &Manifest) -> Result<(), PluginError> {
    if manifest.plugin.api == API_VERSION {
        Ok(())
    } else {
        Err(PluginError::Api {
            plugin: manifest.plugin.api.clone(),
        })
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
    /// The user's settings for the plugin, as a JSON object.
    async fn settings(&self) -> Result<String, String>;
    /// Sends a request to a host the manifest allows.
    async fn http_fetch(&self, request: HttpRequest) -> Result<HttpResponse, String>;
    /// The active branch of one of the user's conversations, oldest first.
    async fn messages(
        &self,
        conversation: i64,
        range: MessageRange,
    ) -> Result<Vec<ChatMessage>, String>;
    /// Asks the model through one of the user's connection profiles, or the
    /// active one.
    async fn generate(
        &self,
        messages: Vec<Message>,
        profile: Option<i64>,
        conversation: Option<i64>,
        blocking: bool,
    ) -> Result<String, String>;
    /// Sends `payload`, as JSON, to the plugin's frontend.
    async fn send_frontend(&self, name: String, payload: String) -> Result<(), String>;
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
    handle: String,
    /// The manifest's `[provides] events`.
    provides: Vec<String>,
    /// The topics passed to `subscribe`.
    subscriptions: Vec<String>,
    /// What the current call emitted, as `(topic, payload)`.
    emitted: Vec<(String, String)>,
    /// Registering is only allowed while `init` runs.
    initializing: bool,
    services: Arc<dyn HostServices>,
    logs: LogBuffer,
    /// Set only while a command runs, with whether it declared
    /// `mutates-context`.
    context: Option<(ContextPtr, bool)>,
    /// The manifest's `[network] hosts`.
    hosts: Vec<String>,
    /// When the current call began, and how long it has spent in
    /// `http-fetch`, which doesn't count toward its limit.
    started: Instant,
    waited: Duration,
    /// The conversation the current call is acting on, if any.
    conversation: Option<i64>,
    /// Whether a user is waiting for the current call; see [`acting`].
    blocking: bool,
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
            Hook::PromptBuild | Hook::LorebookActivate => Some("prompt:write"),
            Hook::AfterGenerate | Hook::TurnStart | Hook::TurnEnd => Some("chat:read"),
            Hook::MessageFinalize | Hook::UserMessage => Some("chat:write"),
            Hook::ToolCall | Hook::ToolResult => Some("tools:write"),
            Hook::Upgrade | Hook::SettingsChanged | Hook::Shutdown => None,
        };
        if let Some(permission) = permission {
            self.require(permission)?;
        }
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

    async fn emit(&mut self, topic: String, payload: String) -> Result<(), String> {
        if self.initializing {
            return Err("events can't be emitted during `init`".into());
        }
        let declared = topic
            .strip_prefix(self.handle.as_str())
            .and_then(|t| t.strip_prefix('.'))
            .is_some_and(|name| self.provides.iter().any(|e| e == name));
        if !declared {
            let message = format!(
                "`{topic}` isn't `{}.<name>` with a name in the manifest's `[provides] events`",
                self.handle
            );
            self.logs
                .record(Level::Warn, LogSource::Host, message.clone());
            return Err(message);
        }
        self.emitted.push((topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: String) -> Result<(), String> {
        if !self.subscriptions.contains(&topic) {
            self.subscriptions.push(topic);
        }
        Ok(())
    }

    async fn unsubscribe(&mut self, topic: String) -> Result<(), String> {
        self.subscriptions.retain(|t| *t != topic);
        Ok(())
    }

    async fn send_frontend(&mut self, name: String, payload: String) -> Result<(), String> {
        self.services.send_frontend(name, payload).await
    }

    async fn active_persona(&mut self) -> Result<Persona, String> {
        self.require("persona:read")?;
        self.services.active_persona().await
    }

    async fn settings(&mut self) -> Result<String, String> {
        self.services.settings().await
    }

    async fn http_fetch(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        self.require("net:fetch")?;
        if !allowed(&self.hosts, &request.url) {
            let message = format!("`{}` isn't in the manifest's network hosts", request.url);
            self.logs
                .record(Level::Warn, LogSource::Host, message.clone());
            return Err(message);
        }
        let start = Instant::now();
        let response = self.services.http_fetch(request).await;
        self.waited += start.elapsed();
        response
    }

    async fn messages(
        &mut self,
        conversation: i64,
        range: MessageRange,
    ) -> Result<Vec<ChatMessage>, String> {
        self.require("chat:read")?;
        self.services.messages(conversation, range).await
    }

    async fn generate(
        &mut self,
        messages: Vec<Message>,
        profile: Option<i64>,
    ) -> Result<String, String> {
        self.require("model:generate")?;
        let (conversation, blocking) = (self.conversation, self.blocking);
        let start = Instant::now();
        let reply = self
            .services
            .generate(messages, profile, conversation, blocking)
            .await;
        self.waited += start.elapsed();
        reply
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
            deadline: ticks(call_timeout).max(1),
        })
    }

    /// Reads `manifest.toml` from `dir` and compiles the backend component.
    pub fn compile(&self, dir: &Path) -> Result<PluginPackage, PluginError> {
        let manifest = Manifest::from_dir(dir)?;
        check_api(&manifest)?;
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
            handle: package.manifest.plugin.handle.clone(),
            provides: package.manifest.provides.events.clone(),
            subscriptions: Vec::new(),
            emitted: Vec::new(),
            initializing: true,
            services,
            logs,
            context: None,
            hosts: package.manifest.network.hosts.clone(),
            started: Instant::now(),
            waited: Duration::ZERO,
            conversation: None,
            blocking: false,
        };
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(self.deadline);
        // Time spent in `http-fetch` is given back.
        let deadline = self.deadline;
        store.epoch_deadline_callback(move |store| {
            let state = store.data();
            let used = ticks(state.started.elapsed().saturating_sub(state.waited));
            Ok(match deadline.checked_sub(used) {
                Some(left) if left > 0 => UpdateDeadline::Continue(left),
                _ => UpdateDeadline::Interrupt,
            })
        });
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
        prompt: PromptAssembly,
    ) -> Result<Vec<Message>, PluginError> {
        if !self.store.data().hooks.contains(&Hook::PromptBuild) {
            return Ok(prompt.messages);
        }
        match self.handle(Event::PromptBuild(prompt)).await? {
            Response::PromptBuild(prompt) => Ok(prompt),
            _ => Err(mismatch("prompt-build")),
        }
    }

    /// Runs the `after-generate` hook, if the plugin registered it.
    pub async fn after_generate(&mut self, generated: Generated) -> Result<(), PluginError> {
        if !self.store.data().hooks.contains(&Hook::AfterGenerate) {
            return Ok(());
        }
        match self.handle(Event::AfterGenerate(generated)).await? {
            Response::AfterGenerate => Ok(()),
            _ => Err(mismatch("after-generate")),
        }
    }

    /// Runs the `message-finalize` hook. Returns the content unchanged if the
    /// plugin did not register it.
    pub async fn message_finalize(&mut self, reply: Finalizing) -> Result<String, PluginError> {
        if !self.store.data().hooks.contains(&Hook::MessageFinalize) {
            return Ok(reply.content);
        }
        match self.handle(Event::MessageFinalize(reply)).await? {
            Response::MessageFinalize(content) => Ok(content),
            _ => Err(mismatch("message-finalize")),
        }
    }

    /// Runs the `user-message` hook. Allows the input if the plugin did not
    /// register it.
    pub async fn user_message(&mut self, input: UserInput) -> Result<Verdict, PluginError> {
        if !self.store.data().hooks.contains(&Hook::UserMessage) {
            return Ok(Verdict::Allow);
        }
        match self.handle(Event::UserMessage(input)).await? {
            Response::UserMessage(verdict) => Ok(verdict),
            _ => Err(mismatch("user-message")),
        }
    }

    /// Runs the `lorebook-activate` hook. Returns the entries unchanged if the
    /// plugin did not register it.
    pub async fn lorebook_activate(
        &mut self,
        activation: LorebookActivation,
    ) -> Result<Vec<LorebookEntry>, PluginError> {
        if !self.store.data().hooks.contains(&Hook::LorebookActivate) {
            return Ok(activation.entries);
        }
        match self.handle(Event::LorebookActivate(activation)).await? {
            Response::LorebookActivate(entries) => Ok(entries),
            _ => Err(mismatch("lorebook-activate")),
        }
    }

    /// Runs the `tool-call` hook. Allows the call if the plugin did not
    /// register it.
    pub async fn tool_call(&mut self, call: ToolInvocation) -> Result<Verdict, PluginError> {
        if !self.store.data().hooks.contains(&Hook::ToolCall) {
            return Ok(Verdict::Allow);
        }
        match self.handle(Event::ToolCall(call)).await? {
            Response::ToolCall(verdict) => Ok(verdict),
            _ => Err(mismatch("tool-call")),
        }
    }

    /// Runs the `tool-result` hook. Returns the result unchanged if the plugin
    /// did not register it.
    pub async fn tool_result(&mut self, output: ToolOutput) -> Result<String, PluginError> {
        if !self.store.data().hooks.contains(&Hook::ToolResult) {
            return Ok(output.result);
        }
        match self.handle(Event::ToolResult(output)).await? {
            Response::ToolResult(result) => Ok(result),
            _ => Err(mismatch("tool-result")),
        }
    }

    /// Runs the `turn-start` hook, if the plugin registered it.
    pub async fn turn_start(&mut self, turn: TurnStarted) -> Result<(), PluginError> {
        if !self.store.data().hooks.contains(&Hook::TurnStart) {
            return Ok(());
        }
        match self.handle(Event::TurnStart(turn)).await? {
            Response::TurnStart => Ok(()),
            _ => Err(mismatch("turn-start")),
        }
    }

    /// Runs the `turn-end` hook, if the plugin registered it.
    pub async fn turn_end(&mut self, turn: TurnEnded) -> Result<(), PluginError> {
        if !self.store.data().hooks.contains(&Hook::TurnEnd) {
            return Ok(());
        }
        match self.handle(Event::TurnEnd(turn)).await? {
            Response::TurnEnd => Ok(()),
            _ => Err(mismatch("turn-end")),
        }
    }

    /// Runs the `upgrade` hook, if the plugin registered it.
    pub async fn upgrade(&mut self, from: Option<String>) -> Result<(), PluginError> {
        if !self.store.data().hooks.contains(&Hook::Upgrade) {
            return Ok(());
        }
        match self.handle(Event::Upgrade(from)).await? {
            Response::Upgrade => Ok(()),
            _ => Err(mismatch("upgrade")),
        }
    }

    /// Runs the `settings-changed` hook, if the plugin registered it.
    pub async fn settings_changed(&mut self) -> Result<(), PluginError> {
        if !self.store.data().hooks.contains(&Hook::SettingsChanged) {
            return Ok(());
        }
        match self.handle(Event::SettingsChanged).await? {
            Response::SettingsChanged => Ok(()),
            _ => Err(mismatch("settings-changed")),
        }
    }

    /// Runs the `shutdown` hook, if the plugin registered it.
    pub async fn shutdown(&mut self) -> Result<(), PluginError> {
        if !self.store.data().hooks.contains(&Hook::Shutdown) {
            return Ok(());
        }
        match self.handle(Event::Shutdown).await? {
            Response::Shutdown => Ok(()),
            _ => Err(mismatch("shutdown")),
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
        conversation: Option<i64>,
    ) -> Result<Vec<Block>, PluginError> {
        match self
            .handle(Event::Source(SourceCall {
                node,
                conversation,
                settings,
            }))
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
        conversation: Option<i64>,
        message: Option<i64>,
        blocking: bool,
    ) -> Result<String, PluginError> {
        match self
            .handle(Event::Frontend(FrontendCall {
                name,
                args,
                conversation,
                message,
                blocking,
            }))
            .await?
        {
            Response::Frontend(value) => Ok(value),
            _ => Err(mismatch("frontend")),
        }
    }

    /// Delivers an event emitted under `topic`. Does nothing if the plugin
    /// didn't subscribe to it.
    pub async fn custom_event(
        &mut self,
        topic: String,
        payload: String,
    ) -> Result<(), PluginError> {
        if !self.store.data().subscriptions.contains(&topic) {
            return Ok(());
        }
        match self
            .handle(Event::Custom(CustomEvent { topic, payload }))
            .await?
        {
            Response::Custom => Ok(()),
            _ => Err(mismatch("custom")),
        }
    }

    /// Takes the events the last call emitted, as `(topic, payload)`. A failed
    /// call's events are dropped.
    pub fn take_emitted(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.store.data_mut().emitted)
    }

    async fn handle(&mut self, event: Event) -> Result<Response, PluginError> {
        self.store.set_epoch_deadline(self.deadline);
        let state = self.store.data_mut();
        state.emitted.clear();
        state.started = Instant::now();
        state.waited = Duration::ZERO;
        (state.conversation, state.blocking) = acting(&event);
        let result = match self.instance.call_handle(&mut self.store, &event).await {
            Ok(result) => result.map_err(PluginError::Call),
            Err(err) => Err(err.into()),
        };
        flush_logs(&mut self.store, result.as_ref().err()).await;
        if result.is_err() {
            self.store.data_mut().emitted.clear();
        }
        result
    }
}

/// What a call is acting on
fn acting(event: &Event) -> (Option<i64>, bool) {
    match event {
        Event::PromptBuild(prompt) => (prompt.conversation, false),
        Event::LorebookActivate(activation) => (activation.conversation, false),
        Event::Source(call) => (call.conversation, false),
        Event::Frontend(call) => (call.conversation, call.blocking),
        Event::AfterGenerate(generated) => (Some(generated.conversation), false),
        Event::MessageFinalize(reply) => (Some(reply.conversation), false),
        Event::UserMessage(input) => (Some(input.conversation), false),
        Event::TurnStart(turn) => (Some(turn.conversation), false),
        Event::TurnEnd(turn) => (Some(turn.conversation), false),
        Event::ToolCall(_)
        | Event::ToolResult(_)
        | Event::Command(_)
        | Event::Processor(_)
        | Event::Custom(_)
        | Event::Upgrade(_)
        | Event::SettingsChanged
        | Event::Shutdown => (None, false),
    }
}

fn ticks(time: Duration) -> u64 {
    (time.as_millis() / EPOCH_TICK.as_millis()) as u64
}

/// Whether `url` is http(s) to one of `hosts`. A host alone allows the
/// scheme's default port; `host:port` allows that port.
fn allowed(hosts: &[String], url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = url
        .host_str()
        .filter(|_| matches!(url.scheme(), "http" | "https"))
    else {
        return false;
    };
    let target = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    hosts.iter().any(|h| h.eq_ignore_ascii_case(&target))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_is_the_wit_package_version() {
        let wit = include_str!("../wit/plugin.wit");
        assert!(
            wit.starts_with(&format!("package lorealm:plugin@{API_VERSION};")),
            "API_VERSION and the package version in wit/plugin.wit have to change together"
        );
    }

    #[test]
    fn the_rust_sdk_bundles_the_same_wit() {
        assert!(
            include_str!("../wit/plugin.wit") == include_str!("../../../sdk/rust/wit/plugin.wit"),
            "copy wit/plugin.wit to sdk/rust/wit/plugin.wit"
        );
        let sdk = include_str!("../../../sdk/rust/Cargo.toml");
        assert!(
            sdk.contains(&format!("\nversion = \"{API_VERSION}\"\n")),
            "the SDK's version is the plugin API version"
        );
    }

    #[test]
    fn refuses_another_api_version() {
        let manifest = |api: &str| -> Manifest {
            toml::from_str(&format!(
                r#"
                [plugin]
                name = "Hello"
                handle = "hello"
                description = ""
                author = "Test"
                version = "1.0.0"
                api = "{api}"
                "#
            ))
            .unwrap()
        };

        assert!(check_api(&manifest(&API_VERSION.to_string())).is_ok());
        let error = check_api(&manifest("0.0.9")).unwrap_err();
        assert!(matches!(&error, PluginError::Api { plugin } if *plugin == Version::new(0, 0, 9)));
        assert!(error.to_string().contains("0.0.9"));
    }
}
