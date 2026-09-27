use crate::engine::{Effect, Engine, Shared};
use crate::model::AppState;
use crate::{profiles, store, webviews, window};
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::MutexGuard;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

pub type EffectResult = Result<(), Box<dyn Error>>;

pub struct EffectSender(pub Sender<Vec<Effect>>);

pub struct StorePath(pub PathBuf);

pub fn engine(app: &AppHandle) -> MutexGuard<'_, Engine> {
    app.state::<Shared>().inner().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn run(app: &AppHandle, effects: Vec<Effect>) {
    if !effects.is_empty() {
        let _ = app.state::<EffectSender>().0.send(effects);
    }
}

pub fn notice(app: &AppHandle, text: impl Into<String>) {
    let _ = app.emit_to("shell", "notice", text.into());
}

pub fn spawn_worker(app: AppHandle, receiver: Receiver<Vec<Effect>>, sweep_state: Option<AppState>) {
    std::thread::spawn(move || {
        let mut sweep_state = sweep_state;
        let mut bridged_workspaces = HashSet::new();
        for effects in receiver {
            apply(&app, effects, &mut bridged_workspaces);
            if let Some(state) = sweep_state.take() {
                profiles::sweep(&app, &state);
            }
        }
    });
}

fn apply(app: &AppHandle, effects: Vec<Effect>, bridged_workspaces: &mut HashSet<Uuid>) {
    let mut changed = false;
    for effect in effects {
        let result = match effect {
            Effect::Changed => {
                changed = true;
                Ok(())
            }
            Effect::Create { workspace_id, tab_id, url } => {
                webviews::create(app, workspace_id, tab_id, url, bridged_workspaces)
            }
            Effect::Navigate { workspace_id, tab_id, url } => {
                webviews::with_webview(app, workspace_id, tab_id, |webview| {
                    webviews::loading(app, tab_id, true);
                    webview.navigate(url)
                })
            }
            Effect::Reload { workspace_id, tab_id } => webviews::with_webview(app, workspace_id, tab_id, |webview| {
                webviews::loading(app, tab_id, true);
                webview.reload()
            }),
            Effect::Destroy { workspace_id, tab_id } => {
                webviews::with_webview(app, workspace_id, tab_id, |webview| webview.close())
            }
            Effect::Show { workspace_id, tab_id } => {
                window::show_only(app, Some(&webviews::label(workspace_id, tab_id)))
            }
            Effect::HideContent => window::show_only(app, None),
            Effect::OpenExternal(url) => webviews::open_external(app, &url),
            Effect::ClearProfile { workspace_id, delete } => profiles::clear(app, workspace_id, delete),
            Effect::Theme(appearance) => window::apply_theme(app, appearance),
        };
        if let Err(error) = result {
            eprintln!("[ncw] effect failed: {error}");
        }
    }
    if changed {
        publish(app);
    }
}

fn publish(app: &AppHandle) {
    let state = engine(app).state.clone();
    let _ = app.emit_to("shell", "state-changed", &state);
    if let Err(error) = store::save(&app.state::<StorePath>().0, &state) {
        eprintln!("[ncw] save failed: {error}");
    }
}
