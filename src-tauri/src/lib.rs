mod commands;
mod downloads;
mod engine;
mod menus;
mod model;
mod router;
mod store;
mod urls;
mod webviews;

use engine::{Engine, MAX_LIVE};
use std::sync::{mpsc, Mutex};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            commands::nc_report_location,
            commands::nc_report_meta,
            commands::workspace_menu,
            commands::tab_menu,
            commands::apps_menu,
            commands::reveal_download,
        ])
        .on_menu_event(|app, event| menus::on_event(app, event.id().as_ref()))
        .setup(|app| {
            let handle = app.handle().clone();
            let store_path = app.path().app_config_dir()?.join("workspaces.json");
            let (state, notice) = store::load(&store_path);
            // An unreadable workspaces.json loads an empty recovery state (see store::load); sweeping
            // profiles against it would delete every still-live profile, so skip the sweep entirely.
            let sweep_state = notice.is_none().then(|| state.clone());
            let mut engine = Engine::new(state, MAX_LIVE);
            let startup = engine.startup();
            let (tx, rx) = mpsc::channel();
            app.manage(Mutex::new(engine));
            app.manage(commands::Notice(Mutex::new(notice)));
            app.manage(webviews::StorePath(store_path));
            app.manage(webviews::EffectTx(tx));
            webviews::create_main_window(&handle)?;
            webviews::spawn_worker(handle.clone(), rx, sweep_state);
            webviews::run(&handle, startup);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
