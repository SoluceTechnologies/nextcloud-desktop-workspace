pub mod commands;
#[cfg(target_os = "linux")]
pub mod linux;

use percent_encoding::percent_decode_str;
use serde::Serialize;
#[cfg(not(target_os = "linux"))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::sync::{LazyLock, Mutex};
#[cfg(not(target_os = "linux"))]
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

#[cfg(not(target_os = "linux"))]
static DESTINATIONS: LazyLock<Mutex<HashMap<String, PathBuf>>> = LazyLock::new(Default::default);

#[derive(Clone, Serialize)]
struct Finished {
    path: Option<String>,
    success: bool,
}

#[cfg(not(target_os = "linux"))]
pub fn handle(app: &AppHandle, event: DownloadEvent<'_>) -> bool {
    let mut destinations = DESTINATIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match event {
        DownloadEvent::Requested { url, destination } => {
            let path = prepare_destination(&download_directory(app), &suggested_name(destination, &url));
            destinations.insert(url.to_string(), path.clone());
            *destination = path;
        }
        DownloadEvent::Finished { url, path, success } => {
            let chosen = destinations.remove(url.as_str());
            finished(app, path.or(chosen), success.then_some(()).ok_or_else(|| "the download failed".into()));
        }
        _ => {}
    }
    true
}

pub fn download_directory(app: &AppHandle) -> PathBuf {
    app.path().download_dir().unwrap_or_else(|_| std::env::temp_dir())
}

pub fn prepare_destination(directory: &Path, suggested: &str) -> PathBuf {
    if let Err(error) = std::fs::create_dir_all(directory) {
        log::warn!("could not create {}: {error}", directory.display());
    }
    unique_path(directory, &sanitize(suggested))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn outcome(failure: Option<String>, expected_bytes: u64, received_bytes: u64) -> Result<(), String> {
    match failure {
        Some(reason) => Err(reason),
        None if expected_bytes > 0 && received_bytes < expected_bytes => {
            Err(format!("incomplete: received {received_bytes} of {expected_bytes} bytes"))
        }
        None => Ok(()),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn discard_partial(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => log::info!("removed incomplete download {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!("could not remove incomplete download {}: {error}", path.display()),
    }
}

pub fn finished(app: &AppHandle, path: Option<PathBuf>, outcome: Result<(), String>) {
    let path = path.map(|path| path.display().to_string());
    match &outcome {
        Ok(()) => log::info!("download finished: {}", path.as_deref().unwrap_or("?")),
        Err(reason) => log::warn!("download failed ({reason}): {}", path.as_deref().unwrap_or("?")),
    }
    let _ = app.emit_to("shell", "download-finished", Finished { path, success: outcome.is_ok() });
}

#[cfg_attr(target_os = "linux", allow(dead_code))]
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
#[path = "../../../tests/unit/features/downloads.rs"]
mod tests;
