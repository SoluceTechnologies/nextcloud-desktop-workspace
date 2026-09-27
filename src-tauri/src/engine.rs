use crate::model::{AppEntry, AppState, Appearance, Tab, Workspace, AUTH};
use crate::router::{self, Route};
use crate::urls;
use std::collections::HashSet;
use url::Url;
use uuid::Uuid;

pub const MAX_LIVE: usize = 16;

pub type Shared = std::sync::Mutex<Engine>;

const MAX_NAME: usize = 64;
const MAX_TITLE: usize = 256;
const MAX_ICON: usize = 65_536;
const MAX_APPS: usize = 64;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppLink {
    pub name: String,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Create { workspace_id: Uuid, tab_id: Uuid, url: Url },
    Navigate { workspace_id: Uuid, tab_id: Uuid, url: Url },
    Reload { workspace_id: Uuid, tab_id: Uuid },
    Destroy { workspace_id: Uuid, tab_id: Uuid },
    Show { workspace_id: Uuid, tab_id: Uuid },
    HideContent,
    OpenExternal(Url),
    ClearProfile { workspace_id: Uuid, delete: bool },
    Theme(Appearance),
    Changed,
}

pub struct Engine {
    pub state: AppState,
    live: Vec<Uuid>,
    offline: HashSet<Uuid>,
    overlay: bool,
    max_live: usize,
}

impl Engine {
    pub fn new(state: AppState, max_live: usize) -> Self {
        Self { state, live: Vec::new(), offline: HashSet::new(), overlay: false, max_live }
    }

    pub fn is_live(&self, tab_id: Uuid) -> bool {
        self.live.contains(&tab_id)
    }

    pub fn is_offline(&self, tab_id: Uuid) -> bool {
        self.offline.contains(&tab_id)
    }

    pub fn startup(&mut self) -> Vec<Effect> {
        let active_exists = self.state.active_workspace_id.and_then(|id| self.state.workspace(id)).is_some();
        if !active_exists {
            self.state.active_workspace_id = self.state.workspaces.first().map(|workspace| workspace.id);
        }
        self.present(Vec::new())
    }

    pub fn set_theme(&mut self, theme: Appearance) -> Vec<Effect> {
        self.state.theme = theme;
        vec![Effect::Theme(theme), Effect::Changed]
    }

    pub fn set_overlay(&mut self, on: bool) -> Vec<Effect> {
        self.overlay = on;
        self.present(Vec::new())
    }

    pub fn add_workspace(&mut self, input: &str) -> Result<Vec<Effect>, String> {
        let base = urls::normalize(input)?;
        let existing = self.state.workspaces.iter().find(|workspace| workspace.base_url == base);
        let workspace_id = match existing {
            Some(workspace) => workspace.id,
            None => {
                let workspace = Workspace::new(base);
                let workspace_id = workspace.id;
                self.state.workspaces.push(workspace);
                workspace_id
            }
        };
        Ok(self.activate_workspace(workspace_id))
    }

    pub fn remove_workspace(&mut self, workspace_id: Uuid) -> Vec<Effect> {
        let Some(index) = self.state.workspaces.iter().position(|workspace| workspace.id == workspace_id) else {
            return Vec::new();
        };
        let removed = self.state.workspaces.remove(index);
        let mut effects = Vec::new();
        for tab in &removed.tabs {
            self.destroy_webview(workspace_id, tab.id, &mut effects);
        }
        effects.push(Effect::ClearProfile { workspace_id, delete: true });
        if self.state.active_workspace_id == Some(workspace_id) {
            let remaining = self.state.workspaces.len();
            self.state.active_workspace_id =
                (remaining > 0).then(|| self.state.workspaces[index.min(remaining - 1)].id);
        }
        self.present(effects)
    }

    pub fn rename_workspace(&mut self, workspace_id: Uuid, name: &str) -> Vec<Effect> {
        let Some(workspace) = self.state.workspace_mut(workspace_id) else { return Vec::new() };
        let name = name.trim();
        if name.is_empty() {
            workspace.name = workspace.base_url.host_str().unwrap_or_default().to_string();
            workspace.name_custom = false;
        } else {
            workspace.name = truncate(name, MAX_NAME);
            workspace.name_custom = true;
        }
        vec![Effect::Changed]
    }

