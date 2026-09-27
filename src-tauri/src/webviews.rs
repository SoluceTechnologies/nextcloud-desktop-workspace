use crate::engine::Effect;
use crate::router::{self, Route};
use crate::runtime::{engine, run, EffectResult};
use crate::{auth, downloads, offline, profiles, window};
use std::collections::HashSet;
use tauri::ipc::CapabilityBuilder;
use tauri::utils::acl::RemoteUrlPattern;
use tauri::utils::config::BackgroundThrottlingPolicy;
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, Emitter, Manager, Webview, WebviewUrl};
use tauri_plugin_opener::OpenerExt;
use url::Url;
use uuid::Uuid;

const BRIDGE_JS: &str = include_str!("bridge.js");
const CONTENT_PREFIX: &str = "ws-";
const TAB_SEPARATOR: &str = "-t-";
const URL_PATTERN_SYNTAX: &str = ":*(){}+?\\";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TabLoading {
    tab_id: Uuid,
    loading: bool,
}

pub fn label_prefix(workspace_id: Uuid) -> String {
    format!("{CONTENT_PREFIX}{}{TAB_SEPARATOR}", workspace_id.simple())
}

pub fn label(workspace_id: Uuid, tab_id: Uuid) -> String {
    format!("{}{}", label_prefix(workspace_id), tab_id.simple())
}

pub fn tab_of(label: &str) -> Option<Uuid> {
    let (_, tab_id) = label.strip_prefix(CONTENT_PREFIX)?.split_once(TAB_SEPARATOR)?;
    Uuid::parse_str(tab_id).ok()
}

pub fn is_content(label: &str) -> bool {
    label.starts_with(CONTENT_PREFIX)
}

pub fn with_webview(
    app: &AppHandle,
    workspace_id: Uuid,
    tab_id: Uuid,
    action: impl FnOnce(Webview) -> tauri::Result<()>,
) -> EffectResult {
    match app.get_webview(&label(workspace_id, tab_id)) {
        Some(webview) => Ok(action(webview)?),
        None => Ok(()),
    }
}

pub fn create(
    app: &AppHandle,
    workspace_id: Uuid,
    tab_id: Uuid,
    url: Url,
    bridged_workspaces: &mut HashSet<Uuid>,
) -> EffectResult {
    let label = label(workspace_id, tab_id);
    if app.get_webview(&label).is_some() {
        return Ok(());
    }
    let window = window::main_window(app)?;
    if !bridged_workspaces.contains(&workspace_id) {
        grant_bridge(app, workspace_id, &url)?;
        bridged_workspaces.insert(workspace_id);
    }
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(url))
        .background_throttling(BackgroundThrottlingPolicy::Throttle)
        .initialization_script(BRIDGE_JS);
    let builder = with_handlers(app, builder, workspace_id, tab_id);
    #[cfg(target_os = "macos")]
    let builder = match safari_user_agent() {
        Some(user_agent) => builder.user_agent(user_agent),
        None => builder,
    };
    let (position, size) = window::content_rect(&window)?;
    let webview = window.add_child(profiles::with_profile(app, builder, workspace_id)?, position, size)?;
    webview.hide()?;
    loading(app, tab_id, true);
    #[cfg(target_os = "linux")]
    crate::media::enable(&webview);
    Ok(())
}

fn with_handlers(
    app: &AppHandle,
    builder: WebviewBuilder<tauri::Wry>,
    workspace_id: Uuid,
    tab_id: Uuid,
) -> WebviewBuilder<tauri::Wry> {
    let (navigation_app, popup_app, title_app, download_app, load_app) =
        (app.clone(), app.clone(), app.clone(), app.clone(), app.clone());
    builder
        .on_navigation(move |url| allow_navigation(&navigation_app, workspace_id, tab_id, url))
        .on_new_window(move |url, _features| {
            let effects = engine(&popup_app).on_new_window(&url);
            run(&popup_app, effects);
            NewWindowResponse::Deny
        })
        .on_document_title_changed(move |_, title| {
            let effects = engine(&title_app).observe_title(tab_id, &title);
            run(&title_app, effects);
        })
        .on_download(move |_, event| downloads::handle(&download_app, event))
        .on_page_load(move |_, page| match page.event() {
            PageLoadEvent::Started => loading(&load_app, tab_id, true),
            PageLoadEvent::Finished => {
                loading(&load_app, tab_id, false);
                auth::page_answered(tab_id, page.url());
            }
        })
}

pub fn loading(app: &AppHandle, tab_id: Uuid, loading: bool) {
    if loading {
        offline::watch_load(app, tab_id);
    }
    let _ = app.emit_to("shell", "tab-loading", TabLoading { tab_id, loading });
}

pub fn open_external(app: &AppHandle, url: &Url) -> EffectResult {
    if !matches!(url.scheme(), "http" | "https" | "mailto" | "tel") {
        return Err(format!("refusing to open {url}").into());
    }
    app.opener().open_url(url.as_str(), None::<&str>)?;
    Ok(())
}

pub fn bridge_pattern(url: &Url) -> Result<String, String> {
    let mut host = String::new();
    for character in url.host_str().unwrap_or_default().chars() {
        if URL_PATTERN_SYNTAX.contains(character) {
            host.push('\\');
        }
        host.push(character);
    }
    let port = url.port().map(|port| format!(":{port}")).unwrap_or_default();
    let pattern = format!("{}://{host}{port}/*", url.scheme());
    pattern.parse::<RemoteUrlPattern>().map_err(|error| error.to_string())?;
    Ok(pattern)
}

fn grant_bridge(app: &AppHandle, workspace_id: Uuid, url: &Url) -> EffectResult {
    app.add_capability(
        CapabilityBuilder::new(format!("nc-{}", workspace_id.simple()))
            .remote(bridge_pattern(url)?)
            .webview(format!("{}*", label_prefix(workspace_id)))
            .local(false)
            .permission("allow-nc-report-location")
            .permission("allow-nc-report-meta")
            .permission("allow-nc-report-fullscreen"),
    )?;
    Ok(())
}

fn allow_navigation(app: &AppHandle, workspace_id: Uuid, tab_id: Uuid, url: &Url) -> bool {
    match router::classify_navigation(url) {
        Route::InPlace => auth::allow_navigation(app, workspace_id, tab_id, url),
        Route::External(url) => {
            run(app, vec![Effect::OpenExternal(url)]);
            false
        }
        Route::Activate { .. } | Route::Deny => false,
    }
}

#[cfg(target_os = "macos")]
fn safari_user_agent() -> Option<&'static str> {
    static USER_AGENT: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    USER_AGENT
        .get_or_init(|| {
            let output = std::process::Command::new("defaults")
                .args(["read", "/Applications/Safari.app/Contents/Info.plist", "CFBundleShortVersionString"])
                .output()
                .ok()?;
            let version = String::from_utf8(output.stdout).ok()?;
            let version = version.trim();
            let valid = !version.is_empty() && version.chars().all(|character| character.is_ascii_digit() || character == '.');
            valid.then(|| {
                format!(
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{version} Safari/605.1.15"
                )
            })
        })
        .as_deref()
}

#[cfg(test)]
#[path = "../tests/unit/webviews.rs"]
mod tests;
