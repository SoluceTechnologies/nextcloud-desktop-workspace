//! Webview side of the app: main window layout, the effect worker, content webview creation with
//! per-workspace profiles, and the per-origin bridge capability.

use crate::engine::{Effect, Engine, Shared};
use crate::model::AppState;
use crate::router::{self, Route};
use crate::store;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::MutexGuard;
use tauri::ipc::CapabilityBuilder;
use tauri::webview::WebviewBuilder;
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Webview, WebviewUrl, Window, WindowEvent, Wry,
};
use tauri_plugin_opener::OpenerExt;
use url::Url;
use uuid::Uuid;

pub const SIDEBAR_W: f64 = 68.0;
pub const TABBAR_H: f64 = 40.0;

type Res = Result<(), Box<dyn Error>>;

pub struct EffectTx(pub Sender<Vec<Effect>>);
pub struct StorePath(pub PathBuf);

pub fn engine(app: &AppHandle) -> MutexGuard<'_, Engine> {
    app.state::<Shared>().inner().lock().unwrap_or_else(|e| e.into_inner())
}

/// Queues effects for the worker thread. Safe to call from any thread, including webview callbacks.
pub fn run(app: &AppHandle, fx: Vec<Effect>) {
    if !fx.is_empty() {
        let _ = app.state::<EffectTx>().0.send(fx);
    }
}

/// Single worker: applies effect batches in order, off the main thread and outside the engine lock.
/// `sweep_state`, when present, triggers `sweep_profiles` first (skipped when `workspaces.json` failed
/// to load: the caller passes `None` rather than sweep against an empty recovery state, which would
/// delete every live profile). Run here, not in `setup`, because the macOS data-store calls need the
/// main thread's event loop pumping, which it isn't yet during `setup`.
pub fn spawn_worker(app: AppHandle, rx: Receiver<Vec<Effect>>, sweep_state: Option<AppState>) {
    std::thread::spawn(move || {
        if let Some(state) = sweep_state {
            sweep_profiles(&app, &state);
        }
        let mut granted = HashSet::new();
        for batch in rx {
            apply_batch(&app, batch, &mut granted);
        }
    });
}

pub fn label(ws: Uuid, tab: Uuid) -> String {
    format!("ws-{}-t-{}", ws.simple(), tab.simple())
}

pub fn tab_of(label: &str) -> Option<Uuid> {
    let (_, tab) = label.strip_prefix("ws-")?.split_once("-t-")?;
    Uuid::parse_str(tab).ok()
}

/// Main window: no own webview; the React shell is a full-size child, content webviews go on top of it.
pub fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let window = WindowBuilder::new(app, "main")
        .title("NC Workspaces")
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 500.0)
        .build()?;
    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    window.add_child(
        WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize(),
        LogicalPosition::new(0.0, 0.0),
        size,
    )?;
    let w = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }) {
            relayout(&w);
        }
    });
    Ok(())
}

fn main_window(app: &AppHandle) -> Result<Window, Box<dyn Error>> {
    app.get_window("main").ok_or_else(|| "main window missing".into())
}

fn content_rect(window: &Window) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    Ok((
        LogicalPosition::new(SIDEBAR_W, TABBAR_H),
        LogicalSize::new((size.width - SIDEBAR_W).max(0.0), (size.height - TABBAR_H).max(0.0)),
    ))
}

/// Keeps every content webview (label `ws-…`) in the content rectangle.
pub fn relayout(window: &Window) {
    let Ok((pos, size)) = content_rect(window) else { return };
    for wv in window.webviews() {
        if wv.label().starts_with("ws-") {
            let _ = wv.set_position(pos);
            let _ = wv.set_size(size);
        }
    }
}

