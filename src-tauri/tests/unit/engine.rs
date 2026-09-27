use super::*;

fn u(s: &str) -> Url {
    Url::parse(s).unwrap()
}

fn engine_with(bases: &[&str], max_live: usize) -> Engine {
    let mut e = Engine::new(AppState::default(), max_live);
    for b in bases {
        e.add_workspace(b).unwrap();
    }
    e
}

fn ws(e: &Engine, i: usize) -> Uuid {
    e.state.workspaces[i].id
}

fn tab_of_app(e: &Engine, i: usize, app: &str) -> Uuid {
    e.state.workspaces[i].tab_by_app(app).unwrap().id
}

fn shown(fx: &[Effect]) -> Option<Uuid> {
    fx.iter().rev().find_map(|f| match f {
        Effect::Show { tab, .. } => Some(*tab),
        _ => None,
    })
}

fn created(fx: &[Effect]) -> Vec<Uuid> {
    fx.iter().filter_map(|f| match f { Effect::Create { tab, .. } => Some(*tab), _ => None }).collect()
}

fn destroyed(fx: &[Effect]) -> Vec<Uuid> {
    fx.iter().filter_map(|f| match f { Effect::Destroy { tab, .. } => Some(*tab), _ => None }).collect()
}

#[test]
fn add_workspace_creates_and_shows_auth_tab() {
    let mut e = Engine::new(AppState::default(), MAX_LIVE);
    let fx = e.add_workspace("cloud.soluce.com").unwrap();
    let w = &e.state.workspaces[0];
    assert_eq!(w.base_url, u("https://cloud.soluce.com/"));
    assert_eq!(e.state.active_workspace_id, Some(w.id));
    let tab = w.tabs[0].id;
    assert_eq!(created(&fx), vec![tab]);
    assert_eq!(shown(&fx), Some(tab));
    assert_eq!(fx.last(), Some(&Effect::Changed));
}

#[test]
fn add_existing_url_activates_instead_of_duplicating() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    e.add_workspace(" https://A.com/index.php/apps/files ").unwrap();
    assert_eq!(e.state.workspaces.len(), 2);
    assert_eq!(e.state.active_workspace_id, Some(ws(&e, 0)));
}

#[test]
fn add_invalid_url_is_an_error() {
    let mut e = Engine::new(AppState::default(), MAX_LIVE);
    assert!(e.add_workspace("ftp://x.com").is_err());
    assert!(e.state.workspaces.is_empty());
}

#[test]
fn startup_creates_only_the_active_tab_and_repairs_selection() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let a = ws(&e, 0);
    e.open_app(a, "files", u("https://a.com/apps/files/"), true);
    let files = tab_of_app(&e, 0, "files");
    let mut fresh = Engine::new(e.state.clone(), MAX_LIVE);
    assert_eq!(created(&fresh.startup()), vec![files]);
    fresh.state.active_workspace_id = None;
    let mut again = Engine::new(fresh.state.clone(), MAX_LIVE);
    again.startup();
    assert_eq!(again.state.active_workspace_id, Some(a));
}

#[test]
fn open_app_creates_tab_once_then_reuses_it() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    let fx = e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    let files = tab_of_app(&e, 0, "files");
    assert_eq!(created(&fx), vec![files]);
    assert_eq!(shown(&fx), Some(files));
    let deep = u("https://a.com/apps/files/?dir=/Photos");
    let fx = e.open_app(w, "files", deep.clone(), true);
    assert!(created(&fx).is_empty());
    assert!(fx.contains(&Effect::Navigate { ws: w, tab: files, url: deep }));
    assert_eq!(e.state.workspaces[0].tabs.len(), 2);
}

#[test]
fn open_app_from_picker_does_not_navigate_existing_tab() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    e.open_app(w, "files", u("https://a.com/apps/files/?dir=/Photos"), true);
    let fx = e.open_app(w, "files", u("https://a.com/apps/files/"), false);
    assert!(!fx.iter().any(|f| matches!(f, Effect::Navigate { .. })));
}

#[test]
fn open_app_names_tab_from_app_cache() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    e.state.workspaces[0].apps.push(AppEntry { id: "spreed".into(), name: "Talk".into(), href: u("https://a.com/apps/spreed/") });
    e.open_app(ws(&e, 0), "spreed", u("https://a.com/apps/spreed/"), true);
    assert_eq!(e.state.workspaces[0].tab_by_app("spreed").unwrap().title, "Talk");
}

