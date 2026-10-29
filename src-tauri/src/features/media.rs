use glib::prelude::*;
use tauri::Webview;
use webkit2gtk::{PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, WebViewExt};

pub fn enable(webview: &Webview) {
    let _ = webview.with_webview(|platform_webview| {
        let view = platform_webview.inner();
        if let Some(settings) = WebViewExt::settings(&view) {
            settings.set_enable_media_stream(true);
            settings.set_enable_webrtc(true);
        }
        view.connect_permission_request(|_, request| {
            if request.is::<UserMediaPermissionRequest>() {
                request.allow();
                true
            } else {
                false
            }
        });
    });
}
