//! `workspaces.json` persistence. Holds only the model: never credentials (they live in WebView profiles).

use crate::model::AppState;
use std::fmt::Display;
use std::fs;
use std::path::Path;

/// Missing file → empty state. Unreadable file → renamed to `*.json.bak`, empty state, notice for the user.
pub fn load(path: &Path) -> (AppState, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(state) => (state, None),
            Err(err) => backed_up(path, err),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (AppState::default(), None),
        Err(err) => backed_up(path, err),
    }
}

/// Renames file to `*.json.bak` and returns default state with user notice.
fn backed_up(path: &Path, err: impl Display) -> (AppState, Option<String>) {
    let _ = fs::rename(path, path.with_extension("json.bak"));
    (AppState::default(), Some(format!("Saved workspaces could not be read ({err}). A backup was kept.")))
}

/// Atomic write: temp file, then rename over the old one.
pub fn save(path: &Path, state: &AppState) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(state)?)?;
    fs::rename(tmp, path)
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
