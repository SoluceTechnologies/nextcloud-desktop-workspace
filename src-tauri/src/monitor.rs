use crate::{auth, urls};
use crate::webviews::{engine, notice, run};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use url::Url;
use uuid::Uuid;

const WATCH_DELAY: Duration = Duration::from_secs(4);
const RETRY_EVERY: Duration = Duration::from_secs(5);
const POLL_EVERY: Duration = Duration::from_secs(60);
const MAX_BANNERS: usize = 3;

static PENDING: LazyLock<Mutex<HashMap<Uuid, u64>>> = LazyLock::new(Default::default);
static NEXT: AtomicU64 = AtomicU64::new(0);

fn pending() -> std::sync::MutexGuard<'static, HashMap<Uuid, u64>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Serialize)]
struct TabOffline {
    tab: Uuid,
    offline: bool,
}

fn reachable(base: &Url) -> bool {
    auth::agent().get(urls::join(base, "status.php").as_str()).call().is_ok()
}

pub fn watch_load(app: &AppHandle, tab: Uuid) {
    let generation = NEXT.fetch_add(1, Ordering::Relaxed);
    pending().insert(tab, generation);
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(WATCH_DELAY);
        if pending().get(&tab) != Some(&generation) {
            return;
        }
        let Some(base) = engine(&app).state.find_tab(tab).map(|(w, _)| w.base_url.clone()) else { return };
        if reachable(&base) {
            return;
        }
        let fx = engine(&app).set_offline(tab);
        if fx.is_empty() {
            return;
        }
        run(&app, fx);
        let _ = app.emit_to("shell", "tab-offline", TabOffline { tab, offline: true });
        while engine(&app).is_offline(tab) {
            thread::sleep(RETRY_EVERY);
            if reachable(&base) {
                retry(&app, tab);
            }
        }
    });
}

pub fn page_reported(tab: Uuid) {
    pending().remove(&tab);
}

pub fn retry(app: &AppHandle, tab: Uuid) {
    let fx = engine(app).retry_tab(tab);
    if fx.is_empty() {
        return;
    }
    let _ = app.emit_to("shell", "tab-offline", TabOffline { tab, offline: false });
    run(app, fx);
}

#[derive(Deserialize)]
struct Ocs<T> {
    ocs: OcsData<T>,
}

#[derive(Deserialize)]
struct OcsData<T> {
    data: T,
}

#[derive(Deserialize)]
struct Notification {
    notification_id: u64,
    subject: String,
}

enum Fetch {
    List(Vec<Notification>),
    Rejected,
    Unavailable,
}

fn fetch(base: &Url, c: &auth::Credentials) -> Fetch {
    let mut url = urls::join(base, "ocs/v2.php/apps/notifications/api/v2/notifications");
    url.set_query(Some("format=json"));
    let res = auth::agent()
        .get(url.as_str())
        .header("Authorization", c.header())
        .header("OCS-APIRequest", "true")
        .header("Accept", "application/json")
        .call();
    match res {
        Ok(r) if r.status() == 401 => Fetch::Rejected,
        Ok(mut r) if r.status() == 200 => match r.body_mut().read_json::<Ocs<Vec<Notification>>>() {
            Ok(o) => Fetch::List(o.ocs.data),
            Err(_) => Fetch::Unavailable,
        },
        _ => Fetch::Unavailable,
    }
}

fn fresh(list: &[Notification], seen: Option<u64>) -> Vec<&Notification> {
    let Some(seen) = seen else { return Vec::new() };
    let mut new: Vec<&Notification> = list.iter().filter(|n| n.notification_id > seen).collect();
    new.sort_by_key(|n| std::cmp::Reverse(n.notification_id));
    new.truncate(MAX_BANNERS);
    new
}

pub fn spawn_poller(app: AppHandle) {
    thread::spawn(move || {
        let mut seen: HashMap<Uuid, u64> = HashMap::new();
        let mut wait = Duration::from_secs(5);
        loop {
            thread::sleep(wait);
            wait = POLL_EVERY;
            let targets: Vec<(Uuid, Url, String)> = engine(&app)
                .state
                .workspaces
                .iter()
                .filter(|w| w.login.is_some())
                .map(|w| (w.id, w.base_url.clone(), w.name.clone()))
                .collect();
            let focused = app.get_window("main").and_then(|w| w.is_focused().ok()).unwrap_or(false);
            let mut unread: HashMap<Uuid, usize> = HashMap::new();
            for (ws, base, name) in targets {
                let Some(c) = auth::load(ws) else { continue };
                match fetch(&base, &c) {
                    Fetch::List(list) => {
                        if !focused {
                            for n in fresh(&list, seen.get(&ws).copied()) {
                                let _ = app.notification().builder().title(&name).body(&n.subject).show();
                            }
                        }
                        let max = list.iter().map(|n| n.notification_id).max().unwrap_or(0);
                        let entry = seen.entry(ws).or_insert(max);
                        *entry = (*entry).max(max);
                        unread.insert(ws, list.len());
                    }
                    Fetch::Rejected => {
                        auth::sign_out(&app, ws, false);
                        let fx = engine(&app).reopen_tabs(ws);
                        run(&app, fx);
                        notice(&app, format!("{name}: the app password was revoked. Sign in again from the workspace menu."));
                    }
                    Fetch::Unavailable => {}
                }
            }
            let total: usize = unread.values().sum();
            let _ = app.emit_to("shell", "unread", &unread);
            if let Some(w) = app.get_window("main") {
                let _ = w.set_badge_count((total > 0).then_some(total as i64));
            }
        }
    });
}

#[cfg(test)]
#[path = "../tests/unit/monitor.rs"]
mod tests;
