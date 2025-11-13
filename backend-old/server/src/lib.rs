// // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// use std::sync::Arc;

// use futures::StreamExt;
// use llm_providers::{llm_provider::LLMProvider, openrouter::OpenRouterProvider, types::Message};
// use plugin_system::plugin_manager::{PluginManager, PluginPermissionRequest};
// use tauri::{AppHandle, Manager, Emitter};
// use tokio::sync::Mutex;

// pub mod file;
// pub mod llm;
// mod plugin_system;
// mod state;

// lazy_static::lazy_static! {
//     static ref LOGGING_EXCLUSIONS: Vec<&'static str> = vec!["cranelift_codegen::*", "wasmtime::*"];
// }

// async fn initialize_plugin_manager(plugin_manager: Arc<Mutex<Option<PluginManager>>>) {
//     async move {
//         let mut guard = plugin_manager.lock().await;
//         if let Some(plugin_manager) = guard.as_mut() {
//             plugin_manager
//                 .initialize()
//                 .await
//                 .expect("Failed to initialize plugin manager");
//         }
//     }.await;
// }

// #[cfg_attr(mobile, tauri::mobile_entry_point)]
// pub async fn run() {
//     let plugin_manager_arc = Arc::new(Mutex::new(Some(PluginManager::new())));
//     tauri::Builder::default()
//         .manage(state::AppState {
//             plugin_manager: plugin_manager_arc.clone(),
//         })
//         .plugin(tauri_plugin_opener::init())
//         .plugin(
//             tauri_plugin_log::Builder::new()
//                 .filter(|metadata| !LOGGING_EXCLUSIONS.contains(&metadata.target()))
//                 .level(log::LevelFilter::Info)
//                 .build(),
//         )
//         .setup(move |_app| {
//             log::info!("Running setup");
            

//             tauri::async_runtime::spawn({
//                 let plugin_manager_arc = plugin_manager_arc.clone();
//                 initialize_plugin_manager(plugin_manager_arc)
//             });

//             Ok(())
//         })
//         .invoke_handler(tauri::generate_handler![get_permission_requests, submit_permission_grant, send_request, send_request_stream, get_plugin_components])
//         .run(tauri::generate_context!())
//         .expect("error while running tauri application");

//     println!("debug");
// }

// #[tauri::command]
// async fn get_permission_requests(app: AppHandle) -> Result<Vec<PluginPermissionRequest>, String> {
//     let state = app.state::<state::AppState>();
//     let plugin_manager = state.plugin_manager.lock().await;
//     let plugin_manager = plugin_manager.as_ref().ok_or("No plugin manager")?;
//     let permission_requests = plugin_manager.prepare_permission_requests();

//     Ok(permission_requests)
// }

// #[tauri::command]
// async fn submit_permission_grant(app: AppHandle, permission_grant: PluginPermissionRequest) -> Result<(), String> {
//     println!("submit_permission_grant: {:?}", permission_grant.plugin_id);

//     let state = app.state::<state::AppState>();
//     let plugin_manager = state.plugin_manager.lock().await;
//     let plugin_manager = plugin_manager.as_ref().ok_or("No plugin manager")?;
//     plugin_manager.submit_permission_grants(permission_grant.plugin_id, permission_grant.to_permissions_list());

//     Ok(())
// }

// #[tauri::command]
// async fn get_plugin_components(app: AppHandle) -> Vec<String> {
//     let state = app.state::<state::AppState>();
//     let plugin_manager = state.plugin_manager.lock().await;
//     let plugin_manager = plugin_manager.as_ref().unwrap();
//     plugin_manager.get_custom_components()
// }

// #[tauri::command]
// async fn send_request(_app: AppHandle, messages: Vec<Message>) -> Result<String, String> {
//     let provider = OpenRouterProvider::new();
//     match provider.generate(messages).await {
//         Ok(res) => Ok(res.text()),
//         Err(err) => Err(err.to_string()),
//     }
// }

// #[tauri::command]
// async fn send_request_stream(
//     app: AppHandle,
//     messages: Vec<Message>,
//     callback: String,
// ) -> Result<(), String> {
//     let provider = Arc::new(OpenRouterProvider::new());
//     let message_clone = messages.clone();
//     let stream_result = provider.generate_stream(message_clone).await;
//     match stream_result {
//         Ok(stream) => {
//             tauri::async_runtime::spawn(async move {
//                 let mut stream = stream;
//                 while let Some(result) = stream.next().await {
//                     match result {
//                         Ok(response) => {
//                             let text = response.text();
//                             if !text.is_empty() || response.json().get("done").is_some() {
//                                 let _ = app.emit(&callback, StreamChunk {
//                                     text,
//                                     done: response.json().get("done").is_some(),
//                                 });
//                             }
//                             if response.json().get("done").is_some() {
//                                 break;
//                             }
//                         }
//                         Err(e) => {
//                             let _ = app.emit(&callback, StreamChunk {
//                                 text: format!("Error: {}", e),
//                                 done: true,
//                             });
//                             break;
//                         }
//                     }
//                 }
//                 let _ = app.emit(&callback, StreamChunk {
//                     text: String::new(),
//                     done: true,
//                 });
//             });
            
//             Ok(())
//         }
//         Err(e) => Err(e.to_string()),
//     }
// }

// #[derive(serde::Serialize, Clone)]
// struct StreamChunk {
//     text: String,
//     done: bool,
// }