//! Native context menus (drawn above the child webviews, unlike HTML menus) and their events.

use crate::webviews::{engine, run};
use serde::Serialize;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, Manager, Wry};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    WsRename(Uuid),
    WsReload(Uuid),
    WsClear(Uuid),
    WsRemove(Uuid),
    TabPin(Uuid, Uuid, bool),
    TabReload(Uuid, Uuid),
    TabHome(Uuid, Uuid),
    TabClose(Uuid, Uuid),
    OpenApp(Uuid, String),
    OpenHome(Uuid),
}

impl Action {
    pub fn id(&self) -> String {
        match self {
            Action::WsRename(w) => format!("ws-rename|{w}"),
            Action::WsReload(w) => format!("ws-reload|{w}"),
            Action::WsClear(w) => format!("ws-clear|{w}"),
            Action::WsRemove(w) => format!("ws-remove|{w}"),
            Action::TabPin(w, t, pin) => format!("tab-pin|{w}|{t}|{pin}"),
            Action::TabReload(w, t) => format!("tab-reload|{w}|{t}"),
            Action::TabHome(w, t) => format!("tab-home|{w}|{t}"),
            Action::TabClose(w, t) => format!("tab-close|{w}|{t}"),
            Action::OpenApp(w, app) => format!("open-app|{w}|{app}"),
            Action::OpenHome(w) => format!("open-home|{w}"),
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        let parts: Vec<&str> = id.split('|').collect();
        let uuid = |i: usize| parts.get(i).and_then(|s| Uuid::parse_str(s).ok());
        Some(match parts[0] {
            "ws-rename" => Action::WsRename(uuid(1)?),
            "ws-reload" => Action::WsReload(uuid(1)?),
            "ws-clear" => Action::WsClear(uuid(1)?),
            "ws-remove" => Action::WsRemove(uuid(1)?),
            "tab-pin" => Action::TabPin(uuid(1)?, uuid(2)?, *parts.get(3)? == "true"),
            "tab-reload" => Action::TabReload(uuid(1)?, uuid(2)?),
            "tab-home" => Action::TabHome(uuid(1)?, uuid(2)?),
            "tab-close" => Action::TabClose(uuid(1)?, uuid(2)?),
            "open-app" => Action::OpenApp(uuid(1)?, parts.get(2)?.to_string()),
            "open-home" => Action::OpenHome(uuid(1)?),
            _ => return None,
        })
    }
}

#[derive(Clone, Serialize)]
struct UiRequest {
    kind: &'static str,
    ws: Uuid,
    tab: Option<Uuid>,
}

fn item(app: &AppHandle, action: Action, text: &str) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, action.id(), text, true, None::<&str>)
}

fn popup(app: &AppHandle, items: &[&dyn IsMenuItem<Wry>]) -> tauri::Result<()> {
    let Some(window) = app.get_window("main") else { return Ok(()) };
    window.popup_menu(&Menu::with_items(app, items)?)
}

pub fn popup_workspace(app: &AppHandle, ws: Uuid) -> tauri::Result<()> {
    let rename = item(app, Action::WsRename(ws), "Edit workspace…")?;
    let reload = item(app, Action::WsReload(ws), "Reload")?;
    let clear = item(app, Action::WsClear(ws), "Clear browsing data…")?;
    let sep = PredefinedMenuItem::separator(app)?;
    let remove = item(app, Action::WsRemove(ws), "Remove workspace…")?;
    popup(app, &[&rename, &reload, &clear, &sep, &remove])
}

pub fn popup_tab(app: &AppHandle, ws: Uuid, tab: Uuid) -> tauri::Result<()> {
    let pinned = engine(app).state.ws(ws).and_then(|w| w.tab(tab)).is_some_and(|t| t.pinned);
    let pin = item(app, Action::TabPin(ws, tab, !pinned), if pinned { "Unpin" } else { "Pin" })?;
    let reload = item(app, Action::TabReload(ws, tab), "Reload")?;
    let home = item(app, Action::TabHome(ws, tab), "Reload at app home")?;
    let sep = PredefinedMenuItem::separator(app)?;
    let close = item(app, Action::TabClose(ws, tab), "Close tab")?;
    popup(app, &[&pin, &reload, &home, &sep, &close])
}

pub fn popup_apps(app: &AppHandle, ws: Uuid) -> tauri::Result<()> {
    let (apps, open): (Vec<(String, String)>, Vec<String>) = {
        let e = engine(app);
        let Some(w) = e.state.ws(ws) else { return Ok(()) };
        (
            w.apps.iter().map(|a| (a.id.clone(), a.name.clone())).collect(),
            w.tabs.iter().map(|t| t.app_id.clone()).collect(),
        )
    };
    if apps.is_empty() {
        let home = item(app, Action::OpenHome(ws), "Open home page")?;
        return popup(app, &[&home]);
    }
    let items = apps
        .iter()
        .map(|(id, name)| CheckMenuItem::with_id(app, Action::OpenApp(ws, id.clone()).id(), name, true, open.contains(id), None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i as &dyn IsMenuItem<Wry>).collect();
    popup(app, &refs)
}

pub fn on_event(app: &AppHandle, id: &str) {
    let Some(action) = Action::parse(id) else { return };
    let request = |kind: &'static str, ws: Uuid, tab: Option<Uuid>| {
        let _ = app.emit_to("shell", "ui-request", UiRequest { kind, ws, tab });
    };
    let fx = match action {
        Action::WsRename(ws) => return request("edit", ws, None),
        Action::WsClear(ws) => return request("confirm-clear", ws, None),
        Action::WsRemove(ws) => return request("confirm-remove", ws, None),
        Action::WsReload(ws) => engine(app).reload_workspace(ws),
        Action::TabPin(ws, tab, pin) => engine(app).set_pinned(ws, tab, pin),
        Action::TabReload(ws, tab) => engine(app).reload_tab(ws, tab),
        Action::TabHome(ws, tab) => engine(app).reload_home(ws, tab),
        Action::TabClose(ws, tab) => {
            let pinned = engine(app).state.ws(ws).and_then(|w| w.tab(tab)).is_some_and(|t| t.pinned);
            if pinned {
                return request("confirm-close", ws, Some(tab));
            }
            engine(app).close_tab(ws, tab)
        }
        Action::OpenApp(ws, app_id) => engine(app).open_app_from_menu(ws, &app_id),
        Action::OpenHome(ws) => engine(app).open_home(ws),
    };
    run(app, fx);
}

#[cfg(test)]
#[path = "../tests/unit/menus.rs"]
mod tests;
