use serde_json::{Value, json};

use crate::fixture_test::{event, form_from};
use crate::ids::{CallId, MessageId};
use crate::message::{AssistantMessage, ChatMessage, Item, StepTiming, ToolItem};
use crate::recoverable::{Recoverable, question_tool_link};
use crate::tool_events::{ToolTransition, record};

fn question_form() -> oc_client::form::FormInfo {
    form_from(json!({
        "id": "frm_1",
        "sessionID": "ses_ask",
        "metadata": {"kind": "question", "tool": {"messageID": "msg_ask", "id": "tool_ask"}},
        "fields": [{"key": "q0", "type": "string"}]
    }))
}

fn tool_message(status: oc_client::types::ToolStatus) -> ChatMessage {
    ChatMessage::Assistant(AssistantMessage {
        id: MessageId::from("msg_ask"),
        agent: String::new(),
        model: Default::default(),
        items: vec![Item::Tool(ToolItem {
            call_id: CallId::from("tool_ask"),
            name: "question".to_owned(),
            input: Value::Null,
            status,
        })],
        timing: StepTiming::Running { started: 1 },
    })
}

#[test]
fn a_failed_question_records_its_death() {
    let form = question_form();
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let failed = event(
        "session.tool.failed",
        1,
        json!({
            "sessionID": "ses_ask",
            "assistantMessageID": "msg_ask",
            "id": "tool_ask",
            "error": {"type": "aborted", "message": "Tool execution interrupted"}
        }),
    );
    assert_eq!(
        record(&mut recovery, &failed),
        ToolTransition::Authoritative
    );
    let link = question_tool_link(&form).expect("link");
    assert!(recovery.has_disposition(&link));
    let messages = vec![tool_message(oc_client::types::ToolStatus::Error)];
    assert_eq!(
        recovery
            .offer(&messages, &["ses_ask".to_owned()], &Default::default())
            .len(),
        1
    );
}

#[test]
fn an_authoritative_success_replaces_the_local_interrupt_mark() {
    let form = question_form();
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let mut messages = vec![tool_message(oc_client::types::ToolStatus::Running)];
    recovery.evidence_mut().mark_trailing(&mut messages);
    assert!(recovery.evidence().is_local("msg_ask", "tool_ask"));

    let succeeded = event(
        "session.tool.success",
        2,
        json!({"sessionID": "ses_ask", "assistantMessageID": "msg_ask", "id": "tool_ask"}),
    );
    assert_eq!(
        record(&mut recovery, &succeeded),
        ToolTransition::ReplaceLocalMark
    );
    assert!(!recovery.evidence().is_local("msg_ask", "tool_ask"));
    let link = question_tool_link(&form).expect("link");
    assert!(recovery.has_disposition(&link));
    assert!(
        recovery
            .offer(&messages, &["ses_ask".to_owned()], &Default::default())
            .is_empty()
    );
}

#[test]
fn a_dismissed_question_keeps_its_dismissal() {
    let form = question_form();
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let failed = event(
        "session.tool.failed",
        1,
        json!({
            "sessionID": "ses_ask",
            "assistantMessageID": "msg_ask",
            "id": "tool_ask",
            "error": {"type": "tool.execution", "message": "form cancelled"}
        }),
    );
    record(&mut recovery, &failed);
    let link = question_tool_link(&form).expect("link");
    assert!(recovery.has_disposition(&link));
    let messages = vec![tool_message(oc_client::types::ToolStatus::Error)];
    assert!(
        recovery
            .offer(&messages, &["ses_ask".to_owned()], &Default::default())
            .is_empty()
    );
}
