use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{state::AppState, AppResult, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub session_token: Option<String>,
    pub csrf_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub auth: AuthState,
    pub server_url: String,
    /// Whatever the frontend asked us to remember. Opaque here.
    pub store: HashMap<String, Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auth: AuthState {
                session_token: None,
                csrf_token: None,
            },
            server_url: "http://127.0.0.1:1551".to_string(),
            store: HashMap::new(),
        }
    }
}

const CONFIG_FILENAME: &str = "config.json";

impl AppConfig {
    /// Resolve the config file path using Tauri's app config dir.
    ///
    /// Typically resolves to:
    ///   - Linux:   ~/.config/<bundle-id>/config.json
    ///   - macOS:   ~/Library/Application Support/<bundle-id>/config.json
    ///   - Windows: C:\Users\<User>\AppData\Roaming\<bundle-id>/config.json
    fn path(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Failed to resolve config directory: {e}"))?;
        Ok(dir.join(CONFIG_FILENAME))
    }

    /// Load the config from disk. Returns the default config if the file
    /// doesn't exist yet
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let path = Self::path(app)?;

        if !path.exists() {
            let config = Self::default();
            config.write_to(&path)?;
            return Ok(config);
        }

        let data =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read config file: {e}"))?;

        let mut config: Self =
            serde_json::from_str(&data).map_err(|e| format!("Failed to parse config file: {e}"))?;
        config.fix_self();
        Ok(config)
    }

    fn fix_self(&mut self) {
        let new_url = self.server_url.trim_end_matches("/").to_string();
        self.server_url = new_url
    }

    /// Save the current config to disk.
    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let path = Self::path(app)?;
        self.write_to(&path)
    }

    fn write_to(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        println!("Saving config...");
        fs::write(path, json).map_err(|e| format!("Failed to write config file: {e}"))
    }
}

async fn read<T>(handle: &AppHandle, view: impl FnOnce(&AppConfig) -> T) -> AppResult<T> {
    let state = handle
        .try_state::<Mutex<AppState>>()
        .ok_or(Error::Internal("AppState not yet available".into()))?;
    let state = state.lock().await;
    Ok(view(&state.config))
}

async fn update(handle: &AppHandle, edit: impl FnOnce(&mut AppConfig)) -> AppResult<()> {
    let state = handle
        .try_state::<Mutex<AppState>>()
        .ok_or(Error::Internal("AppState not yet available".into()))?;
    let mut state = state.lock().await;
    edit(&mut state.config);
    state.config.save(handle).map_err(Error::Config)
}

#[tauri::command]
pub async fn store_get(handle: AppHandle, key: String) -> AppResult<Option<Value>> {
    read(&handle, |c| c.store.get(&key).cloned()).await
}

#[tauri::command]
pub async fn store_set(handle: AppHandle, key: String, value: Value) -> AppResult<()> {
    update(&handle, |c| {
        c.store.insert(key, value);
    })
    .await
}

#[tauri::command]
pub async fn store_delete(handle: AppHandle, key: String) -> AppResult<()> {
    update(&handle, |c| {
        c.store.remove(&key);
    })
    .await
}

#[tauri::command]
pub async fn get_server_url(handle: AppHandle) -> AppResult<String> {
    read(&handle, |c| c.server_url.clone()).await
}

#[tauri::command]
pub async fn set_server_url(handle: AppHandle, url: String) -> AppResult<()> {
    update(&handle, |c| {
        c.server_url = url.trim().trim_end_matches('/').to_string();
    })
    .await
}
