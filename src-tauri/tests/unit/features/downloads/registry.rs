use super::*;
use std::time::{Duration, Instant};
use uuid::Uuid;

fn started(registry: &mut Registry, now: Instant) -> Uuid {
    let id = Uuid::new_v4();
    registry.start(id, "a.bin".into(), Some("/d/a.bin".into()), true, now);
    id
}

#[test]
fn start_reports_an_active_download() {
    let mut registry = Registry::default();
    let now = Instant::now();
    let id = Uuid::new_v4();
    let download = registry.start(id, "a.bin".into(), Some("/d/a.bin".into()), true, now);
    assert_eq!(download.state, State::Active);
    assert_eq!((download.received, download.total), (0, None));
    assert_eq!(registry.list(), vec![download]);
}

#[test]
fn progress_is_throttled_by_time_and_by_one_percent() {
    let mut registry = Registry::default();
    let now = Instant::now();
    let id = started(&mut registry, now);
    assert!(registry.progress(id, 10, Some(1000), now).is_some(), "first progress is reported");
    assert!(registry.progress(id, 15, Some(1000), now + Duration::from_millis(50)).is_none());
    assert!(registry.progress(id, 25, Some(1000), now + Duration::from_millis(60)).is_some(), "1% step");
    assert!(registry.progress(id, 26, None, now + Duration::from_millis(100)).is_none());
    let late = registry.progress(id, 27, None, now + Duration::from_millis(400)).unwrap();
    assert_eq!(late.received, 27);
    assert_eq!(registry.list()[0].received, 27);
}

#[test]
fn finish_happens_once_and_maps_cancellation() {
    let mut registry = Registry::default();
    let now = Instant::now();
    let done = started(&mut registry, now);
    let cancelled = started(&mut registry, now);
    let failed = started(&mut registry, now);
    assert_eq!(registry.finish(done, Ok(())).unwrap().state, State::Completed);
    assert!(registry.finish(done, Err("late".into())).is_none());
    assert_eq!(registry.finish(cancelled, Err(CANCELLED.into())).unwrap().state, State::Cancelled);
    let failure = registry.finish(failed, Err("disk full".into())).unwrap();
    assert_eq!((failure.state, failure.error.as_deref()), (State::Failed, Some("disk full")));
    assert!(registry.progress(done, 5, None, now + Duration::from_secs(1)).is_none());
}

#[test]
fn only_recent_finished_downloads_are_kept() {
    let mut registry = Registry::default();
    let now = Instant::now();
    let active = started(&mut registry, now);
    for _ in 0..KEPT + 5 {
        let id = started(&mut registry, now);
        registry.finish(id, Ok(()));
    }
    let list = registry.list();
    assert_eq!(list.len(), KEPT);
    assert!(list.iter().any(|download| download.id == active));
}
