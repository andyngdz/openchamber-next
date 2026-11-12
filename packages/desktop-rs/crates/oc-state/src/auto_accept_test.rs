use std::collections::BTreeMap;

use oc_client::EventData;
use oc_client::control::{ControlEvent, LeftForUser};
use oc_client::routing::{AutoAcceptSnapshot, PermissionMode, RoutingPermissionHeld, RoutingState};

use crate::action::Action;
use crate::fixture_test::{
    TURN_CONTROL, child_session, control_events, directory, events, listed, open_empty,
    sessions_of, sidebar_session,
};
use crate::ids::SessionId;
use crate::state::State;

fn routing_with_safety() -> RoutingState {
    RoutingState {
        available: true,
        jev_available: true,
        held_permissions: Vec::new(),
    }
}

fn snapshot(modes: &[(&str, PermissionMode)], revision: u64) -> AutoAcceptSnapshot {
    AutoAcceptSnapshot {
        sessions: BTreeMap::new(),
        modes: modes
            .iter()
            .map(|(session, mode)| ((*session).to_owned(), *mode))
            .collect(),
        revision,
    }
}

fn asked_for<'a>(events: &'a [oc_client::Event], session: &str) -> &'a oc_client::Event {
    events
        .iter()
        .find(|event| {
            matches!(&event.data, EventData::PermissionAsked(asked) if asked.session_id == session)
        })
        .expect("the capture asks in this session")
}

fn left_for_user(control: &[ControlEvent], session: &str) -> LeftForUser {
    control
        .iter()
        .find_map(|event| match event {
            ControlEvent::LeftForUser(left) if left.session_id == session => Some(left.clone()),
            _ => None,
        })
        .expect("the capture reports a request left for the user")
}

fn turned_control_state() -> (
    State,
    String,
    AutoAcceptSnapshot,
    Vec<oc_client::Event>,
    Vec<ControlEvent>,
) {
    let control = control_events();
    let captured = events(TURN_CONTROL);
    let (session, snapshot) = control
        .iter()
        .find_map(|event| match event {
            ControlEvent::AutoAcceptUpdated(snapshot) => snapshot
                .modes
                .keys()
                .next()
                .map(|session| (session.clone(), snapshot.clone())),
            _ => None,
        })
        .expect("the capture sets a mode");
    let ids: Vec<String> = sessions_of(TURN_CONTROL);
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut state = listed(&refs);
    open_empty(&mut state, &session);
    state.auto_accept_loaded(snapshot.clone());
    state.routing_loaded(routing_with_safety());
    (state, session, snapshot, captured, control)
}

#[test]
fn inherits_the_nearest_explicit_mode_up_the_lineage() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![
        sidebar_session("ses_parent", &directory(), 2),
        child_session("ses_child", "ses_parent"),
    ]));
    state.auto_accept_loaded(snapshot(&[("ses_parent", PermissionMode::Auto)], 1));
    assert_eq!(state.effective_mode("ses_child"), PermissionMode::Auto);
    assert_eq!(state.effective_mode("ses_unknown"), PermissionMode::Ask);
}

#[test]
fn a_childs_own_mode_overrides_its_parent() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![
        sidebar_session("ses_parent", &directory(), 2),
        child_session("ses_child", "ses_parent"),
    ]));
    state.auto_accept_loaded(snapshot(
        &[
            ("ses_parent", PermissionMode::Auto),
            ("ses_child", PermissionMode::Ask),
        ],
        1,
    ));
    assert_eq!(state.effective_mode("ses_child"), PermissionMode::Ask);
}

#[test]
fn an_older_snapshot_keeps_the_modes_already_held() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        "ses_a",
        &directory(),
        1,
    )]));
    state.auto_accept_loaded(snapshot(&[("ses_a", PermissionMode::Auto)], 2));
    state.auto_accept_loaded(snapshot(&[("ses_a", PermissionMode::Safety)], 1));
    assert_eq!(state.effective_mode("ses_a"), PermissionMode::Auto);
}

