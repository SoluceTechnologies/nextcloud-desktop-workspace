use super::*;

fn u(s: &str) -> Url {
    Url::parse(s).unwrap()
}

#[test]
fn new_workspace_has_one_selected_auth_tab_named_after_host() {
    let w = Workspace::new(u("https://cloud.soluce.com/"));
    assert_eq!(w.name, "cloud.soluce.com");
    assert!(!w.name_custom);
    assert_eq!(w.tabs.len(), 1);
    assert_eq!(w.tabs[0].app_id, AUTH);
    assert_eq!(w.tabs[0].url, u("https://cloud.soluce.com/"));
    assert_eq!(w.active_tab_id, Some(w.tabs[0].id));
}

#[test]
fn find_tab_returns_owner_workspace() {
    let mut s = AppState::default();
    s.workspaces.push(Workspace::new(u("https://a.com/")));
    s.workspaces.push(Workspace::new(u("https://b.com/")));
    let tab = s.workspaces[1].tabs[0].id;
    assert_eq!(s.find_tab(tab).map(|(w, _)| w.id), Some(s.workspaces[1].id));
    assert!(s.find_tab(Uuid::new_v4()).is_none());
}

#[test]
fn serializes_camel_case_and_round_trips() {
    let mut s = AppState::default();
    s.workspaces.push(Workspace::new(u("https://a.com/nc")));
    s.active_workspace_id = Some(s.workspaces[0].id);
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"activeWorkspaceId\""));
    assert!(json.contains("\"baseUrl\":\"https://a.com/nc\""));
    assert!(json.contains("\"appId\":\"AUTH\""));
    let back: AppState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
}

#[test]
fn appearance_defaults_to_system_and_serializes_lowercase() {
    let old: AppState = serde_json::from_str(r#"{"version":1,"workspaces":[],"activeWorkspaceId":null}"#).unwrap();
    assert_eq!(old.theme, Appearance::System);
    let s = AppState { theme: Appearance::Dark, ..AppState::default() };
    assert!(serde_json::to_string(&s).unwrap().contains(r#""theme":"dark""#));
}
