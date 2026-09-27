use super::*;

#[test]
fn action_ids_round_trip() {
    let (workspace_id, tab_id) = (Uuid::new_v4(), Uuid::new_v4());
    let actions = [
        Action::EditWorkspace(workspace_id),
        Action::ReloadWorkspace(workspace_id),
        Action::ClearWorkspace(workspace_id),
        Action::SignIn(workspace_id),
        Action::SignOut(workspace_id),
        Action::RemoveWorkspace(workspace_id),
        Action::PinTab(workspace_id, tab_id, true),
        Action::PinTab(workspace_id, tab_id, false),
        Action::ReloadTab(workspace_id, tab_id),
        Action::ReloadTabHome(workspace_id, tab_id),
        Action::CloseTab(workspace_id, tab_id),
        Action::OpenApp(workspace_id, "spreed".into()),
        Action::OpenHome(workspace_id),
    ];
    for action in actions {
        assert_eq!(Action::parse(&action.id()), Some(action.clone()), "{}", action.id());
    }
    assert_eq!(Action::parse("quit"), None);
    assert_eq!(Action::parse("ws-rename|not-a-uuid"), None);
}
