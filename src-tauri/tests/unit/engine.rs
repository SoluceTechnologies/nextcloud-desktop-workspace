use super::*;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn engine_with(bases: &[&str], max_live: usize) -> Engine {
    let mut engine = Engine::new(AppState::default(), max_live);
    for base in bases {
        engine.add_workspace(base).unwrap();
    }
    engine
}

fn workspace_at(engine: &Engine, index: usize) -> Uuid {
    engine.state.workspaces[index].id
}

fn tab_of_app(engine: &Engine, index: usize, app: &str) -> Uuid {
    engine.state.workspaces[index].tab_by_app(app).unwrap().id
}

fn shown(effects: &[Effect]) -> Option<Uuid> {
    effects.iter().rev().find_map(|effect| match effect {
        Effect::Show { tab_id, .. } => Some(*tab_id),
        _ => None,
    })
}

fn created(effects: &[Effect]) -> Vec<Uuid> {
    effects.iter().filter_map(|effect| match effect { Effect::Create { tab_id, .. } => Some(*tab_id), _ => None }).collect()
}

fn destroyed(effects: &[Effect]) -> Vec<Uuid> {
    effects.iter().filter_map(|effect| match effect { Effect::Destroy { tab_id, .. } => Some(*tab_id), _ => None }).collect()
}

#[test]
fn add_workspace_creates_and_shows_auth_tab() {
    let mut engine = Engine::new(AppState::default(), MAX_LIVE);
    let effects = engine.add_workspace("cloud.soluce.com").unwrap();
    let workspace = &engine.state.workspaces[0];
    assert_eq!(workspace.base_url, url("https://cloud.soluce.com/"));
    assert_eq!(engine.state.active_workspace_id, Some(workspace.id));
    let tab_id = workspace.tabs[0].id;
    assert_eq!(created(&effects), vec![tab_id]);
    assert_eq!(shown(&effects), Some(tab_id));
    assert_eq!(effects.last(), Some(&Effect::Changed));
}

#[test]
fn add_existing_url_activates_instead_of_duplicating() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    engine.add_workspace(" https://A.com/index.php/apps/files ").unwrap();
    assert_eq!(engine.state.workspaces.len(), 2);
    assert_eq!(engine.state.active_workspace_id, Some(workspace_at(&engine, 0)));
}

#[test]
fn add_invalid_url_is_an_error() {
    let mut engine = Engine::new(AppState::default(), MAX_LIVE);
    assert!(engine.add_workspace("ftp://x.com").is_err());
    assert!(engine.state.workspaces.is_empty());
}

#[test]
fn startup_creates_only_the_active_tab_and_repairs_selection() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let first_workspace_id = workspace_at(&engine, 0);
    engine.open_app(first_workspace_id, "files", url("https://a.com/apps/files/"), true);
    let files = tab_of_app(&engine, 0, "files");
    let mut restarted = Engine::new(engine.state.clone(), MAX_LIVE);
    assert_eq!(created(&restarted.startup()), vec![files]);
    restarted.state.active_workspace_id = None;
    let mut restarted_again = Engine::new(restarted.state.clone(), MAX_LIVE);
    restarted_again.startup();
    assert_eq!(restarted_again.state.active_workspace_id, Some(first_workspace_id));
}

#[test]
fn open_app_creates_tab_once_then_reuses_it() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    let effects = engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    let files = tab_of_app(&engine, 0, "files");
    assert_eq!(created(&effects), vec![files]);
    assert_eq!(shown(&effects), Some(files));
    let deep = url("https://a.com/apps/files/?dir=/Photos");
    let effects = engine.open_app(workspace_id, "files", deep.clone(), true);
    assert!(created(&effects).is_empty());
    assert!(effects.contains(&Effect::Navigate { workspace_id, tab_id: files, url: deep }));
    assert_eq!(engine.state.workspaces[0].tabs.len(), 2);
}

#[test]
fn open_app_from_picker_does_not_navigate_existing_tab() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/?dir=/Photos"), true);
    let effects = engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), false);
    assert!(!effects.iter().any(|effect| matches!(effect, Effect::Navigate { .. })));
}

