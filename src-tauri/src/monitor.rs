use crate::{auth, urls};
use crate::webviews::{engine, run};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use url::Url;
use uuid::Uuid;

const WATCH_DELAY: Duration = Duration::from_secs(4);
const RETRY_EVERY: Duration = Duration::from_secs(5);

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
