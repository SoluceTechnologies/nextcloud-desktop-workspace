use tauri::Webview;
use url::Url;

pub fn load(webview: &Webview, url: &Url, authorization: String) -> tauri::Result<()> {
    let url = url.to_string();
    webview.with_webview(move |platform_webview| platform::load(platform_webview, &url, &authorization))
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSMutableURLRequest, NSString, NSURL};
    use tauri::webview::PlatformWebview;

    pub fn load(platform_webview: PlatformWebview, url: &str, authorization: &str) {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else { return };
        let request = NSMutableURLRequest::requestWithURL(&url);
        request.setValue_forHTTPHeaderField(
            Some(&NSString::from_str(authorization)),
            &NSString::from_str("Authorization"),
        );
        unsafe {
            let webview = platform_webview.inner() as *mut AnyObject;
            let _: *mut AnyObject = msg_send![webview, loadRequest: &*request];
        }
    }
}

#[cfg(windows)]
mod platform {
    use tauri::webview::PlatformWebview;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment9, ICoreWebView2_10};
    use windows_core::{Interface, HSTRING};

    pub fn load(platform_webview: PlatformWebview, url: &str, authorization: &str) {
        unsafe {
            let Ok(core_webview) = platform_webview.controller().CoreWebView2() else { return };
            let Ok(environment) = platform_webview.environment().cast::<ICoreWebView2Environment9>() else {
                return;
            };
            let headers = HSTRING::from(format!("Authorization: {authorization}\n"));
            let request =
                environment.CreateWebResourceRequest(&HSTRING::from(url), &HSTRING::from("GET"), None, &headers);
            let Ok(request) = request else { return };
            if let Ok(core_webview) = core_webview.cast::<ICoreWebView2_10>() {
                let _ = core_webview.NavigateWithWebResourceRequest(&request);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use tauri::webview::PlatformWebview;
    use webkit2gtk::{URIRequest, URIRequestExt, WebViewExt};

    pub fn load(platform_webview: PlatformWebview, url: &str, authorization: &str) {
        let request = URIRequest::new(url);
        if let Some(headers) = request.http_headers() {
            headers.append("Authorization", authorization);
        }
        platform_webview.inner().load_request(&request);
    }
}
