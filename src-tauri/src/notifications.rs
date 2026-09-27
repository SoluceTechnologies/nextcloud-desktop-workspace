use crate::keychain::{self, Credentials};
use crate::runtime::{engine, notice, run};
use crate::{auth, http, urls};
use serde::Deserialize;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use url::Url;
use uuid::Uuid;

const FIRST_POLL_DELAY: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_secs(60);
const MAX_BANNERS: usize = 3;

#[derive(Deserialize)]
struct OcsResponse<T> {
    ocs: OcsBody<T>,
}

#[derive(Deserialize)]
struct OcsBody<T> {
    data: T,
}

#[derive(Deserialize)]
struct Notification {
    notification_id: u64,
    subject: String,
}

enum Fetched {
    Notifications(Vec<Notification>),
    Rejected,
    Unavailable,
}

struct SignedInWorkspace {
    id: Uuid,
    name: String,
    base: Url,
}

pub fn spawn_poller(app: AppHandle) {
    thread::spawn(move || {
        let mut last_seen: HashMap<Uuid, u64> = HashMap::new();
        thread::sleep(FIRST_POLL_DELAY);
        loop {
            poll(&app, &mut last_seen);
            thread::sleep(POLL_INTERVAL);
        }
    });
}

fn poll(app: &AppHandle, last_seen: &mut HashMap<Uuid, u64>) {
    let window_focused = app.get_window("main").and_then(|window| window.is_focused().ok()).unwrap_or(false);
    let mut unread: HashMap<Uuid, usize> = HashMap::new();
    for workspace in signed_in_workspaces(app) {
        let Some(credentials) = keychain::load(workspace.id) else { continue };
        match fetch(&workspace.base, &credentials) {
            Fetched::Notifications(notifications) => {
                if !window_focused {
                    for notification in fresh(&notifications, last_seen.get(&workspace.id).copied()) {
                        show_banner(app, &workspace.name, &notification.subject);
                    }
                }
                let newest = notifications.iter().map(|notification| notification.notification_id).max().unwrap_or(0);
                let seen = last_seen.entry(workspace.id).or_insert(newest);
                *seen = (*seen).max(newest);
                unread.insert(workspace.id, notifications.len());
            }
            Fetched::Rejected => {
                auth::sign_out(app, workspace.id, false);
                let effects = engine(app).reopen_tabs(workspace.id);
                run(app, effects);
                notice(
                    app,
                    format!("{}: the app password was revoked. Sign in again from the workspace menu.", workspace.name),
                );
            }
            Fetched::Unavailable => {}
        }
    }
    let total: usize = unread.values().sum();
    let _ = app.emit_to("shell", "unread", &unread);
    if let Some(window) = app.get_window("main") {
        let _ = window.set_badge_count((total > 0).then_some(total as i64));
    }
}

fn signed_in_workspaces(app: &AppHandle) -> Vec<SignedInWorkspace> {
    engine(app)
        .state
        .workspaces
        .iter()
        .filter(|workspace| workspace.login.is_some())
        .map(|workspace| SignedInWorkspace {
            id: workspace.id,
            name: workspace.name.clone(),
            base: workspace.base_url.clone(),
        })
        .collect()
}

fn fetch(base: &Url, credentials: &Credentials) -> Fetched {
    let mut url = urls::join(base, "ocs/v2.php/apps/notifications/api/v2/notifications");
    url.set_query(Some("format=json"));
    let response = http::client()
        .get(url.as_str())
        .header("Authorization", credentials.authorization())
        .header("OCS-APIRequest", "true")
        .header("Accept", "application/json")
        .call();
    match response {
        Ok(response) if response.status() == 401 => Fetched::Rejected,
        Ok(mut response) if response.status() == 200 => {
            match response.body_mut().read_json::<OcsResponse<Vec<Notification>>>() {
                Ok(body) => Fetched::Notifications(body.ocs.data),
                Err(_) => Fetched::Unavailable,
            }
        }
        _ => Fetched::Unavailable,
    }
}

fn fresh(notifications: &[Notification], last_seen: Option<u64>) -> Vec<&Notification> {
    let Some(last_seen) = last_seen else { return Vec::new() };
    let mut fresh: Vec<&Notification> =
        notifications.iter().filter(|notification| notification.notification_id > last_seen).collect();
    fresh.sort_by_key(|notification| Reverse(notification.notification_id));
    fresh.truncate(MAX_BANNERS);
    fresh
}

fn show_banner(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

#[cfg(test)]
#[path = "../tests/unit/notifications.rs"]
mod tests;
