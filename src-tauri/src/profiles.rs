use crate::model::AppState;
use crate::runtime::EffectResult;
use crate::{webviews, window};
use std::collections::HashSet;
use std::error::Error;
use tauri::webview::WebviewBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, WebviewUrl, Wry};
use uuid::Uuid;

#[cfg(not(target_os = "macos"))]
use std::path::PathBuf;
#[cfg(not(target_os = "macos"))]
use tauri::Manager;

pub fn with_profile(
    app: &AppHandle,
    builder: WebviewBuilder<Wry>,
    workspace_id: Uuid,
) -> Result<WebviewBuilder<Wry>, Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        Ok(builder.data_store_identifier(*workspace_id.as_bytes()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let directory = profile_directory(app, workspace_id)?;
        std::fs::create_dir_all(&directory)?;
        Ok(builder.data_directory(directory))
    }
}

pub fn sweep(app: &AppHandle, state: &AppState) {
    let kept: HashSet<Uuid> = state.workspaces.iter().map(|workspace| workspace.id).collect();
    #[cfg(target_os = "macos")]
    sweep_data_stores(app, &kept);
    #[cfg(not(target_os = "macos"))]
    sweep_directories(app, &kept);
}

pub fn clear(app: &AppHandle, workspace_id: Uuid, delete: bool) -> EffectResult {
    #[cfg(target_os = "macos")]
    if delete {
        return Ok(tauri::async_runtime::block_on(app.remove_data_store(*workspace_id.as_bytes()))?);
    }
    clear_browsing_data(app, workspace_id)?;
    #[cfg(not(target_os = "macos"))]
    if delete {
        if let Ok(directory) = profile_directory(app, workspace_id) {
            let _ = std::fs::remove_dir_all(directory);
        }
    }
    Ok(())
}

fn clear_browsing_data(app: &AppHandle, workspace_id: Uuid) -> EffectResult {
    let prefix = webviews::label_prefix(workspace_id);
    let window = window::main_window(app)?;
    let existing = window.webviews().into_iter().find(|webview| webview.label().starts_with(&prefix));
    if let Some(webview) = existing {
        webview.clear_all_browsing_data()?;
        return Ok(());
    }
    let blank = WebviewBuilder::new(format!("{prefix}clear"), WebviewUrl::External("about:blank".parse()?));
    let temporary = window.add_child(
        with_profile(app, blank, workspace_id)?,
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(1.0, 1.0),
    )?;
    temporary.hide()?;
    temporary.clear_all_browsing_data()?;
    temporary.close()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn sweep_data_stores(app: &AppHandle, kept: &HashSet<Uuid>) {
    let identifiers = match tauri::async_runtime::block_on(app.fetch_data_store_identifiers()) {
        Ok(identifiers) => identifiers,
        Err(error) => {
            eprintln!("[ncw] fetch_data_store_identifiers failed: {error}");
            return;
        }
    };
    for identifier in identifiers {
        if kept.contains(&Uuid::from_bytes(identifier)) {
            continue;
        }
        if let Err(error) = tauri::async_runtime::block_on(app.remove_data_store(identifier)) {
            eprintln!("[ncw] remove_data_store failed: {error}");
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn sweep_directories(app: &AppHandle, kept: &HashSet<Uuid>) {
    let Ok(root) = profiles_root(app) else { return };
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let workspace_id = Uuid::parse_str(&entry.file_name().to_string_lossy()).ok();
        if !workspace_id.is_some_and(|workspace_id| kept.contains(&workspace_id)) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn profiles_root(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join("profiles"))
}

#[cfg(not(target_os = "macos"))]
fn profile_directory(app: &AppHandle, workspace_id: Uuid) -> tauri::Result<PathBuf> {
    Ok(profiles_root(app)?.join(workspace_id.simple().to_string()))
}
