//! Commands. Shell commands are granted to the `shell` webview only (capabilities/default.json);
//! `nc_*` bridge commands are granted per workspace origin at runtime (webviews::grant_bridge).

use crate::engine::{AppLink, Effect, Engine};
use crate::menus;
use crate::model::AppState;
use crate::webviews::{self, engine, run, tab_of, Fullscreen};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, Webview};
use tauri_plugin_opener::OpenerExt;
use url::Url;
use uuid::Uuid;

/// One-shot startup message for the shell (e.g. unreadable workspaces.json).
pub struct Notice(pub Mutex<Option<String>>);

fn apply(app: &AppHandle, op: impl FnOnce(&mut Engine) -> Vec<Effect>) {
    let fx = op(&mut engine(app));
    run(app, fx);
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
    // Refuse up front an address whose bridge capability can't be built: `webviews::grant_bridge` only
    // runs on the first `Create`, where the failure would leave a workspace whose tabs never load.
    let base = crate::urls::normalize(&url)?;
    if crate::webviews::bridge_pattern(&base).is_err() {
        return Err("This server address is not supported".into());
    }
    let fx = engine(&app).add_workspace(&url)?;
    run(&app, fx);
    Ok(())
}

#[tauri::command]
pub fn remove_workspace(app: AppHandle, ws: Uuid) {
    apply(&app, |e| e.remove_workspace(ws));
}

#[tauri::command]
pub fn rename_workspace(app: AppHandle, ws: Uuid, name: String) {
    apply(&app, |e| e.rename_workspace(ws, &name));
}

#[tauri::command]
pub fn reorder_workspaces(app: AppHandle, ids: Vec<Uuid>) {
    apply(&app, |e| e.reorder_workspaces(&ids));
}

#[tauri::command]
pub fn activate_workspace(app: AppHandle, ws: Uuid) {
    apply(&app, |e| e.activate_workspace(ws));
}

#[tauri::command]
pub fn activate_tab(app: AppHandle, ws: Uuid, tab: Uuid) {
    apply(&app, |e| e.activate_tab(ws, tab));
}

#[tauri::command]
pub fn close_tab(app: AppHandle, ws: Uuid, tab: Uuid) {
    apply(&app, |e| e.close_tab(ws, tab));
}

#[tauri::command]
pub fn reorder_tabs(app: AppHandle, ws: Uuid, ids: Vec<Uuid>) {
    apply(&app, |e| e.reorder_tabs(ws, &ids));
}

#[tauri::command]
pub fn set_overlay(app: AppHandle, on: bool) {
    apply(&app, |e| e.set_overlay(on));
}

#[tauri::command]
pub fn clear_browsing_data(app: AppHandle, ws: Uuid) {
    apply(&app, |e| e.clear_browsing_data(ws));
}

/// Bridge: main-frame location of a Nextcloud page. The engine checks the URL belongs to the tab's workspace.
#[tauri::command]
pub fn nc_report_location(app: AppHandle, webview: Webview, url: String) {
    let (Some(tab), Ok(url)) = (tab_of(webview.label()), Url::parse(&url)) else { return };
    apply(&app, |e| e.observe_location(tab, url));
}

/// Bridge: icon + app menu of a Nextcloud page (untrusted, filtered by the engine).
#[tauri::command]
pub fn nc_report_meta(app: AppHandle, webview: Webview, icon: Option<String>, apps: Vec<AppLink>) {
    let Some(tab) = tab_of(webview.label()) else { return };
    apply(&app, |e| e.observe_meta(tab, icon, apps));
}

#[tauri::command]
pub fn workspace_menu(app: AppHandle, ws: Uuid) -> Result<(), String> {
    menus::popup_workspace(&app, ws).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tab_menu(app: AppHandle, ws: Uuid, tab: Uuid) -> Result<(), String> {
    menus::popup_tab(&app, ws, tab).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apps_menu(app: AppHandle, ws: Uuid) -> Result<(), String> {
    menus::popup_apps(&app, ws).map_err(|e| e.to_string())
}

/// "Show" on a download notice. Only paths inside the Downloads folder.
#[tauri::command]
pub fn reveal_download(app: AppHandle, path: String) -> Result<(), String> {
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let path = std::path::PathBuf::from(path);
    // `starts_with` compares components, so `Downloads/../x` would pass without the `..` check.
    if !path.starts_with(&dir) || path.components().any(|c| c == std::path::Component::ParentDir) {
        return Err("not a download".into());
    }
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

/// Bridge: the page entered/left HTML fullscreen → window fullscreen + webview covers the window.
#[tauri::command]
pub fn nc_report_fullscreen(app: AppHandle, webview: Webview, on: bool) {
    *app.state::<Fullscreen>().0.lock().unwrap_or_else(|e| e.into_inner()) = on.then(|| webview.label().to_string());
    let window = webview.window();
    let _ = window.set_fullscreen(on);
    webviews::relayout(&window);
}
