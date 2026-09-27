use super::*;
use crate::model::Workspace;

fn u(s: &str) -> Url {
    Url::parse(s).unwrap()
}

#[test]
fn normalize_accepts_and_cleans_user_input() {
    let ok = |input: &str| normalize(input).unwrap().to_string();
    assert_eq!(ok("cloud.example.com"), "https://cloud.example.com/");
    assert_eq!(ok("  https://Cloud.Example.com/  "), "https://cloud.example.com/");
    assert_eq!(ok("https://x.com/nextcloud/index.php/apps/files/?dir=/#x"), "https://x.com/nextcloud");
    assert_eq!(ok("https://x.com/login?redirect_url=/apps/files"), "https://x.com/");
    assert_eq!(ok("x.com:8443/nc/"), "https://x.com:8443/nc");
    assert_eq!(ok("https://user:pw@x.com"), "https://x.com/");
    assert_eq!(ok("http://intranet.local"), "http://intranet.local/");
}

#[test]
fn normalize_rejects_bad_input() {
    assert!(normalize("").is_err());
    assert!(normalize("   ").is_err());
    assert!(normalize("ftp://x.com").is_err());
    assert!(normalize("https://").is_err());
}

#[test]
fn relative_path_respects_origin_and_segment_boundary() {
    let root = u("https://x.com/");
    let sub = u("https://x.com/nc");
    assert_eq!(relative_path(&u("https://x.com/apps/files/"), &root), Some("apps/files/"));
    assert_eq!(relative_path(&u("https://x.com/nc"), &sub), Some(""));
    assert_eq!(relative_path(&u("https://x.com/nc/apps/deck"), &sub), Some("apps/deck"));
    assert_eq!(relative_path(&u("https://x.com/ncx/apps"), &sub), None);
    assert_eq!(relative_path(&u("https://y.com/apps/files/"), &root), None);
    assert_eq!(relative_path(&u("http://x.com/apps/files/"), &root), None);
    assert!(!belongs(&u("about:blank"), &root));
}

#[test]
fn find_workspace_prefers_longest_base_path() {
    let mut s = AppState::default();
    s.workspaces.push(Workspace::new(u("https://x.com/")));
    s.workspaces.push(Workspace::new(u("https://x.com/nc")));
    let (root, sub) = (s.workspaces[0].id, s.workspaces[1].id);
    assert_eq!(find_workspace(&s, &u("https://x.com/nc/apps/files/")), Some(sub));
    assert_eq!(find_workspace(&s, &u("https://x.com/apps/files/")), Some(root));
    assert_eq!(find_workspace(&s, &u("https://other.com/")), None);
}

#[test]
fn join_appends_under_base_path() {
    assert_eq!(join(&u("https://x.com/nc"), "apps/files/").as_str(), "https://x.com/nc/apps/files/");
    assert_eq!(join(&u("https://x.com/"), "settings/user").as_str(), "https://x.com/settings/user");
}
