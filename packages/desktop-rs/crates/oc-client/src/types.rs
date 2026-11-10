//! Response and request bodies, holding only the fields the spike reads.
//!
//! Shapes come from `fixtures/api/`. Optional fields carry `#[serde(default)]`; fields
//! the server adds later are ignored.

use serde::{Deserialize, Serialize};

/// `GET /auth/session`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthStatus {
    /// `true` when the server lets this client through. With no UI password the server
    /// also sends `disabled: true`, which the spike does not read.
    pub authenticated: bool,
}

/// `{"data": …}` around a single object.
#[derive(Debug, Clone, Deserialize)]
pub struct Data<T> {
    /// The wrapped object.
    pub data: T,
}

/// `{"data": […], "cursor": {…}}`; the spike does not page, so the cursor is dropped.
#[derive(Debug, Clone, Deserialize)]
pub struct Page<T> {
    /// The entries of this page.
    pub data: Vec<T>,
}

/// Where a session or event lives; only the directory is read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Location {
    /// Absolute project directory.
    #[serde(default)]
    pub directory: String,
}

/// Session timestamps in milliseconds since the epoch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct SessionTime {
    /// When the session was created.
    #[serde(default)]
    pub created: u64,
    /// When the session last changed.
    #[serde(default)]
    pub updated: u64,
    /// When the last turn started; missing until the first turn.
    #[serde(default)]
    pub busy: Option<u64>,
    /// When the last turn ended; missing until the first turn ends.
    #[serde(default)]
    pub idle: Option<u64>,
}

/// One session of the list or of a create answer. `title` is missing until the first
/// turn has been titled.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Session {
    /// `ses_…` id.
    pub id: String,
    /// Missing until the first turn has been titled.
    #[serde(default)]
    pub title: Option<String>,
    /// Directory the session belongs to.
    #[serde(default)]
    pub location: Location,
    /// Created and updated times.
    #[serde(default)]
    pub time: SessionTime,
}

/// Body of `POST /api/session`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateSessionRequest {
    /// Directory the new session is created in.
    pub location: Location,
}

/// Body of `POST /api/session/:id/prompt`; the plain composer send carries nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PromptRequest {
    /// Client-minted `msg_…` id, see [`crate::new_message_id`].
    pub id: String,
    /// What the user typed.
    pub text: String,
}

/// The accepted prompt, echoed back by the server as the user message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PromptAccepted {
    /// The `msg_…` id the client sent.
    pub id: String,
    /// Session the prompt went to.
    #[serde(rename = "sessionID", default)]
    pub session_id: String,
}

/// The model a step or message ran on; an object on the wire, never a string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ModelRef {
    /// Model id.
    #[serde(default)]
    pub id: String,
    /// Provider id.
    #[serde(rename = "providerID", default)]
    pub provider_id: String,
    /// Variant, when the model has one.
    #[serde(default)]
    pub variant: Option<String>,
}

/// Message timestamps in milliseconds since the epoch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct MessageTime {
    /// When the message was created.
    #[serde(default)]
    pub created: u64,
    /// When an assistant message finished; absent while it streams.
    #[serde(default)]
    pub completed: Option<u64>,
}

/// A text block or a tool call inside an assistant message.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantContent {
    /// A text block.
    #[serde(rename = "text")]
    Text {
        /// The block's text.
        text: String,
    },
    /// A tool call.
    #[serde(rename = "tool")]
    Tool {
        /// Call id.
        id: String,
        /// Tool name.
        name: String,
        /// Current state of the call.
        state: ToolState,
    },
    /// Content kinds the spike does not draw (reasoning, files).
    #[serde(other)]
    Other,
}

/// State of a tool call; only the status is read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolState {
    /// Where the call is in its life.
    pub status: ToolStatus,
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

/// One entry of `GET /api/session/:id/message`. Entries are newest first by default.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "user")]
    User {
        id: String,
        #[serde(default)]
        text: String,
        #[serde(default)]
        time: MessageTime,
    },
    #[serde(rename = "assistant")]
    Assistant {
        id: String,
        #[serde(default)]
        agent: String,
        #[serde(default)]
        model: ModelRef,
        #[serde(default)]
        content: Vec<AssistantContent>,
        #[serde(default)]
        time: MessageTime,
    },
    /// End-of-turn marker with the turn's `outcome`.
    #[serde(rename = "idle")]
    Idle {
        id: String,
        #[serde(default)]
        outcome: Option<String>,
    },
    /// `model-switched` and any type the spike does not draw.
    #[serde(other)]
    Other,
}

#[cfg(test)]
#[path = "types_test.rs"]
mod types_test;
