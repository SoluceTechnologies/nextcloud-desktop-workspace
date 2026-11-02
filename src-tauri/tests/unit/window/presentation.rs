use super::*;

#[test]
fn a_ready_tab_is_shown_at_once() {
    let mut shown = Shown::default();
    assert_eq!(shown.show(Some("a")), Some("a".into()));
}

#[test]
fn a_starting_tab_waits_until_ready() {
    let mut shown = Shown::default();
    shown.start("a");
    assert_eq!(shown.show(Some("a")), None);
    assert!(shown.ready("a"));
    assert_eq!(shown.show(Some("a")), Some("a".into()));
}

#[test]
fn a_tab_left_while_starting_stays_hidden_when_ready() {
    let mut shown = Shown::default();
    shown.start("a");
    shown.show(Some("a"));
    assert_eq!(shown.show(Some("b")), Some("b".into()));
    assert!(!shown.ready("a"));
}

#[test]
fn readiness_counts_once() {
    let mut shown = Shown::default();
    shown.start("a");
    shown.show(Some("a"));
    assert!(shown.ready("a"));
    assert!(!shown.ready("a"));
}

#[test]
fn hiding_all_content_shows_nothing() {
    let mut shown = Shown::default();
    assert_eq!(shown.show(None), None);
}
