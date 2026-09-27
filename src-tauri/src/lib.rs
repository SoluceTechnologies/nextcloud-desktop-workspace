mod auth;
mod commands;
mod downloads;
mod engine;
mod http;
mod keychain;
#[cfg(target_os = "linux")]
mod media;
mod menus;
mod model;
#[cfg(target_os = "macos")]
mod mouse_tracking;
mod notifications;
mod offline;
mod profiles;
mod router;
mod runtime;
mod session;
mod signed_load;
mod store;
mod urls;
mod webviews;
mod window;

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
            commands::retry_tab,
            commands::set_overlay,
            commands::clear_browsing_data,
            commands::set_theme,
            commands::set_workspace_icon,
            commands::workspace_menu,
            commands::tab_menu,
            commands::apps_menu,
            commands::reveal_download,
            commands::nc_report_location,
            commands::nc_report_meta,
            commands::nc_report_fullscreen,
        ])
        .on_menu_event(|app, event| menus::on_event(app, event.id().as_ref()))
        .setup(|app| {
            let handle = app.handle().clone();
            let store_path = app.path().app_config_dir()?.join("workspaces.json");
            let (state, notice) = store::load(&store_path);
            let sweep_state = notice.is_none().then(|| state.clone());
            let mut engine = Engine::new(state, MAX_LIVE);
            let startup_effects = engine.startup();
            let (sender, receiver) = mpsc::channel();
            app.manage(Mutex::new(engine));
            app.manage(commands::Notice(Mutex::new(notice)));
            app.manage(runtime::StorePath(store_path));
            app.manage(runtime::EffectSender(sender));
            app.manage(window::Fullscreen::default());
            #[cfg(target_os = "macos")]
            mouse_tracking::deliver_mouse_moves_only_to_topmost_webview();
            window::create_main_window(&handle)?;
            runtime::spawn_worker(handle.clone(), receiver, sweep_state);
            runtime::run(&handle, startup_effects);
            notifications::spawn_poller(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
