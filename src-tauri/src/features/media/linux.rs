use tauri::Webview;
use webkit2gtk::{SettingsExt, WebViewExt};

// Must run before the tab loads its first real page: WebKit exposes
// RTCPeerConnection to a document only if WebRTC was enabled when it was created.
// Permissions are decided by `media::on_permission_request`.
pub fn enable(webview: &Webview) {
    let _ = webview.with_webview(|platform_webview| {
        if let Some(settings) = WebViewExt::settings(&platform_webview.inner()) {
            settings.set_enable_media_stream(true);
            settings.set_enable_webrtc(true);
        }
    });
}
