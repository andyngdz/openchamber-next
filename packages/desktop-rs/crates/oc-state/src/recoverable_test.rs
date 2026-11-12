use std::collections::BTreeSet;

use oc_client::types::{AssistantContent, Message, ToolStatus};
use serde_json::{Value, json};

use crate::action::Action;
use crate::fixture_test::{api_messages, directory, form_from, opened};
use crate::ids::{CallId, MessageId, SessionId};
use crate::message::{AssistantMessage, ChatMessage, Item, StepTiming, ToolItem};
use crate::recoverable::{
    ClosedOutcome, QuestionAskOutcome, Recoverable, RecoveryEvidence, question_tool_link,
};

fn question_form(id: &str, session: &str) -> oc_client::form::FormInfo {
    form_from(json!({
        "id": id,
        "sessionID": session,
        "title": "Questions",
        "metadata": {"kind": "question", "tool": {"messageID": "msg_ask", "id": "tool_ask"}},
        "fields": [{"key": "q0", "type": "string"}]
    }))
}

fn tool_message(status: ToolStatus) -> ChatMessage {
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

fn scope() -> Vec<String> {
    vec!["ses_ask".to_owned()]
}

fn pending() -> BTreeSet<String> {
    BTreeSet::new()
}

#[test]
fn keeps_a_question_when_the_request_vanished_without_a_settle_event() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery.evidence_mut().record_failure(
        "msg_ask",
        "tool_ask",
        "Tool execution interrupted",
        Some("aborted"),
    );
    let messages = vec![tool_message(ToolStatus::Error)];
    let recovered = recovery.offer(&messages, &scope(), &pending());
    assert_eq!(recovered.len(), 1);
    let entry = &recovered[0];
    assert_eq!(entry.form.id, "frm_1");
    assert_eq!(entry.tool_message_id, "msg_ask");
    assert_eq!(entry.evidence, RecoveryEvidence::ToolInterrupted);
}

#[test]
fn keeps_the_question_the_local_interrupt_mark_closed() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let mut messages = vec![tool_message(ToolStatus::Running)];
    recovery.evidence_mut().mark_trailing(&mut messages);
    let recovered = recovery.offer(&messages, &scope(), &pending());
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].evidence, RecoveryEvidence::LocalInterruptMark);
}

#[test]
fn never_shows_a_question_whose_request_is_still_pending() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery.evidence_mut().record_failure(
        "msg_ask",
        "tool_ask",
        "Tool execution interrupted",
        Some("aborted"),
    );
    let mut still_pending = BTreeSet::new();
    still_pending.insert("frm_1".to_owned());
    let messages = vec![tool_message(ToolStatus::Error)];
    assert!(
        recovery
            .offer(&messages, &scope(), &still_pending)
            .is_empty()
    );
}

#[test]
fn keeps_only_questions_inside_the_session_scope() {
    let form = question_form("frm_1", "ses_child");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery.evidence_mut().record_failure(
        "msg_ask",
        "tool_ask",
        "Tool execution interrupted",
        Some("aborted"),
    );
    let messages = vec![tool_message(ToolStatus::Error)];
    assert_eq!(
        recovery
            .offer(
                &messages,
                &["ses_parent".to_owned(), "ses_child".to_owned()],
                &pending()
            )
            .len(),
        1
    );
    assert!(
        recovery
            .offer(&messages, &["ses_parent".to_owned()], &pending())
            .is_empty()
    );
}

#[test]
fn a_replayed_interruption_cannot_replace_a_confirmed_answer() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let link = question_tool_link(&form).expect("link");
    recovery.record_disposition("ses_ask", &link, QuestionAskOutcome::Answered);
    let messages = vec![tool_message(ToolStatus::Error)];
    assert!(recovery.offer(&messages, &scope(), &pending()).is_empty());
}

#[test]
fn a_reply_event_or_a_local_dismissal_closes_it_for_good() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery.note_replied("frm_1");
    let messages = vec![tool_message(ToolStatus::Error)];
    assert!(recovery.offer(&messages, &scope(), &pending()).is_empty());

    let mut dismissed = Recoverable::default();
    dismissed.note_form(&form, "/repo");
    dismissed.note_cancelled("frm_1");
    assert!(dismissed.offer(&messages, &scope(), &pending()).is_empty());
}

