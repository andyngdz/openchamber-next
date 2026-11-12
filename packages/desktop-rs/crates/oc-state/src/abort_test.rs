use crate::ABORT_WINDOW_MS;
use crate::fixture_test::opened;
use crate::ids::SessionId;

#[test]
fn the_second_escape_inside_the_window_confirms_once() {
    let mut state = opened("ses_a");
    let session = SessionId::from("ses_a");

    state.arm_abort(&session, 1000);
    assert!(state.confirm_abort(&session, 3999));
    assert!(!state.confirm_abort(&session, 4000));
}

#[test]
fn exactly_the_window_length_is_outside() {
    let mut state = opened("ses_a");
    let session = SessionId::from("ses_a");

    state.arm_abort(&session, 1000);
    assert!(!state.confirm_abort(&session, 1000 + ABORT_WINDOW_MS));
}

#[test]
fn another_session_does_not_confirm() {
    let mut state = opened("ses_a");
    let armed = SessionId::from("ses_a");
    let other = SessionId::from("ses_b");

    state.arm_abort(&armed, 1000);
    assert!(!state.confirm_abort(&other, 1500));
    assert!(state.confirm_abort(&armed, 1500));
}

#[test]
fn a_confirmation_without_an_armed_window_sends_nothing() {
    let mut state = opened("ses_a");
    let session = SessionId::from("ses_a");

    assert!(!state.confirm_abort(&session, 1000));
}

#[test]
fn a_later_first_escape_restarts_the_window() {
    let mut state = opened("ses_a");
    let session = SessionId::from("ses_a");

    state.arm_abort(&session, 1000);
    state.arm_abort(&session, 3500);
    assert!(state.confirm_abort(&session, 5000));
}
