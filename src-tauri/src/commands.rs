//! Commands. Shell commands are granted to the `shell` webview only (capabilities/default.json);
//! `nc_*` bridge commands are granted per workspace origin at runtime (webviews::grant_bridge).

use crate::engine::{Effect, Engine};
use crate::model::AppState;
use crate::webviews::{engine, run};
use std::sync::Mutex;
use tauri::{AppHandle, State};
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
