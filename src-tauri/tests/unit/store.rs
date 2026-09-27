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