    pub fn set_icon(&mut self, workspace_id: Uuid, icon: Option<String>) -> Vec<Effect> {
        let Some(workspace) = self.state.workspace_mut(workspace_id) else { return Vec::new() };
        match icon {
            Some(icon) if !valid_icon(&icon) => return Vec::new(),
            Some(icon) => (workspace.icon, workspace.icon_custom) = (Some(icon), true),
            None => (workspace.icon, workspace.icon_custom) = (None, false),
        }
        vec![Effect::Changed]
    }

    pub fn set_login(&mut self, workspace_id: Uuid, login: Option<String>) -> Vec<Effect> {
        match self.state.workspace_mut(workspace_id) {
            Some(workspace) if workspace.login != login => {
                workspace.login = login;
                vec![Effect::Changed]
            }
            _ => Vec::new(),
        }
    }

    pub fn reorder_workspaces(&mut self, ids: &[Uuid]) -> Vec<Effect> {
        let reordered = reorder(&mut self.state.workspaces, ids, |workspace| workspace.id);
        if reordered { vec![Effect::Changed] } else { Vec::new() }
    }

    pub fn activate_workspace(&mut self, workspace_id: Uuid) -> Vec<Effect> {
        if self.state.workspace(workspace_id).is_none() {
            return Vec::new();
        }
        self.state.active_workspace_id = Some(workspace_id);
        self.present(Vec::new())
    }

    pub fn reload_workspace(&mut self, workspace_id: Uuid) -> Vec<Effect> {
        match self.state.workspace(workspace_id).and_then(|workspace| workspace.active_tab_id) {
            Some(tab_id) => self.reload_tab(workspace_id, tab_id),
            None => Vec::new(),
        }
    }

    pub fn clear_browsing_data(&mut self, workspace_id: Uuid) -> Vec<Effect> {
        let mut effects = vec![Effect::ClearProfile { workspace_id, delete: false }];
        effects.extend(
            self.live_tabs(workspace_id).into_iter().map(|(tab_id, _)| Effect::Reload { workspace_id, tab_id }),
        );
        effects
    }

    pub fn reopen_tabs(&self, workspace_id: Uuid) -> Vec<Effect> {
        self.live_tabs(workspace_id)
            .into_iter()
            .map(|(tab_id, url)| Effect::Navigate { workspace_id, tab_id, url })
            .collect()
    }

    pub fn activate_tab(&mut self, workspace_id: Uuid, tab_id: Uuid) -> Vec<Effect> {
        if self.state.workspace(workspace_id).and_then(|workspace| workspace.tab(tab_id)).is_none() {
            return Vec::new();
        }
        self.select(workspace_id, tab_id);
        self.present(Vec::new())
    }

