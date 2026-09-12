use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::{AppResult, Error};

async fn picked(
    rx: tokio::sync::oneshot::Receiver<Option<tauri_plugin_dialog::FilePath>>,
) -> AppResult<Option<std::path::PathBuf>> {
    let Some(path) = rx
        .await
        .map_err(|e| Error::Internal(format!("The file dialog closed unexpectedly: {e}")))?
    else {
        return Ok(None); // Cancelled
    };

    path.into_path()
        .map(Some)
        .map_err(|e| Error::Internal(format!("That file has no path we can read: {e}")))
}

/// Ask for a file and read it. `None` when the user picked nothing.
#[tauri::command]
pub async fn pick_file(
    app: AppHandle,
    label: String,
    extensions: Vec<String>,
) -> AppResult<Option<String>> {
    let exts: Vec<&str> = extensions.iter().map(String::as_str).collect();
    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .add_filter(&label, &exts)
        .pick_file(move |path| {
            let _ = tx.send(path);
        });

    let Some(path) = picked(rx).await? else {
        return Ok(None);
    };

    tokio::fs::read_to_string(&path)
        .await
        .map(Some)
        .map_err(|e| Error::Internal(format!("Could not read {}: {e}", path.display())))
}

/// The media type for an extension we offered in the dialog.
fn media_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("avif") => "image/avif",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Ask for a file and read it as a `data:` URI. `None` when nothing was picked.
#[tauri::command]
pub async fn pick_file_binary(
    app: AppHandle,
    label: String,
    extensions: Vec<String>,
) -> AppResult<Option<String>> {
    let exts: Vec<&str> = extensions.iter().map(String::as_str).collect();
    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .add_filter(&label, &exts)
        .pick_file(move |path| {
            let _ = tx.send(path);
        });

    let Some(path) = picked(rx).await? else {
        return Ok(None);
    };

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| Error::Internal(format!("Could not read {}: {e}", path.display())))?;

    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:{};base64,{encoded}", media_type(&path))))
}

/// Ask where to put `contents` and write it. `false` when the user cancelled.
#[tauri::command]
pub async fn save_file(
    app: AppHandle,
    contents: String,
    label: String,
    extensions: Vec<String>,
    suggested_name: Option<String>,
) -> AppResult<bool> {
    let exts: Vec<&str> = extensions.iter().map(String::as_str).collect();
    let (tx, rx) = tokio::sync::oneshot::channel();

    let mut dialog = app.dialog().file().add_filter(&label, &exts);
    if let Some(name) = suggested_name {
        dialog = dialog.set_file_name(name);
    }
    dialog.save_file(move |path| {
        let _ = tx.send(path);
    });

    let Some(path) = picked(rx).await? else {
        return Ok(false);
    };

    tokio::fs::write(&path, contents)
        .await
        .map_err(|e| Error::Internal(format!("Could not write {}: {e}", path.display())))?;

    Ok(true)
}
