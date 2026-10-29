use super::*;

#[test]
fn crash_reason_ignores_api_termination() {
    assert_eq!(crash_reason(WebProcessTerminationReason::Crashed), Some("crashed"));
    assert_eq!(crash_reason(WebProcessTerminationReason::ExceededMemoryLimit), Some("ran out of memory"));
    assert_eq!(crash_reason(WebProcessTerminationReason::TerminatedByApi), None);
}
