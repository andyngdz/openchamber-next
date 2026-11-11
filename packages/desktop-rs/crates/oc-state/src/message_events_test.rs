use oc_client::EventData;
use oc_client::types::ToolStatus;
use serde_json::json;

use crate::fixture_test::{
    AGENT_TURN, INTERRUPTED_TURN, TEXT_REPLY, api_config, assistants, ended_texts, event, events,
    messages, opened, replay, session_of,
};
use crate::message::{ChatMessage, Delivery, Item, StepTiming, TextItem};
use crate::status::SessionStatus;

fn texts(items: &[Item]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text(TextItem { text, .. }) => Some(text.as_str()),
            Item::Tool(_) => None,
        })
        .collect()
}

fn tool_rows(items: &[Item]) -> Vec<(&str, ToolStatus)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Tool(tool) => Some((tool.name.as_str(), tool.status)),
            Item::Text(_) => None,
        })
        .collect()
}

#[test]
fn a_cancelled_inbox_message_leaves_the_chat() {
    let mut state = opened("ses_a");
    let payload = |inbox: &str| json!({"sessionID": "ses_a", "inboxID": inbox});
    let queued = |inbox: &str| json!({"sessionID": "ses_a", "inboxID": inbox, "item": {"payload": {"text": inbox}}});
    replay(
        &mut state,
        &[
            event("session.inbox.enqueued", 1, queued("msg_1")),
            event("session.inbox.enqueued", 2, queued("msg_2")),
            event("session.inbox.cancelled", 3, payload("msg_1")),
        ],
    );

    let [ChatMessage::User(left)] = messages(&state) else {
        panic!("only msg_2 should stay: {:?}", messages(&state));
    };
    assert_eq!(left.id.as_str(), "msg_2");
}

#[test]
fn full_text_reply_builds_the_user_message_and_one_assistant_step() {
    let session_id = session_of(TEXT_REPLY);
    let captured = events(TEXT_REPLY);
    let mut state = opened(&session_id);
    replay(&mut state, &captured);

    let [ChatMessage::User(user), ChatMessage::Assistant(reply)] = messages(&state) else {
        panic!("user then assistant expected: {:?}", messages(&state));
    };
    assert_eq!(user.text, "Reply with the fixture text.");
    assert_eq!(user.delivery, Delivery::Delivered);
    assert_eq!(texts(&reply.items), ended_texts(&captured));
    assert_eq!(
        Some(reply.agent.as_str()),
        api_config().default_agent.as_deref()
    );
    assert_eq!(reply.model.id, "stream-300cps");
    assert!(reply.timing.duration_ms().is_some());
    assert_eq!(state.open_status(), SessionStatus::Idle);
}

#[test]
fn text_grows_with_each_delta_while_the_session_is_busy() {
    let session_id = session_of(TEXT_REPLY);
    let captured = events(TEXT_REPLY);
    let last_delta = captured
        .iter()
        .rposition(|event| matches!(event.data, EventData::TextDelta(_)))
        .expect("the capture streams deltas");
    let mut state = opened(&session_id);
    replay(&mut state, &captured[..=last_delta]);

    let joined: String = captured[..=last_delta]
        .iter()
        .filter_map(|event| match &event.data {
            EventData::TextDelta(delta) => Some(delta.delta.as_str()),
            _ => None,
        })
        .collect();
    let steps = assistants(&state);
    let step = &steps[0];
    assert_eq!(texts(&step.items), [joined.as_str()]);
    assert!(matches!(step.timing, StepTiming::Running { .. }));
    assert_eq!(state.open_status(), SessionStatus::Busy);
}

#[test]
fn a_delta_before_its_start_creates_the_item() {
    let session_id = session_of(TEXT_REPLY);
    let captured = events(TEXT_REPLY);
    let without_start: Vec<_> = captured
        .iter()
        .filter(|event| !matches!(event.data, EventData::TextStarted(_)))
        .cloned()
        .collect();
    let first_delta = without_start
        .iter()
        .position(|event| matches!(event.data, EventData::TextDelta(_)))
        .expect("the capture streams deltas");
    let EventData::TextDelta(delta) = &without_start[first_delta].data else {
        panic!("text.delta expected");
    };
    let mut state = opened(&session_id);

    replay(&mut state, &without_start[..=first_delta]);
    assert_eq!(texts(&assistants(&state)[0].items), [delta.delta.as_str()]);

    replay(&mut state, &without_start[first_delta + 1..]);
    assert_eq!(texts(&assistants(&state)[0].items), ended_texts(&captured));
}

