use std::{
    collections::VecDeque,
    convert::Infallible,
    ops::ControlFlow,
    path::PathBuf,
    sync::{Arc, OnceLock, Weak},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use futures_util::Stream;
use lorealm_plugins::{
    ChatMessage, Finalizing, FrontendSlot, Generated, HostServices, HttpRequest, HttpResponse,
    Level, LoadedPlugin, LogEntry, LogSource, Manifest, Message, MessageRange, Persona, PluginHost,
    PluginInfo, PluginPackage, PromptAssembly, Role, ToolInvocation, ToolOutput, TurnEnded,
    TurnStarted, UserInput, Verdict, Version, resolve,
};
use lorealm_storage::{
    Storage, StorageError,
    models::{
        ChatRole, PluginLog, PluginLogLevel, PluginLogSource, Snowflake, UserPlugin, is_identifier,
    },
};
use secrecy::zeroize::Zeroizing;
use serde::Serialize;
use tokio::sync::{Mutex, broadcast};
use tokio_util::sync::CancellationToken;
use weaver_lang::InputType;

mod fetch;
mod install;
mod settings;
mod vocabulary;

pub use install::{InstallReport, MAX_PACKAGE};
pub use vocabulary::{PLUGIN_SOURCE_PREFIX, PluginVocabulary};

use crate::{
    config::PluginConfig,
    core::CleanableService,
    errors::PluginError,
    services::{
        blueprint::InputSpec, chat::PromptMessage, commands::Generation,
        connection::ConnectionService, context::PromptRole, snowflake::SnowflakeService,
        state::StateService,
    },
};

/// How long a single call into a plugin may run.
const CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// How many events deep a chain of events, each emitted while handling the
/// last, may go.
const MAX_EVENT_DEPTH: usize = 8;

/// How long a user's plugins stay mounted without being used.
const IDLE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Serialize)]
pub struct InstalledPlugin {
    #[serde(flatten)]
    pub info: PluginInfo,
    /// If so, `info` is the version the user is on; otherwise the latest.
    pub enabled: bool,
    /// The newest version that can be enabled, if any.
    pub latest: Option<Version>,
    /// Where the plugin appears in the client.
    pub slots: Vec<FrontendSlot>,
    /// The API key types it declares.
    pub api_keys: Vec<String>,
    /// Whether it ships a `settings.wtn` for the user to fill in.
    pub has_settings: bool,
    /// Where it may send requests.
    pub hosts: Vec<String>,
}

/// A gate hook's refusal.
#[derive(Debug, thiserror::Error)]
#[error("refused by plugin `{handle}`: {reason}")]
pub struct Refusal {
    pub handle: String,
    pub reason: String,
}

impl Refusal {
    fn new(handle: &str, reason: String) -> Self {
        Self {
            handle: handle.to_owned(),
            reason,
        }
    }
}

/// What a plugin's settings ask for, and the user's values.
#[derive(Default, Serialize)]
pub struct PluginSettings {
    pub inputs: Vec<InputSpec>,
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// What a plugin's backend sent its frontend.
#[derive(Clone, Serialize)]
pub struct FrontendEvent {
    #[serde(skip)]
    user_id: Snowflake,
    pub handle: String,
    pub name: String,
    pub payload: serde_json::Value,
}

/// A user's mounted plugins, in load order.
struct Mounted {
    user_id: Snowflake,
    plugins: Vec<(String, LoadedPlugin)>,
    last_used: Instant,
    logs: Arc<PluginLogs>,
}

/// One user's mount slot: `None` until their plugins are first needed.
type Slot = Arc<Mutex<Option<Mounted>>>;

impl Mounted {
    /// Runs `op` against `handle`'s plugin, then delivers the events it
    /// emitted.
    async fn call<T>(
        &mut self,
        handle: &str,
        op: impl AsyncFnOnce(&mut LoadedPlugin) -> Result<T, lorealm_plugins::PluginError>,
    ) -> Result<T, String> {
        self.last_used = Instant::now();
        let Some(i) = self.plugins.iter().position(|(h, _)| h == handle) else {
            return Err(format!("plugin `{handle}` is not loaded"));
        };
        let value = self.run(i, op).await?;
        let emitted = self.plugins[i].1.take_emitted();
        self.deliver(emitted).await;
        Ok(value)
    }

    /// Hands `value` to each plugin in load order, each seeing the last one's
    /// output, until `op` breaks. A plugin that fails is skipped; the plugin
    /// crate logged why.
    async fn pipe<T: Clone, B>(
        &mut self,
        mut value: T,
        op: impl AsyncFn(
            &str,
            &mut LoadedPlugin,
            T,
        ) -> Result<ControlFlow<B, T>, lorealm_plugins::PluginError>,
    ) -> Result<T, B> {
        let handles: Vec<String> = self.plugins.iter().map(|(h, _)| h.clone()).collect();
        for handle in handles {
            let input = value.clone();
            match self
                .call(&handle, async |p| op(&handle, p, input).await)
                .await
            {
                Ok(ControlFlow::Continue(next)) => value = next,
                Ok(ControlFlow::Break(b)) => return Err(b),
                Err(_) => {}
            }
        }
        Ok(value)
    }

    /// Lets each plugin in `slot` flush with `shutdown`, then empties it.
    async fn shut_down(slot: &mut Option<Mounted>) {
        if let Some(mounted) = slot.as_mut() {
            // Dependents first, so a plugin's dependencies outlive it.
            mounted.plugins.reverse();
            let _: Result<_, Infallible> = mounted
                .pipe((), async move |_, p, ()| {
                    p.shutdown().await?;
                    Ok(ControlFlow::Continue(()))
                })
                .await;
        }
        *slot = None;
    }

