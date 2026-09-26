//! Persisted data model (spec §4). No behaviour beyond lookups.

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// app_id of pages that never own a tab: auth flows, the root, and non-app resources
/// (remote.php, ocs, f/<id>…). A tab starts as AUTH and adopts the first real app it lands on.
pub const AUTH: &str = "AUTH";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub version: u32,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: Option<Uuid>,
    /// Window appearance chosen in the sidebar; Nextcloud pages follow it too (prefers-color-scheme).
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
    pub apps: Vec<AppEntry>,
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
            apps: Vec::new(),
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

    /// Display name of an app from the cached app menu, falling back to its id.
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
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn new_workspace_has_one_selected_auth_tab_named_after_host() {
        let w = Workspace::new(u("https://cloud.soluce.com/"));
        assert_eq!(w.name, "cloud.soluce.com");
        assert!(!w.name_custom);
        assert_eq!(w.tabs.len(), 1);
        assert_eq!(w.tabs[0].app_id, AUTH);
        assert_eq!(w.tabs[0].url, u("https://cloud.soluce.com/"));
        assert_eq!(w.active_tab_id, Some(w.tabs[0].id));
    }

    #[test]
    fn find_tab_returns_owner_workspace() {
        let mut s = AppState::default();
        s.workspaces.push(Workspace::new(u("https://a.com/")));
        s.workspaces.push(Workspace::new(u("https://b.com/")));
        let tab = s.workspaces[1].tabs[0].id;
        assert_eq!(s.find_tab(tab).map(|(w, _)| w.id), Some(s.workspaces[1].id));
        assert!(s.find_tab(Uuid::new_v4()).is_none());
    }

    #[test]
    fn serializes_camel_case_and_round_trips() {
        let mut s = AppState::default();
        s.workspaces.push(Workspace::new(u("https://a.com/nc")));
        s.active_workspace_id = Some(s.workspaces[0].id);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"activeWorkspaceId\""));
        assert!(json.contains("\"baseUrl\":\"https://a.com/nc\""));
        assert!(json.contains("\"appId\":\"AUTH\""));
        let back: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn appearance_defaults_to_system_and_serializes_lowercase() {
        let old: AppState = serde_json::from_str(r#"{"version":1,"workspaces":[],"activeWorkspaceId":null}"#).unwrap();
        assert_eq!(old.theme, Appearance::System);
        let s = AppState { theme: Appearance::Dark, ..AppState::default() };
        assert!(serde_json::to_string(&s).unwrap().contains(r#""theme":"dark""#));
    }
}