#[test]
fn switching_workspaces_restores_each_selected_tab_without_reload() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (a, b) = (ws(&e, 0), ws(&e, 1));
    e.open_app(a, "calendar", u("https://a.com/apps/calendar/"), true);
    let cal = tab_of_app(&e, 0, "calendar");
    e.open_app(b, "deck", u("https://b.com/apps/deck/"), true);
    let deck = tab_of_app(&e, 1, "deck");
    assert_eq!(shown(&e.activate_workspace(a)), Some(cal));
    let fx = e.activate_workspace(b);
    assert_eq!(shown(&fx), Some(deck));
    assert!(created(&fx).is_empty());
}

#[test]
fn closing_selected_tab_selects_right_then_left_neighbour() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    let auth = e.state.workspaces[0].tabs[0].id;
    e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
    let (files, deck) = (tab_of_app(&e, 0, "files"), tab_of_app(&e, 0, "deck"));
    e.activate_tab(w, files);
    let fx = e.close_tab(w, files);
    assert_eq!(destroyed(&fx), vec![files]);
    assert_eq!(shown(&fx), Some(deck));
    assert_eq!(shown(&e.close_tab(w, deck)), Some(auth));
    let fx = e.close_tab(w, auth);
    assert!(fx.contains(&Effect::HideContent));
    assert_eq!(e.state.workspaces[0].active_tab_id, None);
}

#[test]
fn lru_evicts_least_recent_unpinned_tab_and_recreates_on_return() {
    let mut e = engine_with(&["https://a.com"], 2);
    let w = ws(&e, 0);
    let auth = e.state.workspaces[0].tabs[0].id;
    e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    let fx = e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
    assert_eq!(destroyed(&fx), vec![auth]);
    assert!(!e.is_live(auth));
    assert_eq!(created(&e.activate_tab(w, auth)), vec![auth]);
}

#[test]
fn set_theme_saves_and_applies_it() {
    let mut e = engine_with(&[], MAX_LIVE);
    assert_eq!(e.set_theme(Appearance::Dark), vec![Effect::Theme(Appearance::Dark), Effect::Changed]);
    assert_eq!(e.state.theme, Appearance::Dark);
}

#[test]
fn lru_skips_pinned_tabs() {
    let mut e = engine_with(&["https://a.com"], 2);
    let w = ws(&e, 0);
    let auth = e.state.workspaces[0].tabs[0].id;
    e.set_pinned(w, auth, true);
    e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    let files = tab_of_app(&e, 0, "files");
    let fx = e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
    assert_eq!(destroyed(&fx), vec![files]);
    assert!(e.is_live(auth));
}

#[test]
fn overlay_hides_content_and_restores_it() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    let fx = e.set_overlay(true);
    assert!(fx.contains(&Effect::HideContent));
    assert_eq!(shown(&fx), None);
    assert_eq!(shown(&e.set_overlay(false)), Some(tab));
}

#[test]
fn remove_workspace_destroys_tabs_clears_profile_and_selects_neighbour() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (a, b) = (ws(&e, 0), ws(&e, 1));
    let b_tab = e.state.workspaces[1].tabs[0].id;
    let fx = e.remove_workspace(b);
    assert_eq!(destroyed(&fx), vec![b_tab]);
    assert!(fx.contains(&Effect::ClearProfile { ws: b, delete: true }));
    assert_eq!(e.state.active_workspace_id, Some(a));
    let fx = e.remove_workspace(a);
    assert_eq!(e.state.active_workspace_id, None);
    assert!(fx.contains(&Effect::HideContent));
}

#[test]
fn reorder_requires_a_permutation() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (a, b) = (ws(&e, 0), ws(&e, 1));
    assert!(e.reorder_workspaces(&[a]).is_empty());
    assert!(e.reorder_workspaces(&[a, a]).is_empty());
    assert_eq!(e.reorder_workspaces(&[b, a]), vec![Effect::Changed]);
    assert_eq!(ws(&e, 0), b);
}

