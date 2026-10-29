use super::{discard_partial, download_directory, finished, outcome, prepare_destination};
use glib::translate::{IntoGlib, ToGlibPtr};
use glib::StaticType;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tauri::{AppHandle, Webview};
use webkit2gtk::{Download, DownloadExt, URIResponseExt, WebContext, WebContextExt, WebViewExt};

// Every wry webview adds its own download handler to the WebKitWebContext it shares
// with the other tabs of its workspace (wry's default attributes include one). The
// first one wins `decide-destination`, never creates a missing Downloads folder,
// and a closed tab's handler makes Tauri cancel every later download. So after each
// webview is created, the context keeps exactly one handler: this one.
pub fn watch(app: &AppHandle, webview: &Webview) {
    let app = app.clone();
    let _ = webview.with_webview(move |platform_webview| {
        let Some(context) = platform_webview.inner().context() else { return };
        disconnect_download_handlers(&context);
        context.connect_download_started(move |_, download| track(&app, download));
    });
}

fn disconnect_download_handlers(context: &WebContext) {
    unsafe {
        let signal = glib::gobject_ffi::g_signal_lookup(c"download-started".as_ptr(), WebContext::static_type().into_glib());
        let instance: *mut webkit2gtk::ffi::WebKitWebContext = context.to_glib_none().0;
        glib::gobject_ffi::g_signal_handlers_disconnect_matched(
            instance.cast(),
            glib::gobject_ffi::G_SIGNAL_MATCH_ID,
            signal,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}

fn track(app: &AppHandle, download: &Download) {
    let directory = download_directory(app);
    download.connect_decide_destination(move |download, suggested| {
        let destination = prepare_destination(&directory, suggested);
        log::debug!("download destination: {}", destination.display());
        download.set_destination(&destination.to_string_lossy());
        true
    });
    let failure: Rc<RefCell<Option<String>>> = Rc::default();
    download.connect_failed({
        let failure = failure.clone();
        move |_, error| *failure.borrow_mut() = Some(error.to_string())
    });
    let app = app.clone();
    download.connect_finished(move |download| {
        let expected = download.response().map_or(0, |response| response.content_length());
        let outcome = outcome(failure.borrow_mut().take(), expected, download.received_data_length());
        let path = download.destination().map(PathBuf::from);
        if let (Err(_), Some(path)) = (&outcome, &path) {
            discard_partial(path);
        }
        finished(&app, path, outcome);
    });
}