#[test]
fn the_cycle_skips_safety_only_while_the_net_is_unavailable() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        "ses_a",
        &directory(),
        1,
    )]));
    assert_eq!(state.next_mode("ses_a"), PermissionMode::Auto);

    state.routing_loaded(routing_with_safety());
    assert_eq!(state.next_mode("ses_a"), PermissionMode::Safety);
    state.auto_accept_loaded(snapshot(&[("ses_a", PermissionMode::Safety)], 1));
    assert_eq!(state.next_mode("ses_a"), PermissionMode::Auto);
}

#[test]
fn a_fresh_request_the_server_may_answer_waits_until_left_for_user() {
    let (mut state, session, _, captured, control) = turned_control_state();
    let asked = asked_for(&captured, &session);
    let request_id = match &asked.data {
        EventData::PermissionAsked(asked) => asked.id.clone(),
        _ => unreachable!(),
    };
    state.apply(Action::Event(asked.clone()));
    let open = SessionId::from(session.as_str());
    assert!(state.scoped_permissions(&open).is_empty());

    let left = left_for_user(&control, &session);
    assert_eq!(left.permission_id, request_id);
    state.control_event(&ControlEvent::LeftForUser(left));
    assert_eq!(state.scoped_permissions(&open).len(), 1);
}

#[test]
fn a_left_for_user_report_before_its_request_shows_it_at_once() {
    let (mut state, session, _, captured, control) = turned_control_state();
    state.control_event(&ControlEvent::LeftForUser(left_for_user(
        &control, &session,
    )));
    state.apply(Action::Event(asked_for(&captured, &session).clone()));
    let open = SessionId::from(session.as_str());
    assert_eq!(state.scoped_permissions(&open).len(), 1);
}

#[test]
fn replying_drops_a_waiting_request_and_its_early_report() {
    let (mut state, session, _, captured, control) = turned_control_state();
    state.apply(Action::Event(asked_for(&captured, &session).clone()));

    let replied = captured
        .iter()
        .find(|event| {
            matches!(&event.data, EventData::PermissionReplied(replied) if replied.session_id == session)
        })
        .expect("the capture replies in this session");
    state.apply(Action::Event(replied.clone()));
    state.control_event(&ControlEvent::LeftForUser(left_for_user(
        &control, &session,
    )));
    let open = SessionId::from(session.as_str());
    assert!(state.scoped_permissions(&open).is_empty());
}

#[test]
fn a_safety_hold_is_added_and_released_by_a_reply() {
    let mut state = State::new(directory());
    let held = RoutingPermissionHeld {
        permission_id: "per_held".to_owned(),
        session_id: "ses_a".to_owned(),
        directory: None,
        score: 0.9,
        kind: Some("writes_project".to_owned()),
    };
    state.control_event(&ControlEvent::PermissionHeld(held));
    let hold = state.held_permission("per_held").expect("hold");
    assert_eq!(hold.kind.as_deref(), Some("writes_project"));
    state.settle_permission("ses_a", "per_held");
    assert!(state.held_permission("per_held").is_none());
}

#[test]
fn routing_availability_drives_the_shields_safety_net() {
    let mut state = State::new(directory());
    assert!(!state.safety_net().available());
    state.routing_loaded(routing_with_safety());
    assert!(state.safety_net().available());
    state.control_event(&ControlEvent::RoutingUpdated(
        oc_client::routing::RoutingUpdated {
            available: true,
            jev_available: false,
        },
    ));
    assert!(!state.safety_net().available());
}

#[test]
fn a_fresh_request_is_shown_when_the_mode_is_ask() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        "ses_a",
        &directory(),
        1,
    )]));
    state.apply(Action::Event(crate::fixture_test::event(
        "permission.asked",
        1,
        serde_json::json!({"id": "per_1", "sessionID": "ses_a"}),
    )));
    let open = SessionId::from("ses_a");
    assert_eq!(state.scoped_permissions(&open).len(), 1);
}
