use crate::action::Action;
use crate::fixture_test::{
    AGENT_TURN, INTERRUPTED_TURN, PERMISSION_TURN, TEXT_REPLY, api_messages, api_sessions,
    directory, events, messages, opened, replay, session, session_of,
};
use crate::ids::SessionId;
use crate::message::ChatMessage;
use crate::state::State;
use crate::status::LoadStatus;

/// Loading the turn's message list shows the same messages as replaying its events.
fn assert_load_matches_replay(capture: &str, message_list: &str) {
    let session_id = session_of(capture);
    let mut replayed = opened(&session_id);
    replay(&mut replayed, &events(capture));

    let mut loaded = opened(&session_id);
    loaded.apply(Action::MessagesLoaded {
        session: SessionId::from(session_id.as_str()),
        messages: api_messages(message_list),
    });

    assert!(
        !messages(&loaded).is_empty(),
        "{message_list} loads messages"
    );
    assert_eq!(
        messages(&loaded),
        messages(&replayed),
        "{capture} vs {message_list}"
    );
}

#[test]
fn loaded_text_turn_matches_its_replayed_events() {
    assert_load_matches_replay(TEXT_REPLY, "messages-text-turn");
}

#[test]
fn loaded_agent_turn_matches_its_replayed_events() {
    assert_load_matches_replay(AGENT_TURN, "messages-agent-turn");
}

#[test]
fn loaded_interrupted_turn_matches_its_replayed_events() {
    assert_load_matches_replay(INTERRUPTED_TURN, "messages-interrupted-turn");
}

#[test]
fn loaded_permission_turn_matches_its_replayed_events() {
    assert_load_matches_replay(PERMISSION_TURN, "messages-permission-turn");
}

#[test]
fn loaded_messages_are_oldest_first_without_turn_markers() {
    let mut state = opened(&session_of(AGENT_TURN));
    state.apply(Action::MessagesLoaded {
        session: SessionId::from(session_of(AGENT_TURN).as_str()),
        messages: api_messages("messages-agent-turn"),
    });

    let loaded = messages(&state);
    assert!(matches!(loaded.first(), Some(ChatMessage::User(_))));
    assert!(matches!(loaded.last(), Some(ChatMessage::Assistant(_))));
    assert_eq!(loaded.len(), 22);
}

#[test]
fn session_list_keeps_the_directory_titles_and_marks_the_sidebar_ready() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(api_sessions(
        "session-list-after-turn",
    )));

    assert_eq!(state.sidebar(), &LoadStatus::Ready);
    let entry = state.sessions().iter().next().expect("one session");
    assert_eq!(entry.id.as_str(), session_of(TEXT_REPLY));
    assert!(entry.title.is_some());
}

#[test]
fn an_empty_list_is_ready_not_loading() {
    let mut state = State::new(directory());
    assert_eq!(state.sidebar(), &LoadStatus::Loading);
    state.apply(Action::SessionsLoaded(api_sessions("session-list-empty")));
    assert_eq!(state.sidebar(), &LoadStatus::Ready);
    assert!(state.sessions().is_empty());
}

#[test]
fn sessions_of_another_directory_are_not_listed() {
    let mut state = State::new(format!("{}-other", directory()));
    state.apply(Action::SessionsLoaded(api_sessions(
        "session-list-after-turn",
    )));
    assert!(state.sessions().is_empty());
}

#[test]
fn a_reload_keeps_live_status_and_newest_first_order() {
    let mut state = opened("ses_a");
    replay(
        &mut state,
        &[crate::fixture_test::event(
            "session.execution.started",
            100,
            serde_json::json!({"sessionID": "ses_a"}),
        )],
    );
    state.apply(Action::SessionsLoaded(vec![
        session("ses_a", 5),
        session("ses_b", 50),
    ]));

    let order: Vec<_> = state
        .sessions()
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(order, ["ses_a", "ses_b"]);
    assert!(state.open_entry().is_some_and(|entry| entry.is_busy()));
}

