use crate::engine::{Effect, Engine, Shared};
use crate::model::{AppState, Appearance};
use crate::router::{self, Route};
use crate::session::{self, Decision, RetryBudget};
use crate::{auth, monitor, store};
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Instant;
use tauri::ipc::CapabilityBuilder;
use tauri::utils::config::BackgroundThrottlingPolicy;
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::window::{Color, WindowBuilder};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Theme, Webview, WebviewUrl, Window, WindowEvent, Wry,
};
use tauri_plugin_opener::OpenerExt;
use url::Url;
use uuid::Uuid;

pub const SIDEBAR_W: f64 = 68.0;
pub const TABBAR_H: f64 = 40.0;

type Res = Result<(), Box<dyn Error>>;

const BRIDGE_JS: &str = include_str!("bridge.js");

pub struct EffectTx(pub Sender<Vec<Effect>>);
pub struct StorePath(pub PathBuf);

pub fn engine(app: &AppHandle) -> MutexGuard<'_, Engine> {
    app.state::<Shared>().inner().lock().unwrap_or_else(|e| e.into_inner())
}

pub fn notice(app: &AppHandle, text: impl Into<String>) {
    let _ = app.emit_to("shell", "notice", text.into());
}

pub fn run(app: &AppHandle, fx: Vec<Effect>) {
    if !fx.is_empty() {
        let _ = app.state::<EffectTx>().0.send(fx);
    }
}

pub fn spawn_worker(app: AppHandle, rx: Receiver<Vec<Effect>>, sweep_state: Option<AppState>) {
    std::thread::spawn(move || {
        let mut sweep_state = sweep_state;
        let mut granted = HashSet::new();
        for batch in rx {
            apply_batch(&app, batch, &mut granted);
            // After the startup batch, so the first page does not wait for it.
            if let Some(state) = sweep_state.take() {
                sweep_profiles(&app, &state);
            }
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

pub fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let appearance = engine(app).state.theme;
    let builder = WindowBuilder::new(app, "main")
        .title("NC Workspaces")
        .theme(window_theme(appearance))
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 500.0);
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Transparent);
    let window = builder.build()?;
    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    window.add_child(
        WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize(),
        LogicalPosition::new(0.0, 0.0),
        size,
    )?;
    if let Ok(theme) = window.theme() {
        paint_chrome(&window, theme);
    }
    let w = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => relayout(&w),
        WindowEvent::ThemeChanged(theme) => paint_chrome(&w, *theme),
        _ => {}
    });
    Ok(())
}

fn window_theme(appearance: Appearance) -> Option<Theme> {
    match appearance {
        Appearance::System => None,
        Appearance::Light => Some(Theme::Light),
        Appearance::Dark => Some(Theme::Dark),
    }
}

fn apply_theme(app: &AppHandle, appearance: Appearance) -> Res {
    let window = main_window(app)?;
    window.set_theme(window_theme(appearance))?;
    paint_chrome(&window, window.theme()?);
    Ok(())
}

fn paint_chrome(window: &Window, theme: Theme) {
    let color = match theme {
        Theme::Dark => Color(30, 31, 34, 255),
        _ => Color(233, 234, 237, 255),
    };
    let _ = window.set_background_color(Some(color));
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

#[derive(Default)]
pub struct Fullscreen(pub Mutex<Option<String>>);

fn rect_for(window: &Window, label: &str) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let full = window
        .app_handle()
        .state::<Fullscreen>()
        .0
        .lock()
        .map(|f| f.as_deref() == Some(label))
        .unwrap_or(false);
    if full {
        let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
        return Ok((LogicalPosition::new(0.0, 0.0), size));
    }
    content_rect(window)
}

pub fn relayout(window: &Window) {
    for wv in window.webviews() {
        if wv.label().starts_with("ws-") {
            if let Ok((pos, size)) = rect_for(window, wv.label()) {
                let _ = wv.set_position(pos);
                let _ = wv.set_size(size);
            }
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
            Effect::Navigate { ws, tab, url } => with_webview(app, ws, tab, |w| {
                loading(app, tab, true);
                w.navigate(url)
            }),
            Effect::Reload { ws, tab } => with_webview(app, ws, tab, |w| {
                loading(app, tab, true);
                w.reload()
            }),
            Effect::Destroy { ws, tab } => with_webview(app, ws, tab, |w| w.close()),
            Effect::Show { ws, tab } => show_only(app, Some(&label(ws, tab))),
            Effect::HideContent => show_only(app, None),
            Effect::OpenExternal(url) => open_external(app, &url),
            Effect::ClearProfile { ws, delete } => clear_profile(app, ws, delete),
            Effect::Theme(appearance) => apply_theme(app, appearance),
        };
        if let Err(err) = result {
            eprintln!("[ncw] effect failed: {err}");
        }
    }
    if changed {
        publish(app);
    }
}

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