#[test]
fn rename_sets_custom_name_and_empty_resets_to_host() {
    let mut e = engine_with(&["https://cloud.a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    e.rename_workspace(w, "  Soluce  ");
    assert_eq!((e.state.workspaces[0].name.as_str(), e.state.workspaces[0].name_custom), ("Soluce", true));
    e.rename_workspace(w, " ");
    assert_eq!((e.state.workspaces[0].name.as_str(), e.state.workspaces[0].name_custom), ("cloud.a.com", false));
}

#[test]
fn clear_browsing_data_reloads_live_tabs_only() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    let tab = e.state.workspaces[0].tabs[0].id;
    e.state.workspaces[0].tabs.push(Tab::new("deck", "Deck", u("https://a.com/apps/deck/")));
    assert_eq!(
        e.clear_browsing_data(w),
        vec![Effect::ClearProfile { ws: w, delete: false }, Effect::Reload { ws: w, tab }]
    );
}

#[test]
fn auth_tab_adopts_first_real_app_after_login() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    e.observe_location(tab, u("https://a.com/login?redirect_url=/"));
    assert_eq!(e.state.workspaces[0].tabs[0].app_id, AUTH);
    e.observe_location(tab, u("https://a.com/apps/dashboard/"));
    let t = &e.state.workspaces[0].tabs[0];
    assert_eq!((t.app_id.as_str(), t.url.as_str()), ("dashboard", "https://a.com/apps/dashboard/"));
}

#[test]
fn auth_tab_landing_on_open_app_merges_into_it() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    let files = tab_of_app(&e, 0, "files");
    let auth = created(&e.open_home(w))[0];
    let docs = u("https://a.com/apps/files/?dir=/Docs");
    let fx = e.observe_location(auth, docs.clone());
    assert!(destroyed(&fx).contains(&auth));
    assert!(e.state.workspaces[0].tab(auth).is_none());
    assert_eq!(shown(&fx), Some(files));
    assert!(fx.contains(&Effect::Navigate { ws: w, tab: files, url: docs }));
}

#[test]
fn page_of_other_app_retags_tab_unless_that_app_is_open() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    e.open_app(w, "deck", u("https://a.com/apps/deck/"), true);
    let files = tab_of_app(&e, 0, "files");
    e.observe_location(files, u("https://a.com/apps/deck/#/board/1"));
    assert_eq!(e.state.workspaces[0].tab(files).unwrap().app_id, "files", "deck open elsewhere: tolerated duplicate");
    e.observe_location(files, u("https://a.com/apps/calendar/"));
    assert_eq!(e.state.workspaces[0].tab(files).unwrap().app_id, "calendar");
}

#[test]
fn location_report_updates_url_and_ignores_foreign_urls() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    e.open_app(w, "files", u("https://a.com/apps/files/"), true);
    let files = tab_of_app(&e, 0, "files");
    let photos = u("https://a.com/apps/files/?dir=/Photos");
    e.observe_location(files, photos.clone());
    assert_eq!(e.state.workspaces[0].tab(files).unwrap().url, photos);
    assert!(e.observe_location(files, u("https://idp.example.com/auth")).is_empty());
    assert_eq!(e.state.workspaces[0].tab(files).unwrap().url, photos);
}

#[test]
fn title_sets_tab_title_and_workspace_name_unless_custom() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    e.observe_title(tab, "Documents - Files - Soluce Cloud");
    assert_eq!(e.state.workspaces[0].tabs[0].title, "Documents - Files");
    assert_eq!(e.state.workspaces[0].name, "Soluce Cloud");
    e.observe_title(tab, "Files – Nextcloud");
    assert_eq!(e.state.workspaces[0].name, "Soluce Cloud", "default instance name ignored");
    e.rename_workspace(ws(&e, 0), "Mine");
    e.observe_title(tab, "Files - Other");
    assert_eq!(e.state.workspaces[0].name, "Mine");
}

