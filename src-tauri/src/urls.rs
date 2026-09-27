use crate::model::AppState;
use url::Url;
use uuid::Uuid;

pub fn normalize(input: &str) -> Result<Url, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Enter a server URL".into());
    }
    let full = if input.contains("://") { input.to_string() } else { format!("https://{input}") };
    let mut url = Url::parse(&full).map_err(|error| format!("Invalid URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http:// and https:// addresses are supported".into());
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err("The address has no host".into());
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    let kept_segments: Vec<String> = url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty())
                .take_while(|segment| !matches!(*segment, "index.php" | "login" | "apps"))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    url.set_path(&format!("/{}", kept_segments.join("/")));
    Ok(url)
}

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

pub fn find_workspace(state: &AppState, url: &Url) -> Option<Uuid> {
    state
        .workspaces
        .iter()
        .filter(|workspace| belongs(url, &workspace.base_url))
        .max_by_key(|workspace| workspace.base_url.path().len())
        .map(|workspace| workspace.id)
}

pub fn join(base: &Url, relative: &str) -> Url {
    let mut url = base.clone();
    url.set_path(&format!("{}/{}", base.path().trim_end_matches('/'), relative.trim_start_matches('/')));
    url
}

#[cfg(test)]
#[path = "../tests/unit/urls.rs"]
mod tests;
