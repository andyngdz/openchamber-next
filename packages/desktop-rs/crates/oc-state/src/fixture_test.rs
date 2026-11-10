//! Fixture loading and state builders shared by the tests.

use oc_client::filesystem::Home;
use oc_client::settings::Settings;
use oc_client::types::{Data, Message, Page, Session};
use oc_client::{Event, EventData};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::action::Action;
use crate::ids::SessionId;
use crate::message::{AssistantMessage, ChatMessage};
use crate::state::State;

const FIXTURES: &str = env!("OC_DESKTOP_FIXTURES");

/// Captured scenarios in `fixtures/events/`.
pub(crate) const TEXT_REPLY: &str = "text-reply-stream-300cps";
pub(crate) const AGENT_TURN: &str = "agent-20tools-300cps";
pub(crate) const INTERRUPTED_TURN: &str = "interrupted-agent-turn";
pub(crate) const PERMISSION_TURN: &str = "permission-asked-agent-turn";
pub(crate) const TWO_SESSIONS: &str = "two-sessions-stream-300cps";
pub(crate) const SESSION_LIFECYCLE: &str = "session-lifecycle";

fn read_json<T: DeserializeOwned>(relative: &str) -> T {
    let path = format!("{FIXTURES}/{relative}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// The events of one capture, in arrival order.
pub(crate) fn events(name: &str) -> Vec<Event> {
    read_json(&format!("events/{name}.json"))
}

/// The session ids `fixtures/events/index.json` names for a capture.
pub(crate) fn sessions_of(name: &str) -> Vec<String> {
    let index: Value = read_json("events/index.json");
    let sessions = index[name]["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    sessions
        .iter()
        .filter_map(|id| id.as_str().map(str::to_owned))
        .collect()
}

/// The one session of a single-session capture.
pub(crate) fn session_of(name: &str) -> String {
    sessions_of(name)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{name} names no session"))
}

/// The message list of `fixtures/api/<name>.json`, newest first.
pub(crate) fn api_messages(name: &str) -> Vec<Message> {
    let page: Page<Message> = read_json(&format!("api/{name}.json"));
    page.data
}

/// The session list of `fixtures/api/<name>.json`.
pub(crate) fn api_sessions(name: &str) -> Vec<Session> {
    let page: Page<Session> = read_json(&format!("api/{name}.json"));
    page.data
}

/// The captured settings document.
pub(crate) fn api_settings() -> Settings {
    read_json("api/settings.json")
}

/// The captured home info.
pub(crate) fn api_home() -> Home {
    read_json("api/fs-home.json")
}

/// Hand-built settings, with every key the test does not name taking its default.
pub(crate) fn settings(value: Value) -> Settings {
    serde_json::from_value(value).unwrap_or_else(|err| panic!("settings: {err}"))
}

/// Hand-built home info.
pub(crate) fn home(value: Value) -> Home {
    serde_json::from_value(value).unwrap_or_else(|err| panic!("home: {err}"))
}

/// A global-list session in `directory`, last updated at `updated` ms.
pub(crate) fn sidebar_session(id: &str, directory: &str, updated: u64) -> Session {
    sidebar_from(json!({
        "id": id,
        "location": {"directory": directory},
        "time": {"created": updated, "updated": updated},
    }))
}

/// A global-list session from a hand-built JSON object.
pub(crate) fn sidebar_from(value: Value) -> Session {
    serde_json::from_value(value).unwrap_or_else(|err| panic!("sidebar session: {err}"))
}

/// The project directory every capture ran in.
pub(crate) fn directory() -> String {
    let created: Data<Session> = read_json("api/session-create.json");
    created.data.location.directory
}

/// A session of the capture directory, last updated at `updated` ms.
pub(crate) fn session(id: &str, updated: u64) -> Session {
    let value = json!({
        "id": id,
        "location": {"directory": directory()},
        "time": {"created": updated, "updated": updated},
    });
    serde_json::from_value(value).unwrap_or_else(|err| panic!("session {id}: {err}"))
}

/// A hand-built wire event, for types the fixture provider never sends.
pub(crate) fn event(kind: &str, created: u64, data: Value) -> Event {
    let value = json!({"id": format!("evt_test_{created}"), "created": created, "type": kind, "data": data});
    serde_json::from_value(value).unwrap_or_else(|err| panic!("{kind}: {err}"))
}

/// State with `listed` sessions in the list and none open.
pub(crate) fn listed(listed: &[&str]) -> State {
    let mut state = State::new(directory());
    let sessions = listed.iter().map(|id| session(id, 1)).collect();
    state.apply(Action::SessionsLoaded(sessions));
    state
}

/// State with `session_id` listed and open with no messages, connected.
pub(crate) fn opened(session_id: &str) -> State {
    let mut state = listed(&[session_id]);
    open_empty(&mut state, session_id);
    state
}

/// Opens a session and answers its message request with an empty list.
pub(crate) fn open_empty(state: &mut State, session_id: &str) {
    let session = SessionId::from(session_id);
    state.apply(Action::OpenSession(session.clone()));
    state.apply(Action::MessagesLoaded {
        session,
        messages: Vec::new(),
    });
}

/// Feeds every event to the state.
pub(crate) fn replay(state: &mut State, events: &[Event]) {
    for event in events {
        state.apply(Action::Event(event.clone()));
    }
}

/// The open session's messages.
pub(crate) fn messages(state: &State) -> &[ChatMessage] {
    state.open().map_or(&[], |open| open.messages.as_slice())
}

/// The open session's assistant messages.
pub(crate) fn assistants(state: &State) -> Vec<&AssistantMessage> {
    messages(state)
        .iter()
        .filter_map(|message| match message {
            ChatMessage::Assistant(assistant) => Some(assistant),
            ChatMessage::User(_) => None,
        })
        .collect()
}

/// The full text of the capture's `session.text.ended` events, in order.
pub(crate) fn ended_texts(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match &event.data {
            EventData::TextEnded(ended) => Some(ended.text.clone()),
            _ => None,
        })
        .collect()
}
