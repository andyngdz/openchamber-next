//! Which wire events this reducer owns, and the session each one names.

use oc_client::EventData;

/// Whether the event belongs to the message column.
pub(crate) fn is_message_event(data: &EventData) -> bool {
    matches!(
        data,
        EventData::InboxEnqueued(_)
            | EventData::InboxDelivered(_)
            | EventData::InboxCancelled(_)
            | EventData::StepStarted(_)
            | EventData::StepEnded(_)
            | EventData::TextStarted(_)
            | EventData::TextDelta(_)
            | EventData::TextEnded(_)
            | EventData::ToolInputStarted(_)
            | EventData::ToolCalled(_)
            | EventData::ToolSuccess(_)
            | EventData::ToolFailed(_)
    )
}

/// The session a message event belongs to.
pub(crate) fn message_session(data: &EventData) -> &str {
    match data {
        EventData::InboxEnqueued(enqueued) => &enqueued.session_id,
        EventData::InboxDelivered(delivered) => &delivered.session_id,
        EventData::InboxCancelled(cancelled) => &cancelled.session_id,
        EventData::StepStarted(step) => &step.session_id,
        EventData::StepEnded(step) => &step.session_id,
        EventData::TextStarted(text) => &text.session_id,
        EventData::TextDelta(delta) => &delta.session_id,
        EventData::TextEnded(text) => &text.session_id,
        EventData::ToolInputStarted(tool) => &tool.session_id,
        EventData::ToolCalled(tool) => &tool.session_id,
        EventData::ToolSuccess(tool) => &tool.session_id,
        EventData::ToolFailed(tool) => &tool.session_id,
        _ => "",
    }
}
