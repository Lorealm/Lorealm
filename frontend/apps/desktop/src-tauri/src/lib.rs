use serde::Serialize;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::{config::AppConfig, state::AppState};

mod auth;
mod config;
mod files;
mod http;
mod snowflake;
mod state;

type AppResult<T> = Result<T, Error>;

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "error_type", content = "data")]
pub enum Error {
    #[error("Parse Error: {0}")]
    Parse(String),
    #[error("Config Error: {0}")]
    Config(String),
    #[error("HTTP Error: {0}")]
    Http(String),
    #[error("Internal Client Error: {0}")]
    Internal(String),
    #[error("Server Error: {error_code}")]
    Server { error_code: String, message: String },
    #[error("HTTP {status}")]
    HttpStatus {
        status: u16,
        body: serde_json::Value,
    },
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            http::http_get,
            http::http_post,
            http::http_put,
            http::http_patch,
            http::http_delete,
            http::http_get_blob,
            http::http_put_blob,
            http::http_stream,
            auth::login,
            auth::validate_token,
            auth::get_stored_token,
            auth::clear_stored_token,
            config::store_get,
            config::store_set,
            config::store_delete,
            config::get_server_url,
            config::set_server_url,
            files::pick_file,
            files::save_file
        ])
        .setup(|app| {
            match AppConfig::load(app.handle()) {
                Ok(cfg) => app.manage(Mutex::new(AppState::new(cfg))),
                Err(e) => {
                    print!("An error occured during app state creation");
                    return Err(e.into());
                }
            };

            let win = app.get_webview_window("main").unwrap();
            #[cfg(not(target_os = "macos"))]
            win.set_decorations(false).unwrap();
            win.show().unwrap();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