#[test]
fn meta_keeps_only_safe_icon_and_workspace_app_links() {
    let mut e = engine_with(&["https://a.com/nc"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    let link = |name: &str, href: &str| AppLink { name: name.into(), href: href.into() };
    e.observe_meta(
        tab,
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
    let w = &e.state.workspaces[0];
    assert_eq!(w.icon, None);
    assert_eq!(w.apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["files", "spreed"]);
    e.observe_meta(tab, Some("data:image/png;base64,AAAA".into()), vec![]);
    let w = &e.state.workspaces[0];
    assert_eq!(w.icon.as_deref(), Some("data:image/png;base64,AAAA"));
    assert_eq!(w.apps.len(), 2, "an empty report keeps the cache");
}

#[test]
fn new_window_to_other_workspace_switches_and_opens_app() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (a, b) = (ws(&e, 0), ws(&e, 1));
    e.activate_workspace(a);
    let board = u("https://b.com/apps/deck/#/board/5");
    let fx = e.on_new_window(&board);
    let deck = tab_of_app(&e, 1, "deck");
    assert_eq!(e.state.active_workspace_id, Some(b));
    assert_eq!(shown(&fx), Some(deck));
    assert_eq!(e.state.workspaces[1].tab(deck).unwrap().url, board);
    let gh = u("https://github.com/x");
    assert_eq!(e.on_new_window(&gh), vec![Effect::OpenExternal(gh)]);
}

#[test]
fn new_window_links_give_documents_their_own_tabs_and_reuse_app_homes() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let selected = |e: &Engine| e.state.workspaces[0].active_tab_id.unwrap();
    let doc1 = u("https://a.com/apps/eurooffice/1?filePath=%2Fa.docx");
    e.on_new_window(&doc1);
    let t1 = selected(&e);
    e.on_new_window(&u("https://a.com/apps/eurooffice/2?filePath=%2Fb.docx"));
    let t2 = selected(&e);
    assert_ne!(t1, t2, "a second document of the same app gets its own tab");
    let fx = e.on_new_window(&doc1);
    assert_eq!(shown(&fx), Some(t1), "the same document selects its tab");
    assert!(created(&fx).is_empty());
    e.on_new_window(&u("https://a.com/apps/files/?dir=/Photos"));
    let files = selected(&e);
    e.activate_tab(ws(&e, 0), t1);
    let fx = e.on_new_window(&u("https://a.com/apps/files/"));
    assert_eq!(shown(&fx), Some(files));
    assert!(!fx.iter().any(|f| matches!(f, Effect::Navigate { .. })));
    assert_eq!(e.state.workspaces[0].tabs.len(), 4, "auth + 2 documents + files");
}

#[test]
fn menu_opens_cached_app_and_home_opens_new_auth_tab() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    assert!(e.open_app_from_menu(w, "deck").is_empty(), "unknown app");
    e.state.workspaces[0].apps.push(AppEntry { id: "deck".into(), name: "Deck".into(), href: u("https://a.com/apps/deck/") });
    e.open_app_from_menu(w, "deck");
    assert_eq!(e.state.workspaces[0].tab_by_app("deck").unwrap().title, "Deck");
    e.open_home(w);
    assert_eq!(e.state.workspaces[0].tabs.iter().filter(|t| t.app_id == AUTH).count(), 2);
}

#[test]
fn reload_home_navigates_then_reloads() {
    let mut e = engine_with(&["https://a.com/nc"], MAX_LIVE);
    let w = ws(&e, 0);
    e.open_app(w, "files", u("https://a.com/nc/apps/files/?dir=/x"), true);
    let files = tab_of_app(&e, 0, "files");
    let fx = e.reload_home(w, files);
    assert!(fx.contains(&Effect::Navigate { ws: w, tab: files, url: u("https://a.com/nc/apps/files/") }));
    assert!(e.reload_home(w, files).contains(&Effect::Reload { ws: w, tab: files }));
}

#[test]
fn location_merge_in_background_workspace_updates_that_workspaces_selection_only() {
    let mut e = engine_with(&["https://a.com", "https://b.com"], MAX_LIVE);
    let (a, b) = (ws(&e, 0), ws(&e, 1));
    e.open_app(a, "files", u("https://a.com/apps/files/"), true);
    let files = tab_of_app(&e, 0, "files");
    let auth = created(&e.open_home(a))[0];
    e.activate_workspace(b);
    let docs = u("https://a.com/apps/files/?dir=/Docs");
    let fx = e.observe_location(auth, docs);
    assert_eq!(e.state.workspaces[0].active_tab_id, Some(files), "A's own selection moves to the merge target");
    assert_eq!(e.state.active_workspace_id, Some(b), "background report never switches the shown workspace");
    assert!(e.state.workspaces[0].tab(auth).is_none());
    assert!(destroyed(&fx).contains(&auth));
    let b_tab = e.state.workspaces[1].tabs[0].id;
    assert_eq!(shown(&fx), Some(b_tab), "B, not A, stays on screen");
}

