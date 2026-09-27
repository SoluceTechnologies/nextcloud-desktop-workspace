use crate::urls;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;
use uuid::Uuid;

const RETRY_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RETRIES: usize = 3;
const LOOP_WINDOW: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Route {
    SignIn,
    SignOut,
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Allow,
    Reauthenticate(Url),
    Looping,
    Rejected,
    SignOut,
}

pub fn route(url: &Url, base: &Url) -> Option<Route> {
    let relative = urls::relative_path(url, base)?;
    let relative = relative.strip_prefix("index.php").map(|rest| rest.trim_start_matches('/')).unwrap_or(relative);
    let segments: Vec<&str> = relative.split('/').filter(|segment| !segment.is_empty()).collect();
    match segments.as_slice() {
        ["login"] => Some(Route::SignIn),
        ["logout"] => Some(Route::SignOut),
        _ => None,
    }
}

pub fn redirect_target(url: &Url, base: &Url) -> Option<Url> {
    let (_, value) = url.query_pairs().find(|(key, _)| key == "redirect_url")?;
    let target = base.join(&value).ok()?;
    (urls::belongs(&target, base) && route(&target, base).is_none()).then_some(target)
}

#[derive(Default)]
pub struct RetryBudget {
    outstanding: HashMap<Uuid, (Url, Instant)>,
    recent: HashMap<Uuid, Vec<Instant>>,
}

impl RetryBudget {
    pub fn release_if_answered(&mut self, tab_id: Uuid, url: &Url) {
        if self.outstanding.get(&tab_id).is_some_and(|(target, _)| target == url) {
            self.outstanding.remove(&tab_id);
        }
    }

    fn is_spent(&self, tab_id: Uuid, target: &Url, now: Instant) -> bool {
        self.outstanding
            .get(&tab_id)
            .is_some_and(|(outstanding, issued_at)| outstanding == target && now.duration_since(*issued_at) < RETRY_TIMEOUT)
    }

    fn is_looping(&mut self, tab_id: Uuid, now: Instant) -> bool {
        let recent = self.recent.entry(tab_id).or_default();
        recent.retain(|retried_at| now.saturating_duration_since(*retried_at) < LOOP_WINDOW);
        recent.len() >= MAX_RETRIES
    }

    fn spend(&mut self, tab_id: Uuid, target: Url, now: Instant) {
        self.recent.entry(tab_id).or_default().push(now);
        self.outstanding.insert(tab_id, (target, now));
    }
}

pub fn decide(
    url: &Url,
    base: &Url,
    signed_in: bool,
    budget: &mut RetryBudget,
    tab_id: Uuid,
    now: Instant,
) -> Decision {
    if !signed_in {
        return Decision::Allow;
    }
    match route(url, base) {
        None => Decision::Allow,
        Some(Route::SignOut) => Decision::SignOut,
        Some(Route::SignIn) => {
            let target = redirect_target(url, base).unwrap_or_else(|| base.clone());
            if budget.is_spent(tab_id, &target, now) {
                budget.outstanding.remove(&tab_id);
                return Decision::Rejected;
            }
            if budget.is_looping(tab_id, now) {
                return Decision::Looping;
            }
            budget.spend(tab_id, target.clone(), now);
            Decision::Reauthenticate(target)
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/session.rs"]
mod tests;
