//! Every state mutation (spec §5–§7). Pure: ops return `Effect`s for the webview executor
//! (`webviews.rs`) and never touch Tauri, so the lock is never held during webview calls.

use crate::model::{AppState, Tab, Workspace, AUTH};
use crate::urls;
use url::Url;
use uuid::Uuid;

pub const MAX_LIVE: usize = 8;

pub type Shared = std::sync::Mutex<Engine>;

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
            None => {
                let w = self.state.ws_mut(ws).expect("checked above");
                let t = Tab::new(app_id, &w.app_name(app_id), url);
                let id = t.id;
                w.tabs.push(t);
                id
            }
        };
        self.select(ws, tab);
        self.presented(fx)
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
    use crate::model::AppEntry;

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
}