fn apply_batch(app: &AppHandle, batch: Vec<Effect>, granted: &mut HashSet<Uuid>) {
    let mut changed = false;
    for effect in batch {
        let result = match effect {
            Effect::Changed => {
                changed = true;
                Ok(())
            }
            Effect::Create { ws, tab, url } => create(app, ws, tab, url, granted),
            Effect::Navigate { ws, tab, url } => with_webview(app, ws, tab, |w| w.navigate(url)),
            Effect::Reload { ws, tab } => with_webview(app, ws, tab, |w| w.reload()),
            Effect::Destroy { ws, tab } => with_webview(app, ws, tab, |w| w.close()),
            Effect::Show { ws, tab } => show_only(app, Some(&label(ws, tab))),
            Effect::HideContent => show_only(app, None),
            Effect::OpenExternal(url) => open_external(app, &url),
            Effect::ClearProfile { ws, delete } => clear_profile(app, ws, delete),
        };
        if let Err(err) = result {
            eprintln!("[ncw] effect failed: {err}");
        }
    }
    if changed {
        publish(app);
    }
}

/// Emits the snapshot to the shell and saves it. The lock is released before emitting.
fn publish(app: &AppHandle) {
    let state = engine(app).state.clone();
    let _ = app.emit_to("shell", "state-changed", &state);
    if let Err(err) = store::save(&app.state::<StorePath>().0, &state) {
        eprintln!("[ncw] save failed: {err}");
    }
}

fn with_webview(app: &AppHandle, ws: Uuid, tab: Uuid, f: impl FnOnce(Webview) -> tauri::Result<()>) -> Res {
    match app.get_webview(&label(ws, tab)) {
        Some(w) => Ok(f(w)?),
        None => Ok(()), // cold tab: nothing to do
    }
}

/// Shows `target` and hides every other content webview; `None` hides them all.
fn show_only(app: &AppHandle, target: Option<&str>) -> Res {
    let window = main_window(app)?;
    let (pos, size) = content_rect(&window)?;
    let content: Vec<Webview> = window.webviews().into_iter().filter(|w| w.label().starts_with("ws-")).collect();
    for w in content.iter().filter(|w| Some(w.label()) == target) {
        w.set_position(pos)?;
        w.set_size(size)?;
        w.show()?;
        w.set_focus()?;
    }
    for w in content.iter().filter(|w| Some(w.label()) != target) {
        w.hide()?;
    }
    Ok(())
}

fn open_external(app: &AppHandle, url: &Url) -> Res {
    if !matches!(url.scheme(), "http" | "https" | "mailto" | "tel") {
        return Err(format!("refusing to open {url}").into());
    }
    app.opener().open_url(url.as_str(), None::<&str>)?;
    Ok(())
}

fn create(app: &AppHandle, ws: Uuid, tab: Uuid, url: Url, granted: &mut HashSet<Uuid>) -> Res {
    let label = label(ws, tab);
    if app.get_webview(&label).is_some() {
        return Ok(());
    }
    let window = main_window(app)?;
    if !granted.contains(&ws) {
        grant_bridge(app, ws, &url)?;
        granted.insert(ws);
    }
    let nav = app.clone();
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(url)).on_navigation(move |u| navigation(&nav, u));
    #[cfg(target_os = "macos")]
    let builder = match safari_user_agent() {
        Some(ua) => builder.user_agent(ua),
        None => builder,
    };
    let (pos, size) = content_rect(&window)?;
    let webview = window.add_child(with_profile(app, builder, ws)?, pos, size)?;
    webview.hide()?;
    Ok(())
}

/// Builds and validates the `remote` URLPattern for this workspace's bridge capability. Host chars
/// that are URLPattern syntax (`:` in IPv6 literals; `*`, `+`, `(`… which WHATWG allows in domains)
/// are escaped so they match literally. Tauri's ACL resolver (`Resolved::resolve`, called from
/// `Manager::add_capability`) panics on an unparsable pattern, poisoning the ACL mutex and crashing
/// the app, so the pattern is also parsed here with the same parser to turn any leftover case into `Err`.
pub fn bridge_pattern(url: &Url) -> Result<String, String> {
    let mut host = String::new();
    for c in url.host_str().unwrap_or_default().chars() {
        if ":*(){}+?\\".contains(c) {
            host.push('\\');
        }
        host.push(c);
    }
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    let pattern = format!("{}://{host}{port}/*", url.scheme());
    pattern.parse::<tauri::utils::acl::RemoteUrlPattern>().map_err(|e| e.to_string())?;
    Ok(pattern)
}

