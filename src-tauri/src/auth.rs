use crate::keychain::{self, Credentials};
use crate::runtime::{engine, notice, run};
use crate::session::{self, Decision, RetryBudget};
use crate::{http, signed_load, urls, webviews};
use serde::Deserialize;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use url::Url;
use uuid::Uuid;

const FLOW_LIFETIME: Duration = Duration::from_secs(20 * 60);
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Deserialize)]
struct LoginFlow {
    poll: PollEndpoint,
    login: Url,
}

#[derive(Deserialize)]
struct PollEndpoint {
    token: String,
    endpoint: Url,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrantedAccess {
    login_name: String,
    app_password: String,
}

fn retry_budget() -> MutexGuard<'static, RetryBudget> {
    static BUDGET: LazyLock<Mutex<RetryBudget>> = LazyLock::new(Default::default);
    BUDGET.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn base_url(app: &AppHandle, workspace_id: Uuid) -> Option<Url> {
    engine(app).state.workspace(workspace_id).map(|workspace| workspace.base_url.clone())
}

pub fn sign_in(app: &AppHandle, workspace_id: Uuid) {
    let Some(base) = base_url(app, workspace_id) else { return };
    let app = app.clone();
    thread::spawn(move || {
        let flow = match start_login_flow(&base) {
            Ok(flow) => flow,
            Err(message) => return notice(&app, message),
        };
        let (tab_id, effects) = engine(&app).open_login(workspace_id, flow.login);
        run(&app, effects);
        let deadline = Instant::now() + FLOW_LIFETIME;
        while Instant::now() < deadline {
            thread::sleep(POLL_INTERVAL);
            if engine(&app).state.find_tab(tab_id).is_none() {
                return;
            }
            let Ok(Some(credentials)) = poll_login_flow(&flow.poll) else { continue };
            if let Err(message) = keychain::save(workspace_id, &credentials) {
                revoke(&base, &credentials);
                return notice(&app, message);
            }
            let effects = engine(&app).finish_login(workspace_id, tab_id, &credentials.login);
            run(&app, effects);
            return notice(&app, format!("Signed in as {}. This workspace now stays signed in.", credentials.login));
        }
    });
}

pub fn sign_out(app: &AppHandle, workspace_id: Uuid, revoke_on_server: bool) {
    let base = base_url(app, workspace_id);
    let cached = keychain::forget_cached(workspace_id);
    let effects = engine(app).set_login(workspace_id, None);
    run(app, effects);
    thread::spawn(move || {
        let credentials = keychain::delete(workspace_id).or(cached);
        if let (true, Some(base), Some(credentials)) = (revoke_on_server, base, credentials) {
            revoke(&base, &credentials);
        }
    });
}

pub fn allow_navigation(app: &AppHandle, workspace_id: Uuid, tab_id: Uuid, url: &Url) -> bool {
    let workspace = engine(app)
        .state
        .workspace(workspace_id)
        .map(|workspace| (workspace.base_url.clone(), workspace.login.is_some()));
    let Some((base, signed_in)) = workspace else { return true };
    let decision = session::decide(url, &base, signed_in, &mut retry_budget(), tab_id, Instant::now());
    match decision {
        Decision::Allow => true,
        Decision::Reauthenticate(target) => {
            reload_signed_in(app, workspace_id, tab_id, target);
            false
        }
        Decision::Looping => {
            notice(app, "This workspace keeps losing its session. Sign in on the page to continue.");
            true
        }
        Decision::Rejected => {
            sign_out(app, workspace_id, false);
            notice(app, "The app password was rejected. Sign in, then choose “Stay signed in…” in the workspace menu.");
            true
        }
        Decision::SignOut => {
            sign_out(app, workspace_id, true);
            true
        }
    }
}

pub fn page_answered(tab_id: Uuid, url: &Url) {
    retry_budget().release_if_answered(tab_id, url);
}

fn reload_signed_in(app: &AppHandle, workspace_id: Uuid, tab_id: Uuid, url: Url) {
    let app = app.clone();
    thread::spawn(move || {
        let Some(webview) = app.get_webview(&webviews::label(workspace_id, tab_id)) else { return };
        webviews::loading(&app, tab_id, true);
        match keychain::load(workspace_id) {
            Some(credentials) => {
                let _ = signed_load::load(&webview, &url, credentials.authorization());
            }
            None => {
                sign_out(&app, workspace_id, false);
                let _ = webview.navigate(url);
            }
        }
    });
}

fn start_login_flow(base: &Url) -> Result<LoginFlow, String> {
    let url = urls::join(base, "index.php/login/v2");
    let mut response = http::client()
        .post(url.as_str())
        .send_empty()
        .map_err(|error| format!("Could not reach the server ({error})"))?;
    if response.status() != 200 {
        return Err(format!("This server does not offer app passwords (HTTP {})", response.status()));
    }
    let flow: LoginFlow = response.body_mut().read_json().map_err(|error| error.to_string())?;
    if !urls::belongs(&flow.login, base) || !urls::belongs(&flow.poll.endpoint, base) {
        return Err("The server reports a different address for itself (check overwrite.cli.url)".into());
    }
    Ok(flow)
}

fn poll_login_flow(poll: &PollEndpoint) -> Result<Option<Credentials>, String> {
    let mut response = http::client()
        .post(poll.endpoint.as_str())
        .send_form([("token", poll.token.as_str())])
        .map_err(|error| error.to_string())?;
    if response.status() != 200 {
        return Ok(None);
    }
    let granted: GrantedAccess = response.body_mut().read_json().map_err(|error| error.to_string())?;
    Ok(Some(Credentials { login: granted.login_name, password: granted.app_password }))
}

fn revoke(base: &Url, credentials: &Credentials) {
    let url = urls::join(base, "ocs/v2.php/core/apppassword");
    let _ = http::client()
        .delete(url.as_str())
        .header("Authorization", credentials.authorization())
        .header("OCS-APIRequest", "true")
        .call();
}
