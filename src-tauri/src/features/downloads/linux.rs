use super::registry::CANCELLED;
use super::{discard_partial, download_directory, finished, outcome, prepare_destination, progressed, started};
use glib::translate::{IntoGlib, ToGlibPtr};
use glib::StaticType;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use tauri::{AppHandle, Webview};
use uuid::Uuid;
use webkit2gtk::{Download, DownloadError, DownloadExt, URIResponseExt, WebContext, WebContextExt, WebViewExt};

thread_local! {
    static ACTIVE: RefCell<HashMap<Uuid, Download>> = RefCell::default();
}

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

pub fn cancel(id: Uuid) -> bool {
    let download = ACTIVE.with(|active| active.borrow().get(&id).cloned());
    download.map(|download| download.cancel()).is_some()
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

fn expected_bytes(download: &Download) -> u64 {
    download.response().map_or(0, |response| response.content_length())
}

fn track(app: &AppHandle, download: &Download) {
    let id: Rc<Cell<Option<Uuid>>> = Rc::default();
    download.connect_decide_destination({
        let (app, id, directory) = (app.clone(), id.clone(), download_directory(app));
        move |download, suggested| {
            let destination = prepare_destination(&directory, suggested);
            let new_id = Uuid::new_v4();
            id.set(Some(new_id));
            ACTIVE.with(|active| active.borrow_mut().insert(new_id, download.clone()));
            started(&app, new_id, &destination, true);
            download.set_destination(&destination.to_string_lossy());
            true
        }
    });
    download.connect_received_data({
        let (app, id) = (app.clone(), id.clone());
        move |download, _| {
            if let Some(id) = id.get() {
                progressed(&app, id, download.received_data_length(), expected_bytes(download));
            }
        }
    });
    let failure: Rc<RefCell<Option<String>>> = Rc::default();
    download.connect_failed({
        let failure = failure.clone();
        move |_, error| {
            let reason = if error.matches(DownloadError::CancelledByUser) { CANCELLED.into() } else { error.to_string() };
            *failure.borrow_mut() = Some(reason);
        }
    });
    let app = app.clone();
    download.connect_finished(move |download| {
        let outcome = outcome(failure.borrow_mut().take(), expected_bytes(download), download.received_data_length());
        let path = download.destination().map(PathBuf::from);
        if let (Err(_), Some(path)) = (&outcome, &path) {
            discard_partial(path);
        }
        if let Some(id) = id.get() {
            ACTIVE.with(|active| active.borrow_mut().remove(&id));
        }
        finished(&app, id.get(), path, outcome);
    });
}
