use super::*;

#[test]
fn action_ids_round_trip() {
    let (w, t) = (Uuid::new_v4(), Uuid::new_v4());
    let all = [
        Action::WsRename(w),
        Action::WsReload(w),
        Action::WsClear(w),
        Action::WsRemove(w),
        Action::TabPin(w, t, true),
        Action::TabPin(w, t, false),
        Action::TabReload(w, t),
        Action::TabHome(w, t),
        Action::TabClose(w, t),
        Action::OpenApp(w, "spreed".into()),
        Action::OpenHome(w),
    ];
    for a in all {
        assert_eq!(Action::parse(&a.id()), Some(a.clone()), "{}", a.id());
    }
    assert_eq!(Action::parse("quit"), None);
    assert_eq!(Action::parse("ws-rename|not-a-uuid"), None);
}