#[test]
fn open_app_names_tab_from_app_cache() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    engine.state.workspaces[0].apps.push(AppEntry { id: "spreed".into(), name: "Talk".into(), href: url("https://a.com/apps/spreed/") });
    engine.open_app(workspace_at(&engine, 0), "spreed", url("https://a.com/apps/spreed/"), true);
    assert_eq!(engine.state.workspaces[0].tab_by_app("spreed").unwrap().title, "Talk");
}

#[test]
fn switching_workspaces_restores_each_selected_tab_without_reload() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (first_workspace_id, second_workspace_id) = (workspace_at(&engine, 0), workspace_at(&engine, 1));
    engine.open_app(first_workspace_id, "calendar", url("https://a.com/apps/calendar/"), true);
    let calendar = tab_of_app(&engine, 0, "calendar");
    engine.open_app(second_workspace_id, "deck", url("https://b.com/apps/deck/"), true);
    let deck = tab_of_app(&engine, 1, "deck");
    assert_eq!(shown(&engine.activate_workspace(first_workspace_id)), Some(calendar));
    let effects = engine.activate_workspace(second_workspace_id);
    assert_eq!(shown(&effects), Some(deck));
    assert!(created(&effects).is_empty());
}

#[test]
fn closing_selected_tab_selects_right_then_left_neighbour() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    let auth = engine.state.workspaces[0].tabs[0].id;
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    engine.open_app(workspace_id, "deck", url("https://a.com/apps/deck/"), true);
    let (files, deck) = (tab_of_app(&engine, 0, "files"), tab_of_app(&engine, 0, "deck"));
    engine.activate_tab(workspace_id, files);
    let effects = engine.close_tab(workspace_id, files);
    assert_eq!(destroyed(&effects), vec![files]);
    assert_eq!(shown(&effects), Some(deck));
    assert_eq!(shown(&engine.close_tab(workspace_id, deck)), Some(auth));
    let effects = engine.close_tab(workspace_id, auth);
    assert!(effects.contains(&Effect::HideContent));
    assert_eq!(engine.state.workspaces[0].active_tab_id, None);
}

#[test]
fn lru_evicts_least_recent_unpinned_tab_and_recreates_on_return() {
    let mut engine = engine_with(&["https://a.com"], 2);
    let workspace_id = workspace_at(&engine, 0);
    let auth = engine.state.workspaces[0].tabs[0].id;
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    let effects = engine.open_app(workspace_id, "deck", url("https://a.com/apps/deck/"), true);
    assert_eq!(destroyed(&effects), vec![auth]);
    assert!(!engine.is_live(auth));
    assert_eq!(created(&engine.activate_tab(workspace_id, auth)), vec![auth]);
}

#[test]
fn set_theme_saves_and_applies_it() {
    let mut engine = engine_with(&[], MAX_LIVE);
    assert_eq!(engine.set_theme(Appearance::Dark), vec![Effect::Theme(Appearance::Dark), Effect::Changed]);
    assert_eq!(engine.state.theme, Appearance::Dark);
}

#[test]
fn lru_skips_pinned_tabs() {
    let mut engine = engine_with(&["https://a.com"], 2);
    let workspace_id = workspace_at(&engine, 0);
    let auth = engine.state.workspaces[0].tabs[0].id;
    engine.set_pinned(workspace_id, auth, true);
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    let files = tab_of_app(&engine, 0, "files");
    let effects = engine.open_app(workspace_id, "deck", url("https://a.com/apps/deck/"), true);
    assert_eq!(destroyed(&effects), vec![files]);
    assert!(engine.is_live(auth));
}

#[test]
fn overlay_hides_content_and_restores_it() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    let effects = engine.set_overlay(true);
    assert!(effects.contains(&Effect::HideContent));
    assert_eq!(shown(&effects), None);
    assert_eq!(shown(&engine.set_overlay(false)), Some(tab_id));
}

#[test]
fn remove_workspace_destroys_tabs_clears_profile_and_selects_neighbour() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (first_workspace_id, second_workspace_id) = (workspace_at(&engine, 0), workspace_at(&engine, 1));
    let second_workspace_tab_id = engine.state.workspaces[1].tabs[0].id;
    let effects = engine.remove_workspace(second_workspace_id);
    assert_eq!(destroyed(&effects), vec![second_workspace_tab_id]);
    assert!(effects.contains(&Effect::ClearProfile { workspace_id: second_workspace_id, delete: true }));
    assert_eq!(engine.state.active_workspace_id, Some(first_workspace_id));
    let effects = engine.remove_workspace(first_workspace_id);
    assert_eq!(engine.state.active_workspace_id, None);
    assert!(effects.contains(&Effect::HideContent));
}

