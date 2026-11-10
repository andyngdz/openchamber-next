use oc_client::Event;
use oc_state::{Action, SessionId};
use serde_json::json;

use super::WorkTracker;

fn event(kind: &str, data: serde_json::Value) -> Action {
    let event: Event =
        serde_json::from_value(json!({"id": "evt_1", "created": 1, "type": kind, "data": data}))
            .expect("event decodes");
    Action::Event(event)
}

fn work_update(session: &str, state: &str) -> Action {
    event(
        "session.metadata.updated",
        json!({"sessionID": session, "metadata": {"openchamber": {"work": {"state": state}}}}),
    )
}

#[test]
fn an_open_work_state_moves_the_session_into_work() {
    let mut tracker = WorkTracker::default();

    tracker.observe(&work_update("ses_a", "open"));

    assert!(tracker.is_in_work(&SessionId::from("ses_a")));
    assert!(!tracker.is_in_work(&SessionId::from("ses_b")));
}

#[test]
fn any_other_work_state_moves_it_back() {
    let mut tracker = WorkTracker::default();
    tracker.observe(&work_update("ses_a", "open"));

    tracker.observe(&work_update("ses_a", "done"));

    assert!(!tracker.is_in_work(&SessionId::from("ses_a")));
}

#[test]
fn metadata_without_work_and_other_events_change_nothing() {
    let mut tracker = WorkTracker::default();
    tracker.observe(&work_update("ses_a", "open"));

    tracker.observe(&event(
        "session.metadata.updated",
        json!({"sessionID": "ses_a", "metadata": {"openchamber": {"turn": {"durationMs": 5}}}}),
    ));
    tracker.observe(&event(
        "session.renamed",
        json!({"sessionID": "ses_a", "title": "x"}),
    ));
    tracker.observe(&Action::DraftChanged("text".into()));

    assert!(tracker.is_in_work(&SessionId::from("ses_a")));
}