#[test]
fn an_authoritative_answer_revokes_an_already_promoted_card() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let link = question_tool_link(&form).expect("link");
    recovery.record_disposition("ses_ask", &link, QuestionAskOutcome::TurnDeath);
    let messages = vec![tool_message(ToolStatus::Error)];
    assert_eq!(recovery.offer(&messages, &scope(), &pending()).len(), 1);

    recovery.record_disposition("ses_ask", &link, QuestionAskOutcome::Answered);
    assert!(recovery.offer(&messages, &scope(), &pending()).is_empty());
}

#[test]
fn a_shutdown_cancellation_is_never_recovered() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery.evidence_mut().record_failure(
        "msg_ask",
        "tool_ask",
        "Interaction cancelled because the location shut down",
        Some("aborted"),
    );
    let messages = vec![tool_message(ToolStatus::Error)];
    assert!(recovery.offer(&messages, &scope(), &pending()).is_empty());
}

#[test]
fn a_running_tool_call_stays_unrecovered_until_its_death_is_recorded() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let mut messages = vec![tool_message(ToolStatus::Running)];
    assert!(recovery.offer(&messages, &scope(), &pending()).is_empty());
    recovery.evidence_mut().mark_trailing(&mut messages);
    assert_eq!(recovery.offer(&messages, &scope(), &pending()).len(), 1);
}

#[test]
fn unrecognized_evidence_fails_closed() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery
        .evidence_mut()
        .record_failure("msg_ask", "tool_ask", "weird", Some("weird"));
    let messages = vec![tool_message(ToolStatus::Error)];
    assert!(recovery.offer(&messages, &scope(), &pending()).is_empty());
}

#[test]
fn bounds_seen_outcomes_and_dispositions_to_one_hundred() {
    let mut recovery = Recoverable::default();
    for index in 0..160 {
        let id = format!("frm_{index}");
        let mut form = question_form(&id, "ses_ask");
        let message_id = format!("msg_{index}");
        let tool_id = format!("tool_{index}");
        form.metadata.insert(
            "tool".to_owned(),
            json!({"messageID": message_id, "id": tool_id}),
        );
        recovery.note_form(&form, "/repo");
        let link = question_tool_link(&form).expect("link");
        recovery.record_disposition("ses_ask", &link, QuestionAskOutcome::TurnDeath);
        recovery.note_replied(&id);
    }
    assert_eq!(recovery.seen_len(), 100);
    assert_eq!(recovery.outcome_len(), 100);
    assert_eq!(recovery.disposition_len(), 100);
    assert!(!recovery.has_seen("frm_0"));
    assert!(!recovery.has_outcome("frm_0"));
    assert!(
        !recovery.has_disposition(&crate::recoverable::QuestionToolLink {
            message_id: "msg_0".to_owned(),
            tool_id: "tool_0".to_owned(),
        })
    );

    recovery.drop_session("ses_ask");
    assert_eq!(recovery.seen_len(), 0);
    assert_eq!(recovery.outcome_len(), 0);
    assert_eq!(recovery.disposition_len(), 0);
}

#[test]
fn unobserved_and_evicted_replies_do_not_accumulate_outcomes() {
    let mut recovery = Recoverable::default();
    for index in 0..160 {
        recovery.note_replied(&format!("frm_{index}"));
        recovery.note_cancelled(&format!("cancel_{index}"));
    }
    assert_eq!(recovery.outcome_len(), 0);
}

#[test]
fn a_list_read_captures_questions_a_later_death_can_recover() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_snapshot("/repo", std::slice::from_ref(&form));
    recovery.evidence_mut().record_failure(
        "msg_ask",
        "tool_ask",
        "Tool execution interrupted",
        Some("aborted"),
    );
    let messages = vec![tool_message(ToolStatus::Error)];
    assert_eq!(recovery.offer(&messages, &scope(), &pending()).len(), 1);
    recovery.note_snapshot("/repo", &[]);
    assert_eq!(recovery.offer(&messages, &scope(), &pending()).len(), 1);
}

