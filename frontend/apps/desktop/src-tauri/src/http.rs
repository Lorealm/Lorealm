use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{state::AppState, AppResult, Error};

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

#[tauri::command]
pub async fn http_get(handle: AppHandle, path: String) -> AppResult<HttpResponse> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;

    println!("Sending request to: {}", format!("{base_url}{path}"));
    let mut req = client.get(format!("{base_url}{path}"));
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }

    send_request(req).await
}

#[tauri::command]
pub async fn http_post(
    handle: AppHandle,
    path: String,
    body: serde_json::Value,
) -> AppResult<HttpResponse> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;

    let mut req = client.post(format!("{base_url}{path}")).json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    // Attach CSRF if you have it stored
    // req = req.header("x-csrf-token", csrf);

    send_request(req).await
}

async fn extract_request_deps(
    handle: &AppHandle,
) -> AppResult<(String, Arc<reqwest::Client>, Option<String>)> {
    let state = handle
        .try_state::<Mutex<AppState>>()
        .ok_or(Error::Internal("AppState not yet available".into()));
    match state {
        Ok(state) => {
            let state = state.lock().await;
            Ok((
                state.config.server_url.clone(),
                state.api_client.clone(),
                state.config.auth.session_token.clone(),
            ))
        }
        Err(e) => Err(e),
    }
}

async fn send_request(req: reqwest::RequestBuilder) -> AppResult<HttpResponse> {
    let response = req.send().await.map_err(|e| Error::Http(e.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::Value::Null);

    if status >= 400 {
        return Err(Error::Http(format!("HTTP {status}: {body}")));
    }

    Ok(HttpResponse { status, body })
}
