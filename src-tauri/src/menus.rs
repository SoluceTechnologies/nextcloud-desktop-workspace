use crate::auth;
use crate::runtime::{engine, run};
use serde::Serialize;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Wry};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    EditWorkspace(Uuid),
    ReloadWorkspace(Uuid),
    ClearWorkspace(Uuid),
    SignIn(Uuid),
    SignOut(Uuid),
    RemoveWorkspace(Uuid),
    PinTab(Uuid, Uuid, bool),
    ReloadTab(Uuid, Uuid),
    ReloadTabHome(Uuid, Uuid),
    CloseTab(Uuid, Uuid),
    OpenApp(Uuid, String),
    OpenHome(Uuid),
}

impl Action {
    pub fn id(&self) -> String {
        match self {
            Action::EditWorkspace(workspace_id) => format!("ws-rename|{workspace_id}"),
            Action::ReloadWorkspace(workspace_id) => format!("ws-reload|{workspace_id}"),
            Action::ClearWorkspace(workspace_id) => format!("ws-clear|{workspace_id}"),
            Action::SignIn(workspace_id) => format!("ws-sign-in|{workspace_id}"),
            Action::SignOut(workspace_id) => format!("ws-sign-out|{workspace_id}"),
            Action::RemoveWorkspace(workspace_id) => format!("ws-remove|{workspace_id}"),
            Action::PinTab(workspace_id, tab_id, pinned) => format!("tab-pin|{workspace_id}|{tab_id}|{pinned}"),
            Action::ReloadTab(workspace_id, tab_id) => format!("tab-reload|{workspace_id}|{tab_id}"),
            Action::ReloadTabHome(workspace_id, tab_id) => format!("tab-home|{workspace_id}|{tab_id}"),
            Action::CloseTab(workspace_id, tab_id) => format!("tab-close|{workspace_id}|{tab_id}"),
            Action::OpenApp(workspace_id, app_id) => format!("open-app|{workspace_id}|{app_id}"),
            Action::OpenHome(workspace_id) => format!("open-home|{workspace_id}"),
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        let parts: Vec<&str> = id.split('|').collect();
        let uuid_at = |index: usize| parts.get(index).and_then(|part| Uuid::parse_str(part).ok());
        Some(match parts[0] {
            "ws-rename" => Action::EditWorkspace(uuid_at(1)?),
            "ws-reload" => Action::ReloadWorkspace(uuid_at(1)?),
            "ws-clear" => Action::ClearWorkspace(uuid_at(1)?),
            "ws-sign-in" => Action::SignIn(uuid_at(1)?),
            "ws-sign-out" => Action::SignOut(uuid_at(1)?),
            "ws-remove" => Action::RemoveWorkspace(uuid_at(1)?),
            "tab-pin" => Action::PinTab(uuid_at(1)?, uuid_at(2)?, *parts.get(3)? == "true"),
            "tab-reload" => Action::ReloadTab(uuid_at(1)?, uuid_at(2)?),
            "tab-home" => Action::ReloadTabHome(uuid_at(1)?, uuid_at(2)?),
            "tab-close" => Action::CloseTab(uuid_at(1)?, uuid_at(2)?),
            "open-app" => Action::OpenApp(uuid_at(1)?, parts.get(2)?.to_string()),
            "open-home" => Action::OpenHome(uuid_at(1)?),
            _ => return None,
        })
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DialogRequest {
    kind: &'static str,
    workspace_id: Uuid,
    tab_id: Option<Uuid>,
}

fn menu_item(app: &AppHandle, action: Action, text: &str) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, action.id(), text, true, None::<&str>)
}

fn popup(app: &AppHandle, at: LogicalPosition<f64>, items: &[&dyn IsMenuItem<Wry>]) -> tauri::Result<()> {
    let Some(window) = app.get_window("main") else { return Ok(()) };
    let menu = Menu::with_items(app, items)?;
    // Wayland exposes no global pointer position, so GTK can't place a menu "at the cursor".
    #[cfg(target_os = "linux")]
    return window.popup_menu_at(&menu, in_decorated_window(&window, at));
    #[cfg(not(target_os = "linux"))]
    {
        let _ = at;
        window.popup_menu(&menu)
    }
}

// Menus are placed relative to the whole GTK window surface, which with client-side
// decorations also holds the shadow and header bar above/left of the webview area.
#[cfg(target_os = "linux")]
fn in_decorated_window(window: &tauri::Window, at: LogicalPosition<f64>) -> LogicalPosition<f64> {
    use gtk::prelude::WidgetExt;
    let (Ok(gtk_window), Ok(content)) = (window.gtk_window(), window.default_vbox()) else { return at };
    match content.translate_coordinates(&gtk_window, 0, 0) {
        Some((dx, dy)) => LogicalPosition::new(at.x + f64::from(dx), at.y + f64::from(dy)),
        None => at,
    }
}

fn is_pinned(app: &AppHandle, workspace_id: Uuid, tab_id: Uuid) -> bool {
    engine(app)
        .state
        .workspace(workspace_id)
        .and_then(|workspace| workspace.tab(tab_id))
        .is_some_and(|tab| tab.pinned)
}

pub fn popup_workspace(app: &AppHandle, workspace_id: Uuid, at: LogicalPosition<f64>) -> tauri::Result<()> {
    let edit = menu_item(app, Action::EditWorkspace(workspace_id), "Edit workspace…")?;
    let reload = menu_item(app, Action::ReloadWorkspace(workspace_id), "Reload")?;
    let clear = menu_item(app, Action::ClearWorkspace(workspace_id), "Clear browsing data…")?;
    let login = engine(app).state.workspace(workspace_id).and_then(|workspace| workspace.login.clone());
    let session = match login {
        Some(login) => menu_item(app, Action::SignOut(workspace_id), &format!("Sign out ({login})"))?,
        None => menu_item(app, Action::SignIn(workspace_id), "Stay signed in…")?,
    };
    let separator = PredefinedMenuItem::separator(app)?;
    let remove = menu_item(app, Action::RemoveWorkspace(workspace_id), "Remove workspace…")?;
    popup(app, at, &[&edit, &reload, &clear, &session, &separator, &remove])
}

pub fn popup_tab(app: &AppHandle, workspace_id: Uuid, tab_id: Uuid, at: LogicalPosition<f64>) -> tauri::Result<()> {
    let pinned = is_pinned(app, workspace_id, tab_id);
    let pin = menu_item(app, Action::PinTab(workspace_id, tab_id, !pinned), if pinned { "Unpin" } else { "Pin" })?;
    let reload = menu_item(app, Action::ReloadTab(workspace_id, tab_id), "Reload")?;
    let home = menu_item(app, Action::ReloadTabHome(workspace_id, tab_id), "Reload at app home")?;
    let separator = PredefinedMenuItem::separator(app)?;
    let close = menu_item(app, Action::CloseTab(workspace_id, tab_id), "Close tab")?;
    popup(app, at, &[&pin, &reload, &home, &separator, &close])
}

pub fn popup_apps(app: &AppHandle, workspace_id: Uuid, at: LogicalPosition<f64>) -> tauri::Result<()> {
    let (apps, open_apps): (Vec<(String, String)>, Vec<String>) = {
        let engine = engine(app);
        let Some(workspace) = engine.state.workspace(workspace_id) else { return Ok(()) };
        (
            workspace.apps.iter().map(|app| (app.id.clone(), app.name.clone())).collect(),
            workspace.tabs.iter().map(|tab| tab.app_id.clone()).collect(),
        )
    };
    if apps.is_empty() {
        let home = menu_item(app, Action::OpenHome(workspace_id), "Open home page")?;
        return popup(app, at, &[&home]);
    }
    let items = apps
        .iter()
        .map(|(app_id, name)| {
            let action = Action::OpenApp(workspace_id, app_id.clone());
            CheckMenuItem::with_id(app, action.id(), name, true, open_apps.contains(app_id), None::<&str>)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let references: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|item| item as &dyn IsMenuItem<Wry>).collect();
    popup(app, at, &references)
}

pub fn on_event(app: &AppHandle, id: &str) {
    let Some(action) = Action::parse(id) else { return };
    let open_dialog = |kind: &'static str, workspace_id: Uuid, tab_id: Option<Uuid>| {
        let _ = app.emit_to("shell", "ui-request", DialogRequest { kind, workspace_id, tab_id });
    };
    let effects = match action {
        Action::EditWorkspace(workspace_id) => return open_dialog("edit", workspace_id, None),
        Action::ClearWorkspace(workspace_id) => return open_dialog("confirm-clear", workspace_id, None),
        Action::RemoveWorkspace(workspace_id) => return open_dialog("confirm-remove", workspace_id, None),
        Action::SignIn(workspace_id) => return auth::sign_in(app, workspace_id),
        Action::SignOut(workspace_id) => return auth::sign_out(app, workspace_id, true),
        Action::ReloadWorkspace(workspace_id) => engine(app).reload_workspace(workspace_id),
        Action::PinTab(workspace_id, tab_id, pinned) => engine(app).set_pinned(workspace_id, tab_id, pinned),
        Action::ReloadTab(workspace_id, tab_id) => engine(app).reload_tab(workspace_id, tab_id),
        Action::ReloadTabHome(workspace_id, tab_id) => engine(app).reload_home(workspace_id, tab_id),
        Action::CloseTab(workspace_id, tab_id) => {
            if is_pinned(app, workspace_id, tab_id) {
                return open_dialog("confirm-close", workspace_id, Some(tab_id));
            }
            engine(app).close_tab(workspace_id, tab_id)
        }
        Action::OpenApp(workspace_id, app_id) => engine(app).open_app_from_menu(workspace_id, &app_id),
        Action::OpenHome(workspace_id) => engine(app).open_home(workspace_id),
    };
    run(app, effects);
}

#[cfg(test)]
#[path = "../tests/unit/menus.rs"]
mod tests;