#[test]
fn records_a_disposition_only_for_a_captured_questions_own_call() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    let other = crate::recoverable::QuestionToolLink {
        message_id: "msg_other".to_owned(),
        tool_id: "tool_other".to_owned(),
    };
    recovery.record_disposition("ses_ask", &other, QuestionAskOutcome::TurnDeath);
    assert!(!recovery.has_disposition(&other));
}

#[test]
fn offers_a_dismissed_capture_from_the_loaded_question_history() {
    let messages = api_messages("messages-question-interrupted-turn");
    let (message_id, call_id) = messages
        .iter()
        .find_map(|message| match message {
            Message::Assistant { content, .. } => content.iter().find_map(|entry| match entry {
                AssistantContent::Tool { id, name, .. } if name == "question" => {
                    let message_id = match message {
                        Message::Assistant { id, .. } => id.clone(),
                        _ => return None,
                    };
                    Some((message_id, id.clone()))
                }
                _ => None,
            }),
            _ => None,
        })
        .expect("the captured question call");
    let form = form_from(json!({
        "id": "frm_loaded",
        "sessionID": "ses_q",
        "metadata": {"kind": "question", "tool": {"messageID": message_id, "id": call_id}},
        "fields": [{"key": "q0", "type": "string"}]
    }));
    let mut state = opened("ses_q");
    let session = SessionId::from("ses_q");
    state.apply(Action::MessagesLoaded {
        session: session.clone(),
        messages,
    });
    state.forms_loaded(&directory(), vec![form]);
    state.forms_loaded(&directory(), Vec::new());
    let recovered = state.recovered_questions(&session);
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].evidence, RecoveryEvidence::ToolInterrupted);
}

#[test]
fn a_live_interrupt_marks_the_open_question_and_offers_it_again() {
    let mut state = opened("ses_a");
    state.apply(Action::Event(crate::fixture_test::event(
        "form.created",
        1,
        json!({"form": {
            "id": "frm_live",
            "sessionID": "ses_a",
            "metadata": {"kind": "question", "tool": {"messageID": "msg_ask", "id": "tool_ask"}},
            "fields": [{"key": "q0", "type": "string"}]
        }}),
    )));
    state.apply(Action::Event(crate::fixture_test::event(
        "form.cancelled",
        2,
        json!({"id": "frm_live", "sessionID": "ses_a"}),
    )));
    state.apply(Action::Event(crate::fixture_test::event(
        "session.step.started",
        3,
        json!({"sessionID": "ses_a", "assistantMessageID": "msg_ask"}),
    )));
    state.apply(Action::Event(crate::fixture_test::event(
        "session.tool.input.started",
        4,
        json!({"sessionID": "ses_a", "assistantMessageID": "msg_ask", "id": "tool_ask", "name": "question"}),
    )));
    state.apply(Action::Event(crate::fixture_test::event(
        "session.tool.called",
        5,
        json!({"sessionID": "ses_a", "assistantMessageID": "msg_ask", "id": "tool_ask", "input": {}}),
    )));
    state.apply(Action::Event(crate::fixture_test::event(
        "session.execution.interrupted",
        6,
        json!({"sessionID": "ses_a", "reason": "user"}),
    )));

    let recovered = state.recovered_questions(&SessionId::from("ses_a"));
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].evidence, RecoveryEvidence::LocalInterruptMark);
}

#[test]
fn close_keeps_the_first_outcome() {
    let form = question_form("frm_1", "ses_ask");
    let mut recovery = Recoverable::default();
    recovery.note_form(&form, "/repo");
    recovery.note_replied("frm_1");
    recovery.note_cancelled("frm_1");
    assert!(recovery.has_outcome("frm_1"));
    assert_eq!(
        recovery
            .offer(&[tool_message(ToolStatus::Error)], &scope(), &pending())
            .len(),
        0
    );
    let _ = ClosedOutcome::Answered;
}
