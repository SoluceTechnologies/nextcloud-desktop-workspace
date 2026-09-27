use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

static DESTINATIONS: LazyLock<Mutex<HashMap<String, PathBuf>>> = LazyLock::new(Default::default);

#[derive(Clone, Serialize)]
struct Finished {
    path: Option<String>,
    success: bool,
}

pub fn handle(app: &AppHandle, event: DownloadEvent<'_>) -> bool {
    let mut destinations = DESTINATIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match event {
        DownloadEvent::Requested { url, destination } => {
            let directory = app.path().download_dir().unwrap_or_else(|_| std::env::temp_dir());
            let path = unique_path(&directory, &sanitize(&suggested_name(destination, &url)));
            destinations.insert(url.to_string(), path.clone());
            *destination = path;
        }
        DownloadEvent::Finished { url, path, success } => {
            let chosen = destinations.remove(url.as_str());
            let path = path.or(chosen).map(|path| path.display().to_string());
            let _ = app.emit_to("shell", "download-finished", Finished { path, success });
        }
        _ => {}
    }
    true
}

pub fn suggested_name(destination: &Path, url: &Url) -> String {
    destination
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            let last_segment = url.path_segments()?.next_back()?;
            (!last_segment.is_empty()).then(|| percent_decode_str(last_segment).decode_utf8_lossy().into_owned())
        })
        .unwrap_or_else(|| "download".into())
}

pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| {
            if FORBIDDEN_CHARACTERS.contains(&character) || character.is_control() { '_' } else { character }
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.');
    if cleaned.is_empty() { "download".into() } else { cleaned.to_string() }
}

pub fn unique_path(directory: &Path, name: &str) -> PathBuf {
    let first = directory.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, format!(".{extension}")),
        _ => (name, String::new()),
    };
    (1..)
        .map(|index| directory.join(format!("{stem} ({index}){extension}")))
        .find(|path| !path.exists())
        .expect("unbounded range")
}

#[cfg(test)]
#[path = "../tests/unit/downloads.rs"]
mod tests;
