//! Every state mutation (spec §5–§7). Pure: ops return `Effect`s for the webview executor
//! (`webviews.rs`) and never touch Tauri, so the lock is never held during webview calls.

use crate::model::{AppEntry, AppState, Appearance, Tab, Workspace, AUTH};
use crate::router::{self, Route};
use crate::urls;
use std::collections::HashSet;
use url::Url;
use uuid::Uuid;

pub const MAX_LIVE: usize = 8;

pub type Shared = std::sync::Mutex<Engine>;

const MAX_TITLE: usize = 256;
const MAX_ICON: usize = 65_536;
const MAX_APPS: usize = 64;

/// App menu link as reported by the bridge (untrusted input).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppLink {
    pub name: String,
    pub href: String,
}

/// Side effects for the webview executor, applied in order on one worker thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Create the tab's webview, hidden, at `url`.
    Create { ws: Uuid, tab: Uuid, url: Url },
    Navigate { ws: Uuid, tab: Uuid, url: Url },
    Reload { ws: Uuid, tab: Uuid },
    Destroy { ws: Uuid, tab: Uuid },
    /// Show this tab's webview and hide every other content webview.
    Show { ws: Uuid, tab: Uuid },
    HideContent,
    OpenExternal(Url),
    /// Clear the workspace profile's browsing data; `delete` also removes the profile directory.
    ClearProfile { ws: Uuid, delete: bool },
    /// Apply the window appearance (light, dark or follow the system).
    Theme(Appearance),
    /// State changed: emit a snapshot to the shell and save to disk.
    Changed,
}

pub struct Engine {
    pub state: AppState,
    /// Tabs with a live webview, least recently shown first.
    live: Vec<Uuid>,
    overlay: bool,
    max_live: usize,
}

impl Engine {
    pub fn new(state: AppState, max_live: usize) -> Self {
        Self { state, live: Vec::new(), overlay: false, max_live }
    }

    /// Icon chosen in the app; `None` goes back to the server's (from the next page report).
    pub fn set_icon(&mut self, ws: Uuid, icon: Option<String>) -> Vec<Effect> {
        let Some(w) = self.state.ws_mut(ws) else { return Vec::new() };
        match icon {
            Some(i) if !valid_icon(&i) => return Vec::new(),
            Some(i) => (w.icon, w.icon_custom) = (Some(i), true),
            None => (w.icon, w.icon_custom) = (None, false),
        }
        vec![Effect::Changed]
    }

    pub fn set_theme(&mut self, theme: Appearance) -> Vec<Effect> {
        self.state.theme = theme;
        vec![Effect::Theme(theme), Effect::Changed]
    }

    pub fn is_live(&self, tab: Uuid) -> bool {
        self.live.contains(&tab)
    }

    /// First presentation after launch: only the selected tab of the selected workspace gets a webview.
    pub fn startup(&mut self) -> Vec<Effect> {
        if self.state.active_workspace_id.and_then(|id| self.state.ws(id)).is_none() {
            self.state.active_workspace_id = self.state.workspaces.first().map(|w| w.id);
        }
        self.presented(Vec::new())
    }

    pub fn add_workspace(&mut self, input: &str) -> Result<Vec<Effect>, String> {
        let base = urls::normalize(input)?;
        let id = match self.state.workspaces.iter().find(|w| w.base_url == base) {
            Some(w) => w.id,
            None => {
                let w = Workspace::new(base);
                let id = w.id;
                self.state.workspaces.push(w);
                id
            }
        };
        Ok(self.activate_workspace(id))
    }

    pub fn remove_workspace(&mut self, ws: Uuid) -> Vec<Effect> {
        let Some(idx) = self.state.workspaces.iter().position(|w| w.id == ws) else { return Vec::new() };
        let removed = self.state.workspaces.remove(idx);
        let mut fx = Vec::new();
        for t in &removed.tabs {
            self.kill(ws, t.id, &mut fx);
        }
        fx.push(Effect::ClearProfile { ws, delete: true });
        if self.state.active_workspace_id == Some(ws) {
            let n = self.state.workspaces.len();
            self.state.active_workspace_id = (n > 0).then(|| self.state.workspaces[idx.min(n - 1)].id);
        }
        self.presented(fx)
    }

    /// Empty name = back to automatic naming (host, then page title).
    pub fn rename_workspace(&mut self, ws: Uuid, name: &str) -> Vec<Effect> {
        let Some(w) = self.state.ws_mut(ws) else { return Vec::new() };
        let name = name.trim();
        if name.is_empty() {
            w.name = w.base_url.host_str().unwrap_or_default().to_string();
            w.name_custom = false;
        } else {
            w.name = truncate(name, 64);
            w.name_custom = true;
        }
        vec![Effect::Changed]
    }

