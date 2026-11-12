//! Turns `GET /api/session` and `GET /api/session/:id/message` answers into state, with the
//! same identities the event reducer uses: text items by ordinal inside their message, tool
//! rows by call id.

use oc_client::types::{AssistantContent, Message, MessageTime, Session, ToolStatus};

use crate::ids::{CallId, MessageId, SessionId};
use crate::message::{
    AssistantMessage, ChatMessage, Delivery, Item, StepTiming, TextItem, ToolItem, UserMessage,
};
use crate::recoverable::RecordedOutcome;
use crate::session_entry::SessionEntry;
use crate::status::SessionStatus;

/// A sidebar row from a listed or created session.
pub(crate) fn session_entry(session: &Session) -> SessionEntry {
    let time = &session.time;
    let idle = time.idle.unwrap_or_default();
    let busy = time.busy.unwrap_or_default();
    let activity = [idle, time.updated, time.created]
        .into_iter()
        .find(|stamp| *stamp > 0)
        .unwrap_or_default();
    // A list carries no live status, only when the last turn started and ended: the turn runs
    // when it started after it last ended. Ties read as ended.
    let status = if busy > idle {
        SessionStatus::Busy
    } else {
        SessionStatus::Idle
    };
    SessionEntry::new(
        SessionId::from(session.id.as_str()),
        session.title.clone(),
        time.created,
        activity,
    )
    .with_directory(session.location.directory.as_str())
    .with_updated(time.updated)
    .with_split_facts(
        time.archived.is_some_and(|archived| archived > 0),
        session.metadata.pinned(),
        session.metadata.pin_list().map(str::to_owned),
        session.metadata.btw(),
        session.parent_id.clone(),
    )
    .with_listed_status(status, busy.max(idle))
    .with_work(session.metadata.work_state())
}

/// The chat column from a message list, which the server sends newest first. Turn markers
/// (`idle`), model switches and unknown types are not drawn.
pub(crate) fn chat_messages(messages: &[Message]) -> Vec<ChatMessage> {
    messages.iter().rev().filter_map(chat_message).collect()
}

fn chat_message(message: &Message) -> Option<ChatMessage> {
    match message {
        Message::User { id, text, .. } => Some(ChatMessage::User(UserMessage {
            id: MessageId::from(id.as_str()),
            text: text.clone(),
            delivery: Delivery::Delivered,
        })),
        Message::Assistant {
            id,
            agent,
            model,
            content,
            time,
        } => Some(ChatMessage::Assistant(AssistantMessage {
            id: MessageId::from(id.as_str()),
            agent: agent.clone(),
            model: model.clone(),
            items: items(content),
            timing: timing(time),
        })),
        Message::Idle { .. } | Message::Other => None,
    }
}

fn items(content: &[AssistantContent]) -> Vec<Item> {
    let mut next_ordinal = 0;
    content
        .iter()
        .filter_map(|entry| match entry {
            AssistantContent::Text { text } => {
                let ordinal = next_ordinal;
                next_ordinal += 1;
                Some(Item::Text(TextItem {
                    ordinal,
                    text: text.clone(),
                }))
            }
            AssistantContent::Tool { id, name, state } => Some(Item::Tool(ToolItem {
                call_id: CallId::from(id.as_str()),
                name: name.clone(),
                input: state.input.clone(),
                status: state.status,
            })),
            AssistantContent::Other => None,
        })
        .collect()
}

fn timing(time: &MessageTime) -> StepTiming {
    match time.completed {
        Some(completed) => StepTiming::Completed {
            started: time.created,
            completed,
        },
        None => StepTiming::Running {
            started: time.created,
        },
    }
}

/// How the loaded history recorded the end of its trailing turn, newest first
/// (`sync-context.tsx:2795-2810`).
pub(crate) fn recorded_turn(messages: &[Message]) -> Option<RecordedOutcome> {
    let mut end = None;
    for message in messages {
        match message {
            Message::User { .. } => return None,
            Message::Assistant { .. } => return end,
            Message::Idle { outcome, .. } => {
                if end.is_none() {
                    end = parse_outcome(outcome.as_deref());
                }
            }
            Message::Other => {}
        }
    }
    end
}

fn parse_outcome(outcome: Option<&str>) -> Option<RecordedOutcome> {
    match outcome? {
        "succeeded" => Some(RecordedOutcome::Succeeded),
        "interrupted" => Some(RecordedOutcome::Interrupted),
        "failed" => Some(RecordedOutcome::Failed),
        _ => Some(RecordedOutcome::Other),
    }
}

/// The newest assistant message before any newer user message, from a newest-first list.
pub(crate) fn trailing_loaded(messages: &[Message]) -> Option<&Message> {
    for message in messages {
        match message {
            Message::User { .. } => return None,
            Message::Assistant { .. } => return Some(message),
            Message::Idle { .. } | Message::Other => {}
        }
    }
    None
}

/// Sets the loaded tool parts the history mark promoted to `error`.
pub(crate) fn promote_loaded(messages: &mut [ChatMessage], promoted: &[(String, String)]) {
    for message in messages {
        let ChatMessage::Assistant(step) = message else {
            continue;
        };
        for item in &mut step.items {
            let Item::Tool(tool) = item else {
                continue;
            };
            if promoted.iter().any(|(message_id, call_id)| {
                message_id == step.id.as_str() && call_id == tool.call_id.as_str()
            }) {
                tool.status = ToolStatus::Error;
            }
        }
    }
}

#[cfg(test)]
#[path = "load_test.rs"]
mod load_test;
