//! A tool call's state in a message list, including the failure OpenCode reported.
//!
//! Message lists carry a failed call as `{status, input, error: {type, message}}`
//! (`fixtures/api/messages-question-interrupted-turn.json`). The nested shape is kept so
//! the recovered-question rule can classify the failure exactly.

use serde::Deserialize;
use serde_json::Value;

/// State of a tool call: its status, its input and the error that ended it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolState {
    /// Where the call is in its life.
    pub status: ToolStatus,
    /// The tool's arguments; missing while the input still streams.
    #[serde(default)]
    pub input: Value,
    /// The failure OpenCode reported, as `{type, message}`.
    #[serde(default)]
    pub error: Option<ToolError>,
    /// A flat `errorType` some servers send beside the nested error.
    #[serde(rename = "errorType", default)]
    pub error_type: Option<String>,
    /// `local` when a client manufactured an interruption; absent from the API.
    #[serde(rename = "errorSource", default)]
    pub error_source: Option<ToolErrorSource>,
}

impl ToolState {
    /// The failure's message, when the call failed.
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

/// The failure OpenCode reports for a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ToolError {
    /// The failure kind, such as `aborted`.
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    /// The failure text.
    #[serde(default)]
    pub message: String,
}

/// Who wrote a tool error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolErrorSource {
    /// This client manufactured the interruption.
    Local,
    /// A source this client does not know.
    #[serde(other)]
    Other,
}

/// Status of a tool call as the message list reports it. A status the spike does not
/// know decodes to [`ToolStatus::Other`] so a newer server does not break the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    /// Input is still streaming.
    Pending,
    /// The tool is executing.
    Running,
    /// The tool returned a result.
    Completed,
    /// The tool failed.
    Error,
    /// A status this client does not know.
    #[serde(other)]
    Other,
}

#[cfg(test)]
#[path = "tool_state_test.rs"]
mod tool_state_test;
