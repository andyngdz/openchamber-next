use serde::de::DeserializeOwned;

use super::{
    AssistantContent, AuthStatus, CreateSessionRequest, Data, Location, Message, Page, Session,
    ToolStatus, WorkState,
};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

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
fn listed_session_time_carries_the_turn_stamps_when_the_server_sends_them() {
    let after: Page<Session> = decode("session-list-after-turn.json");
    let time = &after.data[0].time;
    assert!(time.busy.is_some() && time.idle.is_some());

    let bare: Session = serde_json::from_str(r#"{"id":"ses_x","time":{"created":1,"updated":2}}"#)
        .expect("session without turn stamps");
    assert_eq!((bare.time.busy, bare.time.idle), (None, None));
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
fn listed_session_carries_its_work_state() {
    let after: Page<Session> = decode("session-list-after-turn.json");
    assert_eq!(after.data[0].metadata.work_state(), Some(WorkState::Open));
    let fresh: Page<Session> = decode("session-list-nonempty.json");
    assert_eq!(fresh.data[0].metadata.work_state(), None);
}

#[test]
fn the_global_pages_carry_archived_pinned_and_btw_facts() {
    let first: Page<Session> = decode("session-list-global-page-1.json");
    let second: Page<Session> = decode("session-list-global-page-2.json");

    assert!(first.data[0].location.directory.contains("/chats/"));
    assert!(first.data[1].metadata.pinned());
    assert_eq!(first.data[1].metadata.pin_list(), None);
    assert!(second.data[0].metadata.btw());
    assert!(second.data[1].time.archived.is_some());
    assert!(first.data[1].time.archived.is_none());
}

#[test]
fn parent_id_decodes_from_the_camel_case_key() {
    let session: Session = serde_json::from_value(serde_json::json!({
        "id": "ses_child",
        "parentID": "ses_parent",
        "time": {"created": 1, "updated": 2},
    }))
    .expect("session with a parent");

    assert_eq!(session.parent_id.as_deref(), Some("ses_parent"));
}

#[test]
fn pinned_and_btw_need_their_full_markers() {
    let filed: Session = serde_json::from_value(serde_json::json!({
        "id": "ses_a",
        "metadata": {"openchamber": {"pinned": {"at": 3, "list": "mine"}}},
    }))
    .expect("filed pin");
    let empty_list: Session = serde_json::from_value(serde_json::json!({
        "id": "ses_b",
        "metadata": {"openchamber": {"pinned": {"at": 3, "list": ""}}},
    }))
    .expect("empty list");
    let kind_only: Session = serde_json::from_value(serde_json::json!({
        "id": "ses_c",
        "metadata": {"openchamber": {"kind": "btw"}},
    }))
    .expect("kind only");
    let fork: Session = serde_json::from_value(serde_json::json!({
        "id": "ses_d",
        "metadata": {"openchamber": {"kind": "btw", "originalSessionID": "ses_parent"}},
    }))
    .expect("btw fork");

    assert!(filed.metadata.pinned());
    assert_eq!(filed.metadata.pin_list(), Some("mine"));
    assert!(!empty_list.metadata.pinned());
    assert!(!kind_only.metadata.btw());
    assert!(fork.metadata.btw());
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
    assert!(!lists.is_empty(), "{lists:?}");
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
    let patterns: Vec<_> = assistants
        .iter()
        .flat_map(|content| content.iter())
        .filter_map(|c| match c {
            AssistantContent::Tool { state, .. } => state.input["pattern"].as_str(),
            _ => None,
        })
        .collect();
    assert_eq!(patterns.len(), 20);
    assert_eq!(patterns.last(), Some(&"packages/*/package.json"));
    assert!(
        page.data
            .iter()
            .any(|m| matches!(m, Message::Idle { outcome: Some(o), .. } if o == "succeeded"))
    );
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
        agent: None,
        model: None,
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