    /// Hands each event to every plugin in load order, then the events those
    /// emitted, up to [`MAX_EVENT_DEPTH`] deep.
    async fn deliver(&mut self, emitted: Vec<(String, String)>) {
        let mut queue: VecDeque<_> = emitted.into_iter().map(|e| (e, 1)).collect();
        while let Some(((topic, payload), depth)) = queue.pop_front() {
            let mut i = 0;
            while i < self.plugins.len() {
                let count = self.plugins.len();
                let (t, p) = (topic.clone(), payload.clone());
                let _ = self
                    .run(i, async move |plugin| plugin.custom_event(t, p).await)
                    .await;
                // It crashed and was removed.
                if self.plugins.len() < count {
                    continue;
                }
                let emitted = self.plugins[i].1.take_emitted();
                if depth < MAX_EVENT_DEPTH {
                    queue.extend(emitted.into_iter().map(|e| (e, depth + 1)));
                } else if !emitted.is_empty() {
                    let message = format!(
                        "Dropped the events it emitted: a chain of events stops at {MAX_EVENT_DEPTH}."
                    );
                    let handle = &self.plugins[i].0;
                    self.logs
                        .host(self.user_id, handle, Level::Warn, message)
                        .await;
                }
                i += 1;
            }
        }
    }

    /// Runs `op` against the `i`th plugin. A plugin whose call trapped (f.ex.
    /// timed out) can't be entered again, so it is unmounted until the user's
    /// plugins next load.
    async fn run<T>(
        &mut self,
        i: usize,
        op: impl AsyncFnOnce(&mut LoadedPlugin) -> Result<T, lorealm_plugins::PluginError>,
    ) -> Result<T, String> {
        let handle = self.plugins[i].0.clone();
        // On failure, the plugin crate logged why.
        let err = match op(&mut self.plugins[i].1).await {
            Ok(value) => return Ok(value),
            Err(err) => err,
        };
        if !matches!(err, lorealm_plugins::PluginError::Call(_)) {
            let message =
                "Plugin stopped after a crash. It will load again the next time your plugins load.";
            self.logs
                .host(self.user_id, &handle, Level::Warn, message.into())
                .await;
            self.plugins.remove(i);
        }
        Err(err.to_string())
    }
}

pub struct PluginService {
    storage: Arc<dyn Storage>,
    state: Arc<StateService>,
    host: Arc<PluginHost>,
    logs: Arc<PluginLogs>,
    /// `{data_directory}/plugins`, one directory per installed version, named
    /// `{handle}@{version}`.
    dir: PathBuf,
    /// Compiled components by `handle@version`. Shared by every user.
    packages: DashMap<String, Arc<PluginPackage>>,
    /// `None` until the user's plugins are first needed.
    mounted: DashMap<Snowflake, Slot>,
    /// Decrypts the keys plugins put in their requests.
    connection: Arc<ConnectionService>,
    /// Answers `generate`. Weak, since the chat service holds this one.
    generation: OnceLock<Weak<dyn Generation>>,
    /// How long a plugin's `generate` may wait for the model.
    generate_timeout: Duration,
    /// Sends every plugin's `http-fetch` requests.
    http: reqwest::Client,
    /// What every user's plugins send their frontends.
    frontend: broadcast::Sender<FrontendEvent>,
}

impl PluginService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        state: Arc<StateService>,
        connection: Arc<ConnectionService>,
        config: PluginConfig,
        dir: PathBuf,
    ) -> Self {
        Self {
            logs: Arc::new(PluginLogs {
                storage: Arc::clone(&storage),
                snowflake,
                cap: config.log_cap,
            }),
            storage,
            state,
            host: Arc::new(PluginHost::new(CALL_TIMEOUT).expect("plugin host should initialize")),
            dir,
            packages: DashMap::new(),
            mounted: DashMap::new(),
            connection,
            generation: OnceLock::new(),
            generate_timeout: Duration::from_secs(config.generate_timeout),
            http: fetch::client(),
            frontend: broadcast::channel(256).0,
        }
    }

    /// Hands the service what answers `generate`.
    pub fn set_generation(&self, generation: Weak<dyn Generation>) {
        let _ = self.generation.set(generation);
    }

    /// Every installed plugin, at the version the user is on or else the
    /// latest.
    pub async fn list(
        &self,
        user_id: Snowflake,
        allow_unsafe: bool,
    ) -> Result<Vec<InstalledPlugin>, PluginError> {
        let enabled = self.storage.list_user_plugins(user_id).await?;

        let mut installed = Vec::new();
        for handle in self.installed_handles() {
            let Some(version) = self.shown_version(&enabled, &handle) else {
                continue;
            };
            match self.package(&handle, &version).await {
                Ok(package) => installed.push(InstalledPlugin {
                    enabled: enabled.iter().any(|p| p.handle == handle),
                    latest: self.latest(&handle),
                    info: package.manifest.plugin.clone(),
                    slots: usable_slots(&package.manifest, allow_unsafe)
                        .cloned()
                        .collect(),
                    api_keys: package.manifest.definitions.api_keys.clone(),
                    hosts: package.manifest.network.hosts.clone(),
                    has_settings: self
                        .version_dir(&handle, &version)
                        .join(&package.manifest.structure.settings)
                        .is_file(),
                }),
                Err(err) => tracing::warn!(%handle, error = %err, "skipping plugin"),
            }
        }
        Ok(installed)
    }

    pub async fn enable(&self, user_id: Snowflake, handle: &str) -> Result<(), PluginError> {
        let version = self
            .latest(handle)
            .ok_or_else(|| PluginError::NotFound(handle.to_owned()))?;
        let package = self.package(handle, &version).await?;
        let enabled = self.storage.list_user_plugins(user_id).await?;
        if enabled.iter().any(|p| p.handle == handle) {
            return Ok(());
        }

        let others = self.installed(&enabled).await;
        let manifests: Vec<_> = others
            .iter()
            .map(|p| &p.manifest)
            .chain([&package.manifest])
            .collect();
        if let Some((_, reason)) = resolve(&manifests)
            .skipped
            .into_iter()
            .find(|(h, _)| h == handle)
        {
            return Err(PluginError::CannotEnable(handle.to_owned(), reason));
        }

        self.storage
            .enable_user_plugin(user_id, handle, &version.to_string())
            .await?;
        self.unmount(user_id).await;
        Ok(())
    }

    /// Moves the user to `version` of the plugin, or the latest. Fails if the
    /// user's plugins wouldn't load together on it.
    pub async fn update(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: Option<Version>,
    ) -> Result<(), PluginError> {
        let not_found = || PluginError::NotFound(handle.to_owned());
        let enabled = self.storage.list_user_plugins(user_id).await?;
        let current = enabled
            .iter()
            .find(|p| p.handle == handle)
            .ok_or_else(not_found)?;
        let target = match version {
            Some(v) if self.is_available(handle, &v) => v,
            Some(_) => return Err(not_found()),
            None => self.latest(handle).ok_or_else(not_found)?,
        };
        if current.version == target.to_string() {
            return Ok(());
        }

        let package = self.package(handle, &target).await?;
        let others: Vec<UserPlugin> = enabled
            .iter()
            .filter(|p| p.handle != handle)
            .cloned()
            .collect();
        let others = self.installed(&others).await;
        let before = self.installed(&enabled).await;
        let skipped = |packages: Vec<&Manifest>| resolve(&packages).skipped;
        let was_skipped = skipped(before.iter().map(|p| &p.manifest).collect());
        let now_skipped = skipped(
            others
                .iter()
                .map(|p| &p.manifest)
                .chain([&package.manifest])
                .collect(),
        );
        if let Some((skipped, reason)) = now_skipped
            .into_iter()
            .find(|(h, _)| !was_skipped.iter().any(|(was, _)| was == h))
        {
            return Err(PluginError::CannotUpdate(
                handle.to_owned(),
                format!("`{skipped}` {reason}"),
            ));
        }

        self.storage
            .set_user_plugin_version(user_id, handle, &target.to_string())
            .await?;
        self.unmount(user_id).await;
        self.collect(handle).await;
        Ok(())
    }

    /// Disable this plugin.
    ///
    /// Fails while other enabled plugins require this one.
    pub async fn disable(&self, user_id: Snowflake, handle: &str) -> Result<(), PluginError> {
        let enabled = self.storage.list_user_plugins(user_id).await?;
        let required_by: Vec<String> = self
            .installed(&enabled)
            .await
            .iter()
            .filter(|p| p.manifest.compatibility.requires.contains_key(handle))
            .map(|p| p.manifest.plugin.handle.clone())
            .collect();
        if !required_by.is_empty() {
            return Err(PluginError::Required(handle.to_owned(), required_by));
        }

        self.storage.disable_user_plugin(user_id, handle).await?;
        self.unmount(user_id).await;
        self.collect(handle).await;
        Ok(())
    }

    /// Sets the user's load order. Manifest constraints still take precedence.
    pub async fn set_order(
        &self,
        user_id: Snowflake,
        handles: Vec<String>,
    ) -> Result<(), PluginError> {
        let mut enabled: Vec<String> = self
            .storage
            .list_user_plugins(user_id)
            .await?
            .into_iter()
            .map(|p| p.handle)
            .collect();
        let mut given = handles.clone();
        enabled.sort();
        given.sort();
        if enabled != given {
            return Err(PluginError::InvalidOrder);
        }

        self.storage
            .set_user_plugin_order(user_id, &handles)
            .await?;
        self.unmount(user_id).await;
        Ok(())
    }

    /// What the user's loaded plugins add to Weaver and context templates.
    /// `None` if they have none loaded.
    pub async fn vocabulary(&self, user_id: Snowflake) -> Option<PluginVocabulary> {
        let slot = self.mount(user_id).await;
        // Before locking: nothing may be awaited while `plugins` is borrowed.
        let versions = self
            .storage
            .list_user_plugins(user_id)
            .await
            .unwrap_or_default();
        let guard = slot.lock().await;
        let plugins = &guard.as_ref()?.plugins;
        if plugins.is_empty() {
            return None;
        }
        let infos = plugins
            .iter()
            .filter_map(|(handle, _)| {
                let pinned = versions.iter().find(|p| &p.handle == handle)?;
                let key = format!("{handle}@{}", pinned.version);
                let info = &self.packages.get(&key)?.manifest.plugin;
                Some((
                    handle.clone(),
                    (info.name.clone(), info.description.clone()),
                ))
            })
            .collect();
        Some(PluginVocabulary::new(Arc::clone(&slot), plugins, &infos))
    }

    /// A frontend entry the plugin declared, if the user has it enabled.
    pub async fn frontend_entry(
        &self,
        user_id: Snowflake,
        handle: &str,
        entry: &str,
        allow_unsafe: bool,
    ) -> Result<String, PluginError> {
        let not_found = || PluginError::NotFound(handle.to_owned());
        let version = self
            .storage
            .list_user_plugins(user_id)
            .await?
            .into_iter()
            .find(|p| p.handle == handle)
            .and_then(|p| p.version.parse::<Version>().ok())
            .ok_or_else(not_found)?;
        let package = self.package(handle, &version).await?;
        let frontend = &package.manifest.frontend;
        if !usable_slots(&package.manifest, allow_unsafe).any(|s| s.entry == entry) {
            return Err(not_found());
        }
        let path = self
            .version_dir(handle, &version)
            .join(&frontend.path)
            .join(entry);
        tokio::fs::read_to_string(path)
            .await
            .map_err(|_| not_found())
    }

    /// Calls the plugin's backend on behalf of its frontend.
    pub async fn call_frontend(
        &self,
        user_id: Snowflake,
        handle: &str,
        name: String,
        args: serde_json::Value,
        conversation: Option<Snowflake>,
        message: Option<Snowflake>,
        blocking: bool,
    ) -> Result<serde_json::Value, PluginError> {
        let slot = self.mount(user_id).await;
        let mut guard = slot.lock().await;
        let mounted = guard
            .as_mut()
            .filter(|m| m.plugins.iter().any(|(h, _)| h == handle))
            .ok_or_else(|| PluginError::NotFound(handle.to_owned()))?;
        let args = args.to_string();
        let (conversation, message) = (conversation.map(|id| id.0), message.map(|id| id.0));
        let json = mounted
            .call(handle, async |p| {
                p.call_frontend(name, args, conversation, message, blocking)
                    .await
            })
            .await
            .map_err(|e| PluginError::CallFailed(handle.to_owned(), e))?;
        serde_json::from_str(&json)
            .map_err(|_| PluginError::CallFailed(handle.to_owned(), "returned invalid JSON".into()))
    }

    /// What the user's plugins send their frontends from now on. Events
    /// missed while the client falls behind are dropped.
    pub fn frontend_events(&self, user_id: Snowflake) -> impl Stream<Item = FrontendEvent> + use<> {
        futures_util::stream::unfold(self.frontend.subscribe(), move |mut events| async move {
            loop {
                match events.recv().await {
                    Ok(event) if event.user_id == user_id => return Some((event, events)),
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        })
    }

    /// The plugin's README, at the version the user is on or else the latest.
    pub async fn readme(&self, user_id: Snowflake, handle: &str) -> Result<String, PluginError> {
        let not_found = || PluginError::NotFound(handle.to_owned());
        let enabled = self.storage.list_user_plugins(user_id).await?;
        let version = self.shown_version(&enabled, handle).ok_or_else(not_found)?;
        let package = self.package(handle, &version).await?;
        let path = self
            .version_dir(handle, &version)
            .join(&package.manifest.structure.readme);
        tokio::fs::read_to_string(path)
            .await
            .map_err(|_| not_found())
    }

    /// A given plugin's settings for a given user
    pub async fn settings(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<PluginSettings, PluginError> {
        let Some(source) = self.settings_source(user_id, handle).await? else {
            return Ok(PluginSettings::default());
        };
        let invalid = |reason| PluginError::InvalidSettings(handle.to_owned(), reason);
        let inputs = settings::declarations(&source).map_err(invalid)?;
        let saved = settings::saved(&*self.storage, user_id, handle).await?;
        let keys = self.keys(user_id).await?;
        let values = match settings::resolve(&source, &saved, &keys) {
            Ok(serde_json::Value::Object(values)) => values,
            _ => saved,
        };
        Ok(PluginSettings {
            inputs: inputs.iter().map(InputSpec::of).collect(),
            values,
        })
    }

    /// Saves the user's settings for `handle`, if they fit what it asks for.
    pub async fn save_settings(
        &self,
        user_id: Snowflake,
        handle: &str,
        values: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), PluginError> {
        let invalid = |reason| PluginError::InvalidSettings(handle.to_owned(), reason);
        let source = self.settings_source(user_id, handle).await?;
        let keys = self.keys(user_id).await?;
        settings::resolve(source.as_deref().unwrap_or("#inputs"), &values, &keys)
            .map_err(invalid)?;
        self.storage
            .set_plugin_settings(user_id, handle, &serde_json::Value::Object(values))
            .await?;

        // A plugin that isn't loaded reads them when it next loads.
        if let Some(slot) = self.mounted.get(&user_id).map(|s| Arc::clone(s.value())) {
            let handle = handle.to_owned();
            tokio::spawn(async move {
                if let Some(mounted) = slot.lock().await.as_mut() {
                    let _ = mounted
                        .call(&handle, async move |p| p.settings_changed().await)
                        .await;
                }
            });
        }
        Ok(())
    }

    /// The API key types the user's enabled plugins declare.
    pub async fn key_types(&self, user_id: Snowflake) -> Result<Vec<String>, PluginError> {
        let enabled = self.storage.list_user_plugins(user_id).await?;
        let mut types: Vec<String> = self
            .installed(&enabled)
            .await
            .iter()
            .flat_map(|p| p.manifest.definitions.api_keys.clone())
            .collect();
        types.sort();
        types.dedup();
        Ok(types)
    }

    /// Each key type the user's plugins declare, with the user's keys of it.
    async fn keys(&self, user_id: Snowflake) -> Result<settings::Keys, PluginError> {
        let types = self.key_types(user_id).await?;
        Ok(settings::keys(&*self.storage, user_id, types).await?)
    }

    /// The `settings.wtn` of the version shown to the user, if it has one.
    async fn settings_source(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<Option<String>, PluginError> {
        let enabled = self.storage.list_user_plugins(user_id).await?;
        let version = self
            .shown_version(&enabled, handle)
            .ok_or_else(|| PluginError::NotFound(handle.to_owned()))?;
        let package = self.package(handle, &version).await?;
        let path = self
            .version_dir(handle, &version)
            .join(&package.manifest.structure.settings);
        Ok(tokio::fs::read_to_string(path).await.ok())
    }

    /// The version to show the user: the one they're on, or else the latest.
    fn shown_version(&self, enabled: &[UserPlugin], handle: &str) -> Option<Version> {
        match enabled.iter().find(|p| p.handle == handle) {
            Some(pinned) => pinned.version.parse().ok(),
            None => self.latest(handle),
        }
    }

    /// The user's log for `handle`, newest first.
    pub async fn logs(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<Vec<PluginLog>, PluginError> {
        Ok(self
            .storage
            .list_plugin_logs(user_id, handle, self.logs.cap)
            .await?)
    }

    /// Runs the user's `prompt-build` hooks. A plugin that fails is logged and
    /// skipped.
    pub async fn prompt_build(
        &self,
        user_id: Snowflake,
        conversation: Option<Snowflake>,
        prompt: Vec<PromptMessage>,
    ) -> Vec<PromptMessage> {
        let messages: Vec<Message> = prompt.into_iter().map(Into::into).collect();
        let conversation = conversation.map(|id| id.0);
        let Ok(messages): Result<_, Infallible> = self
            .pipe(user_id, messages, async move |_, p, messages| {
                let assembly = PromptAssembly {
                    conversation,
                    messages,
                };
                Ok(ControlFlow::Continue(p.prompt_build(assembly).await?))
            })
            .await;
        messages.into_iter().map(Into::into).collect()
    }

    /// Runs the user's `user-message` hooks until one refuses. A plugin that
    /// fails is logged and skipped.
    pub async fn user_message(
        &self,
        user_id: Snowflake,
        conversation: Snowflake,
        content: String,
    ) -> Result<String, Refusal> {
        self.pipe(user_id, content, async move |handle, p, content| {
            let input = UserInput {
                conversation: conversation.0,
                content: content.clone(),
            };
            Ok(match p.user_message(input).await? {
                Verdict::Allow => ControlFlow::Continue(content),
                Verdict::Change(next) => ControlFlow::Continue(next),
                Verdict::Deny(reason) => ControlFlow::Break(Refusal::new(handle, reason)),
            })
        })
        .await
    }

    /// Runs the user's `tool-call` hooks until one refuses. A plugin that
    /// fails, or changes the arguments to something other than JSON, is logged
    /// and skipped.
    pub async fn tool_call(
        &self,
        user_id: Snowflake,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, Refusal> {
        let (logs, name) = (Arc::clone(&self.logs), name.to_owned());
        self.pipe(user_id, arguments, async move |handle, p, arguments| {
            let call = ToolInvocation {
                name: name.clone(),
                arguments: arguments.to_string(),
            };
            Ok(match p.tool_call(call).await? {
                Verdict::Allow => ControlFlow::Continue(arguments),
                Verdict::Change(next) => ControlFlow::Continue(
                    parse(&logs, user_id, handle, &name, "arguments", &next)
                        .await
                        .unwrap_or(arguments),
                ),
                Verdict::Deny(reason) => ControlFlow::Break(Refusal::new(handle, reason)),
            })
        })
        .await
    }

    /// Runs the user's `tool-result` hooks on a tool call's result: its value,
    /// or its error message. A plugin that fails, or changes the result to
    /// something other than JSON, is logged and skipped.
    pub async fn tool_result(
        &self,
        user_id: Snowflake,
        name: &str,
        arguments: &serde_json::Value,
        result: Result<serde_json::Value, String>,
    ) -> Result<serde_json::Value, String> {
        let is_error = result.is_err();
        let value = result.unwrap_or_else(serde_json::Value::String);
        let (logs, name, arguments) = (
            Arc::clone(&self.logs),
            name.to_owned(),
            arguments.to_string(),
        );
        let Ok(value): Result<_, Infallible> = self
            .pipe(user_id, value, async move |handle, p, value| {
                let output = ToolOutput {
                    name: name.clone(),
                    arguments: arguments.clone(),
                    result: value.to_string(),
                    is_error,
                };
                let next = p.tool_result(output).await?;
                Ok(ControlFlow::Continue(
                    parse(&logs, user_id, handle, &name, "result", &next)
                        .await
                        .unwrap_or(value),
                ))
            })
            .await;
        match (is_error, value) {
            (false, value) => Ok(value),
            (true, serde_json::Value::String(message)) => Err(message),
            (true, value) => Err(value.to_string()),
        }
    }

    /// Runs the user's `message-finalize` hooks. A plugin that fails is
    /// logged and skipped.
    pub async fn message_finalize(
        &self,
        user_id: Snowflake,
        conversation: Snowflake,
        content: String,
    ) -> String {
        let Ok(content): Result<_, Infallible> = self
            .pipe(user_id, content, async move |_, p, content| {
                let reply = Finalizing {
                    conversation: conversation.0,
                    content,
                };
                Ok(ControlFlow::Continue(p.message_finalize(reply).await?))
            })
            .await;
        content
    }

    /// Runs the user's `after-generate` hooks. A plugin that fails is logged
    /// and skipped. Callers shouldn't wait for it.
    pub async fn after_generate(
        &self,
        user_id: Snowflake,
        conversation: Snowflake,
        message: Snowflake,
        content: String,
    ) {
        let _: Result<_, Infallible> = self
            .pipe(user_id, (), async move |_, p, ()| {
                let generated = Generated {
                    conversation: conversation.0,
                    message: message.0,
                    content: content.clone(),
                };
                p.after_generate(generated).await?;
                Ok(ControlFlow::Continue(()))
            })
            .await;
    }

    /// Runs the user's `turn-start` hooks. A plugin that fails is logged and
    /// skipped. Callers shouldn't wait for it.
    pub async fn turn_start(&self, user_id: Snowflake, conversation: Snowflake, agent: String) {
        let _: Result<_, Infallible> = self
            .pipe(user_id, (), async move |_, p, ()| {
                let turn = TurnStarted {
                    conversation: conversation.0,
                    agent: agent.clone(),
                };
                p.turn_start(turn).await?;
                Ok(ControlFlow::Continue(()))
            })
            .await;
    }

    /// Runs the user's `turn-end` hooks. A plugin that fails is logged and
    /// skipped. Callers shouldn't wait for it.
    pub async fn turn_end(
        &self,
        user_id: Snowflake,
        conversation: Snowflake,
        agent: String,
        message: Snowflake,
        content: String,
    ) {
        let _: Result<_, Infallible> = self
            .pipe(user_id, (), async move |_, p, ()| {
                let turn = TurnEnded {
                    conversation: conversation.0,
                    agent: agent.clone(),
                    message: message.0,
                    content: content.clone(),
                };
                p.turn_end(turn).await?;
                Ok(ControlFlow::Continue(()))
            })
            .await;
    }

    /// Hands `value` through the user's plugins; see [`Mounted::pipe`].
    /// `op` should be an `async move` closure that owns what it captures:
    /// borrowed captures make the future fail to be `Send` (a rustc
    /// limitation with async closures), which breaks the chat stream.
    async fn pipe<T: Clone, B>(
        &self,
        user_id: Snowflake,
        value: T,
        op: impl AsyncFn(
            &str,
            &mut LoadedPlugin,
            T,
        ) -> Result<ControlFlow<B, T>, lorealm_plugins::PluginError>,
    ) -> Result<T, B> {
        let slot = self.mount(user_id).await;
        let mut guard = slot.lock().await;
        match guard.as_mut() {
            Some(mounted) => mounted.pipe(value, op).await,
            None => Ok(value),
        }
    }

    /// The user's mount slot, filled on first use. A plugin that fails to
    /// load is logged and left out.
    async fn mount(&self, user_id: Snowflake) -> Slot {
        let slot = Arc::clone(self.mounted.entry(user_id).or_default().value());
        let mut guard = slot.lock().await;
        if guard.is_some() {
            drop(guard);
            return slot;
        }

        let enabled = match self.storage.list_user_plugins(user_id).await {
            Ok(enabled) => enabled,
            Err(err) => {
                tracing::error!(%user_id, error = %err, "failed to list plugins");
                Vec::new()
            }
        };

        let mut packages = Vec::new();
        for plugin in &enabled {
            match self.pinned(plugin).await {
                Ok(package) => packages.push(package),
                Err(err) => {
                    self.logs
                        .host(user_id, &plugin.handle, Level::Error, err.to_string())
                        .await;
                }
            }
        }

        let resolution = resolve(&packages.iter().map(|p| &p.manifest).collect::<Vec<_>>());
        for (handle, reason) in resolution.skipped {
            let message = format!("Not loaded: it {reason}.");
            self.logs
                .host(user_id, &handle, Level::Error, message)
                .await;
        }

        let loaded: Vec<_> = resolution
            .order
            .iter()
            .map(|handle| {
                packages
                    .iter()
                    .find(|p| &p.manifest.plugin.handle == handle)
                    .expect("resolved plugins come from `packages`")
            })
            .collect();
        // Settings may hold keys of any type a loaded plugin declares.
        let mut key_types: Vec<String> = loaded
            .iter()
            .flat_map(|p| p.manifest.definitions.api_keys.clone())
            .collect();
        key_types.sort();
        key_types.dedup();
        let key_types = Arc::new(key_types);

        let mut plugins = Vec::new();
        for package in loaded {
            let handle = package.manifest.plugin.handle.clone();
            let settings = self
                .version_dir(&handle, &package.manifest.plugin.version)
                .join(&package.manifest.structure.settings);
            let services = Arc::new(UserServices {
                user_id,
                handle: handle.clone(),
                storage: Arc::clone(&self.storage),
                state: Arc::clone(&self.state),
                logs: Arc::clone(&self.logs),
                settings: tokio::fs::read_to_string(settings).await.ok(),
                key_types: Arc::clone(&key_types),
                connection: Arc::clone(&self.connection),
                generation: self.generation.get().cloned(),
                generate_timeout: self.generate_timeout,
                http: self.http.clone(),
                user_agent: fetch::user_agent(&handle, &package.manifest.plugin.version),
                frontend: self.frontend.clone(),
            });
            // On failure, the plugin crate logged why.
            let Ok(mut plugin) = self.host.instantiate(package, services).await else {
                continue;
            };
            let version = package.manifest.plugin.version.to_string();
            if self.upgrade(user_id, &handle, &version, &mut plugin).await {
                plugins.push((handle, plugin));
            }
        }

        *guard = Some(Mounted {
            user_id,
            plugins,
            last_used: Instant::now(),
            logs: Arc::clone(&self.logs),
        });
        drop(guard);
        slot
    }

    /// Brings the user's data up to `version` of the plugin.
    ///
    /// Returns whether the plugin may run
    async fn upgrade(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
        plugin: &mut LoadedPlugin,
    ) -> bool {
        let result = async {
            let from = self
                .storage
                .get_plugin_data_version(user_id, handle)
                .await?;
            if from.as_deref() != Some(version) {
                // On failure, the plugin crate logged why.
                if plugin.upgrade(from).await.is_err() {
                    return Ok(false);
                }
                self.storage
                    .set_plugin_data_version(user_id, handle, version)
                    .await?;
            }
            Ok::<_, StorageError>(true)
        }
        .await;
        let message = match result {
            Ok(true) => return true,
            Ok(false) => "Not loaded: upgrading your data failed. It will try again the next time your plugins load.".to_owned(),
            Err(err) => format!("Not loaded: {err}"),
        };
        self.logs.host(user_id, handle, Level::Error, message).await;
        false
    }

    /// Unloads the user's plugins, letting each flush with `shutdown` first.
    async fn unmount(&self, user_id: Snowflake) {
        if let Some((_, slot)) = self.mounted.remove(&user_id) {
            Mounted::shut_down(&mut *slot.lock().await).await;
        }
    }

    /// The compiled plugins `enabled` names, in the same order. Ones that
    /// fail to load are left out.
    async fn installed(&self, enabled: &[UserPlugin]) -> Vec<Arc<PluginPackage>> {
        let mut packages = Vec::new();
        for plugin in enabled {
            if let Ok(package) = self.pinned(plugin).await {
                packages.push(package);
            }
        }
        packages
    }

    async fn pinned(&self, plugin: &UserPlugin) -> Result<Arc<PluginPackage>, PluginError> {
        let version = plugin
            .version
            .parse()
            .map_err(|_| PluginError::NotFound(plugin.handle.clone()))?;
        self.package(&plugin.handle, &version).await
    }

    /// The compiled plugin for `handle`, compiling it on first use.
    async fn package(
        &self,
        handle: &str,
        version: &Version,
    ) -> Result<Arc<PluginPackage>, PluginError> {
        let key = format!("{handle}@{version}");
        if let Some(package) = self.packages.get(&key) {
            return Ok(Arc::clone(&package));
        }
        let dir = self.version_dir(handle, version);
        if !is_identifier(handle) || !dir.is_dir() {
            return Err(PluginError::NotFound(handle.to_owned()));
        }

        let host = Arc::clone(&self.host);
        let package = tokio::task::spawn_blocking(move || host.compile(&dir))
            .await
            .expect("plugin compilation should not panic")
            .map_err(|err| PluginError::Invalid(handle.to_owned(), err))?;
        let info = &package.manifest.plugin;
        if info.handle != handle || &info.version != version {
            return Err(PluginError::NotFound(handle.to_owned()));
        }

        let package = Arc::new(package);
        self.packages.insert(key, Arc::clone(&package));
        Ok(package)
    }
}

#[async_trait]
impl CleanableService for PluginService {
    fn name(&self) -> &'static str {
        "plugins"
    }

    /// Unmounts users whose plugins sat idle past [`IDLE_TTL`]. Slots in use
    /// right now are kept.
    async fn clean(&self) -> usize {
        let slots: Vec<(Snowflake, Slot)> = self
            .mounted
            .iter()
            .map(|e| (*e.key(), Arc::clone(e.value())))
            .collect();
        let mut cleaned = 0;
        for (user_id, slot) in slots {
            let Ok(mut guard) = slot.try_lock() else {
                continue;
            };
            if guard
                .as_ref()
                .is_some_and(|m| m.last_used.elapsed() < IDLE_TTL)
            {
                continue;
            }
            self.mounted.remove(&user_id);
            Mounted::shut_down(&mut guard).await;
            cleaned += 1;
        }
        cleaned
    }
}

/// `json` as JSON. If it isn't, logs that `handle`'s change to the `what` of
/// tool `name` was ignored.
async fn parse(
    logs: &PluginLogs,
    user_id: Snowflake,
    handle: &str,
    name: &str,
    what: &str,
    json: &str,
) -> Option<serde_json::Value> {
    match serde_json::from_str(json) {
        Ok(value) => Some(value),
        Err(err) => {
            let message = format!("Ignored its change to `{name}`'s {what}: {err}");
            logs.host(user_id, handle, Level::Warn, message).await;
            None
        }
    }
}

/// The manifest's slots the user may run. An unsafe slot needs both the
/// plugin's `ui:unsafe` permission and a user allowed unsafe plugins.
fn usable_slots(manifest: &Manifest, allow_unsafe: bool) -> impl Iterator<Item = &FrontendSlot> {
    let unsafe_ok = allow_unsafe && manifest.plugin.permissions.iter().any(|p| p == "ui:unsafe");
    manifest
        .frontend
        .slots
        .iter()
        .filter(move |s| !s.r#unsafe || unsafe_ok)
}

/// Host services bound to one user's instance of one plugin.
struct UserServices {
    user_id: Snowflake,
    handle: String,
    storage: Arc<dyn Storage>,
    state: Arc<StateService>,
    logs: Arc<PluginLogs>,
    /// The plugin's `settings.wtn`, if it has one.
    settings: Option<String>,
    /// The API key types the user's loaded plugins declare.
    key_types: Arc<Vec<String>>,
    connection: Arc<ConnectionService>,
    /// Answers `generate`, if the service was given one.
    generation: Option<Weak<dyn Generation>>,
    /// How long `generate` may wait for the model.
    generate_timeout: Duration,
    http: reqwest::Client,
    /// Sent with its requests unless they set their own.
    user_agent: String,
    frontend: broadcast::Sender<FrontendEvent>,
}

impl UserServices {
    /// Logs a storage failure and gives the plugin a generic error in its place.
    fn storage_error(&self, err: StorageError) -> String {
        tracing::error!(handle = %self.handle, user_id = %self.user_id, error = %err, "plugin data access failed");
        "storage error".to_owned()
    }
}

#[async_trait]
impl HostServices for UserServices {
    async fn active_persona(&self) -> Result<Persona, String> {
        let persona = self
            .state
            .get_active_persona(self.user_id)
            .await
            .map_err(|err| err.to_string())?;
        Ok(Persona {
            name: persona.name,
            description: persona.description.unwrap_or_default(),
        })
    }

    async fn persist_data(&self, key: String, value: Vec<u8>) -> Result<(), String> {
        self.storage
            .set_plugin_data(self.user_id, &self.handle, &key, &value)
            .await
            .map_err(|err| self.storage_error(err))
    }

    async fn load_data(&self, key: String) -> Result<Option<Vec<u8>>, String> {
        self.storage
            .get_plugin_data(self.user_id, &self.handle, &key)
            .await
            .map_err(|err| self.storage_error(err))
    }

    async fn delete_data(&self, key: String) -> Result<(), String> {
        self.storage
            .delete_plugin_data(self.user_id, &self.handle, &key)
            .await
            .map_err(|err| self.storage_error(err))
    }

    async fn settings(&self) -> Result<String, String> {
        Ok(self.resolved_settings().await?.to_string())
    }

    async fn http_fetch(&self, mut request: HttpRequest) -> Result<HttpResponse, String> {
        let names: Vec<String> = request
            .headers
            .iter()
            .flat_map(|(_, value)| fetch::placeholders(value))
            .collect();
        if !names.is_empty() {
            if !fetch::may_carry_keys(&request.url) {
                return Err("keys are only sent over https, or to this machine".into());
            }
            for name in names {
                let key = self.key(&name).await?;
                let placeholder = format!("{{secret:{name}}}");
                for (_, value) in &mut request.headers {
                    *value = value.replace(&placeholder, &key);
                }
            }
        }
        fetch::send(&self.http, request, &self.user_agent).await
    }

    /// Only the user's own conversations, and only the branch they are on.
    async fn messages(
        &self,
        conversation: i64,
        range: MessageRange,
    ) -> Result<Vec<ChatMessage>, String> {
        let id = Snowflake(conversation);
        let convo = self
            .storage
            .get_conversation(id)
            .await
            .map_err(|err| self.storage_error(err))?
            .filter(|c| c.user_id == self.user_id)
            .ok_or("no such conversation")?;
        let path = self
            .storage
            .get_active_path(convo.id)
            .await
            .map_err(|err| self.storage_error(err))?;
        let wanted = match range {
            MessageRange::All => path.len(),
            MessageRange::Latest(n) => (n as usize).min(path.len()),
        };
        Ok(path[path.len() - wanted..]
            .iter()
            .map(|m| ChatMessage {
                id: m.id.0,
                role: match m.role {
                    ChatRole::System => Role::System,
                    ChatRole::User => Role::User,
                    ChatRole::Assistant => Role::Assistant,
                },
                content: m.content.clone(),
            })
            .collect())
    }

    /// Sent to every client the user has open; none is not an error.
    async fn send_frontend(&self, name: String, payload: String) -> Result<(), String> {
        let payload = serde_json::from_str(&payload).map_err(|_| "the payload isn't valid JSON")?;
        let _ = self.frontend.send(FrontendEvent {
            user_id: self.user_id,
            handle: self.handle.clone(),
            name,
            payload,
        });
        Ok(())
    }

    /// Runs one completion through the user's profile. Nothing is saved.
    async fn generate(
        &self,
        messages: Vec<Message>,
        profile: Option<i64>,
        conversation: Option<i64>,
        blocking: bool,
    ) -> Result<String, String> {
        let chat = self
            .generation
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or("generating isn't available")?;
        let profile = match profile {
            Some(id) => Snowflake(id),
            None => {
                self.state
                    .get_active_profile(self.user_id)
                    .await
                    .map_err(|err| format!("no connection profile: {err}"))?
                    .id
            }
        };
        let prompt = messages.into_iter().map(Into::into).collect();
        // The guard holds the slot for as long as the completion runs.
        let slot = match conversation.filter(|_| blocking).map(Snowflake) {
            Some(id) => Some(
                chat.try_begin_cancel(id)
                    .ok_or("that conversation is already generating")?,
            ),
            None => None,
        };
        let cancel = match &slot {
            Some((token, _)) => token.clone(),
            None => CancellationToken::new(),
        };
        let waited = chat.complete(self.user_id, profile, prompt, cancel);
        match tokio::time::timeout(self.generate_timeout, waited).await {
            Ok(reply) => reply.map_err(|err| err.to_string()),
            Err(_) => Err(format!(
                "the model didn't answer within {}s",
                self.generate_timeout.as_secs()
            )),
        }
    }

    async fn log(&self, entries: Vec<LogEntry>) {
        self.logs.write(self.user_id, &self.handle, entries).await;
    }
}

impl UserServices {
    async fn key(&self, name: &str) -> Result<Zeroizing<String>, String> {
        let source = self.settings.as_deref().unwrap_or("#inputs");
        let is_key = settings::declarations(source)?.iter().any(|d| {
            d.name == name && matches!(&d.ty, InputType::Ref(kind) if self.key_types.contains(kind))
        });
        if !is_key {
            return Err(format!("`{name}` isn't an API key setting"));
        }
        let id: Snowflake = self.resolved_settings().await?[name]
            .as_str()
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| format!("`{name}` holds no key"))?;
        self.connection
            .reveal_secret(self.user_id, id)
            .await
            .map_err(|err| err.to_string())
    }

    async fn resolved_settings(&self) -> Result<serde_json::Value, String> {
        let saved = settings::saved(&*self.storage, self.user_id, &self.handle)
            .await
            .map_err(|err| self.storage_error(err))?;
        let keys = settings::keys(&*self.storage, self.user_id, self.key_types.iter().cloned())
            .await
            .map_err(|err| self.storage_error(err))?;
        let source = self.settings.as_deref().unwrap_or("#inputs");
        match settings::resolve(source, &saved, &keys) {
            Ok(values) => Ok(values),
            Err(reason) => {
                let message = format!(
                    "Couldn't read your settings: {reason}. Fix them under Settings → You → Plugins."
                );
                self.logs
                    .host(self.user_id, &self.handle, Level::Warn, message)
                    .await;
                Err(reason)
            }
        }
    }
}

/// Stores plugin log entries, keeping the newest `cap` per user per plugin.
struct PluginLogs {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    cap: u32,
}

impl PluginLogs {
    async fn host(&self, user_id: Snowflake, handle: &str, level: Level, message: String) {
        let entry = LogEntry {
            level,
            source: LogSource::Host,
            message,
        };
        self.write(user_id, handle, vec![entry]).await;
    }

    /// Writes a list of [`LogEntry`]s to tracing
    ///
    /// Host entries are mirrored to tracing at their own level,
    /// Guest logs are written to debug to avoid flooding stdout.
    async fn write(&self, user_id: Snowflake, handle: &str, entries: Vec<LogEntry>) {
        let created_at = Utc::now();
        let logs: Vec<PluginLog> = entries
            .into_iter()
            .map(|entry| {
                let message = &entry.message;
                match (entry.source, entry.level) {
                    (LogSource::Guest, _) => {
                        tracing::debug!(%handle, %user_id, "plugin: {message}")
                    }
                    (LogSource::Host, Level::Error) => {
                        tracing::error!(%handle, %user_id, "{message}")
                    }
                    (LogSource::Host, Level::Warn) => {
                        tracing::warn!(%handle, %user_id, "{message}")
                    }
                    (LogSource::Host, _) => tracing::info!(%handle, %user_id, "{message}"),
                }
                PluginLog {
                    id: self.snowflake.generate(),
                    user_id,
                    handle: handle.to_owned(),
                    level: match entry.level {
                        Level::Trace => PluginLogLevel::Trace,
                        Level::Debug => PluginLogLevel::Debug,
                        Level::Info => PluginLogLevel::Info,
                        Level::Warn => PluginLogLevel::Warn,
                        Level::Error => PluginLogLevel::Error,
                    },
                    source: match entry.source {
                        LogSource::Host => PluginLogSource::Host,
                        LogSource::Guest => PluginLogSource::Guest,
                    },
                    message: entry.message,
                    created_at,
                }
            })
            .collect();

        if let Err(err) = self
            .storage
            .append_plugin_logs(user_id, handle, &logs, self.cap)
            .await
        {
            tracing::error!(%handle, %user_id, error = %err, "failed to store plugin logs");
        }
    }
}

impl From<PromptMessage> for Message {
    fn from(m: PromptMessage) -> Self {
        let role = match m.role {
            PromptRole::System => Role::System,
            PromptRole::User => Role::User,
            PromptRole::Assistant => Role::Assistant,
        };
        Self {
            role,
            content: m.content,
        }
    }
}

impl From<Message> for PromptMessage {
    fn from(m: Message) -> Self {
        let role = match m.role {
            Role::System => PromptRole::System,
            Role::User => PromptRole::User,
            Role::Assistant => PromptRole::Assistant,
        };
        Self {
            role,
            content: m.content,
        }
    }
}
