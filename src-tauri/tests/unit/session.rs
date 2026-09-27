use super::*;

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn base() -> Url {
    url("https://x.com/")
}

fn login_page() -> Url {
    url("https://x.com/login?redirect_url=/apps/files/")
}

fn files() -> Url {
    url("https://x.com/apps/files/")
}

fn seconds(count: u64) -> Duration {
    Duration::from_secs(count)
}

#[test]
fn routes_are_one_component_under_the_workspace() {
    let base = url("https://x.com/nc");
    assert_eq!(route(&url("https://x.com/nc/login?redirect_url=/nc/apps/files/"), &base), Some(Route::SignIn));
    assert_eq!(route(&url("https://x.com/nc/index.php/login"), &base), Some(Route::SignIn));
    assert_eq!(route(&url("https://x.com/nc/index.php/logout?requesttoken=a"), &base), Some(Route::SignOut));
    assert_eq!(route(&url("https://x.com/nc/login/v2/flow/abc"), &base), None);
    assert_eq!(route(&url("https://x.com/nc/apps/files/login"), &base), None);
    assert_eq!(route(&url("https://x.com/login"), &base), None);
    assert_eq!(route(&url("http://x.com/nc/login"), &base), None);
    assert_eq!(route(&url("https://x.com:8443/nc/login"), &base), None);
}

#[test]
fn redirect_target_stays_in_the_workspace() {
    let target_of = |value: &str| redirect_target(&url(&format!("https://x.com/login?redirect_url={value}")), &base());
    assert_eq!(
        target_of("/index.php/apps/files/%3Fdir%3D/a"),
        Some(url("https://x.com/index.php/apps/files/?dir=/a"))
    );
    assert_eq!(target_of("//evil.com/x"), None);
    assert_eq!(target_of("https://evil.com/x"), None);
    assert_eq!(target_of("/logout"), None);
    assert_eq!(target_of("/login"), None);
    assert_eq!(redirect_target(&url("https://x.com/login"), &base()), None);
}

#[test]
fn workspace_without_app_password_is_never_intercepted() {
    let mut budget = RetryBudget::default();
    let decision = decide(&login_page(), &base(), false, &mut budget, Uuid::new_v4(), Instant::now());
    assert_eq!(decision, Decision::Allow);
}

#[test]
fn second_sign_in_form_for_the_same_retry_means_rejected() {
    let (mut budget, tab_id, start) = (RetryBudget::default(), Uuid::new_v4(), Instant::now());
    assert_eq!(decide(&login_page(), &base(), true, &mut budget, tab_id, start), Decision::Reauthenticate(files()));
    assert_eq!(decide(&login_page(), &base(), true, &mut budget, tab_id, start + seconds(1)), Decision::Rejected);
}

#[test]
fn answered_retry_allows_a_later_one() {
    let (mut budget, tab_id, start) = (RetryBudget::default(), Uuid::new_v4(), Instant::now());
    decide(&login_page(), &base(), true, &mut budget, tab_id, start);
    budget.release_if_answered(tab_id, &files());
    assert_eq!(decide(&login_page(), &base(), true, &mut budget, tab_id, start), Decision::Reauthenticate(files()));
}

#[test]
fn each_tab_has_its_own_budget() {
    let (mut budget, start) = (RetryBudget::default(), Instant::now());
    decide(&login_page(), &base(), true, &mut budget, Uuid::new_v4(), start);
    let other_tab = decide(&login_page(), &base(), true, &mut budget, Uuid::new_v4(), start);
    assert_eq!(other_tab, Decision::Reauthenticate(files()));
}

#[test]
fn retry_lost_past_the_timeout_is_not_a_rejection() {
    let (mut budget, tab_id, start) = (RetryBudget::default(), Uuid::new_v4(), Instant::now());
    decide(&login_page(), &base(), true, &mut budget, tab_id, start);
    let later = decide(&login_page(), &base(), true, &mut budget, tab_id, start + RETRY_TIMEOUT);
    assert_eq!(later, Decision::Reauthenticate(files()));
}

#[test]
fn stops_retrying_a_tab_that_keeps_losing_its_session() {
    let (mut budget, tab_id, start) = (RetryBudget::default(), Uuid::new_v4(), Instant::now());
    for attempt in 0..MAX_RETRIES as u64 {
        let decision = decide(&login_page(), &base(), true, &mut budget, tab_id, start + seconds(attempt));
        assert_eq!(decision, Decision::Reauthenticate(files()));
        budget.release_if_answered(tab_id, &files());
    }
    assert_eq!(decide(&login_page(), &base(), true, &mut budget, tab_id, start + seconds(10)), Decision::Looping);
    let after_window = start + LOOP_WINDOW + seconds(10);
    assert_eq!(decide(&login_page(), &base(), true, &mut budget, tab_id, after_window), Decision::Reauthenticate(files()));
}

#[test]
fn logout_signs_out_and_other_pages_pass() {
    let (mut budget, tab_id, now) = (RetryBudget::default(), Uuid::new_v4(), Instant::now());
    assert_eq!(decide(&url("https://x.com/logout"), &base(), true, &mut budget, tab_id, now), Decision::SignOut);
    assert_eq!(decide(&files(), &base(), true, &mut budget, tab_id, now), Decision::Allow);
}
