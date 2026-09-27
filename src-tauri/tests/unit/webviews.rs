use super::*;

#[test]
fn bridge_pattern_matches_only_its_origin() {
    use tauri::utils::acl::RemoteUrlPattern;
    for (base, same, other) in [
        ("https://cloud.example.com/nc/", "https://cloud.example.com/index.php/apps/files/?dir=/#x", "http://cloud.example.com/"),
        ("http://localhost:8080/", "http://localhost:8080/x", "http://localhost:8081/x"),
        ("https://[::1]:8443/", "https://[::1]:8443/x", "https://[::2]:8443/x"),
        ("https://a+b.example/", "https://a+b.example/x", "https://aab.example/x"),
        ("https://a*b.example/", "https://a*b.example/x", "https://axxb.example/x"),
    ] {
        let p: RemoteUrlPattern = bridge_pattern(&Url::parse(base).unwrap()).unwrap().parse().unwrap();
        assert!(p.test(&Url::parse(same).unwrap()), "{base} should match {same}");
        assert!(!p.test(&Url::parse(other).unwrap()), "{base} should not match {other}");
    }
}

#[test]
fn labels_round_trip_and_reject_others() {
    let (ws, tab) = (Uuid::new_v4(), Uuid::new_v4());
    let l = label(ws, tab);
    assert!(l.starts_with("ws-"));
    assert_eq!(tab_of(&l), Some(tab));
    assert_eq!(tab_of("shell"), None);
    assert_eq!(tab_of(&format!("ws-{}-t-clear", ws.simple())), None);
}

#[cfg(target_os = "macos")]
#[test]
fn safari_user_agent_has_version_and_safari_tokens() {
    let ua = safari_user_agent().expect("Safari.app present on macOS CI/dev machines");
    assert!(ua.contains("Version/"));
    assert!(ua.ends_with("Safari/605.1.15"));
}
