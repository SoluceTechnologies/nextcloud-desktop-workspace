mod commands;
mod domain;
mod features;
mod runtime;
mod shared;
mod window;

use domain::engine::{Engine, MAX_LIVE};
use features::{downloads, menus, notifications, tabs, workspaces};
use shared::diagnostics;
use std::sync::{mpsc, Mutex};
use tauri::Manager;

pub fn run() {
    #[cfg(target_os = "linux")]
    shared::rendering::prepare();
    diagnostics::install_panic_hook();
    let result = tauri::Builder::default()
        .plugin(diagnostics::log_plugin())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::take_notice,
            workspaces::commands::add_workspace,
            workspaces::commands::remove_workspace,
            workspaces::commands::rename_workspace,
            workspaces::commands::reorder_workspaces,
            workspaces::commands::activate_workspace,
            tabs::commands::activate_tab,
            tabs::commands::close_tab,
            tabs::commands::reorder_tabs,
            tabs::commands::retry_tab,
            commands::set_overlay,
            workspaces::commands::clear_browsing_data,
            commands::set_theme,
            workspaces::commands::set_workspace_icon,
            menus::commands::workspace_menu,
            menus::commands::tab_menu,
            menus::commands::apps_menu,
            downloads::commands::reveal_download,
            downloads::commands::list_downloads,
            downloads::commands::cancel_download,
            tabs::commands::nc_report_location,
            tabs::commands::nc_report_meta,
            tabs::commands::nc_report_fullscreen,
            tabs::commands::nc_report_media_support,
            tabs::commands::open_in_browser,
        ])
        .on_menu_event(|app, event| menus::on_event(app, event.id().as_ref()))
        .setup(|app| {
            let handle = app.handle().clone();
            diagnostics::log_startup(&handle);
            #[cfg(target_os = "linux")]
            diagnostics::forward_glib_logs();
            let store_path = app.path().app_config_dir()?.join("workspaces.json");
            let (state, notice) = workspaces::store::load(&store_path);
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
            window::mouse_tracking::deliver_mouse_moves_only_to_topmost_webview();
            window::create_main_window(&handle)?;
            runtime::spawn_worker(handle.clone(), receiver, sweep_state);
            runtime::run(&handle, startup_effects);
            notifications::spawn_poller(handle);
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = result {
        log::error!("NC Workspaces stopped: {error}");
        eprintln!("NC Workspaces stopped: {error}");
        std::process::exit(1);
    }
}