#[test]
fn reorder_requires_a_permutation() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (first_workspace_id, second_workspace_id) = (workspace_at(&engine, 0), workspace_at(&engine, 1));
    assert!(engine.reorder_workspaces(&[first_workspace_id]).is_empty());
    assert!(engine.reorder_workspaces(&[first_workspace_id, first_workspace_id]).is_empty());
    assert_eq!(engine.reorder_workspaces(&[second_workspace_id, first_workspace_id]), vec![Effect::Changed]);
    assert_eq!(workspace_at(&engine, 0), second_workspace_id);
}

#[test]
fn rename_sets_custom_name_and_empty_resets_to_host() {
    let mut engine = engine_with(&["https://cloud.a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    engine.rename_workspace(workspace_id, "  Soluce  ");
    assert_eq!((engine.state.workspaces[0].name.as_str(), engine.state.workspaces[0].name_custom), ("Soluce", true));
    engine.rename_workspace(workspace_id, " ");
    assert_eq!((engine.state.workspaces[0].name.as_str(), engine.state.workspaces[0].name_custom), ("cloud.a.com", false));
}

#[test]
fn clear_browsing_data_reloads_live_tabs_only() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    engine.state.workspaces[0].tabs.push(Tab::new("deck", "Deck", url("https://a.com/apps/deck/")));
    assert_eq!(
        engine.clear_browsing_data(workspace_id),
        vec![Effect::ClearProfile { workspace_id, delete: false }, Effect::Reload { workspace_id, tab_id }]
    );
}

#[test]
fn auth_tab_adopts_first_real_app_after_login() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    engine.observe_location(tab_id, url("https://a.com/login?redirect_url=/"));
    assert_eq!(engine.state.workspaces[0].tabs[0].app_id, AUTH);
    engine.observe_location(tab_id, url("https://a.com/apps/dashboard/"));
    let t = &engine.state.workspaces[0].tabs[0];
    assert_eq!((t.app_id.as_str(), t.url.as_str()), ("dashboard", "https://a.com/apps/dashboard/"));
}

#[test]
fn auth_tab_landing_on_open_app_merges_into_it() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    let files = tab_of_app(&engine, 0, "files");
    let auth = created(&engine.open_home(workspace_id))[0];
    let docs = url("https://a.com/apps/files/?dir=/Docs");
    let effects = engine.observe_location(auth, docs.clone());
    assert!(destroyed(&effects).contains(&auth));
    assert!(engine.state.workspaces[0].tab(auth).is_none());
    assert_eq!(shown(&effects), Some(files));
    assert!(effects.contains(&Effect::Navigate { workspace_id, tab_id: files, url: docs }));
}

#[test]
fn page_of_other_app_retags_tab_unless_that_app_is_open() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    engine.open_app(workspace_id, "deck", url("https://a.com/apps/deck/"), true);
    let files = tab_of_app(&engine, 0, "files");
    engine.observe_location(files, url("https://a.com/apps/deck/#/board/1"));
    assert_eq!(engine.state.workspaces[0].tab(files).unwrap().app_id, "files", "deck open elsewhere: tolerated duplicate");
    engine.observe_location(files, url("https://a.com/apps/calendar/"));
    assert_eq!(engine.state.workspaces[0].tab(files).unwrap().app_id, "calendar");
}

#[test]
fn location_report_updates_url_and_ignores_foreign_urls() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    engine.open_app(workspace_id, "files", url("https://a.com/apps/files/"), true);
    let files = tab_of_app(&engine, 0, "files");
    let photos = url("https://a.com/apps/files/?dir=/Photos");
    engine.observe_location(files, photos.clone());
    assert_eq!(engine.state.workspaces[0].tab(files).unwrap().url, photos);
    assert!(engine.observe_location(files, url("https://idp.example.com/auth")).is_empty());
    assert_eq!(engine.state.workspaces[0].tab(files).unwrap().url, photos);
}

