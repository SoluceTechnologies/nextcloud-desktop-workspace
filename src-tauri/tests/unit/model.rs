use super::*;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn new_workspace_has_one_selected_auth_tab_named_after_host() {
    let workspace = Workspace::new(url("https://cloud.soluce.com/"));
    assert_eq!(workspace.name, "cloud.soluce.com");
    assert!(!workspace.name_custom);
    assert_eq!(workspace.tabs.len(), 1);
    assert_eq!(workspace.tabs[0].app_id, AUTH);
    assert_eq!(workspace.tabs[0].url, url("https://cloud.soluce.com/"));
    assert_eq!(workspace.active_tab_id, Some(workspace.tabs[0].id));
    assert_eq!(workspace.active_tab().map(|tab| tab.id), Some(workspace.tabs[0].id));
}

#[test]
fn find_tab_returns_owner_workspace() {
    let mut state = AppState::default();
    state.workspaces.push(Workspace::new(url("https://a.com/")));
    state.workspaces.push(Workspace::new(url("https://b.com/")));
    let tab_id = state.workspaces[1].tabs[0].id;
    assert_eq!(state.find_tab(tab_id).map(|(workspace, _)| workspace.id), Some(state.workspaces[1].id));
    assert!(state.find_tab(Uuid::new_v4()).is_none());
}

#[test]
fn serializes_camel_case_and_round_trips() {
    let mut state = AppState::default();
    state.workspaces.push(Workspace::new(url("https://a.com/nc")));
    state.active_workspace_id = Some(state.workspaces[0].id);
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("\"activeWorkspaceId\""));
    assert!(json.contains("\"baseUrl\":\"https://a.com/nc\""));
    assert!(json.contains("\"appId\":\"AUTH\""));
    let restored: AppState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, state);
}

#[test]
fn appearance_defaults_to_system_and_serializes_lowercase() {
    let saved_before_themes: AppState =
        serde_json::from_str(r#"{"version":1,"workspaces":[],"activeWorkspaceId":null}"#).unwrap();
    assert_eq!(saved_before_themes.theme, Appearance::System);
    let dark = AppState { theme: Appearance::Dark, ..AppState::default() };
    assert!(serde_json::to_string(&dark).unwrap().contains(r#""theme":"dark""#));
}
