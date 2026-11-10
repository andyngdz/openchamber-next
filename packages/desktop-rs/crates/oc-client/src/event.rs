//! Wire events of `GET /api/event`.
//!
//! Every frame is `{id, created?, type, data, …}`. The envelope is decoded into
//! [`Event`]; `type` selects the typed payload in [`EventData`]. Types the spike does not
//! handle (notices such as `provider.updated`, `session.usage.updated`) decode to
//! [`EventData::Raw`], never to an error. Payload field names follow the captured
//! fixtures in `fixtures/events/`; `session.status`, `session.idle`,
//! `session.execution.failed`, `session.tool.failed` and the `form.*` events were never
//! sent by the captured server, so their shapes follow `events.ts`.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub use crate::payload::*;

/// One decoded frame.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawEvent")]
pub struct Event {
    pub id: String,
    /// Milliseconds since the epoch; `server.connected` has none.
    pub created: Option<u64>,
    pub data: EventData,
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(default)]
    id: String,
    #[serde(default)]
    created: Option<u64>,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    data: Value,
}

/// The handled event types, and [`EventData::Raw`] for the rest.
#[derive(Debug, Clone, PartialEq)]
pub enum EventData {
    ServerConnected,
    SessionCreated(SessionCreated),
    SessionRenamed(SessionRenamed),
    SessionDeleted(SessionRef),
    InboxEnqueued(InboxEnqueued),
    InboxDelivered(InboxDelivered),
    ExecutionStarted(SessionRef),
    ExecutionSucceeded(SessionRef),
    ExecutionInterrupted(ExecutionInterrupted),
    ExecutionFailed(ExecutionFailed),
    SessionStatus(SessionStatus),
    SessionIdle(SessionRef),
    StepStarted(StepStarted),
    StepEnded(StepEnded),
    TextStarted(TextRef),
    TextDelta(TextDelta),
    TextEnded(TextEnded),
    ToolInputStarted(ToolInputStarted),
    ToolCalled(ToolRef),
    ToolSuccess(ToolRef),
    ToolFailed(ToolFailed),
    PermissionAsked(PermissionAsked),
    PermissionReplied(PermissionReplied),
    FormCreated(FormCreated),
    FormReplied(FormReplied),
    FormCancelled(FormReplied),
    /// An event type the spike does not handle, kept whole.
    Raw {
        kind: String,
        data: Value,
    },
}

impl TryFrom<RawEvent> for Event {
    type Error = serde_json::Error;

    fn try_from(raw: RawEvent) -> Result<Self, Self::Error> {
        let data = EventData::decode(raw.kind, raw.data)?;
        Ok(Self {
            id: raw.id,
            created: raw.created,
            data,
        })
    }
}

fn payload<T: DeserializeOwned>(data: Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(data)
}

impl EventData {
    fn decode(kind: String, data: Value) -> Result<Self, serde_json::Error> {
        Ok(match kind.as_str() {
            "server.connected" => Self::ServerConnected,
            "session.created" => Self::SessionCreated(payload(data)?),
            "session.renamed" => Self::SessionRenamed(payload(data)?),
            "session.deleted" => Self::SessionDeleted(payload(data)?),
            "session.inbox.enqueued" => Self::InboxEnqueued(payload(data)?),
            "session.inbox.delivered" => Self::InboxDelivered(payload(data)?),
            "session.execution.started" => Self::ExecutionStarted(payload(data)?),
            "session.execution.succeeded" => Self::ExecutionSucceeded(payload(data)?),
            "session.execution.interrupted" => Self::ExecutionInterrupted(payload(data)?),
            "session.execution.failed" => Self::ExecutionFailed(payload(data)?),
            "session.status" => Self::SessionStatus(payload(data)?),
            "session.idle" => Self::SessionIdle(payload(data)?),
            "session.step.started" => Self::StepStarted(payload(data)?),
            "session.step.ended" => Self::StepEnded(payload(data)?),
            "session.text.started" => Self::TextStarted(payload(data)?),
            "session.text.delta" => Self::TextDelta(payload(data)?),
            "session.text.ended" => Self::TextEnded(payload(data)?),
            "session.tool.input.started" => Self::ToolInputStarted(payload(data)?),
            "session.tool.called" => Self::ToolCalled(payload(data)?),
            "session.tool.success" => Self::ToolSuccess(payload(data)?),
            "session.tool.failed" => Self::ToolFailed(payload(data)?),
            "permission.asked" => Self::PermissionAsked(payload(data)?),
            "permission.replied" => Self::PermissionReplied(payload(data)?),
            "form.created" => Self::FormCreated(payload(data)?),
            "form.replied" => Self::FormReplied(payload(data)?),
            "form.cancelled" => Self::FormCancelled(payload(data)?),
            _ => Self::Raw { kind, data },
        })
    }
}

#[cfg(test)]
#[path = "event_test.rs"]
mod event_test;