    pub fn reorder_workspaces(&mut self, ids: &[Uuid]) -> Vec<Effect> {
        if reorder(&mut self.state.workspaces, ids, |w| w.id) { vec![Effect::Changed] } else { Vec::new() }
    }

    pub fn activate_workspace(&mut self, ws: Uuid) -> Vec<Effect> {
        if self.state.ws(ws).is_none() {
            return Vec::new();
        }
        self.state.active_workspace_id = Some(ws);
        self.presented(Vec::new())
    }

    pub fn activate_tab(&mut self, ws: Uuid, tab: Uuid) -> Vec<Effect> {
        if self.state.ws(ws).and_then(|w| w.tab(tab)).is_none() {
            return Vec::new();
        }
        self.select(ws, tab);
        self.presented(Vec::new())
    }

    /// Selects the tab of `app_id` in `ws`, creating it if missing. `AUTH` always opens a new tab.
    /// `navigate_existing`: routed links move an existing tab to `url`; the app picker does not.
    pub fn open_app(&mut self, ws: Uuid, app_id: &str, url: Url, navigate_existing: bool) -> Vec<Effect> {
        let mut fx = Vec::new();
        let Some(w) = self.state.ws(ws) else { return fx };
        let existing = if app_id == AUTH { None } else { w.tab_by_app(app_id).map(|t| t.id) };
        let tab = match existing {
            Some(t) => {
                if navigate_existing {
                    self.navigate_tab(ws, t, url, &mut fx);
                }
                t
            }
            None => self.push_tab(ws, app_id, url),
        };
        self.select(ws, tab);
        self.presented(fx)
    }

    /// A link opened as a new window (`window.open`, `target=_blank`, cross-app click): a tab already at
    /// that URL is selected; an app home (app menu link) selects the app's tab without reloading it;
    /// any other link, such as a document, gets a tab of its own, so one app can have several tabs.
    fn open_link(&mut self, ws: Uuid, app_id: &str, url: Url) -> Vec<Effect> {
        let Some(w) = self.state.ws(ws) else { return Vec::new() };
        if let Some(t) = w.tabs.iter().find(|t| t.url == url).map(|t| t.id) {
            self.select(ws, t);
            return self.presented(Vec::new());
        }
        let home = url == router::home_url(&w.base_url, app_id) || w.apps.iter().any(|a| a.href == url);
        if home {
            return self.open_app(ws, app_id, url, false);
        }
        let tab = self.push_tab(ws, app_id, url);
        self.select(ws, tab);
        self.presented(Vec::new())
    }

    fn push_tab(&mut self, ws: Uuid, app_id: &str, url: Url) -> Uuid {
        let w = self.state.ws_mut(ws).expect("caller checked the workspace");
        let t = Tab::new(app_id, &w.app_name(app_id), url);
        let id = t.id;
        w.tabs.push(t);
        id
    }

