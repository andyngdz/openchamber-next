//! Turns `GET /api/session` and `GET /api/session/:id/message` answers into state, with the
//! same identities the event reducer uses: text items by ordinal inside their message, tool
//! rows by call id.

use oc_client::types::{AssistantContent, Message, MessageTime, Session};

use crate::ids::{CallId, MessageId, SessionId};
use crate::message::{
    AssistantMessage, ChatMessage, Delivery, Item, StepTiming, TextItem, ToolItem, UserMessage,
};
use crate::session_list::SessionEntry;
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

#[cfg(test)]
#[path = "load_test.rs"]
mod load_test;
