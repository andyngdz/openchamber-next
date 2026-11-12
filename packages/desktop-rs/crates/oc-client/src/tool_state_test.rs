use crate::types::{AssistantContent, Message, Page};

use super::{ToolErrorSource, ToolStatus};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn decode<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

#[test]
fn a_failed_question_keeps_its_nested_error_shape() {
    let page: Page<Message> = decode("messages-question-interrupted-turn.json");
    let state = page
        .data
        .iter()
        .find_map(|message| match message {
            Message::Assistant { content, .. } => content.iter().find_map(|entry| match entry {
                AssistantContent::Tool { name, state, .. } if name == "question" => Some(state),
                _ => None,
            }),
            _ => None,
        })
        .expect("the captured question call");
    assert_eq!(state.status, ToolStatus::Error);
    assert_eq!(state.error_message(), Some("Tool execution interrupted"));
    assert_eq!(state.error_kind(), Some("aborted"));
    assert!(state.error_source.is_none());
}

#[test]
fn flat_fields_still_decode_when_a_server_sends_them() {
    let state: super::ToolState = serde_json::from_value(serde_json::json!({
        "status": "error",
        "input": {},
        "error": {"message": "Interrupted"},
        "errorType": "aborted",
        "errorSource": "local"
    }))
    .expect("tool state");
    assert_eq!(state.error_kind(), Some("aborted"));
    assert_eq!(state.error_source, Some(ToolErrorSource::Local));
}
