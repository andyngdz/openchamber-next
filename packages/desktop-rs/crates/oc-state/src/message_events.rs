//! Message events of the open session: the user's message from the inbox, one assistant
//! message per step, text items by `(assistantMessageID, ordinal)`, tool rows by call id inside
//! their message. Events of a listed session that is not open change nothing here; its messages
//! load when it opens.

use oc_client::event::{
    InboxEnqueued, StepEnded, StepStarted, TextDelta, TextEnded, TextRef, ToolInputStarted,
};
use oc_client::types::ToolStatus;
use oc_client::{Event, EventData};

use crate::ids::{CallId, MessageId};
use crate::message::{
    AssistantMessage, ChatMessage, Delivery, Item, StepTiming, ToolItem, UserMessage,
};
use crate::state::State;

/// Applies a message event; every other event type changes nothing here.
pub(crate) fn reduce_message_event(state: &mut State, event: &Event) {
    let at = event.created.unwrap_or_default();
    let session_id = match &event.data {
        EventData::InboxEnqueued(enqueued) => &enqueued.session_id,
        EventData::InboxDelivered(delivered) => &delivered.session_id,
        EventData::InboxCancelled(cancelled) => &cancelled.session_id,
        EventData::StepStarted(step) => &step.session_id,
        EventData::StepEnded(step) => &step.session_id,
        EventData::TextStarted(text) => &text.session_id,
        EventData::TextDelta(delta) => &delta.session_id,
        EventData::TextEnded(text) => &text.session_id,
        EventData::ToolInputStarted(tool) => &tool.session_id,
        EventData::ToolCalled(tool) | EventData::ToolSuccess(tool) => &tool.session_id,
        EventData::ToolFailed(tool) => &tool.session_id,
        EventData::ServerConnected
        | EventData::SessionCreated(_)
        | EventData::SessionRenamed(_)
        | EventData::SessionDeleted(_)
        | EventData::ExecutionStarted(_)
        | EventData::ExecutionSucceeded(_)
        | EventData::ExecutionInterrupted(_)
        | EventData::ExecutionFailed(_)
        | EventData::SessionStatus(_)
        | EventData::SessionIdle(_)
        | EventData::PermissionAsked(_)
        | EventData::PermissionReplied(_)
        | EventData::FormCreated(_)
        | EventData::FormReplied(_)
        | EventData::FormCancelled(_)
        | EventData::Raw { .. } => return,
    };
    if let Some(messages) = state.open_messages_mut(session_id) {
        apply(messages, event, at);
    }
}

fn apply(messages: &mut Vec<ChatMessage>, event: &Event, at: u64) {
    match &event.data {
        EventData::InboxEnqueued(enqueued) => enqueue(messages, enqueued),
        EventData::InboxDelivered(delivered) => deliver(messages, &delivered.inbox_id),
        EventData::InboxCancelled(cancelled) => withdraw(messages, &cancelled.inbox_id),
        EventData::StepStarted(step) => start_step(messages, step, at),
        EventData::StepEnded(step) => end_step(messages, step, at),
        EventData::TextStarted(text) => start_text(messages, event, text),
        EventData::TextDelta(delta) => append_text(messages, event, delta),
        EventData::TextEnded(text) => replace_text(messages, event, text),
        EventData::ToolInputStarted(tool) => start_tool(messages, event, tool),
        EventData::ToolCalled(tool) => {
            set_tool(
                messages,
                event,
                &tool.assistant_message_id,
                &tool.id,
                ToolStatus::Running,
            );
        }
        EventData::ToolSuccess(tool) => {
            set_tool(
                messages,
                event,
                &tool.assistant_message_id,
                &tool.id,
                ToolStatus::Completed,
            );
        }
        EventData::ToolFailed(tool) => {
            set_tool(
                messages,
                event,
                &tool.assistant_message_id,
                &tool.id,
                ToolStatus::Error,
            );
        }
        _ => {}
    }
}

fn enqueue(messages: &mut Vec<ChatMessage>, enqueued: &InboxEnqueued) {
    if messages
        .iter()
        .any(|message| message.id() == enqueued.inbox_id.as_str())
    {
        return;
    }
    messages.push(ChatMessage::User(UserMessage {
        id: MessageId::from(enqueued.inbox_id.as_str()),
        text: enqueued.item.payload.text.clone(),
        delivery: Delivery::Queued,
    }));
}

