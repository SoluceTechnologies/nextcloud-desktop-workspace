//! `workspaces.json` persistence. Holds only the model: never credentials (they live in WebView profiles).

use crate::model::AppState;
use std::fs;
use std::path::Path;

/// Missing file → empty state. Unreadable file → renamed to `*.json.bak`, empty state, notice for the user.
pub fn load(path: &Path) -> (AppState, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(state) => (state, None),
            Err(err) => {
                let _ = fs::rename(path, path.with_extension("json.bak"));
                (AppState::default(), Some(format!("Saved workspaces could not be read ({err}). A backup was kept.")))
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (AppState::default(), None),
        Err(err) => {
            let _ = fs::rename(path, path.with_extension("json.bak"));
            (AppState::default(), Some(format!("Saved workspaces could not be read ({err}). A backup was kept.")))
        }
    }
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
mod tests {
    use super::*;
    use crate::model::Workspace;

    #[test]
    fn missing_file_gives_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("workspaces.json")), (AppState::default(), None));
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/workspaces.json");
        let mut s = AppState::default();
        s.workspaces.push(Workspace::new("https://a.com/".parse().unwrap()));
        save(&path, &s).unwrap();
        assert_eq!(load(&path), (s, None));
    }

    #[test]
    fn corrupt_file_is_backed_up_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        fs::write(&path, "{not json").unwrap();
        let (s, notice) = load(&path);
        assert_eq!(s, AppState::default());
        assert!(notice.is_some());
        assert!(dir.path().join("workspaces.json.bak").exists());
        assert!(!path.exists());
    }

    #[test]
    fn invalid_utf8_is_backed_up_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        fs::write(&path, &[0xff, 0xfe, 0x00]).unwrap();
        let (s, notice) = load(&path);
        assert_eq!(s, AppState::default());
        assert!(notice.is_some());
        assert!(dir.path().join("workspaces.json.bak").exists());
        assert!(!path.exists());
    }
}