#[test]
fn agent_turn_has_one_assistant_message_per_step_and_twenty_finished_tools() {
    let session_id = session_of(AGENT_TURN);
    let mut state = opened(&session_id);
    replay(&mut state, &events(AGENT_TURN));

    let steps = assistants(&state);
    assert_eq!(steps.len(), 21);
    let tools: Vec<_> = steps
        .iter()
        .flat_map(|step| tool_rows(&step.items))
        .collect();
    assert_eq!(tools.len(), 20);
    assert!(
        tools
            .iter()
            .all(|row| *row == ("glob", ToolStatus::Completed))
    );
    let patterns: Vec<_> = steps
        .iter()
        .flat_map(|step| step.items.iter())
        .filter_map(|item| match item {
            Item::Tool(tool) => tool.input["pattern"].as_str(),
            Item::Text(_) => None,
        })
        .collect();
    assert_eq!(
        patterns.len(),
        20,
        "every row keeps the input it was called with"
    );
    assert_eq!(patterns[0], "packages/*/package.json");
    assert!(steps.iter().all(|step| step.timing.duration_ms().is_some()));
    assert!(tool_rows(&steps[20].items).is_empty());
    assert_eq!(state.open_status(), SessionStatus::Idle);
}

#[test]
fn interrupted_turn_settles_with_the_finished_tool() {
    let session_id = session_of(INTERRUPTED_TURN);
    let mut state = opened(&session_id);
    replay(&mut state, &events(INTERRUPTED_TURN));

    let steps = assistants(&state);
    assert_eq!(steps.len(), 1);
    assert_eq!(
        tool_rows(&steps[0].items),
        [("glob", ToolStatus::Completed)]
    );
    assert_eq!(state.open_status(), SessionStatus::Idle);
}

#[test]
fn a_failed_tool_shows_error_and_a_late_success_does_not_undo_it() {
    let mut state = opened("ses_a");
    let step = json!({"sessionID": "ses_a", "assistantMessageID": "msg_a"});
    let call = json!({"sessionID": "ses_a", "assistantMessageID": "msg_a", "id": "call_1"});
    let named = json!({"sessionID": "ses_a", "assistantMessageID": "msg_a", "id": "call_1", "name": "bash"});
    let failed = json!({"sessionID": "ses_a", "assistantMessageID": "msg_a", "id": "call_1", "error": {"type": "unknown", "message": "boom"}});
    replay(
        &mut state,
        &[
            event("session.step.started", 1, step),
            event("session.tool.input.started", 2, named),
            event("session.tool.called", 3, call.clone()),
            event("session.tool.failed", 4, failed),
            event("session.tool.success", 5, call),
        ],
    );

    assert_eq!(
        tool_rows(&assistants(&state)[0].items),
        [("bash", ToolStatus::Error)]
    );
}

#[test]
fn parts_of_a_step_never_started_are_dropped() {
    let mut state = opened("ses_a");
    let delta = json!({"sessionID": "ses_a", "assistantMessageID": "msg_missing", "ordinal": 0, "delta": "hi"});
    replay(&mut state, &[event("session.text.delta", 1, delta)]);

    assert!(messages(&state).is_empty());
}

#[test]
fn events_of_a_listed_session_that_is_not_open_leave_the_chat_column_alone() {
    let session_id = session_of(TEXT_REPLY);
    let mut state = opened("ses_other");
    state.apply(crate::action::Action::SessionsLoaded(vec![
        crate::fixture_test::session("ses_other", 1),
        crate::fixture_test::session(&session_id, 1),
    ]));
    replay(&mut state, &events(TEXT_REPLY));

    assert!(messages(&state).is_empty());
    let entry = state
        .sessions()
        .get(&session_id.as_str().into())
        .expect("listed");
    assert!(entry.title.is_some());
}
