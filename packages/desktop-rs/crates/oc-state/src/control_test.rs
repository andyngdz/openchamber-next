use oc_client::control::ControlEvent;
use oc_client::routing::{PermissionMode, RoutingPermissionHeld};

use crate::action::Action;
use crate::fixture_test::{
    api_auto_accept, api_routing, control_events, directory, sidebar_session,
};
use crate::state::State;

#[test]
fn applies_the_captured_auto_accept_policy() {
    let snapshot = control_events()
        .into_iter()
        .find_map(|event| match event {
            ControlEvent::AutoAcceptUpdated(snapshot) if !snapshot.modes.is_empty() => {
                Some(snapshot)
            }
            _ => None,
        })
        .expect("the capture sets a mode");
    let session = snapshot.modes.keys().next().expect("a session").clone();
    let mode = snapshot.modes[&session];
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        &session,
        &directory(),
        1,
    )]));
    state.control_event(&ControlEvent::AutoAcceptUpdated(snapshot));
    assert_eq!(state.effective_mode(&session), mode);
}

#[test]
fn the_captured_snapshots_read_as_they_were_saved() {
    let mut state = State::new(directory());
    state.auto_accept_loaded(api_auto_accept());
    state.routing_loaded(api_routing());
    assert_eq!(state.effective_mode("ses_a"), PermissionMode::Ask);
    assert!(!state.safety_net().available());
}

#[test]
fn control_frames_this_crate_does_not_act_on_change_nothing() {
    let mut state = State::new(directory());
    let before = state.clone();
    for event in control_events() {
        if matches!(
            event,
            ControlEvent::Other { .. } | ControlEvent::Heartbeat | ControlEvent::Ready
        ) {
            state.control_event(&event);
        }
    }
    assert_eq!(state, before);
}

#[test]
fn a_permission_held_frame_adds_a_safety_hold() {
    let mut state = State::new(directory());
    let held = RoutingPermissionHeld {
        permission_id: "per_held".to_owned(),
        session_id: "ses_a".to_owned(),
        directory: Some(directory()),
        score: 0.7,
        kind: Some("read_only".to_owned()),
    };
    state.control_event(&ControlEvent::PermissionHeld(held));
    assert_eq!(
        state
            .held_permission("per_held")
            .and_then(|hold| hold.kind.as_deref()),
        Some("read_only")
    );
    assert_eq!(state.effective_mode("ses_a"), PermissionMode::Ask);
}