    pub fn close_tab(&mut self, workspace_id: Uuid, tab_id: Uuid) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.remove_tab(workspace_id, tab_id, &mut effects);
        self.present(effects)
    }

    pub fn reorder_tabs(&mut self, workspace_id: Uuid, ids: &[Uuid]) -> Vec<Effect> {
        let reordered = self
            .state
            .workspace_mut(workspace_id)
            .is_some_and(|workspace| reorder(&mut workspace.tabs, ids, |tab| tab.id));
        if reordered { vec![Effect::Changed] } else { Vec::new() }
    }

    pub fn set_pinned(&mut self, workspace_id: Uuid, tab_id: Uuid, pinned: bool) -> Vec<Effect> {
        match self.state.workspace_mut(workspace_id).and_then(|workspace| workspace.tab_mut(tab_id)) {
            Some(tab) => {
                tab.pinned = pinned;
                vec![Effect::Changed]
            }
            None => Vec::new(),
        }
    }

    pub fn reload_tab(&mut self, workspace_id: Uuid, tab_id: Uuid) -> Vec<Effect> {
        if self.is_live(tab_id) { vec![Effect::Reload { workspace_id, tab_id }] } else { Vec::new() }
    }

    pub fn reload_home(&mut self, workspace_id: Uuid, tab_id: Uuid) -> Vec<Effect> {
        let home = self.state.workspace(workspace_id).and_then(|workspace| {
            let tab = workspace.tab(tab_id)?;
            Some(router::home_url(&workspace.base_url, &tab.app_id))
        });
        let Some(home) = home else { return Vec::new() };
        let mut effects = Vec::new();
        self.navigate_tab(workspace_id, tab_id, home, &mut effects);
        if effects.is_empty() {
            effects = self.reload_tab(workspace_id, tab_id);
        }
        effects.push(Effect::Changed);
        effects
    }

    pub fn set_offline(&mut self, tab_id: Uuid) -> Vec<Effect> {
        if self.state.find_tab(tab_id).is_none() || !self.offline.insert(tab_id) {
            return Vec::new();
        }
        self.present(Vec::new())
    }

    pub fn retry_tab(&mut self, tab_id: Uuid) -> Vec<Effect> {
        if !self.offline.remove(&tab_id) {
            return Vec::new();
        }
        let Some((workspace, tab)) = self.state.find_tab(tab_id) else { return Vec::new() };
        let navigate = Effect::Navigate { workspace_id: workspace.id, tab_id, url: tab.url.clone() };
        let effects = if self.is_live(tab_id) { vec![navigate] } else { Vec::new() };
        self.present(effects)
    }

    pub fn open_app(&mut self, workspace_id: Uuid, app_id: &str, url: Url, navigate_existing: bool) -> Vec<Effect> {
        let mut effects = Vec::new();
        let Some(workspace) = self.state.workspace(workspace_id) else { return effects };
        let existing = if app_id == AUTH { None } else { workspace.tab_by_app(app_id).map(|tab| tab.id) };
        let tab_id = match existing {
            Some(tab_id) => {
                if navigate_existing {
                    self.navigate_tab(workspace_id, tab_id, url, &mut effects);
                }
                tab_id
            }
            None => self.push_tab(workspace_id, app_id, url),
        };
        self.select(workspace_id, tab_id);
        self.present(effects)
    }

    pub fn open_app_from_menu(&mut self, workspace_id: Uuid, app_id: &str) -> Vec<Effect> {
        let href = self
            .state
            .workspace(workspace_id)
            .and_then(|workspace| workspace.apps.iter().find(|app| app.id == app_id))
            .map(|app| app.href.clone());
        match href {
            Some(href) => self.open_app(workspace_id, app_id, href, false),
            None => Vec::new(),
        }
    }

    pub fn open_home(&mut self, workspace_id: Uuid) -> Vec<Effect> {
        match self.state.workspace(workspace_id).map(|workspace| workspace.base_url.clone()) {
            Some(base) => self.open_app(workspace_id, AUTH, base, false),
            None => Vec::new(),
        }
    }

    pub fn open_login(&mut self, workspace_id: Uuid, url: Url) -> (Uuid, Vec<Effect>) {
        let Some(workspace) = self.state.workspace(workspace_id) else { return (Uuid::nil(), Vec::new()) };
        let mut effects = Vec::new();
        let tab_id = match workspace.tab_by_app(AUTH).map(|tab| tab.id) {
            Some(tab_id) => {
                self.navigate_tab(workspace_id, tab_id, url, &mut effects);
                tab_id
            }
            None => self.push_tab(workspace_id, AUTH, url),
        };
        self.select(workspace_id, tab_id);
        (tab_id, self.present(effects))
    }

    pub fn finish_login(&mut self, workspace_id: Uuid, tab_id: Uuid, login: &str) -> Vec<Effect> {
        let mut effects = self.set_login(workspace_id, Some(login.to_string()));
        if let Some(base) = self.state.workspace(workspace_id).map(|workspace| workspace.base_url.clone()) {
            self.navigate_tab(workspace_id, tab_id, base, &mut effects);
        }
        effects.push(Effect::Changed);
        effects
    }

    pub fn on_new_window(&mut self, url: &Url) -> Vec<Effect> {
        match router::classify_new_window(&self.state, url) {
            Route::Activate { workspace_id, app_id, url } => self.open_link(workspace_id, &app_id, url),
            Route::External(url) => vec![Effect::OpenExternal(url)],
            Route::InPlace | Route::Deny => Vec::new(),
        }
    }

    pub fn observe_location(&mut self, tab_id: Uuid, url: Url) -> Vec<Effect> {
        let Some((workspace, tab)) = self.state.find_tab(tab_id) else { return Vec::new() };
        if !urls::belongs(&url, &workspace.base_url) {
            return Vec::new();
        }
        let workspace_id = workspace.id;
        let current_app = tab.app_id.clone();
        let page_app = router::app_id(&url, &workspace.base_url);
        let other_tab_id = workspace
            .tab_by_app(&page_app)
            .map(|other| other.id)
            .filter(|&other_id| other_id != tab_id && page_app != AUTH);
        let was_active_tab = workspace.active_tab_id == Some(tab_id);

        if current_app == AUTH && page_app != AUTH {
            if let Some(other_tab_id) = other_tab_id {
                let mut effects = Vec::new();
                self.navigate_tab(workspace_id, other_tab_id, url, &mut effects);
                if was_active_tab {
                    if let Some(workspace) = self.state.workspace_mut(workspace_id) {
                        workspace.active_tab_id = Some(other_tab_id);
                    }
                }
                self.remove_tab(workspace_id, tab_id, &mut effects);
                return self.present(effects);
            }
        }
        let tab = self
            .state
            .workspace_mut(workspace_id)
            .and_then(|workspace| workspace.tab_mut(tab_id))
            .expect("found above");
        if page_app != AUTH && page_app != current_app && other_tab_id.is_none() {
            tab.app_id = page_app;
        }
        tab.url = url;
        vec![Effect::Changed]
    }

    pub fn observe_title(&mut self, tab_id: Uuid, title: &str) -> Vec<Effect> {
        let (page, instance) = split_title(title);
        let Some(workspace) = self.state.workspace_of_tab_mut(tab_id) else { return Vec::new() };
        if !workspace.name_custom {
            if let Some(instance) = instance {
                workspace.name = truncate(instance, MAX_NAME);
            }
        }
        if !page.is_empty() {
            if let Some(tab) = workspace.tab_mut(tab_id) {
                tab.title = truncate(page, MAX_TITLE);
            }
        }
        vec![Effect::Changed]
    }

    pub fn observe_meta(&mut self, tab_id: Uuid, icon: Option<String>, links: Vec<AppLink>) -> Vec<Effect> {
        let Some(workspace) = self.state.workspace_of_tab_mut(tab_id) else { return Vec::new() };
        match icon {
            _ if workspace.icon_custom => {}
            Some(icon) if icon.is_empty() => workspace.icon = None,
            Some(icon) if valid_icon(&icon) => workspace.icon = Some(icon),
            _ => {}
        }
        let apps = app_entries(links, &workspace.base_url);
        if !apps.is_empty() {
            workspace.apps = apps;
        }
        vec![Effect::Changed]
    }

    fn open_link(&mut self, workspace_id: Uuid, app_id: &str, url: Url) -> Vec<Effect> {
        let Some(workspace) = self.state.workspace(workspace_id) else { return Vec::new() };
        if let Some(tab_id) = workspace.tabs.iter().find(|tab| tab.url == url).map(|tab| tab.id) {
            self.select(workspace_id, tab_id);
            return self.present(Vec::new());
        }
        let is_app_home = url == router::home_url(&workspace.base_url, app_id)
            || workspace.apps.iter().any(|app| app.href == url);
        if is_app_home {
            return self.open_app(workspace_id, app_id, url, false);
        }
        let tab_id = self.push_tab(workspace_id, app_id, url);
        self.select(workspace_id, tab_id);
        self.present(Vec::new())
    }

    fn push_tab(&mut self, workspace_id: Uuid, app_id: &str, url: Url) -> Uuid {
        let workspace = self.state.workspace_mut(workspace_id).expect("caller checked the workspace");
        let tab = Tab::new(app_id, &workspace.app_name(app_id), url);
        let tab_id = tab.id;
        workspace.tabs.push(tab);
        tab_id
    }

    fn live_tabs(&self, workspace_id: Uuid) -> Vec<(Uuid, Url)> {
        let Some(workspace) = self.state.workspace(workspace_id) else { return Vec::new() };
        workspace
            .tabs
            .iter()
            .filter(|tab| self.is_live(tab.id))
            .map(|tab| (tab.id, tab.url.clone()))
            .collect()
    }

    fn select(&mut self, workspace_id: Uuid, tab_id: Uuid) {
        if let Some(workspace) = self.state.workspace_mut(workspace_id) {
            workspace.active_tab_id = Some(tab_id);
        }
        self.state.active_workspace_id = Some(workspace_id);
    }

    fn navigate_tab(&mut self, workspace_id: Uuid, tab_id: Uuid, url: Url, effects: &mut Vec<Effect>) {
        let live = self.is_live(tab_id);
        let Some(tab) = self.state.workspace_mut(workspace_id).and_then(|workspace| workspace.tab_mut(tab_id)) else {
            return;
        };
        if tab.url == url {
            return;
        }
        tab.url = url.clone();
        if live {
            effects.push(Effect::Navigate { workspace_id, tab_id, url });
        }
    }

    fn remove_tab(&mut self, workspace_id: Uuid, tab_id: Uuid, effects: &mut Vec<Effect>) {
        let Some(workspace) = self.state.workspace_mut(workspace_id) else { return };
        let Some(index) = workspace.tabs.iter().position(|tab| tab.id == tab_id) else { return };
        workspace.tabs.remove(index);
        if workspace.active_tab_id == Some(tab_id) {
            let neighbour = workspace.tabs.get(index).or_else(|| workspace.tabs.last());
            workspace.active_tab_id = neighbour.map(|tab| tab.id);
        }
        self.destroy_webview(workspace_id, tab_id, effects);
    }

    fn destroy_webview(&mut self, workspace_id: Uuid, tab_id: Uuid, effects: &mut Vec<Effect>) {
        self.offline.remove(&tab_id);
        if self.is_live(tab_id) {
            self.live.retain(|&live_id| live_id != tab_id);
            effects.push(Effect::Destroy { workspace_id, tab_id });
        }
    }

    fn present(&mut self, mut effects: Vec<Effect>) -> Vec<Effect> {
        let target = self.state.active_workspace_id.and_then(|workspace_id| {
            let tab = self.state.workspace(workspace_id)?.active_tab()?;
            Some((workspace_id, tab.id, tab.url.clone()))
        });
        match target {
            Some((workspace_id, tab_id, url)) if !self.overlay => {
                if !self.is_live(tab_id) {
                    effects.push(Effect::Create { workspace_id, tab_id, url });
                }
                self.live.retain(|&live_id| live_id != tab_id);
                self.live.push(tab_id);
                self.evict(&mut effects);
                effects.push(if self.is_offline(tab_id) {
                    Effect::HideContent
                } else {
                    Effect::Show { workspace_id, tab_id }
                });
            }
            _ => effects.push(Effect::HideContent),
        }
        effects.push(Effect::Changed);
        effects
    }

    fn evict(&mut self, effects: &mut Vec<Effect>) {
        while self.live.len() > self.max_live {
            let shown = *self.live.last().expect("just pushed");
            let pinned = |tab_id: Uuid| self.state.find_tab(tab_id).is_some_and(|(_, tab)| tab.pinned);
            let Some(victim) = self.live.iter().copied().find(|&tab_id| tab_id != shown && !pinned(tab_id)) else {
                break;
            };
            let workspace_id = self.state.find_tab(victim).map(|(workspace, _)| workspace.id);
            self.live.retain(|&live_id| live_id != victim);
            self.offline.remove(&victim);
            if let Some(workspace_id) = workspace_id {
                effects.push(Effect::Destroy { workspace_id, tab_id: victim });
            }
        }
    }
}

