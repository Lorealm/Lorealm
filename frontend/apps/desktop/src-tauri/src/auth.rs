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

#[derive(Deserialize)]
pub struct ServerError {
    pub error_code: String,
    pub message: String,
}

/// Parse a reqwest::Response, checking status before deserializing.
/// On non-2xx, attempts to read the body as a ServerError.
async fn parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> AppResult<T> {
    if !response.status().is_success() {
        let status = response.status();
        return Err(match response.json::<ServerError>().await {
            Ok(e) => Error::Server {
                error_code: e.error_code,
                message: e.message,
            },
            Err(_) => Error::Http(format!("Request failed with status {status}")),
        });
    }

    response
        .json::<T>()
        .await
        .map_err(|e| Error::Parse(e.to_string()))
}

#[tauri::command]
pub async fn login(handle: AppHandle, username: &str, password: &str) -> AppResult<LoginResponse> {
    let (base_url, api_client) = {
        let state = handle
            .try_state::<Mutex<AppState>>()
            .ok_or(Error::Internal("AppState not yet available".into()))?;
        let state = state.lock().await;
        (state.config.server_url.clone(), state.api_client.clone())
    };

    let request = api_client
        .post(format!("{base_url}/api/v1/users/login"))
        .json(&json!({"username": username, "password": password}))
        .build()
        .map_err(|e| Error::Http(e.to_string()))?;

    let response = api_client
        .execute(request)
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    let res: LoginResponse = parse_response(response).await?;

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

#[tauri::command]
pub async fn validate_token(handle: AppHandle) -> AppResult<MeResponse> {
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

    let session_token = session_token.ok_or_else(|| Error::Server {
        error_code: "UNAUTHORIZED".to_string(),
        message: "Missing bearer token".to_string(),
    })?;

    let response = api_client
        .get(format!("{base_url}/api/v1/users/me"))
        .bearer_auth(session_token)
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    parse_response(response).await
}

#[tauri::command]
pub async fn get_stored_token(handle: AppHandle) -> Option<String> {
    let state = handle.try_state::<Mutex<AppState>>()?;
    let state = state.lock().await;
    state.config.auth.session_token.clone()
}

#[tauri::command]
pub async fn clear_stored_token(handle: AppHandle) -> AppResult<()> {
    let state = handle
        .try_state::<Mutex<AppState>>()
        .ok_or(Error::Internal("AppState not yet available".into()))?;
    let mut state = state.lock().await;

    state.config.auth.session_token = None;
    state
        .config
        .save(&handle)
        .map_err(|e| Error::Config(e.to_string()))?;

    Ok(())
}
