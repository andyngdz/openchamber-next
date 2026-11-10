//! Payload structs of the handled wire events, see [`crate::event::EventData`].
//!
//! Field names follow `fixtures/events/`; ids keep the wire spelling through `serde`
//! renames.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{Location, ModelRef};

/// Payloads that carry only the session id.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SessionRef {
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

/// `session.created` has no title; it arrives in `session.renamed`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionCreated {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(default)]
    pub location: Location,
}

/// `session.renamed`: the session got its title.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionRenamed {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub title: String,
}

/// `session.inbox.enqueued`: the user message arrived; `inbox_id` is the message id the client minted.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InboxEnqueued {
    #[serde(rename = "inboxID")]
    pub inbox_id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub item: InboxItem,
}

/// The queued item of [`InboxEnqueued`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InboxItem {
    #[serde(default)]
    pub payload: InboxPayload,
}

/// Text of a queued user message.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct InboxPayload {
    #[serde(default)]
    pub text: String,
}

/// `session.inbox.delivered`: the queued message reached the model.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InboxDelivered {
    #[serde(rename = "inboxID")]
    pub inbox_id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

/// `reason` is `"shutdown"` when OpenCode itself is going away; the turn stays open then.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExecutionInterrupted {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// `session.execution.failed`: the turn ended in an error.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExecutionFailed {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(default)]
    pub error: Option<Value>,
}

/// `session.status`: an explicit busy/idle/retry status.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionStatus {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub status: StatusKind,
}

/// Status carried by `session.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StatusKind {
    /// A turn is running.
    Busy,
    /// Nothing is running.
    Idle,
    /// The turn is waiting to retry.
    Retry,
    /// A status this client does not know.
    #[serde(other)]
    Other,
}

/// `session.step.started`: opens one assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StepStarted {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub model: ModelRef,
    #[serde(default)]
    pub started: Option<u64>,
}

/// `session.step.ended`: closes the step's assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StepEnded {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    #[serde(default)]
    pub finish: Option<String>,
}

/// Identifies one text item: the step's assistant message and its ordinal.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TextRef {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    #[serde(default)]
    pub ordinal: u32,
}

/// `session.text.delta`: more text for an item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TextDelta {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    #[serde(default)]
    pub ordinal: u32,
    pub delta: String,
}

/// `text` is the whole item and replaces whatever deltas built up.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TextEnded {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    #[serde(default)]
    pub ordinal: u32,
    pub text: String,
}

/// The request id is `id` here and `requestID` in [`PermissionReplied`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PermissionAsked {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

/// `permission.replied`: the request is answered.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PermissionReplied {
    #[serde(rename = "requestID")]
    pub request_id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

/// `form.created`: the agent asks the user to fill a form.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FormCreated {
    pub form: FormRef,
}

/// The form of [`FormCreated`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FormRef {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

/// `form.replied` and `form.cancelled`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FormReplied {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
}
