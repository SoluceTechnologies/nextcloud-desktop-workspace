use glib::translate::ToGlibPtr;
use std::ffi::{c_char, c_int, c_void, CStr};
use tauri::Webview;
use webkit2gtk::{SettingsExt, WebViewExt};

// View Transitions make the web process enter accelerated compositing even where the
// UI process has no AcceleratedBackingStore (e.g. Ubuntu's WebKitGTK on NVIDIA), and
// WebKitGTK 2.52 then dereferences it unchecked and crashes the whole app
// (AcceleratedBackingStore::update, https://bugs.webkit.org/show_bug.cgi?id=321683).
// Pages feature-detect the API, so turning it off only loses the animation.
const DISABLED_FEATURES: [&str; 2] = ["ViewTransitions", "CrossDocumentViewTransitions"];

// Must run before the tab loads its first real page: WebKit exposes RTCPeerConnection
// and the features above to a document only as configured when it was created.
// Permissions are decided by `media::on_permission_request`.
pub fn configure(webview: &Webview) {
    let _ = webview.with_webview(|platform_webview| {
        let Some(settings) = WebViewExt::settings(&platform_webview.inner()) else { return };
        settings.set_enable_media_stream(true);
        settings.set_enable_webrtc(true);
        if disable_features(&settings, &DISABLED_FEATURES).is_none() {
            log::warn!("this WebKitGTK cannot toggle features; view transitions stay enabled");
        }
    });
}

type GetAllFeatures = unsafe extern "C" fn() -> *mut c_void;
type FeatureListLength = unsafe extern "C" fn(*mut c_void) -> usize;
type FeatureListGet = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FeatureListUnref = unsafe extern "C" fn(*mut c_void);
type FeatureIdentifier = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type SetFeatureEnabled = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int);

// The feature API exists since WebKitGTK 2.42; it is looked up at runtime so the app
// still starts on older WebKitGTK.
fn symbol<T: Copy>(name: &CStr) -> Option<T> {
    let pointer = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    (!pointer.is_null()).then(|| unsafe { std::mem::transmute_copy::<*mut c_void, T>(&pointer) })
}

fn disable_features(settings: &webkit2gtk::Settings, identifiers: &[&str]) -> Option<()> {
    let all_features: GetAllFeatures = symbol(c"webkit_settings_get_all_features")?;
    let length: FeatureListLength = symbol(c"webkit_feature_list_get_length")?;
    let get: FeatureListGet = symbol(c"webkit_feature_list_get")?;
    let unref: FeatureListUnref = symbol(c"webkit_feature_list_unref")?;
    let identifier: FeatureIdentifier = symbol(c"webkit_feature_get_identifier")?;
    let set_enabled: SetFeatureEnabled = symbol(c"webkit_settings_set_feature_enabled")?;
    let settings: *mut webkit2gtk::ffi::WebKitSettings = settings.to_glib_none().0;
    unsafe {
        let list = all_features();
        for index in 0..length(list) {
            let feature = get(list, index);
            let name = CStr::from_ptr(identifier(feature)).to_str().unwrap_or_default();
            if identifiers.contains(&name) {
                set_enabled(settings.cast(), feature, 0);
            }
        }
        unref(list);
    }
    Some(())
}