    pub fn close_tab(&mut self, ws: Uuid, tab: Uuid) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.remove_tab(ws, tab, &mut fx);
        self.presented(fx)
    }

    pub fn reorder_tabs(&mut self, ws: Uuid, ids: &[Uuid]) -> Vec<Effect> {
        let ok = self.state.ws_mut(ws).is_some_and(|w| reorder(&mut w.tabs, ids, |t| t.id));
        if ok { vec![Effect::Changed] } else { Vec::new() }
    }

    pub fn set_pinned(&mut self, ws: Uuid, tab: Uuid, pinned: bool) -> Vec<Effect> {
        match self.state.ws_mut(ws).and_then(|w| w.tab_mut(tab)) {
            Some(t) => {
                t.pinned = pinned;
                vec![Effect::Changed]
            }
            None => Vec::new(),
        }
    }

    /// While a shell dialog is open the content webviews are hidden (they would cover it).
    pub fn set_overlay(&mut self, on: bool) -> Vec<Effect> {
        self.overlay = on;
        self.presented(Vec::new())
    }

    pub fn reload_tab(&mut self, ws: Uuid, tab: Uuid) -> Vec<Effect> {
        if self.is_live(tab) { vec![Effect::Reload { ws, tab }] } else { Vec::new() }
    }

    pub fn reload_workspace(&mut self, ws: Uuid) -> Vec<Effect> {
        match self.state.ws(ws).and_then(|w| w.active_tab_id) {
            Some(tab) => self.reload_tab(ws, tab),
            None => Vec::new(),
        }
    }

    pub fn clear_browsing_data(&mut self, ws: Uuid) -> Vec<Effect> {
        let Some(w) = self.state.ws(ws) else { return Vec::new() };
        let mut fx = vec![Effect::ClearProfile { ws, delete: false }];
        fx.extend(w.tabs.iter().filter(|t| self.is_live(t.id)).map(|t| Effect::Reload { ws, tab: t.id }));
        fx
    }

    /// Main-frame location report (page load or SPA change). Keeps one tab per app (spec §5.4):
    /// AUTH tabs adopt or merge into the app they land on, other tabs are retagged when free.
    pub fn observe_location(&mut self, tab: Uuid, url: Url) -> Vec<Effect> {
        let Some((w, t)) = self.state.find_tab(tab) else { return Vec::new() };
        if !urls::belongs(&url, &w.base_url) {
            return Vec::new();
        }
        let ws = w.id;
        let current = t.app_id.clone();
        let page = router::app_id(&url, &w.base_url);
        let other = w.tab_by_app(&page).map(|o| o.id).filter(|&o| o != tab && page != AUTH);
        let was_active_tab = w.active_tab_id == Some(tab);

        if current == AUTH && page != AUTH {
            if let Some(o) = other {
                let mut fx = Vec::new();
                self.navigate_tab(ws, o, url, &mut fx);
                if was_active_tab {
                    // Only hand this workspace's own selection to `o`; never change which
                    // workspace is globally shown from a background page report.
                    if let Some(w) = self.state.ws_mut(ws) {
                        w.active_tab_id = Some(o);
                    }
                }
                self.remove_tab(ws, tab, &mut fx);
                return self.presented(fx);
            }
        }
        let t = self.state.ws_mut(ws).and_then(|w| w.tab_mut(tab)).expect("found above");
        if page != AUTH && page != current && other.is_none() {
            t.app_id = page;
        }
        t.url = url;
        vec![Effect::Changed]
    }

    /// Native document-title change: tab title = page part; workspace name = instance part unless user-named.
    pub fn observe_title(&mut self, tab: Uuid, title: &str) -> Vec<Effect> {
        let (page, instance) = split_title(title);
        let Some(w) = self.state.ws_of_tab_mut(tab) else { return Vec::new() };
        if !w.name_custom {
            if let Some(i) = instance {
                w.name = truncate(i, 64);
            }
        }
        if !page.is_empty() {
            if let Some(t) = w.tab_mut(tab) {
                t.title = truncate(page, MAX_TITLE);
            }
        }
        vec![Effect::Changed]
    }

    /// Once-per-page metadata from the bridge. Keeps only data-URL images and in-workspace app links.
    pub fn observe_meta(&mut self, tab: Uuid, icon: Option<String>, apps: Vec<AppLink>) -> Vec<Effect> {
        let Some(w) = self.state.ws_of_tab_mut(tab) else { return Vec::new() };
        // Some("") = the server has no custom favicon or logo (initials); None = unknown, keep.
        match icon {
            _ if w.icon_custom => {}
            Some(i) if i.is_empty() => w.icon = None,
            Some(i) if valid_icon(&i) => w.icon = Some(i),
            _ => {}
        }
        let base = w.base_url.clone();
        let mut seen = HashSet::new();
        let entries: Vec<AppEntry> = apps
            .into_iter()
            .filter_map(|a| {
                let href = Url::parse(&a.href).ok().filter(|h| urls::belongs(h, &base))?;
                let id = router::app_id(&href, &base);
                let name = a.name.trim();
                (id != AUTH && !name.is_empty() && seen.insert(id.clone()))
                    .then(|| AppEntry { id, name: truncate(name, 64), href })
            })
            .take(MAX_APPS)
            .collect();
        if !entries.is_empty() {
            w.apps = entries;
        }
        vec![Effect::Changed]
    }

    pub fn on_new_window(&mut self, url: &Url) -> Vec<Effect> {
        match router::classify_new_window(&self.state, url) {
            Route::Activate { ws, app_id, url } => self.open_link(ws, &app_id, url),
            Route::External(u) => vec![Effect::OpenExternal(u)],
            Route::InPlace | Route::Deny => Vec::new(),
        }
    }

    pub fn open_app_from_menu(&mut self, ws: Uuid, app_id: &str) -> Vec<Effect> {
        let href = self.state.ws(ws).and_then(|w| w.apps.iter().find(|a| a.id == app_id)).map(|a| a.href.clone());
        match href {
            Some(href) => self.open_app(ws, app_id, href, false),
            None => Vec::new(),
        }
    }

    pub fn open_home(&mut self, ws: Uuid) -> Vec<Effect> {
        match self.state.ws(ws).map(|w| w.base_url.clone()) {
            Some(base) => self.open_app(ws, AUTH, base, false),
            None => Vec::new(),
        }
    }

    /// Tab menu "Reload at app home": navigate to the app's landing page, or reload if already there.
    pub fn reload_home(&mut self, ws: Uuid, tab: Uuid) -> Vec<Effect> {
        let Some(home) = self.state.ws(ws).and_then(|w| w.tab(tab).map(|t| router::home_url(&w.base_url, &t.app_id)))
        else {
            return Vec::new();
        };
        let mut fx = Vec::new();
        self.navigate_tab(ws, tab, home, &mut fx);
        if fx.is_empty() {
            fx = self.reload_tab(ws, tab);
        }
        fx.push(Effect::Changed);
        fx
    }

    fn select(&mut self, ws: Uuid, tab: Uuid) {
        if let Some(w) = self.state.ws_mut(ws) {
            w.active_tab_id = Some(tab);
        }
        self.state.active_workspace_id = Some(ws);
    }

    fn navigate_tab(&mut self, ws: Uuid, tab: Uuid, url: Url, fx: &mut Vec<Effect>) {
        let live = self.is_live(tab);
        let Some(t) = self.state.ws_mut(ws).and_then(|w| w.tab_mut(tab)) else { return };
        if t.url == url {
            return;
        }
        t.url = url.clone();
        if live {
            fx.push(Effect::Navigate { ws, tab, url });
        }
    }

    /// Removes the tab; a selected tab hands the selection to its right neighbour, else its left one.
    fn remove_tab(&mut self, ws: Uuid, tab: Uuid, fx: &mut Vec<Effect>) {
        let Some(w) = self.state.ws_mut(ws) else { return };
        let Some(idx) = w.tabs.iter().position(|t| t.id == tab) else { return };
        w.tabs.remove(idx);
        if w.active_tab_id == Some(tab) {
            w.active_tab_id = w.tabs.get(idx).or_else(|| w.tabs.last()).map(|t| t.id);
        }
        self.kill(ws, tab, fx);
    }

    fn kill(&mut self, ws: Uuid, tab: Uuid, fx: &mut Vec<Effect>) {
        if self.is_live(tab) {
            self.live.retain(|&t| t != tab);
            fx.push(Effect::Destroy { ws, tab });
        }
    }

    /// Ends every selection change: selected tab live and shown (or all hidden), LRU enforced, `Changed`.
    fn presented(&mut self, mut fx: Vec<Effect>) -> Vec<Effect> {
        let target = self.state.active_workspace_id.and_then(|ws| {
            let w = self.state.ws(ws)?;
            let t = w.tab(w.active_tab_id?)?;
            Some((ws, t.id, t.url.clone()))
        });
        match target {
            Some((ws, tab, url)) if !self.overlay => {
                if !self.is_live(tab) {
                    fx.push(Effect::Create { ws, tab, url });
                }
                self.live.retain(|&t| t != tab);
                self.live.push(tab);
                self.evict(&mut fx);
                fx.push(Effect::Show { ws, tab });
            }
            _ => fx.push(Effect::HideContent),
        }
        fx.push(Effect::Changed);
        fx
    }

    fn evict(&mut self, fx: &mut Vec<Effect>) {
        while self.live.len() > self.max_live {
            let shown = *self.live.last().expect("just pushed");
            let pinned = |t: Uuid| self.state.find_tab(t).is_some_and(|(_, tab)| tab.pinned);
            let Some(victim) = self.live.iter().copied().find(|&t| t != shown && !pinned(t)) else { break };
            let ws = self.state.find_tab(victim).map(|(w, _)| w.id);
            self.live.retain(|&t| t != victim);
            if let Some(ws) = ws {
                fx.push(Effect::Destroy { ws, tab: victim });
            }
        }
    }
}

