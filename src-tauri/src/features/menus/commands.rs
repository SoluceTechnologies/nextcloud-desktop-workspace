use crate::features::menus;
use tauri::{AppHandle, LogicalPosition};
use uuid::Uuid;

#[tauri::command]
pub fn workspace_menu(app: AppHandle, workspace_id: Uuid, x: f64, y: f64) -> Result<(), String> {
    menus::popup_workspace(&app, workspace_id, LogicalPosition::new(x, y)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn tab_menu(app: AppHandle, workspace_id: Uuid, tab_id: Uuid, x: f64, y: f64) -> Result<(), String> {
    menus::popup_tab(&app, workspace_id, tab_id, LogicalPosition::new(x, y)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn apps_menu(app: AppHandle, workspace_id: Uuid, x: f64, y: f64) -> Result<(), String> {
    menus::popup_apps(&app, workspace_id, LogicalPosition::new(x, y)).map_err(|error| error.to_string())
}
