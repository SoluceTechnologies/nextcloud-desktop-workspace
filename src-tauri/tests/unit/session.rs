use super::*;

fn u(s: &str) -> Url {
    Url::parse(s).unwrap()
}

#[test]
fn routes_are_one_component_under_the_workspace() {
    let base = u("https://x.com/nc");
    assert_eq!(route(&u("https://x.com/nc/login?redirect_url=/nc/apps/files/"), &base), Some(Route::SignIn));
    assert_eq!(route(&u("https://x.com/nc/index.php/login"), &base), Some(Route::SignIn));
    assert_eq!(route(&u("https://x.com/nc/index.php/logout?requesttoken=a"), &base), Some(Route::SignOut));
    assert_eq!(route(&u("https://x.com/nc/login/v2/flow/abc"), &base), None);
    assert_eq!(route(&u("https://x.com/nc/apps/files/login"), &base), None);
    assert_eq!(route(&u("https://x.com/login"), &base), None);
    assert_eq!(route(&u("http://x.com/nc/login"), &base), None);
    assert_eq!(route(&u("https://x.com:8443/nc/login"), &base), None);
}

#[test]
fn redirect_target_stays_in_the_workspace() {
    let base = u("https://x.com/");
    let t = |q: &str| redirect_target(&u(&format!("https://x.com/login?redirect_url={q}")), &base);
    assert_eq!(t("/index.php/apps/files/%3Fdir%3D/a"), Some(u("https://x.com/index.php/apps/files/?dir=/a")));
    assert_eq!(t("//evil.com/x"), None);
    assert_eq!(t("https://evil.com/x"), None);
    assert_eq!(t("/logout"), None);
    assert_eq!(t("/login"), None);
    assert_eq!(redirect_target(&u("https://x.com/login"), &base), None);
}

#[test]
fn one_silent_retry_then_rejected() {
    let base = u("https://x.com/");
    let login = u("https://x.com/login?redirect_url=/apps/files/");
    let target = u("https://x.com/apps/files/");
    let t0 = Instant::now();
    let fresh = || (RetryBudget::default(), Uuid::new_v4());

    let (mut b, tab) = fresh();
    assert_eq!(decide(&login, &base, false, &mut b, tab, t0), Decision::Allow);
    assert_eq!(decide(&login, &base, true, &mut b, tab, t0), Decision::Reauth(target.clone()));
    assert_eq!(decide(&login, &base, true, &mut b, tab, t0 + Duration::from_secs(1)), Decision::Rejected);

    let (mut b, tab) = fresh();
    assert_eq!(decide(&login, &base, true, &mut b, tab, t0), Decision::Reauth(target.clone()));
    b.release_if_answered(tab, &target);
    assert_eq!(decide(&login, &base, true, &mut b, tab, t0), Decision::Reauth(target.clone()));
    assert!(matches!(decide(&login, &base, true, &mut b, Uuid::new_v4(), t0), Decision::Reauth(_)));

    let (mut b, tab) = fresh();
    decide(&login, &base, true, &mut b, tab, t0);
    assert_eq!(decide(&login, &base, true, &mut b, tab, t0 + WINDOW), Decision::Reauth(target.clone()));

    let (mut b, tab) = fresh();
    for i in 0..MAX_RETRIES as u64 {
        assert!(matches!(decide(&login, &base, true, &mut b, tab, t0 + Duration::from_secs(i)), Decision::Reauth(_)));
        b.release_if_answered(tab, &target);
    }
    assert_eq!(decide(&login, &base, true, &mut b, tab, t0 + Duration::from_secs(10)), Decision::Looping);
    assert!(matches!(decide(&login, &base, true, &mut b, tab, t0 + LOOP_WINDOW + Duration::from_secs(10)), Decision::Reauth(_)));

    let (mut b, tab) = fresh();
    assert_eq!(decide(&u("https://x.com/logout"), &base, true, &mut b, tab, t0), Decision::SignOut);
    assert_eq!(decide(&u("https://x.com/apps/files/"), &base, true, &mut b, tab, t0), Decision::Allow);
}
