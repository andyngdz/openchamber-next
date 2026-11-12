//! Payload structs of the tool-call events, see [`crate::event::EventData`]. A call is keyed
//! by its call id inside its step's assistant message.

use serde::Deserialize;
use serde_json::Value;

use crate::types::{ToolError, ToolErrorSource};

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
    /// The failure OpenCode reported, as `{type, message}`.
    #[serde(default)]
    pub error: Option<ToolError>,
    /// A flat `errorType` some servers send beside the nested error.
    #[serde(rename = "errorType", default)]
    pub error_type: Option<String>,
    /// `local` for a client-made interruption; absent from the API.
    #[serde(rename = "errorSource", default)]
    pub error_source: Option<ToolErrorSource>,
}

impl ToolFailed {
    /// The failure's message.
    pub fn error_message(&self) -> Option<&str> {
        self.error
            .as_ref()
            .map(|error| error.message.as_str())
            .filter(|message| !message.is_empty())
    }

    /// The failure's kind: the flat field first, then the nested `type`.
    pub fn error_kind(&self) -> Option<&str> {
        self.error_type
            .as_deref()
            .or_else(|| self.error.as_ref().and_then(|error| error.kind.as_deref()))
    }
}