fn app_entries(links: Vec<AppLink>, base: &Url) -> Vec<AppEntry> {
    let mut seen = HashSet::new();
    links
        .into_iter()
        .filter_map(|link| {
            let href = Url::parse(&link.href).ok().filter(|href| urls::belongs(href, base))?;
            let id = router::app_id(&href, base);
            let name = link.name.trim();
            let keep = id != AUTH && !name.is_empty() && seen.insert(id.clone());
            keep.then(|| AppEntry { id, name: truncate(name, MAX_NAME), href })
        })
        .take(MAX_APPS)
        .collect()
}

fn valid_icon(icon: &str) -> bool {
    icon.len() <= MAX_ICON && icon.starts_with("data:image/")
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn split_title(title: &str) -> (&str, Option<&str>) {
    let title = title.trim();
    let cut = [" - ", " – "]
        .iter()
        .filter_map(|separator| title.rfind(separator).map(|index| (index, separator.len())))
        .max();
    match cut {
        Some((index, separator_length)) => {
            let instance = title[index + separator_length..].trim();
            let instance = (!instance.is_empty() && instance != "Nextcloud").then_some(instance);
            (title[..index].trim(), instance)
        }
        None => (title, None),
    }
}

fn reorder<T>(items: &mut [T], ids: &[Uuid], key: impl Fn(&T) -> Uuid) -> bool {
    if ids.len() != items.len() || !items.iter().all(|item| ids.contains(&key(item))) {
        return false;
    }
    items.sort_by_key(|item| ids.iter().position(|id| *id == key(item)));
    true
}

#[cfg(test)]
#[path = "../tests/unit/engine.rs"]
mod tests;
