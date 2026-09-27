use super::*;
use crate::model::Workspace;

#[test]
fn missing_file_gives_empty_state() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(load(&directory.path().join("workspaces.json")), (AppState::default(), None));
}

#[test]
fn save_then_load_round_trips() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sub/workspaces.json");
    let mut state = AppState::default();
    state.workspaces.push(Workspace::new("https://a.com/".parse().unwrap()));
    save(&path, &state).unwrap();
    assert_eq!(load(&path), (state, None));
}

#[test]
fn corrupt_file_is_backed_up_and_reported() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspaces.json");
    fs::write(&path, "{not json").unwrap();
    let (state, notice) = load(&path);
    assert_eq!(state, AppState::default());
    assert!(notice.is_some());
    assert!(directory.path().join("workspaces.json.bak").exists());
    assert!(!path.exists());
}

#[test]
fn invalid_utf8_is_backed_up_and_reported() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspaces.json");
    fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
    let (state, notice) = load(&path);
    assert_eq!(state, AppState::default());
    assert!(notice.is_some());
    assert!(directory.path().join("workspaces.json.bak").exists());
    assert!(!path.exists());
}
