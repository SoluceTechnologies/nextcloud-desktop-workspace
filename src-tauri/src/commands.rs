use crate::engine::{AppLink, Effect, Engine};
use crate::model::{AppState, Appearance};
use crate::runtime::{engine, run};
use crate::window::{self, Fullscreen};
use crate::{auth, menus, offline, urls, webviews};
use std::path::{Component, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, LogicalPosition, Manager, State, Webview};
use tauri_plugin_opener::OpenerExt;
use url::Url;
use uuid::Uuid;

pub struct Notice(pub Mutex<Option<String>>);

fn apply(app: &AppHandle, operation: impl FnOnce(&mut Engine) -> Vec<Effect>) {
    let effects = operation(&mut engine(app));
    run(app, effects);
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> AppState {
    engine(&app).state.clone()
}

#[tauri::command]
pub fn take_notice(notice: State<'_, Notice>) -> Option<String> {
    notice.0.lock().ok()?.take()
}

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
    apply(&app, |engine| engine.remove_workspace(workspace_id));
}

#[tauri::command]
pub fn rename_workspace(app: AppHandle, workspace_id: Uuid, name: String) {
    apply(&app, |engine| engine.rename_workspace(workspace_id, &name));
}

#[tauri::command]
pub fn reorder_workspaces(app: AppHandle, ids: Vec<Uuid>) {
    apply(&app, |engine| engine.reorder_workspaces(&ids));
}

#[tauri::command]
pub fn activate_workspace(app: AppHandle, workspace_id: Uuid) {
    apply(&app, |engine| engine.activate_workspace(workspace_id));
}

#[tauri::command]
pub fn activate_tab(app: AppHandle, workspace_id: Uuid, tab_id: Uuid) {
    apply(&app, |engine| engine.activate_tab(workspace_id, tab_id));
}

#[tauri::command]
pub fn close_tab(app: AppHandle, workspace_id: Uuid, tab_id: Uuid) {
    apply(&app, |engine| engine.close_tab(workspace_id, tab_id));
}

#[tauri::command]
pub fn reorder_tabs(app: AppHandle, workspace_id: Uuid, ids: Vec<Uuid>) {
    apply(&app, |engine| engine.reorder_tabs(workspace_id, &ids));
}

#[tauri::command]
pub fn retry_tab(app: AppHandle, tab_id: Uuid) {
    offline::retry(&app, tab_id);
}

#[tauri::command]
pub fn set_overlay(app: AppHandle, open: bool) {
    apply(&app, |engine| engine.set_overlay(open));
}

#[tauri::command]
pub fn clear_browsing_data(app: AppHandle, workspace_id: Uuid) {
    auth::sign_out(&app, workspace_id, true);
    apply(&app, |engine| engine.clear_browsing_data(workspace_id));
}

#[tauri::command]
pub fn set_workspace_icon(app: AppHandle, workspace_id: Uuid, icon: Option<String>) {
    apply(&app, |engine| engine.set_icon(workspace_id, icon));
}

#[tauri::command]
pub fn set_theme(app: AppHandle, theme: Appearance) {
    apply(&app, |engine| engine.set_theme(theme));
}

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
pub fn nc_report_location(app: AppHandle, webview: Webview, url: String) {
    let (Some(tab_id), Ok(url)) = (webviews::tab_of(webview.label()), Url::parse(&url)) else { return };
    offline::page_reported(tab_id);
    apply(&app, |engine| engine.observe_location(tab_id, url));
}

#[tauri::command]
pub fn nc_report_meta(app: AppHandle, webview: Webview, icon: Option<String>, apps: Vec<AppLink>) {
    let Some(tab_id) = webviews::tab_of(webview.label()) else { return };
    apply(&app, |engine| engine.observe_meta(tab_id, icon, apps));
}

#[tauri::command]
pub fn nc_report_fullscreen(app: AppHandle, webview: Webview, fullscreen: bool) {
    let label = fullscreen.then(|| webview.label().to_string());
    *app.state::<Fullscreen>().0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = label;
    let main_window = webview.window();
    let _ = main_window.set_fullscreen(fullscreen);
    window::relayout(&main_window);
}
