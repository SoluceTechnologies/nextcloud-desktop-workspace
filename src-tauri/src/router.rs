use crate::model::{AppState, AUTH};
use crate::urls::{find_workspace, join, relative_path};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    InPlace,
    Activate { workspace_id: Uuid, app_id: String, url: Url },
    External(Url),
    Deny,
}

pub fn app_id(url: &Url, base: &Url) -> String {
    let Some(relative) = relative_path(url, base) else { return AUTH.into() };
    let mut segments: Vec<&str> = relative.split('/').filter(|segment| !segment.is_empty()).collect();
    if segments.first() == Some(&"index.php") {
        segments.remove(0);
    }
    match segments.as_slice() {
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

pub fn classify_navigation(url: &Url) -> Route {
    match url.scheme() {
        "http" | "https" | "about" | "blob" | "data" => Route::InPlace,
        "mailto" | "tel" => Route::External(url.clone()),
        _ => Route::Deny,
    }
}

pub fn classify_new_window(state: &AppState, url: &Url) -> Route {
    match url.scheme() {
        "http" | "https" => match find_workspace(state, url).and_then(|id| state.workspace(id)) {
            Some(workspace) => Route::Activate {
                workspace_id: workspace.id,
                app_id: app_id(url, &workspace.base_url),
                url: url.clone(),
            },
            None => Route::External(url.clone()),
        },
        "mailto" | "tel" => Route::External(url.clone()),
        _ => Route::Deny,
    }
}

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