#[test]
fn title_sets_tab_title_and_workspace_name_unless_custom() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    engine.observe_title(tab_id, "Documents - Files - Soluce Cloud");
    assert_eq!(engine.state.workspaces[0].tabs[0].title, "Documents - Files");
    assert_eq!(engine.state.workspaces[0].name, "Soluce Cloud");
    engine.observe_title(tab_id, "Files – Nextcloud");
    assert_eq!(engine.state.workspaces[0].name, "Soluce Cloud", "default instance name ignored");
    engine.rename_workspace(workspace_at(&engine, 0), "Mine");
    engine.observe_title(tab_id, "Files - Other");
    assert_eq!(engine.state.workspaces[0].name, "Mine");
}

#[test]
fn meta_keeps_only_safe_icon_and_workspace_app_links() {
    let mut engine = engine_with(&["https://a.com/nc"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    let link = |name: &str, href: &str| AppLink { name: name.into(), href: href.into() };
    engine.observe_meta(
        tab_id,
        Some("javascript:alert(1)".into()),
        vec![
            link("Files", "https://a.com/nc/apps/files/"),
            link("Files again", "https://a.com/nc/index.php/apps/files/"),
            link("Talk", "https://a.com/nc/apps/spreed/"),
            link("Evil", "https://evil.com/apps/x/"),
            link("OIDC", "https://a.com/nc/apps/user_oidc/"),
            link("", "https://a.com/nc/apps/deck/"),
        ],
    );
    let workspace = &engine.state.workspaces[0];
    assert_eq!(workspace.icon, None);
    assert_eq!(workspace.apps.iter().map(|app| app.id.as_str()).collect::<Vec<_>>(), vec!["files", "spreed"]);
    engine.observe_meta(tab_id, Some("data:image/png;base64,AAAA".into()), vec![]);
    let workspace = &engine.state.workspaces[0];
    assert_eq!(workspace.icon.as_deref(), Some("data:image/png;base64,AAAA"));
    assert_eq!(workspace.apps.len(), 2, "an empty report keeps the cache");
}

#[test]
fn new_window_to_other_workspace_switches_and_opens_app() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (first_workspace_id, second_workspace_id) = (workspace_at(&engine, 0), workspace_at(&engine, 1));
    engine.activate_workspace(first_workspace_id);
    let board = url("https://b.com/apps/deck/#/board/5");
    let effects = engine.on_new_window(&board);
    let deck = tab_of_app(&engine, 1, "deck");
    assert_eq!(engine.state.active_workspace_id, Some(second_workspace_id));
    assert_eq!(shown(&effects), Some(deck));
    assert_eq!(engine.state.workspaces[1].tab(deck).unwrap().url, board);
    let github = url("https://github.com/x");
    assert_eq!(engine.on_new_window(&github), vec![Effect::OpenExternal(github)]);
}

#[test]
fn new_window_links_give_documents_their_own_tabs_and_reuse_app_homes() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let selected = |engine: &Engine| engine.state.workspaces[0].active_tab_id.unwrap();
    let first_document = url("https://a.com/apps/eurooffice/1?filePath=%2Fa.docx");
    engine.on_new_window(&first_document);
    let first_document_tab = selected(&engine);
    engine.on_new_window(&url("https://a.com/apps/eurooffice/2?filePath=%2Fb.docx"));
    let second_document_tab = selected(&engine);
    assert_ne!(first_document_tab, second_document_tab, "a second document of the same app gets its own tab");
    let effects = engine.on_new_window(&first_document);
    assert_eq!(shown(&effects), Some(first_document_tab), "the same document selects its tab");
    assert!(created(&effects).is_empty());
    engine.on_new_window(&url("https://a.com/apps/files/?dir=/Photos"));
    let files = selected(&engine);
    engine.activate_tab(workspace_at(&engine, 0), first_document_tab);
    let effects = engine.on_new_window(&url("https://a.com/apps/files/"));
    assert_eq!(shown(&effects), Some(files));
    assert!(!effects.iter().any(|effect| matches!(effect, Effect::Navigate { .. })));
    assert_eq!(engine.state.workspaces[0].tabs.len(), 4, "auth + 2 documents + files");
}