#[test]
fn meta_icon_size_boundary_enforced_and_previous_kept_on_reject() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    let prefix = "data:image/png;base64,";
    let ok = format!("{prefix}{}", "A".repeat(MAX_ICON - prefix.len()));
    assert_eq!(ok.len(), MAX_ICON);
    e.observe_meta(tab, Some(ok.clone()), vec![]);
    assert_eq!(e.state.workspaces[0].icon.as_deref(), Some(ok.as_str()));
    let too_big = format!("{ok}A");
    e.observe_meta(tab, Some(too_big), vec![]);
    assert_eq!(e.state.workspaces[0].icon.as_deref(), Some(ok.as_str()), "oversized icon rejected, previous kept");
}

#[test]
fn empty_icon_report_clears_the_server_icon_but_never_a_custom_one() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let (w, tab) = (ws(&e, 0), e.state.workspaces[0].tabs[0].id);
    e.observe_meta(tab, Some("data:image/png;base64,SERVER".into()), vec![]);
    e.observe_meta(tab, None, vec![]);
    assert_eq!(e.state.workspaces[0].icon.as_deref(), Some("data:image/png;base64,SERVER"), "unknown keeps it");
    e.observe_meta(tab, Some(String::new()), vec![]);
    assert_eq!(e.state.workspaces[0].icon, None, "no custom image on the server: initials");
    assert_eq!(e.set_icon(w, Some("data:image/png;base64,MINE".into())), vec![Effect::Changed]);
    e.observe_meta(tab, Some("data:image/png;base64,SERVER".into()), vec![]);
    e.observe_meta(tab, Some(String::new()), vec![]);
    assert_eq!(e.state.workspaces[0].icon.as_deref(), Some("data:image/png;base64,MINE"));
    assert!(e.set_icon(w, Some("javascript:alert(1)".into())).is_empty(), "invalid icon rejected");
    e.set_icon(w, None);
    assert!(!e.state.workspaces[0].icon_custom);
    e.observe_meta(tab, Some("data:image/png;base64,SERVER".into()), vec![]);
    assert_eq!(e.state.workspaces[0].icon.as_deref(), Some("data:image/png;base64,SERVER"), "back to the server's");
}

#[test]
fn meta_caps_app_links_to_max_apps_in_input_order() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    let links: Vec<AppLink> = (0..70)
        .map(|i| AppLink { name: format!("App {i}"), href: format!("https://a.com/apps/app{i}/") })
        .collect();
    e.observe_meta(tab, None, links);
    let w = &e.state.workspaces[0];
    assert_eq!(w.apps.len(), MAX_APPS);
    let want: Vec<String> = (0..MAX_APPS).map(|i| format!("app{i}")).collect();
    assert_eq!(w.apps.iter().map(|a| a.id.clone()).collect::<Vec<_>>(), want);
}

#[test]
fn title_page_part_truncated_to_max_title() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let tab = e.state.workspaces[0].tabs[0].id;
    let long_page = "x".repeat(300);
    e.observe_title(tab, &format!("{long_page} - Some Cloud"));
    let title = &e.state.workspaces[0].tabs[0].title;
    assert_eq!(title.chars().count(), MAX_TITLE);
    assert_eq!(title, &"x".repeat(MAX_TITLE));
}

#[test]
fn login_flow_reuses_the_sign_in_tab_then_goes_home() {
    let mut e = engine_with(&["https://a.com"], MAX_LIVE);
    let w = ws(&e, 0);
    let auth = e.state.workspaces[0].tabs[0].id;
    let (tab, fx) = e.open_login(w, u("https://a.com/login/v2/flow/x"));
    assert_eq!(tab, auth);
    assert!(fx.contains(&Effect::Navigate { ws: w, tab, url: u("https://a.com/login/v2/flow/x") }));
    let fx = e.finish_login(w, tab, "me");
    assert_eq!(e.state.workspaces[0].login.as_deref(), Some("me"));
    assert!(fx.contains(&Effect::Navigate { ws: w, tab, url: u("https://a.com/") }));
    assert!(e.set_login(w, Some("me".into())).is_empty());
    assert_eq!(e.set_login(w, None), vec![Effect::Changed]);

    e.observe_location(tab, u("https://a.com/apps/files/"));
    let (other, _) = e.open_login(w, u("https://a.com/login/v2/flow/y"));
    assert_ne!(other, tab);
    assert_eq!(e.state.workspaces[0].active_tab_id, Some(other));
}
