use crate::urls;
use crate::webviews::{self, engine, notice, run};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use url::Url;
use uuid::Uuid;

const SERVICE: &str = "nc-workspaces";
const FLOW_TTL: Duration = Duration::from_secs(20 * 60);

#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub login: String,
    pub password: String,
}

impl Credentials {
    pub fn header(&self) -> String {
        let raw = format!("{}:{}", self.login, self.password);
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(raw))
    }
}

static CACHE: LazyLock<Mutex<HashMap<Uuid, Credentials>>> = LazyLock::new(Default::default);

fn entry(ws: Uuid) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, &ws.simple().to_string())
}

pub fn load(ws: Uuid) -> Option<Credentials> {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = cache.get(&ws) {
        return Some(c.clone());
    }
    let c: Credentials = serde_json::from_str(&entry(ws).ok()?.get_password().ok()?).ok()?;
    cache.insert(ws, c.clone());
    Some(c)
}

fn save(ws: Uuid, c: &Credentials) -> Result<(), String> {
    let json = serde_json::to_string(c).map_err(|e| e.to_string())?;
    entry(ws).and_then(|e| e.set_password(&json)).map_err(|e| format!("Could not store the app password in the system keychain ({e})"))?;
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).insert(ws, c.clone());
    Ok(())
}

fn forget(ws: Uuid) -> Option<Credentials> {
    let cached = CACHE.lock().unwrap_or_else(|e| e.into_inner()).remove(&ws);
    let stored = entry(ws).ok().and_then(|e| {
        let c = e.get_password().ok().and_then(|j| serde_json::from_str(&j).ok());
        let _ = e.delete_credential();
        c
    });
    cached.or(stored)
}

pub fn agent() -> &'static ureq::Agent {
    static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .user_agent("NC Workspaces")
            .http_status_as_error(false)
            .tls_config(tls)
            .build()
            .into()
    });
    &AGENT
}

#[derive(Deserialize)]
struct Flow {
    poll: Poll,
    login: Url,
}

#[derive(Deserialize)]
struct Poll {
    token: String,
    endpoint: Url,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Granted {
    login_name: String,
    app_password: String,
}

fn begin(base: &Url) -> Result<Flow, String> {
    let url = urls::join(base, "index.php/login/v2");
    let mut res = agent().post(url.as_str()).send_empty().map_err(|e| format!("Could not reach the server ({e})"))?;
    if res.status() != 200 {
        return Err(format!("This server does not offer app passwords (HTTP {})", res.status()));
    }
    let flow: Flow = res.body_mut().read_json().map_err(|e| e.to_string())?;
    if !urls::belongs(&flow.login, base) || !urls::belongs(&flow.poll.endpoint, base) {
        return Err("The server reports a different address for itself (check overwrite.cli.url)".into());
    }
    Ok(flow)
}

fn poll(p: &Poll) -> Result<Option<Credentials>, String> {
    let mut res = agent().post(p.endpoint.as_str()).send_form([("token", p.token.as_str())]).map_err(|e| e.to_string())?;
    if res.status() != 200 {
        return Ok(None);
    }
    let g: Granted = res.body_mut().read_json().map_err(|e| e.to_string())?;
    Ok(Some(Credentials { login: g.login_name, password: g.app_password }))
}

fn revoke(base: &Url, c: &Credentials) {
    let url = urls::join(base, "ocs/v2.php/core/apppassword");
    let _ = agent().delete(url.as_str()).header("Authorization", c.header()).header("OCS-APIRequest", "true").call();
}

pub fn sign_in(app: &AppHandle, ws: Uuid) {
    let Some(base) = engine(app).state.ws(ws).map(|w| w.base_url.clone()) else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let flow = match begin(&base) {
            Ok(f) => f,
            Err(e) => return notice(&app, e),
        };
        let (tab, fx) = engine(&app).open_login(ws, flow.login);
        run(&app, fx);
        let deadline = Instant::now() + FLOW_TTL;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_secs(1));
            if engine(&app).state.find_tab(tab).is_none() {
                return;
            }
            let Ok(Some(c)) = poll(&flow.poll) else { continue };
            if let Err(e) = save(ws, &c) {
                revoke(&base, &c);
                return notice(&app, e);
            }
            let fx = engine(&app).finish_login(ws, tab, &c.login);
            run(&app, fx);
            return notice(&app, format!("Signed in as {}. This workspace now stays signed in.", c.login));
        }
    });
}

pub fn sign_out(app: &AppHandle, ws: Uuid, revoke_it: bool) {
    let base = engine(app).state.ws(ws).map(|w| w.base_url.clone());
    let cached = CACHE.lock().unwrap_or_else(|e| e.into_inner()).remove(&ws);
    let fx = engine(app).set_login(ws, None);
    run(app, fx);
    std::thread::spawn(move || {
        let creds = forget(ws).or(cached);
        if let (true, Some(base), Some(c)) = (revoke_it, base, creds) {
            revoke(&base, &c);
        }
    });
}

pub fn reload_signed_in(app: &AppHandle, ws: Uuid, tab: Uuid, url: Url) {
    let app = app.clone();
    std::thread::spawn(move || {
        let Some(webview) = app.get_webview(&webviews::label(ws, tab)) else { return };
        webviews::loading(&app, tab, true);
        let Some(c) = load(ws) else {
            sign_out(&app, ws, false);
            let _ = webview.navigate(url);
            return;
        };
        let (url, header) = (url.to_string(), c.header());
        let _ = webview.with_webview(move |w| platform::load(w, &url, &header));
    });
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSMutableURLRequest, NSString, NSURL};

    pub fn load(w: tauri::webview::PlatformWebview, url: &str, header: &str) {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else { return };
        let request = NSMutableURLRequest::requestWithURL(&url);
        request.setValue_forHTTPHeaderField(Some(&NSString::from_str(header)), &NSString::from_str("Authorization"));
        unsafe {
            let webview = w.inner() as *mut AnyObject;
            let _: *mut AnyObject = msg_send![webview, loadRequest: &*request];
        }
    }
}

#[cfg(windows)]
mod platform {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment9, ICoreWebView2_10};
    use windows_core::{Interface, HSTRING};

    pub fn load(w: tauri::webview::PlatformWebview, url: &str, header: &str) {
        unsafe {
            let Ok(core) = w.controller().CoreWebView2() else { return };
            let Ok(env) = w.environment().cast::<ICoreWebView2Environment9>() else { return };
            let headers = HSTRING::from(format!("Authorization: {header}\n"));
            let Ok(req) = env.CreateWebResourceRequest(&HSTRING::from(url), &HSTRING::from("GET"), None, &headers) else {
                return;
            };
            if let Ok(core) = core.cast::<ICoreWebView2_10>() {
                let _ = core.NavigateWithWebResourceRequest(&req);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use webkit2gtk::{URIRequest, URIRequestExt, WebViewExt};

    pub fn load(w: tauri::webview::PlatformWebview, url: &str, header: &str) {
        let req = URIRequest::new(url);
        if let Some(headers) = req.http_headers() {
            headers.append("Authorization", header);
        }
        w.inner().load_request(&req);
    }
}
