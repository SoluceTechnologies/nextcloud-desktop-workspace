use super::*;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn guard_returns_the_result() {
    assert_eq!(guard("test", || 2), Some(2));
}

#[test]
fn guard_contains_a_panic() {
    assert_eq!(guard("test", || -> i32 { panic!("boom") }), None);
}

#[test]
fn spawned_panic_does_not_reach_the_caller() {
    let (sender, receiver) = mpsc::channel::<()>();
    spawn("panicking-test-thread", move || {
        let _keep_sender_alive = &sender;
        panic!("boom");
    });
    assert_eq!(receiver.recv_timeout(Duration::from_secs(1)), Err(mpsc::RecvTimeoutError::Disconnected));
}

#[cfg(target_os = "linux")]
#[test]
fn glib_debug_is_trace_and_critical_is_error() {
    assert_eq!(glib_level(glib::LogLevel::Debug), log::Level::Trace);
    assert_eq!(glib_level(glib::LogLevel::Critical), log::Level::Error);
    assert_eq!(glib_level(glib::LogLevel::Warning), log::Level::Warn);
}
