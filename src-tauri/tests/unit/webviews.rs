use super::*;

#[test]
fn bridge_pattern_matches_only_its_origin() {
    let cases = [
        ("https://cloud.example.com/nc/", "https://cloud.example.com/index.php/apps/files/?dir=/#x", "http://cloud.example.com/"),
        ("http://localhost:8080/", "http://localhost:8080/x", "http://localhost:8081/x"),
        ("https://[::1]:8443/", "https://[::1]:8443/x", "https://[::2]:8443/x"),
        ("https://a+b.example/", "https://a+b.example/x", "https://aab.example/x"),
        ("https://a*b.example/", "https://a*b.example/x", "https://axxb.example/x"),
    ];
    for (base, same_origin, other_origin) in cases {
        let pattern: RemoteUrlPattern = bridge_pattern(&Url::parse(base).unwrap()).unwrap().parse().unwrap();
        assert!(pattern.test(&Url::parse(same_origin).unwrap()), "{base} should match {same_origin}");
        assert!(!pattern.test(&Url::parse(other_origin).unwrap()), "{base} should not match {other_origin}");
    }
}

#[test]
fn labels_round_trip_and_reject_others() {
    let (workspace_id, tab_id) = (Uuid::new_v4(), Uuid::new_v4());
    let tab_label = label(workspace_id, tab_id);
    assert!(is_content(&tab_label));
    assert!(tab_label.starts_with(&label_prefix(workspace_id)));
    assert_eq!(tab_of(&tab_label), Some(tab_id));
    assert!(!is_content("shell"));
    assert_eq!(tab_of("shell"), None);
    assert_eq!(tab_of(&format!("{}clear", label_prefix(workspace_id))), None);
}

#[cfg(target_os = "macos")]
#[test]
fn safari_user_agent_has_version_and_safari_tokens() {
    let user_agent = safari_user_agent().expect("Safari.app present on macOS CI/dev machines");
    assert!(user_agent.contains("Version/"));
    assert!(user_agent.ends_with("Safari/605.1.15"));
}
