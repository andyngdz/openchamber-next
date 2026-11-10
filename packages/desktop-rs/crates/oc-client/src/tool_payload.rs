//! Payload structs of the tool-call events, see [`crate::event::EventData`]. A call is keyed
//! by its call id inside its step's assistant message.

use serde::Deserialize;
use serde_json::Value;

/// The only tool event that carries the tool name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ToolInputStarted {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    /// Call id.
    pub id: String,
    pub name: String,
}

/// `session.tool.called`: the input is complete and the tool starts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ToolCalled {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    /// Call id.
    pub id: String,
    /// The tool's arguments, as the model wrote them.
    #[serde(default)]
    pub input: Value,
}

/// `session.tool.success`, keyed by call id.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ToolRef {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    /// Call id.
    pub id: String,
}

/// `session.tool.failed`: the call ended in an error.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolFailed {
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "assistantMessageID")]
    pub assistant_message_id: String,
    /// Call id.
    pub id: String,
    #[serde(default)]
    pub error: Option<Value>,
}
