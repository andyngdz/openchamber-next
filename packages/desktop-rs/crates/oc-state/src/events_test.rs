use oc_client::EventData;
use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{
    PERMISSION_TURN, SESSION_LIFECYCLE, TEXT_REPLY, TWO_SESSIONS, directory, event, events, listed,
    messages, opened, replay, session, session_of, sessions_of,
};
use crate::ids::{RequestId, SessionId};
use crate::session_entry::PendingRequest;
use crate::state::State;
use crate::status::SessionStatus;

fn order(state: &State) -> Vec<&str> {
    state
        .sessions()
        .iter()
        .map(|entry| entry.id.as_str())
        .collect()
}

#[test]
fn permission_asked_sets_the_waiting_flag_until_replied() {
    let session_id = session_of(PERMISSION_TURN);
    let captured = events(PERMISSION_TURN);
    let asked = captured
        .iter()
        .position(|event| matches!(event.data, EventData::PermissionAsked(_)))
        .expect("the capture asks");
    let EventData::PermissionAsked(request) = &captured[asked].data else {
        panic!("permission.asked expected");
    };
    let mut state = opened(&session_id);

    replay(&mut state, &captured[..=asked]);
    assert!(state.open_is_waiting());
    let entry = state.open_entry().expect("listed");
    assert_eq!(
        entry.pending,
        [PendingRequest::Permission(RequestId::from(
            request.id.as_str()
        ))]
    );

    replay(&mut state, &captured[asked + 1..]);
    assert!(!state.open_is_waiting());
    assert_eq!(state.open_status(), SessionStatus::Idle);
}

#[test]
fn a_form_waits_until_replied_or_cancelled() {
    let mut state = opened("ses_a");
    let form = |id: &str| json!({"form": {"id": id, "sessionID": "ses_a"}});
    let settled = |id: &str| json!({"id": id, "sessionID": "ses_a"});

    replay(
        &mut state,
        &[
            event("form.created", 1, form("frm_1")),
            event("form.created", 2, form("frm_2")),
        ],
    );
    assert_eq!(state.open_entry().map(|entry| entry.pending.len()), Some(2));

    replay(&mut state, &[event("form.replied", 3, settled("frm_1"))]);
    assert!(state.open_is_waiting());
    replay(&mut state, &[event("form.cancelled", 4, settled("frm_2"))]);
    assert!(!state.open_is_waiting());
}

#[test]
fn a_permission_reply_does_not_clear_a_form_with_the_same_id() {
    let mut state = opened("ses_a");
    replay(
        &mut state,
        &[
            event(
                "form.created",
                1,
                json!({"form": {"id": "req_1", "sessionID": "ses_a"}}),
            ),
            event(
                "permission.replied",
                2,
                json!({"requestID": "req_1", "sessionID": "ses_a"}),
            ),
        ],
    );
    assert!(state.open_is_waiting());
}

#[test]
fn execution_events_drive_the_session_status() {
    let mut state = opened("ses_a");
    let only = json!({"sessionID": "ses_a"});
    let status = |kind: &str| json!({"sessionID": "ses_a", "status": {"type": kind}});

    replay(
        &mut state,
        &[event("session.execution.started", 1, only.clone())],
    );
    assert_eq!(state.open_status(), SessionStatus::Busy);
    replay(
        &mut state,
        &[event(
            "session.execution.failed",
            2,
            json!({"sessionID": "ses_a", "error": {"message": "boom"}}),
        )],
    );
    assert_eq!(state.open_status(), SessionStatus::Error);
    replay(&mut state, &[event("session.status", 3, status("busy"))]);
    assert_eq!(state.open_status(), SessionStatus::Busy);
    replay(&mut state, &[event("session.status", 4, status("idle"))]);
    assert_eq!(state.open_status(), SessionStatus::Idle);
    replay(
        &mut state,
        &[
            event("session.status", 5, status("retry")),
            event("session.idle", 6, only),
        ],
    );
    assert_eq!(state.open_status(), SessionStatus::Idle);
}

