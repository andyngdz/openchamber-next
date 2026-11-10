use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{event, opened, replay};
use crate::state::State;
use crate::status::{ConnectionStatus, Region, SessionStatus};

fn ready_to_send() -> State {
    let mut state = opened("ses_a");
    state.apply(Action::Connection(ConnectionStatus::Connected));
    state.apply(Action::DraftChanged("hello".to_owned()));
    state
}

#[test]
fn a_new_state_is_starting_with_nothing_open() {
    let state = State::new("/project");
    assert_eq!(state.connection(), &ConnectionStatus::Starting);
    assert!(state.open().is_none());
    assert_eq!(state.open_status(), SessionStatus::Idle);
    assert!(!state.can_send());
    assert_eq!(state.region_error(Region::Sidebar), None);
}

#[test]
fn send_is_enabled_only_when_connected_idle_and_typed() {
    assert!(ready_to_send().can_send());

    let mut busy = ready_to_send();
    replay(
        &mut busy,
        &[event(
            "session.execution.started",
            1,
            json!({"sessionID": "ses_a"}),
        )],
    );
    assert!(!busy.can_send());

    let mut reconnecting = ready_to_send();
    reconnecting.apply(Action::Connection(ConnectionStatus::Reconnecting {
        attempt: 1,
    }));
    assert!(!reconnecting.can_send());

    let mut blank = ready_to_send();
    blank.apply(Action::DraftChanged("   ".to_owned()));
    assert!(!blank.can_send());

    let mut sending = ready_to_send();
    sending.apply(Action::PromptSent);
    assert!(!sending.can_send());
}
