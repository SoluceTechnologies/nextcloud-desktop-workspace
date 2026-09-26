//! Every state mutation (spec §5–§7). Pure: ops return `Effect`s for the webview executor
//! (`webviews.rs`) and never touch Tauri, so the lock is never held during webview calls.

use crate::model::{AppEntry, AppState, Tab, Workspace, AUTH};
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

    pub fn is_live(&self, tab: Uuid) -> bool {
        self.live.contains(&tab)
    }

    /// Creates one cold tab in the background (hidden) so that switching to it is instant: the active
    /// workspace's tabs in bar order, then each other workspace's selected tab. Nothing once the live
    /// budget is full. Preloaded tabs join the LRU as least recent, so they are evicted first.
    pub fn preload_next(&mut self) -> Vec<Effect> {
        if self.live.len() >= self.max_live {
            return Vec::new();
        }
        let active = self.state.active_workspace_id;
        let active_tabs = self.state.workspaces.iter().filter(|w| Some(w.id) == active).flat_map(|w| w.tabs.iter().map(move |t| (w.id, t)));
        let other_selected = self
            .state
            .workspaces
            .iter()
            .filter(|w| Some(w.id) != active)
            .filter_map(|w| Some((w.id, w.tab(w.active_tab_id?)?)));
        let next = active_tabs.chain(other_selected).find(|(_, t)| !self.live.contains(&t.id));
        let Some((ws, tab, url)) = next.map(|(ws, t)| (ws, t.id, t.url.clone())) else { return Vec::new() };
        self.live.insert(0, tab);
        vec![Effect::Create { ws, tab, url }]
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
        if let Some(i) = icon.filter(|i| i.len() <= MAX_ICON && i.starts_with("data:image/")) {
            w.icon = Some(i);
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
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn engine_with(bases: &[&str], max_live: usize) -> Engine {
        let mut e = Engine::new(AppState::default(), max_live);
        for b in bases {
            e.add_workspace(b).unwrap();
        }
        e
    }

    fn ws(e: &Engine, i: usize) -> Uuid {
        e.state.workspaces[i].id
    }

    fn tab_of_app(e: &Engine, i: usize, app: &str) -> Uuid {
        e.state.workspaces[i].tab_by_app(app).unwrap().id
    }

    fn shown(fx: &[Effect]) -> Option<Uuid> {
        fx.iter().rev().find_map(|f| match f {
            Effect::Show { tab, .. } => Some(*tab),
            _ => None,
        })
    }

    fn created(fx: &[Effect]) -> Vec<Uuid> {
        fx.iter().filter_map(|f| match f { Effect::Create { tab, .. } => Some(*tab), _ => None }).collect()
    }

    fn destroyed(fx: &[Effect]) -> Vec<Uuid> {
        fx.iter().filter_map(|f| match f { Effect::Destroy { tab, .. } => Some(*tab), _ => None }).collect()
    }

    #[test]
    fn add_workspace_creates_and_shows_auth_tab() {
        let mut e = Engine::new(AppState::default(), MAX_LIVE);
        let fx = e.add_workspace("cloud.soluce.com").unwrap();
        let w = &e.state.workspaces[0];
        assert_eq!(w.base_url, u("https://cloud.soluce.com/"));
        assert_eq!(e.state.active_workspace_id, Some(w.id));
        let tab = w.tabs[0].id;
        assert_eq!(created(&fx), vec![tab]);
        assert_eq!(shown(&fx), Some(tab));
        assert_eq!(fx.last(), Some(&Effect::Changed));
    }

    #[test]
    fn add_existing_url_activates_instead_of_duplicating() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        e.add_workspace(" https://A.com/index.php/apps/files ").unwrap();
        assert_eq!(e.state.workspaces.len(), 2);
        assert_eq!(e.state.active_workspace_id, Some(ws(&e, 0)));
    }

    #[test]
    fn add_invalid_url_is_an_error() {
        let mut e = Engine::new(AppState::default(), MAX_LIVE);
        assert!(e.add_workspace("ftp://x.com").is_err());
        assert!(e.state.workspaces.is_empty());
    }

    #[test]
    fn startup_creates_only_the_active_tab_and_repairs_selection() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let a = ws(&e, 0);
        e.open_app(a, "files", u("https://a.com/apps/files/"), true);
        let files = tab_of_app(&e, 0, "files");
        let mut fresh = Engine::new(e.state.clone(), MAX_LIVE);
        assert_eq!(created(&fresh.startup()), vec![files]);
        fresh.state.active_workspace_id = None;
        let mut again = Engine::new(fresh.state.clone(), MAX_LIVE);
        again.startup();
        assert_eq!(again.state.active_workspace_id, Some(a));
    }

    #[test]
    fn open_app_creates_tab_once_then_reuses_it() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        let fx = e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        let files = tab_of_app(&e, 0, "files");
        assert_eq!(created(&fx), vec![files]);
        assert_eq!(shown(&fx), Some(files));
        let deep = u("https://a.com/apps/files/?dir=/Photos");
        let fx = e.open_app(w, "files", deep.clone(), true);
        assert!(created(&fx).is_empty());
        assert!(fx.contains(&Effect::Navigate { ws: w, tab: files, url: deep }));
        assert_eq!(e.state.workspaces[0].tabs.len(), 2);
    }

    #[test]
    fn open_app_from_picker_does_not_navigate_existing_tab() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        e.open_app(w, "files", u("https://a.com/apps/files/?dir=/Photos"), true);
        let fx = e.open_app(w, "files", u("https://a.com/apps/files/"), false);
        assert!(!fx.iter().any(|f| matches!(f, Effect::Navigate { .. })));
    }

    #[test]
    fn open_app_names_tab_from_app_cache() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        e.state.workspaces[0].apps.push(AppEntry { id: "spreed".into(), name: "Talk".into(), href: u("https://a.com/apps/spreed/") });
        e.open_app(ws(&e, 0), "spreed", u("https://a.com/apps/spreed/"), true);
        assert_eq!(e.state.workspaces[0].tab_by_app("spreed").unwrap().title, "Talk");
    }

    #[test]
    fn switching_workspaces_restores_each_selected_tab_without_reload() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let (a, b) = (ws(&e, 0), ws(&e, 1));
        e.open_app(a, "calendar", u("https://a.com/apps/calendar/"), true);
        let cal = tab_of_app(&e, 0, "calendar");
        e.open_app(b, "deck", u("https://b.com/apps/deck/"), true);
        let deck = tab_of_app(&e, 1, "deck");
        assert_eq!(shown(&e.activate_workspace(a)), Some(cal));
        let fx = e.activate_workspace(b);
        assert_eq!(shown(&fx), Some(deck));
        assert!(created(&fx).is_empty());
    }

    #[test]
    fn closing_selected_tab_selects_right_then_left_neighbour() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        let auth = e.state.workspaces[0].tabs[0].id;
        e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
        let (files, deck) = (tab_of_app(&e, 0, "files"), tab_of_app(&e, 0, "deck"));
        e.activate_tab(w, files);
        let fx = e.close_tab(w, files);
        assert_eq!(destroyed(&fx), vec![files]);
        assert_eq!(shown(&fx), Some(deck));
        assert_eq!(shown(&e.close_tab(w, deck)), Some(auth));
        let fx = e.close_tab(w, auth);
        assert!(fx.contains(&Effect::HideContent));
        assert_eq!(e.state.workspaces[0].active_tab_id, None);
    }

    #[test]
    fn lru_evicts_least_recent_unpinned_tab_and_recreates_on_return() {
        let mut e = engine_with(&["https://a.com"], 2);
        let w = ws(&e, 0);
        let auth = e.state.workspaces[0].tabs[0].id;
        e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        let fx = e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
        assert_eq!(destroyed(&fx), vec![auth]);
        assert!(!e.is_live(auth));
        assert_eq!(created(&e.activate_tab(w, auth)), vec![auth]);
    }

    #[test]
    fn preload_warms_active_workspace_then_other_selected_tabs_and_evicts_them_first() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let a = ws(&e, 0);
        e.open_app(a, "files", u("https://a.com/apps/files/"), true);
        e.open_app(a, "deck", u("https://a.com/apps/deck/"), true);
        // Restart: only the selected tab (deck in a) is live.
        let mut e = Engine::new(e.state.clone(), 4);
        e.startup();
        let (auth_a, files) = (e.state.workspaces[0].tabs[0].id, tab_of_app(&e, 0, "files"));
        let b_selected = e.state.workspaces[1].active_tab_id.unwrap();
        assert_eq!(created(&e.preload_next()), vec![auth_a]);
        assert_eq!(created(&e.preload_next()), vec![files]);
        assert_eq!(created(&e.preload_next()), vec![b_selected]);
        assert!(e.preload_next().is_empty(), "live budget is full");
        // A tab the user opens pushes out a preloaded one, not the visited deck tab.
        let fx = e.open_app(a, "spreed", u("https://a.com/apps/spreed/"), true);
        assert_eq!(destroyed(&fx), vec![b_selected]);
        assert!(e.is_live(tab_of_app(&e, 0, "deck")));
    }

    #[test]
    fn lru_skips_pinned_tabs() {
        let mut e = engine_with(&["https://a.com"], 2);
        let w = ws(&e, 0);
        let auth = e.state.workspaces[0].tabs[0].id;
        e.set_pinned(w, auth, true);
        e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        let files = tab_of_app(&e, 0, "files");
        let fx = e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
        assert_eq!(destroyed(&fx), vec![files]);
        assert!(e.is_live(auth));
    }

    #[test]
    fn overlay_hides_content_and_restores_it() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        let fx = e.set_overlay(true);
        assert!(fx.contains(&Effect::HideContent));
        assert_eq!(shown(&fx), None);
        assert_eq!(shown(&e.set_overlay(false)), Some(tab));
    }

    #[test]
    fn remove_workspace_destroys_tabs_clears_profile_and_selects_neighbour() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let (a, b) = (ws(&e, 0), ws(&e, 1));
        let b_tab = e.state.workspaces[1].tabs[0].id;
        let fx = e.remove_workspace(b);
        assert_eq!(destroyed(&fx), vec![b_tab]);
        assert!(fx.contains(&Effect::ClearProfile { ws: b, delete: true }));
        assert_eq!(e.state.active_workspace_id, Some(a));
        let fx = e.remove_workspace(a);
        assert_eq!(e.state.active_workspace_id, None);
        assert!(fx.contains(&Effect::HideContent));
    }

    #[test]
    fn reorder_requires_a_permutation() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let (a, b) = (ws(&e, 0), ws(&e, 1));
        assert!(e.reorder_workspaces(&[a]).is_empty());
        assert!(e.reorder_workspaces(&[a, a]).is_empty());
        assert_eq!(e.reorder_workspaces(&[b, a]), vec![Effect::Changed]);
        assert_eq!(ws(&e, 0), b);
    }

    #[test]
    fn rename_sets_custom_name_and_empty_resets_to_host() {
        let mut e = engine_with(&["https://cloud.a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        e.rename_workspace(w, "  Soluce  ");
        assert_eq!((e.state.workspaces[0].name.as_str(), e.state.workspaces[0].name_custom), ("Soluce", true));
        e.rename_workspace(w, " ");
        assert_eq!((e.state.workspaces[0].name.as_str(), e.state.workspaces[0].name_custom), ("cloud.a.com", false));
    }

    #[test]
    fn clear_browsing_data_reloads_live_tabs_only() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        let tab = e.state.workspaces[0].tabs[0].id;
        e.state.workspaces[0].tabs.push(Tab::new("deck", "Deck", u("https://a.com/apps/deck/")));
        assert_eq!(
            e.clear_browsing_data(w),
            vec![Effect::ClearProfile { ws: w, delete: false }, Effect::Reload { ws: w, tab }]
        );
    }

    #[test]
    fn auth_tab_adopts_first_real_app_after_login() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        e.observe_location(tab, u("https://a.com/login?redirect_url=/"));
        assert_eq!(e.state.workspaces[0].tabs[0].app_id, AUTH);
        e.observe_location(tab, u("https://a.com/apps/dashboard/"));
        let t = &e.state.workspaces[0].tabs[0];
        assert_eq!((t.app_id.as_str(), t.url.as_str()), ("dashboard", "https://a.com/apps/dashboard/"));
    }

    #[test]
    fn auth_tab_landing_on_open_app_merges_into_it() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        let files = tab_of_app(&e, 0, "files");
        let auth = created(&e.open_home(w))[0];
        let docs = u("https://a.com/apps/files/?dir=/Docs");
        let fx = e.observe_location(auth, docs.clone());
        assert!(destroyed(&fx).contains(&auth));
        assert!(e.state.workspaces[0].tab(auth).is_none());
        assert_eq!(shown(&fx), Some(files));
        assert!(fx.contains(&Effect::Navigate { ws: w, tab: files, url: docs }));
    }

    #[test]
    fn page_of_other_app_retags_tab_unless_that_app_is_open() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
        let files = tab_of_app(&e, 0, "files");
        e.observe_location(files, u("https://a.com/apps/deck/#/board/1"));
        assert_eq!(e.state.workspaces[0].tab(files).unwrap().app_id, "files", "deck open elsewhere: tolerated duplicate");
        e.observe_location(files, u("https://a.com/apps/calendar/"));
        assert_eq!(e.state.workspaces[0].tab(files).unwrap().app_id, "calendar");
    }

    #[test]
    fn location_report_updates_url_and_ignores_foreign_urls() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        e.open_app(w, "files", u("https://a.com/apps/files/"), true);
        let files = tab_of_app(&e, 0, "files");
        let photos = u("https://a.com/apps/files/?dir=/Photos");
        e.observe_location(files, photos.clone());
        assert_eq!(e.state.workspaces[0].tab(files).unwrap().url, photos);
        assert!(e.observe_location(files, u("https://idp.example.com/auth")).is_empty());
        assert_eq!(e.state.workspaces[0].tab(files).unwrap().url, photos);
    }

    #[test]
    fn title_sets_tab_title_and_workspace_name_unless_custom() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        e.observe_title(tab, "Documents - Files - Soluce Cloud");
        assert_eq!(e.state.workspaces[0].tabs[0].title, "Documents - Files");
        assert_eq!(e.state.workspaces[0].name, "Soluce Cloud");
        e.observe_title(tab, "Files – Nextcloud");
        assert_eq!(e.state.workspaces[0].name, "Soluce Cloud", "default instance name ignored");
        e.rename_workspace(ws(&e, 0), "Mine");
        e.observe_title(tab, "Files - Other");
        assert_eq!(e.state.workspaces[0].name, "Mine");
    }

    #[test]
    fn meta_keeps_only_safe_icon_and_workspace_app_links() {
        let mut e = engine_with(&["https://a.com/nc"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        let link = |name: &str, href: &str| AppLink { name: name.into(), href: href.into() };
        e.observe_meta(
            tab,
            Some("javascript:alert(1)".into()),
            vec![
                link("Files", "https://a.com/nc/apps/files/"),
                link("Files again", "https://a.com/nc/index.php/apps/files/"),
                link("Talk", "https://a.com/nc/apps/spreed/"),
                link("Evil", "https://evil.com/apps/x/"),
                link("OIDC", "https://a.com/nc/apps/user_oidc/"),
                link("", "https://a.com/nc/apps/deck/"),
            ],
        );
        let w = &e.state.workspaces[0];
        assert_eq!(w.icon, None);
        assert_eq!(w.apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["files", "spreed"]);
        e.observe_meta(tab, Some("data:image/png;base64,AAAA".into()), vec![]);
        let w = &e.state.workspaces[0];
        assert_eq!(w.icon.as_deref(), Some("data:image/png;base64,AAAA"));
        assert_eq!(w.apps.len(), 2, "an empty report keeps the cache");
    }

    #[test]
    fn new_window_to_other_workspace_switches_and_opens_app() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let (a, b) = (ws(&e, 0), ws(&e, 1));
        e.activate_workspace(a);
        let board = u("https://b.com/apps/deck/#/board/5");
        let fx = e.on_new_window(&board);
        let deck = tab_of_app(&e, 1, "deck");
        assert_eq!(e.state.active_workspace_id, Some(b));
        assert_eq!(shown(&fx), Some(deck));
        assert_eq!(e.state.workspaces[1].tab(deck).unwrap().url, board);
        let gh = u("https://github.com/x");
        assert_eq!(e.on_new_window(&gh), vec![Effect::OpenExternal(gh)]);
    }

    #[test]
    fn new_window_links_give_documents_their_own_tabs_and_reuse_app_homes() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let selected = |e: &Engine| e.state.workspaces[0].active_tab_id.unwrap();
        let doc1 = u("https://a.com/apps/eurooffice/1?filePath=%2Fa.docx");
        e.on_new_window(&doc1);
        let t1 = selected(&e);
        e.on_new_window(&u("https://a.com/apps/eurooffice/2?filePath=%2Fb.docx"));
        let t2 = selected(&e);
        assert_ne!(t1, t2, "a second document of the same app gets its own tab");
        let fx = e.on_new_window(&doc1);
        assert_eq!(shown(&fx), Some(t1), "the same document selects its tab");
        assert!(created(&fx).is_empty());
        // An app home (app menu link) selects the app's tab and keeps its page.
        e.on_new_window(&u("https://a.com/apps/files/?dir=/Photos"));
        let files = selected(&e);
        e.activate_tab(ws(&e, 0), t1);
        let fx = e.on_new_window(&u("https://a.com/apps/files/"));
        assert_eq!(shown(&fx), Some(files));
        assert!(!fx.iter().any(|f| matches!(f, Effect::Navigate { .. })));
        assert_eq!(e.state.workspaces[0].tabs.len(), 4, "auth + 2 documents + files");
    }

    #[test]
    fn menu_opens_cached_app_and_home_opens_new_auth_tab() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let w = ws(&e, 0);
        assert!(e.open_app_from_menu(w, "deck").is_empty(), "unknown app");
        e.state.workspaces[0].apps.push(AppEntry { id: "deck".into(), name: "Deck".into(), href: u("https://a.com/apps/deck/") });
        e.open_app_from_menu(w, "deck");
        assert_eq!(e.state.workspaces[0].tab_by_app("deck").unwrap().title, "Deck");
        e.open_home(w);
        assert_eq!(e.state.workspaces[0].tabs.iter().filter(|t| t.app_id == AUTH).count(), 2);
    }

    #[test]
    fn reload_home_navigates_then_reloads() {
        let mut e = engine_with(&["https://a.com/nc"], MAX_LIVE);
        let w = ws(&e, 0);
        e.open_app(w, "files", u("https://a.com/nc/apps/files/?dir=/x"), true);
        let files = tab_of_app(&e, 0, "files");
        let fx = e.reload_home(w, files);
        assert!(fx.contains(&Effect::Navigate { ws: w, tab: files, url: u("https://a.com/nc/apps/files/") }));
        assert!(e.reload_home(w, files).contains(&Effect::Reload { ws: w, tab: files }));
    }

    #[test]
    fn location_merge_in_background_workspace_updates_that_workspaces_selection_only() {
        let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
        let (a, b) = (ws(&e, 0), ws(&e, 1));
        e.open_app(a, "files", u("https://a.com/apps/files/"), true);
        let files = tab_of_app(&e, 0, "files");
        let auth = created(&e.open_home(a))[0];
        e.activate_workspace(b);
        let docs = u("https://a.com/apps/files/?dir=/Docs");
        let fx = e.observe_location(auth, docs);
        assert_eq!(e.state.workspaces[0].active_tab_id, Some(files), "A's own selection moves to the merge target");
        assert_eq!(e.state.active_workspace_id, Some(b), "background report never switches the shown workspace");
        assert!(e.state.workspaces[0].tab(auth).is_none());
        assert!(destroyed(&fx).contains(&auth));
        let b_tab = e.state.workspaces[1].tabs[0].id;
        assert_eq!(shown(&fx), Some(b_tab), "B, not A, stays on screen");
    }

    #[test]
    fn meta_icon_size_boundary_enforced_and_previous_kept_on_reject() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        let prefix = "data:image/png;base64,";
        let ok = format!("{prefix}{}", "A".repeat(MAX_ICON - prefix.len()));
        assert_eq!(ok.len(), MAX_ICON);
        e.observe_meta(tab, Some(ok.clone()), vec![]);
        assert_eq!(e.state.workspaces[0].icon.as_deref(), Some(ok.as_str()));
        let too_big = format!("{ok}A");
        e.observe_meta(tab, Some(too_big), vec![]);
        assert_eq!(e.state.workspaces[0].icon.as_deref(), Some(ok.as_str()), "oversized icon rejected, previous kept");
    }

    #[test]
    fn meta_caps_app_links_to_max_apps_in_input_order() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        let links: Vec<AppLink> = (0..70)
            .map(|i| AppLink { name: format!("App {i}"), href: format!("https://a.com/apps/app{i}/") })
            .collect();
        e.observe_meta(tab, None, links);
        let w = &e.state.workspaces[0];
        assert_eq!(w.apps.len(), MAX_APPS);
        let want: Vec<String> = (0..MAX_APPS).map(|i| format!("app{i}")).collect();
        assert_eq!(w.apps.iter().map(|a| a.id.clone()).collect::<Vec<_>>(), want);
    }

    #[test]
    fn title_page_part_truncated_to_max_title() {
        let mut e = engine_with(&["https://a.com"], MAX_LIVE);
        let tab = e.state.workspaces[0].tabs[0].id;
        let long_page = "x".repeat(300);
        e.observe_title(tab, &format!("{long_page} - Some Cloud"));
        let title = &e.state.workspaces[0].tabs[0].title;
        assert_eq!(title.chars().count(), MAX_TITLE);
        assert_eq!(title, &"x".repeat(MAX_TITLE));
    }
}
