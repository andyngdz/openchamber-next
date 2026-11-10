use serde::de::DeserializeOwned;

use super::{
    AssistantContent, AuthStatus, CreateSessionRequest, Data, Location, Message, Page,
    PromptAccepted, PromptRequest, Session, ToolStatus,
};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/api");

fn fixture(name: &str) -> String {
    let path = format!("{FIXTURES}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"))
}

fn decode<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_str(&fixture(name)).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

fn names(prefix: &str, suffix: &str) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(FIXTURES)
        .expect("fixtures/api")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with(prefix) && name.ends_with(suffix))
        .filter(|name| !name.ends_with(".request.json"))
        .collect();
    found.sort();
    found
}

#[test]
fn auth_status_decodes() {
    let status: AuthStatus = decode("auth-status.json");
    assert!(status.authenticated);
}

#[test]
fn every_session_list_fixture_decodes() {
    let lists = names("session-list-", ".json");
    assert!(lists.len() >= 4, "{lists:?}");
    for name in lists {
        let page: Page<Session> = decode(&name);
        for session in &page.data {
            assert!(session.id.starts_with("ses_"), "{name}");
        }
    }
}

#[test]
fn listed_session_has_no_title_until_a_turn_is_titled() {
    let fresh: Page<Session> = decode("session-list-nonempty.json");
    assert_eq!(fresh.data[0].title, None);
    let after: Page<Session> = decode("session-list-after-turn.json");
    assert!(
        after.data[0]
            .title
            .as_deref()
            .is_some_and(|t| !t.is_empty())
    );
    assert!(after.data[0].location.directory.ends_with("/project"));
}

#[test]
fn every_created_session_decodes() {
    let created = names("session-create", ".json");
    assert!(created.len() >= 6, "{created:?}");
    for name in created {
        let session: Data<Session> = decode(&name);
        assert!(session.data.id.starts_with("ses_"), "{name}");
    }
}

#[test]
fn every_message_list_decodes_and_only_model_switches_are_skipped() {
    let lists = names("messages-", ".json");
    assert_eq!(lists.len(), 5, "{lists:?}");
    for name in lists {
        let raw: Page<serde_json::Value> = decode(&name);
        let switches = raw
            .data
            .iter()
            .filter(|m| m["type"] == "model-switched")
            .count();
        let page: Page<Message> = decode(&name);
        let skipped = page
            .data
            .iter()
            .filter(|m| matches!(m, Message::Other))
            .count();
        assert_eq!(skipped, switches, "{name}");
        assert_eq!(page.data.len(), raw.data.len(), "{name}");
    }
}

#[test]
fn agent_turn_yields_one_assistant_message_per_step_with_tool_calls() {
    let page: Page<Message> = decode("messages-agent-turn.json");
    let assistants: Vec<_> = page
        .data
        .iter()
        .filter_map(|m| match m {
            Message::Assistant { content, .. } => Some(content),
            _ => None,
        })
        .collect();
    assert_eq!(assistants.len(), 21);
    let tools: Vec<_> = assistants
        .iter()
        .flat_map(|content| content.iter())
        .filter_map(|c| match c {
            AssistantContent::Tool { name, state, .. } => Some((name.as_str(), state.status)),
            _ => None,
        })
        .collect();
    assert_eq!(tools.len(), 20);
    assert!(tools.iter().all(|t| *t == ("glob", ToolStatus::Completed)));
    assert!(
        page.data
            .iter()
            .any(|m| matches!(m, Message::Idle { outcome: Some(o), .. } if o == "succeeded"))
    );
}

#[test]
fn every_prompt_response_decodes() {
    let prompts = names("prompt-", ".json");
    assert!(prompts.len() >= 6, "{prompts:?}");
    for name in prompts {
        let accepted: Data<PromptAccepted> = decode(&name);
        assert!(accepted.data.id.starts_with("msg_"), "{name}");
        assert!(accepted.data.session_id.starts_with("ses_"), "{name}");
    }
}

#[test]
fn prompt_request_serializes_to_the_captured_body() {
    let captured: serde_json::Value = decode("prompt-text.request.json");
    let request = PromptRequest {
        id: captured["id"].as_str().expect("id").to_owned(),
        text: captured["text"].as_str().expect("text").to_owned(),
    };
    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn create_session_request_serializes_to_the_captured_body() {
    let captured: serde_json::Value = decode("session-create.request.json");
    let request = CreateSessionRequest {
        location: Location {
            directory: captured["location"]["directory"]
                .as_str()
                .expect("directory")
                .to_owned(),
        },
    };
    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn unknown_message_and_content_kinds_do_not_fail_the_list() {
    let body = serde_json::json!({"data": [
        {"type": "compaction", "id": "x"},
        {"type": "assistant", "id": "msg_1", "content": [
            {"type": "reasoning", "text": "t"},
            {"type": "tool", "id": "c", "name": "n", "state": {"status": "weird"}},
        ]},
    ]});
    let page: Page<Message> = serde_json::from_value(body).expect("decode");
    assert_eq!(page.data[0], Message::Other);
    let Message::Assistant { content, .. } = &page.data[1] else {
        panic!("assistant expected");
    };
    assert_eq!(content[0], AssistantContent::Other);
    assert!(
        matches!(&content[1], AssistantContent::Tool { state, .. } if state.status == ToolStatus::Other)
    );
}

#[test]
fn a_body_of_the_wrong_shape_does_not_decode() {
    assert!(serde_json::from_str::<Page<Session>>(r#"{"error":"nope"}"#).is_err());
}
