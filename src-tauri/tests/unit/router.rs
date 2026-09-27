use super::*;
use crate::model::Workspace;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn app_id_table() {
    let cases = [
        ("https://x.com/", "https://x.com/apps/files/", "files"),
        ("https://x.com/", "https://x.com/index.php/apps/calendar/dayGridMonth/now", "calendar"),
        ("https://x.com/nc", "https://x.com/nc/index.php/apps/deck/#/board/5", "deck"),
        ("https://x.com/", "https://x.com/settings/user", "settings"),
        ("https://x.com/", "https://x.com/index.php/s/AbC123", "share:AbC123"),
        ("https://x.com/", "https://x.com/", AUTH),
        ("https://x.com/", "https://x.com/login?redirect_url=/apps/files", AUTH),
        ("https://x.com/", "https://x.com/login/challenge/totp", AUTH),
        ("https://x.com/", "https://x.com/apps/user_oidc/login/1", AUTH),
        ("https://x.com/", "https://x.com/apps/user_saml/saml/login", AUTH),
        ("https://x.com/", "https://x.com/apps/twofactor_totp/settings", AUTH),
        ("https://x.com/", "https://x.com/remote.php/dav/files/a/b.pdf", AUTH),
        ("https://x.com/", "https://x.com/index.php/f/42", AUTH),
        ("https://x.com/", "https://other.com/apps/files/", AUTH),
    ];
    for (base, page, expected) in cases {
        assert_eq!(app_id(&url(page), &url(base)), expected, "{page}");
    }
}

#[test]
fn navigation_only_filters_schemes() {
    assert_eq!(classify_navigation(&url("https://idp.example.com/auth")), Route::InPlace);
    assert_eq!(classify_navigation(&url("http://x.com/apps/files/")), Route::InPlace);
    assert_eq!(classify_navigation(&url("about:blank")), Route::InPlace);
    assert_eq!(classify_navigation(&url("blob:https://x.com/0b1c")), Route::InPlace);
    assert_eq!(classify_navigation(&url("mailto:a@b.c")), Route::External(url("mailto:a@b.c")));
    assert_eq!(classify_navigation(&url("file:///etc/passwd")), Route::Deny);
}

#[test]
fn new_window_routes_known_workspaces_and_sends_rest_outside() {
    let mut state = AppState::default();
    state.workspaces.push(Workspace::new(url("https://a.com/")));
    state.workspaces.push(Workspace::new(url("https://b.com/nc")));
    let (first_id, second_id) = (state.workspaces[0].id, state.workspaces[1].id);
    let deck = url("https://b.com/nc/apps/deck/#/board/5");
    assert_eq!(classify_new_window(&state, &deck), Route::Activate { workspace_id: second_id, app_id: "deck".into(), url: deck.clone() });
    let pdf = url("https://a.com/remote.php/dav/files/me/x.pdf");
    assert_eq!(classify_new_window(&state, &pdf), Route::Activate { workspace_id: first_id, app_id: AUTH.into(), url: pdf.clone() });
    assert_eq!(classify_new_window(&state, &url("https://github.com/")), Route::External(url("https://github.com/")));
    assert_eq!(classify_new_window(&state, &url("tel:+331")), Route::External(url("tel:+331")));
    assert_eq!(classify_new_window(&state, &url("about:blank")), Route::Deny);
}

#[test]
fn home_urls() {
    let base = url("https://x.com/nc");
    assert_eq!(home_url(&base, AUTH), base);
    assert_eq!(home_url(&base, "settings").as_str(), "https://x.com/nc/settings/user");
    assert_eq!(home_url(&base, "share:AbC").as_str(), "https://x.com/nc/s/AbC");
    assert_eq!(home_url(&base, "files").as_str(), "https://x.com/nc/apps/files/");
}