fn show_only(app: &AppHandle, target: Option<&str>) -> Res {
    let window = main_window(app)?;
    let content: Vec<Webview> = window.webviews().into_iter().filter(|w| w.label().starts_with("ws-")).collect();
    for w in content.iter().filter(|w| Some(w.label()) == target) {
        let (pos, size) = rect_for(&window, w.label())?;
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
    let (nav, popup, title, dl, load) = (app.clone(), app.clone(), app.clone(), app.clone(), app.clone());
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(url))
        .background_throttling(BackgroundThrottlingPolicy::Throttle)
        .initialization_script(BRIDGE_JS)
        .on_navigation(move |u| navigation(&nav, ws, tab, u))
        .on_new_window(move |u, _features| {
            let fx = engine(&popup).on_new_window(&u);
            run(&popup, fx);
            NewWindowResponse::Deny
        })
        .on_document_title_changed(move |_, t| {
            let fx = engine(&title).observe_title(tab, &t);
            run(&title, fx);
        })
        .on_download(move |_, event| crate::downloads::handle(&dl, event))
        .on_page_load(move |_, page| match page.event() {
            PageLoadEvent::Started => loading(&load, tab, true),
            PageLoadEvent::Finished => {
                loading(&load, tab, false);
                budget().release_if_answered(tab, page.url());
            }
        });
    #[cfg(target_os = "macos")]
    let builder = match safari_user_agent() {
        Some(ua) => builder.user_agent(ua),
        None => builder,
    };
    let (pos, size) = content_rect(&window)?;
    let webview = window.add_child(with_profile(app, builder, ws)?, pos, size)?;
    webview.hide()?;
    loading(app, tab, true);
    #[cfg(target_os = "linux")]
    crate::media::enable(&webview);
    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct TabLoading {
    tab: Uuid,
    loading: bool,
}

pub fn loading(app: &AppHandle, tab: Uuid, loading: bool) {
    if loading {
        monitor::watch_load(app, tab);
    }
    let _ = app.emit_to("shell", "tab-loading", TabLoading { tab, loading });
}

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

fn budget() -> MutexGuard<'static, RetryBudget> {
    static BUDGET: LazyLock<Mutex<RetryBudget>> = LazyLock::new(Default::default);
    BUDGET.lock().unwrap_or_else(|e| e.into_inner())
}

fn navigation(app: &AppHandle, ws: Uuid, tab: Uuid, url: &Url) -> bool {
    match router::classify_navigation(url) {
        Route::InPlace => keep_signed_in(app, ws, tab, url),
        Route::External(u) => {
            run(app, vec![Effect::OpenExternal(u)]);
            false
        }
        Route::Activate { .. } | Route::Deny => false,
    }
}

fn keep_signed_in(app: &AppHandle, ws: Uuid, tab: Uuid, url: &Url) -> bool {
    let Some((base, signed_in)) = engine(app).state.ws(ws).map(|w| (w.base_url.clone(), w.login.is_some())) else {
        return true;
    };
    let decision = session::decide(url, &base, signed_in, &mut budget(), tab, Instant::now());
    match decision {
        Decision::Allow => true,
        Decision::Reauth(target) => {
            auth::reload_signed_in(app, ws, tab, target);
            false
        }
        Decision::Looping => {
            notice(app, "This workspace keeps losing its session. Sign in on the page to continue.");
            true
        }
        Decision::Rejected => {
            auth::sign_out(app, ws, false);
            notice(app, "The app password was rejected. Sign in, then choose “Stay signed in…” in the workspace menu.");
            true
        }
        Decision::SignOut => {
            auth::sign_out(app, ws, true);
            true
        }
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
#[path = "../tests/unit/webviews.rs"]
mod tests;
