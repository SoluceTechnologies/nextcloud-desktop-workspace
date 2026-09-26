//! Routing rules (spec §5.3, §5.4). Pure functions.

use crate::model::{AppState, AUTH};
use crate::urls::{find_workspace, join, relative_path};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    InPlace,
    Activate { ws: Uuid, app_id: String, url: Url },
    External(Url),
    Deny,
}

/// Which Nextcloud app a URL belongs to (spec §5.3). Mirrored in `bridge.js` `appKey()`.
pub fn app_id(url: &Url, base: &Url) -> String {
    let Some(rel) = relative_path(url, base) else { return AUTH.into() };
    let mut segs: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    if segs.first() == Some(&"index.php") {
        segs.remove(0);
    }
    match segs.as_slice() {
        ["apps", id, ..] if is_auth_app(id) => AUTH.into(),
        ["apps", id, ..] => (*id).to_string(),
        ["settings", ..] => "settings".into(),
        ["s", token, ..] => format!("share:{token}"),
        _ => AUTH.into(),
    }
}

fn is_auth_app(id: &str) -> bool {
    id == "user_oidc" || id == "user_saml" || id.starts_with("twofactor_")
}

/// `on_navigation` (every frame on macOS, so no app routing here — spec §5.4).
pub fn classify_navigation(url: &Url) -> Route {
    match url.scheme() {
        "http" | "https" | "about" | "blob" | "data" => Route::InPlace,
        "mailto" | "tel" => Route::External(url.clone()),
        _ => Route::Deny,
    }
}

/// `on_new_window`: `window.open`, `target=_blank`, and links rewritten by the bridge click interceptor.
pub fn classify_new_window(state: &AppState, url: &Url) -> Route {
    match url.scheme() {
        "http" | "https" => match find_workspace(state, url).and_then(|id| state.ws(id)) {
            Some(w) => Route::Activate { ws: w.id, app_id: app_id(url, &w.base_url), url: url.clone() },
            None => Route::External(url.clone()),
        },
        "mailto" | "tel" => Route::External(url.clone()),
        _ => Route::Deny,
    }
}

/// Landing page of an app, used by "Reload at app home".
pub fn home_url(base: &Url, app_id: &str) -> Url {
    match app_id {
        AUTH => base.clone(),
        "settings" => join(base, "settings/user"),
        app => match app.strip_prefix("share:") {
            Some(token) => join(base, &format!("s/{token}")),
            None => join(base, &format!("apps/{app}/")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Workspace;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
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
        for (base, url, want) in cases {
            assert_eq!(app_id(&u(url), &u(base)), want, "{url}");
        }
    }

    #[test]
    fn navigation_only_filters_schemes() {
        assert_eq!(classify_navigation(&u("https://idp.example.com/auth")), Route::InPlace);
        assert_eq!(classify_navigation(&u("http://x.com/apps/files/")), Route::InPlace);
        assert_eq!(classify_navigation(&u("about:blank")), Route::InPlace);
        assert_eq!(classify_navigation(&u("blob:https://x.com/0b1c")), Route::InPlace);
        assert_eq!(classify_navigation(&u("mailto:a@b.c")), Route::External(u("mailto:a@b.c")));
        assert_eq!(classify_navigation(&u("file:///etc/passwd")), Route::Deny);
    }

    #[test]
    fn new_window_routes_known_workspaces_and_sends_rest_outside() {
        let mut s = AppState::default();
        s.workspaces.push(Workspace::new(u("https://a.com/")));
        s.workspaces.push(Workspace::new(u("https://b.com/nc")));
        let (a, b) = (s.workspaces[0].id, s.workspaces[1].id);
        let deck = u("https://b.com/nc/apps/deck/#/board/5");
        assert_eq!(classify_new_window(&s, &deck), Route::Activate { ws: b, app_id: "deck".into(), url: deck.clone() });
        let pdf = u("https://a.com/remote.php/dav/files/me/x.pdf");
        assert_eq!(classify_new_window(&s, &pdf), Route::Activate { ws: a, app_id: AUTH.into(), url: pdf.clone() });
        assert_eq!(classify_new_window(&s, &u("https://github.com/")), Route::External(u("https://github.com/")));
        assert_eq!(classify_new_window(&s, &u("tel:+331")), Route::External(u("tel:+331")));
        assert_eq!(classify_new_window(&s, &u("about:blank")), Route::Deny);
    }

    #[test]
    fn home_urls() {
        let base = u("https://x.com/nc");
        assert_eq!(home_url(&base, AUTH), base);
        assert_eq!(home_url(&base, "settings").as_str(), "https://x.com/nc/settings/user");
        assert_eq!(home_url(&base, "share:AbC").as_str(), "https://x.com/nc/s/AbC");
        assert_eq!(home_url(&base, "files").as_str(), "https://x.com/nc/apps/files/");
    }
}
