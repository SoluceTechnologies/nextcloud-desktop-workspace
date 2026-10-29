#[cfg(target_os = "linux")]
pub mod linux;

use crate::domain::urls;
use crate::runtime::engine;
use tauri::webview::{PermissionKind, PermissionResponse};
use tauri::{AppHandle, Webview};
use url::Url;
use uuid::Uuid;

// Camera, microphone and screen capture are granted without a prompt to the
// workspace's own pages, and refused to anything else a tab may have navigated to.
pub fn decide(kind: PermissionKind, page: Option<&Url>, base: &Url) -> PermissionResponse {
    match kind {
        PermissionKind::Microphone | PermissionKind::Camera | PermissionKind::DisplayCapture => {
            if page.is_some_and(|page| urls::belongs(page, base)) {
                PermissionResponse::Allow
            } else {
                log::warn!("refused {kind:?} to {}", page.map_or_else(|| "an unknown page".into(), Url::to_string));
                PermissionResponse::Deny
            }
        }
        _ => PermissionResponse::Default,
    }
}

pub fn on_permission_request(app: &AppHandle, workspace_id: Uuid, webview: &Webview, kind: PermissionKind) -> PermissionResponse {
    let base = engine(app).state.workspace(workspace_id).map(|workspace| workspace.base_url.clone());
    let Some(base) = base else { return PermissionResponse::Deny };
    decide(kind, webview.url().ok().as_ref(), &base)
}

#[cfg(test)]
#[path = "../../../tests/unit/features/media.rs"]
mod tests;