/// A session of the capture directory with the turn stamps the server sends.
fn turn_session(id: &str, busy: Option<u64>, idle: Option<u64>) -> oc_client::types::Session {
    let value = serde_json::json!({
        "id": id,
        "location": {"directory": directory()},
        "time": {"created": 1, "updated": 2, "busy": busy, "idle": idle},
    });
    serde_json::from_value(value).expect("session with turn stamps")
}

fn started(at: u64) -> oc_client::Event {
    crate::fixture_test::event(
        "session.execution.started",
        at,
        serde_json::json!({"sessionID": "ses_a"}),
    )
}

#[test]
fn a_reload_with_idle_after_busy_clears_a_stale_busy_status() {
    let mut state = opened("ses_a");
    state.connection = crate::status::ConnectionStatus::Connected;
    state.apply(Action::DraftChanged("next".to_owned()));
    replay(&mut state, &[started(100)]);
    assert!(!state.can_send(), "busy blocks sending");

    // The turn ended while the event stream was down: only the reload can tell.
    state.apply(Action::SessionsLoaded(vec![turn_session(
        "ses_a",
        Some(100),
        Some(200),
    )]));

    assert!(state.open_entry().is_some_and(|entry| !entry.is_busy()));
    assert!(state.can_send());
}

#[test]
fn a_reload_with_busy_after_idle_marks_the_session_busy() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![turn_session(
        "ses_a",
        Some(300),
        Some(200),
    )]));
    assert!(state.sessions().iter().all(|entry| entry.is_busy()));
}

#[test]
fn an_older_snapshot_does_not_override_a_newer_event() {
    let mut state = opened("ses_a");
    // The list was read at 200, when the turn had ended; a new turn started at 300 since.
    replay(&mut state, &[started(300)]);
    state.apply(Action::SessionsLoaded(vec![turn_session(
        "ses_a",
        Some(100),
        Some(200),
    )]));
    assert!(state.open_entry().is_some_and(|entry| entry.is_busy()));
}

#[test]
fn sessions_are_ordered_by_the_end_of_their_last_turn() {
    let mut state = State::new(directory());
    // `ses_old` was updated last (a rename) but its turn ended before `ses_new`'s did.
    let mut old = turn_session("ses_old", Some(10), Some(20));
    old.time.updated = 900;
    let mut new = turn_session("ses_new", Some(30), Some(40));
    new.time.updated = 500;
    let untouched = turn_session("ses_none", None, None);
    state.apply(Action::SessionsLoaded(vec![old, untouched, new]));

    let order: Vec<_> = state
        .sessions()
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    // `ses_none` has no turn stamp, so it falls back to its `updated` and sorts last.
    assert_eq!(order, ["ses_new", "ses_old", "ses_none"]);
}

fn asked(at: u64) -> oc_client::Event {
    crate::fixture_test::event(
        "permission.asked",
        at,
        serde_json::json!({"id": "per_1", "sessionID": "ses_a"}),
    )
}

#[test]
fn a_reload_of_an_idle_session_drops_its_pending_request() {
    let mut state = opened("ses_a");
    replay(&mut state, &[started(100), asked(110)]);
    assert!(state.open_is_waiting());

    // The reply and the turn's end were missed while the event stream was down.
    state.apply(Action::SessionsLoaded(vec![turn_session(
        "ses_a",
        Some(100),
        Some(200),
    )]));

    assert!(!state.open_is_waiting());
}

#[test]
fn a_reload_of_a_busy_session_keeps_its_pending_request() {
    let mut state = opened("ses_a");
    replay(&mut state, &[started(100), asked(110)]);

    state.apply(Action::SessionsLoaded(vec![turn_session(
        "ses_a",
        Some(100),
        Some(50),
    )]));

    assert!(state.open_is_waiting());
}
