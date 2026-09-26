fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            // shell webview
            "get_state",
            "take_notice",
            "add_workspace",
            "remove_workspace",
            "rename_workspace",
            "reorder_workspaces",
            "activate_workspace",
            "activate_tab",
            "close_tab",
            "reorder_tabs",
            "set_overlay",
            "clear_browsing_data",
            "workspace_menu",
            "tab_menu",
            "apps_menu",
            "reveal_download",
            // nextcloud pages (granted at runtime per workspace origin)
            "nc_report_location",
            "nc_report_meta",
            "nc_report_fullscreen",
        ]),
    ))
    .expect("failed to run tauri-build");
}
