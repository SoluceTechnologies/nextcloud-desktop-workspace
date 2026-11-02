use crate::domain::model::{AppState, Appearance};
use crate::runtime::{engine, update};
use std::sync::Mutex;
use tauri::{AppHandle, State};

pub struct Notice(pub Mutex<Option<String>>);

#[tauri::command]
pub fn get_state(app: AppHandle) -> AppState {
    engine(&app).state.clone()
}

#[tauri::command]
pub fn take_notice(notice: State<'_, Notice>) -> Option<String> {
    notice.0.lock().ok()?.take()
}

#[tauri::command]
pub fn set_overlay(app: AppHandle, open: bool) {
    update(&app, |engine| engine.set_overlay(open));
}

#[tauri::command]
pub fn set_theme(app: AppHandle, theme: Appearance) {
    update(&app, |engine| engine.set_theme(theme));
}
