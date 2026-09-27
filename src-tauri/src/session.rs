use crate::urls;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Route {
    SignIn,
    SignOut,
}

pub fn route(url: &Url, base: &Url) -> Option<Route> {
    let rel = urls::relative_path(url, base)?;
    let rel = rel.strip_prefix("index.php").map(|r| r.trim_start_matches('/')).unwrap_or(rel);
    let segs: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        ["login"] => Some(Route::SignIn),
        ["logout"] => Some(Route::SignOut),
        _ => None,
    }
}

pub fn redirect_target(url: &Url, base: &Url) -> Option<Url> {
    let value = url.query_pairs().find(|(k, _)| k == "redirect_url")?.1;
    let target = base.join(&value).ok()?;
    (urls::belongs(&target, base) && route(&target, base).is_none()).then_some(target)
}

const WINDOW: Duration = Duration::from_secs(60);

const MAX_RETRIES: usize = 3;
const LOOP_WINDOW: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct RetryBudget(HashMap<Uuid, (Url, Instant)>, HashMap<Uuid, Vec<Instant>>);

impl RetryBudget {
    fn is_spent(&self, tab: Uuid, target: &Url, now: Instant) -> bool {
        self.0.get(&tab).is_some_and(|(t, at)| t == target && now.duration_since(*at) < WINDOW)
    }

    pub fn release_if_answered(&mut self, tab: Uuid, url: &Url) {
        if self.0.get(&tab).is_some_and(|(t, _)| t == url) {
            self.0.remove(&tab);
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Allow,
    Reauth(Url),
    Looping,
    Rejected,
    SignOut,
}

pub fn decide(url: &Url, base: &Url, signed_in: bool, budget: &mut RetryBudget, tab: Uuid, now: Instant) -> Decision {
    if !signed_in {
        return Decision::Allow;
    }
    match route(url, base) {
        None => Decision::Allow,
        Some(Route::SignOut) => Decision::SignOut,
        Some(Route::SignIn) => {
            let target = redirect_target(url, base).unwrap_or_else(|| base.clone());
            if budget.is_spent(tab, &target, now) {
                budget.0.remove(&tab);
                return Decision::Rejected;
            }
            let recent = budget.1.entry(tab).or_default();
            recent.retain(|at| now.saturating_duration_since(*at) < LOOP_WINDOW);
            if recent.len() >= MAX_RETRIES {
                return Decision::Looping;
            }
            recent.push(now);
            budget.0.insert(tab, (target.clone(), now));
            Decision::Reauth(target)
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/session.rs"]
mod tests;
