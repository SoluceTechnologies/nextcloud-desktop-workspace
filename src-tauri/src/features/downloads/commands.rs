use super::registry::Download;
use std::path::{Component, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

#[tauri::command]
pub fn reveal_download(app: AppHandle, path: String) -> Result<(), String> {
    let downloads = app.path().download_dir().map_err(|error| error.to_string())?;
    let path = PathBuf::from(path);
    let escapes = path.components().any(|component| component == Component::ParentDir);
    if !path.starts_with(&downloads) || escapes {
        return Err("not a download".into());
    }
    app.opener().reveal_item_in_dir(path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_downloads() -> Vec<Download> {
    super::list()
}

#[tauri::command]
pub fn cancel_download(id: Uuid) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if super::linux::cancel(id) {
        return Ok(());
    }
    let _ = id;
    Err("This download can no longer be cancelled".into())
}
