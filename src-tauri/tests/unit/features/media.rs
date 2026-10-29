use super::*;
use tauri::webview::{PermissionKind, PermissionResponse};
use url::Url;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn capture_is_allowed_only_for_the_workspace_itself() {
    let base = url("https://cloud.example.com/nc/");
    let talk = url("https://cloud.example.com/nc/index.php/call/abc");
    for kind in [PermissionKind::Microphone, PermissionKind::Camera, PermissionKind::DisplayCapture] {
        assert_eq!(decide(kind, Some(&talk), &base), PermissionResponse::Allow);
        assert_eq!(decide(kind, Some(&url("https://evil.example.net/")), &base), PermissionResponse::Deny);
        assert_eq!(decide(kind, Some(&url("https://cloud.example.com/other/")), &base), PermissionResponse::Deny);
        assert_eq!(decide(kind, None, &base), PermissionResponse::Deny);
    }
}

#[test]
fn other_permissions_keep_the_engine_default() {
    let base = url("https://cloud.example.com/");
    assert_eq!(decide(PermissionKind::Geolocation, Some(&base), &base), PermissionResponse::Default);
    assert_eq!(decide(PermissionKind::Notifications, Some(&base), &base), PermissionResponse::Default);
}