fn valid_icon(icon: &str) -> bool {
    icon.len() <= MAX_ICON && icon.starts_with("data:image/")
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// "Documents - Files - Soluce Cloud" → ("Documents - Files", Some("Soluce Cloud")).
/// The default instance name "Nextcloud" says nothing about the workspace, so it yields None.
fn split_title(title: &str) -> (&str, Option<&str>) {
    let t = title.trim();
    let cut = [" - ", " – "].iter().filter_map(|sep| t.rfind(sep).map(|i| (i, sep.len()))).max();
    match cut {
        Some((i, n)) => {
            let instance = t[i + n..].trim();
            (t[..i].trim(), (!instance.is_empty() && instance != "Nextcloud").then_some(instance))
        }
        None => (t, None),
    }
}

/// Reorders `items` to follow `ids`; no-op (false) unless `ids` is a permutation of the items' ids.
fn reorder<T>(items: &mut [T], ids: &[Uuid], key: impl Fn(&T) -> Uuid) -> bool {
    if ids.len() != items.len() || !items.iter().all(|i| ids.contains(&key(i))) {
        return false;
    }
    items.sort_by_key(|i| ids.iter().position(|id| *id == key(i)));
    true
}

#[cfg(test)]
#[path = "../tests/unit/engine.rs"]
mod tests;