/// Pages of this workspace's origin, in this workspace's webviews only, may call the report-only bridge.
fn grant_bridge(app: &AppHandle, ws: Uuid, url: &Url) -> Res {
    let pattern = bridge_pattern(url)?;
    app.add_capability(
        CapabilityBuilder::new(format!("nc-{}", ws.simple()))
            .remote(pattern)
            .webview(format!("ws-{}-t-*", ws.simple()))
            .local(false)
            .permission("allow-nc-report-location")
            .permission("allow-nc-report-meta")
            .permission("allow-nc-report-fullscreen"),
    )?;
    Ok(())
}

/// `on_navigation` runs for iframes too (macOS), so it only filters schemes (spec §5.4).
fn navigation(app: &AppHandle, url: &Url) -> bool {
    match router::classify_navigation(url) {
        Route::InPlace => true,
        Route::External(u) => {
            run(app, vec![Effect::OpenExternal(u)]);
            false
        }
        Route::Activate { .. } | Route::Deny => false,
    }
}

fn with_profile(app: &AppHandle, builder: WebviewBuilder<Wry>, ws: Uuid) -> Result<WebviewBuilder<Wry>, Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        Ok(builder.data_store_identifier(*ws.as_bytes()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let dir = profile_dir(app, ws)?;
        std::fs::create_dir_all(&dir)?;
        Ok(builder.data_directory(dir))
    }
}

/// Nextcloud rejects WKWebView's default user agent (no Safari version) as an unsupported browser.
/// WKWebView is the system WebKit, the same engine as the installed Safari, so send Safari's user agent.
#[cfg(target_os = "macos")]
fn safari_user_agent() -> Option<&'static str> {
    static UA: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    UA.get_or_init(|| {
        let out = std::process::Command::new("defaults")
            .args(["read", "/Applications/Safari.app/Contents/Info.plist", "CFBundleShortVersionString"])
            .output()
            .ok()?;
        let version = String::from_utf8(out.stdout).ok()?;
        let version = version.trim();
        let valid = !version.is_empty() && version.chars().all(|c| c.is_ascii_digit() || c == '.');
        valid.then(|| {
            format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{version} Safari/605.1.15"
            )
        })
    })
    .as_deref()
}

#[cfg(not(target_os = "macos"))]
fn profiles_root(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join("profiles"))
}

#[cfg(not(target_os = "macos"))]
fn profile_dir(app: &AppHandle, ws: Uuid) -> tauri::Result<PathBuf> {
    Ok(profiles_root(app)?.join(ws.simple().to_string()))
}

