use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use lorealm_plugins::{
    HostServices, Level, LoadedPlugin, LogEntry, LogSource, Message, Persona, PluginHost,
    PluginInfo, PluginPackage, Role,
};
use lorealm_storage::{
    Storage, StorageError,
    models::{PluginLog, PluginLogLevel, PluginLogSource, Snowflake, is_identifier},
};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{
    config::PluginConfig,
    core::CleanableService,
    errors::PluginError,
    services::{
        chat::PromptMessage, context::PromptRole, snowflake::SnowflakeService, state::StateService,
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
    pub enabled: bool,
}

/// A user's mounted plugins, in load order.
struct Mounted {
    plugins: Vec<(String, LoadedPlugin)>,
    last_used: Instant,
}

pub struct PluginService {
    storage: Arc<dyn Storage>,
    state: Arc<StateService>,
    host: Arc<PluginHost>,
    logs: Arc<PluginLogs>,
    /// `{data_directory}/plugins`, one directory per plugin, named by handle.
    dir: PathBuf,
    /// Compiled components by handle. Shared by every user.
    packages: DashMap<String, Arc<PluginPackage>>,
    /// `None` until the user's plugins are first needed.
    mounted: DashMap<Snowflake, Arc<Mutex<Option<Mounted>>>>,
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

    /// Every plugin in the plugins directory, flagged by whether `user_id` enabled it.
    pub async fn list(&self, user_id: Snowflake) -> Result<Vec<InstalledPlugin>, PluginError> {
        let enabled = self.storage.list_user_plugins(user_id).await?;

        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Ok(Vec::new());
        };
        let mut installed = Vec::new();
        for entry in entries.flatten() {
            let handle = entry.file_name().to_string_lossy().into_owned();
            match self.package(&handle).await {
                Ok(package) => installed.push(InstalledPlugin {
                    enabled: enabled.contains(&handle),
                    info: package.manifest.plugin.clone(),
                }),
                Err(err) => tracing::warn!(%handle, error = %err, "skipping plugin"),
            }
        }
        installed.sort_by(|a, b| a.info.handle.cmp(&b.info.handle));
        Ok(installed)
    }

    pub async fn enable(&self, user_id: Snowflake, handle: &str) -> Result<(), PluginError> {
        self.package(handle).await?;
        self.storage.enable_user_plugin(user_id, handle).await?;
        self.mounted.remove(&user_id);
        Ok(())
    }

    pub async fn disable(&self, user_id: Snowflake, handle: &str) -> Result<(), PluginError> {
        self.storage.disable_user_plugin(user_id, handle).await?;
        self.mounted.remove(&user_id);
        Ok(())
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
        mounted.last_used = Instant::now();
        if mounted.plugins.is_empty() {
            return prompt;
        }

        let mut messages: Vec<Message> = prompt.into_iter().map(Into::into).collect();
        // A plugin whose call trapped (f.ex. timed out) can't be entered
        // again, so it is unmounted until the user's plugins next mount.
        let mut trapped = Vec::new();
        for (handle, plugin) in &mut mounted.plugins {
            match plugin.prompt_build(messages.clone()).await {
                Ok(next) => messages = next,
                // The plugin crate logged why.
                Err(lorealm_plugins::PluginError::Hook(_)) => {}
                Err(_) => {
                    let message = "Plugin stopped after a crash. It will load again the next time your plugins load.";
                    self.logs
                        .host(user_id, handle, Level::Warn, message.into())
                        .await;
                    trapped.push(handle.clone());
                }
            }
        }
        mounted
            .plugins
            .retain(|(handle, _)| !trapped.contains(handle));
        messages.into_iter().map(Into::into).collect()
    }

    /// The user's mount slot, filled on first use. A plugin that fails to
    /// load is logged and left out.
    async fn mount(&self, user_id: Snowflake) -> Arc<Mutex<Option<Mounted>>> {
        let slot = Arc::clone(self.mounted.entry(user_id).or_default().value());
        let mut guard = slot.lock().await;
        if guard.is_some() {
            drop(guard);
            return slot;
        }

        let handles = match self.storage.list_user_plugins(user_id).await {
            Ok(handles) => handles,
            Err(err) => {
                tracing::error!(%user_id, error = %err, "failed to list plugins");
                Vec::new()
            }
        };

        let mut plugins = Vec::new();
        for handle in handles {
            let services = Arc::new(UserServices {
                user_id,
                handle: handle.clone(),
                storage: Arc::clone(&self.storage),
                state: Arc::clone(&self.state),
                logs: Arc::clone(&self.logs),
            });
            let package = match self.package(&handle).await {
                Ok(package) => package,
                Err(err) => {
                    self.logs
                        .host(user_id, &handle, Level::Error, err.to_string())
                        .await;
                    continue;
                }
            };
            // On failure, the plugin crate logged why.
            if let Ok(plugin) = self.host.instantiate(&package, services).await {
                plugins.push((handle, plugin));
            }
        }

        *guard = Some(Mounted {
            plugins,
            last_used: Instant::now(),
        });
        drop(guard);
        slot
    }

    /// The compiled plugin for `handle`, compiling it on first use.
    async fn package(&self, handle: &str) -> Result<Arc<PluginPackage>, PluginError> {
        if let Some(package) = self.packages.get(handle) {
            return Ok(Arc::clone(&package));
        }
        let dir = self.dir.join(handle);
        if !is_identifier(handle) || !dir.is_dir() {
            return Err(PluginError::NotFound(handle.to_owned()));
        }

        let host = Arc::clone(&self.host);
        let package = tokio::task::spawn_blocking(move || host.compile(&dir))
            .await
            .expect("plugin compilation should not panic")
            .map_err(|err| PluginError::Invalid(handle.to_owned(), err))?;
        if package.manifest.plugin.handle != handle {
            return Err(PluginError::NotFound(handle.to_owned()));
        }

        let package = Arc::new(package);
        self.packages
            .insert(handle.to_owned(), Arc::clone(&package));
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

/// Host services bound to one user's instance of one plugin.
struct UserServices {
    user_id: Snowflake,
    handle: String,
    storage: Arc<dyn Storage>,
    state: Arc<StateService>,
    logs: Arc<PluginLogs>,
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
