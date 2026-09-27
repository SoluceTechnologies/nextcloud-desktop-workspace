//! Downloads land in the user's Downloads folder with a unique, sanitized name.

use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

/// Destination chosen per URL, for engines that report no path on completion (macOS).
static PENDING: LazyLock<Mutex<HashMap<String, PathBuf>>> = LazyLock::new(Default::default);

#[derive(Clone, Serialize)]
struct Finished {
    path: Option<String>,
    success: bool,
}

pub fn handle(app: &AppHandle, event: DownloadEvent<'_>) -> bool {
    match event {
        DownloadEvent::Requested { url, destination } => {
            let dir = app.path().download_dir().unwrap_or_else(|_| std::env::temp_dir());
            let path = unique_path(&dir, &sanitize(&suggested_name(destination, &url)));
            PENDING.lock().unwrap_or_else(|e| e.into_inner()).insert(url.to_string(), path.clone());
            *destination = path;
        }
        DownloadEvent::Finished { url, path, success } => {
            let pending = PENDING.lock().unwrap_or_else(|e| e.into_inner()).remove(url.as_str());
            let path = path.or(pending).map(|p| p.display().to_string());
            let _ = app.emit_to("shell", "download-finished", Finished { path, success });
        }
        _ => {}
    }
    true
}

pub fn suggested_name(destination: &Path, url: &Url) -> String {
    destination
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            let last = url.path_segments()?.last()?;
            (!last.is_empty()).then(|| percent_decode_str(last).decode_utf8_lossy().into_owned())
        })
        .unwrap_or_else(|| "download".into())
}

pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() { "download".into() } else { cleaned }
}

pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
        _ => (name, String::new()),
    };
    (1..).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).expect("unbounded range")
}

#[cfg(test)]
#[path = "../tests/unit/downloads.rs"]
mod tests;