#[test]
fn menu_opens_cached_app_and_home_opens_new_auth_tab() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    assert!(engine.open_app_from_menu(workspace_id, "deck").is_empty(), "unknown app");
    engine.state.workspaces[0].apps.push(AppEntry { id: "deck".into(), name: "Deck".into(), href: url("https://a.com/apps/deck/") });
    engine.open_app_from_menu(workspace_id, "deck");
    assert_eq!(engine.state.workspaces[0].tab_by_app("deck").unwrap().title, "Deck");
    engine.open_home(workspace_id);
    assert_eq!(engine.state.workspaces[0].tabs.iter().filter(|tab_id| tab_id.app_id == AUTH).count(), 2);
}

#[test]
fn reload_home_navigates_then_reloads() {
    let mut engine = engine_with(&["https://a.com/nc"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    engine.open_app(workspace_id, "files", url("https://a.com/nc/apps/files/?dir=/x"), true);
    let files = tab_of_app(&engine, 0, "files");
    let effects = engine.reload_home(workspace_id, files);
    assert!(effects.contains(&Effect::Navigate { workspace_id, tab_id: files, url: url("https://a.com/nc/apps/files/") }));
    assert!(engine.reload_home(workspace_id, files).contains(&Effect::Reload { workspace_id, tab_id: files }));
}

#[test]
fn location_merge_in_background_workspace_updates_that_workspaces_selection_only() {
    let mut engine = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (first_workspace_id, second_workspace_id) = (workspace_at(&engine, 0), workspace_at(&engine, 1));
    engine.open_app(first_workspace_id, "files", url("https://a.com/apps/files/"), true);
    let files = tab_of_app(&engine, 0, "files");
    let auth = created(&engine.open_home(first_workspace_id))[0];
    engine.activate_workspace(second_workspace_id);
    let docs = url("https://a.com/apps/files/?dir=/Docs");
    let effects = engine.observe_location(auth, docs);
    assert_eq!(engine.state.workspaces[0].active_tab_id, Some(files), "A's own selection moves to the merge target");
    assert_eq!(engine.state.active_workspace_id, Some(second_workspace_id), "background report never switches the shown workspace");
    assert!(engine.state.workspaces[0].tab(auth).is_none());
    assert!(destroyed(&effects).contains(&auth));
    let second_workspace_tab_id = engine.state.workspaces[1].tabs[0].id;
    assert_eq!(shown(&effects), Some(second_workspace_tab_id), "B, not A, stays on screen");
}

#[test]
fn meta_icon_size_boundary_enforced_and_previous_kept_on_reject() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    let prefix = "data:image/png;base64,";
    let largest_icon = format!("{prefix}{}", "A".repeat(MAX_ICON - prefix.len()));
    assert_eq!(largest_icon.len(), MAX_ICON);
    engine.observe_meta(tab_id, Some(largest_icon.clone()), vec![]);
    assert_eq!(engine.state.workspaces[0].icon.as_deref(), Some(largest_icon.as_str()));
    let too_big = format!("{largest_icon}A");
    engine.observe_meta(tab_id, Some(too_big), vec![]);
    assert_eq!(engine.state.workspaces[0].icon.as_deref(), Some(largest_icon.as_str()), "oversized icon rejected, previous kept");
}

#[test]
fn empty_icon_report_clears_the_server_icon_but_never_a_custom_one() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let (workspace_id, tab_id) = (workspace_at(&engine, 0), engine.state.workspaces[0].tabs[0].id);
    engine.observe_meta(tab_id, Some("data:image/png;base64,SERVER".into()), vec![]);
    engine.observe_meta(tab_id, None, vec![]);
    assert_eq!(engine.state.workspaces[0].icon.as_deref(), Some("data:image/png;base64,SERVER"), "unknown keeps it");
    engine.observe_meta(tab_id, Some(String::new()), vec![]);
    assert_eq!(engine.state.workspaces[0].icon, None, "no custom image on the server: initials");
    assert_eq!(engine.set_icon(workspace_id, Some("data:image/png;base64,MINE".into())), vec![Effect::Changed]);
    engine.observe_meta(tab_id, Some("data:image/png;base64,SERVER".into()), vec![]);
    engine.observe_meta(tab_id, Some(String::new()), vec![]);
    assert_eq!(engine.state.workspaces[0].icon.as_deref(), Some("data:image/png;base64,MINE"));
    assert!(engine.set_icon(workspace_id, Some("javascript:alert(1)".into())).is_empty(), "invalid icon rejected");
    engine.set_icon(workspace_id, None);
    assert!(!engine.state.workspaces[0].icon_custom);
    engine.observe_meta(tab_id, Some("data:image/png;base64,SERVER".into()), vec![]);
    assert_eq!(engine.state.workspaces[0].icon.as_deref(), Some("data:image/png;base64,SERVER"), "back to the server's");
}

