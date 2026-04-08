use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{snowflake::Snowflake, state::AppState, AppResult, Error};

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: Snowflake,
    pub csrf_token: String,
    pub session_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct MeResponse {
    pub username: String,
    pub id: Snowflake,
}

#[tauri::command]
pub async fn login(handle: AppHandle, username: &str, password: &str) -> AppResult<LoginResponse> {
    println!("login");
    // Extract only what we need, then release the lock
    // Else the AppState may be unavaiable during the duration of the network request
    let (base_url, api_client) = {
        let state = handle
            .try_state::<Mutex<AppState>>()
            .ok_or(Error::Internal("AppState not yet available".into()))?;
        let state = state.lock().await;
        (state.config.server_url.clone(), state.api_client.clone())
    };

    println!(
        "[LOGIN] URL: {:?}",
        format!("{base_url}/api/v1/users/login")
    );

    // Network request happens without holding the lock
    let request = api_client
        .post(format!("{base_url}/api/v1/users/login"))
        .json(&json!({"username": username, "password": password}))
        .build()
        .map_err(|e| Error::Parse(e.to_string()))?;

    println!("[LOGIN] Headers: {:?}", request.headers());

    let response = api_client
        .execute(request)
        .await
        .map_err(|e| Error::Parse(e.to_string()))?;

    match response.json::<LoginResponse>().await {
        Ok(res) => {
            // Re-acquire lock only to update the token
            let state = handle.state::<Mutex<AppState>>();
            let mut state = state.lock().await;
            state.config.auth.session_token = Some(res.session_token.clone());
            state.config.auth.csrf_token = Some(res.csrf_token.clone());
            state
                .config
                .save(&handle)
                .map_err(|e| Error::Config(e.to_string()))?;
            Ok(res)
        }
        Err(e) => {
            println!("{:?}", e);
            Err(Error::Parse(e.to_string()))
        }
    }
}

#[tauri::command]
pub async fn validate_token(handle: AppHandle) -> AppResult<MeResponse> {
    println!("validate_token");
    // Extract only what we need, then release the lock
    // Else the AppState may be unavaiable during the duration of the network request
    let (base_url, api_client, session_token) = {
        let state = handle
            .try_state::<Mutex<AppState>>()
            .ok_or(Error::Internal("AppState not yet available".into()))?;
        let state = state.lock().await;
        (
            state.config.server_url.clone(),
            state.api_client.clone(),
            state.config.auth.session_token.clone(),
        )
    };

    println!("[LOGIN] URL: {:?}", format!("{base_url}/api/v1/users/me"));

    if session_token.is_none() {
        return Err(Error::Auth("Missing bearer token".to_string()));
    }

    // Network request happens without holding the lock
    let response = api_client
        .get(format!("{base_url}/api/v1/users/me"))
        .bearer_auth(session_token.unwrap())
        .send()
        .await
        .map_err(|e| Error::Parse(e.to_string()))?;

    match response.json::<MeResponse>().await {
        Ok(res) => Ok(res),
        Err(e) => Err(Error::Parse(e.to_string())),
    }
}

#[tauri::command]
pub async fn get_stored_token(handle: AppHandle) -> Option<String> {
    println!("get_stored_token");
    let maybe_state = handle
        .try_state::<Mutex<AppState>>()
        .ok_or(Error::Internal("AppState not yet available".into()));

    if let Ok(state) = maybe_state {
        let state = state.lock().await;

        return state.config.auth.session_token.clone();
    }

    return None;
}

#[tauri::command]
pub async fn clear_stored_token(handle: AppHandle) -> AppResult<()> {
    println!("clear_stored_token");
    let maybe_state = handle
        .try_state::<Mutex<AppState>>()
        .ok_or(Error::Internal("AppState not yet available".into()));

    if let Ok(state) = maybe_state {
        let mut state = state.lock().await;

        state.config.auth.session_token = None;
        state
            .config
            .save(&handle)
            .map_err(|e| Error::Config(e.to_string()))?;
    }

    Ok(())
}
