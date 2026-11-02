use super::*;

fn notification(id: u64) -> Notification {
    Notification { notification_id: id, subject: format!("notification {id}") }
}

#[test]
fn only_notifications_newer_than_last_seen_make_banners() {
    let notifications = [notification(3), notification(7), notification(5), notification(6), notification(4)];
    assert!(fresh(&notifications, None).is_empty());
    let fresh_ids = |last_seen| {
        fresh(&notifications, Some(last_seen)).iter().map(|notification| notification.notification_id).collect::<Vec<_>>()
    };
    assert_eq!(fresh_ids(5), vec![7, 6]);
    assert_eq!(fresh_ids(0), vec![7, 6, 5]);
    assert!(fresh_ids(7).is_empty());
}

#[test]
fn ocs_notifications_parse() {
    let json = r#"{"ocs":{"meta":{"status":"ok"},"data":[{"notification_id":12,"app":"files","subject":"Hi","message":""}]}}"#;
    let response: OcsResponse<Vec<Notification>> = serde_json::from_str(json).unwrap();
    assert_eq!(response.ocs.data[0].notification_id, 12);
    assert_eq!(response.ocs.data[0].subject, "Hi");
}
