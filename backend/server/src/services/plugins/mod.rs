use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use lorealm_plugins::{
    FrontendSlot, HostServices, Level, LoadedPlugin, LogEntry, LogSource, Manifest, Message,
    Persona, PluginHost, PluginInfo, PluginPackage, Role, Version, resolve,
};
use lorealm_storage::{
    Storage, StorageError,
    models::{PluginLog, PluginLogLevel, PluginLogSource, Snowflake, UserPlugin, is_identifier},
};
use serde::Serialize;
use tokio::sync::Mutex;

mod install;
mod settings;
mod vocabulary;

pub use install::InstallReport;
pub use vocabulary::{PLUGIN_SOURCE_PREFIX, PluginVocabulary};

use crate::{
    config::PluginConfig,
    core::CleanableService,
    errors::PluginError,
    services::{
        blueprint::InputSpec, chat::PromptMessage, context::PromptRole,
        snowflake::SnowflakeService, state::StateService,
    },
};

/// How long a single call into a plugin may run.
const CALL_TIMEOUT: Duration = Duration::from_secs(5);

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
}

/// What a plugin's settings ask for, and the user's values.
#[derive(Default, Serialize)]
pub struct PluginSettings {
    pub inputs: Vec<InputSpec>,
    pub values: serde_json::Map<String, serde_json::Value>,
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
    /// Runs `op` against `handle`'s plugin. A plugin whose call trapped (f.ex.
    /// timed out) can't be entered again, so it is unmounted until the user's
    /// plugins next load.
    async fn call<T>(
        &mut self,
        handle: &str,
        op: impl AsyncFnOnce(&mut LoadedPlugin) -> Result<T, lorealm_plugins::PluginError>,
    ) -> Result<T, String> {
        self.last_used = Instant::now();
        let Some(i) = self.plugins.iter().position(|(h, _)| h == handle) else {
            return Err(format!("plugin `{handle}` is not loaded"));
        };
        // On failure, the plugin crate logged why.
        let err = match op(&mut self.plugins[i].1).await {
            Ok(value) => return Ok(value),
            Err(err) => err,
        };
        if !matches!(err, lorealm_plugins::PluginError::Call(_)) {
            let message =
                "Plugin stopped after a crash. It will load again the next time your plugins load.";
            self.logs
                .host(self.user_id, handle, Level::Warn, message.into())
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
}

impl PluginService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        state: Arc<StateService>,
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
        }
    }

    /// Every installed plugin, at the version the user is on or else the
    /// latest. `allow_unsafe` is whether the user may run unsafe slots; others
    /// are left out of each plugin's `slots`.
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
        self.mounted.remove(&user_id);
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
        self.mounted.remove(&user_id);
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
        self.mounted.remove(&user_id);
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
        self.mounted.remove(&user_id);
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
    ) -> Result<serde_json::Value, PluginError> {
        let slot = self.mount(user_id).await;
        let mut guard = slot.lock().await;
        let mounted = guard
            .as_mut()
            .filter(|m| m.plugins.iter().any(|(h, _)| h == handle))
            .ok_or_else(|| PluginError::NotFound(handle.to_owned()))?;
        let args = args.to_string();
        let json = mounted
            .call(handle, async |p| p.call_frontend(name, args).await)
            .await
            .map_err(|e| PluginError::CallFailed(handle.to_owned(), e))?;
        serde_json::from_str(&json)
            .map_err(|_| PluginError::CallFailed(handle.to_owned(), "returned invalid JSON".into()))
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

    /// Runs the user's `prompt-build` hooks in load order. A plugin that fails
    /// is logged and skipped.
    pub async fn prompt_build(
        &self,
        user_id: Snowflake,
        prompt: Vec<PromptMessage>,
    ) -> Vec<PromptMessage> {
        let slot = self.mount(user_id).await;
        let mut guard = slot.lock().await;
        let Some(mounted) = guard.as_mut() else {
            return prompt;
        };
        let handles: Vec<String> = mounted.plugins.iter().map(|(h, _)| h.clone()).collect();
        if handles.is_empty() {
            return prompt;
        }

        let mut messages: Vec<Message> = prompt.into_iter().map(Into::into).collect();
        for handle in handles {
            let input = messages.clone();
            if let Ok(next) = mounted
                .call(&handle, async move |p| p.prompt_build(input).await)
                .await
            {
                messages = next;
            }
        }
        messages.into_iter().map(Into::into).collect()
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
            });
            // On failure, the plugin crate logged why.
            if let Ok(plugin) = self.host.instantiate(package, services).await {
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
        let before = self.mounted.len();
        self.mounted.retain(|_, slot| match slot.try_lock() {
            Ok(guard) => guard
                .as_ref()
                .is_some_and(|m| m.last_used.elapsed() < IDLE_TTL),
            Err(_) => true,
        });
        before - self.mounted.len()
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
        let saved = settings::saved(&*self.storage, self.user_id, &self.handle)
            .await
            .map_err(|err| self.storage_error(err))?;
        let keys = settings::keys(&*self.storage, self.user_id, self.key_types.iter().cloned())
            .await
            .map_err(|err| self.storage_error(err))?;
        let source = self.settings.as_deref().unwrap_or("#inputs");
        match settings::resolve(source, &saved, &keys) {
            Ok(values) => Ok(values.to_string()),
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

    async fn log(&self, entries: Vec<LogEntry>) {
        self.logs.write(self.user_id, &self.handle, entries).await;
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
