use crate::domain::urls;
use crate::features::auth;
use crate::features::tabs::webviews;
use crate::runtime::{engine, run, update};
use tauri::AppHandle;
use uuid::Uuid;

#[tauri::command]
pub fn add_workspace(app: AppHandle, url: String) -> Result<(), String> {
    let base = urls::normalize(&url)?;
    if webviews::bridge_pattern(&base).is_err() {
        return Err("This server address is not supported".into());
    }
    let already_added = engine(&app).state.workspaces.iter().any(|workspace| workspace.base_url == base);
    let effects = engine(&app).add_workspace(&url)?;
    run(&app, effects);
    let active_workspace_id = engine(&app).state.active_workspace_id;
    if let (false, Some(workspace_id)) = (already_added, active_workspace_id) {
        auth::sign_in(&app, workspace_id);
    }
    Ok(())
}

#[tauri::command]
pub fn remove_workspace(app: AppHandle, workspace_id: Uuid) {
    auth::sign_out(&app, workspace_id, true);
    update(&app, |engine| engine.remove_workspace(workspace_id));
}

#[tauri::command]
pub fn rename_workspace(app: AppHandle, workspace_id: Uuid, name: String) {
    update(&app, |engine| engine.rename_workspace(workspace_id, &name));
}

#[tauri::command]
pub fn reorder_workspaces(app: AppHandle, ids: Vec<Uuid>) {
    update(&app, |engine| engine.reorder_workspaces(&ids));
}

#[tauri::command]
pub fn activate_workspace(app: AppHandle, workspace_id: Uuid) {
    update(&app, |engine| engine.activate_workspace(workspace_id));
}

#[tauri::command]
pub fn clear_browsing_data(app: AppHandle, workspace_id: Uuid) {
    auth::sign_out(&app, workspace_id, true);
    update(&app, |engine| engine.clear_browsing_data(workspace_id));
}

#[tauri::command]
pub fn set_workspace_icon(app: AppHandle, workspace_id: Uuid, icon: Option<String>) {
    update(&app, |engine| engine.set_icon(workspace_id, icon));
}
