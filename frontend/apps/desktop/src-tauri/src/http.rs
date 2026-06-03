use std::sync::Arc;

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{state::AppState, AppResult, Error};

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEvent {
    Message { data: serde_json::Value },
    Error { message: String },
    Done,
}

#[tauri::command]
pub async fn http_get(handle: AppHandle, path: String) -> AppResult<HttpResponse> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;

    let mut req = client.get(format!("{base_url}{path}"));
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }

    send_request(&handle, req).await
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

    send_request(&handle, req).await
}

#[tauri::command]
pub async fn http_put(
    handle: AppHandle,
    path: String,
    body: serde_json::Value,
) -> AppResult<HttpResponse> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;
    let mut req = client.put(format!("{base_url}{path}")).json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    // Attach CSRF if you have it stored
    // req = req.header("x-csrf-token", csrf);

    send_request(&handle, req).await
}

#[tauri::command]
pub async fn http_delete(handle: AppHandle, path: String) -> AppResult<HttpResponse> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;
    let mut req = client.delete(format!("{base_url}{path}"));
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    // Attach CSRF if you have it stored
    // req = req.header("x-csrf-token", csrf);

    send_request(&handle, req).await
}

#[tauri::command]
pub async fn http_get_blob(handle: AppHandle, path: String) -> AppResult<tauri::ipc::Response> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;

    let mut req = client.get(format!("{base_url}{path}"));
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }

    send_blob_request(&handle, req).await
}

#[tauri::command]
pub async fn http_stream(
    handle: AppHandle,
    path: String,
    body: serde_json::Value,
    on_event: Channel<StreamEvent>,
) -> AppResult<()> {
    let (base_url, client, token) = extract_request_deps(&handle).await?;

    let mut req = client.post(format!("{base_url}{path}")).json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }

    let response = req.send().await.map_err(|e| Error::Http(e.to_string()))?;
    let status = response.status().as_u16();

    handle_token_rotation(&handle, &response).await?;

    // Pre-flight error (the handler's `?` path): small JSON body, route through
    // Err so the frontend rebuilds ApiError just like getBlob does.
    if status >= 400 {
        let text = response.text().await.unwrap_or_default();
        let body = serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or(serde_json::Value::String(text));
        return Err(Error::HttpStatus { status, body });
    }

    let mut stream = response.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        match event {
            Ok(ev) => {
                let data = serde_json::from_str::<serde_json::Value>(&ev.data)
                    .unwrap_or(serde_json::Value::String(ev.data));

                // Your axum handler tags failures with `.event("error")`.
                if ev.event == "error" {
                    let message = data
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("stream error")
                        .to_string();
                    let _ = on_event.send(StreamEvent::Error { message });
                    return Ok(());
                }

                let _ = on_event.send(StreamEvent::Message { data });
            }
            Err(e) => {
                // Transport-level break mid-stream (connection dropped, etc.)
                let _ = on_event.send(StreamEvent::Error {
                    message: e.to_string(),
                });
                return Ok(());
            }
        }
    }

    let _ = on_event.send(StreamEvent::Done);
    Ok(())
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

async fn send_request(handle: &AppHandle, req: reqwest::RequestBuilder) -> AppResult<HttpResponse> {
    let response = req.send().await.map_err(|e| Error::Http(e.to_string()))?;
    let status = response.status().as_u16();

    handle_token_rotation(handle, &response).await?;

    let text = response.text().await.unwrap_or_default();
    let body =
        serde_json::from_str::<serde_json::Value>(&text).unwrap_or(serde_json::Value::String(text));

    /*     if status >= 400 {
        return Err(Error::Http(format!("HTTP {status}: {body}")));
    } */

    Ok(HttpResponse { status, body })
}

async fn send_blob_request(
    handle: &AppHandle,
    req: reqwest::RequestBuilder,
) -> AppResult<tauri::ipc::Response> {
    let response = req.send().await.map_err(|e| Error::Http(e.to_string()))?;
    let status = response.status().as_u16();

    handle_token_rotation(handle, &response).await?;

    // Errors are small JSON — route them through Err so getBlob can rebuild ApiError.
    if status >= 400 {
        let text = response.text().await.unwrap_or_default();
        let body = serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or(serde_json::Value::String(text));
        return Err(Error::HttpStatus { status, body });
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    Ok(tauri::ipc::Response::new(bytes.to_vec()))
}

async fn handle_token_rotation(handle: &AppHandle, response: &reqwest::Response) -> AppResult<()> {
    if let Some(new_token) = response
        .headers()
        .get("X-New-Session-Token")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(state) = handle.try_state::<Mutex<AppState>>() {
            let mut state = state.lock().await;
            state.config.auth.session_token = Some(new_token.to_string());
            state
                .config
                .save(handle)
                .map_err(|e| Error::Config(e.to_string()))?;
        }
    }
    Ok(())
}
