use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

pub const AUTH: &str = "AUTH";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub version: u32,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: Option<Uuid>,
    #[serde(default)]
    pub theme: Appearance,
}

impl Default for AppState {
    fn default() -> Self {
        Self { version: 1, workspaces: Vec::new(), active_workspace_id: None, theme: Appearance::System }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: Uuid,
    pub base_url: Url,
    pub name: String,
    #[serde(default)]
    pub name_custom: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_custom: bool,
    #[serde(default)]
    pub apps: Vec<AppEntry>,
    #[serde(default)]
    pub login: Option<String>,
    pub tabs: Vec<Tab>,
    pub active_tab_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: Uuid,
    pub app_id: String,
    pub title: String,
    pub url: Url,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub href: Url,
}

impl AppState {
    pub fn ws(&self, id: Uuid) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.id == id)
    }

    pub fn ws_mut(&mut self, id: Uuid) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|w| w.id == id)
    }

    pub fn find_tab(&self, tab: Uuid) -> Option<(&Workspace, &Tab)> {
        self.workspaces.iter().find_map(|w| w.tab(tab).map(|t| (w, t)))
    }

    pub fn ws_of_tab_mut(&mut self, tab: Uuid) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|w| w.tab(tab).is_some())
    }
}

impl Workspace {
    /// New workspace named after its host, with one selected AUTH tab at the base URL (→ login page).
    pub fn new(base_url: Url) -> Self {
        let name = base_url.host_str().unwrap_or_default().to_string();
        let tab = Tab::new(AUTH, "Nextcloud", base_url.clone());
        Self {
            id: Uuid::new_v4(),
            name,
            name_custom: false,
            icon: None,
            icon_custom: false,
            apps: Vec::new(),
            login: None,
            active_tab_id: Some(tab.id),
            tabs: vec![tab],
            base_url,
        }
    }

    pub fn tab(&self, id: Uuid) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn tab_mut(&mut self, id: Uuid) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn tab_by_app(&self, app_id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.app_id == app_id)
    }

    pub fn app_name(&self, app_id: &str) -> String {
        self.apps.iter().find(|a| a.id == app_id).map_or_else(|| app_id.to_string(), |a| a.name.clone())
    }
}

impl Tab {
    pub fn new(app_id: &str, title: &str, url: Url) -> Self {
        Self { id: Uuid::new_v4(), app_id: app_id.into(), title: title.into(), url, pinned: false }
    }
}

#[cfg(test)]
#[path = "../tests/unit/model.rs"]
mod tests;
