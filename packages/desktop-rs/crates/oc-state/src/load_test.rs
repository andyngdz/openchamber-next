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
