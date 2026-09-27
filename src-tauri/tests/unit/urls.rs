use super::*;
use crate::model::Workspace;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn normalize_accepts_and_cleans_user_input() {
    let normalized = |input: &str| normalize(input).unwrap().to_string();
    assert_eq!(normalized("cloud.example.com"), "https://cloud.example.com/");
    assert_eq!(normalized("  https://Cloud.Example.com/  "), "https://cloud.example.com/");
    assert_eq!(normalized("https://x.com/nextcloud/index.php/apps/files/?dir=/#x"), "https://x.com/nextcloud");
    assert_eq!(normalized("https://x.com/login?redirect_url=/apps/files"), "https://x.com/");
    assert_eq!(normalized("x.com:8443/nc/"), "https://x.com:8443/nc");
    assert_eq!(normalized("https://user:pw@x.com"), "https://x.com/");
    assert_eq!(normalized("http://intranet.local"), "http://intranet.local/");
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
    let root = url("https://x.com/");
    let subdirectory = url("https://x.com/nc");
    assert_eq!(relative_path(&url("https://x.com/apps/files/"), &root), Some("apps/files/"));
    assert_eq!(relative_path(&url("https://x.com/nc"), &subdirectory), Some(""));
    assert_eq!(relative_path(&url("https://x.com/nc/apps/deck"), &subdirectory), Some("apps/deck"));
    assert_eq!(relative_path(&url("https://x.com/ncx/apps"), &subdirectory), None);
    assert_eq!(relative_path(&url("https://y.com/apps/files/"), &root), None);
    assert_eq!(relative_path(&url("http://x.com/apps/files/"), &root), None);
    assert!(!belongs(&url("about:blank"), &root));
}

#[test]
fn find_workspace_prefers_longest_base_path() {
    let mut state = AppState::default();
    state.workspaces.push(Workspace::new(url("https://x.com/")));
    state.workspaces.push(Workspace::new(url("https://x.com/nc")));
    let (root_id, subdirectory_id) = (state.workspaces[0].id, state.workspaces[1].id);
    assert_eq!(find_workspace(&state, &url("https://x.com/nc/apps/files/")), Some(subdirectory_id));
    assert_eq!(find_workspace(&state, &url("https://x.com/apps/files/")), Some(root_id));
    assert_eq!(find_workspace(&state, &url("https://other.com/")), None);
}

#[test]
fn join_appends_under_base_path() {
    assert_eq!(join(&url("https://x.com/nc"), "apps/files/").as_str(), "https://x.com/nc/apps/files/");
    assert_eq!(join(&url("https://x.com/"), "settings/user").as_str(), "https://x.com/settings/user");
}
