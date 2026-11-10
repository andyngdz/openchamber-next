//! The In work flag: read from the listed metadata at load, replaced by every
//! `session.metadata.updated`.

use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{api_sessions, directory, event, listed};
use crate::ids::SessionId;
use crate::state::State;

fn in_work(state: &State, id: &str) -> bool {
    state
        .sessions()
        .get(&SessionId::from(id))
        .is_some_and(|entry| entry.is_in_work())
}

fn metadata_update(session: &str, metadata: serde_json::Value) -> Action {
    Action::Event(event(
        "session.metadata.updated",
        7,
        json!({"sessionID": session, "metadata": metadata}),
    ))
}

#[test]
fn a_listed_session_with_open_work_starts_in_work() {
    let sessions = api_sessions("session-list-after-turn");
    let id = sessions[0].id.clone();
    let mut state = State::new(directory());

    state.apply(Action::SessionsLoaded(sessions));

    assert!(in_work(&state, &id));
}

#[test]
fn a_listed_session_without_work_metadata_is_not_in_work() {
    let sessions = api_sessions("session-list-nonempty");
    let id = sessions[0].id.clone();
    let mut state = State::new(directory());

    state.apply(Action::SessionsLoaded(sessions));

    assert!(!in_work(&state, &id));
}

#[test]
fn metadata_updates_replace_the_work_state() {
    let mut state = listed(&["ses_a", "ses_b"]);

    state.apply(metadata_update(
        "ses_a",
        json!({"openchamber": {"work": {"state": "open"}}}),
    ));
    assert!(in_work(&state, "ses_a"));
    assert!(!in_work(&state, "ses_b"));

    state.apply(metadata_update(
        "ses_a",
        json!({"openchamber": {"work": {"state": "done"}}}),
    ));
    assert!(!in_work(&state, "ses_a"));

    state.apply(metadata_update(
        "ses_a",
        json!({"openchamber": {"work": {"state": "open"}}}),
    ));
    state.apply(metadata_update(
        "ses_a",
        json!({"openchamber": {"turn": {"durationMs": 5}}}),
    ));
    assert!(!in_work(&state, "ses_a"));
}

#[test]
fn a_reload_takes_the_listed_work_state() {
    let mut state = listed(&["ses_a"]);
    state.apply(metadata_update(
        "ses_a",
        json!({"openchamber": {"work": {"state": "open"}}}),
    ));

    let reloaded = crate::fixture_test::session("ses_a", 2);
    state.apply(Action::SessionsLoaded(vec![reloaded]));

    assert!(!in_work(&state, "ses_a"));
}
