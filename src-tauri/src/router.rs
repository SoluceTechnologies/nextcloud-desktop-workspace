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
#[path = "../tests/unit/router.rs"]
mod tests;
