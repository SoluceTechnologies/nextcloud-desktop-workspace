use crate::offline;
use tauri::{AppHandle, Webview};
use uuid::Uuid;
use webkit2gtk::{WebProcessTerminationReason, WebViewExt};

pub fn watch(app: &AppHandle, webview: &Webview, tab_id: Uuid) {
    let app = app.clone();
    let _ = webview.with_webview(move |platform_webview| {
        platform_webview.inner().connect_web_process_terminated(move |_, reason| match crash_reason(reason) {
            Some(reason) => offline::crashed(&app, tab_id, reason),
            None => log::debug!("web process of tab {tab_id} ended on request"),
        });
    });
}

fn crash_reason(reason: WebProcessTerminationReason) -> Option<&'static str> {
    match reason {
        WebProcessTerminationReason::Crashed => Some("crashed"),
        WebProcessTerminationReason::ExceededMemoryLimit => Some("ran out of memory"),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../tests/unit/web_process.rs"]
mod tests;
