//! URL rules (spec §5.1, §5.2). Named `urls` to avoid clashing with the `url` crate.

use crate::model::AppState;
use url::Url;
use uuid::Uuid;

/// Normalizes an "Add Nextcloud" input into a workspace base URL (spec §5.1).
pub fn normalize(input: &str) -> Result<Url, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("Enter a server URL".into());
    }
    let full = if s.contains("://") { s.to_string() } else { format!("https://{s}") };
    let mut url = Url::parse(&full).map_err(|e| format!("Invalid URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http:// and https:// addresses are supported".into());
    }
    if url.host_str().map_or(true, str::is_empty) {
        return Err("The address has no host".into());
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    let keep: Vec<String> = url
        .path_segments()
        .map(|segs| {
            segs.filter(|s| !s.is_empty())
                .take_while(|s| !matches!(*s, "index.php" | "login" | "apps"))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    url.set_path(&format!("/{}", keep.join("/")));
    Ok(url)
}

/// Path of `url` below `base` (no leading slash), if `url` is inside the workspace (spec §5.2).
pub fn relative_path<'a>(url: &'a Url, base: &Url) -> Option<&'a str> {
    if url.origin() != base.origin() {
        return None;
    }
    let rest = url.path().strip_prefix(base.path().trim_end_matches('/'))?;
    if rest.is_empty() { Some("") } else { rest.strip_prefix('/') }
}

pub fn belongs(url: &Url, base: &Url) -> bool {
    relative_path(url, base).is_some()
}

/// Workspace containing `url`; the longest base path wins (several instances on one host).
pub fn find_workspace(state: &AppState, url: &Url) -> Option<Uuid> {
    state
        .workspaces
        .iter()
        .filter(|w| belongs(url, &w.base_url))
        .max_by_key(|w| w.base_url.path().len())
        .map(|w| w.id)
}

/// `base` + relative path, keeping the base path (unlike `Url::join`, which replaces the last segment).
pub fn join(base: &Url, rel: &str) -> Url {
    let mut url = base.clone();
    url.set_path(&format!("{}/{}", base.path().trim_end_matches('/'), rel.trim_start_matches('/')));
    url
}

#[cfg(test)]
#[path = "../tests/unit/urls.rs"]
mod tests;
