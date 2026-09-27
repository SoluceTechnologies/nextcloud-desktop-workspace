use crate::model::AppState;
use std::fmt::Display;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

pub fn load(path: &Path) -> (AppState, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(state) => (state, None),
            Err(error) => back_up(path, error),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => (AppState::default(), None),
        Err(error) => back_up(path, error),
    }
}

fn back_up(path: &Path, error: impl Display) -> (AppState, Option<String>) {
    let _ = fs::rename(path, path.with_extension("json.bak"));
    let notice = format!("Saved workspaces could not be read ({error}). A backup was kept.");
    (AppState::default(), Some(notice))
}

pub fn save(path: &Path, state: &AppState) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(state)?)?;
    fs::rename(temporary, path)
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