fn deliver(messages: &mut [ChatMessage], inbox_id: &str) {
    for message in messages {
        if let ChatMessage::User(user) = message
            && user.id == *inbox_id
        {
            user.delivery = Delivery::Delivered;
        }
    }
}

/// A withdrawn user message leaves the chat (`message.removed` in `events.ts`).
fn withdraw(messages: &mut Vec<ChatMessage>, inbox_id: &str) {
    messages.retain(|message| !matches!(message, ChatMessage::User(user) if user.id == *inbox_id));
}

/// Opens the step's assistant message; a repeated start keeps the items it already has.
fn start_step(messages: &mut Vec<ChatMessage>, step: &StepStarted, at: u64) {
    let started = step.started.unwrap_or(at);
    if let Some(existing) = assistant_mut(messages, &step.assistant_message_id) {
        existing.agent.clone_from(&step.agent);
        existing.model.clone_from(&step.model);
        return;
    }
    messages.push(ChatMessage::Assistant(AssistantMessage {
        id: MessageId::from(step.assistant_message_id.as_str()),
        agent: step.agent.clone(),
        model: step.model.clone(),
        items: Vec::new(),
        timing: StepTiming::Running { started },
    }));
}

fn end_step(messages: &mut [ChatMessage], step: &StepEnded, at: u64) {
    if let Some(existing) = assistant_mut(messages, &step.assistant_message_id) {
        let started = match existing.timing {
            StepTiming::Running { started } | StepTiming::Completed { started, .. } => started,
        };
        existing.timing = StepTiming::Completed {
            started,
            completed: at,
        };
    }
}

fn start_text(messages: &mut [ChatMessage], event: &Event, text: &TextRef) {
    with_step(messages, event, &text.assistant_message_id, |step| {
        step.start_text(text.ordinal);
    });
}

fn append_text(messages: &mut [ChatMessage], event: &Event, delta: &TextDelta) {
    with_step(messages, event, &delta.assistant_message_id, |step| {
        step.append_text(delta.ordinal, &delta.delta);
    });
}

/// `session.text.ended` carries the whole text and replaces whatever the deltas built.
fn replace_text(messages: &mut [ChatMessage], event: &Event, text: &TextEnded) {
    with_step(messages, event, &text.assistant_message_id, |step| {
        step.replace_text(text.ordinal, &text.text);
    });
}

fn start_tool(messages: &mut [ChatMessage], event: &Event, tool: &ToolInputStarted) {
    with_step(messages, event, &tool.assistant_message_id, |step| {
        if step.tool_mut(&tool.id).is_none() {
            step.items.push(Item::Tool(ToolItem {
                call_id: CallId::from(tool.id.as_str()),
                name: tool.name.clone(),
                status: ToolStatus::Pending,
            }));
        }
    });
}

/// A finished call keeps its result: a late or replayed transition does not move it back.
fn set_tool(
    messages: &mut [ChatMessage],
    event: &Event,
    message_id: &str,
    call_id: &str,
    status: ToolStatus,
) {
    with_step(messages, event, message_id, |step| {
        match step.tool_mut(call_id) {
            Some(row) if !matches!(row.status, ToolStatus::Completed | ToolStatus::Error) => {
                row.status = status;
            }
            Some(_) => {}
            None => missing(event, call_id),
        }
    });
}

fn assistant_mut<'a>(
    messages: &'a mut [ChatMessage],
    message_id: &str,
) -> Option<&'a mut AssistantMessage> {
    messages.iter_mut().find_map(|message| match message {
        ChatMessage::Assistant(assistant) if assistant.id == *message_id => Some(assistant),
        ChatMessage::Assistant(_) | ChatMessage::User(_) => None,
    })
}

/// Runs `change` on the step's message. A part whose step was never started is dropped: the
/// message list loaded after a reconnect is the authority for it.
fn with_step(
    messages: &mut [ChatMessage],
    event: &Event,
    message_id: &str,
    change: impl FnOnce(&mut AssistantMessage),
) {
    match assistant_mut(messages, message_id) {
        Some(step) => change(step),
        None => missing(event, message_id),
    }
}

fn missing(event: &Event, target: &str) {
    tracing::debug!(
        target: crate::logging::STATE,
        target_id = target,
        event = %event.id,
        "[STATE] Dropped event for a message or tool row not seen yet"
    );
}

#[cfg(test)]
#[path = "message_events_test.rs"]
mod message_events_test;
