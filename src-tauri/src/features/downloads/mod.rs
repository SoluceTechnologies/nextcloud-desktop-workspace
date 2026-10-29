pub mod commands;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod registry;

use percent_encoding::percent_decode_str;
use registry::{Download, Registry, CANCELLED};
use serde::Serialize;
#[cfg(not(target_os = "linux"))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Instant;
#[cfg(not(target_os = "linux"))]
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use uuid::Uuid;

const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

#[cfg(not(target_os = "linux"))]
static DESTINATIONS: LazyLock<Mutex<HashMap<String, (PathBuf, Uuid)>>> = LazyLock::new(Default::default);

fn registry() -> MutexGuard<'static, Registry> {
    static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Default::default);
    REGISTRY.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
            let id = Uuid::new_v4();
            started(app, id, &path, false);
            destinations.insert(url.to_string(), (path.clone(), id));
            *destination = path;
        }
        DownloadEvent::Finished { url, path, success } => {
            let (chosen, id) = destinations.remove(url.as_str()).unzip();
            let outcome = success.then_some(()).ok_or_else(|| "the download failed".into());
            finished(app, id, path.or(chosen), outcome);
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

pub fn list() -> Vec<Download> {
    registry().list()
}

pub fn started(app: &AppHandle, id: Uuid, path: &Path, cancellable: bool) {
    let file_name = path.file_name().map_or_else(|| "download".into(), |name| name.to_string_lossy().into_owned());
    let download = registry().start(id, file_name, Some(path.display().to_string()), cancellable, Instant::now());
    changed(app, &download);
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn progressed(app: &AppHandle, id: Uuid, received: u64, total: u64) {
    let download = registry().progress(id, received, Some(total), Instant::now());
    if let Some(download) = download {
        changed(app, &download);
    }
}

fn changed(app: &AppHandle, download: &Download) {
    let _ = app.emit_to("shell", "download-changed", download);
}

pub fn finished(app: &AppHandle, id: Option<Uuid>, path: Option<PathBuf>, outcome: Result<(), String>) {
    let download = id.and_then(|id| registry().finish(id, outcome.clone()));
    if let Some(download) = &download {
        changed(app, download);
    }
    if outcome.as_ref().is_err_and(|reason| reason == CANCELLED) {
        log::info!("download cancelled");
        return;
    }
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
