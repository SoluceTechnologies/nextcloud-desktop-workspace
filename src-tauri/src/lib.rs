mod auth;
mod commands;
mod downloads;
mod engine;
#[cfg(target_os = "linux")]
mod media;
mod menus;
mod model;
mod monitor;
mod router;
mod session;
mod store;
mod urls;
mod webviews;

use engine::{Engine, MAX_LIVE};
use std::sync::{mpsc, Mutex};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::take_notice,
            commands::add_workspace,
            commands::remove_workspace,
            commands::rename_workspace,
            commands::reorder_workspaces,
            commands::activate_workspace,
            commands::activate_tab,
            commands::close_tab,
            commands::reorder_tabs,
            commands::set_overlay,
            commands::clear_browsing_data,
            commands::set_theme,
            commands::set_workspace_icon,
            commands::nc_report_location,
            commands::nc_report_meta,
            commands::nc_report_fullscreen,
            commands::workspace_menu,
            commands::tab_menu,
            commands::apps_menu,
            commands::reveal_download,
            commands::retry_tab,
        ])
        .on_menu_event(|app, event| menus::on_event(app, event.id().as_ref()))
        .setup(|app| {
            let handle = app.handle().clone();
            let store_path = app.path().app_config_dir()?.join("workspaces.json");
            let (state, notice) = store::load(&store_path);
            let sweep_state = notice.is_none().then(|| state.clone());
            let mut engine = Engine::new(state, MAX_LIVE);
            let startup = engine.startup();
            let (tx, rx) = mpsc::channel();
            app.manage(Mutex::new(engine));
            app.manage(commands::Notice(Mutex::new(notice)));
            app.manage(webviews::StorePath(store_path));
            app.manage(webviews::EffectTx(tx));
            app.manage(webviews::Fullscreen::default());
            webviews::create_main_window(&handle)?;
            webviews::spawn_worker(handle.clone(), rx, sweep_state);
            webviews::run(&handle, startup);
            monitor::spawn_poller(handle.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
