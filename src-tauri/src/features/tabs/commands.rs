use crate::domain::engine::AppLink;
use crate::features::tabs::{offline, webviews};
use crate::runtime::update;
use crate::window::{self, Fullscreen};
use tauri::{AppHandle, Manager, Webview};
use url::Url;
use uuid::Uuid;

#[tauri::command]
pub fn activate_tab(app: AppHandle, workspace_id: Uuid, tab_id: Uuid) {
    update(&app, |engine| engine.activate_tab(workspace_id, tab_id));
}

#[tauri::command]
pub fn close_tab(app: AppHandle, workspace_id: Uuid, tab_id: Uuid) {
    update(&app, |engine| engine.close_tab(workspace_id, tab_id));
}

#[tauri::command]
pub fn reorder_tabs(app: AppHandle, workspace_id: Uuid, ids: Vec<Uuid>) {
    update(&app, |engine| engine.reorder_tabs(workspace_id, &ids));
}

#[tauri::command]
pub fn retry_tab(app: AppHandle, tab_id: Uuid) {
    offline::retry(&app, tab_id);
}

#[tauri::command]
pub fn nc_report_location(app: AppHandle, webview: Webview, url: String) {
    let (Some(tab_id), Ok(url)) = (webviews::tab_of(webview.label()), Url::parse(&url)) else { return };
    offline::page_reported(tab_id);
    update(&app, |engine| engine.observe_location(tab_id, url));
}

#[tauri::command]
pub fn nc_report_meta(app: AppHandle, webview: Webview, icon: Option<String>, apps: Vec<AppLink>) {
    let Some(tab_id) = webviews::tab_of(webview.label()) else { return };
    update(&app, |engine| engine.observe_meta(tab_id, icon, apps));
}

#[tauri::command]
pub fn nc_report_fullscreen(app: AppHandle, webview: Webview, fullscreen: bool) {
    let label = fullscreen.then(|| webview.label().to_string());
    *app.state::<Fullscreen>().0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = label;
    let main_window = webview.window();
    let _ = main_window.set_fullscreen(fullscreen);
    window::relayout(&main_window);
}
