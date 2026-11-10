//! Response and request bodies, holding only the fields the spike reads.
//!
//! Shapes come from `fixtures/api/`. Optional fields carry `#[serde(default)]`; fields
//! the server adds later are ignored.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::metadata::{SessionMetadata, WorkState};
pub use crate::requests::{
    CreateSessionRequest, SessionRecord, SwitchAgentRequest, SwitchModelRequest,
};

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

/// `{"data": […], "cursor": {…}}` around a list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Page<T> {
    /// The entries of this page.
    pub data: Vec<T>,
    /// Where the next and previous pages are; absent on an unpaged answer.
    #[serde(default)]
    pub cursor: PageCursor,
}

/// The opaque page anchors of a list answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PageCursor {
    /// The cursor of the page before this one.
    #[serde(default)]
    pub previous: Option<String>,
    /// The cursor to request next; the last page still carries one.
    #[serde(default)]
    pub next: Option<String>,
}

/// `{"location": {"directory"}, "data": …}` around a directory-scoped list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Located<T> {
    /// The directory the answer belongs to.
    #[serde(default)]
    pub location: Location,
    /// The list.
    pub data: T,
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
    /// Metadata the server and clients attach; the spike reads the work state.
    #[serde(default)]
    pub metadata: SessionMetadata,
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

/// The model a step, message or request names; a catalog `id` plus its provider.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ModelRef {
    /// Catalog model id.
    #[serde(default)]
    pub id: String,
    /// Provider id.
    #[serde(rename = "providerID", default)]
    pub provider_id: String,
    /// Variant, when the model has one; left out of a request for Default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// State of a tool call: its status and its input.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolState {
    /// Where the call is in its life.
    pub status: ToolStatus,
    /// The tool's arguments; missing while the input still streams.
    #[serde(default)]
    pub input: Value,
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
