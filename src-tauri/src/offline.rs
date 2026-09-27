use crate::runtime::{engine, run};
use crate::{http, urls};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use url::Url;
use uuid::Uuid;

const WATCH_DELAY: Duration = Duration::from_secs(4);
const RETRY_INTERVAL: Duration = Duration::from_secs(5);

static NEXT_LOAD: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TabOffline {
    tab_id: Uuid,
    offline: bool,
}

fn unreported_loads() -> MutexGuard<'static, HashMap<Uuid, u64>> {
    static LOADS: LazyLock<Mutex<HashMap<Uuid, u64>>> = LazyLock::new(Default::default);
    LOADS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn watch_load(app: &AppHandle, tab_id: Uuid) {
    let load = NEXT_LOAD.fetch_add(1, Ordering::Relaxed);
    unreported_loads().insert(tab_id, load);
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(WATCH_DELAY);
        if unreported_loads().get(&tab_id) != Some(&load) {
            return;
        }
        let Some(base) = engine(&app).state.find_tab(tab_id).map(|(workspace, _)| workspace.base_url.clone()) else {
            return;
        };
        if is_reachable(&base) {
            return;
        }
        let effects = engine(&app).set_offline(tab_id);
        if effects.is_empty() {
            return;
        }
        run(&app, effects);
        let _ = app.emit_to("shell", "tab-offline", TabOffline { tab_id, offline: true });
        while engine(&app).is_offline(tab_id) {
            thread::sleep(RETRY_INTERVAL);
            if is_reachable(&base) {
                retry(&app, tab_id);
            }
        }
    });
}

pub fn page_reported(tab_id: Uuid) {
    unreported_loads().remove(&tab_id);
}

pub fn retry(app: &AppHandle, tab_id: Uuid) {
    let effects = engine(app).retry_tab(tab_id);
    if effects.is_empty() {
        return;
    }
    let _ = app.emit_to("shell", "tab-offline", TabOffline { tab_id, offline: false });
    run(app, effects);
}

fn is_reachable(base: &Url) -> bool {
    http::client().get(urls::join(base, "status.php").as_str()).call().is_ok()
}