/// Deletes profile directories of workspaces that no longer exist (a delete can fail while
/// WebView2 still holds the files; this retries at next launch). On macOS, sweeps orphaned
/// WKWebsiteDataStores instead (spec §7.1).
pub fn sweep_profiles(app: &AppHandle, state: &AppState) {
    #[cfg(target_os = "macos")]
    {
        sweep_data_stores(app, state);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(root) = profiles_root(app) else { return };
        let Ok(entries) = std::fs::read_dir(root) else { return };
        let keep: HashSet<String> = state.workspaces.iter().map(|w| w.id.simple().to_string()).collect();
        for entry in entries.flatten() {
            if !keep.contains(entry.file_name().to_string_lossy().as_ref()) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

/// Removes every WKWebsiteDataStore whose identifier isn't a current workspace id: orphaned by a
/// `remove_workspace` whose `remove_data_store` call didn't make it to disk (e.g. a crash), or by
/// workspaces removed from `workspaces.json` by hand.
#[cfg(target_os = "macos")]
fn sweep_data_stores(app: &AppHandle, state: &AppState) {
    let keep: HashSet<Uuid> = state.workspaces.iter().map(|w| w.id).collect();
    let ids = match tauri::async_runtime::block_on(app.fetch_data_store_identifiers()) {
        Ok(ids) => ids,
        Err(err) => {
            eprintln!("[ncw] fetch_data_store_identifiers failed: {err}");
            return;
        }
    };
    for id in ids {
        if !keep.contains(&Uuid::from_bytes(id)) {
            if let Err(err) = tauri::async_runtime::block_on(app.remove_data_store(id)) {
                eprintln!("[ncw] remove_data_store failed: {err}");
            }
        }
    }
}

/// Clears the workspace profile using one of its webviews (a temporary hidden one if none is live).
/// On macOS, a full delete (workspace removal) skips the webview entirely: removing the
/// WKWebsiteDataStore by identifier is the actual removal spec §7.1 asks for, and doesn't need one.
fn clear_profile(app: &AppHandle, ws: Uuid, delete: bool) -> Res {
    #[cfg(target_os = "macos")]
    if delete {
        return Ok(tauri::async_runtime::block_on(app.remove_data_store(*ws.as_bytes()))?);
    }
    let prefix = format!("ws-{}-t-", ws.simple());
    let window = main_window(app)?;
    let existing = window.webviews().into_iter().find(|w| w.label().starts_with(&prefix));
    let (webview, temporary) = match existing {
        Some(w) => (w, false),
        None => {
            let blank = WebviewBuilder::new(format!("{prefix}clear"), WebviewUrl::External("about:blank".parse()?));
            let w = window.add_child(with_profile(app, blank, ws)?, LogicalPosition::new(0.0, 0.0), LogicalSize::new(1.0, 1.0))?;
            w.hide()?;
            (w, true)
        }
    };
    webview.clear_all_browsing_data()?;
    if temporary {
        webview.close()?;
    }
    #[cfg(not(target_os = "macos"))]
    if delete {
        if let Ok(dir) = profile_dir(app, ws) {
            let _ = std::fs::remove_dir_all(dir); // may fail on Windows until restart; sweep_profiles retries
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_pattern_matches_only_its_origin() {
        use tauri::utils::acl::RemoteUrlPattern;
        for (base, same, other) in [
            ("https://cloud.example.com/nc/", "https://cloud.example.com/index.php/apps/files/?dir=/#x", "http://cloud.example.com/"),
            ("http://localhost:8080/", "http://localhost:8080/x", "http://localhost:8081/x"),
            ("https://[::1]:8443/", "https://[::1]:8443/x", "https://[::2]:8443/x"),
            ("https://a+b.example/", "https://a+b.example/x", "https://aab.example/x"),
            ("https://a*b.example/", "https://a*b.example/x", "https://axxb.example/x"),
        ] {
            let p: RemoteUrlPattern = bridge_pattern(&Url::parse(base).unwrap()).unwrap().parse().unwrap();
            assert!(p.test(&Url::parse(same).unwrap()), "{base} should match {same}");
            assert!(!p.test(&Url::parse(other).unwrap()), "{base} should not match {other}");
        }
    }

    #[test]
    fn labels_round_trip_and_reject_others() {
        let (ws, tab) = (Uuid::new_v4(), Uuid::new_v4());
        let l = label(ws, tab);
        assert!(l.starts_with("ws-"));
        assert_eq!(tab_of(&l), Some(tab));
        assert_eq!(tab_of("shell"), None);
        assert_eq!(tab_of(&format!("ws-{}-t-clear", ws.simple())), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn safari_user_agent_has_version_and_safari_tokens() {
        let ua = safari_user_agent().expect("Safari.app present on macOS CI/dev machines");
        assert!(ua.contains("Version/"));
        assert!(ua.ends_with("Safari/605.1.15"));
    }
}