#[test]
fn a_shutdown_interrupt_keeps_the_turn_running() {
    let mut state = opened("ses_a");
    replay(
        &mut state,
        &[
            event(
                "session.execution.started",
                1,
                json!({"sessionID": "ses_a"}),
            ),
            event(
                "session.execution.interrupted",
                2,
                json!({"sessionID": "ses_a", "reason": "shutdown"}),
            ),
        ],
    );
    assert_eq!(state.open_status(), SessionStatus::Busy);
}

#[test]
fn events_of_another_session_are_dropped() {
    let [first, second] = sessions_of(TWO_SESSIONS).try_into().expect("two sessions");
    let mut state = opened(&first);
    replay(&mut state, &events(TWO_SESSIONS));

    assert_eq!(order(&state), [first.as_str()]);
    assert!(
        state
            .sessions()
            .get(&SessionId::from(second.as_str()))
            .is_none()
    );
    assert_eq!(messages(&state).len(), 2);
}

#[test]
fn unknown_and_notice_events_change_nothing() {
    let session_id = session_of(TEXT_REPLY);
    let mut state = opened(&session_id);
    let before = state.clone();
    let notices: Vec<_> = events(TWO_SESSIONS)
        .into_iter()
        .filter(|event| {
            matches!(
                event.data,
                EventData::Raw { .. } | EventData::ServerConnected
            )
        })
        .chain([event(
            "session.made.up",
            1,
            json!({"sessionID": session_id}),
        )])
        .collect();
    assert!(notices.len() > 1);

    replay(&mut state, &notices);
    assert_eq!(state, before);
}

#[test]
fn a_created_session_joins_the_list_until_deleted() {
    let captured = events(SESSION_LIFECYCLE);
    let session_id = session_of(SESSION_LIFECYCLE);
    let mut state = listed(&[]);

    replay(&mut state, &captured[..2]);
    assert_eq!(order(&state), [session_id.as_str()]);
    assert_eq!(
        state
            .sessions()
            .iter()
            .next()
            .and_then(|entry| entry.title.clone()),
        None
    );

    replay(&mut state, &captured[2..]);
    assert!(state.sessions().is_empty());
}

#[test]
fn deleting_the_open_session_closes_it() {
    let mut state = opened("ses_a");
    replay(
        &mut state,
        &[event("session.deleted", 1, json!({"sessionID": "ses_a"}))],
    );
    assert!(state.open().is_none());
}

#[test]
fn a_session_created_in_another_directory_is_dropped() {
    let mut state = listed(&[]);
    let data =
        json!({"sessionID": "ses_x", "location": {"directory": format!("{}-other", directory())}});
    replay(&mut state, &[event("session.created", 1, data)]);
    assert!(state.sessions().is_empty());
}

#[test]
fn a_turn_start_or_end_moves_the_session_to_the_top_and_a_rename_does_not() {
    let mut state = listed(&[]);
    state.apply(Action::SessionsLoaded(vec![
        session("ses_old", 10),
        session("ses_new", 20),
    ]));
    assert_eq!(order(&state), ["ses_new", "ses_old"]);

    replay(
        &mut state,
        &[event(
            "session.renamed",
            30,
            json!({"sessionID": "ses_old", "title": "Renamed"}),
        )],
    );
    assert_eq!(order(&state), ["ses_new", "ses_old"]);

    replay(
        &mut state,
        &[event(
            "session.execution.started",
            40,
            json!({"sessionID": "ses_old"}),
        )],
    );
    assert_eq!(order(&state), ["ses_old", "ses_new"]);

    replay(
        &mut state,
        &[event(
            "session.execution.started",
            50,
            json!({"sessionID": "ses_new"}),
        )],
    );
    replay(
        &mut state,
        &[event(
            "session.execution.started",
            60,
            json!({"sessionID": "ses_old"}),
        )],
    );
    assert_eq!(
        order(&state),
        ["ses_new", "ses_old"],
        "a repeated busy does not move it"
    );
}