#[test]
fn meta_caps_app_links_to_max_apps_in_input_order() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    let links: Vec<AppLink> = (0..70)
        .map(|index| AppLink { name: format!("App {index}"), href: format!("https://a.com/apps/app{index}/") })
        .collect();
    engine.observe_meta(tab_id, None, links);
    let workspace = &engine.state.workspaces[0];
    assert_eq!(workspace.apps.len(), MAX_APPS);
    let expected: Vec<String> = (0..MAX_APPS).map(|index| format!("app{index}")).collect();
    assert_eq!(workspace.apps.iter().map(|app| app.id.clone()).collect::<Vec<_>>(), expected);
}

#[test]
fn title_page_part_truncated_to_max_title() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    let long_page = "x".repeat(300);
    engine.observe_title(tab_id, &format!("{long_page} - Some Cloud"));
    let title = &engine.state.workspaces[0].tabs[0].title;
    assert_eq!(title.chars().count(), MAX_TITLE);
    assert_eq!(title, &"x".repeat(MAX_TITLE));
}

#[test]
fn offline_tab_is_hidden_until_retried_at_its_url() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let tab_id = engine.state.workspaces[0].tabs[0].id;
    let effects = engine.set_offline(tab_id);
    assert!(engine.is_offline(tab_id));
    assert_eq!(shown(&effects), None);
    assert!(effects.contains(&Effect::HideContent));
    assert!(engine.set_offline(tab_id).is_empty());
    let effects = engine.retry_tab(tab_id);
    assert!(!engine.is_offline(tab_id));
    assert!(effects.contains(&Effect::Navigate { workspace_id: workspace_at(&engine, 0), tab_id, url: url("https://a.com/") }));
    assert_eq!(shown(&effects), Some(tab_id));
    assert!(engine.retry_tab(tab_id).is_empty());
}

#[test]
fn reopen_tabs_navigates_only_live_tabs_to_their_url() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    let live_tab_id = engine.state.workspaces[0].tabs[0].id;
    engine.open_app(workspace_id, "deck", url("https://a.com/apps/deck/"), false);
    let deck = tab_of_app(&engine, 0, "deck");
    engine.activate_tab(workspace_id, live_tab_id);
    engine.destroy_webview(workspace_id, deck, &mut Vec::new());
    assert_eq!(engine.reopen_tabs(workspace_id), vec![Effect::Navigate { workspace_id, tab_id: live_tab_id, url: url("https://a.com/") }]);
}

#[test]
fn login_flow_reuses_the_sign_in_tab_then_goes_home() {
    let mut engine = engine_with(&["https://a.com"], MAX_LIVE);
    let workspace_id = workspace_at(&engine, 0);
    let auth = engine.state.workspaces[0].tabs[0].id;
    let (tab_id, effects) = engine.open_login(workspace_id, url("https://a.com/login/v2/flow/x"));
    assert_eq!(tab_id, auth);
    assert!(effects.contains(&Effect::Navigate { workspace_id, tab_id, url: url("https://a.com/login/v2/flow/x") }));
    let effects = engine.finish_login(workspace_id, tab_id, "me");
    assert_eq!(engine.state.workspaces[0].login.as_deref(), Some("me"));
    assert!(effects.contains(&Effect::Navigate { workspace_id, tab_id, url: url("https://a.com/") }));
    assert!(engine.set_login(workspace_id, Some("me".into())).is_empty());
    assert_eq!(engine.set_login(workspace_id, None), vec![Effect::Changed]);

    engine.observe_location(tab_id, url("https://a.com/apps/files/"));
    let (new_login_tab_id, _) = engine.open_login(workspace_id, url("https://a.com/login/v2/flow/y"));
    assert_ne!(new_login_tab_id, tab_id);
    assert_eq!(engine.state.workspaces[0].active_tab_id, Some(new_login_tab_id));
}
