use super::*;

fn n(id: u64) -> Notification {
    Notification { notification_id: id, subject: format!("n{id}") }
}

#[test]
fn only_notifications_newer_than_seen_make_banners() {
    let list = [n(3), n(7), n(5), n(6), n(4)];
    assert!(fresh(&list, None).is_empty());
    let ids = |seen| fresh(&list, Some(seen)).iter().map(|n| n.notification_id).collect::<Vec<_>>();
    assert_eq!(ids(5), vec![7, 6]);
    assert_eq!(ids(0), vec![7, 6, 5]);
    assert!(ids(7).is_empty());
}

#[test]
fn ocs_notifications_parse() {
    let json = r#"{"ocs":{"meta":{"status":"ok"},"data":[{"notification_id":12,"app":"files","subject":"Hi","message":""}]}}"#;
    let o: Ocs<Vec<Notification>> = serde_json::from_str(json).unwrap();
    assert_eq!(o.ocs.data[0].notification_id, 12);
    assert_eq!(o.ocs.data[0].subject, "Hi");
}
